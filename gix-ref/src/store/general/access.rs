use std::path::Path;

use crate::{FullNameRef, Namespace, store, store::WriteReflog};

impl crate::Store {
    /// Return the worktree-local Git directory used by this store.
    pub fn git_dir(&self) -> &Path {
        match &self.inner {
            store::State::Files { store } => store.git_dir(),
            store::State::Reftable { store } => store.git_dir(),
        }
    }

    /// Return the separate common Git directory for a linked worktree.
    pub fn common_dir(&self) -> Option<&Path> {
        match &self.inner {
            store::State::Files { store } => store.common_dir(),
            store::State::Reftable { store } => store.common_dir(),
        }
    }

    /// Return the common Git directory, or the worktree-local Git directory when they are the same.
    pub fn common_dir_resolved(&self) -> &Path {
        match &self.inner {
            store::State::Files { store } => store.common_dir_resolved(),
            store::State::Reftable { store } => store.common_dir_resolved(),
        }
    }

    /// Return the configured reference namespace.
    pub fn namespace(&self) -> Option<&Namespace> {
        match &self.inner {
            store::State::Files { store } => store.namespace.as_ref(),
            store::State::Reftable { store } => store.namespace.as_ref(),
        }
    }

    /// Replace the reference namespace and return the previous one.
    pub fn replace_namespace(&mut self, namespace: Option<Namespace>) -> Option<Namespace> {
        match &mut self.inner {
            store::State::Files { store } => std::mem::replace(&mut store.namespace, namespace),
            store::State::Reftable { store } => std::mem::replace(&mut store.namespace, namespace),
        }
    }

    /// Return the configured reflog write policy.
    pub fn write_reflog(&self) -> WriteReflog {
        match &self.inner {
            store::State::Files { store } => store.write_reflog,
            store::State::Reftable { store } => store.write_reflog,
        }
    }

    /// Set the reflog write policy and return the previous value.
    pub fn set_write_reflog(&mut self, write_reflog: WriteReflog) -> WriteReflog {
        match &mut self.inner {
            store::State::Files { store } => std::mem::replace(&mut store.write_reflog, write_reflog),
            store::State::Reftable { store } => std::mem::replace(&mut store.write_reflog, write_reflog),
        }
    }

    /// Discard in-memory reference caches so a subsequent read observes changes made to the
    /// backend by another process, such as `git gc` or `git pack-refs`, without depending on
    /// filesystem modification times.
    ///
    /// The files backend refreshes its `packed-refs` buffer. The reftable backend does nothing,
    /// as each snapshot re-reads `tables.list` and validates its generation.
    pub fn force_refresh(&self) -> Result<(), store::BackendError> {
        match &self.inner {
            store::State::Files { store } => store
                .force_refresh_packed_buffer()
                .map_err(|err| store::BackendError::new("refresh reference storage", err)),
            store::State::Reftable { .. } => Ok(()),
        }
    }

    /// Return whether the reference store still has its freshly initialized observable state.
    ///
    /// `None` means the backend has no authoritative `HEAD` from which to determine the state.
    /// Backend access and validation failures are returned instead of being treated as uncertainty.
    pub fn is_pristine(&self, default_ref: &FullNameRef) -> Result<Option<bool>, store::BackendError> {
        match &self.inner {
            store::State::Files { store } => Ok(store.is_pristine(default_ref)),
            store::State::Reftable { store } => {
                let snapshot = store
                    .snapshot()
                    .map_err(|err| store::BackendError::new("inspect pristine reftable state", err))?;
                let head_name = "HEAD".try_into().expect("HEAD is a valid full reference name");
                let Some(head) = snapshot.find_full(head_name).map_err(|err| {
                    store::BackendError::new("find reftable HEAD while inspecting pristine state", err)
                })?
                else {
                    return Ok(None);
                };
                if head.target.try_name() != Some(default_ref) {
                    return Ok(Some(false));
                }
                let mut references = snapshot.all().into_iter();
                match references.next() {
                    Some(reference) => {
                        reference.map_err(|err| {
                            store::BackendError::new("inspect references in a pristine reftable state", err)
                        })?;
                        Ok(Some(false))
                    }
                    None => Ok(Some(true)),
                }
            }
        }
    }
}
