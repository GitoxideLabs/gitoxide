use std::path::PathBuf;

use crate::bstr::ByteSlice;
use gix_error::{ClassificationMarker, ErrorExt, Message, Result, ResultExt, bail, message, not_found};

use crate::{
    FullName, FullNameRef, Reference, Target, packed,
    packed::transaction::buffer_into_transaction,
    store_impl::{
        file,
        file::{
            ReferenceExt, Transaction, loose,
            transaction::{Edit, PackedRefs},
        },
    },
    transaction::{Change, LogChange, PreviousValue, RefEdit, RefEditsExt, RefLog},
};

impl Transaction<'_, '_> {
    fn add_head_reflog(&self, updates: &mut Vec<Edit>) -> Result {
        let head: &FullNameRef = "HEAD".try_into().expect("valid static reference name");
        let head_reference = match self.store.find_existing_inner(head, None) {
            Ok(reference) => reference,
            Err(err) if err.is_not_found() || err.downcast_any_ref::<file::find::ReferenceDecode>().is_some() => {
                return Ok(());
            }
            Err(err) => return Err(err),
        };
        let Target::Symbolic(referent) = &head_reference.target else {
            return Ok(());
        };
        let head_path = self.store.reference_path(head)?;
        let referent_path = self.store.reference_path(referent.as_ref())?;
        let paths = updates
            .iter()
            .map(|edit| self.store.reference_path(edit.update.name.as_ref()))
            .collect::<Result<Vec<_>>>()?;
        // An explicit HEAD edit owns its reflog, including when HEAD is being detached or retargeted.
        if paths.contains(&head_path) {
            return Ok(());
        }
        for (index, (edit, path)) in updates.iter().zip(&paths).enumerate() {
            if *path != referent_path {
                continue;
            }
            let (new, log) = match &edit.update.change {
                Change::Update {
                    new,
                    log:
                        log @ LogChange {
                            mode: RefLog::AndReference,
                            ..
                        },
                    ..
                } => (
                    match new {
                        Target::Object(_) => new.clone(),
                        Target::Symbolic(_) => self.store.object_hash.null().into(),
                    },
                    LogChange {
                        mode: RefLog::Only,
                        ..log.clone()
                    },
                ),
                Change::Delete {
                    log: RefLog::AndReference,
                    ..
                } => (
                    self.store.object_hash.null().into(),
                    LogChange {
                        mode: RefLog::Only,
                        message: self.reflog_message.clone(),
                        ..Default::default()
                    },
                ),
                _ => continue,
            };
            updates.push(Edit {
                update: RefEdit::update_with_log(
                    head.to_owned(),
                    new,
                    PreviousValue::MustExistAndMatch(head_reference.target),
                    log,
                ),
                lock: None,
                parent_index: Some(index),
                leaf_referent_previous_oid: None,
                symbolic_new_oid: None,
                implicit_head: true,
            });
            break;
        }
        Ok(())
    }

    fn resolved_id(
        store: &file::Store,
        target: &Target,
        packed: Option<&packed::Buffer>,
    ) -> Result<Option<gix_hash::ObjectId>> {
        match target {
            Target::Object(object_id) => Ok(Some(*object_id)),
            Target::Symbolic(name) => match store.find_existing_inner(name.as_ref(), packed) {
                Ok(mut reference) => match reference.follow_to_object_packed(store, packed) {
                    Ok(object_id) => Ok(Some(object_id)),
                    Err(err) if err.is_not_found() => Ok(None),
                    Err(err) => Err(err),
                },
                Err(err) if err.is_not_found() => Ok(None),
                Err(err) => Err(err),
            },
        }
    }

    fn previous_id(
        store: &file::Store,
        target: &Target,
        expected: &PreviousValue,
        packed: Option<&packed::Buffer>,
    ) -> Result<Option<gix_hash::ObjectId>> {
        match Self::resolved_id(store, target, packed) {
            // Unconstrained edits can repair symbolic refs whose referents cannot be resolved.
            Err(_) if matches!(expected, PreviousValue::Any | PreviousValue::MustExist) => Ok(None),
            result => result,
        }
    }

