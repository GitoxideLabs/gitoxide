use crate::{
    Repository, Result,
    error::{ResultExt, bail, message},
};

impl Repository {
    /// Start an application-owned filesystem monitor for this worktree.
    ///
    /// The returned handle owns the server thread and its watches, without retaining this repository.
    /// Startup waits for the endpoint and native watches to be ready, and fails if another server owns
    /// the endpoint. Dropping the handle or calling its `shutdown()` method stops the server and waits
    /// for native cleanup; perform this away from an application's UI thread.
    ///
    /// The endpoint is in this worktree's private Git directory, including for linked worktrees.
    /// Enable `core.fsmonitor=true`, for example through an in-memory configuration override, to have
    /// status calls query it. This method does not change configuration or launch another executable.
    pub fn fsmonitor_server(&self) -> Result<gix_fsmonitor::server::Handle> {
        let Some(worktree) = self.workdir() else {
            bail!(message("Filesystem monitoring requires a worktree"));
        };
        let resolve = |path: &std::path::Path| {
            self.current_dir()
                .join(path)
                .canonicalize()
                .or_raise(|| message!("Could not resolve filesystem monitor root {}", path.display()))
        };
        let worktree = resolve(worktree)?;
        let git_dir = resolve(self.git_dir())?;
        let common_dir = resolve(self.common_dir())?;
        let filesystem = self.filesystem_options()?;
        gix_fsmonitor::server::spawn(gix_fsmonitor::server::Context {
            worktree,
            endpoint: git_dir.join("fsmonitor--daemon.ipc"),
            git_dir,
            common_dir,
            precompose_unicode: filesystem.precompose_unicode,
            ignore_case: filesystem.ignore_case,
        })
    }
}
