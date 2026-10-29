use std::sync::atomic::AtomicBool;

use crate::{
    ObjectId, Remote, Repository, Result,
    error::{ResultExt, bail, message},
};
use gix_ref::{FullName, FullNameRef};

pub(crate) fn update_refs(remote: &Remote<'_>, outcome: &super::Outcome) -> Result<Vec<gix_ref::transaction::RefEdit>> {
    use gix_ref::transaction::{PreviousValue, RefEdit};

    if outcome.dry_run {
        return Ok(Vec::new());
    }
    let mut edits = Vec::new();
    let mut add = |destination: &FullNameRef, new_id: Option<ObjectId>| -> Result<()> {
        let Some(name) = name(remote, destination)? else {
            return Ok(());
        };
        let edit = match new_id.filter(|id| !id.is_null()) {
            Some(object_id) => {
                let object = remote.repo.find_object(object_id).or_raise(|| {
                    message("Push tracking update refers to an unavailable object").with("reference", name.as_bstr())
                })?;
                if name.category() == Some(gix_ref::Category::LocalBranch) && object.kind != gix_object::Kind::Commit {
                    bail!(
                        message("Push tracking branches must refer to commits")
                            .validation()
                            .with("reference", name.as_bstr())
                    );
                }
                RefEdit::update(name, object_id, PreviousValue::Any, "update by push")
            }
            None => RefEdit::delete(name, PreviousValue::Any),
        };
        edits.push(edit.with_deref(true));
        Ok(())
    };
    for update in &outcome.updates {
        if !matches!(update.status, super::Status::Updated { .. } | super::Status::UpToDate) {
            continue;
        }
        let mut reported = false;
        for report in outcome
            .response
            .iter()
            .flat_map(|response| &response.refs)
            .filter(|report| report.name == update.update.destination)
        {
            reported = true;
            if report.error.is_none() {
                add(
                    report
                        .report
                        .ref_name
                        .as_ref()
                        .unwrap_or(&update.update.destination)
                        .as_ref(),
                    report.report.new_id.or(update.update.new_id),
                )?;
            }
        }
        if !reported {
            add(update.update.destination.as_ref(), update.update.new_id)?;
        }
    }
    if edits.is_empty() {
        return Ok(edits);
    }
    let mut repo = remote.repo.clone();
    let namespace = repo.refs.namespace.clone();
    repo.committer_or_set_generic_fallback()?;
    repo.refs.namespace = namespace;
    repo.edit_references(edits)
}

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
