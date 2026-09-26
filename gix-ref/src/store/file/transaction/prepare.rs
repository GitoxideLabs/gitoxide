use gix_error::Result;
use gix_error::{ErrorExt, ExnResult, Message, ResultExt, message};

use crate::{
    FullName, FullNameRef, Reference, Target, packed,
    packed::transaction::buffer_into_transaction,
    store_impl::{
        file,
        file::{
            Transaction, loose,
            transaction::{Edit, PackedRefs},
        },
    },
    transaction::{Change, LogChange, PreviousValue, RefEdit, RefEditsExt, RefLog},
};

impl Transaction<'_, '_> {
    /// Read the current value of a reference from loose storage, falling back to packed refs.
    ///
    /// Must be called while holding the lock for `name` so the subsequent CAS check
    /// compares against a value that cannot be changed concurrently.
    fn read_existing_ref(
        store: &file::Store,
        name: &FullNameRef,
        packed: Option<&packed::Buffer>,
    ) -> ExnResult<Option<Reference>> {
        let loose = store
            .ref_contents(name)
            .or_raise_erased(|| message("Could not read existing reference"))?
            // Git permits replacing malformed loose references, but I/O errors must propagate.
            .and_then(|buf| loose::Reference::try_from_path(name.to_owned(), &buf, store.object_hash).ok())
            .map(Reference::from);
        match (loose, packed) {
            (None, Some(packed)) => packed
                .try_find(name)
                .map(|reference| reference.map(Into::into))
                .or_erased(),
            (reference, _) => Ok(reference),
        }
    }

