use crate::{
    Repository, Result,
    error::{OptionExt, ResultExt, message},
    fsmonitor::server,
};

impl Repository {
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
        server::spawn_with_options(
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
