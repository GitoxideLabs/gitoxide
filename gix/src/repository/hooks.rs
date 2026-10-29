use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::error::{ErrorExt, OptionExt, bail, ensure, message, permission_denied, validation};
use crate::{
    Repository, Result,
    config::{self, tree::Core},
};

impl Repository {
    /// Find an executable traditional hook called `name`, without running it.
    ///
    /// Default hooks are read from `hooks` in the common Git directory, including when called from a linked worktree.
    /// Trusted `core.hooksPath` values replace that directory. Relative values, including the empty path, are resolved
    /// against the worktree root, or the Git directory in a bare repository. Returned paths are absolute and retain
    /// symlinks. Missing and non-executable files are ignored; other filesystem errors are returned.
    /// On Windows, regular files are executable candidates, with `.exe` tried after a missing extensionless hook.
    ///
    /// `name` must be a single filename, without path separators, NUL bytes, or a Windows drive prefix.
    /// Repository hooks require full repository trust. This method only discovers traditional hooks;
    /// it does not inspect configured `hook.<name>.event` commands or run receive-side hooks with a different directory.
    pub fn find_hook(&self, name: &str) -> Result<Option<PathBuf>> {
        gix_hook::validate_name(name)?;
        self.require_hook_trust()?;
        gix_hook::find(name, &self.hooks_directory()?)
    }

    /// Prepare the traditional hook at `path` for execution with this repository's environment and working directory.
    ///
    /// Relative paths start at the worktree root, or the Git directory in a bare repository. Repository paths are
    /// anchored to the working directory captured when this repository was opened. `GIT_COMMON_DIR` names this repository,
    /// and bare repositories explicitly unset inherited `GIT_WORK_TREE`. Arguments and I/O can be set on
    /// the returned command. By default, stdin is closed, stdout is captured, and stderr follows
    /// [`command_context()`][Self::command_context()]. Per-hook variables, such as `GIT_INDEX_FILE` and `GIT_EDITOR`, remain caller choices.
    ///
    /// Full repository trust is required. The path is used literally, without shell interpretation or an executable
    /// check. Windows shebang handling follows [`gix_command::prepare()`]. This only prepares a command and does not
    /// implement Git's execution-time shell fallback for executable scripts without a shebang; use
    /// [`run_hook()`][Self::run_hook()] for that.
    pub fn prepare_hook(&self, path: impl AsRef<Path>) -> Result<Command> {
        self.require_hook_trust()?;
        let working_dir = self.current_dir().join(self.workdir().unwrap_or(self.git_dir()));
        gix_hook::prepare(
            path.as_ref(),
            &gix_hook::Context {
                working_dir: &working_dir,
                common_dir: &self.current_dir().join(self.common_dir()),
                command: self.command_context()?,
            },
            Default::default(),
        )
    }

    /// Execute the traditional hook at `path`, capturing its stdout, stderr, and exit status.
    ///
    /// This uses [`prepare_hook()`][Self::prepare_hook()] for repository paths, environment, and trust checks.
    /// Arguments and environment values in `options` are passed literally. On Unix, if the operating system rejects
    /// the executable format, the command is retried through Git's shell, supporting scripts without a shebang.
    /// Like Git, Windows supports shebang scripts through [`gix_command::prepare()`] without an execution-time retry.
    /// Other launch failures, including permission errors and missing interpreters, are returned without retrying.
    ///
    /// Input is written concurrently with output collection. A hook may close stdin early; a broken pipe is ignored
    /// so its exit status and output remain available. Failed hook exit statuses are returned in the output, while
    /// launch and communication failures are errors. The caller decides how a failed hook affects its operation.
    pub fn run_hook(&self, path: impl AsRef<Path>, options: crate::hook::Options<'_>) -> Result<std::process::Output> {
        self.require_hook_trust()?;
        let working_dir = self.current_dir().join(self.workdir().unwrap_or(self.git_dir()));
        gix_hook::run(
            path.as_ref(),
            &gix_hook::Context {
                working_dir: &working_dir,
                common_dir: &self.current_dir().join(self.common_dir()),
                command: self.command_context()?,
            },
            options,
        )
    }

    fn require_hook_trust(&self) -> Result<()> {
        ensure!(
            self.git_dir_trust() == gix_sec::Trust::Full,
            permission_denied("Repository hooks require full repository trust")
        );
        Ok(())
    }

    /// Return the absolute directory for traditional hook files, whether or not it exists.
    ///
    /// Trusted `core.hooksPath` values take precedence over `hooks` in the common Git directory.
    /// Relative paths start at the worktree root, or the Git directory in a bare repository.
    /// Optional missing paths fall back to the default directory; invalid configuration and other
    /// filesystem errors are returned. Symlinks are retained.
    ///
    /// This resolves the directory only. Use [`find_hook()`][Self::find_hook()] to validate a hook name,
    /// check repository trust, and find an executable hook.
    pub fn hooks_directory(&self) -> Result<PathBuf> {
        let default = self.current_dir().join(self.common_dir()).join("hooks");
        let Some(value) = self
            .config
            .resolved
            .string_implicit_filter(Core::HOOKS_PATH, self.filter_config_section())
        else {
            return Ok(default);
        };
        let value = value.ok_or_raise(|| validation("core.hooksPath requires a value"))?;
        let path = gix_config::Path::from(value);
        let optional = path.is_optional;
        if optional && path.is_empty() {
            return Ok(default);
        }
        let install_dir = crate::path::install_dir().ok();
        let home = config::cache::access::home_dir(self.open_options().permissions.env);
        let context = config::cache::interpolate_context(install_dir.as_deref(), home.as_deref());
        let path = self
            .current_dir()
            .join(self.workdir().unwrap_or(self.git_dir()))
            .join(if path.is_empty() {
                PathBuf::new()
            } else {
                path.interpolate(context)?
            });
        if optional {
            match path.metadata() {
                Ok(_) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(default),
                Err(err) => {
                    bail!(err.and_raise(message("Could not inspect optional core.hooksPath")))
                }
            }
        }
        Ok(path)
    }
}
