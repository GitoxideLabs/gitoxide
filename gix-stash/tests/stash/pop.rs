use gix_date::parse::TimeBuf;

use crate::{
    commit_tree, git_dir, head_commit_oid, new_blob_merge_platform, new_diff_cache, open_odb, open_ref_store,
    test_committer,
};

fn pop_fixture() -> gix_testtools::Result<gix_testtools::tempfile::TempDir> {
    gix_testtools::scripted_fixture_writable("make_pop_repo.sh")
}

fn pop_two_stashes_fixture() -> gix_testtools::Result<gix_testtools::tempfile::TempDir> {
    gix_testtools::scripted_fixture_writable("make_pop_two_stashes_repo.sh")
}

fn pop_untracked_fixture() -> gix_testtools::Result<gix_testtools::tempfile::TempDir> {
    gix_testtools::scripted_fixture_writable("make_pop_untracked_repo.sh")
}

fn pop_untracked_conflict_fixture() -> gix_testtools::Result<gix_testtools::tempfile::TempDir> {
    gix_testtools::scripted_fixture_writable("make_pop_untracked_conflict_repo.sh")
}

fn pop_conflict_fixture() -> gix_testtools::Result<gix_testtools::tempfile::TempDir> {
    gix_testtools::scripted_fixture_writable("make_pop_conflict_repo.sh")
}

fn empty_repo_fixture() -> gix_testtools::Result<gix_testtools::tempfile::TempDir> {
    gix_testtools::scripted_fixture_writable("make_list_empty_repo.sh")
}

#[test]
fn pop_applies_stash_to_clean_wt() -> gix_testtools::Result {
    let tmp = pop_fixture()?;
    let worktree = tmp.path();
    let gd = git_dir(worktree);
    let refs = open_ref_store(&gd);
    let odb = open_odb(&gd)?;

    let head_oid = head_commit_oid(worktree, &refs, &odb)?;
    let head_tree = commit_tree(&odb, head_oid)?;

    let committer = test_committer();
    let mut time_buf = TimeBuf::default();
    let committer_ref = committer.to_ref(&mut time_buf);

    let mut diff_cache = new_diff_cache(worktree);
    let mut blob_merge = new_blob_merge_platform(worktree);

    let outcome = gix_stash::pop(
        gix_stash::PopContext {
            refs: &refs,
            objects: &odb,
            committer: committer_ref,
            worktree,
            blob_merge: &mut blob_merge,
            diff_cache: &mut diff_cache,
            checkout_options: gix_worktree_state::checkout::Options {
                overwrite_existing: true,
                ..Default::default()
            },
        },
        head_tree,
    )?;

    assert!(!outcome.had_conflicts, "pop of a clean stash should have no conflicts");
    assert!(
        outcome.new_top.is_none(),
        "after popping the only stash, new_top must be None"
    );

    // refs/stash must be gone.
    assert!(
        refs.try_find("refs/stash")?.is_none(),
        "refs/stash must be deleted after popping the last entry"
    );

    // The working tree file should now contain the stashed modification.
    let content = std::fs::read_to_string(worktree.join("file.txt"))?;
    assert_eq!(
        content.trim(),
        "stashed modification",
        "pop must restore the stashed working tree content"
    );

    Ok(())
}

