use crate::store;

/// A coordinated read view of a reference store.
///
/// A snapshot pins adapter-specific aggregate state, if any, without exposing
/// it to callers. It does not freeze independently stored state: for the files
/// backend, packed references remain fixed while loose references are read
/// live. For the reftable backend, the main and current-worktree stacks are
/// pinned immediately, while an explicitly addressed other-worktree stack is
/// pinned on its first access through this snapshot. A snapshot therefore is
/// not a transactional or MVCC view.
pub struct Snapshot<'store> {
    pub(super) state: State<'store>,
}

pub(super) enum State<'store> {
    Files {
        store: &'store crate::file::Store,
        packed: Option<crate::file::packed::SharedBufferSnapshot>,
    },
    Reftable {
        snapshot: crate::store_impl::reftable::Snapshot<'store>,
    },
}

/// The error returned when obtaining a [`Snapshot`].
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error(crate::store::BackendError);

impl crate::Store {
    /// Obtain a coordinated view of the store for related reads and iteration.
    pub fn snapshot(&self) -> Result<Snapshot<'_>, Error> {
        Ok(Snapshot {
            state: match &self.inner {
                store::State::Files { store } => State::Files {
                    store,
                    packed: store
                        .cached_packed_buffer()
                        .map_err(|err| Error(crate::store::BackendError::new("obtain a reference snapshot", err)))?,
                },
                store::State::Reftable { store } => State::Reftable {
                    snapshot: store.snapshot().map_err(|err| {
                        Error(crate::store::BackendError::new(
                            "obtain a stable reftable reference snapshot",
                            err,
                        ))
                    })?,
                },
            },
        })
    }
}