    /// Read the current value of a reference from loose storage, falling back to packed refs.
    ///
    /// Must be called while holding the lock for `name` so the subsequent CAS check
    /// compares against a value that cannot be changed concurrently.
    fn read_existing_ref(
        store: &file::Store,
        name: &FullNameRef,
        packed: Option<&packed::Buffer>,
    ) -> Result<Option<Reference>> {
        let loose = store
            .ref_contents(name)
            .or_raise(|| message("Could not read existing reference"))?
            // Git permits replacing malformed loose references, but I/O errors must propagate.
            .map(|buf| {
                loose::Reference::try_from_path(name.to_owned(), &buf, store.object_hash)
                    .ok()
                    .map(Reference::from)
            });
        match (loose, packed) {
            (None, Some(packed)) => {
                let Some(name) = packed::find::transform_full_name_for_lookup(name) else {
                    return Ok(None);
                };
                let namespaced = store
                    .namespace
                    .as_ref()
                    .map(|namespace| namespace.clone().into_namespaced_name(name));
                packed
                    .try_find(namespaced.as_ref().map_or(name, AsRef::as_ref))
                    .map(|reference| reference.map(Into::into))
            }
            // Even malformed loose contents shadow an older packed value.
            (reference, _) => Ok(reference.flatten()),
        }
    }

