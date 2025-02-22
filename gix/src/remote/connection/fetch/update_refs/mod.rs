#![allow(clippy::result_large_err)]
use std::{collections::BTreeMap, path::PathBuf};

use gix_object::Exists;
use gix_ref::{
    transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog},
    Target, TargetRef,
};

use crate::{
    ext::ObjectIdExt,
    remote::{
        fetch,
        fetch::{
            refmap::Source,
            refs::update::{Mode, TypeChange},
            RefLogMessage,
        },
    },
    Repository,
};

///
pub mod update;

/// Information about the update of a single reference, corresponding the respective entry in [`RefMap::mappings`][crate::remote::fetch::RefMap::mappings].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    /// The way the update was performed.
    pub mode: Mode,
    ///  If not `None`, the update also affects the type of the reference. This also implies that `edit_index` is not None.
    pub type_change: Option<TypeChange>,
    /// The index to the edit that was created from the corresponding mapping, or `None` if there was no local ref.
    pub edit_index: Option<usize>,
}

impl From<Mode> for Update {
    fn from(mode: Mode) -> Self {
        Update {
            mode,
            type_change: None,
            edit_index: None,
        }
    }
}

/// Update all refs as derived from `refmap.mappings` and produce an `Outcome` informing about all applied changes in detail, with each
/// [`update`][Update] corresponding to the [`fetch::Mapping`] of at the same index.
/// If `dry_run` is true, ref transactions won't actually be applied, but are assumed to work without error so the underlying
/// `repo` is not actually changed. Also it won't perform an 'object exists' check as these are likely not to exist as the pack
/// wasn't fetched either.
/// `action` is the prefix used for reflog entries, and is typically "fetch".
///
/// It can be used to produce typical information that one is used to from `git fetch`.
///
/// We will reject updates only if…
///
/// * …fast-forward rules are violated
/// * …the local ref is currently checked out
/// * …existing refs would not become 'unborn', i.e. point to a reference that doesn't exist and won't be created due to ref-specs
///
/// With these safeguards in place, one can handle each naturally and implement mirrors or bare repos easily.
#[allow(clippy::too_many_arguments)]
pub(crate) fn update(
    repo: &Repository,
    message: RefLogMessage,
    mappings: &[fetch::refmap::Mapping],
    refspecs: &[gix_refspec::RefSpec],
    extra_refspecs: &[gix_refspec::RefSpec],
    fetch_tags: fetch::Tags,
    dry_run: fetch::DryRun,
    write_packed_refs: fetch::WritePackedRefs,
) -> Result<update::Outcome, update::Error> {
    let _span = gix_trace::detail!("update_refs()", mappings = mappings.len());
    let mut edits = Vec::new();
    let mut updates = Vec::new();
    let mut edit_indices_to_validate = Vec::new();

    let implicit_tag_refspec = fetch_tags
        .to_refspec()
        .filter(|_| matches!(fetch_tags, crate::remote::fetch::Tags::Included));
    for (remote, local, spec, is_implicit_tag) in mappings.iter().filter_map(
        |fetch::refmap::Mapping {
             remote,
             local,
             spec_index,
         }| {
            spec_index.get(refspecs, extra_refspecs).map(|spec| {
                (
                    remote,
                    local,
                    spec,
                    implicit_tag_refspec.is_some_and(|tag_spec| spec.to_ref() == tag_spec),
                )
            })
        },
    ) {
        // `None` only if unborn.
        let remote_id = remote.as_id();
        if matches!(dry_run, fetch::DryRun::No) && !remote_id.map_or(true, |id| repo.objects.exists(id)) {
            if let Some(remote_id) = remote_id.filter(|id| !repo.objects.exists(id)) {
                let update = if is_implicit_tag {
                    Mode::ImplicitTagNotSentByRemote.into()
                } else {
                    // Assure the ODB is not to blame for the missing object.
                    repo.try_find_object(remote_id)?;
                    Mode::RejectedSourceObjectNotFound { id: remote_id.into() }.into()
                };
                updates.push(update);
                continue;
            }
        }
        let mut checked_out_branches = worktree_branches(repo)?;
        let (mode, edit_index, type_change) = match local {
            Some(name) => {
                let (mode, reflog_message, name, previous_value) = match repo.try_find_reference(name)? {
                    Some(existing) => {
                        if let Some(wt_dirs) = checked_out_branches.get_mut(existing.name()) {
                            wt_dirs.sort();
                            wt_dirs.dedup();
                            let mode = Mode::RejectedCurrentlyCheckedOut {
                                worktree_dirs: wt_dirs.to_owned(),
                            };
                            updates.push(mode.into());
                            continue;
                        }

                        match existing
                            .try_id()
                            .map_or_else(|| existing.clone().peel_to_id_in_place(), Ok)
                            .map(crate::Id::detach)
                        {
                            Ok(local_id) => {
                                let remote_id = match remote_id {
                                    Some(id) => id,
                                    None => {
                                        // we don't allow to go back to unborn state if there is a local reference already present.
                                        // Note that we will be changing it to a symbolic reference just fine.
                                        updates.push(Mode::RejectedToReplaceWithUnborn.into());
                                        continue;
                                    }
                                };
                                let (mode, reflog_message) = if local_id == remote_id {
                                    (Mode::NoChangeNeeded, "no update will be performed")
                                } else if let Some(gix_ref::Category::Tag) = existing.name().category() {
                                    if spec.allow_non_fast_forward() {
                                        (Mode::Forced, "updating tag")
                                    } else {
                                        updates.push(Mode::RejectedTagUpdate.into());
                                        continue;
                                    }
                                } else {
                                    let mut force = spec.allow_non_fast_forward();
                                    let is_fast_forward = match dry_run {
                                        fetch::DryRun::No => {
                                            let ancestors = repo
                                                .find_object(local_id)?
                                                .try_into_commit()
                                                .map_err(|_| ())
                                                .and_then(|c| c.committer().map(|a| a.time.seconds).map_err(|_| ()))
                                                .and_then(|local_commit_time| {
                                                    remote_id
                                                        .to_owned()
                                                        .ancestors(&repo.objects)
                                                        .sorting(
                                                            gix_traverse::commit::simple::Sorting::ByCommitTimeCutoff {
                                                                order: Default::default(),
                                                                seconds: local_commit_time,
                                                            },
                                                        )
                                                        .map_err(|_| ())
                                                });
                                            match ancestors {
                                                Ok(mut ancestors) => {
                                                    ancestors.any(|cid| cid.is_ok_and(|c| c.id == local_id))
                                                }
                                                Err(_) => {
                                                    force = true;
                                                    false
                                                }
                                            }
                                        }
                                        fetch::DryRun::Yes => true,
                                    };
                                    if is_fast_forward {
                                        (
                                            Mode::FastForward,
                                            matches!(dry_run, fetch::DryRun::Yes)
                                                .then(|| "fast-forward (guessed in dry-run)")
                                                .unwrap_or("fast-forward"),
                                        )
                                    } else if force {
                                        (Mode::Forced, "forced-update")
                                    } else {
                                        updates.push(Mode::RejectedNonFastForward.into());
                                        continue;
                                    }
                                };
                                (
                                    mode,
                                    reflog_message,
                                    existing.name().to_owned(),
                                    PreviousValue::MustExistAndMatch(existing.target().into_owned()),
                                )
                            }
                            Err(crate::reference::peel::Error::ToId(gix_ref::peel::to_id::Error::FollowToObject(
                                gix_ref::peel::to_object::Error::Follow(_),
                            ))) => {
                                // An unborn reference, always allow it to be changed to whatever the remote wants.
                                (
                                    if existing.target().try_name().map(gix_ref::FullNameRef::as_bstr)
                                        == remote.as_target()
                                    {
                                        Mode::NoChangeNeeded
                                    } else {
                                        Mode::Forced
                                    },
                                    "change unborn ref",
                                    existing.name().to_owned(),
                                    PreviousValue::MustExistAndMatch(existing.target().into_owned()),
                                )
                            }
                            Err(err) => return Err(err.into()),
                        }
                    }
                    None => {
                        let name: gix_ref::FullName = name.try_into()?;
                        let reflog_msg = match name.category() {
                            Some(gix_ref::Category::Tag) => "storing tag",
                            Some(gix_ref::Category::LocalBranch) => "storing head",
                            _ => "storing ref",
                        };
                        (
                            Mode::New,
                            reflog_msg,
                            name,
                            PreviousValue::ExistingMustMatch(new_value_by_remote(repo, remote, mappings)?),
                        )
                    }
                };

                let new = new_value_by_remote(repo, remote, mappings)?;
                let type_change = match (&previous_value, &new) {
                    (
                        PreviousValue::ExistingMustMatch(Target::Object(_))
                        | PreviousValue::MustExistAndMatch(Target::Object(_)),
                        Target::Symbolic(_),
                    ) => Some(TypeChange::DirectToSymbolic),
                    (
                        PreviousValue::ExistingMustMatch(Target::Symbolic(_))
                        | PreviousValue::MustExistAndMatch(Target::Symbolic(_)),
                        Target::Object(_),
                    ) => Some(TypeChange::SymbolicToDirect),
                    _ => None,
                };
                // We are here because this edit should work and fast-forward rules are respected.
                // But for setting a symref-target, we have to be sure that the target already exists
                // or will exists. To be sure all rules are respected, we delay the check to when the
                // edit-list has been built.
                let edit_index = edits.len();
                if matches!(new, Target::Symbolic(_)) {
                    let anticipated_update_index = updates.len();
                    edit_indices_to_validate.push((anticipated_update_index, edit_index));
                }
                let edit = RefEdit {
                    change: Change::Update {
                        log: LogChange {
                            mode: RefLog::AndReference,
                            force_create_reflog: false,
                            message: message.compose(reflog_message),
                        },
                        expected: previous_value,
                        new,
                    },
                    name,
                    // We must not deref symrefs or we will overwrite their destination, which might be checked out
                    // and we don't check for that case.
                    deref: false,
                };
                edits.push(edit);
                (mode, Some(edit_index), type_change)
            }
            None => (Mode::NoChangeNeeded, None, None),
        };
        updates.push(Update {
            mode,
            type_change,
            edit_index,
        });
    }

    for (update_index, edit_index) in edit_indices_to_validate {
        let edit = &edits[edit_index];
        if update_needs_adjustment_as_edits_symbolic_target_is_missing(edit, repo, &edits) {
            let edit = &mut edits[edit_index];
            let update = &mut updates[update_index];

            update.mode = Mode::RejectedToReplaceWithUnborn;
            update.type_change = None;

            match edit.change {
                Change::Update {
                    ref expected,
                    ref mut new,
                    ref mut log,
                    ..
                } => match expected {
                    PreviousValue::MustExistAndMatch(existing) => {
                        *new = existing.clone();
                        log.message = "no-op".into();
                    }
                    _ => unreachable!("at this point it can only be one variant"),
                },
                Change::Delete { .. } => {
                    unreachable!("we don't do that here")
                }
            }
        }
    }

    let edits = match dry_run {
        fetch::DryRun::No => {
            let _span = gix_trace::detail!("apply", edits = edits.len());
            let (file_lock_fail, packed_refs_lock_fail) = repo
                .config
                .lock_timeout()
                .map_err(crate::reference::edit::Error::from)?;
            repo.refs
                .transaction()
                .packed_refs(
                    match write_packed_refs {
                        fetch::WritePackedRefs::Only => {
                            gix_ref::file::transaction::PackedRefs::DeletionsAndNonSymbolicUpdatesRemoveLooseSourceReference(Box::new(&repo.objects))},
                        fetch::WritePackedRefs::Never => gix_ref::file::transaction::PackedRefs::DeletionsOnly
                    }
                )
                .prepare(edits, file_lock_fail, packed_refs_lock_fail)
                .map_err(crate::reference::edit::Error::from)?
                .commit(repo.committer().transpose().map_err(|err| update::Error::EditReferences(crate::reference::edit::Error::ParseCommitterTime(err)))?)
                .map_err(crate::reference::edit::Error::from)?
        }
        fetch::DryRun::Yes => edits,
    };

    Ok(update::Outcome { edits, updates })
}

