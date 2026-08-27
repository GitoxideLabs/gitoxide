use std::{
    borrow::{Borrow, BorrowMut},
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

use gix_hash::ObjectId;
use gix_object::bstr::{BStr, ByteSlice};

use crate::{
    FullNameRef, Reference, Target,
    store::{WriteReflog, transaction::WriteStrategy},
    transaction::{Change, PreviousValue, RefEdit, RefEditsExt, RefLog},
};

use super::{Error, Route, StackLocation, Store, WorktreeContext, validate_name_conflicts};

pub(crate) struct Transaction<'store> {
    store: &'store Store,
    objects: Option<Box<dyn gix_object::Find + 'store>>,
    prepared: Option<Prepared>,
}

struct Prepared {
    edits: Vec<Edit>,
    stacks: Vec<PreparedStack>,
}

struct PreparedStack {
    path: std::path::PathBuf,
    addition: gix_reftable::LockedAddition,
    edit_indices: Vec<usize>,
}

#[derive(Debug)]
struct Edit {
    update: RefEdit,
    parent_index: Option<usize>,
    leaf_referent_previous_oid: Option<ObjectId>,
    route: Option<Route>,
    previous: Option<Target>,
    effective: bool,
    peeled: Option<ObjectId>,
}

impl Borrow<RefEdit> for Edit {
    fn borrow(&self) -> &RefEdit {
        &self.update
    }
}

impl BorrowMut<RefEdit> for Edit {
    fn borrow_mut(&mut self) -> &mut RefEdit {
        &mut self.update
    }
}

impl<'store> Transaction<'store> {
    pub(crate) fn new(store: &'store Store) -> Self {
        Transaction {
            store,
            objects: None,
            prepared: None,
        }
    }

    pub(crate) fn write_strategy(mut self, strategy: WriteStrategy<'store>) -> Self {
        self.objects = match strategy {
            WriteStrategy::Default => None,
            WriteStrategy::Compact { objects, .. } => Some(objects),
        };
        self
    }

    pub(crate) fn prepare(
        mut self,
        edits: impl IntoIterator<Item = RefEdit>,
        _individual_lock_fail: gix_lock::acquire::Fail,
        aggregate_lock_fail: gix_lock::acquire::Fail,
    ) -> Result<Self, Error> {
        if self.prepared.is_some() {
            return Err(Error::AlreadyPrepared);
        }
        let original = edits.into_iter().collect::<Vec<_>>();
        let provisional = {
            let snapshot = self.store.snapshot()?;
            preprocess_with(&original, |name| snapshot.try_find(name))?
        };
        let mut locations = locations_for(self.store, &provisional)?;
        let lock_options = gix_reftable::LockOptions {
            timeout: match aggregate_lock_fail {
                gix_lock::acquire::Fail::Immediately => Duration::ZERO,
                gix_lock::acquire::Fail::AfterDurationWithBackoff(duration) => duration,
            },
        };

        for _ in 0..5 {
            let mut stacks = Vec::with_capacity(locations.len());
            for (path, location) in &locations {
                let addition = self.store.stack(location)?.begin_addition(lock_options)?;
                stacks.push(PreparedStack {
                    path: path.clone(),
                    addition,
                    edit_indices: Vec::new(),
                });
            }

            let mut missing_locations = BTreeMap::new();
            let mut edits = preprocess_with(&original, |name| {
                find_locked(self.store, &stacks, name, &mut missing_locations)
            })?;
            let expanded_locations = locations_for(self.store, &edits)?;
            for (path, location) in missing_locations.into_iter().chain(expanded_locations) {
                locations.entry(path).or_insert(location);
            }
            if locations.len() != stacks.len() {
                continue;
            }

            validate_and_route(self.store, &mut edits, &mut stacks, self.objects.as_deref())?;
            self.prepared = Some(Prepared { edits, stacks });
            return Ok(self);
        }
        Err(Error::RoutingDidNotStabilize)
    }

