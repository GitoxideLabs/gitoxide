use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use bstr::BStr;
use gix_fs::{Stack, stack::ToNormalPathComponents};

use crate::SymlinkCheck;

#[derive(Debug, thiserror::Error)]
#[error("Cannot step through symlink to perform an lstat")]
struct CannotStepThroughSymlink;

pub(crate) fn is_symlink_step_error(err: &std::io::Error) -> bool {
    err.get_ref()
        .and_then(|source| source.downcast_ref::<CannotStepThroughSymlink>())
        .is_some()
}

impl SymlinkCheck {
    /// Create a new stack that starts operating at `root`.
    pub fn new(root: PathBuf) -> Self {
        Self {
            inner: gix_fs::Stack::new(root),
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
        let rela_path = gix_path::try_from_bstr(relative_path).map_err(std::io::Error::other)?;
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

impl gix_fs::stack::Delegate for Delegate {
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
                return Err(std::io::Error::other(CannotStepThroughSymlink));
            }
            Ok(())
        }
    }

    fn pop_directory(&mut self) {}
}
