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
    pub reject_temrinal_symlinks: bool,
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
                    self.reject_temrinal_symlinks,
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
    use gix_error::ResultExt;

    let Some(last_component) = stack.current_relative().components().next_back() else {
        return Ok(());
    };
    let last_component = gix_path::try_into_bstr(std::borrow::Cow::Borrowed(last_component.as_os_str().as_ref()))
        .or_raise(|| {
            gix_error::message!(
                "Path component {last_component:?} of path \"{}\" contained invalid UTF-8 and could not be validated",
                stack.current_relative().display()
            )
        })
        .map_err(|err| std::io::Error::other(err.into_error()))?;

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
    #[cfg_attr(not(windows), allow(unused_variables))] check_terminal_symlinks: bool,
) -> std::io::Result<()> {
    if is_last_component && !crate::stack::mode_is_dir(mode).unwrap_or(false) {
        #[cfg(not(windows))]
        {
            return Ok(());
        }
        #[cfg(windows)]
        {
            // Forced checkout delegates terminal symlink detection and removal to the caller immediately before
            // replacement, avoiding a redundant check here. Callers combining these flags must uphold that no-follow
            // contract; this stack only protects leading components in that mode.
            if unlink_on_collision || !check_terminal_symlinks {
                return Ok(());
            }
            return match stack.current().symlink_metadata() {
                Ok(meta) if meta.file_type().is_symlink() => Err(std::io::ErrorKind::AlreadyExists.into()),
                Ok(_) => Ok(()),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(err) => Err(err),
            };
        }
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
