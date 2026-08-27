use crate::{store, transaction::RefEdit};

/// A transaction against any built-in reference storage adapter.
pub struct Transaction<'store> {
    state: State<'store>,
}

enum State<'store> {
    Files(crate::file::Transaction<'store, 'store>),
}

/// A backend-neutral hint for how an adapter should physically organize
/// successfully updated direct references.
#[derive(Default)]
pub enum WriteStrategy<'a> {
    /// Preserve the adapter's normal storage behavior.
    #[default]
    Default,
    /// Prefer the adapter's compact representation for direct references.
    Compact {
        /// Object access used to compute peeled values when the representation stores them.
        objects: Box<dyn gix_object::Find + 'a>,
        /// Remove a redundant separately stored source reference after the compact form is durable.
        remove_separate_source: bool,
    },
}

impl crate::Store {
    /// Begin a reference transaction using the adapter's default write strategy.
    pub fn transaction(&self) -> Transaction<'_> {
        Transaction {
            state: match &self.inner {
                store::State::Files { store } => State::Files(store.transaction()),
            },
        }
    }
}

impl<'store> Transaction<'store> {
    /// Select how the adapter should physically organize direct reference updates.
    pub fn write_strategy(self, strategy: WriteStrategy<'store>) -> Self {
        Transaction {
            state: match self.state {
                State::Files(transaction) => State::Files(match strategy {
                    WriteStrategy::Default => transaction,
                    WriteStrategy::Compact {
                        objects,
                        remove_separate_source,
                    } => transaction.packed_refs(if remove_separate_source {
                        crate::file::transaction::PackedRefs::DeletionsAndNonSymbolicUpdatesRemoveLooseSourceReference(
                            objects,
                        )
                    } else {
                        crate::file::transaction::PackedRefs::DeletionsAndNonSymbolicUpdates(objects)
                    }),
                }),
            },
        }
    }

    /// Validate edits and acquire every lock needed to commit them.
    ///
    /// `individual_lock_fail` controls locks for separately addressable references,
    /// while `aggregate_lock_fail` controls an adapter's shared aggregate resource.
    pub fn prepare(
        self,
        edits: impl IntoIterator<Item = RefEdit>,
        individual_lock_fail: gix_lock::acquire::Fail,
        aggregate_lock_fail: gix_lock::acquire::Fail,
    ) -> Result<Self, prepare::Error> {
        Ok(Transaction {
            state: match self.state {
                State::Files(transaction) => State::Files(
                    transaction
                        .prepare(edits, individual_lock_fail, aggregate_lock_fail)
                        .map_err(|err| {
                            prepare::Error(crate::store::BackendError::new("prepare a reference transaction", err))
                        })?,
                ),
            },
        })
    }

    /// Make a prepared transaction durable and return the edits with their observed previous values.
    pub fn commit<'a>(
        self,
        committer: impl Into<Option<gix_actor::SignatureRef<'a>>>,
    ) -> Result<Vec<RefEdit>, commit::Error> {
        let committer = committer.into();
        match self.state {
            State::Files(transaction) => transaction
                .commit(committer)
                .map_err(|err| commit::Error(crate::store::BackendError::new("commit a reference transaction", err))),
        }
    }

    /// Roll back prepared state and return the edits as far as they were resolved.
    pub fn rollback(self) -> Vec<RefEdit> {
        match self.state {
            State::Files(transaction) => transaction.rollback(),
        }
    }
}

/// Transaction preparation errors.
pub mod prepare {
    /// The error returned by [`super::Transaction::prepare()`].
    #[derive(Debug, thiserror::Error)]
    #[error(transparent)]
    pub struct Error(pub(super) crate::store::BackendError);
}

/// Transaction commit errors.
pub mod commit {
    /// The error returned by [`super::Transaction::commit()`].
    #[derive(Debug, thiserror::Error)]
    #[error(transparent)]
    pub struct Error(pub(super) crate::store::BackendError);
}
