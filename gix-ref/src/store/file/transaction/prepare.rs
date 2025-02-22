use crate::{
    packed,
    packed::transaction::buffer_into_transaction,
    store_impl::{
        file,
        file::{
            loose,
            transaction::{Edit, PackedRefs},
            Transaction,
        },
    },
    transaction::{Change, LogChange, PreviousValue, RefEdit, RefEditsExt, RefLog},
    FullName, FullNameRef, Reference, Target,
};

impl Transaction<'_, '_> {
    fn lock_ref_and_apply_change(
        store: &file::Store,
        lock_fail_mode: gix_lock::acquire::Fail,
        packed: Option<&packed::Buffer>,
        change: &mut Edit,
        has_global_lock: bool,
        direct_to_packed_refs: bool,
    ) -> Result<(), Error> {
        use std::io::Write;
        assert!(
            change.lock.is_none(),
            "locks can only be acquired once and it's all or nothing"
        );

        let existing_ref = store
            .ref_contents(change.update.name.as_ref())
            .map_err(Error::from)
            .and_then(|maybe_loose| {
                maybe_loose
                    .map(|buf| {
                        loose::Reference::try_from_path(change.update.name.clone(), &buf)
                            .map(Reference::from)
                            .map_err(Error::from)
                    })
                    .transpose()
            })
            .or_else(|err| match err {
                Error::ReferenceDecode(_) => Ok(None),
                other => Err(other),
            })
            .and_then(|maybe_loose| match (maybe_loose, packed) {
                (None, Some(packed)) => packed
                    .try_find(change.update.name.as_ref())
                    .map(|opt| opt.map(Into::into))
                    .map_err(Error::from),
                (None, None) => Ok(None),
                (maybe_loose, _) => Ok(maybe_loose),
            })?;
        let lock = match &mut change.update.change {
            Change::Delete { expected, .. } => {
                let (base, relative_path) = store.reference_path_with_base(change.update.name.as_ref());
                let lock = if has_global_lock {
                    None
                } else {
                    gix_lock::Marker::acquire_to_hold_resource(
                        base.join(relative_path.as_ref()),
                        lock_fail_mode,
                        Some(base.clone().into_owned()),
                    )
                    .map_err(|err| Error::LockAcquire {
                        source: err,
                        full_name: "borrowcheck won't allow change.name()".into(),
                    })?
                    .into()
                };

                match (&expected, &existing_ref) {
                    (PreviousValue::MustNotExist, _) => {
                        panic!("BUG: MustNotExist constraint makes no sense if references are to be deleted")
                    }
                    (PreviousValue::ExistingMustMatch(_) | PreviousValue::Any, None)
                    | (PreviousValue::MustExist | PreviousValue::Any, Some(_)) => {}
                    (PreviousValue::MustExist | PreviousValue::MustExistAndMatch(_), None) => {
                        return Err(Error::DeleteReferenceMustExist {
                            full_name: change.name(),
                        })
                    }
                    (
                        PreviousValue::MustExistAndMatch(previous) | PreviousValue::ExistingMustMatch(previous),
                        Some(existing),
                    ) => {
                        let actual = existing.target.clone();
                        if *previous != actual {
                            let expected = previous.clone();
                            return Err(Error::ReferenceOutOfDate {
                                full_name: change.name(),
                                expected,
                                actual,
                            });
                        }
                    }
                }

                // Keep the previous value for the caller and ourselves. Maybe they want to keep a log of sorts.
                if let Some(existing) = existing_ref {
                    *expected = PreviousValue::MustExistAndMatch(existing.target);
                }

                lock
            }
            Change::Update { expected, new, .. } => {
                let (base, relative_path) = store.reference_path_with_base(change.update.name.as_ref());
                let obtain_lock = || {
                    gix_lock::File::acquire_to_update_resource(
                        base.join(relative_path.as_ref()),
                        lock_fail_mode,
                        Some(base.clone().into_owned()),
                    )
                    .map_err(|err| Error::LockAcquire {
                        source: err,
                        full_name: "borrowcheck won't allow change.name() and this will be corrected by caller".into(),
                    })
                };
                let mut lock = (!has_global_lock).then(obtain_lock).transpose()?;

                match (&expected, &existing_ref) {
                    (PreviousValue::Any, _)
                    | (PreviousValue::MustExist, Some(_))
                    | (PreviousValue::MustNotExist | PreviousValue::ExistingMustMatch(_), None) => {}
                    (PreviousValue::MustExist, None) => {
                        let expected = Target::Object(store.object_hash.null());
                        let full_name = change.name();
                        return Err(Error::MustExist { full_name, expected });
                    }
                    (PreviousValue::MustNotExist, Some(existing)) => {
                        if existing.target != *new {
                            let new = new.clone();
                            return Err(Error::MustNotExist {
                                full_name: change.name(),
                                actual: existing.target.clone(),
                                new,
                            });
                        }
                    }
                    (
                        PreviousValue::MustExistAndMatch(previous) | PreviousValue::ExistingMustMatch(previous),
                        Some(existing),
                    ) => {
                        if *previous != existing.target {
                            let actual = existing.target.clone();
                            let expected = previous.to_owned();
                            let full_name = change.name();
                            return Err(Error::ReferenceOutOfDate {
                                full_name,
                                actual,
                                expected,
                            });
                        }
                    }

                    (PreviousValue::MustExistAndMatch(previous), None) => {
                        let expected = previous.to_owned();
                        let full_name = change.name();
                        return Err(Error::MustExist { full_name, expected });
                    }
                }

                fn new_would_change_existing(new: &Target, existing: &Target) -> (bool, bool) {
                    match (new, existing) {
                        (Target::Object(new), Target::Object(old)) => (old != new, false),
                        (Target::Symbolic(new), Target::Symbolic(old)) => (old != new, true),
                        (Target::Object(_), _) => (true, false),
                        (Target::Symbolic(_), _) => (true, true),
                    }
                }

                let (is_effective, is_symbolic) = if let Some(existing) = existing_ref {
                    let (effective, is_symbolic) = new_would_change_existing(new, &existing.target);
                    *expected = PreviousValue::MustExistAndMatch(existing.target);
                    (effective, is_symbolic)
                } else {
                    (true, matches!(new, Target::Symbolic(_)))
                };

                if (is_effective && !direct_to_packed_refs) || is_symbolic {
                    let mut lock = lock.take().map_or_else(obtain_lock, Ok)?;

                    lock.with_mut(|file| match new {
                        Target::Object(oid) => write!(file, "{oid}"),
                        Target::Symbolic(name) => writeln!(file, "ref: {}", name.0),
                    })?;
                    Some(lock.close()?)
                } else {
                    None
                }
            }
        };
        change.lock = lock;
        Ok(())
    }
}

