//! Discover and run traditional Git hooks.
//!
//! Callers provide resolved repository paths and command context.
//! This crate does not discover repositories or decide repository trust.
#![deny(missing_docs)]
#![forbid(unsafe_code)]

use gix_error::{Result, ResultExt, ensure, message, validation};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

mod run;
pub use run::{prepare, run};

/// Literal arguments, environment overrides, and input for a hook invocation.
#[derive(Debug, Default, Clone, Copy)]
pub struct Options<'a> {
    /// Arguments passed literally after the hook path.
    pub arguments: &'a [&'a OsStr],
    /// Environment entries that override inherited values and the repository's command context.
    pub environment: &'a [(&'a OsStr, &'a OsStr)],
    /// Bytes written to the hook's standard input. `None` provides a closed input stream.
    pub stdin: Option<&'a [u8]>,
}

/// Resolved execution context for a traditional hook.
///
/// Directory paths must be absolute. Repository trust and configuration interpolation are the
/// caller's responsibility. An absent `command.worktree_dir` represents a bare repository and
/// removes inherited `GIT_WORK_TREE`, unless explicitly overridden in [`Options::environment`].
#[derive(Debug, Clone)]
pub struct Context<'a> {
    /// The directory in which hooks run, selected by the caller for the current event.
    pub working_dir: &'a Path,
    /// The repository's common Git directory, exported as `GIT_COMMON_DIR`.
    pub common_dir: &'a Path,
    /// Git environment settings, including absolute Git and worktree directory paths.
    pub command: gix_command::Context,
}

/// Find an executable traditional hook `name` in the resolved `directory`.
///
/// Missing and non-executable files are ignored; other filesystem errors are returned. On Windows,
/// regular files are executable candidates and `.exe` is tried after a missing extensionless hook.
/// Symlinks are retained and the directory is not canonicalized. [`validate_name()`] prevents path escape.
pub fn find(name: &str, directory: &Path) -> Result<Option<PathBuf>> {
    validate_name(name)?;
    let hook = directory.join(name);
    if executable(&hook)? {
        return Ok(Some(hook));
    }
    #[cfg(windows)]
    {
        let mut executable_path = hook.into_os_string();
        executable_path.push(".exe");
        let executable_path = PathBuf::from(executable_path);
        if executable(&executable_path)? {
            return Ok(Some(executable_path));
        }
    }
    Ok(None)
}

fn executable(path: &Path) -> Result<bool> {
    gix_sec::identity::is_path_executable_by_current_user(path)
        .or_raise(|| message!("Could not inspect hook at {path:?}"))
}

/// Reject empty names, directory components, NUL bytes, and Windows drive prefixes.
pub fn validate_name(name: &str) -> Result<()> {
    ensure!(
        !name.is_empty() && !matches!(name, "." | "..") && !name.contains(['/', '\\', '\0', ':']),
        validation("A hook name must be a single filename")
    );
    Ok(())
}