    fn lock_ref_and_apply_change(
        store: &file::Store,
        lock_fail_mode: gix_lock::acquire::Fail,
        packed: Option<&packed::Buffer>,
        change: &mut Edit,
        direct_to_packed_refs: bool,
    ) -> ExnResult {
        use std::io::Write;
        assert!(
            change.lock.is_none(),
            "locks can only be acquired once and it's all or nothing"
        );

        // Reject Windows reserved device names before acquiring the lock.
        // The lock file itself (e.g. `CON.lock`) is also a device name,
        // so acquiring it would fail or open the device instead of
        // returning the configured validation error.
        store
            .check_windows_device_name(change.update.name.as_ref())
            .or_raise_erased(|| message("Invalid reference filename"))?;

        let lock = match &mut change.update.change {
            Change::Delete { expected, .. } => {
                let (base, relative_path) = store.reference_path_with_base(change.update.name.as_ref());
                let lock = gix_lock::Marker::acquire_to_hold_resource(
                    base.join(relative_path.as_ref()),
                    lock_fail_mode,
                    Some(base.clone().into_owned()),
                    0,
                )
                .or_erased()?;

                let existing_ref = Self::read_existing_ref(store, change.update.name.as_ref(), packed)?;

                match (&expected, &existing_ref) {
                    (PreviousValue::MustNotExist, _) => {
                        panic!("BUG: MustNotExist constraint makes no sense if references are to be deleted")
                    }
                    (PreviousValue::ExistingMustMatch(_) | PreviousValue::Any, None)
                    | (PreviousValue::MustExist | PreviousValue::Any, Some(_)) => {}
                    (PreviousValue::MustExist | PreviousValue::MustExistAndMatch(_), None) => {
                        return Err(gix_error::not_found("The reference to delete must exist").raise_erased());
                    }
                    (
                        PreviousValue::MustExistAndMatch(previous) | PreviousValue::ExistingMustMatch(previous),
                        Some(existing),
                    ) => {
                        let actual = existing.target.clone();
                        if *previous != actual {
                            let context = message!("Expected reference content {previous}");
                            return Err(ReferenceOutOfDate {
                                full_name: change.name(),
                                actual,
                            }
                            .and_raise_typed(context)
                            .erased());
                        }
                    }
                }

                // Keep the previous value for the caller and ourselves. Maybe they want to keep a log of sorts.
                if let Some(existing) = existing_ref {
                    *expected = PreviousValue::MustExistAndMatch(existing.target);
                }

                Some(lock)
            }
            Change::Update { expected, new, .. } => {
                let (base, relative_path) = store.reference_path_with_base(change.update.name.as_ref());
                let obtain_lock = || {
                    gix_lock::File::acquire_to_update_resource(
                        base.join(relative_path.as_ref()),
                        lock_fail_mode,
                        Some(base.clone().into_owned()),
                        0,
                    )
                };
                let mut lock = obtain_lock().or_erased()?;

                let existing_ref = Self::read_existing_ref(store, change.update.name.as_ref(), packed)?;

                match (&expected, &existing_ref) {
                    (PreviousValue::Any, _)
                    | (PreviousValue::MustExist, Some(_))
                    | (PreviousValue::MustNotExist | PreviousValue::ExistingMustMatch(_), None) => {}
                    (PreviousValue::MustExist, None) => {
                        return Err(gix_error::not_found("The reference to update must exist").raise_erased());
                    }
                    (PreviousValue::MustNotExist, Some(existing)) => {
                        if existing.target != *new {
                            let context = message!("Expected the reference not to exist when writing {new}");
                            return Err(MustNotExist {
                                full_name: change.name(),
                                actual: existing.target.clone(),
                            }
                            .and_raise_typed(context)
                            .erased());
                        }
                    }
                    (
                        PreviousValue::MustExistAndMatch(previous) | PreviousValue::ExistingMustMatch(previous),
                        Some(existing),
                    ) => {
                        if *previous != existing.target {
                            let actual = existing.target.clone();
                            let context = message!("Expected reference content {previous}");
                            return Err(ReferenceOutOfDate {
                                full_name: change.name(),
                                actual,
                            }
                            .and_raise_typed(context)
                            .erased());
                        }
                    }

                    (PreviousValue::MustExistAndMatch(previous), None) => {
                        return Err(
                            gix_error::not_found(format!("The reference must exist with content {previous}"))
                                .raise_erased(),
                        );
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

                let keep_lock_for_loose_source_delete = direct_to_packed_refs && matches!(new, Target::Object(_));
                if (is_effective && !direct_to_packed_refs) || is_symbolic {
                    lock.with_mut(|file| match new {
                        Target::Object(oid) => writeln!(file, "{oid}"),
                        Target::Symbolic(name) => writeln!(file, "ref: {}", name.0),
                    })
                    .or_raise_erased(|| message("Could not write loose reference"))?;
                    Some(
                        lock.close()
                            .or_raise_erased(|| message("Could not close reference lock"))?,
                    )
                } else if keep_lock_for_loose_source_delete {
                    Some(
                        lock.close()
                            .or_raise_erased(|| message("Could not close reference lock"))?,
                    )
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
    ///
    /// Failed edits identify the requested and resolved names in [metadata](gix_error::Error::metadata()) `reference` and
    /// `referent` (bytes).
    /// [`ReferenceOutOfDate`] and [`MustNotExist`] retain the actual target observed while holding the lock.
    pub fn prepare(
        self,
        edits: impl IntoIterator<Item = RefEdit>,
        ref_files_lock_fail_mode: gix_lock::acquire::Fail,
        packed_refs_lock_fail_mode: gix_lock::acquire::Fail,
    ) -> Result<Self> {
        Ok(self.prepare_inner(
            &mut edits.into_iter(),
            ref_files_lock_fail_mode,
            packed_refs_lock_fail_mode,
        )?)
    }

    /// Failed edits include [metadata](gix_error::Error::metadata()) `reference` (requested name bytes) and `referent`
    /// (resolved name bytes).
    fn prepare_inner(
        mut self,
        edits: &mut dyn Iterator<Item = RefEdit>,
        ref_files_lock_fail_mode: gix_lock::acquire::Fail,
        packed_refs_lock_fail_mode: gix_lock::acquire::Fail,
    ) -> ExnResult<Self> {
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
            .or_raise_erased(|| message("Could not preprocess reference edits"))?;

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
                if let Some(ref mut num_updates) = maybe_updates_for_packed_refs
                    && let Change::Update {
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
                        self.store.packed_transaction(packed_refs_lock_fail_mode)?.into()
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
                matches!(
                    self.packed_refs,
                    PackedRefs::DeletionsAndNonSymbolicUpdatesRemoveLooseSourceReference(_)
                ),
            ) {
                let referent = change.name();
                let mut ref_name = referent.clone();
                let mut cursor = change.parent_index;
                while let Some(parent_idx) = cursor {
                    let parent = &updates[parent_idx];
                    ref_name = parent.name();
                    cursor = parent.parent_index;
                }
                return Err(err
                    .raise(
                        Message::new("Could not prepare reference edit")
                            .with("reference", ref_name)
                            .with("referent", referent),
                    )
                    .erased());
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

/// A reference changed since the caller obtained its expected value.
/// Retrying requires reading and reconciling the new value first.
#[derive(Debug)]
pub struct ReferenceOutOfDate {
    /// The reference whose target did not match the expected value.
    pub full_name: crate::bstr::BString,
    /// The target observed while holding the reference lock.
    pub actual: Target,
}

impl std::fmt::Display for ReferenceOutOfDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "The reference {:?} changed to {}", self.full_name, self.actual)
    }
}

impl std::error::Error for ReferenceOutOfDate {}

/// A reference exists with a different target although the edit required its absence.
#[derive(Debug)]
pub struct MustNotExist {
    /// The reference which unexpectedly exists.
    pub full_name: crate::bstr::BString,
    /// The target observed while holding the reference lock.
    pub actual: Target,
}

impl std::fmt::Display for MustNotExist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The reference {:?} already exists with content {}",
            self.full_name, self.actual
        )
    }
}

impl std::error::Error for MustNotExist {}