#[test]
fn pop_drops_only_top_with_multiple_stashes() -> gix_testtools::Result {
    let tmp = pop_two_stashes_fixture()?;
    let worktree = tmp.path();
    let gd = git_dir(worktree);
    let refs = open_ref_store(&gd);
    let odb = open_odb(&gd)?;

    let head_oid = head_commit_oid(worktree, &refs, &odb)?;
    let head_tree = commit_tree(&odb, head_oid)?;

    // Record the old stash tip before popping.
    let old_stash_tip = refs
        .find("refs/stash")?
        .target
        .try_id()
        .expect("refs/stash must have an OID target")
        .to_owned();

    let committer = test_committer();
    let mut time_buf = TimeBuf::default();
    let committer_ref = committer.to_ref(&mut time_buf);

    let mut diff_cache = new_diff_cache(worktree);
    let mut blob_merge = new_blob_merge_platform(worktree);

    let outcome = gix_stash::pop(
        gix_stash::PopContext {
            refs: &refs,
            objects: &odb,
            committer: committer_ref,
            worktree,
            blob_merge: &mut blob_merge,
            diff_cache: &mut diff_cache,
            checkout_options: gix_worktree_state::checkout::Options {
                overwrite_existing: true,
                ..Default::default()
            },
        },
        head_tree,
    )?;

    assert_eq!(outcome.applied, old_stash_tip, "applied must be the old tip");

    // refs/stash must still exist (there is one more entry).
    assert!(
        refs.try_find("refs/stash")?.is_some(),
        "refs/stash must still exist after popping from a multi-entry stack"
    );

    // The new top must be Some and different from the old tip.
    let new_top = outcome.new_top.expect("new_top must be Some when more stashes remain");
    assert_ne!(new_top, old_stash_tip, "new_top must differ from the popped entry");

    // refs/stash must point at new_top.
    let current_stash = refs
        .find("refs/stash")?
        .target
        .try_id()
        .expect("refs/stash OID")
        .to_owned();
    assert_eq!(current_stash, new_top);

    Ok(())
}

#[test]
fn pop_returns_no_stash_when_unborn() -> gix_testtools::Result {
    let tmp = empty_repo_fixture()?;
    let worktree = tmp.path();
    let gd = git_dir(worktree);
    let refs = open_ref_store(&gd);
    let odb = open_odb(&gd)?;

    let head_oid = head_commit_oid(worktree, &refs, &odb)?;
    let head_tree = commit_tree(&odb, head_oid)?;

    let committer = test_committer();
    let mut time_buf = TimeBuf::default();
    let committer_ref = committer.to_ref(&mut time_buf);

    let mut diff_cache = new_diff_cache(worktree);
    let mut blob_merge = new_blob_merge_platform(worktree);

    let result = gix_stash::pop(
        gix_stash::PopContext {
            refs: &refs,
            objects: &odb,
            committer: committer_ref,
            worktree,
            blob_merge: &mut blob_merge,
            diff_cache: &mut diff_cache,
            checkout_options: Default::default(),
        },
        head_tree,
    );

    match result {
        Err(gix_stash::PopError::NoStash) => {}
        other => {
            return Err(format!("expected Err(NoStash) for a repo with no stash, got: {other:?}").into());
        }
    }

    Ok(())
}

#[test]
fn pop_restores_untracked_when_present() -> gix_testtools::Result {
    let tmp = pop_untracked_fixture()?;
    let worktree = tmp.path();
    let gd = git_dir(worktree);
    let refs = open_ref_store(&gd);
    let odb = open_odb(&gd)?;

    // After the fixture runs `git stash --include-untracked`, the untracked
    // file should NOT be on disk.
    assert!(
        !worktree.join("untracked.txt").exists(),
        "fixture post-condition: untracked.txt should be removed by git stash"
    );

    let head_oid = head_commit_oid(worktree, &refs, &odb)?;
    let head_tree = commit_tree(&odb, head_oid)?;

    let committer = test_committer();
    let mut time_buf = TimeBuf::default();
    let committer_ref = committer.to_ref(&mut time_buf);

    let mut diff_cache = new_diff_cache(worktree);
    let mut blob_merge = new_blob_merge_platform(worktree);

    let outcome = gix_stash::pop(
        gix_stash::PopContext {
            refs: &refs,
            objects: &odb,
            committer: committer_ref,
            worktree,
            blob_merge: &mut blob_merge,
            diff_cache: &mut diff_cache,
            checkout_options: gix_worktree_state::checkout::Options {
                overwrite_existing: true,
                ..Default::default()
            },
        },
        head_tree,
    )?;

    assert!(
        !outcome.had_conflicts,
        "pop of untracked-only stash should have no conflicts"
    );

    // untracked.txt must be restored.
    let content = std::fs::read_to_string(worktree.join("untracked.txt"))?;
    assert_eq!(
        content.trim(),
        "untracked content",
        "pop must restore the untracked file from parent[2]"
    );

    Ok(())
}

