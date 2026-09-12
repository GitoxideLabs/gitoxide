use anyhow::{Context, Result};
use gix::ObjectId;

use super::{rebase, undo};
use crate::{enrich, history::HistoryGraph};

pub(crate) struct Outcome {
    pub selected: ObjectId,
    pub enrichment: enrich::PatchEnrichment,
    pub notice: Option<String>,
    pub ref_rewrites: Vec<rebase::RefRewrite>,
    pub ref_changes: Vec<undo::RefChange>,
}

/// Set or clear the approval, or toggle it when `enabled` is `None`.
/// Only a commit without a patch header requires an editable history graph.
pub(crate) fn refackiewed(
    repo: &gix::Repository,
    graph: Option<&HistoryGraph>,
    commit_id: ObjectId,
    enabled: Option<bool>,
    mut report: impl FnMut(rebase::Progress),
) -> Result<Outcome> {
    let object = repo
        .find_commit(commit_id)
        .context("could not find the patch to refackiew")?;
    let decoded = object.decode().context("could not decode the patch to refackiew")?;
    let patch_id = crate::patch_id::current(repo, &decoded)?;
    let has_header = decoded.extra_headers().find(crate::patch_id::HEADER).is_some();
    let mut commit = decoded.into_owned()?;
    anyhow::ensure!(
        !(rebase::is_pending(&commit) || patch_id.is_none() && has_header),
        "the patch ID is stale or unavailable; complete its rebase before marking it refackiewed"
    );

    if let Some(patch_id) = patch_id {
        let current = enrich::load_patch(
            &mut enrich::open_patch(repo)?,
            crate::change_id::for_commit(repo, commit_id)?,
            patch_id,
        )?;
        let enabled = enabled.unwrap_or(!current.refackiewed);
        let reference: gix::refs::FullName = enrich::PATCH_REF_NAME.try_into().expect("valid patch enrich ref");
        let before = undo::state(repo, reference.as_ref())?;
        let enrichment = enrich::ensure_refackiewed(repo, commit_id, enabled)?;
        let after = undo::state(repo, reference.as_ref())?;
        return Ok(Outcome {
            selected: commit_id,
            enrichment,
            notice: None,
            ref_rewrites: Vec::new(),
            ref_changes: (before != after)
                .then_some(undo::RefChange {
                    name: reference,
                    before,
                    after,
                })
                .into_iter()
                .collect(),
        });
    }

    if enabled == Some(false) {
        return Ok(Outcome {
            selected: commit_id,
            enrichment: enrich::PatchEnrichment::default(),
            notice: None,
            ref_rewrites: Vec::new(),
            ref_changes: Vec::new(),
        });
    }
    let graph = graph.context("adding a patch ID requires an editable history graph")?;
    anyhow::ensure!(
        graph.is_in_edit_scope(commit_id),
        "adding a patch ID requires an editable commit; hidden boundaries cannot be rewritten"
    );
    super::auto_merge::ensure_editable(&commit)?;
    report(rebase::Progress {
        total: 1,
        ..Default::default()
    });
    crate::patch_id::refresh(repo, &mut commit)?;
    let (performed, enrichment) =
        rebase::perform_with_refackiewed_and_progress(repo, graph, commit_id, commit, true, report)?;
    let outcome = performed
        .complete()
        .context("could not add the patch ID and approval atomically")?;
    Ok(Outcome {
        selected: outcome.selected.context("adding a patch ID did not produce a commit")?,
        enrichment,
        notice: outcome.notice,
        ref_rewrites: outcome.ref_rewrites,
        ref_changes: outcome.ref_changes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_mark_rewrites_history_and_approval_in_one_undo_without_touching_the_index() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let original_head = repo.head_id()?.detach();
        let original = repo.rev_parse_single("HEAD~1")?.detach();
        let original_commit = repo.find_commit(original)?.decode()?.into_owned()?;
        let mut fork = repo.find_commit(original_head)?.decode()?.into_owned()?;
        fork.message = "retained fork".into();
        let original_fork = repo.write_object(&fork)?.detach();
        let fork_ref = "refs/worktree/tix/pins/fork";
        repo.reference(
            fork_ref,
            original_fork,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "retain a fork outside checkout ancestry",
        )?;
        std::fs::write(fixture.path().join("tip"), b"staged\n")?;
        assert!(
            gix_testtools::git_command(fixture.path())
                .args(["add", "tip"])
                .status()?
                .success(),
            "git stages the independent index change"
        );
        std::fs::write(fixture.path().join("tip"), b"unstaged\n")?;
        let index_before = std::fs::read(repo.index_path())?;
        let graph = super::super::loaded_graph(&repo)?;

        let outcome = refackiewed(&repo, Some(&graph), original, Some(true), |_| {})?;
        assert_ne!(
            outcome.selected, original,
            "the first approval embeds a patch ID in the commit"
        );
        let rewritten = repo.find_commit(outcome.selected)?.decode()?.into_owned()?;
        assert_eq!(
            rewritten.tree, original_commit.tree,
            "adding the header preserves the result tree"
        );
        assert_eq!(
            rewritten.author, original_commit.author,
            "approval preserves authorship"
        );
        assert_eq!(
            rewritten.message, original_commit.message,
            "approval preserves the commit message"
        );
        assert!(outcome.enrichment.refackiewed, "the new patch version is approved");
        assert!(crate::patch_id::for_commit(&repo, outcome.selected)?.is_some());
        let rewritten_head = repo.head_id()?.detach();
        let rewritten_fork = repo.find_reference(fork_ref)?.id().detach();
        assert!(
            repo.find_commit(rewritten_fork)?
                .decode()?
                .extra_headers()
                .find(crate::patch_id::HEADER)
                .is_none(),
            "an off-checkout descendant without a header is not hashed merely to reparent it"
        );
        for (before, after) in [(original_head, rewritten_head), (original_fork, rewritten_fork)] {
            assert_ne!(before, after, "each retained descendant follows the new parent ID");
            let commit = repo.find_commit(after)?.decode()?.into_owned()?;
            assert_eq!(commit.tree, repo.find_commit(before)?.tree_id()?.detach());
            assert!(
                !rebase::is_pending(&commit),
                "metadata-only descendants remain final on every fork"
            );
        }
        assert_eq!(
            std::fs::read(repo.index_path())?,
            index_before,
            "header insertion preserves staged changes"
        );
        assert_eq!(
            std::fs::read(fixture.path().join("tip"))?,
            b"unstaged\n",
            "worktree bytes stay intact"
        );
        assert!(
            outcome
                .ref_changes
                .iter()
                .any(|change| change.name.as_bstr() == enrich::PATCH_REF_NAME.as_bytes()),
            "the patch approval belongs to the same change set as rewritten history"
        );
        undo::record(&repo, "mark patch refackiewed", &outcome.ref_changes)?;
        undo::plan_undo(&repo)?
            .expect("the combined mark has an undo entry")
            .apply(&repo)?;
        assert_eq!(repo.head_id()?, original_head, "undo restores the original history");
        assert_eq!(repo.find_reference(fork_ref)?.id(), original_fork);
        assert!(
            repo.try_find_reference(enrich::PATCH_REF_NAME)?.is_none(),
            "undo also removes the first approval"
        );
        undo::plan_redo(&repo)?
            .expect("the combined mark can be redone")
            .apply(&repo)?;
        assert_eq!(repo.head_id()?, rewritten_head);
        assert_eq!(repo.find_reference(fork_ref)?.id(), rewritten_fork);
        assert_eq!(
            std::fs::read(repo.index_path())?,
            index_before,
            "undo/redo also preserves staging"
        );

        let again = refackiewed(&repo, None, outcome.selected, Some(true), |_| {})?;
        assert_eq!(
            again.selected, outcome.selected,
            "later marking only reads the existing header"
        );
        assert!(
            again.ref_changes.is_empty(),
            "an already-set approval is an idempotent no-op"
        );
        let mut copy = repo.find_commit(outcome.selected)?.decode()?.into_owned()?;
        copy.message = "same Tix change and patch".into();
        let copy_id = repo.write_object(&copy)?.detach();
        let shared = refackiewed(&repo, None, copy_id, Some(true), |_| {})?;
        assert!(
            shared.ref_changes.is_empty(),
            "the same change and patch share the existing approval"
        );
        let cleared = refackiewed(&repo, None, copy_id, None, |_| {})?;
        assert!(
            !cleared.enrichment.refackiewed,
            "the same action toggles an existing approval off"
        );
        assert_eq!(cleared.selected, copy_id, "clearing only changes notes");
        Ok(())
    }

    #[test]
    fn stale_and_pending_ids_cannot_be_marked_and_legacy_clear_is_a_noop() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let original = repo.head_id()?.detach();
        let cleared = refackiewed(&repo, None, original, Some(false), |_| {})?;
        assert_eq!(
            cleared.selected, original,
            "clearing a legacy commit does not insert a header"
        );
        assert!(cleared.ref_changes.is_empty());
        assert!(repo.try_find_reference(enrich::PATCH_REF_NAME)?.is_none());

        let mut unavailable = repo.find_commit(original)?.decode()?.into_owned()?;
        crate::patch_id::mark_unavailable(&mut unavailable);
        let unavailable_id = repo.write_object(&unavailable)?.detach();
        let error = refackiewed(&repo, None, unavailable_id, Some(true), |_| {})
            .err()
            .expect("a conflict placeholder cannot be approved");
        assert!(format!("{error:#}").contains("stale or unavailable"));

        let mut stale = repo.find_commit(original)?.decode()?.into_owned()?;
        crate::patch_id::refresh(&repo, &mut stale)?;
        stale.tree = repo.find_commit(repo.rev_parse_single("HEAD~1")?)?.tree_id()?.detach();
        let stale_id = repo.write_object(&stale)?.detach();
        let error = refackiewed(&repo, None, stale_id, Some(true), |_| {})
            .err()
            .expect("a patch header whose witness no longer matches cannot be approved");
        assert!(format!("{error:#}").contains("stale or unavailable"));

        let mut pending = repo.find_commit(original)?.decode()?.into_owned()?;
        crate::patch_id::refresh(&repo, &mut pending)?;
        pending.extra_headers.push((
            "tix-rebase-parent".into(),
            pending
                .parents
                .first()
                .expect("the fixture tip has a parent")
                .to_string()
                .into(),
        ));
        let pending_id = repo.write_object(&pending)?.detach();
        assert!(
            refackiewed(&repo, None, pending_id, Some(true), |_| {}).is_err(),
            "a retained patch ID on pending history never authorizes approval"
        );
        assert_eq!(repo.head_id()?, original, "rejected actions leave history untouched");
        assert!(repo.try_find_reference(enrich::PATCH_REF_NAME)?.is_none());
        Ok(())
    }

    #[test]
    fn malformed_approval_aborts_legacy_header_publication() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let commit_id = repo.head_id()?.detach();
        let change_id = crate::change_id::for_commit(&repo, commit_id)?;
        let reference: gix::refs::FullName = enrich::PATCH_REF_NAME.try_into()?;
        repo.notes()?
            .replace_at_ref(reference.as_ref(), ObjectId::from(change_id), b"[patch")?;
        let notes_before = repo.find_reference(enrich::PATCH_REF_NAME)?.id().detach();
        let graph = super::super::loaded_graph(&repo)?;
        assert!(
            refackiewed(&repo, Some(&graph), commit_id, Some(true), |_| {}).is_err(),
            "the history edit cannot publish if its accompanying approval cannot be prepared"
        );
        assert_eq!(
            repo.head_id()?,
            commit_id,
            "failed approval leaves the original commit selected"
        );
        assert_eq!(repo.find_reference(enrich::PATCH_REF_NAME)?.id(), notes_before);
        Ok(())
    }
}