impl Transaction<'_, '_> {
    /// Prepare for calling [`commit(…)`][Transaction::commit()] in a way that can be rolled back perfectly.
    ///
    /// If the operation succeeds, the transaction can be committed or dropped to cause a rollback automatically.
    /// Rollbacks happen automatically on failure and they tend to be perfect.
    /// This method is idempotent.
    pub fn prepare(
        self,
        edits: impl IntoIterator<Item = RefEdit>,
        ref_files_lock_fail_mode: gix_lock::acquire::Fail,
        packed_refs_lock_fail_mode: gix_lock::acquire::Fail,
    ) -> Result<Self, Error> {
        self.prepare_inner(
            &mut edits.into_iter(),
            ref_files_lock_fail_mode,
            packed_refs_lock_fail_mode,
        )
    }

    fn prepare_inner(
        mut self,
        edits: &mut dyn Iterator<Item = RefEdit>,
        ref_files_lock_fail_mode: gix_lock::acquire::Fail,
        packed_refs_lock_fail_mode: gix_lock::acquire::Fail,
    ) -> Result<Self, Error> {
        assert!(self.updates.is_none(), "BUG: Must not call prepare(…) multiple times");
        let store = self.store;
        let mut updates: Vec<_> = edits
            .map(|update| Edit {
                update,
                lock: None,
                parent_index: None,
                leaf_referent_previous_oid: None,
            })
            .collect();
        updates
            .pre_process(
                &mut |name| {
                    let symbolic_refs_are_never_packed = None;
                    store
                        .find_existing_inner(name, symbolic_refs_are_never_packed)
                        .map(|r| r.target)
                        .ok()
                },
                &mut |idx, update| Edit {
                    update,
                    lock: None,
                    parent_index: Some(idx),
                    leaf_referent_previous_oid: None,
                },
            )
            .map_err(Error::PreprocessingFailed)?;

        let mut maybe_updates_for_packed_refs = match self.packed_refs {
            PackedRefs::DeletionsAndNonSymbolicUpdates(_)
            | PackedRefs::DeletionsAndNonSymbolicUpdatesRemoveLooseSourceReference(_) => Some(0_usize),
            PackedRefs::DeletionsOnly => None,
        };
        if maybe_updates_for_packed_refs.is_some()
            || self.store.packed_refs_path().is_file()
            || self.store.packed_refs_lock_path().is_file()
        {
            let mut edits_for_packed_transaction = Vec::<RefEdit>::new();
            let mut needs_packed_refs_lookups = false;
            for edit in &updates {
                let log_mode = match edit.update.change {
                    Change::Update {
                        log: LogChange { mode, .. },
                        ..
                    } => mode,
                    Change::Delete { log, .. } => log,
                };
                if log_mode == RefLog::Only {
                    continue;
                }
                let name = match possibly_adjust_name_for_prefixes(edit.update.name.as_ref()) {
                    Some(n) => n,
                    None => continue,
                };
                if let Some(ref mut num_updates) = maybe_updates_for_packed_refs {
                    if let Change::Update {
                        new: Target::Object(_), ..
                    } = edit.update.change
                    {
                        edits_for_packed_transaction.push(RefEdit {
                            name,
                            ..edit.update.clone()
                        });
                        *num_updates += 1;
                        continue;
                    }
                }
                match edit.update.change {
                    Change::Update {
                        expected: PreviousValue::ExistingMustMatch(_) | PreviousValue::MustExistAndMatch(_),
                        ..
                    } => needs_packed_refs_lookups = true,
                    Change::Delete { .. } => {
                        edits_for_packed_transaction.push(RefEdit {
                            name,
                            ..edit.update.clone()
                        });
                    }
                    _ => {
                        needs_packed_refs_lookups = true;
                    }
                }
            }

            if !edits_for_packed_transaction.is_empty() || needs_packed_refs_lookups {
                // What follows means that we will only create a transaction if we have to access packed refs for looking
                // up current ref values, or that we definitely have a transaction if we need to make updates. Otherwise
                // we may have no transaction at all which isn't required if we had none and would only try making deletions.
                let packed_transaction: Option<_> =
                    if maybe_updates_for_packed_refs.unwrap_or(0) > 0 || self.store.packed_refs_lock_path().is_file() {
                        // We have to create a packed-ref even if it doesn't exist
                        self.store
                            .packed_transaction(packed_refs_lock_fail_mode)
                            .map_err(|err| match err {
                                file::packed::transaction::Error::BufferOpen(err) => Error::from(err),
                                file::packed::transaction::Error::TransactionLock(err) => {
                                    Error::PackedTransactionAcquire(err)
                                }
                            })?
                            .into()
                    } else {
                        // A packed transaction is optional - we only have deletions that can't be made if
                        // no packed-ref file exists anyway
                        self.store
                            .assure_packed_refs_uptodate()?
                            .map(|p| {
                                buffer_into_transaction(
                                    p,
                                    packed_refs_lock_fail_mode,
                                    self.store.precompose_unicode,
                                    self.store.namespace.clone(),
                                )
                                .map_err(Error::PackedTransactionAcquire)
                            })
                            .transpose()?
                    };
                if let Some(transaction) = packed_transaction {
                    self.packed_transaction = Some(match &mut self.packed_refs {
                        PackedRefs::DeletionsAndNonSymbolicUpdatesRemoveLooseSourceReference(f)
                        | PackedRefs::DeletionsAndNonSymbolicUpdates(f) => {
                            transaction.prepare(&mut edits_for_packed_transaction.into_iter(), &**f)?
                        }
                        PackedRefs::DeletionsOnly => transaction
                            .prepare(&mut edits_for_packed_transaction.into_iter(), &gix_object::find::Never)?,
                    });
                }
            }
        }

        for cid in 0..updates.len() {
            let change = &mut updates[cid];
            if let Err(err) = Self::lock_ref_and_apply_change(
                self.store,
                ref_files_lock_fail_mode,
                self.packed_transaction.as_ref().and_then(packed::Transaction::buffer),
                change,
                self.packed_transaction.is_some(),
                matches!(
                    self.packed_refs,
                    PackedRefs::DeletionsAndNonSymbolicUpdatesRemoveLooseSourceReference(_)
                ),
            ) {
                let err = match err {
                    Error::LockAcquire {
                        source,
                        full_name: _bogus,
                    } => Error::LockAcquire {
                        source,
                        full_name: {
                            let mut cursor = change.parent_index;
                            let mut ref_name = change.name();
                            while let Some(parent_idx) = cursor {
                                let parent = &updates[parent_idx];
                                if parent.parent_index.is_none() {
                                    ref_name = parent.name();
                                } else {
                                    cursor = parent.parent_index;
                                }
                            }
                            ref_name
                        },
                    },
                    other => other,
                };
                return Err(err);
            }

            // traverse parent chain from leaf/peeled ref and set the leaf previous oid accordingly
            // to help with their reflog entries
            if let (Some(crate::TargetRef::Object(oid)), Some(parent_idx)) =
                (change.update.change.previous_value(), change.parent_index)
            {
                let oid = oid.to_owned();
                let mut parent_idx_cursor = Some(parent_idx);
                while let Some(parent) = parent_idx_cursor.take().map(|idx| &mut updates[idx]) {
                    parent_idx_cursor = parent.parent_index;
                    parent.leaf_referent_previous_oid = Some(oid);
                }
            }
        }
        self.updates = Some(updates);
        Ok(self)
    }

