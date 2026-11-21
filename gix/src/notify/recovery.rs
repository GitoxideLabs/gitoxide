use std::time::Instant;

use super::RepositoryMonitor;

/// The state of all enabled native coverage, including additional linked worktrees.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CoverageStatus {
    /// No repository or application paths are subscribed to.
    #[default]
    Inactive,
    /// Subscriptions are being installed, updated or verified, without a pending failure retry.
    Updating,
    /// Every enabled subscription is installed and verified.
    Healthy,
    /// Coverage needs repair after a failure, loss, or an explicit rescan.
    Recovering {
        /// The earliest pending repair deadline, excluding unrelated inventory verification.
        ///
        /// `None` means no retry is scheduled. After successful repair, coverage can still be
        /// [`Updating`](Self::Updating) until verification finishes. Retain earlier diagnostics
        /// until [`Healthy`](Self::Healthy), rather than clearing them on an empty error list.
        retry_at: Option<Instant>,
    },
}

impl RepositoryMonitor {
    /// Current coverage state, including quiet recovery backoff and linked checkouts.
    ///
    /// Unlike an empty [`super::Outcome::errors`] list, [`CoverageStatus::Healthy`] certifies
    /// that every enabled subscription has been installed and verified.
    pub fn coverage_status(&self) -> CoverageStatus {
        if !self.active() {
            CoverageStatus::Inactive
        } else if self.needs_repair() {
            CoverageStatus::Recovering {
                retry_at: self.recovery_retry_at(),
            }
        } else if self.subscriptions_verified() {
            CoverageStatus::Healthy
        } else {
            CoverageStatus::Updating
        }
    }

    fn subscriptions_verified(&self) -> bool {
        self.maintenance.is_none()
            && (self.shared_metadata || !self.metadata_enabled() || self.metadata.watcher.is_some())
            && (!self.worktree_enabled() || self.worktree.watcher.is_some())
            && self.other_worktrees.values().all(Self::subscriptions_verified)
    }

    fn needs_repair(&self) -> bool {
        #[cfg(target_os = "macos")]
        if self.external_ignores.retry.is_some() {
            return true;
        }
        self.deferred_loss.is_some()
            || self.registration_needs_repair()
            || !self.undiscovered_worktrees.is_empty()
            || self.other_worktrees.values().any(Self::needs_repair)
    }

    fn registration_needs_repair(&self) -> bool {
        self.maintenance_failed
            || self.metadata.rebuild && self.metadata_enabled()
            || self.worktree.rebuild && self.worktree_enabled()
    }

    fn recovery_retry_at(&self) -> Option<Instant> {
        [
            self.maintenance.filter(|_| self.registration_needs_repair()),
            self.deferred_loss.as_ref().map(|loss| loss.deadline),
            #[cfg(target_os = "macos")]
            self.external_ignores.retry,
        ]
        .into_iter()
        .flatten()
        .chain(self.undiscovered_worktrees.values().copied())
        .chain(self.other_worktrees.values().filter_map(Self::recovery_retry_at))
        .min()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use gix_error::TestResult;

    use super::*;
    use crate::notify::Options;

    #[test]
    fn recovery_deadlines_exclude_unrelated_verification_and_include_undiscovered_checkouts() -> TestResult {
        let root = gix_testtools::scripted_fixture_read_only("make_notify_repo.sh")?.join("unignored");
        let repo = crate::open_opts(root, crate::open::Options::isolated())?;
        let mut monitor = repo.monitor(Options::default())?;
        let mut child = repo.monitor(Options::default())?;
        let now = Instant::now();
        let child_retry = now + Duration::from_secs(5);
        child.maintenance_failed = true;
        child.maintenance = Some(child_retry);
        monitor.maintenance = Some(now);
        monitor.other_worktrees.insert(child.layout.git_dir.clone(), child);
        assert_eq!(
            monitor.coverage_status(),
            CoverageStatus::Recovering {
                retry_at: Some(child_retry)
            },
            "ordinary parent verification does not advertise an earlier child repair"
        );
        #[cfg(target_os = "macos")]
        {
            let ignore_retry = now + Duration::from_secs(3);
            monitor.external_ignores.retry = Some(ignore_retry);
            assert_eq!(
                monitor.coverage_status(),
                CoverageStatus::Recovering {
                    retry_at: Some(ignore_retry)
                },
                "an external ignore retry contributes its own deadline"
            );
        }
        let discovery_retry = now + Duration::from_secs(2);
        monitor
            .undiscovered_worktrees
            .insert(monitor.layout.git_dir.join("worktrees/unavailable"), discovery_retry);
        assert_eq!(
            monitor.coverage_status(),
            CoverageStatus::Recovering {
                retry_at: Some(discovery_retry)
            },
            "unresolved linked checkouts contribute their own retry deadline"
        );
        monitor.maintenance_failed = true;
        assert_eq!(
            monitor.coverage_status(),
            CoverageStatus::Recovering { retry_at: Some(now) },
            "a degraded coordinator contributes its own earlier retry"
        );
        Ok(())
    }
}
