use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use crate::{Stack, SymlinkCheck, stack::ToNormalPathComponents};
use bstr::BStr;
use gix_error::Message;

const CANNOT_STEP_THROUGH_SYMLINK: &str = "Cannot step through symlink to perform an lstat";

impl SymlinkCheck {
    /// Return whether `err` reports a parent symlink rejected by this checker.
    pub fn is_symlink_step_error(err: &std::io::Error) -> bool {
        err.get_ref()
            .and_then(|source| source.downcast_ref::<Message>())
            .is_some_and(|err| gix_error::classify(err).is_validation() && err.message == CANNOT_STEP_THROUGH_SYMLINK)
    }

    /// Create a new stack that starts operating at `root`.
    pub fn new(root: PathBuf) -> Self {
        Self {
            inner: Stack::new(root),
        }
    }

    /// Return a valid filesystem path located in our root by appending `relative_path`, which is guaranteed to
    /// not pass through a symbolic link. That way the caller can be sure to not be misled by an attacker that
    /// tries to make us reach outside of the repository.
    ///
    /// Note that the file pointed to by `relative_path` may still be a symbolic link, or not exist at all,
    /// and that an error may also be produced if directories on the path leading to the leaf
    /// component of `relative_path` are missing.
    ///
    /// ### Note
    ///
    /// On windows, no verification is performed, instead only the combined path is provided as usual.
    pub fn verified_path(&mut self, relative_path: impl ToNormalPathComponents) -> std::io::Result<&Path> {
        self.inner.make_relative_path_current(relative_path, &mut Delegate)?;
        Ok(self.inner.current())
    }

    /// Like [`Self::verified_path()`], but do not fail if there is no directory entry at `relative_path` or on the way
    /// to `relative_path`. Instead.
    /// For convenience, this incarnation is tuned to be easy to use with Git paths, i.e. slash-separated `BString` path.
    pub fn verified_path_allow_nonexisting(&mut self, relative_path: &BStr) -> std::io::Result<Cow<'_, Path>> {
        let rela_path = gix_path::from_bstr(relative_path)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
        if let Err(err) = self.verified_path(rela_path.as_ref()) {
            if err.kind() == std::io::ErrorKind::NotFound {
                Ok(Cow::Owned(self.inner.root().join(rela_path)))
            } else {
                Err(err)
            }
        } else {
            Ok(Cow::Borrowed(self.inner.current()))
        }
    }
}

struct Delegate;

impl crate::stack::Delegate for Delegate {
    fn push_directory(&mut self, _stack: &Stack) -> std::io::Result<()> {
        Ok(())
    }

    #[cfg_attr(windows, allow(unused_variables))]
    fn push(&mut self, is_last_component: bool, stack: &Stack) -> std::io::Result<()> {
        #[cfg(windows)]
        {
            Ok(())
        }
        #[cfg(not(windows))]
        {
            if is_last_component {
                return Ok(());
            }

            if stack.current().symlink_metadata()?.is_symlink() {
                return Err(std::io::Error::other(
                    gix_error::message!("{}", CANNOT_STEP_THROUGH_SYMLINK).validation(),
                ));
            }
            Ok(())
        }
    }

    fn pop_directory(&mut self) {}
}
