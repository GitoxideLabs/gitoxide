use std::{collections::HashSet, path::Path};

use anyhow::{Context, Result};
use gix::{
    ObjectId,
    bstr::{BStr, BString, ByteSlice},
    refs::Target,
};

use crate::{history, open_repository};

const HEADER: &[u8] = b"tix-rebase";
const ONTO: &[u8] = b"onto ";
const RETURN_TO: &[u8] = b"tix-review-return-to";

#[derive(Debug)]
pub(crate) struct Started {
    pub commit: ObjectId,
    pub reference: gix::refs::FullName,
    pub checkout_error: Option<anyhow::Error>,
}

pub(crate) struct Finished {
    pub commit: ObjectId,
    pub outcome: super::rebase::Outcome,
}

pub(crate) enum Finish {
    Complete(Finished),
    Conflict(super::rebase::Conflict),
    SelectReturn { tip: ObjectId },
}

pub(crate) fn reference(commit: &gix::objs::Commit) -> Result<Option<gix::refs::FullName>> {
    commit
        .extra_headers
        .iter()
        .find_map(|(name, value)| {
            (name.as_slice() == HEADER)
                .then(|| value.as_slice().strip_prefix(ONTO))
                .flatten()
        })
        .map(|name| {
            if history::review_number(name.as_bstr()).is_none() {
                anyhow::bail!("review commit names an invalid review reference");
            }
            BString::from(name)
                .try_into()
                .context("review commit names an invalid reference")
        })
        .transpose()
}

pub(crate) fn is_review(commit: &gix::objs::Commit) -> bool {
    reference(commit).ok().flatten().is_some()
}

pub(super) fn remove_identity(commit: &mut gix::objs::Commit, review: &BStr) {
    commit.extra_headers.retain(|(name, value)| {
        !(name.as_slice() == HEADER && value.as_slice().strip_prefix(ONTO) == Some(review.as_ref()))
            && name.as_slice() != RETURN_TO
    });
}

pub(super) fn return_to(commit: &gix::objs::Commit) -> Result<Option<gix::refs::FullName>> {
    commit
        .extra_headers
        .iter()
        .find(|(name, _)| name.as_slice() == RETURN_TO)
        .map(|(_, value)| {
            gix::refs::FullName::try_from(value.clone()).context("review commit names an invalid return reference")
        })
        .transpose()
}

pub(super) fn deletions(
    repo: &gix::Repository,
    commit: &gix::objs::Commit,
) -> Result<Vec<(gix::refs::FullName, gix::refs::Target)>> {
    let Some(name) = reference(commit)? else {
        return Ok(Vec::new());
    };
    resources(repo, name)
}

pub(super) fn resources(
    repo: &gix::Repository,
    name: gix::refs::FullName,
) -> Result<Vec<(gix::refs::FullName, gix::refs::Target)>> {
    let stash = stash_reference(name.as_bstr())?;
    let mut out = Vec::new();
    for name in [name, stash] {
        if let Some(reference) = repo.try_find_reference(name.as_ref())? {
            out.push((name, reference.target().into_owned()));
        }
    }
    Ok(out)
}

pub(super) fn stash_reference(review: &BStr) -> Result<gix::refs::FullName> {
    let number = history::review_number(review).context("review reference has no numeric identity")?;
    format!(
        "{}{}",
        String::from_utf8_lossy(history::REVIEW_STASH_PREFIX),
        number.to_str_lossy()
    )
    .try_into()
    .context("generated an invalid review stash reference")
}

/// Ordinary additions between the reviewed history and the review, oldest first.
pub(super) fn inserted_parents(
    repo: &gix::Repository,
    graph: &history::HistoryGraph,
    review_commit_id: ObjectId,
    tip_commit_id: ObjectId,
) -> Result<Vec<ObjectId>> {
    let mut reviewed_ancestors = HashSet::new();
    let mut pending = vec![tip_commit_id];
    while let Some(commit_id) = pending.pop() {
        if reviewed_ancestors.insert(commit_id) {
            pending.extend(graph.parents_of(commit_id).unwrap_or_default());
        }
    }
    let mut commit_id = repo
        .find_commit(review_commit_id)?
        .parent_ids()
        .next()
        .context("a review commit must have a base")?
        .detach();
    let mut prefix = Vec::new();
    while !reviewed_ancestors.contains(&commit_id) {
        anyhow::ensure!(
            graph.is_in_edit_scope(commit_id) && !graph.is_read_only(commit_id),
            "added review ancestor {commit_id} is outside editable history"
        );
        let commit = repo.find_commit(commit_id)?.decode()?.into_owned()?;
        anyhow::ensure!(
            !super::rebase::is_pending(&commit),
            "added review ancestor {commit_id} has a pending rebase"
        );
        anyhow::ensure!(
            commit.parents.len() == 1 && !is_review(&commit) && !super::auto_merge::is_auto_merge(&commit),
            "added review ancestor {commit_id} must be an ordinary single-parent commit"
        );
        prefix.push(commit_id);
        commit_id = commit.parents[0];
    }
    prefix.reverse();
    Ok(prefix)
}

#[tracing::instrument(skip_all, fields(%tip, %base))]
pub(crate) fn start(
    repository_path: &Path,
    bare: bool,
    graph: &history::HistoryGraph,
    tip: ObjectId,
    base: ObjectId,
) -> Result<Started> {
    let repo = open_repository(repository_path, bare, false).context("could not open repository to start review")?;
    let workdir = repo.workdir().context("review requires a worktree")?.to_owned();
    let head = repo.head().context("could not read HEAD before review")?;
    let restore = (
        head.referent_name().map(ToOwned::to_owned),
        head.id().map(gix::Id::detach),
    );
    if tip == base || !graph.is_ancestor(base, tip) {
        anyhow::bail!("the review base must be an ancestor of the reviewed commit");
    }
    for (label, id) in [("reviewed commit", tip), ("review base", base)] {
        let commit = repo
            .find_commit(id)
            .with_context(|| format!("could not find {label}"))?
            .decode()?
            .into_owned()?;
        if super::rebase::is_pending(&commit) {
            anyhow::bail!("{label} has a pending rebase");
        }
    }
    let name = next_reference(&repo)?;
    let departure_pin = match restore.1 {
        Some(id) => {
            let target = restore.0.clone().map_or(Target::Object(id), Target::Symbolic);
            let pin_name = return_pin_reference(name.as_bstr())?;
            Some(super::time_travel::create_named_pin(
                &repo,
                pin_name,
                target,
                id,
                "tix review departure",
            )?)
        }
        None => None,
    };

    let mut commit = gix::objs::Commit {
        tree: repo.find_commit(base)?.tree_id()?.detach(),
        parents: [base].into_iter().collect(),
        author: repo
            .author()
            .context("no Git author is configured")?
            .context("could not resolve the Git author")?
            .to_owned()?,
        committer: repo
            .committer()
            .context("no Git committer is configured")?
            .context("could not resolve the Git committer")?
            .to_owned()?,
        encoding: None,
        message: "review".into(),
        extra_headers: Vec::new(),
    };
    commit
        .extra_headers
        .push((HEADER.into(), format!("onto {name}").into()));
    let return_to = departure_pin.as_ref().map(|pin| pin.name.clone());
    if let Some(return_to) = return_to {
        commit
            .extra_headers
            .push((RETURN_TO.into(), return_to.as_bstr().to_owned()));
    }
    crate::patch_id::refresh(&repo, &mut commit)?;
    let id = repo
        .write_object(&commit)
        .context("could not write review commit")?
        .detach();
    drop(repo);

    let review_name = name.as_bstr().to_str_lossy();
    let create_ref = git(&workdir, ["update-ref", review_name.as_ref(), &tip.to_string()]);
    if let Err(err) = create_ref {
        remove_new_departure_pin(repository_path, bare, departure_pin.as_ref())?;
        return Err(err.context("could not create review reference"));
    }

    let checkout_error = (|| {
        git(&workdir, ["checkout", "--quiet", "--detach", &tip.to_string()])
            .context("could not check out the reviewed commit")?;
        git(
            &workdir,
            ["update-ref", "--no-deref", "HEAD", &id.to_string(), &tip.to_string()],
        )
        .context("could not attach the worktree to the review commit")?;
        git(&workdir, ["read-tree", &id.to_string()]).context("could not reset the index to the review base")
    })()
    .err();
    Ok(Started {
        commit: id,
        reference: name,
        checkout_error,
    })
}

