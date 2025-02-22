use std::{
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
};

use crate::config::{CommitAutoRollback, Snapshot, SnapshotMut};

impl Debug for Snapshot<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.repo.config.resolved.to_string())
    }
}

impl Debug for CommitAutoRollback<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.repo.as_ref().expect("still present").config.resolved.to_string())
    }
}

impl Debug for SnapshotMut<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.config.to_string())
    }
}

impl Drop for SnapshotMut<'_> {
    fn drop(&mut self) {
        if let Some(repo) = self.repo.take() {
            self.commit_inner(repo).ok();
        }
    }
}

impl Drop for CommitAutoRollback<'_> {
    fn drop(&mut self) {
        if let Some(repo) = self.repo.take() {
            self.rollback_inner(repo).ok();
        }
    }
}

impl Deref for SnapshotMut<'_> {
    type Target = gix_config::File<'static>;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

impl Deref for Snapshot<'_> {
    type Target = gix_config::File<'static>;

    fn deref(&self) -> &Self::Target {
        self.plumbing()
    }
}

impl Deref for CommitAutoRollback<'_> {
    type Target = crate::Repository;

    fn deref(&self) -> &Self::Target {
        self.repo.as_ref().expect("always present")
    }
}

impl DerefMut for CommitAutoRollback<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.repo.as_mut().expect("always present")
    }
}

impl DerefMut for SnapshotMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.config
    }
}