/// Figure out if target of `edit` points to a reference that doesn't exist in `repo` and won't exist as it's not in any of `edits`.
/// If so, return true.
fn update_needs_adjustment_as_edits_symbolic_target_is_missing(
    edit: &RefEdit,
    repo: &Repository,
    edits: &[RefEdit],
) -> bool {
    match edit.change.new_value().expect("here we need a symlink") {
        TargetRef::Object(_) => unreachable!("BUG: we already know it's symbolic"),
        TargetRef::Symbolic(new_target_ref) => {
            match &edit.change {
                Change::Update { expected, .. } => match expected {
                    PreviousValue::MustExistAndMatch(current_target) => {
                        if let Target::Symbolic(current_target_name) = current_target {
                            if current_target_name.as_ref() == new_target_ref {
                                return false; // no-op are always fine
                            }
                            let current_is_unborn = repo.refs.try_find(current_target_name).ok().flatten().is_none();
                            if current_is_unborn {
                                return false;
                            }
                        }
                    }
                    PreviousValue::ExistingMustMatch(_) => return false, // this means the ref doesn't exist locally, so we can create unborn refs anyway
                    _ => {
                        unreachable!("BUG: we don't do that here")
                    }
                },
                Change::Delete { .. } => {
                    unreachable!("we don't ever delete here")
                }
            }
            let target_ref_exists_locally = repo.refs.try_find(new_target_ref).ok().flatten().is_some();
            if target_ref_exists_locally {
                return false;
            }

            let target_ref_will_be_created = edits.iter().any(|edit| edit.name.as_ref() == new_target_ref);
            !target_ref_will_be_created
        }
    }
}

