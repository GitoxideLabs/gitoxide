use crate::{PathIdMapping, stack::State};

/// Various aggregate numbers related to the stack delegate itself.
#[derive(Default, Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Statistics {
    /// The amount of `std::fs::create_dir` calls.
    ///
    /// This only happens if we are in the respective mode to create leading directories efficiently.
    pub num_mkdir_calls: usize,
    /// Amount of calls to push a path element.
    pub push_element: usize,
    /// Amount of calls to push a directory.
    pub push_directory: usize,
    /// Amount of calls to pop a directory.
    pub pop_directory: usize,
}

pub(crate) struct StackDelegate<'a, 'find> {
    pub state: &'a mut State,
    pub buf: &'a mut Vec<u8>,
    #[cfg_attr(not(feature = "attributes"), allow(dead_code))]
    pub mode: Option<gix_index::entry::Mode>,
    pub id_mappings: &'a Vec<PathIdMapping>,
    pub objects: &'find dyn gix_object::Find,
    pub case: gix_glob::pattern::Case,
    pub statistics: &'a mut super::Statistics,
}

impl gix_fs::stack::Delegate for StackDelegate<'_, '_> {
    fn push_directory(&mut self, stack: &gix_fs::Stack) -> std::io::Result<()> {
        self.statistics.delegate.push_directory += 1;
        let rela_dir_bstr = gix_path::into_bstr(stack.current_relative());
        let rela_dir = gix_path::to_unix_separators_on_windows(rela_dir_bstr);
        match &mut self.state {
            #[cfg(feature = "attributes")]
            State::CreateDirectoryAndAttributesStack { attributes, .. } | State::AttributesStack(attributes) => {
                attributes.push_directory(
                    stack.root(),
                    stack.current(),
                    &rela_dir,
                    self.buf,
                    self.id_mappings,
                    self.objects,
                    &mut self.statistics.attributes,
                )?;
            }
            #[cfg(feature = "attributes")]
            State::AttributesAndIgnoreStack { ignore, attributes } => {
                attributes.push_directory(
                    stack.root(),
                    stack.current(),
                    &rela_dir,
                    self.buf,
                    self.id_mappings,
                    self.objects,
                    &mut self.statistics.attributes,
                )?;
                ignore.push_directory(
                    stack.root(),
                    stack.current(),
                    &rela_dir,
                    self.buf,
                    self.id_mappings,
                    self.objects,
                    self.case,
                    &mut self.statistics.ignore,
                )?;
            }
            State::IgnoreStack(ignore) => ignore.push_directory(
                stack.root(),
                stack.current(),
                &rela_dir,
                self.buf,
                self.id_mappings,
                self.objects,
                self.case,
                &mut self.statistics.ignore,
            )?,
        }
        Ok(())
    }

    #[cfg_attr(not(feature = "attributes"), allow(unused_variables))]
    fn push(&mut self, is_last_component: bool, stack: &gix_fs::Stack) -> std::io::Result<()> {
        self.statistics.delegate.push_element += 1;
        match &mut self.state {
            #[cfg(feature = "attributes")]
            State::CreateDirectoryAndAttributesStack {
                unlink_on_collision,
                validate,
                attributes: _,
            } => {
                validate_last_component(stack, self.mode, *validate)?;
                create_leading_directory(
                    is_last_component,
                    stack,
                    self.mode,
                    &mut self.statistics.delegate.num_mkdir_calls,
                    *unlink_on_collision,
                )?;
            }
            #[cfg(feature = "attributes")]
            State::AttributesAndIgnoreStack { .. } | State::AttributesStack(_) => {}
            State::IgnoreStack(_) => {}
        }
        Ok(())
    }

    fn pop_directory(&mut self) {
        self.statistics.delegate.pop_directory += 1;
        match &mut self.state {
            #[cfg(feature = "attributes")]
            State::CreateDirectoryAndAttributesStack { attributes, .. } | State::AttributesStack(attributes) => {
                attributes.pop_directory();
            }
            #[cfg(feature = "attributes")]
            State::AttributesAndIgnoreStack { attributes, ignore } => {
                attributes.pop_directory();
                ignore.pop_directory();
            }
            State::IgnoreStack(ignore) => {
                ignore.pop_directory();
            }
        }
    }
}

#[cfg(feature = "attributes")]
fn validate_last_component(
    stack: &gix_fs::Stack,
    mode: Option<gix_index::entry::Mode>,
    opts: gix_validate::path::component::Options,
) -> std::io::Result<()> {
    let Some(last_component) = stack.current_relative().components().next_back() else {
        return Ok(());
    };
    let last_component = gix_path::try_into_bstr(std::borrow::Cow::Borrowed(last_component.as_os_str().as_ref()))
        .map_err(|_err| {
            std::io::Error::other(format!(
                "Path component {last_component:?} of path \"{}\" contained invalid UTF-8 and could not be validated",
                stack.current_relative().display()
            ))
        })?;

    if let Err(err) = gix_validate::path::component(
        last_component.as_ref(),
        mode.and_then(|m| {
            (m == gix_index::entry::Mode::SYMLINK).then_some(gix_validate::path::component::Mode::Symlink)
        }),
        opts,
    ) {
        return Err(std::io::Error::other(err));
    }
    Ok(())
}

#[cfg(feature = "attributes")]
fn create_leading_directory(
    is_last_component: bool,
    stack: &gix_fs::Stack,
    mode: Option<gix_index::entry::Mode>,
    mkdir_calls: &mut usize,
    unlink_on_collision: bool,
) -> std::io::Result<()> {
    if is_last_component && !crate::stack::mode_is_dir(mode).unwrap_or(false) {
        return Ok(());
    }
    *mkdir_calls += 1;
    match std::fs::create_dir(stack.current()) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
            let meta = stack.current().symlink_metadata()?;
            if meta.is_dir() {
                Ok(())
            } else if unlink_on_collision {
                if meta.file_type().is_symlink() {
                    gix_fs::symlink::remove(stack.current())?;
                } else {
                    std::fs::remove_file(stack.current())?;
                }
                *mkdir_calls += 1;
                std::fs::create_dir(stack.current())
            } else {
                Err(err)
            }
        }
        Err(err) => Err(err),
    }
}
