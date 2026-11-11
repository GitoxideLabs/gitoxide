use crate::{
    Repository, Result,
    error::{OptionExt, ResultExt, message},
    fsmonitor::server,
};

impl Repository {
    /// Resolve canonical repository paths, filesystem capabilities, and Git's native daemon endpoint.
    ///
    /// Local Git directories use `fsmonitor--daemon.ipc`, ignoring `fsmonitor.socketDir`. On
    /// supported Unix platforms, remote Git directories use the trusted, interpolated socket
    /// directory or the permitted home directory. Bare repositories and path/configuration errors
    /// are rejected. No server is started and no repository handle is retained in the context.
    #[cfg(feature = "fsmonitor-daemon")]
    pub fn fsmonitor_daemon_context(&self) -> Result<server::Context> {
        let worktree = self
            .workdir()
            .ok_or_raise(|| message("filesystem monitoring requires a worktree"))?;
        let worktree = self
            .current_dir()
            .join(worktree)
            .canonicalize()
            .or_raise(|| message("resolving the filesystem monitor worktree"))?;
        let git_dir = self.current_dir().join(self.git_dir()).canonicalize().or_error()?;
        let endpoint = crate::fsmonitor::daemon::endpoint(&git_dir, &worktree, || {
            match self
                .config_snapshot()
                .trusted_path(crate::config::tree::FsMonitor::SOCKET_DIR)?
            {
                Some(path) => Ok(path),
                None => self.interpolate_path("~"),
            }
        })?;
        let capabilities = self.filesystem_options()?;
        Ok(server::Context {
            worktree,
            git_dir,
            common_dir: self.current_dir().join(self.common_dir()).canonicalize().or_error()?,
            endpoint,
            precompose_unicode: capabilities.precompose_unicode,
            ignore_case: capabilities.ignore_case,
        })
    }

    /// Resolve daemon policy and startup timing from repository configuration.
    ///
    /// `fsmonitor.allowRemote` defaults to false. `fsmonitor.startTimeout` is a nonnegative
    /// number of seconds, defaulting to 60; zero allows one readiness check without waiting.
    /// Only trusted configuration sections are used. Invalid values are rejected with
    /// configuration-key context. No filesystem is probed.
    #[cfg(feature = "fsmonitor-daemon")]
    pub fn fsmonitor_daemon_options(&self) -> Result<crate::fsmonitor::daemon::Options> {
        use crate::config::tree::FsMonitor;

        let defaults = crate::fsmonitor::daemon::Options::default();
        let mut trusted = self.filter_config_section();
        Ok(crate::fsmonitor::daemon::Options {
            allow_remote: FsMonitor::ALLOW_REMOTE
                .enrich_error(
                    self.config
                        .resolved
                        .boolean_filter(FsMonitor::ALLOW_REMOTE, &mut trusted),
                )?
                .unwrap_or(defaults.allow_remote),
            start_timeout: FsMonitor::START_TIMEOUT
                .try_into_u64(
                    self.config
                        .resolved
                        .integer_filter(FsMonitor::START_TIMEOUT, &mut trusted),
                )?
                .map_or(defaults.start_timeout, std::time::Duration::from_secs),
        })
    }

    /// Execute a foreground, detached-startup, or client filesystem-monitor command.
    ///
    /// Captures [`fsmonitor_daemon_context()`](Self::fsmonitor_daemon_context) and
    /// [`fsmonitor_daemon_options()`](Self::fsmonitor_daemon_options), then drops this repository
    /// before serving or launching a child, retaining no repository caches.
    /// `should_interrupt` supplies cancellation during serving and startup.
    ///
    /// # Embedding a daemon
    ///
    /// For an application with a `--repository PATH fsmonitor run` foreground command:
    ///
    /// ```no_run
    /// use gix::fsmonitor::daemon::Command;
    ///
    /// fn fsmonitor(repo: gix::Repository, command: Command) -> gix::Result<()> {
    ///     repo.fsmonitor_daemon_execute(
    ///         command,
    ///         std::io::stdout().lock(),
    ///         || false,
    ///         |context| {
    ///             Ok(vec![
    ///                 "--repository".into(),
    ///                 context.worktree.as_os_str().to_owned(),
    ///                 "fsmonitor".into(),
    ///                 "run".into(),
    ///             ])
    ///         },
    ///     )
    /// }
    /// ```
    ///
    /// Replace `|| false` with the application's cancellation check when needed. Select `sha1`
    /// and/or `sha256` features for the object hashes the application needs.
    ///
    /// `child_arguments` returns an **argument prefix**, not an executable path: the launcher
    /// reexecutes the **current executable**, without requiring a standalone daemon binary.
    /// The prefix must select that application's foreground `run` command and preserve any
    /// application-specific options or configuration needed by the child. The launcher preserves
    /// this repository's layout and conditional configuration, and appends `--idle-timeout` and
    /// `--ready-file`; do not include either option in the prefix. The child must pass both to
    /// [`Command::Run`](crate::fsmonitor::daemon::Command::Run), which reports its PID only after
    /// server startup succeeds. Detached startup waits for that child's readiness notification,
    /// not just an existing endpoint, and cleans up unsuccessful children. The callback is called
    /// at most once, only when starting a child; commands such as `Path` do not invoke it.
    ///
    /// See [`daemon::execute()`](crate::fsmonitor::daemon::execute) for output, startup readiness,
    /// cleanup, and hook autostart behavior.
    #[cfg(feature = "fsmonitor-daemon")]
    pub fn fsmonitor_daemon_execute(
        self,
        command: crate::fsmonitor::daemon::Command,
        out: impl std::io::Write,
        should_interrupt: impl FnMut() -> bool,
        child_arguments: impl FnOnce(&server::Context) -> Result<Vec<std::ffi::OsString>>,
    ) -> Result<()> {
        let reopen_git_dir = self.current_dir().join(self.git_dir());
        let context = self.fsmonitor_daemon_context()?;
        let options = if matches!(command, crate::fsmonitor::daemon::Command::Path) {
            Default::default()
        } else {
            self.fsmonitor_daemon_options()?
        };
        drop(self);
        crate::fsmonitor::daemon::execute(
            context,
            &reopen_git_dir,
            command,
            options,
            out,
            should_interrupt,
            child_arguments,
        )
    }

    /// Start an application-owned filesystem monitor serving this worktree to Git clients.
    ///
    /// The server registers its native watches before returning and retains no repository handle.
    /// Dropping the returned handle stops it; `options` can also configure shutdown after client
    /// inactivity. An occupied endpoint or a repository without a worktree is an error.
    ///
    /// The endpoint is `fsmonitor--daemon.ipc` in this worktree's private Git directory. Alternate
    /// socket locations for remote Git directories are not selected by this helper.
    /// Repository configuration is unchanged. Configure external Git clients separately to use
    /// the server; this does not enable filesystem-monitor acceleration in `gix` status operations.
    pub fn fsmonitor_server(&self, options: server::Options) -> Result<server::Handle> {
        let worktree = self
            .workdir()
            .ok_or_raise(|| message("filesystem monitoring requires a worktree"))?;
        let git_dir = self.current_dir().join(self.git_dir());
        let capabilities = self.filesystem_options()?;
        server::spawn(
            server::Context {
                worktree: self.current_dir().join(worktree),
                endpoint: git_dir.join("fsmonitor--daemon.ipc"),
                git_dir,
                common_dir: self.current_dir().join(self.common_dir()),
                precompose_unicode: capabilities.precompose_unicode,
                ignore_case: capabilities.ignore_case,
            },
            options,
        )
        .or_raise(|| message("starting the repository's filesystem monitor"))
    }
}
