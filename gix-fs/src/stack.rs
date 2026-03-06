use std::{
    ffi::OsStr,
    path::{Component, Path, PathBuf},
};

use bstr::{BStr, BString, ByteSlice};

use gix_error::{message, ErrorExt};

use crate::Stack;

///
pub mod to_normal_path_components {
    /// The error used in [`ToNormalPathComponents::to_normal_path_components()`](super::ToNormalPathComponents::to_normal_path_components()).
    pub type Error = gix_error::Exn<gix_error::Message>;
}

/// Obtain an iterator over `OsStr`-components which are normal, none-relative and not absolute.
pub trait ToNormalPathComponents {
    /// Return an iterator over the normal components of a path, without the separator.
    fn to_normal_path_components(&self) -> impl Iterator<Item = Result<&OsStr, to_normal_path_components::Error>>;
}

impl ToNormalPathComponents for &Path {
    fn to_normal_path_components(&self) -> impl Iterator<Item = Result<&OsStr, to_normal_path_components::Error>> {
        self.components().map(|c| component_to_os_str(c, self))
    }
}

impl ToNormalPathComponents for PathBuf {
    fn to_normal_path_components(&self) -> impl Iterator<Item = Result<&OsStr, to_normal_path_components::Error>> {
        self.components().map(|c| component_to_os_str(c, self))
    }
}

fn component_to_os_str<'a>(
    component: Component<'a>,
    path_with_component: &Path,
) -> Result<&'a OsStr, to_normal_path_components::Error> {
    match component {
        Component::Normal(os_str) => Ok(os_str),
        _ => Err(message!(
            "Input path \"{}\" contains relative or absolute components",
            path_with_component.display()
        )
        .raise()),
    }
}

impl ToNormalPathComponents for &BStr {
    fn to_normal_path_components(&self) -> impl Iterator<Item = Result<&OsStr, to_normal_path_components::Error>> {
        self.split(|b| *b == b'/')
            .filter_map(|c| bytes_component_to_os_str(c, self))
    }
}

impl ToNormalPathComponents for &str {
    fn to_normal_path_components(&self) -> impl Iterator<Item = Result<&OsStr, to_normal_path_components::Error>> {
        self.split('/')
            .filter_map(|c| bytes_component_to_os_str(c.as_bytes(), (*self).into()))
    }
}

impl ToNormalPathComponents for &BString {
    fn to_normal_path_components(&self) -> impl Iterator<Item = Result<&OsStr, to_normal_path_components::Error>> {
        self.split(|b| *b == b'/')
            .filter_map(|c| bytes_component_to_os_str(c, self.as_bstr()))
    }
}

fn bytes_component_to_os_str<'a>(
    component: &'a [u8],
    path: &BStr,
) -> Option<Result<&'a OsStr, to_normal_path_components::Error>> {
    if component.is_empty() {
        return None;
    }
    let component = match gix_path::try_from_byte_slice(component.as_bstr())
        .map_err(|_| message("Could not convert to UTF8 or from UTF8 due to ill-formed input").raise())
    {
        Ok(c) => c,
        Err(err) => return Some(Err(err)),
    };
    let component = component.components().next()?;
    Some(component_to_os_str(
        component,
        gix_path::try_from_byte_slice(path.as_ref()).ok()?,
    ))
}

/// Access
impl Stack {
    /// Returns the top-level path of the stack.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the absolute path the currently set path.
    pub fn current(&self) -> &Path {
        &self.current
    }

    /// Returns the currently set path relative to the [`root()`][Stack::root()].
    pub fn current_relative(&self) -> &Path {
        &self.current_relative
    }
}

/// A delegate for use in a [`Stack`].
pub trait Delegate {
    /// Called whenever we push a directory on top of the stack, and after the respective call to [`push()`](Self::push).
    ///
    /// It is only called if the currently acted on path is a directory in itself, which is determined by knowing
    /// that it's not the last component of the path.
    /// Use [`Stack::current()`] to see the directory.
    fn push_directory(&mut self, stack: &Stack) -> std::io::Result<()>;

    /// Called after any component was pushed, with the path available at [`Stack::current()`].
    ///
    /// `is_last_component` is `true` if the path is completely built, which typically means it's not a directory.
    fn push(&mut self, is_last_component: bool, stack: &Stack) -> std::io::Result<()>;

    /// Called right after a directory-component was popped off the stack.
    ///
    /// Use it to pop information off internal data structures. Note that no equivalent call exists for popping
    /// the file-component.
    fn pop_directory(&mut self);
}

impl Stack {
    /// Create a new instance with `root` being the base for all future paths we handle, assuming it to be valid which includes
    /// symbolic links to be included in it as well.
    pub fn new(root: PathBuf) -> Self {
        Stack {
            current: root.clone(),
            current_relative: PathBuf::with_capacity(128),
            valid_components: 0,
            root,
            current_is_directory: true,
        }
    }

    /// Set the current stack to point to the `relative` path and call `push_comp()` each time a new path component is popped
    /// along with the stacks state for inspection to perform an operation that produces some data.
    ///
    /// The full path to `relative` will be returned along with the data returned by `push_comp`.
    /// Note that this only works correctly for the delegate's `push_directory()` and `pop_directory()` methods if
    /// `relative` paths are terminal, so point to their designated file or directory.
    /// The path is also expected to be normalized, and should not contain extra separators, and must not contain `..`
    /// or have leading or trailing slashes (or additionally backslashes on Windows).
    pub fn make_relative_path_current(
        &mut self,
        relative: impl ToNormalPathComponents,
        delegate: &mut dyn Delegate,
    ) -> std::io::Result<()> {
        let mut components = relative.to_normal_path_components().peekable();
        if self.valid_components != 0 && components.peek().is_none() {
            return Err(std::io::Error::other("empty inputs are not allowed"));
        }
        if self.valid_components == 0 {
            delegate.push_directory(self)?;
        }

        let mut existing_components = self.current_relative.components();
        let mut matching_components = 0;
        while let (Some(existing_comp), Some(new_comp)) = (existing_components.next(), components.peek()) {
            match new_comp {
                Ok(new_comp) => {
                    if existing_comp.as_os_str() == *new_comp {
                        components.next();
                        matching_components += 1;
                    } else {
                        break;
                    }
                }
                Err(err) => return Err(std::io::Error::other(err.to_string())),
            }
        }

        for _ in 0..self.valid_components - matching_components {
            self.current.pop();
            self.current_relative.pop();
            if self.current_is_directory {
                delegate.pop_directory();
            }
            self.current_is_directory = true;
        }
        self.valid_components = matching_components;

        if !self.current_is_directory && components.peek().is_some() {
            delegate.push_directory(self)?;
        }

        while let Some(comp) = components.next() {
            let comp = comp.map_err(|e| std::io::Error::other(e.into_error()))?;
            let is_last_component = components.peek().is_none();
            self.current_is_directory = !is_last_component;
            self.current.push(comp);
            self.current_relative.push(comp);
            self.valid_components += 1;
            let res = delegate.push(is_last_component, self);
            if self.current_is_directory {
                delegate.push_directory(self)?;
            }

            if let Err(err) = res {
                self.current.pop();
                self.current_relative.pop();
                self.valid_components -= 1;
                return Err(err);
            }
        }
        Ok(())
    }
}