#[tracing::instrument(skip_all, fields(%review))]
#[cfg(test)]
pub(crate) fn finish(
    repo: gix::Repository,
    graph: &history::HistoryGraph,
    review: ObjectId,
    fallback: Option<ObjectId>,
) -> Result<Finish> {
    finish_with_progress(
        repo,
        graph,
        review,
        fallback,
        super::rebase::CheckoutOptions::default(),
        |_| {},
    )
}

pub(crate) fn finish_with_progress(
    repo: gix::Repository,
    graph: &history::HistoryGraph,
    review: ObjectId,
    fallback: Option<ObjectId>,
    checkout_options: super::rebase::CheckoutOptions<'_>,
    report: impl FnMut(super::rebase::Progress),
) -> Result<Finish> {
    let workdir = repo
        .workdir()
        .context("finishing review requires a worktree")?
        .to_owned();
    let head = repo.head_id()?.detach();
    if !graph.is_ancestor(review, head) {
        anyhow::bail!("HEAD must be the review commit or one of its successors before it can be finished");
    }
    ensure_clean(&workdir)?;
    let commit = repo.find_commit(review)?.decode()?.into_owned()?;
    let review_ref = reference(&commit)?.context("the selected commit is not an active review")?;
    let base = commit
        .parents
        .first()
        .copied()
        .context("a review commit must have a base")?;
    let mut reference = repo
        .find_reference(review_ref.as_ref())
        .context("the review reference is missing")?;
    let legacy_reattach = reference.target().try_name().map(ToOwned::to_owned);
    let tip = reference
        .peel_to_id()
        .context("the review reference does not resolve")?
        .detach();
    let mut delete_refs = resources(&repo, review_ref.clone())?;
    let return_name = return_to(&commit)?.or(legacy_reattach);
    let has_return = return_name.is_some();
    let checkout = if let Some(id) = fallback {
        if !graph.is_ancestor(tip, id) {
            anyhow::bail!("the selected review return commit does not descend from the reviewed commit");
        }
        Some((id, None))
    } else {
        return_name
            .as_ref()
            .map(|name| {
                let Some(mut reference) = repo.try_find_reference(name.as_ref())? else {
                    return Ok(None);
                };
                let checkout_reference = if name.as_bstr().starts_with(history::PIN_PREFIX) {
                    reference.target().try_name().map(ToOwned::to_owned)
                } else {
                    Some(name.clone())
                };
                let id = reference
                    .peel_to_id()
                    .context("the review return reference does not resolve")?
                    .detach();
                if !graph.is_ancestor(tip, id) {
                    anyhow::bail!("the review return reference no longer descends from the reviewed commit");
                }
                Ok(Some((id, checkout_reference)))
            })
            .transpose()?
            .flatten()
    };
    if fallback.is_none() && has_return && checkout.is_none() {
        return Ok(Finish::SelectReturn { tip });
    }
    if let Some(name) = return_name
        .as_ref()
        .filter(|name| name.as_bstr().starts_with(history::REVIEW_PIN_PREFIX))
        && let Some(reference) = repo.try_find_reference(name.as_ref())?
    {
        delete_refs.push((name.clone(), reference.target().into_owned()));
    }
    for (label, id) in [("reviewed commit", tip), ("review base", base)] {
        let endpoint = repo.find_commit(id)?.decode()?.into_owned()?;
        if super::rebase::is_pending(&endpoint) {
            anyhow::bail!("{label} has a pending rebase");
        }
    }
    match super::rebase::finish_review_with_progress(
        &repo,
        graph,
        review,
        tip,
        review_ref,
        delete_refs,
        checkout,
        checkout_options,
        report,
    )? {
        super::rebase::Perform::Complete(outcome) => {
            let finished = outcome
                .map(review)
                .context("finishing review did not produce a commit")?;
            Ok(Finish::Complete(Finished {
                commit: finished,
                outcome,
            }))
        }
        super::rebase::Perform::Conflict(conflict) => Ok(Finish::Conflict(conflict)),
    }
}

pub(super) fn ensure_clean(workdir: &Path) -> Result<()> {
    if is_dirty(workdir)? {
        anyhow::bail!("review requires a clean index and worktree");
    }
    Ok(())
}

pub(super) fn is_dirty(workdir: &Path) -> Result<bool> {
    let output = crate::git_command(workdir)
        .args(["status", "--porcelain=v1", "--untracked-files=all"])
        .output()
        .context("could not inspect worktree status")?;
    if !output.status.success() {
        anyhow::bail!("{}", String::from_utf8_lossy(&output.stderr).trim());
    }
    Ok(!output.stdout.is_empty())
}

fn next_reference(repo: &gix::Repository) -> Result<gix::refs::FullName> {
    for number in 1_u64.. {
        let name: gix::refs::FullName = format!("{}{number}", String::from_utf8_lossy(history::REVIEW_PREFIX))
            .try_into()
            .context("generated an invalid review reference")?;
        let return_pin = return_pin_reference(name.as_bstr())?;
        if repo.try_find_reference(name.as_ref())?.is_none() && repo.try_find_reference(return_pin.as_ref())?.is_none()
        {
            return Ok(name);
        }
    }
    unreachable!("u64 review numbers cannot be exhausted")
}

fn return_pin_reference(review: &BStr) -> Result<gix::refs::FullName> {
    let number = history::review_number(review).context("review reference has no numeric identity")?;
    format!(
        "{}{}",
        String::from_utf8_lossy(history::REVIEW_PIN_PREFIX),
        number.to_str_lossy()
    )
    .try_into()
    .context("generated an invalid review return pin reference")
}

fn git<const N: usize>(workdir: &Path, args: [&str; N]) -> Result<()> {
    let output = crate::git_command(workdir).args(args).output()?;
    if output.status.success() {
        Ok(())
    } else {
        anyhow::bail!("{}", String::from_utf8_lossy(&output.stderr).trim())
    }
}