    /// Rollback all intermediate state and return the `RefEdits` as we know them thus far.
    ///
    /// Note that they have been altered compared to what was initially provided as they have
    /// been split and know about their current state on disk.
    ///
    /// # Note
    ///
    /// A rollback happens automatically as this instance is dropped as well.
    pub fn rollback(self) -> Vec<RefEdit> {
        self.updates
            .map(|updates| updates.into_iter().map(|u| u.update).collect())
            .unwrap_or_default()
    }
}

fn possibly_adjust_name_for_prefixes(name: &FullNameRef) -> Option<FullName> {
    match name.category_and_short_name() {
        Some((c, sn)) => {
            use crate::Category::*;
            let sn = FullNameRef::new_unchecked(sn);
            match c {
                Bisect | Rewritten | WorktreePrivate | LinkedPseudoRef { .. } | PseudoRef | MainPseudoRef => None,
                Tag | LocalBranch | RemoteBranch | Note => name.into(),
                MainRef | LinkedRef { .. } => sn
                    .category()
                    .is_some_and(|cat| !cat.is_worktree_private())
                    .then_some(sn),
            }
            .map(ToOwned::to_owned)
        }
        None => Some(name.to_owned()), // allow (uncategorized/very special) refs to be packed
    }
}

mod error {
    use gix_object::bstr::BString;