    pub(crate) fn commit(self, committer: Option<gix_actor::SignatureRef<'_>>) -> Result<Vec<RefEdit>, Error> {
        let Prepared { edits, stacks } = self.prepared.ok_or(Error::NotPrepared)?;
        let mut publications = Vec::with_capacity(stacks.len());
        for stack in &stacks {
            publications.push(build_records(self.store, &edits, stack, committer)?);
        }
        for (stack, (refs, logs)) in stacks.into_iter().zip(publications) {
            if refs.is_empty() && logs.is_empty() {
                continue;
            }
            stack.addition.commit(&refs, &logs)?;
        }
        Ok(edits.into_iter().map(|edit| edit.update).collect())
    }

    pub(crate) fn rollback(self) -> Vec<RefEdit> {
        self.prepared
            .map(|prepared| prepared.edits.into_iter().map(|edit| edit.update).collect())
            .unwrap_or_default()
    }
}

fn new_edit(update: RefEdit, parent_index: Option<usize>) -> Edit {
    Edit {
        update,
        parent_index,
        leaf_referent_previous_oid: None,
        route: None,
        previous: None,
        effective: false,
        peeled: None,
    }
}

fn preprocess_with(
    original: &[RefEdit],
    mut find: impl FnMut(&crate::PartialNameRef) -> Result<Option<Reference>, Error>,
) -> Result<Vec<Edit>, Error> {
    let mut edits = original
        .iter()
        .cloned()
        .map(|update| new_edit(update, None))
        .collect::<Vec<_>>();
    let mut lookup_error = None;
    edits
        .pre_process(
            &mut |name| match find(name) {
                Ok(reference) => reference.map(|reference| reference.target),
                Err(err) => {
                    if lookup_error.is_none() {
                        lookup_error = Some(err);
                    }
                    None
                }
            },
            &mut |parent_index, update| new_edit(update, Some(parent_index)),
        )
        .map_err(Error::Preprocess)?;
    if let Some(err) = lookup_error {
        return Err(err);
    }
    Ok(edits)
}

fn locations_for(store: &Store, edits: &[Edit]) -> Result<BTreeMap<std::path::PathBuf, StackLocation>, Error> {
    edits
        .iter()
        .map(|edit| {
            let route = store.route(edit.update.name.as_ref());
            Ok((store.stack_path(&route.location)?, route.location))
        })
        .collect()
}

fn find_locked(
    store: &Store,
    stacks: &[PreparedStack],
    name: &crate::PartialNameRef,
    missing: &mut BTreeMap<std::path::PathBuf, StackLocation>,
) -> Result<Option<Reference>, Error> {
    let name: &FullNameRef = name.as_bstr().try_into().map_err(|source| Error::InvalidName {
        name: name.as_bstr().to_owned(),
        source,
    })?;
    let route = store.route(name);
    let path = store.stack_path(&route.location)?;
    let Some(stack) = stacks.iter().find(|stack| stack.path == path) else {
        missing.insert(path, route.location);
        return Ok(None);
    };
    stack
        .addition
        .snapshot()
        .find_ref(route.stored_name.as_bstr())
        .map(|record| store.reference_from_record(record, route.public_name, &route.context))
        .transpose()
}

