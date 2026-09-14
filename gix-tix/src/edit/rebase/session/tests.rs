use std::path::Path;

use super::*;
use crate::edit::{self, rebase};

fn git(path: &Path, args: &[&str]) -> gix_testtools::Result {
    let output = gix_testtools::git_command(path).args(args).output()?;
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn conflict(repo: &gix::Repository) -> Result<rebase::PlanConflict> {
    let tip_commit_id = repo.head_id()?.detach();
    let middle_commit_id = repo.rev_parse_single("HEAD~1")?.detach();
    let base_commit_id = repo.rev_parse_single("HEAD~2")?.detach();
    let scope = vec![middle_commit_id, tip_commit_id];
    let plan = Plan {
        base: base_commit_id,
        steps: vec![rebase::PlanStep {
            commit: rebase::PlanCommit::Pick(tip_commit_id),
            parents: vec![PlanParent::Existing(base_commit_id)],
            squash: Vec::new(),
        }],
        checkout: Some(rebase::PlanCheckout {
            target: PlanParent::Step(0),
            reference: repo.head()?.referent_name().map(ToOwned::to_owned),
        }),
        expected_refs: rebase::capture_refs(repo, &scope, &[tip_commit_id])?,
        scope,
        eager: Vec::new(),
        selection: None,
    };
    match rebase::perform_plan(repo, &edit::loaded_graph(repo)?, plan)? {
        rebase::PlanPerform::Conflict(conflict) => Ok(conflict),
        rebase::PlanPerform::Complete(_) => anyhow::bail!("removing the same-line middle edit must conflict"),
    }
}

pub(crate) fn paused() -> gix_testtools::Result<(gix_testtools::tempfile::TempDir, gix::Repository, ObjectId)> {
    let fixture = gix_testtools::scripted_fixture_writable("rebase_conflict.sh")?;
    let repo = crate::test_repository::open_with(fixture.path(), ["core.filesRefLockTimeout=0"])?;
    let original_commit_id = repo.head_id()?.detach();
    let mut conflict = conflict(&repo)?;
    let tips = vec![conflict.commit()];
    conflict.save_continuation(tips, "rebase")?;
    edit::time_travel::materialize_plan_conflict_reporting(conflict, &[], false)?;
    Ok((fixture, repo, original_commit_id))
}

fn resolve(path: &Path) -> gix_testtools::Result {
    std::fs::write(path.join("file"), "resolved\n")?;
    git(path, &["add", "file"])
}

fn resume(repo: &gix::Repository) -> Result<rebase::PlanPerform> {
    let plan = load(repo)?.context("the test has a saved rebase")?.parsed(repo)?.plan;
    rebase::perform_plan(repo, &edit::loaded_graph(repo)?, plan)
}

#[test]
fn session_is_private_to_its_worktree_hidden_from_history_and_retained_by_gc() -> gix_testtools::Result {
    let (fixture, repo, original_commit_id) = paused()?;
    let session = load(&repo)?.context("the conflict saves a session")?;
    let graph = edit::loaded_graph(&repo)?;
    assert!(
        graph.parents_of(session.commit_id).is_none(),
        "metadata commits never enter edit history"
    );
    for revision in [REF.to_owned(), session.commit_id.to_string()] {
        assert!(
            crate::history::resolve_revision(&repo, revision.as_bytes().as_bstr()).is_err(),
            "internal state cannot become a travel destination"
        );
    }
    let worktree = gix_testtools::tempfile::tempdir()?;
    let linked = worktree.path().join("linked");
    let output = gix_testtools::git_command(fixture.path())
        .args(["worktree", "add", "--detach"])
        .arg(&linked)
        .arg(original_commit_id.to_string())
        .output()?;
    assert!(
        output.status.success(),
        "a separate checkout can be created while the original is paused"
    );
    let linked = crate::test_repository::open(&linked)?;
    assert!(load(&linked)?.is_none(), "another worktree has no active operation");
    stop(&linked)?;
    assert_eq!(
        load(&repo)?.context("the original session remains")?.commit_id,
        session.commit_id,
        "stopping another worktree is harmless"
    );

    git(fixture.path(), &["reflog", "expire", "--expire=now", "--all"])?;
    git(fixture.path(), &["gc", "--prune=now"])?;
    resolve(fixture.path())?;
    resume(&repo)?.complete()?;
    assert!(load(&repo)?.is_none(), "the retained plan completes after GC");
    undo::plan_undo(&repo)?
        .context("the original state is retained for undo")?
        .apply(&repo)?;
    assert_eq!(
        repo.head_id()?,
        original_commit_id,
        "GC does not lose the original checkout"
    );
    Ok(())
}

#[test]
fn amendments_and_stop_share_one_undo_entry_without_restoring_the_session() -> gix_testtools::Result {
    let (fixture, repo, original_commit_id) = paused()?;
    let paused_commit_id = load(&repo)?.context("the rebase is saved")?.commit_id;
    resolve(fixture.path())?;
    let outcome = edit::head::amend_index_reporting(repo.clone(), &edit::loaded_graph(&repo)?)?
        .context("the conflict is amended")?;
    assert!(
        outcome.ref_changes.is_empty(),
        "the session owns the amendment's undo changes"
    );
    let updated = load(&repo)?.context("amending keeps the remaining todo")?;
    assert_ne!(
        updated.commit_id, paused_commit_id,
        "the accepted amendment publishes a new state version"
    );
    assert_eq!(
        status(&repo)?.context("the operation is visible")?.readiness,
        Readiness::Ready,
        "the saved replacement is ready for continuation"
    );
    let head_commit_id = repo.head_id()?.detach();
    let index = std::fs::read(repo.index_path())?;
    let contents = std::fs::read(fixture.path().join("file"))?;
    stop(&repo)?;
    stop(&repo)?;
    assert!(load(&repo)?.is_none(), "stop is idempotent");
    assert_eq!(repo.head_id()?, head_commit_id, "stop keeps the partial commit");
    assert_eq!(
        std::fs::read(repo.index_path())?,
        index,
        "stop leaves the index byte-for-byte intact"
    );
    assert_eq!(
        std::fs::read(fixture.path().join("file"))?,
        contents,
        "stop preserves the resolution"
    );
    assert_eq!(
        undo::history(&repo)?.titles,
        ["rebase history"],
        "pause and amendment form one operation"
    );
    undo::plan_undo(&repo)?
        .context("the partial operation can be undone")?
        .apply(&repo)?;
    assert_eq!(repo.head_id()?, original_commit_id, "undo returns before the pause");
    assert!(load(&repo)?.is_none(), "undo does not resurrect sequencer metadata");
    undo::plan_redo(&repo)?
        .context("the partial operation can be redone")?
        .apply(&repo)?;
    assert_eq!(repo.head_id()?, head_commit_id, "redo restores the amendment");
    Ok(())
}

#[test]
fn incompatible_head_or_refs_block_continuation_without_retargeting() -> gix_testtools::Result {
    for moved_head in [false, true] {
        let (fixture, repo, original_commit_id) = paused()?;
        let base_commit_id = repo
            .find_commit(original_commit_id)?
            .parent_ids()
            .next()
            .context("tip has a parent")?
            .detach();
        if moved_head {
            git(fixture.path(), &["reset", "--hard", &base_commit_id.to_string()])?;
        } else {
            git(
                fixture.path(),
                &["update-ref", "refs/heads/main", &base_commit_id.to_string()],
            )?;
        }
        let before = gix_testtools::repository::snapshot(fixture.path())?;
        assert!(
            matches!(
                status(&repo)?.context("the saved operation remains visible")?.readiness,
                Readiness::Blocked(_)
            ),
            "an incompatible checkout or expected ref blocks the session"
        );
        assert!(resume(&repo).is_err(), "continuation cannot silently retarget itself");
        assert_eq!(
            gix_testtools::repository::snapshot(fixture.path())?,
            before,
            "inspection and refusal leave the repository unchanged"
        );
        stop(&repo)?;
        assert!(load(&repo)?.is_none(), "blocked operations can still be stopped");
    }
    Ok(())
}

#[test]
fn malformed_state_is_visible_and_can_be_stopped_without_touching_the_checkout() -> gix_testtools::Result {
    let (fixture, repo, _) = paused()?;
    let mut commit = repo.head_commit()?.decode()?.into_owned()?;
    commit.message = "broken rebase state".into();
    let broken_commit_id = repo.write_object(&commit)?.detach();
    git(fixture.path(), &["update-ref", REF, &broken_commit_id.to_string()])?;
    let before = repo.head_id()?.detach();
    let index = std::fs::read(repo.index_path())?;
    assert!(
        matches!(
            status(&repo)?
                .context("broken state is still an active operation")?
                .readiness,
            Readiness::Blocked(_)
        ),
        "malformed metadata is reported as blocked"
    );
    assert!(
        ensure_idle(&repo).is_err(),
        "malformed state also blocks unrelated mutations"
    );
    assert!(
        stop(&repo)?.is_some(),
        "stop explains why the damaged undo payload could not be recorded"
    );
    assert_eq!(repo.head_id()?, before, "forgetting malformed state does not move HEAD");
    assert_eq!(
        std::fs::read(repo.index_path())?,
        index,
        "forgetting malformed state does not touch the index"
    );
    Ok(())
}

fn lock(repo: &gix::Repository, reference: &str) -> Result<std::path::PathBuf> {
    let path = repo.git_dir().join(format!("{reference}.lock"));
    std::fs::create_dir_all(path.parent().context("a ref lock has a directory")?)?;
    std::fs::write(&path, "held by another operation")?;
    Ok(path)
}

#[test]
fn failed_state_publication_cannot_materialize_a_conflict() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("rebase_conflict.sh")?;
    let repo = crate::test_repository::open_with(fixture.path(), ["core.filesRefLockTimeout=0"])?;
    let mut conflict = conflict(&repo)?;
    conflict.save_continuation(Vec::new(), "rebase")?;
    let held = lock(&repo, REF)?;
    let before = gix_testtools::repository::snapshot(fixture.path())?;
    edit::time_travel::materialize_plan_conflict_reporting(conflict, &[], false)
        .expect_err("a held state ref prevents publication");
    assert_eq!(
        gix_testtools::repository::snapshot(fixture.path())?,
        before,
        "failed state publication changes no refs, index, or files"
    );
    assert!(load(&repo)?.is_none(), "no accepted pause was published");
    std::fs::remove_file(held)?;
    Ok(())
}