fn new_value_by_remote(
    repo: &Repository,
    remote: &Source,
    mappings: &[fetch::refmap::Mapping],
) -> Result<Target, update::Error> {
    let remote_id = remote.as_id();
    Ok(
        if let Source::Ref(
            gix_protocol::handshake::Ref::Symbolic { target, .. } | gix_protocol::handshake::Ref::Unborn { target, .. },
        ) = &remote
        {
            match mappings.iter().find_map(|m| {
                m.remote.as_name().and_then(|name| {
                    (name == target)
                        .then(|| m.local.as_ref().and_then(|local| local.try_into().ok()))
                        .flatten()
                })
            }) {
                // Map the target on the remote to the local branch name, which should be covered by refspecs.
                Some(local_branch) => {
                    // This is always safe because…
                    // - the reference may exist already
                    // - if it doesn't exist it will be created - we are here because it's in the list of mappings after all
                    // - if it exists and is updated, and the update is rejected due to non-fastforward for instance, the
                    //   target reference still exists and we can point to it.
                    Target::Symbolic(local_branch)
                }
                None => {
                    // If we can't map it, it's usually a an unborn branch causing this, or a the target isn't covered
                    // by any refspec so we don't officially pull it in.
                    match remote_id {
                        Some(desired_id) => {
                            if repo.try_find_reference(target)?.is_some() {
                                // We are allowed to change a direct reference to a symbolic one, which may point to other objects
                                // than the remote. The idea is that we are fine as long as the resulting refs are valid.
                                Target::Symbolic(target.try_into()?)
                            } else {
                                // born branches that we don't have in our refspecs we create peeled. That way they can be used.
                                Target::Object(desired_id.to_owned())
                            }
                        }
                        // Unborn branches we create as such, with the location they point to on the remote which helps mirroring.
                        None => Target::Symbolic(target.try_into()?),
                    }
                }
            }
        } else {
            Target::Object(remote_id.expect("unborn case handled earlier").to_owned())
        },
    )
}

fn insert_head(
    head: Option<crate::Head<'_>>,
    out: &mut BTreeMap<gix_ref::FullName, Vec<PathBuf>>,
) -> Result<(), update::Error> {
    if let Some((head, wd)) = head.and_then(|head| head.repo.work_dir().map(|wd| (head, wd))) {
        out.entry("HEAD".try_into().expect("valid"))
            .or_default()
            .push(wd.to_owned());
        let mut ref_chain = Vec::new();
        let mut cursor = head.try_into_referent();
        while let Some(ref_) = cursor {
            ref_chain.push(ref_.name().to_owned());
            cursor = ref_.follow().transpose()?;
        }
        for name in ref_chain {
            out.entry(name).or_default().push(wd.to_owned());
        }
    }
    Ok(())
}

fn worktree_branches(repo: &Repository) -> Result<BTreeMap<gix_ref::FullName, Vec<PathBuf>>, update::Error> {
    let mut map = BTreeMap::new();
    insert_head(repo.head().ok(), &mut map)?;
    for proxy in repo.worktrees()? {
        let repo = proxy.into_repo_with_possibly_inaccessible_worktree()?;
        insert_head(repo.head().ok(), &mut map)?;
    }
    Ok(map)
}

#[cfg(test)]
mod tests;