fn validate_and_route(
    store: &Store,
    edits: &mut [Edit],
    stacks: &mut [PreparedStack],
    objects: Option<&dyn gix_object::Find>,
) -> Result<(), Error> {
    let mut routed_names = BTreeSet::new();
    for (index, edit) in edits.iter_mut().enumerate() {
        let route = store.route(edit.update.name.as_ref());
        let path = store.stack_path(&route.location)?;
        let stack_index = stacks
            .iter()
            .position(|stack| stack.path == path)
            .expect("all routes were locked before validation");
        let key = (path, route.stored_name.as_bstr().to_owned());
        if !routed_names.insert(key) {
            return Err(Error::DuplicateEdit {
                name: route.stored_name.as_bstr().to_owned(),
            });
        }
        let existing = stacks[stack_index]
            .addition
            .snapshot()
            .find_ref(route.stored_name.as_bstr())
            .map(|record| store.reference_from_record(record, route.public_name.clone(), &route.context))
            .transpose()?;
        validate_predicate(edit, existing)?;
        edit.route = Some(route);
        stacks[stack_index].edit_indices.push(index);
    }

    for index in 0..edits.len() {
        if let (Some(Target::Object(previous_id)), Some(parent_index)) =
            (edits[index].previous.as_ref(), edits[index].parent_index)
        {
            let previous_id = *previous_id;
            let mut cursor = Some(parent_index);
            while let Some(parent_index) = cursor {
                cursor = edits[parent_index].parent_index;
                edits[parent_index].leaf_referent_previous_oid = Some(previous_id);
            }
        }
    }

    if let Some(objects) = objects {
        for edit in edits.iter_mut() {
            if !edit.effective {
                continue;
            }
            let Change::Update {
                log,
                new: Target::Object(object_id),
                ..
            } = &edit.update.change
            else {
                continue;
            };
            if log.mode == RefLog::AndReference {
                edit.peeled = peel(objects, *object_id, edit.update.name.as_bstr())?;
            }
        }
    }

    for stack in stacks {
        validate_names(stack, edits)?;
    }
    Ok(())
}

fn validate_predicate(edit: &mut Edit, existing: Option<Reference>) -> Result<(), Error> {
    let name = edit.update.name.as_bstr().to_owned();
    let previous = existing.as_ref().map(|reference| reference.target.clone());
    match &mut edit.update.change {
        Change::Update { expected, new, .. } => {
            match (&*expected, existing.as_ref()) {
                (PreviousValue::Any, _)
                | (PreviousValue::MustExist, Some(_))
                | (PreviousValue::MustNotExist | PreviousValue::ExistingMustMatch(_), None) => {}
                (PreviousValue::MustExist | PreviousValue::MustExistAndMatch(_), None) => {
                    return Err(Error::MustExist { name });
                }
                (PreviousValue::MustNotExist, Some(reference)) if reference.target != *new => {
                    return Err(Error::MustNotExist { name });
                }
                (
                    PreviousValue::MustExistAndMatch(expected) | PreviousValue::ExistingMustMatch(expected),
                    Some(reference),
                ) if *expected != reference.target => {
                    return Err(Error::OutOfDate {
                        name,
                        expected: expected.clone(),
                        actual: reference.target.clone(),
                    });
                }
                _ => {}
            }
            edit.effective = existing.as_ref().is_none_or(|reference| reference.target != *new);
            if let Some(reference) = existing {
                *expected = PreviousValue::MustExistAndMatch(reference.target);
            }
        }
        Change::Delete { expected, .. } => {
            match (&*expected, existing.as_ref()) {
                (PreviousValue::MustNotExist, _) => return Err(Error::InvalidDeletePredicate { name }),
                (PreviousValue::ExistingMustMatch(_) | PreviousValue::Any, None)
                | (PreviousValue::MustExist | PreviousValue::Any, Some(_)) => {}
                (PreviousValue::MustExist | PreviousValue::MustExistAndMatch(_), None) => {
                    return Err(Error::MustExist { name });
                }
                (
                    PreviousValue::MustExistAndMatch(expected) | PreviousValue::ExistingMustMatch(expected),
                    Some(reference),
                ) if *expected != reference.target => {
                    return Err(Error::OutOfDate {
                        name,
                        expected: expected.clone(),
                        actual: reference.target.clone(),
                    });
                }
                _ => {}
            }
            edit.effective = existing.is_some();
            if let Some(reference) = existing {
                *expected = PreviousValue::MustExistAndMatch(reference.target);
            }
        }
    }
    edit.previous = previous;
    Ok(())
}

