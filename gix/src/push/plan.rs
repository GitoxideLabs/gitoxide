use std::{
    collections::VecDeque,
    sync::atomic::{AtomicBool, Ordering},
};

use gix_object::Exists;

use super::{Lease, Rejection, Status, Update, UpdateOutcome};
use crate::{
    ObjectId, Repository, Result,
    bstr::ByteSlice,
    error::{bail, message},
};

pub(crate) fn advertised_objects(
    source: &Repository,
    refs: &[gix_protocol::handshake::Ref],
    shallow: &[ObjectId],
    interrupt: &AtomicBool,
) -> Result<gix_hashtable::HashSet<ObjectId>> {
    // Senders exclude advertised histories they can resolve locally. Merely
    // existing in the receiver's store does not exclude an object from the
    // incoming pack: dangling and hidden objects can be retransmitted.
    let tips = refs
        .iter()
        .filter_map(|reference| reference.unpack().1)
        .filter(|object_id| source.objects.exists(object_id))
        .map(ToOwned::to_owned);
    let mut advertised = gix_hashtable::HashSet::default();
    for entry in gix_traverse::object::BreadthFirst::new(tips, source)
        .with_object_validation(false)
        .with_shallow_commits(shallow.iter().copied())
    {
        check_interrupt(interrupt)?;
        advertised.insert(entry?.id);
    }
    Ok(advertised)
}

pub(crate) fn updates(
    repo: &Repository,
    remote: &crate::Remote<'_>,
    updates: impl IntoIterator<Item = Update>,
    remote_refs: &[gix_protocol::handshake::Ref],
    shallow: &[ObjectId],
    follow_tags: bool,
    should_interrupt: &AtomicBool,
) -> Result<Vec<UpdateOutcome>> {
    let mut names = std::collections::HashSet::new();
    let mut out = Vec::new();
    for update in updates {
        check_interrupt(should_interrupt)?;
        if !update.destination.as_bstr().starts_with_str("refs/") {
            bail!(message("Push destinations must be fully qualified names under refs/").validation());
        }
        if !names.insert(update.destination.clone()) {
            bail!(message!("Multiple updates target {}", update.destination.as_bstr()).validation());
        }
        let expected_id = match update.lease {
            Lease::Any | Lease::Tracking { .. } => None,
            Lease::MustMatch(id) => id,
        };
        if update
            .new_id
            .into_iter()
            .chain(expected_id)
            .any(|id| id.kind() != repo.object_hash() || id.is_null())
        {
            bail!(message("Push object IDs must be non-null and use the repository's object format").validation());
        }
        let old_id = remote_refs
            .iter()
            .find_map(|reference| {
                let (name, object_id, _) = reference.unpack();
                (name == update.destination.as_bstr()).then(|| object_id.map(ToOwned::to_owned))
            })
            .flatten();
        let status = status(repo, remote, &update, old_id, shallow, should_interrupt)?;
        out.push(UpdateOutcome { update, old_id, status });
    }
    if follow_tags {
        add_follow_tags(repo, &mut out, remote_refs, shallow, should_interrupt)?;
    }
    Ok(out)
}

fn add_follow_tags(
    repo: &Repository,
    updates: &mut Vec<UpdateOutcome>,
    remote_refs: &[gix_protocol::handshake::Ref],
    shallow: &[ObjectId],
    should_interrupt: &AtomicBool,
) -> Result<()> {
    let present: std::collections::HashSet<_> = remote_refs
        .iter()
        .map(|reference| reference.unpack().0.to_owned())
        .chain(
            updates
                .iter()
                .map(|update| update.update.destination.as_bstr().to_owned()),
        )
        .collect();
    let mut candidates = Vec::new();
    for reference in repo.references()?.tags()? {
        check_interrupt(should_interrupt)?;
        let mut reference = reference?;
        if present.contains(reference.name().as_bstr()) {
            continue;
        }
        let tag_id = reference.follow_to_object()?.detach();
        if repo.find_object(tag_id)?.kind == gix_object::Kind::Tag
            && let Some(commit_id) = peel_commit(repo, tag_id)?
        {
            candidates.push((reference.name().to_owned(), tag_id, commit_id));
        }
    }
    if candidates.is_empty() {
        return Ok(());
    }
    let mut tips: Vec<_> = updates.iter().filter_map(|update| update.update.new_id).collect();
    for reference in remote_refs {
        let (name, object_id, _) = reference.unpack();
        // Git retains the advertised tip for deletions as well as untouched references.
        if !updates
            .iter()
            .any(|update| update.update.destination.as_bstr() == name && update.update.new_id.is_some())
        {
            tips.extend(object_id.map(ToOwned::to_owned));
        }
    }
    let mut commit_tips = Vec::new();
    for object_id in tips {
        if repo.try_find_object(object_id)?.is_some()
            && let Some(commit_id) = peel_commit(repo, object_id)?
        {
            commit_tips.push(commit_id);
        }
    }
    let reachable = reachable_ancestors(
        repo,
        candidates.iter().map(|(_, _, commit_id)| *commit_id),
        commit_tips,
        shallow,
        should_interrupt,
    )?;
    updates.extend(
        candidates
            .into_iter()
            .filter(|(_, _, commit_id)| reachable.contains(commit_id))
            .map(|(destination, tag_id, _)| UpdateOutcome {
                update: Update {
                    destination,
                    new_id: Some(tag_id),
                    lease: Lease::Any,
                    force: false,
                },
                old_id: None,
                status: Status::WouldUpdate { forced: false },
            }),
    );
    Ok(())
}