#[test]
fn pop_conflicts_leave_ref_intact() -> gix_testtools::Result {
    let tmp = pop_conflict_fixture()?;
    let worktree = tmp.path();
    let gd = git_dir(worktree);
    let refs = open_ref_store(&gd);
    let odb = open_odb(&gd)?;

    // HEAD is now the "content C" commit; stash has "content B" based on "content A".
    let head_oid = head_commit_oid(worktree, &refs, &odb)?;
    let head_tree = commit_tree(&odb, head_oid)?;

    let stash_tip_before = refs
        .find("refs/stash")?
        .target
        .try_id()
        .expect("refs/stash OID")
        .to_owned();

    let committer = test_committer();
    let mut time_buf = TimeBuf::default();
    let committer_ref = committer.to_ref(&mut time_buf);

    let mut diff_cache = new_diff_cache(worktree);
    let mut blob_merge = new_blob_merge_platform(worktree);

    let outcome = gix_stash::pop(
        gix_stash::PopContext {
            refs: &refs,
            objects: &odb,
            committer: committer_ref,
            worktree,
            blob_merge: &mut blob_merge,
            diff_cache: &mut diff_cache,
            checkout_options: gix_worktree_state::checkout::Options {
                overwrite_existing: true,
                ..Default::default()
            },
        },
        head_tree,
    )?;

    assert!(
        outcome.had_conflicts,
        "merging stash B onto HEAD C (base A) must produce conflicts"
    );

    // refs/stash must still point at the same OID (not dropped on conflict).
    let stash_tip_after = refs
        .find("refs/stash")?
        .target
        .try_id()
        .expect("refs/stash OID")
        .to_owned();
    assert_eq!(
        stash_tip_after, stash_tip_before,
        "refs/stash must not be updated on a conflicted pop"
    );

    Ok(())
}

/// When restoring untracked files (`parent[2]`) during `pop`, if a target
/// path already exists on disk, the pop must report a conflict and leave
/// `refs/stash` intact so no data is lost.
#[test]
fn pop_conflicts_on_untracked_restore_when_target_exists() -> gix_testtools::Result {
    let tmp = pop_untracked_conflict_fixture()?;
    let worktree = tmp.path();
    let gd = git_dir(worktree);
    let refs = open_ref_store(&gd);
    let odb = open_odb(&gd)?;

    // The fixture leaves untracked.txt on disk after stashing.
    assert!(
        worktree.join("untracked.txt").exists(),
        "fixture post-condition: untracked.txt must exist on disk"
    );

    let stash_tip_before = refs
        .find("refs/stash")?
        .target
        .try_id()
        .expect("refs/stash OID")
        .to_owned();

    let head_oid = head_commit_oid(worktree, &refs, &odb)?;
    let head_tree = commit_tree(&odb, head_oid)?;

    let committer = test_committer();
    let mut time_buf = TimeBuf::default();
    let committer_ref = committer.to_ref(&mut time_buf);

    let mut diff_cache = new_diff_cache(worktree);
    let mut blob_merge = new_blob_merge_platform(worktree);

    let outcome = gix_stash::pop(
        gix_stash::PopContext {
            refs: &refs,
            objects: &odb,
            committer: committer_ref,
            worktree,
            blob_merge: &mut blob_merge,
            diff_cache: &mut diff_cache,
            checkout_options: gix_worktree_state::checkout::Options {
                overwrite_existing: true,
                ..Default::default()
            },
        },
        head_tree,
    )?;

    // had_conflicts must be true — an existing file would be clobbered.
    assert!(
        outcome.had_conflicts,
        "pop must report had_conflicts=true when untracked restore would clobber an existing file"
    );

    // refs/stash must still point at the original stash commit.
    let stash_tip_after = refs
        .find("refs/stash")?
        .target
        .try_id()
        .expect("refs/stash OID")
        .to_owned();
    assert_eq!(
        stash_tip_after, stash_tip_before,
        "refs/stash must not be dropped when untracked restore has a conflict"
    );

    // The user's file must not have been overwritten.
    let content = std::fs::read_to_string(worktree.join("untracked.txt"))?;
    assert_eq!(
        content.trim(),
        "user's own content",
        "existing file must not be clobbered during a conflicted pop"
    );

    Ok(())
}