fn validate_names(stack: &PreparedStack, edits: &[Edit]) -> Result<(), Error> {
    let mut names = stack
        .addition
        .snapshot()
        .refs()
        .map(|record| record.name.clone())
        .collect::<BTreeSet<_>>();
    for index in &stack.edit_indices {
        let edit = &edits[*index];
        let route = edit.route.as_ref().expect("validated edits are routed");
        match &edit.update.change {
            Change::Update { log, .. } if log.mode == RefLog::AndReference => {
                names.insert(route.stored_name.as_bstr().to_owned());
            }
            Change::Delete { log, .. } if *log == RefLog::AndReference => {
                names.remove(route.stored_name.as_bstr());
            }
            _ => {}
        }
    }
    validate_name_conflicts(&names)
}

fn peel(objects: &dyn gix_object::Find, object_id: ObjectId, name: &BStr) -> Result<Option<ObjectId>, Error> {
    let mut next_id = object_id;
    let mut buf = Vec::new();
    loop {
        let data = objects
            .try_find(&next_id, &mut buf)
            .map_err(|source| Error::PeelObject {
                name: name.to_owned(),
                source,
            })?
            .ok_or_else(|| Error::MissingObject {
                name: name.to_owned(),
                object_id: next_id,
            })?;
        if data.kind != gix_object::Kind::Tag {
            return Ok((next_id != object_id).then_some(next_id));
        }
        next_id = gix_object::TagRefIter::from_bytes(data.data, data.object_hash)
            .target_id()
            .map_err(|_| Error::MalformedTag {
                name: name.to_owned(),
                object_id: next_id,
            })?;
    }
}

fn build_records(
    store: &Store,
    edits: &[Edit],
    stack: &PreparedStack,
    committer: Option<gix_actor::SignatureRef<'_>>,
) -> Result<(Vec<gix_reftable::RefRecord>, Vec<gix_reftable::LogRecord>), Error> {
    let update_index = stack.addition.next_update_index();
    let mut refs = Vec::new();
    let mut logs = Vec::new();
    for index in &stack.edit_indices {
        let edit = &edits[*index];
        let route = edit.route.as_ref().expect("validated edits are routed");
        match &edit.update.change {
            Change::Update { log, new, expected } => {
                if edit.effective && log.mode == RefLog::AndReference {
                    refs.push(gix_reftable::RefRecord {
                        name: route.stored_name.as_bstr().to_owned(),
                        update_index,
                        value: ref_value(store, route, new, edit.peeled),
                    });
                }
                if edit.effective
                    && should_write_log(store, stack.addition.snapshot(), route, log.force_create_reflog)
                    && let Some((old_id, new_id)) = log_transition(store.object_hash(), edit, new, expected)
                    && old_id != new_id
                {
                    logs.push(gix_reftable::LogRecord {
                        ref_name: route.stored_name.as_bstr().to_owned(),
                        update_index,
                        value: log_value(old_id, new_id, committer, log.message.as_ref())?,
                    });
                }
            }
            Change::Delete { log, .. } => {
                if edit.effective && *log == RefLog::AndReference {
                    refs.push(gix_reftable::RefRecord {
                        name: route.stored_name.as_bstr().to_owned(),
                        update_index,
                        value: gix_reftable::RefValue::Deletion,
                    });
                }
                logs.extend(
                    stack
                        .addition
                        .snapshot()
                        .log_records_for(route.stored_name.as_bstr())
                        .into_iter()
                        .map(|record| gix_reftable::LogRecord {
                            ref_name: record.ref_name.clone(),
                            update_index: record.update_index,
                            value: gix_reftable::LogValue::Deletion,
                        }),
                );
            }
        }
    }
    refs.sort_by(|a, b| a.name.cmp(&b.name));
    logs.sort_by(|a, b| {
        a.ref_name
            .cmp(&b.ref_name)
            .then_with(|| b.update_index.cmp(&a.update_index))
    });
    Ok((refs, logs))
}

