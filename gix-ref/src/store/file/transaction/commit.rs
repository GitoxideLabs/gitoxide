use crate::{
    store_impl::file::{transaction::PackedRefs, Transaction},
    transaction::{Change, LogChange, RefEdit, RefLog},
    Target,
};

impl Transaction<'_, '_> {
    /// Make all [prepared][Transaction::prepare()] permanent and return the performed edits which represent the current
    /// state of the affected refs in the ref store in that instant. Please note that the obtained edits may have been
    /// adjusted to contain more dependent edits or additional information.
    /// `committer` is used in the reflog and only if the reflog is actually written, which is why it is optional. Please note
    /// that if `None` is passed and the reflog needs to be written, the operation will be aborted late and a few refs may have been
    /// successfully committed already, making clear the non-atomic nature of multi-file edits.
    ///
    /// On error the transaction may have been performed partially, depending on the nature of the error, and no attempt to roll back
    /// partial changes is made.
    ///
    /// In this stage, we perform the following operations:
    ///
    /// * update the ref log
    /// * move updated refs into place
    /// * delete reflogs and empty parent directories
    /// * delete packed refs
    /// * delete their corresponding reference (if applicable)
    ///   along with empty parent directories
    ///
    /// Note that transactions will be prepared automatically as needed.
    pub fn commit<'a>(self, committer: impl Into<Option<gix_actor::SignatureRef<'a>>>) -> Result<Vec<RefEdit>, Error> {
        self.commit_inner(committer.into())
    }

    fn commit_inner(self, committer: Option<gix_actor::SignatureRef<'_>>) -> Result<Vec<RefEdit>, Error> {
        let mut updates = self.updates.expect("BUG: must call prepare before commit");
        let delete_loose_refs = matches!(
            self.packed_refs,
            PackedRefs::DeletionsAndNonSymbolicUpdatesRemoveLooseSourceReference(_)
        );

        // Perform updates first so live commits remain referenced
        for change in &mut updates {
            assert!(!change.update.deref, "Deref mode is turned into splits and turned off");
            match &change.update.change {
                // reflog first, then reference
                Change::Update { log, new, expected } => {
                    let lock = change.lock.take();
                    let (update_ref, update_reflog) = match log.mode {
                        RefLog::Only => (false, true),
                        RefLog::AndReference => (true, true),
                    };
                    if update_reflog {
                        let log_update = match new {
                            Target::Symbolic(_) => {
                                // Special HACK: no reflog for symref changes as there is no OID involved which the reflog needs.
                                // Unless, the ref is new and we can obtain a peeled id
                                // identified by the expectation of what could be there, as is the case when cloning.
                                match expected {
                                    PreviousValue::ExistingMustMatch(Target::Object(oid)) => {
                                        Some((Some(gix_hash::ObjectId::null(oid.kind())), oid))
                                    }
                                    _ => None,
                                }
                            }
                            Target::Object(new_oid) => {
                                let previous = match expected {
                                    // Here, this means that the ref already existed, and that it will receive (even transitively)
                                    // the given value
                                    PreviousValue::MustExistAndMatch(Target::Object(oid)) => Some(oid.to_owned()),
                                    _ => None,
                                }
                                .or(change.leaf_referent_previous_oid);
                                Some((previous, new_oid))
                            }
                        };
                        if let Some((previous, new_oid)) = log_update {
                            let do_update = previous.as_ref() != Some(new_oid);
                            if do_update {
                                self.store.reflog_create_or_append(
                                    change.update.name.as_ref(),
                                    previous,
                                    new_oid,
                                    committer,
                                    log.message.as_ref(),
                                    log.force_create_reflog,
                                )?;
                            }
                        }
                    }
                    // Don't do anything else while keeping the lock after potentially updating the reflog.
                    // We delay deletion of the reference and dropping the lock to after the packed-refs were
                    // safely written.
                    if delete_loose_refs && matches!(new, Target::Object(_)) {
                        change.lock = lock;
                        continue;
                    }
                    if update_ref {
                        if let Some(Err(err)) = lock.map(gix_lock::Marker::commit) {
                            // TODO: when Kind::IsADirectory becomes stable, use that.
                            let err = if err.instance.resource_path().is_dir() {
                                gix_tempfile::remove_dir::empty_depth_first(err.instance.resource_path())
                                    .map_err(|io_err| std::io::Error::new(std::io::ErrorKind::Other, io_err))
                                    .and_then(|_| err.instance.commit().map_err(|err| err.error))
                                    .err()
                            } else {
                                Some(err.error)
                            };

                            if let Some(err) = err {
                                return Err(Error::LockCommit {
                                    source: err,
                                    full_name: change.name(),
                                });
                            }
                        }
                    }
                }
                Change::Delete { .. } => {}
            }
        }

        for change in &mut updates {
            let (reflog_root, relative_name) = self.store.reflog_base_and_relative_path(change.update.name.as_ref());
            match &change.update.change {
                Change::Update { .. } => {}
                Change::Delete { .. } => {
                    // Reflog deletion happens first in case it fails a ref without log is less terrible than
                    // a log without a reference.
                    let reflog_path = reflog_root.join(relative_name);
                    if let Err(err) = std::fs::remove_file(&reflog_path) {
                        if err.kind() != std::io::ErrorKind::NotFound {
                            return Err(Error::DeleteReflog {
                                source: err,
                                full_name: change.name(),
                            });
                        }
                    } else {
                        gix_tempfile::remove_dir::empty_upward_until_boundary(
                            reflog_path.parent().expect("never without parent"),
                            &reflog_root,
                        )
                        .ok();
                    }
                }
            }
        }

        if let Some(t) = self.packed_transaction {
            t.commit().map_err(Error::PackedTransactionCommit)?;
            // Always refresh ourselves right away to avoid races. We ignore errors as there may be many reasons this fails, and it's not
            // critical to be done here. In other words, the pack may be refreshed at a later time and then it might work.
            self.store.force_refresh_packed_buffer().ok();
        }

        for change in &mut updates {
            let take_lock_and_delete = match &change.update.change {
                Change::Update {
                    log: LogChange { mode, .. },
                    new,
                    ..
                } => delete_loose_refs && *mode == RefLog::AndReference && matches!(new, Target::Object(_)),
                Change::Delete { log: mode, .. } => *mode == RefLog::AndReference,
            };
            if take_lock_and_delete {
                let lock = change.lock.take();
                let reference_path = self.store.reference_path(change.update.name.as_ref());
                if let Err(err) = std::fs::remove_file(reference_path) {
                    if err.kind() != std::io::ErrorKind::NotFound {
                        return Err(Error::DeleteReference {
                            err,
                            full_name: change.name(),
                        });
                    }
                }
                drop(lock);
            }
        }
        Ok(updates.into_iter().map(|edit| edit.update).collect())
    }
}
mod error {
    use gix_object::bstr::BString;

    use crate::store_impl::{file, packed};

    /// The error returned by various [`Transaction`][super::Transaction] methods.
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("The packed-ref transaction could not be committed")]
        PackedTransactionCommit(#[source] packed::transaction::commit::Error),
        #[error("Edit preprocessing failed with error")]
        PreprocessingFailed { source: std::io::Error },
        #[error("The change for reference {full_name:?} could not be committed")]
        LockCommit { source: std::io::Error, full_name: BString },
        #[error("The reference {full_name} could not be deleted")]
        DeleteReference { full_name: BString, err: std::io::Error },
        #[error("The reflog of reference {full_name:?} could not be deleted")]
        DeleteReflog { full_name: BString, source: std::io::Error },
        #[error("The reflog could not be created or updated")]
        CreateOrUpdateRefLog(#[from] file::log::create_or_update::Error),
    }
}
pub use error::Error;

use crate::transaction::PreviousValue;