fn remove_new_departure_pin(repository_path: &Path, bare: bool, pin: Option<&history::Pin>) -> Result<()> {
    let Some(pin) = pin else { return Ok(()) };
    let repo =
        open_repository(repository_path, bare, false).context("could not reopen repository to remove review pin")?;
    super::time_travel::delete_pin(&repo, pin).context("could not remove review departure pin")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn child(repo: &gix::Repository, parent_commit_id: ObjectId, path: &str) -> Result<ObjectId> {
        let mut commit = repo.find_commit(parent_commit_id)?.decode()?.into_owned()?;
        let mut tree = repo.find_tree(commit.tree)?.edit()?;
        tree.upsert(
            path,
            gix::objs::tree::EntryKind::Blob,
            repo.write_blob(format!("{path}\n"))?,
        )?;
        commit.tree = tree.write()?.detach();
        commit.parents = [parent_commit_id].into_iter().collect();
        commit.message = format!("add {path}\n").into();
        commit.extra_headers.clear();
        Ok(repo.write_object(&commit)?.detach())
    }

    fn review_with_prefix(
        count: usize,
    ) -> gix_testtools::Result<(gix_testtools::tempfile::TempDir, Started, Vec<ObjectId>)> {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let tip_commit_id = repo.rev_parse_single("HEAD~1")?.detach();
        let base_commit_id = repo.rev_parse_single("HEAD~2")?.detach();
        let graph = super::super::loaded_graph(&repo)?;
        drop(repo);
        let mut started = start(fixture.path(), false, &graph, tip_commit_id, base_commit_id)?;
        assert!(started.checkout_error.is_none(), "the review starts successfully");

        let repo = crate::test_repository::open(fixture.path())?;
        let mut review = repo.find_commit(started.commit)?.decode()?.into_owned()?;
        let mut tree = repo.find_commit(tip_commit_id)?.tree()?.edit()?;
        let mut parent_commit_id = base_commit_id;
        let mut prefix = Vec::new();
        // Independent additions below the review must remain distinct when it is finished.
        for number in 0..count {
            let path = format!("extra-{number}");
            parent_commit_id = child(&repo, parent_commit_id, &path)?;
            prefix.push(parent_commit_id);
            repo.reference(
                format!("refs/heads/{path}"),
                parent_commit_id,
                gix::refs::transaction::PreviousValue::MustNotExist,
                "retain an inserted review parent",
            )?;
            tree.upsert(
                &path,
                gix::objs::tree::EntryKind::Blob,
                repo.write_blob(format!("{path}\n"))?,
            )?;
        }
        tree.upsert(
            "review-fix",
            gix::objs::tree::EntryKind::Blob,
            repo.write_blob("review fix\n")?,
        )?;
        review.tree = tree.write()?.detach();
        review.parents = [parent_commit_id].into_iter().collect();
        crate::change_id::inherit(&repo, &mut review, started.commit)?;
        crate::patch_id::refresh(&repo, &mut review)?;
        started.commit = repo.write_object(&review)?.detach();
        run(
            fixture.path(),
            &["checkout", "-q", "--detach", "--force", &started.commit.to_string()],
        )?;
        Ok((fixture, started, prefix))
    }

    fn run(path: &Path, args: &[&str]) -> gix_testtools::Result<Vec<u8>> {
        let output = gix_testtools::git_command(path)
            .args(args)
            // These fixtures deliberately configure their identity in the local repository.
            .env_remove("GIT_AUTHOR_NAME")
            .env_remove("GIT_AUTHOR_EMAIL")
            .env_remove("GIT_COMMITTER_NAME")
            .env_remove("GIT_COMMITTER_EMAIL")
            .env("GIT_AUTHOR_DATE", "2001-01-01T00:00:00 +0000")
            .env("GIT_COMMITTER_DATE", "2001-01-01T00:00:00 +0000")
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        Ok(output.stdout)
    }

    #[test]
    fn finishing_preserves_inserted_parents_and_their_distinct_patches() -> gix_testtools::Result {
        for count in [1, 2] {
            let (fixture, started, prefix) = review_with_prefix(count)?;
            let repo = crate::test_repository::open(fixture.path())?;
            let tip_commit_id = repo.rev_parse_single("refs/patches/middle")?.detach();
            let old_return_commit_id = repo.find_reference("refs/heads/main")?.id().detach();
            let review_tree_id = repo.find_commit(started.commit)?.tree_id()?.detach();
            let side_commit_id = child(&repo, prefix[0], "side")?;
            repo.reference(
                "refs/heads/side",
                side_commit_id,
                gix::refs::transaction::PreviousValue::MustNotExist,
                "fixture",
            )?;
            let successor_commit_id = (count == 2)
                .then(|| child(&repo, started.commit, "successor"))
                .transpose()?;
            if let Some(commit_id) = successor_commit_id {
                repo.reference(
                    "refs/heads/successor",
                    commit_id,
                    gix::refs::transaction::PreviousValue::MustNotExist,
                    "fixture",
                )?;
            }
            let mut notes = repo.notes()?;
            notes.replace_at_ref("refs/notes/commits".try_into()?, prefix[0], b"independent change")?;
            drop(notes);
            let graph = super::super::loaded_graph(&repo)?;
            let Finish::Complete(finished) = finish(repo, &graph, started.commit, None)? else {
                panic!("independent changes finish without conflicts")
            };

            let repo = crate::test_repository::open(fixture.path())?;
            let mut parent_commit_id = tip_commit_id;
            for (number, old_commit_id) in prefix.iter().copied().enumerate() {
                let new_commit_id = finished
                    .outcome
                    .map(old_commit_id)
                    .context("the inserted commit survives")?;
                let commit = repo.find_commit(new_commit_id)?.decode()?.into_owned()?;
                let original = repo.find_commit(old_commit_id)?.decode()?.into_owned()?;
                assert_eq!(
                    commit.parents.as_slice(),
                    &[parent_commit_id],
                    "each inserted commit follows the transplanted prefix"
                );
                assert_eq!(
                    commit.message, original.message,
                    "the independent commit keeps its message"
                );
                assert_eq!(
                    commit.author, original.author,
                    "the independent commit keeps its author"
                );
                assert_eq!(
                    crate::change_id::for_commit(&repo, new_commit_id)?,
                    crate::change_id::for_commit(&repo, old_commit_id)?,
                    "transplanting preserves the change ID"
                );
                assert_eq!(
                    repo.find_reference(format!("refs/heads/extra-{number}").as_str())?.id(),
                    new_commit_id,
                    "mutable references follow the inserted commit"
                );
                assert_eq!(
                    run(
                        fixture.path(),
                        &[
                            "diff",
                            "--name-only",
                            &parent_commit_id.to_string(),
                            &new_commit_id.to_string()
                        ]
                    )?,
                    format!("extra-{number}\n").as_bytes(),
                    "the commit contains only its independent patch"
                );
                parent_commit_id = new_commit_id;
            }
            let review = repo.find_commit(finished.commit)?.decode()?.into_owned()?;
            assert_eq!(
                review.parents.as_slice(),
                &[parent_commit_id],
                "the finished review follows every inserted parent"
            );
            assert_eq!(
                review.tree, review_tree_id,
                "finishing preserves the reviewed tree exactly"
            );
            assert_eq!(
                run(
                    fixture.path(),
                    &[
                        "diff",
                        "--name-only",
                        &parent_commit_id.to_string(),
                        &finished.commit.to_string()
                    ]
                )?,
                b"review-fix\n",
                "the review contains only its own remaining correction"
            );
            let rewritten_side_commit_id = repo.find_reference("refs/heads/side")?.id().detach();
            assert_eq!(
                repo.find_commit(rewritten_side_commit_id)?
                    .parent_ids()
                    .next()
                    .map(gix::Id::detach),
                finished.outcome.map(prefix[0]),
                "side descendants follow the rewritten parent"
            );
            let insertion_commit_id = successor_commit_id.map_or(finished.commit, |commit_id| {
                finished.outcome.map(commit_id).expect("the review successor survives")
            });
            let return_commit_id = repo.find_reference("refs/heads/main")?.id().detach();
            assert_eq!(
                repo.find_commit(return_commit_id)?
                    .parent_ids()
                    .next()
                    .map(gix::Id::detach),
                Some(insertion_commit_id),
                "the original descendants follow the review additions"
            );
            assert_eq!(
                repo.head()?.referent_name().expect("the return is attached"),
                "refs/heads/main"
            );
            assert_ne!(
                repo.head_id()?,
                old_return_commit_id,
                "finishing returns to the rewritten branch"
            );
            let mut notes = repo.notes()?.with_refs(["refs/notes/commits"])?;
            assert_eq!(
                notes
                    .get(finished.outcome.map(prefix[0]).expect("the noted commit survives"))?
                    .first()
                    .map(|note| note.blob.data.as_slice()),
                Some(b"independent change".as_slice()),
                "Git notes follow the transplanted identity"
            );
            let mut approvals = crate::enrich::open_patch(&repo)?;
            for commit_id in prefix
                .iter()
                .map(|commit_id| finished.outcome.map(*commit_id).expect("the prefix survives"))
                .chain([finished.commit])
            {
                assert_eq!(
                    crate::enrich::load_patch_for_commit(&repo, &mut approvals, commit_id)?.refackiewed,
                    commit_id == finished.commit,
                    "finishing approves only the review patch"
                );
            }
            assert!(
                run(fixture.path(), &["status", "--porcelain=v1"])?.is_empty(),
                "the return checkout is clean"
            );
        }
        Ok(())
    }

    #[test]
    fn conflicting_or_unsupported_inserted_parents_leave_the_review_untouched() -> gix_testtools::Result {
        for case in ["conflict", "pending", "merge", "hidden"] {
            let (fixture, mut started, mut prefix) = review_with_prefix(2)?;
            let repo = crate::test_repository::open(fixture.path())?;
            let tip_commit_id = repo.rev_parse_single("refs/patches/middle")?.detach();
            let mut oldest = repo.find_commit(prefix[0])?.decode()?.into_owned()?;
            match case {
                "conflict" => {
                    // Both histories add `middle` with different content. The remaining
                    // review tree is already resolved, but this distinct patch isn't.
                    let mut tree = repo.find_tree(oldest.tree)?.edit()?;
                    tree.upsert(
                        "middle",
                        gix::objs::tree::EntryKind::Blob,
                        repo.write_blob("conflicting\n")?,
                    )?;
                    oldest.tree = tree.write()?.detach();
                }
                "pending" => oldest
                    .extra_headers
                    .push(("tix-rebase-parent".into(), oldest.parents[0].to_string().into())),
                "merge" => oldest.parents.push(tip_commit_id),
                "hidden" => {}
                _ => unreachable!("all test cases are listed above"),
            }
            prefix[0] = repo.write_object(&oldest)?.detach();
            let mut upper = repo.find_commit(prefix[1])?.decode()?.into_owned()?;
            upper.parents = [prefix[0]].into_iter().collect();
            prefix[1] = repo.write_object(&upper)?.detach();
            let mut review = repo.find_commit(started.commit)?.decode()?.into_owned()?;
            review.parents = [prefix[1]].into_iter().collect();
            started.commit = repo.write_object(&review)?.detach();
            for (number, commit_id) in prefix.iter().enumerate() {
                repo.reference(
                    format!("refs/heads/extra-{number}"),
                    *commit_id,
                    gix::refs::transaction::PreviousValue::Any,
                    "fixture",
                )?;
            }
            run(
                fixture.path(),
                &["checkout", "-q", "--detach", "--force", &started.commit.to_string()],
            )?;
            let graph = if case == "hidden" {
                super::super::loaded_explicit_view_graph(
                    &repo,
                    &["HEAD".into(), "refs/heads/main".into()],
                    &[prefix[0].to_string().into()],
                )?
            } else {
                super::super::loaded_graph(&repo)?
            };
            let before = gix_testtools::repository::snapshot(fixture.path())?;
            let err = finish(repo, &graph, started.commit, None)
                .err()
                .context("unsafe ancestry must be rejected before finishing")?;
            let message = format!("{err:#}");
            assert!(
                message.contains(&prefix[0].to_string()),
                "the error identifies the problematic ancestor: {message}"
            );
            assert!(
                message.contains(match case {
                    "conflict" => "conflicts with the reviewed history",
                    "pending" => "has a pending rebase",
                    "merge" => "must be an ordinary single-parent commit",
                    "hidden" => "is outside editable history",
                    _ => unreachable!("all test cases are listed above"),
                }),
                "the error explains the rejected {case}: {message}"
            );
            assert_eq!(
                gix_testtools::repository::snapshot(fixture.path())?,
                before,
                "rejecting {case} preserves refs, undo, index, and worktree"
            );
        }
        Ok(())
    }

    #[test]
    fn inserted_parents_stop_at_a_shared_merge_base() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_merge.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let base_commit_id = repo.rev_parse_single("diamond")?.detach();
        let tip_commit_id = child(&repo, base_commit_id, "tip")?;
        let extra_commit_id = child(&repo, base_commit_id, "extra")?;
        let review_commit_id = child(&repo, extra_commit_id, "review")?;
        for (name, commit_id) in [
            ("refs/heads/tip", tip_commit_id),
            ("refs/heads/review", review_commit_id),
        ] {
            repo.reference(
                name,
                commit_id,
                gix::refs::transaction::PreviousValue::MustNotExist,
                "fixture",
            )?;
        }
        let graph = super::super::loaded_graph(&repo)?;
        assert_eq!(
            inserted_parents(&repo, &graph, review_commit_id, tip_commit_id)?,
            [extra_commit_id],
            "the shared merge and its inputs are not part of the inserted ancestry"
        );
        Ok(())
    }

    fn state_without_undo(path: &Path) -> gix_testtools::Result<gix_testtools::repository::State> {
        let mut state = gix_testtools::repository::snapshot(path)?;
        state
            .references
            .retain(|reference| !super::super::undo::is_queue_ref(reference.name.as_bstr()));
        // Undo intentionally retains otherwise unreachable objects. HEAD and ref IDs
        // still identify the exact commit contents and topology being restored.
        state.commits.clear();
        Ok(state)
    }

    #[test]
    fn finishing_can_be_undone_and_redone_across_active_reviews_and_reopens() -> gix_testtools::Result {
        use super::super::undo;

        for (attached_return, another_review) in [(true, false), (false, false), (true, true), (false, true)] {
            let (fixture, started, _) = review_with_prefix(2)?;
            let repo = crate::test_repository::open(fixture.path())?;
            let return_commit_id = repo.find_reference("refs/heads/main")?.id().detach();
            let tip_commit_id = repo.rev_parse_single("refs/patches/middle")?.detach();
            let stash_ref = stash_reference(started.reference.as_bstr())?;
            repo.reference(
                stash_ref,
                tip_commit_id,
                gix::refs::transaction::PreviousValue::MustNotExist,
                "saved review state",
            )?;
            if another_review {
                let base_commit_id = repo
                    .find_commit(tip_commit_id)?
                    .parent_ids()
                    .next()
                    .context("the tip has a base")?
                    .detach();
                repo.reference(
                    "refs/worktree/tix/review/2",
                    base_commit_id,
                    gix::refs::transaction::PreviousValue::MustNotExist,
                    "another active review",
                )?;
            }
            if !attached_return {
                let return_ref = return_to(&repo.find_commit(started.commit)?.decode()?.into_owned()?)?
                    .context("the review has a return pin")?;
                run(
                    fixture.path(),
                    &[
                        "update-ref",
                        "--no-deref",
                        return_ref.as_bstr().to_str()?,
                        &return_commit_id.to_string(),
                    ],
                )?;
            }
            let before = state_without_undo(fixture.path())?;
            let graph = super::super::loaded_graph(&repo)?;
            let Finish::Complete(finished) = finish(repo, &graph, started.commit, None)? else {
                panic!("the review finishes without conflicts")
            };
            let repo = crate::test_repository::open(fixture.path())?;
            undo::record(&repo, "finish review", &finished.outcome.ref_changes)?
                .context("completion creates an undo entry even with another active review")?;
            let after = state_without_undo(fixture.path())?;
            assert_eq!(
                repo.head()?.referent_name().is_some(),
                attached_return,
                "completion restores the recorded attachment"
            );
            assert_eq!(
                (undo::position(&repo)?.undo, undo::position(&repo)?.redo),
                (1, 0),
                "completion is presented as one undoable operation"
            );
            drop(repo);

            let repo = crate::test_repository::open(fixture.path())?;
            undo::plan_undo(&repo)?
                .context("the persisted finish can be undone")?
                .apply(&repo)?;
            assert_eq!(
                state_without_undo(fixture.path())?,
                before,
                "undo restores commits, resources, approval, attachment, index, and worktree"
            );
            assert_eq!(
                (undo::position(&repo)?.undo, undo::position(&repo)?.redo),
                (0, 1),
                "the restored review still presents its finish for redo"
            );
            assert!(
                undo::record(&repo, "cancel unpublished preview", &[])?.is_none(),
                "an unpublished operation creates no undo entry"
            );
            drop(repo);

            let repo = crate::test_repository::open(fixture.path())?;
            undo::plan_redo(&repo)?
                .context("the restored active review can redo its completion after reopening")?
                .apply(&repo)?;
            assert_eq!(
                state_without_undo(fixture.path())?,
                after,
                "redo restores the exact completed state"
            );
            assert_eq!(
                (undo::position(&repo)?.undo, undo::position(&repo)?.redo),
                (1, 0),
                "redo returns to the single completed operation"
            );
        }
        Ok(())
    }

    #[test]
    fn editing_an_undone_review_discards_redo_and_finishing_can_be_retried() -> gix_testtools::Result {
        use super::super::{head, undo};

        let (fixture, started, _) = review_with_prefix(1)?;
        let repo = crate::test_repository::open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let Finish::Complete(finished) = finish(repo, &graph, started.commit, None)? else {
            panic!("the review finishes without conflicts")
        };
        let repo = crate::test_repository::open(fixture.path())?;
        undo::record(&repo, "finish review", &finished.outcome.ref_changes)?;
        undo::plan_undo(&repo)?
            .context("completion is undoable")?
            .apply(&repo)?;

        std::fs::write(fixture.path().join("review-fix"), "revised review fix\n")?;
        run(fixture.path(), &["add", "review-fix"])?;
        let graph = super::super::loaded_graph(&repo)?;
        let amended = head::amend_index_reporting(repo, &graph)?.context("the reviewed correction is amended")?;
        let review_commit_id = amended.selected.context("amending selects the revised review")?;
        let repo = crate::test_repository::open(fixture.path())?;
        assert!(
            undo::record(&repo, "amend", &amended.ref_changes)?.is_none(),
            "edits within active reviews remain unrecorded"
        );
        assert!(
            undo::plan_redo(&repo)?.is_none(),
            "an edit invalidates the previous completion's redo"
        );
        assert!(
            repo.try_find_reference(undo::TIP_REF)?.is_none(),
            "the stale queue is discarded"
        );

        let before = state_without_undo(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let Finish::Complete(finished) = finish(repo, &graph, review_commit_id, None)? else {
            panic!("the revised review finishes without conflicts")
        };
        let repo = crate::test_repository::open(fixture.path())?;
        undo::record(&repo, "finish review", &finished.outcome.ref_changes)?
            .context("retrying creates a new undo entry")?;
        assert_eq!(
            (undo::position(&repo)?.undo, undo::position(&repo)?.redo),
            (1, 0),
            "retrying starts a new completion history"
        );
        undo::plan_undo(&repo)?
            .context("the retried completion is undoable")?
            .apply(&repo)?;
        assert_eq!(
            state_without_undo(fixture.path())?,
            before,
            "undo restores the revised review rather than its discarded version"
        );
        Ok(())
    }

    #[test]
    fn starts_review_with_base_index_and_tip_worktree() -> gix_testtools::Result {
        let fixture = gix_testtools::tempfile::tempdir()?;
        run(fixture.path(), &["init", "-q", "-b", "main"])?;
        run(fixture.path(), &["config", "user.name", "reviewer"])?;
        run(fixture.path(), &["config", "user.email", "reviewer@example.com"])?;
        std::fs::write(fixture.path().join("file"), "base\n")?;
        run(fixture.path(), &["add", "file"])?;
        run(
            fixture.path(),
            &["-c", "commit.gpgSign=false", "commit", "-q", "-m", "base"],
        )?;
        let base = ObjectId::from_hex(run(fixture.path(), &["rev-parse", "HEAD"])?.trim())?;
        std::fs::write(fixture.path().join("file"), "tip\n")?;
        run(fixture.path(), &["-c", "commit.gpgSign=false", "commit", "-qam", "tip"])?;
        let tip = ObjectId::from_hex(run(fixture.path(), &["rev-parse", "HEAD"])?.trim())?;
        std::fs::write(fixture.path().join("natural"), "natural\n")?;
        run(fixture.path(), &["add", "natural"])?;
        run(
            fixture.path(),
            &["-c", "commit.gpgSign=false", "commit", "-q", "-m", "natural descendant"],
        )?;
        run(fixture.path(), &["branch", "natural"])?;
        run(fixture.path(), &["reset", "--hard", &tip.to_string()])?;

        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=reviewer", "user.email=reviewer@example.com"],
        )?;
        let graph = super::super::loaded_graph(&repo)?;
        drop(repo);
        let started = start(fixture.path(), false, &graph, tip, base)?;
        assert!(started.checkout_error.is_none());

        let repo = crate::test_repository::open(fixture.path())?;
        assert_eq!(repo.head_id()?, started.commit, "HEAD selects the review commit");
        assert_eq!(
            repo.find_commit(started.commit)?.tree_id()?,
            repo.find_commit(base)?.tree_id()?
        );
        let review_ref = repo.find_reference(started.reference.as_ref())?;
        assert_eq!(
            review_ref.id(),
            tip,
            "the review resource remains anchored to the reviewed commit"
        );
        let commit = repo.find_commit(started.commit)?.decode()?.into_owned()?;
        assert_eq!(reference(&commit)?, Some(started.reference.clone()));
        let pins = history::all_pins(&repo)?;
        assert_eq!(pins.len(), 1, "review start preserves its departure with a pin");
        assert_eq!(pins[0].id, tip);
        assert_eq!(
            pins[0].target.try_name().expect("the pin is symbolic"),
            "refs/heads/main",
            "an attached departure uses a symbolic pin"
        );
        assert_eq!(
            return_to(&commit)?.expect("the review records a return"),
            pins[0].name,
            "the review commit records its departure pin"
        );
        assert_eq!(
            std::fs::read(fixture.path().join("file"))?,
            b"tip\n",
            "reviewed content stays in worktree"
        );
        assert_eq!(
            run(fixture.path(), &["diff", "--name-only"])?,
            b"file\n",
            "the reviewed change is unstaged"
        );
        assert!(run(fixture.path(), &["diff", "--cached", "--name-only"])?.is_empty());

        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=reviewer", "user.email=reviewer@example.com"],
        )?;
        let graph = super::super::loaded_graph(&repo)?;
        let amended = super::super::head::perform(repo, &graph, super::super::head::Kind::Amend, None)?
            .expect("the reviewed worktree delta amends the review commit");
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=reviewer", "user.email=reviewer@example.com"],
        )?;
        let amended_commit = repo.find_commit(amended)?.decode()?.into_owned()?;
        assert!(is_review(&amended_commit));
        assert_eq!(
            return_to(&amended_commit)?.expect("the review records a return"),
            pins[0].name,
            "review amendments preserve the return action"
        );
        assert!(run(fixture.path(), &["status", "--porcelain=v1", "--untracked-files=all"])?.is_empty());
        let child = run(
            fixture.path(),
            &[
                "-c",
                "commit.gpgSign=false",
                "commit-tree",
                &format!("{amended}^{{tree}}"),
                "-p",
                &amended.to_string(),
                "-m",
                "review child",
            ],
        )?;
        let child = ObjectId::from_hex(child.trim())?;
        run(
            fixture.path(),
            &["update-ref", "refs/heads/review-child", &child.to_string()],
        )?;
        let stash_ref = stash_reference(started.reference.as_bstr())?;
        run(
            fixture.path(),
            &[
                "update-ref",
                stash_ref.as_bstr().to_str_lossy().as_ref(),
                &tip.to_string(),
            ],
        )?;
        drop(repo);
        run(
            fixture.path(),
            &[
                "update-ref",
                "--no-deref",
                "-d",
                pins[0].name.as_bstr().to_str_lossy().as_ref(),
            ],
        )?;
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=reviewer", "user.email=reviewer@example.com"],
        )?;
        let graph = super::super::loaded_graph(&repo)?;
        let Finish::SelectReturn { tip: return_tip } = finish(repo, &graph, amended, None)? else {
            panic!("the deleted return pin requires replacement selection")
        };
        assert_eq!(return_tip, tip, "fallback checkout must descend from the reviewed tip");
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=reviewer", "user.email=reviewer@example.com"],
        )?;
        assert!(
            repo.try_find_reference(started.reference.as_ref())?.is_some(),
            "asking for a replacement leaves the review untouched"
        );
        assert!(
            repo.try_find_reference(crate::enrich::PATCH_REF_NAME)?.is_none(),
            "choosing a return target does not approve an unfinished review"
        );
        let Finish::Complete(finished) = finish(repo, &graph, amended, Some(tip))? else {
            panic!("the selected descendant completes the review")
        };
        assert_eq!(
            finished.outcome.selected,
            Some(finished.commit),
            "the reviewed tip maps to the newly finished review commit"
        );
        assert!(
            finished
                .outcome
                .ref_changes
                .iter()
                .any(|change| change.name == crate::enrich::PATCH_REF_NAME),
            "finishing and approval belong to the same reference transaction"
        );

        let finished = finished.commit;
        let repo = crate::test_repository::open(fixture.path())?;
        assert_eq!(
            crate::load_patch_enrichment_state(&repo, &mut crate::enrich::open_patch(&repo)?, finished)?,
            crate::app::PatchEnrichmentState::Fresh { refackiewed: true },
            "an unchanged review approves its resulting empty patch"
        );
        assert_eq!(repo.head_id()?, finished);
        assert_eq!(
            repo.head()?.referent_name().map(gix::refs::FullNameRef::as_bstr),
            None,
            "replacement commit selection deliberately leaves HEAD detached"
        );
        assert_eq!(
            repo.find_commit(finished)?.parent_ids().next().map(gix::Id::detach),
            Some(tip)
        );
        assert!(!is_review(&repo.find_commit(finished)?.decode()?.into_owned()?));
        assert!(
            return_to(&repo.find_commit(finished)?.decode()?.into_owned()?)?.is_none(),
            "finished commits contain no review return action"
        );
        let child = repo.find_reference("refs/heads/review-child")?.id().detach();
        assert_eq!(
            repo.find_commit(child)?.parent_ids().next().map(gix::Id::detach),
            Some(finished)
        );
        let natural = repo.find_reference("refs/heads/natural")?.id().detach();
        assert_eq!(
            repo.find_commit(natural)?.parent_ids().next().map(gix::Id::detach),
            Some(child),
            "the natural descendants follow the review side's single leaf"
        );
        assert!(
            !super::super::rebase::is_pending(&repo.find_commit(natural)?.decode()?.into_owned()?),
            "the review leaf retains the original parent tree, so the natural descendant stays final"
        );
        assert!(
            repo.try_find_reference(started.reference.as_ref())?.is_none(),
            "finishing removes the review resource"
        );
        assert!(
            repo.try_find_reference(stash_ref.as_ref())?.is_none(),
            "finishing also removes saved review worktree state"
        );
        Ok(())
    }

    #[test]
    fn a_blocked_checkout_keeps_the_prepared_review_and_dirty_worktree() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let tip = repo.rev_parse_single("refs/patches/tip")?.detach();
        let base = repo
            .find_commit(tip)?
            .parent_ids()
            .next()
            .expect("the reviewed tip has a parent")
            .detach();
        drop(repo);
        std::fs::write(fixture.path().join("tip"), "departure\n")?;
        run(fixture.path(), &["commit", "-qam", "departure"])?;
        let departure = ObjectId::from_hex(run(fixture.path(), &["rev-parse", "HEAD"])?.trim())?;
        std::fs::write(fixture.path().join("tip"), "dirty\n")?;

        let repo = crate::test_repository::open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        drop(repo);
        let started = start(fixture.path(), false, &graph, tip, base)?;
        let checkout_error = started
            .checkout_error
            .as_ref()
            .expect("conflicting dirt blocks only the checkout");
        assert!(format!("{checkout_error:#}").contains("could not check out the reviewed commit"));

        let repo = crate::test_repository::open(fixture.path())?;
        assert_eq!(repo.head_id()?, departure, "the failed checkout leaves HEAD untouched");
        assert_eq!(repo.find_reference(started.reference.as_ref())?.id(), tip);
        let commit = repo.find_commit(started.commit)?.decode()?.into_owned()?;
        let return_name = return_to(&commit)?.expect("the prepared review records its return pin");
        let mut return_ref = repo.find_reference(return_name.as_ref())?;
        assert_eq!(return_ref.peel_to_id()?.detach(), departure);
        assert_eq!(std::fs::read(fixture.path().join("tip"))?, b"dirty\n");
        assert_eq!(run(fixture.path(), &["show", ":tip"])?, b"departure\n");
        Ok(())
    }

    #[test]
    fn a_changed_review_and_its_successors_are_spliced_before_target_successors() -> gix_testtools::Result {
        let fixture = gix_testtools::tempfile::tempdir()?;
        run(fixture.path(), &["init", "-q", "-b", "main"])?;
        crate::test_repository::disable_autocrlf(fixture.path())?;
        run(fixture.path(), &["config", "user.name", "reviewer"])?;
        run(fixture.path(), &["config", "user.email", "reviewer@example.com"])?;
        run(
            fixture.path(),
            &["config", "gitoxide.commit.authorDate", "2001-01-01T00:00:00 +0000"],
        )?;
        run(
            fixture.path(),
            &["config", "gitoxide.commit.committerDate", "2001-01-01T00:00:00 +0000"],
        )?;
        std::fs::write(fixture.path().join("file"), "base\n")?;
        run(fixture.path(), &["add", "file"])?;
        run(
            fixture.path(),
            &["-c", "commit.gpgSign=false", "commit", "-q", "-m", "base"],
        )?;
        let base = ObjectId::from_hex(run(fixture.path(), &["rev-parse", "HEAD"])?.trim())?;
        std::fs::write(fixture.path().join("file"), "B\n")?;
        run(fixture.path(), &["-c", "commit.gpgSign=false", "commit", "-qam", "B"])?;
        let reviewed = ObjectId::from_hex(run(fixture.path(), &["rev-parse", "HEAD"])?.trim())?;
        std::fs::write(fixture.path().join("successor"), "A\n")?;
        run(fixture.path(), &["add", "successor"])?;
        run(
            fixture.path(),
            &["-c", "commit.gpgSign=false", "commit", "-q", "-m", "A"],
        )?;
        let old_successor = ObjectId::from_hex(run(fixture.path(), &["rev-parse", "HEAD"])?.trim())?;

        let open = || {
            crate::test_repository::open_with(
                fixture.path(),
                ["user.name=reviewer", "user.email=reviewer@example.com"],
            )
        };
        let repo = open()?;
        let graph = super::super::loaded_graph(&repo)?;
        drop(repo);
        start(fixture.path(), false, &graph, reviewed, base)?;
        let repo = open()?;
        let pins = history::all_pins(&repo)?;
        assert_eq!(pins.len(), 1, "review start preserves the checked-out descendant tip");
        assert_eq!(pins[0].id, old_successor);
        assert_eq!(
            pins[0].target.try_name().expect("the pin is symbolic"),
            "refs/heads/main",
            "an attached review departure remains attached through a symbolic pin"
        );
        drop(repo);

        std::fs::write(fixture.path().join("reviewed"), "new review change\n")?;
        run(fixture.path(), &["add", "file", "reviewed"])?;
        let repo = open()?;
        let graph = super::super::loaded_graph(&repo)?;
        let review = super::super::head::perform(repo, &graph, super::super::head::Kind::Amend, None)?
            .expect("the staged review change amends the review commit");
        let review_successor = ObjectId::from_hex(
            run(
                fixture.path(),
                &[
                    "-c",
                    "commit.gpgSign=false",
                    "commit-tree",
                    &format!("{review}^{{tree}}"),
                    "-p",
                    &review.to_string(),
                    "-m",
                    "review successor",
                ],
            )?
            .trim(),
        )?;
        run(
            fixture.path(),
            &[
                "update-ref",
                "refs/heads/review-successor",
                &review_successor.to_string(),
            ],
        )?;
        run(fixture.path(), &["checkout", "--detach", &review_successor.to_string()])?;

        let repo = open()?;
        let graph = super::super::loaded_graph(&repo)?;
        let Finish::Complete(finished) = finish(repo, &graph, review, None)? else {
            panic!("the recorded return pin exists")
        };

        let finished = finished.commit;
        let repo = open()?;
        let successor = repo.find_reference("refs/heads/main")?.id().detach();
        assert_eq!(
            repo.head()?.referent_name().expect("HEAD is attached"),
            "refs/heads/main",
            "finishing returns to the branch that contained the reviewed commit"
        );
        assert_eq!(repo.head_id()?, successor);
        assert!(
            history::all_pins(&repo)?.is_empty(),
            "returning consumes the departure pin"
        );
        assert!(
            run(fixture.path(), &["status", "--porcelain=v1", "--untracked-files=all"])?.is_empty(),
            "the restored branch has a matching index and worktree"
        );
        assert_ne!(successor, old_successor, "the branch successor is rewritten");
        assert_eq!(
            repo.find_commit(finished)?.parent_ids().next().map(gix::Id::detach),
            Some(reviewed),
            "the finished review is inserted directly after B"
        );
        assert_eq!(
            repo.find_commit(successor)?.parent_ids().next().map(gix::Id::detach),
            Some(repo.find_reference("refs/heads/review-successor")?.id().detach()),
            "A remains the branch tip and follows the review-side history"
        );
        let review_successor = repo.find_reference("refs/heads/review-successor")?.id().detach();
        let mut approvals = crate::enrich::open_patch(&repo)?;
        for commit_id in [finished, successor, review_successor] {
            assert_eq!(
                matches!(
                    crate::load_patch_enrichment_state(&repo, &mut approvals, commit_id)?,
                    crate::app::PatchEnrichmentState::Fresh { refackiewed: true }
                ),
                commit_id == finished,
                "finishing approves the resulting review patch, leaving descendants unmarked"
            );
        }
        assert_eq!(
            repo.find_commit(review_successor)?
                .parent_ids()
                .next()
                .map(gix::Id::detach),
            Some(finished),
            "the review successor is inserted before the target history successor"
        );
        assert_eq!(
            repo.find_commit(successor)?.message_raw()?,
            b"A\n".as_bstr(),
            "the target history successor is retained"
        );
        assert!(
            !super::super::rebase::is_pending(&repo.find_commit(successor)?.decode()?.into_owned()?),
            "the checked-out review return path is fully replayed"
        );
        crate::test_repository::clear_autocrlf(fixture.path())?;
        insta::assert_snapshot!(
            "changed-review-with-successors",
            gix_testtools::repository::snapshot_portable(fixture.path())?.to_string()
        );
        Ok(())
    }

    #[test]
    fn finish_approval_is_atomic_when_the_return_checkout_conflicts() -> gix_testtools::Result {
        use super::super::{head, undo};

        let fixture = gix_testtools::scripted_fixture_writable("rebase_conflict.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let tip = repo.rev_parse_single("HEAD~1")?.detach();
        let base = repo.rev_parse_single("HEAD~2")?.detach();
        let graph = super::super::loaded_graph(&repo)?;
        drop(repo);
        let mut started = start(fixture.path(), false, &graph, tip, base)?;
        assert!(started.checkout_error.is_none(), "the review starts successfully");
        // Revert the reviewed edit so the return tip's same-line change conflicts.
        run(fixture.path(), &["restore", "--worktree", "."])?;
        let repo = crate::test_repository::open(fixture.path())?;
        let mut review = repo.find_commit(started.commit)?.decode()?.into_owned()?;
        review
            .extra_headers
            .push(("tix-rebase-parent".into(), base.to_string().into()));
        started.commit = repo.write_object(&review)?.detach();
        run(
            fixture.path(),
            &["update-ref", "--no-deref", "HEAD", &started.commit.to_string()],
        )?;
        let graph = super::super::loaded_graph(&repo)?;
        let before = gix_testtools::repository::snapshot(fixture.path())?;
        let before_undo = state_without_undo(fixture.path())?;
        let Finish::Conflict(conflict) = finish(repo, &graph, started.commit, None)? else {
            panic!("the return checkout conflicts with the reviewed change")
        };
        drop(conflict);
        assert_eq!(
            gix_testtools::repository::snapshot(fixture.path())?,
            before,
            "cancelling leaves the review, approval, index, and worktree untouched"
        );

        let repo = crate::test_repository::open(fixture.path())?;
        let Finish::Conflict(conflict) = finish(repo, &graph, started.commit, None)? else {
            panic!("the unchanged review still conflicts with its return checkout")
        };
        let outcome = conflict.persist(super::super::rebase::CheckoutOptions::default())?;
        let finished = outcome
            .map(started.commit)
            .expect("accepting publishes the finished review");
        assert_ne!(
            outcome.selected,
            Some(finished),
            "checkout selects the conflicting descendant"
        );
        assert!(
            outcome
                .ref_changes
                .iter()
                .any(|change| change.name == crate::enrich::PATCH_REF_NAME),
            "approval is published in the same transaction as accepting the conflict"
        );
        let repo = crate::test_repository::open(fixture.path())?;
        assert_eq!(
            crate::load_patch_enrichment_state(&repo, &mut crate::enrich::open_patch(&repo)?, finished)?,
            crate::app::PatchEnrichmentState::Fresh { refackiewed: true },
            "finishing finalizes and approves the pending review instead of its conflicted checkout"
        );
        assert!(
            repo.try_find_reference(started.reference.as_ref())?.is_none(),
            "the review resource is consumed"
        );
        std::fs::write(fixture.path().join("file"), "resolved return checkout\n")?;
        run(fixture.path(), &["add", "file"])?;
        let graph = super::super::loaded_graph(&repo)?;
        let resolved = head::amend_index_reporting(repo, &graph)?.context("the conflicting return is resolved")?;
        let mut changes = outcome.ref_changes;
        changes.extend(resolved.ref_changes);
        let repo = crate::test_repository::open(fixture.path())?;
        undo::record(&repo, "resolve review return conflict", &changes)?.context("the resolved finish is undoable")?;
        assert_eq!(
            undo::position(&repo)?.undo,
            1,
            "accepting and resolving a review finish share one undo step"
        );
        let after = state_without_undo(fixture.path())?;
        undo::plan_undo(&repo)?
            .context("the complete finish is undoable")?
            .apply(&repo)?;
        assert_eq!(
            state_without_undo(fixture.path())?,
            before_undo,
            "undo restores the pre-finish review, including its approval state"
        );
        undo::plan_redo(&repo)?
            .context("the resolved finish can be redone from the restored review")?
            .apply(&repo)?;
        assert_eq!(
            state_without_undo(fixture.path())?,
            after,
            "redo restores the resolved checkout and approval"
        );
        Ok(())
    }

    #[test]
    fn deleting_a_review_returns_to_the_preserved_departure() -> gix_testtools::Result {
        let fixture = gix_testtools::tempfile::tempdir()?;
        run(fixture.path(), &["init", "-q", "-b", "main"])?;
        run(fixture.path(), &["config", "user.name", "reviewer"])?;
        run(fixture.path(), &["config", "user.email", "reviewer@example.com"])?;
        std::fs::write(fixture.path().join("file"), "base\n")?;
        run(fixture.path(), &["add", "file"])?;
        run(
            fixture.path(),
            &["-c", "commit.gpgSign=false", "commit", "-q", "-m", "base"],
        )?;
        let base = ObjectId::from_hex(run(fixture.path(), &["rev-parse", "HEAD"])?.trim())?;
        std::fs::write(fixture.path().join("file"), "reviewed\n")?;
        run(
            fixture.path(),
            &["-c", "commit.gpgSign=false", "commit", "-qam", "reviewed"],
        )?;
        let reviewed = ObjectId::from_hex(run(fixture.path(), &["rev-parse", "HEAD"])?.trim())?;
        std::fs::write(fixture.path().join("tip"), "tip\n")?;
        run(fixture.path(), &["add", "tip"])?;
        run(
            fixture.path(),
            &["-c", "commit.gpgSign=false", "commit", "-q", "-m", "tip"],
        )?;
        let tip = ObjectId::from_hex(run(fixture.path(), &["rev-parse", "HEAD"])?.trim())?;

        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=reviewer", "user.email=reviewer@example.com"],
        )?;
        let graph = super::super::loaded_graph(&repo)?;
        drop(repo);
        let started = start(fixture.path(), false, &graph, reviewed, base)?;
        let repo = crate::test_repository::open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let deleted = super::super::delete::perform(repo, &graph, started.commit)?;
        let return_to = deleted.review_return.context("review deletion has a return checkout")?;
        let (returned, _) =
            super::super::time_travel::checkout_review_return(fixture.path(), false, &return_to, &[], false)?;

        let repo = crate::test_repository::open(fixture.path())?;
        assert_eq!(returned, tip);
        assert_eq!(repo.head_id()?, tip);
        assert_eq!(
            repo.head()?.referent_name().expect("HEAD is attached"),
            "refs/heads/main",
            "cancelling reattaches the original branch"
        );
        assert!(history::all_pins(&repo)?.is_empty(), "the return consumes its pin");
        assert!(
            repo.try_find_reference(started.reference.as_ref())?.is_none(),
            "the cancelled review resource is removed"
        );
        assert!(
            run(fixture.path(), &["status", "--porcelain=v1", "--untracked-files=all"])?.is_empty(),
            "cancelling restores the original checkout without review changes"
        );

        drop(repo);
        run(fixture.path(), &["checkout", "-q", "--detach", &tip.to_string()])?;
        let repo = crate::test_repository::open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        drop(repo);
        let started = start(fixture.path(), false, &graph, reviewed, base)?;
        let repo = crate::test_repository::open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let deleted = super::super::delete::perform(repo, &graph, started.commit)?;
        let return_to = deleted
            .review_return
            .context("detached review deletion has a return checkout")?;
        super::super::time_travel::checkout_review_return(fixture.path(), false, &return_to, &[], false)?;
        let repo = crate::test_repository::open(fixture.path())?;
        assert!(repo.head()?.is_detached(), "cancelling restores detached HEAD");
        assert_eq!(repo.head_id()?, tip);
        assert!(history::all_pins(&repo)?.is_empty());
        Ok(())
    }

    #[test]
    fn review_return_pin_survives_squashing_the_reviewed_tip() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let open = || crate::test_repository::open(fixture.path());
        let repo = open()?;
        let tip = repo.rev_parse_single("refs/patches/tip")?.detach();
        let middle = repo.rev_parse_single("refs/patches/middle")?.detach();
        let base = repo
            .find_commit(middle)?
            .parent_ids()
            .next()
            .expect("middle has a parent")
            .detach();
        let graph = super::super::loaded_graph(&repo)?;
        drop(repo);

        let started = start(fixture.path(), false, &graph, tip, base)?;
        let repo = open()?;
        let review = repo.find_commit(started.commit)?.decode()?.into_owned()?;
        let return_name = return_to(&review)?.expect("the review records its return pin");
        let graph = super::super::loaded_graph(&repo)?;
        drop(repo);
        super::super::time_travel::perform(
            fixture.path(),
            false,
            middle,
            &graph,
            &[started.commit],
            &[],
            Default::default(),
        )?
        .complete()?;

        let repo = open()?;
        let graph = super::super::loaded_graph(&repo)?;
        let plan = super::super::rebase::squash_plan(&repo, &graph, tip, middle)?;
        let outcome = super::super::rebase::perform_plan(&repo, &graph, plan)?.complete()?;
        let combined = outcome.map(middle).expect("the squash retains its target");
        drop(repo);

        run(
            fixture.path(),
            &["checkout", "--quiet", "--detach", &started.commit.to_string()],
        )?;
        super::super::time_travel::checkout_without_replay(fixture.path(), false, combined, &[], false)?;

        let repo = open()?;
        let mut return_ref = repo.find_reference(return_name.as_ref())?;
        assert_eq!(
            return_ref.peel_to_id()?.detach(),
            combined,
            "checking out a rewritten destination preserves the review's return pin"
        );
        assert_eq!(
            return_name.as_bstr(),
            b"refs/worktree/tix/pins/review/1",
            "review-owned pins have an explicit namespace"
        );
        let graph = super::super::loaded_graph(&repo)?;
        drop(repo);

        super::super::time_travel::perform(
            fixture.path(),
            false,
            started.commit,
            &graph,
            &[started.commit],
            &[],
            Default::default(),
        )?
        .complete()?;
        run(fixture.path(), &["add", "--all"])?;
        let repo = open()?;
        let graph = super::super::loaded_graph(&repo)?;
        let review = super::super::head::perform(repo, &graph, super::super::head::Kind::Amend, None)?
            .expect("staged review changes amend the review commit");
        let repo = open()?;
        let graph = super::super::loaded_graph(&repo)?;
        let Finish::Complete(finished) = finish(repo, &graph, review, None)? else {
            panic!("the owned return pin finishes the review without fallback selection")
        };

        let repo = open()?;
        assert_eq!(
            repo.head()?.referent_name().expect("HEAD is attached"),
            "refs/heads/main"
        );
        assert_eq!(repo.head_id()?, finished.commit);
        assert!(
            repo.try_find_reference(return_name.as_ref())?.is_none(),
            "finishing consumes its review-owned return pin"
        );
        Ok(())
    }
}