fn ref_value(
    store: &Store,
    route: &Route,
    target: &Target,
    peeled_object_id: Option<ObjectId>,
) -> gix_reftable::RefValue {
    match target {
        Target::Object(target_object_id) => match peeled_object_id {
            Some(peeled_object_id) => gix_reftable::RefValue::Peeled {
                target: *target_object_id,
                peeled: peeled_object_id,
            },
            None => gix_reftable::RefValue::Direct(*target_object_id),
        },
        Target::Symbolic(target) => {
            let target_route = store.route(target.as_ref());
            let stored = if target.category().is_some_and(|category| category.is_worktree_private())
                && matches!(route.context, WorktreeContext::Main | WorktreeContext::Other(_))
            {
                target_route.local_name.as_bstr().to_owned()
            } else {
                target.as_bstr().to_owned()
            };
            gix_reftable::RefValue::Symbolic(stored)
        }
    }
}

fn should_write_log(store: &Store, snapshot: &gix_reftable::Snapshot, route: &Route, force: bool) -> bool {
    match store.write_reflog {
        WriteReflog::Disable => false,
        WriteReflog::Always => true,
        WriteReflog::Normal => {
            force
                || snapshot.reflog_exists(route.stored_name.as_bstr())
                || should_autocreate_reflog(route.local_name.as_bstr())
        }
    }
}

fn should_autocreate_reflog(name: &BStr) -> bool {
    // Intentional gitoxide extension: the backend-neutral store policy treats
    // `refs/worktree/*` like other mutable refs even though Git's files backend
    // does not automatically create reflogs for this category.
    name == b"HEAD"
        || name.starts_with_str("refs/heads/")
        || name.starts_with_str("refs/remotes/")
        || name.starts_with_str("refs/notes/")
        || name.starts_with_str("refs/worktree/")
}

fn log_transition(
    object_hash: gix_hash::Kind,
    edit: &Edit,
    new: &Target,
    expected: &PreviousValue,
) -> Option<(ObjectId, ObjectId)> {
    match new {
        Target::Object(new_id) => {
            let old_id = edit
                .leaf_referent_previous_oid
                .or(match edit.previous.as_ref() {
                    Some(Target::Object(old_id)) => Some(*old_id),
                    _ => None,
                })
                .unwrap_or_else(|| object_hash.null());
            Some((old_id, *new_id))
        }
        Target::Symbolic(_) => match expected {
            PreviousValue::ExistingMustMatch(Target::Object(new_id)) => Some((object_hash.null(), *new_id)),
            _ => None,
        },
    }
}

fn log_value(
    old_id: ObjectId,
    new_id: ObjectId,
    committer: Option<gix_actor::SignatureRef<'_>>,
    message: &BStr,
) -> Result<gix_reftable::LogValue, Error> {
    let committer = committer.ok_or(Error::MissingCommitter)?.trim();
    if committer.name.find_byteset(b"<>\n").is_some() || committer.email.find_byteset(b"<>\n").is_some() {
        return Err(Error::InvalidIdentity);
    }
    if message.contains(&b'\n') {
        return Err(Error::InvalidLogMessage);
    }
    let time = committer.time().map_err(|_| Error::InvalidLogTime)?;
    let seconds = u64::try_from(time.seconds).map_err(|_| Error::InvalidLogTime)?;
    if time.offset % 60 != 0 {
        return Err(Error::InvalidLogTime);
    }
    let tz_offset = i16::try_from(time.offset / 60).map_err(|_| Error::InvalidLogTime)?;
    Ok(gix_reftable::LogValue::Update {
        old_id,
        new_id,
        name: committer.name.to_owned(),
        email: committer.email.to_owned(),
        time: seconds,
        tz_offset,
        message: message.to_owned(),
    })
}

impl std::fmt::Debug for Transaction<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transaction")
            .field("store", &self.store)
            .field("prepared", &self.prepared.as_ref().map(|prepared| prepared.edits.len()))
            .finish_non_exhaustive()
    }
}
