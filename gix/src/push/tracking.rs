use std::sync::atomic::AtomicBool;

use crate::{
    ObjectId, Remote, Repository, Result,
    error::{ResultExt, message},
};
use gix_ref::{FullName, FullNameRef};

pub(super) fn name(remote: &Remote<'_>, destination: &FullNameRef) -> Result<Option<FullName>> {
    let null_id = remote.repo.object_hash().null();
    let mapping = gix_refspec::MatchGroup::from_fetch_specs(
        remote
            .refspecs(crate::remote::Direction::Fetch)
            .iter()
            .map(|spec| spec.to_ref()),
    )
    .match_lhs_literal(std::iter::once(gix_refspec::match_group::Item {
        full_ref_name: destination.as_bstr(),
        target: &null_id,
        object: None,
    }));
    mapping
        .mappings
        .into_iter()
        .find_map(|mapping| mapping.rhs)
        .map(|name| FullName::try_from(name.into_owned()).or_raise(|| message("Invalid push tracking reference name")))
        .transpose()
}

pub(super) fn tip(
    remote: &Remote<'_>,
    repo: &Repository,
    destination: &FullNameRef,
) -> Result<Option<(FullName, ObjectId)>> {
    let Some(name) = name(remote, destination)? else {
        return Ok(None);
    };
    let Some(mut reference) = repo.try_find_reference(name.as_bstr())? else {
        return Ok(None);
    };
    // Fetch mappings are literal, so a shorthand lookup must not select a different reference.
    if reference.name() != name.as_ref() {
        return Ok(None);
    }
    match reference.follow_to_object() {
        Ok(id) => Ok(Some((name, id.detach()))),
        Err(err) if err.is_not_found() => Ok(None),
        Err(err) => Err(err),
    }
}

pub(super) fn includes(
    repo: &Repository,
    local: &FullNameRef,
    tracking: &FullNameRef,
    remote_id: ObjectId,
    shallow: &[ObjectId],
    interrupt: &AtomicBool,
) -> Result<bool> {
    if repo.try_find_object(remote_id)?.is_none() {
        return Ok(false);
    }
    let Some(remote_id) = super::plan::peel_commit(repo, remote_id)? else {
        return Ok(false);
    };
    let mut buffer = Vec::new();
    let remote_time = repo
        .refs
        .reflog_iter(tracking, &mut buffer)?
        .and_then(Iterator::last)
        .transpose()?
        .map(|line| line.signature.time())
        .transpose()?
        .map_or(0, |time| time.seconds);
    let Some(lines) = repo.refs.reflog_iter(local, &mut buffer)? else {
        return Ok(false);
    };
    let mut candidates = Vec::new();
    for line in lines {
        super::plan::check_interrupt(interrupt)?;
        let line = line?;
        // Git scans backward through the first older entry, including that boundary entry.
        if line.signature.time()?.seconds < remote_time {
            candidates.clear();
        }
        let object_id = line.new_oid();
        if !object_id.is_null() {
            candidates.push(object_id);
        }
    }
    let mut commits = Vec::new();
    for object_id in candidates {
        if repo.try_find_object(object_id)?.is_some()
            && let Some(commit_id) = super::plan::peel_commit(repo, object_id)?
        {
            commits.push(commit_id);
        }
    }
    super::plan::is_ancestor_of_any(repo, remote_id, commits, shallow, interrupt)
}