#[test]
fn failed_completion_restores_the_session_and_staged_resolution() -> gix_testtools::Result {
    let (fixture, repo, _) = paused()?;
    resolve(fixture.path())?;
    let held = lock(&repo, undo::TIP_REF)?;
    let session_commit_id = load(&repo)?.context("the pause is saved")?.commit_id;
    let before = gix_testtools::repository::snapshot(fixture.path())?;
    assert!(resume(&repo).is_err(), "a held undo ref prevents successful completion");
    assert_eq!(
        load(&repo)?.context("failed completion keeps the session")?.commit_id,
        session_commit_id,
        "rollback restores the exact state version"
    );
    assert_eq!(
        gix_testtools::repository::snapshot(fixture.path())?,
        before,
        "a failed publication must preserve the staged resolution and working files"
    );
    std::fs::remove_file(held)?;
    resume(&repo)?.complete()?;
    Ok(())
}

#[test]
fn a_competing_stop_prevents_publication_of_a_prepared_continuation() -> gix_testtools::Result {
    let (fixture, repo, _) = paused()?;
    resolve(fixture.path())?;
    let mut plan = load(&repo)?.context("the pause is saved")?.parsed(&repo)?.plan;
    let mut publication = Publication::for_plan(&repo, &mut plan)?.context("the plan owns the active state version")?;
    stop(&repo)?;
    let before = gix_testtools::repository::snapshot(fixture.path())?;
    let mut edits = Vec::new();
    publication.reserve(&repo, &mut edits, &mut Vec::new())?;
    assert!(
        repo.edit_references(edits).is_err(),
        "the original state-ref OID guards a competing stop"
    );
    assert_eq!(
        gix_testtools::repository::snapshot(fixture.path())?,
        before,
        "the stale continuation cannot overwrite the stopped operation"
    );
    Ok(())
}

#[test]
fn stop_cannot_race_checkout_but_can_clear_an_interrupted_publication() -> gix_testtools::Result {
    let (fixture, repo, _) = paused()?;
    resolve(fixture.path())?;
    let mut plan = load(&repo)?.context("the pause is saved")?.parsed(&repo)?.plan;
    let mut publication =
        Publication::for_plan(&repo, &mut plan)?.context("the continuation owns the saved version")?;
    let mut edits = Vec::new();
    publication.reserve(&repo, &mut edits, &mut Vec::new())?;
    repo.edit_references(edits)?;
    assert!(
        stop(&repo).is_err(),
        "stop cannot delete a record while its publication owns the worktree lock"
    );
    drop(publication);
    assert!(
        matches!(
            status(&repo)?
                .context("the interrupted record remains visible")?
                .readiness,
            Readiness::Blocked(_)
        ),
        "an interrupted publication is blocked"
    );
    stop(&repo)?;
    assert!(
        load(&repo)?.is_none(),
        "an interrupted publication can be forgotten after its writer exits"
    );
    Ok(())
}