fn status(
    repo: &Repository,
    remote: &crate::Remote<'_>,
    update: &Update,
    old_id: Option<ObjectId>,
    shallow: &[ObjectId],
    should_interrupt: &AtomicBool,
) -> Result<Status> {
    if update.new_id.is_some() && old_id == update.new_id {
        return Ok(Status::UpToDate);
    }
    match update.lease {
        Lease::Any => {}
        Lease::MustMatch(expected) if expected != old_id => return Ok(Status::Rejected(Rejection::StaleLease)),
        Lease::MustMatch(_) => {}
        Lease::Tracking { force_if_includes } => {
            let tracking = super::tracking::tip(remote, repo, update.destination.as_ref())?;
            if tracking.as_ref().map(|(_, id)| *id) != old_id {
                return Ok(Status::Rejected(Rejection::StaleLease));
            }
            if force_if_includes
                && let Some((name, commit_id)) = tracking
                && !super::tracking::includes(
                    repo,
                    update.destination.as_ref(),
                    name.as_ref(),
                    commit_id,
                    shallow,
                    should_interrupt,
                )?
            {
                return Ok(Status::Rejected(Rejection::RemoteUpdated));
            }
        }
    }
    let Some(new_id) = update.new_id else {
        return Ok(Status::WouldUpdate { forced: false });
    };
    let new = repo.find_object(new_id)?;
    if update.destination.as_bstr().starts_with_str("refs/heads/") && new.kind != gix_object::Kind::Commit {
        return Ok(Status::Rejected(Rejection::NonCommitBranch));
    }
    let Some(old_id) = old_id else {
        return Ok(Status::WouldUpdate { forced: false });
    };
    let rejection = if update.destination.as_bstr().starts_with_str("refs/tags/") {
        Some(Rejection::AlreadyExists)
    } else if repo.try_find_object(old_id)?.is_none() {
        Some(Rejection::FetchFirst)
    } else {
        match (peel_commit(repo, old_id)?, peel_commit(repo, new_id)?) {
            (Some(old_id), Some(new_id)) => {
                (!is_ancestor(repo, old_id, new_id, shallow, should_interrupt)?).then_some(Rejection::NonFastForward)
            }
            _ => Some(Rejection::NeedsForce),
        }
    };
    let forced = rejection.is_some() && (update.force || !matches!(update.lease, Lease::Any));
    Ok(match rejection {
        Some(reason) if !forced => Status::Rejected(reason),
        _ => Status::WouldUpdate { forced },
    })
}

pub(super) fn peel_commit(repo: &Repository, mut object_id: ObjectId) -> Result<Option<ObjectId>> {
    let mut seen = gix_hashtable::HashSet::default();
    loop {
        if !seen.insert(object_id) {
            bail!(message("A tag cycle prevents determining commit ancestry").corrupted());
        }
        let object = repo.find_object(object_id)?;
        match object.kind {
            gix_object::Kind::Commit => return Ok(Some(object_id)),
            gix_object::Kind::Tag => {
                object_id = gix_object::TagRefIter::from_bytes(&object.data, repo.object_hash()).target_id()?;
            }
            _ => return Ok(None),
        }
    }
}

pub(crate) fn is_ancestor(
    repo: &Repository,
    old_id: ObjectId,
    new_id: ObjectId,
    shallow: &[ObjectId],
    should_interrupt: &AtomicBool,
) -> Result<bool> {
    is_ancestor_of_any(repo, old_id, [new_id], shallow, should_interrupt)
}

pub(super) fn is_ancestor_of_any(
    repo: &Repository,
    old_id: ObjectId,
    new_ids: impl IntoIterator<Item = ObjectId>,
    shallow: &[ObjectId],
    should_interrupt: &AtomicBool,
) -> Result<bool> {
    Ok(!reachable_ancestors(repo, [old_id], new_ids, shallow, should_interrupt)?.is_empty())
}

fn reachable_ancestors(
    repo: &Repository,
    old_ids: impl IntoIterator<Item = ObjectId>,
    new_ids: impl IntoIterator<Item = ObjectId>,
    shallow: &[ObjectId],
    should_interrupt: &AtomicBool,
) -> Result<gix_hashtable::HashSet<ObjectId>> {
    let mut wanted: gix_hashtable::HashSet<_> = old_ids.into_iter().collect();
    let mut found = gix_hashtable::HashSet::default();
    let mut pending = VecDeque::from_iter(new_ids);
    let mut seen = gix_hashtable::HashSet::default();
    while let Some(commit_id) = pending.pop_front() {
        check_interrupt(should_interrupt)?;
        if wanted.remove(&commit_id) {
            found.insert(commit_id);
            if wanted.is_empty() {
                break;
            }
        }
        if !seen.insert(commit_id) || shallow.contains(&commit_id) {
            continue;
        }
        let object = repo.find_object(commit_id)?;
        if object.kind != gix_object::Kind::Commit {
            bail!(message("A commit parent has the wrong object kind").corrupted());
        }
        for edge in gix_object::Data::new(&object.data, object.kind, repo.object_hash()).references() {
            let edge = edge?;
            if edge.kind == gix_object::Kind::Commit {
                pending.push_back(edge.id);
            }
        }
    }
    Ok(found)
}

pub(crate) fn check_interrupt(should_interrupt: &AtomicBool) -> Result<()> {
    if should_interrupt.load(Ordering::Relaxed) {
        bail!(message("Push interrupted").cancelled());
    }
    Ok(())
}