    fn lock_ref_and_apply_change(
        store: &file::Store,
        lock_fail_mode: gix_lock::acquire::Fail,
        packed: Option<&packed::Buffer>,
        change: &mut Edit,
        direct_to_packed_refs: bool,
    ) -> Result {
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
            .or_raise(|| message("Invalid reference filename"))?;

        let (reference_path, lock_boundary) = reference_path_and_lock_boundary(store, change.update.name.as_ref())?;

        let lock = match &mut change.update.change {
            Change::Delete { expected, .. } => {
                let lock = gix_lock::Marker::acquire_to_hold_resource(
                    &reference_path,
                    lock_fail_mode,
                    Some(lock_boundary.clone()),
                    0,
                )?;

                let existing_ref = Self::read_existing_ref(store, change.update.name.as_ref(), packed)?;

                match (&expected, &existing_ref) {
                    (PreviousValue::MustNotExist, _) => {
                        panic!("BUG: MustNotExist constraint makes no sense if references are to be deleted")
                    }
                    (PreviousValue::ExistingMustMatch(_) | PreviousValue::Any, None)
                    | (PreviousValue::MustExist | PreviousValue::Any, Some(_)) => {}
                    (PreviousValue::MustExist | PreviousValue::MustExistAndMatch(_), None) => {
                        bail!(not_found("The reference to delete must exist"));
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
                            .and_raise(context));
                        }
                    }
                }

                // Keep the previous value for the caller and ourselves. Maybe they want to keep a log of sorts.
                if let Some(existing) = existing_ref {
                    change.leaf_referent_previous_oid = Self::previous_id(store, &existing.target, expected, packed)?;
                    *expected = PreviousValue::MustExistAndMatch(existing.target);
                }

                Some(lock)
            }
            Change::Update { expected, new, .. } => {
                let obtain_lock = || {
                    gix_lock::File::acquire_to_update_resource(
                        &reference_path,
                        lock_fail_mode,
                        Some(lock_boundary.clone()),
                        0,
                    )
                };
                let mut lock = obtain_lock()?;

                let existing_ref = Self::read_existing_ref(store, change.update.name.as_ref(), packed)?;

                match (&expected, &existing_ref) {
                    (PreviousValue::Any, _)
                    | (PreviousValue::MustExist, Some(_))
                    | (PreviousValue::MustNotExist | PreviousValue::ExistingMustMatch(_), None) => {}
                    (PreviousValue::MustExist, None) => {
                        bail!(not_found("The reference to update must exist"));
                    }
                    (PreviousValue::MustNotExist, Some(existing)) => {
                        if existing.target != *new {
                            let context = message!("Expected the reference not to exist when writing {new}");
                            return Err(MustNotExist {
                                full_name: change.name(),
                                actual: existing.target.clone(),
                            }
                            .and_raise(context));
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
                            .and_raise(context));
                        }
                    }

                    (PreviousValue::MustExistAndMatch(previous), None) => {
                        bail!("The reference must exist with content {previous}".not_found());
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

                if !change.implicit_head
                    && let Some(existing) = &existing_ref
                {
                    change.leaf_referent_previous_oid = Self::previous_id(store, &existing.target, expected, packed)?;
                }
                if store.write_reflog != crate::store::WriteReflog::Disable && matches!(new, Target::Symbolic(_)) {
                    change.symbolic_new_oid = Self::resolved_id(store, new, packed).ok().flatten();
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
                        Target::Symbolic(name) => file
                            .write_all(b"ref: ")
                            .and_then(|()| file.write_all(name.as_bstr()))
                            .and_then(|()| file.write_all(b"\n")),
                    })
                    .or_raise(|| message("Could not write loose reference"))?;
                    Some(lock.close().or_raise(|| message("Could not close reference lock"))?)
                } else if keep_lock_for_loose_source_delete {
                    Some(lock.close().or_raise(|| message("Could not close reference lock"))?)
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
    /// Failed edits identify the requested name in [metadata](gix_error::Error::metadata()) `reference` (bytes).
    /// `referent` records the resolved name only when it differs from the requested name.
    /// [`ReferenceOutOfDate`] and [`MustNotExist`] retain the actual target observed while holding the lock.
    pub fn prepare(
        self,
        edits: impl IntoIterator<Item = RefEdit>,
        ref_files_lock_fail_mode: gix_lock::acquire::Fail,
        packed_refs_lock_fail_mode: gix_lock::acquire::Fail,
    ) -> Result<Self> {
        self.prepare_inner(
            &mut edits.into_iter(),
            ref_files_lock_fail_mode,
            packed_refs_lock_fail_mode,
            false,
        )
    }

    /// Rename must keep even an absent packed-refs file locked while checking namespace conflicts.
    pub(in crate::store_impl::file) fn prepare_for_rename(
        self,
        edit: RefEdit,
        ref_lock: gix_lock::acquire::Fail,
        packed_lock: gix_lock::acquire::Fail,
    ) -> Result<Self> {
        self.prepare_inner(&mut std::iter::once(edit), ref_lock, packed_lock, true)
    }

    /// Failed edits include [metadata](gix_error::Error::metadata()) `reference` (requested name bytes) and `referent`
    /// (resolved name bytes).
    fn prepare_inner(
        mut self,
        edits: &mut dyn Iterator<Item = RefEdit>,
        ref_files_lock_fail_mode: gix_lock::acquire::Fail,
        packed_refs_lock_fail_mode: gix_lock::acquire::Fail,
        force_packed_lock: bool,
    ) -> Result<Self> {
        assert!(self.updates.is_none(), "BUG: Must not call prepare(…) multiple times");
        let store = self.store;
        let mut updates: Vec<_> = edits
            .map(|update| Edit {
                update,
                lock: None,
                parent_index: None,
                leaf_referent_previous_oid: None,
                symbolic_new_oid: None,
                implicit_head: false,
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
                    symbolic_new_oid: None,
                    implicit_head: false,
                },
            )
            .or_raise(|| message("Could not preprocess reference edits"))?;

        if self.store.write_reflog != crate::store::WriteReflog::Disable
            && !matches!(
                self.packed_refs,
                PackedRefs::DeletionsAndNonSymbolicUpdatesRemoveLooseSourceReference(_)
            )
        {
            self.add_head_reflog(&mut updates)?;
        }

        let mut maybe_updates_for_packed_refs = match self.packed_refs {
            PackedRefs::DeletionsAndNonSymbolicUpdates(_)
            | PackedRefs::DeletionsAndNonSymbolicUpdatesRemoveLooseSourceReference(_) => Some(0_usize),
            PackedRefs::DeletionsOnly => None,
        };
        if force_packed_lock
            || maybe_updates_for_packed_refs.is_some()
            || self.store.packed_refs_path().is_file()
            || self.store.packed_refs_lock_path().is_file()
        {
            let mut edits_for_packed_transaction = Vec::<RefEdit>::new();
            let mut needs_packed_refs_lookups = updates.iter().any(|edit| {
                matches!(
                    edit.update.change,
                    Change::Update {
                        new: Target::Symbolic(_),
                        ..
                    }
                )
            });
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
                let packed_transaction: Option<_> = if force_packed_lock
                    || maybe_updates_for_packed_refs.unwrap_or(0) > 0
                    || self.store.packed_refs_lock_path().is_file()
                {
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

        let mut updated_paths = Vec::new();
        for edit in &updates {
            if !matches!(
                edit.update.change,
                Change::Update {
                    log: LogChange {
                        mode: RefLog::AndReference,
                        ..
                    },
                    ..
                }
            ) {
                continue;
            }
            self.store
                .check_windows_device_name(edit.update.name.as_ref())
                .or_raise(|| message("Invalid reference filename"))
                .or_raise(|| Message::new("Could not prepare reference edit").with("reference", edit.name()))?;
            let path = self.store.reference_path(edit.update.name.as_ref())?;
            updated_paths.push((path, edit.name()));
        }
        updated_paths.sort_by(|a, b| a.0.cmp(&b.0));
        for pair in updated_paths.windows(2) {
            if pair[0].0 != pair[1].0 && pair[1].0.starts_with(&pair[0].0) {
                bail!(
                    gix_error::validation("Reference updates have conflicting names")
                        .with("reference", pair[1].1.as_bstr())
                );
            }
        }
        self.store.check_namespace_conflicts(
            None,
            &updated_paths,
            self.packed_transaction.as_ref().and_then(packed::Transaction::buffer),
        )?;

        for cid in 0..updates.len() {
            let change = &mut updates[cid];
            if let Err(err) = Self::lock_ref_and_apply_change(
                self.store,
                if change.implicit_head {
                    // HEAD transactions take the opposite order; never wait while holding their referent.
                    gix_lock::acquire::Fail::Immediately
                } else {
                    ref_files_lock_fail_mode
                },
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
                let differs = ref_name != referent;
                let mut context = Message::new("Could not prepare reference edit").with("reference", ref_name);
                if differs {
                    context = context.with("referent", referent);
                }
                return Err(err.and_raise(context));
            }

            if change.implicit_head {
                let parent_index = change.parent_index.expect("implicit HEAD has its referent as parent");
                updates[cid].leaf_referent_previous_oid = updates[parent_index].leaf_referent_previous_oid;
                continue;
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
            .map(|updates| {
                updates
                    .into_iter()
                    .filter(|edit| !edit.implicit_head)
                    .map(|u| u.update)
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Resolve the reference path and the directory that empty-parent lock cleanup must preserve.
/// Keep the physical `refs` root for discovery; pseudorefs retain their containing directory.
///
/// For a resolved base of `/repo/.git`, relative paths produce these cleanup boundaries:
///
/// * `refs/heads/topic` -> `/repo/.git/refs`
/// * `refs/namespaces/team/refs/heads/topic` -> `/repo/.git/refs`
/// * `HEAD` -> `/repo/.git`
/// * `worktrees/other/HEAD` -> `/repo/.git/worktrees/other`
/// * `worktrees/other/refs/bisect/bad` -> `/repo/.git/worktrees/other/refs`
/// * `worktrees/refs/refs/bisect/bad` -> `/repo/.git/worktrees/refs/refs`
///
/// The store resolves namespaces and worktree aliases before selecting the boundary.
fn reference_path_and_lock_boundary(store: &file::Store, name: &FullNameRef) -> Result<(PathBuf, PathBuf)> {
    let (base, relative_path) = store.reference_path_with_base(name)?;
    let reference_path = base.join(relative_path.as_ref());
    let mut lock_boundary = base.into_owned();
    let worktree_prefix = relative_path.starts_with("worktrees");
    for (index, component) in relative_path
        .parent()
        .expect("reference paths have a parent")
        .components()
        .enumerate()
    {
        lock_boundary.push(component);
        if component.as_os_str() == "refs" && (!worktree_prefix || index >= 2) {
            break;
        }
    }
    Ok((reference_path, lock_boundary))
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

impl std::error::Error for ReferenceOutOfDate {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(const { &ClassificationMarker::CONFLICT })
    }
}

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

impl std::error::Error for MustNotExist {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(const { &ClassificationMarker::CONFLICT })
    }
}
