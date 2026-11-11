use std::{path::Path, time::Instant};

use gix::notify::{Batch, Budget, Fence, Options, SynchronizeError, Watch, WorktreeCoverage, WorktreeStream};

// Keep the protocol crate independent of repository discovery and porcelain dependencies.
pub(crate) struct Source(WorktreeStream);

impl Source {
    pub(crate) fn new(repo: &gix::Repository) -> gix::Result<Self> {
        Ok(Self(
            repo.monitor(Options {
                metadata: false,
                worktree_coverage: WorktreeCoverage::All,
                ..Options::default()
            })?
            .into_worktree_stream()?,
        ))
    }
}

impl gix_fsmonitor::server::EventSource for Source {
    fn register(&mut self, watches: &[Watch]) -> gix::Result<()> {
        self.0.register(watches)
    }

    fn drain(&mut self, budget: Budget) -> Batch {
        self.0.drain(budget)
    }

    fn synchronize_at(&mut self, directory: &Path, deadline: Instant) -> Result<Fence, SynchronizeError> {
        self.0.synchronize_at(directory, deadline)
    }

    fn supports_cookie_synchronization(&self) -> bool {
        cfg!(windows)
    }

    fn set_waker(&mut self, wake: Box<dyn Fn() + Send + Sync + 'static>) {
        self.0.set_waker(wake);
    }
}