    use crate::{
        store_impl::{file, packed},
        Target,
    };

    /// The error returned by various [`Transaction`][super::Transaction] methods.
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("The packed ref buffer could not be loaded")]
        Packed(#[from] packed::buffer::open::Error),
        #[error("The lock for the packed-ref file could not be obtained")]
        PackedTransactionAcquire(#[source] gix_lock::acquire::Error),
        #[error("The packed transaction could not be prepared")]
        PackedTransactionPrepare(#[from] packed::transaction::prepare::Error),
        #[error("The packed ref file could not be parsed")]
        PackedFind(#[from] packed::find::Error),
        #[error("Edit preprocessing failed with an error")]
        PreprocessingFailed(#[source] std::io::Error),
        #[error("A lock could not be obtained for reference {full_name:?}")]
        LockAcquire {
            source: gix_lock::acquire::Error,
            full_name: BString,
        },
        #[error("An IO error occurred while applying an edit")]
        Io(#[from] std::io::Error),
        #[error("The reference {full_name:?} for deletion did not exist or could not be parsed")]
        DeleteReferenceMustExist { full_name: BString },
        #[error("Reference {full_name:?} was not supposed to exist when writing it with value {new:?}, but actual content was {actual:?}")]
        MustNotExist {
            full_name: BString,
            actual: Target,
            new: Target,
        },
        #[error("Reference {full_name:?} was supposed to exist with value {expected}, but didn't.")]
        MustExist { full_name: BString, expected: Target },
        #[error("The reference {full_name:?} should have content {expected}, actual content was {actual}")]
        ReferenceOutOfDate {
            full_name: BString,
            expected: Target,
            actual: Target,
        },
        #[error("Could not read reference")]
        ReferenceDecode(#[from] file::loose::reference::decode::Error),
    }
}

pub use error::Error;
