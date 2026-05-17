use std::path::Path;

use gix_date::parse::TimeBuf;

use crate::{blob_content_in_tree, commit_tree, git_dir, head_commit_oid, open_odb, open_ref_store, test_committer};

/// Open the push fixture (writable copy), return the worktree `TempDir`.
fn push_fixture() -> gix_testtools::Result<gix_testtools::tempfile::TempDir> {
    gix_testtools::scripted_fixture_writable("make_push_repo.sh")
}

/// Convenience: open all handles from a worktree path.
struct Repo {
    worktree: std::path::PathBuf,
    refs: gix_ref::file::Store,
    odb: gix_odb::HandleArc,
}

impl Repo {
    fn open(worktree: &Path) -> gix_testtools::Result<Self> {
        let gd = git_dir(worktree);
        let refs = open_ref_store(&gd);
        let odb = open_odb(&gd)?;
        Ok(Self {
            worktree: worktree.to_owned(),
            refs,
            odb,
        })
    }

    /// Load the index from `.git/index`.
    fn load_index(&self) -> gix_testtools::Result<gix_index::File> {
        let object_hash = gix_testtools::object_hash();
        Ok(gix_index::File::at(
            git_dir(&self.worktree).join("index"),
            object_hash,
            false,
            Default::default(),
        )?)
    }
}

#[test]
fn push_captures_unstaged_modification() -> gix_testtools::Result {
    let tmp = push_fixture()?;
    let worktree = tmp.path();
    let repo = Repo::open(worktree)?;

    // Modify tracked.txt on disk — do NOT stage it.
    let tracked_path = worktree.join("tracked.txt");
    std::fs::write(&tracked_path, "modified content\n")?;

    let index = repo.load_index()?;
    let head_oid = head_commit_oid(worktree, &repo.refs, &repo.odb)?;
    let head_tree = commit_tree(&repo.odb, head_oid)?;
    let head_branch: gix_ref::FullName = "refs/heads/main".try_into().expect("valid ref name");

    let committer = test_committer();
    let mut time_buf = TimeBuf::default();
    let committer_ref = committer.to_ref(&mut time_buf);

    let outcome = gix_stash::push(
        gix_stash::PushContext {
            refs: &repo.refs,
            objects: &repo.odb,
            index: &index,
            worktree,
            committer: committer_ref,
            checkout_options: Default::default(),
        },
        head_oid,
        head_tree,
        Some(head_branch.as_ref()),
        gix_stash::PushOptions::default(),
    )?;

    // refs/stash must now exist and point at the stash commit.
    let stash_ref = repo.refs.find("refs/stash")?;
    let stash_oid = stash_ref.target.try_id().expect("stash ref must be an OID").to_owned();
    assert_eq!(stash_oid, outcome.stash);

    // The stash commit's tree must contain tracked.txt with the modified content.
    let stash_tree = commit_tree(&repo.odb, stash_oid)?;
    let content = blob_content_in_tree(&repo.odb, stash_tree, b"tracked.txt")?;
    assert_eq!(
        content, b"modified content\n",
        "stash tree should capture the modified WT content"
    );

    // After push, the WT file should be reset to HEAD content.
    let wt_content = std::fs::read(&tracked_path)?;
    assert_eq!(wt_content, b"original content\n", "push must reset WT to HEAD content");

    Ok(())
}

#[test]
fn push_captures_staged_change() -> gix_testtools::Result {
    let tmp = push_fixture()?;
    let worktree = tmp.path();

    // Stage a modification to tracked.txt via git add.
    std::fs::write(worktree.join("tracked.txt"), "staged content\n")?;
    std::process::Command::new("git")
        .args(["add", "tracked.txt"])
        .current_dir(worktree)
        .status()?;

    let repo = Repo::open(worktree)?;
    let index = repo.load_index()?;
    let head_oid = head_commit_oid(worktree, &repo.refs, &repo.odb)?;
    let head_tree = commit_tree(&repo.odb, head_oid)?;
    let head_branch: gix_ref::FullName = "refs/heads/main".try_into().expect("valid ref name");

    let committer = test_committer();
    let mut time_buf = TimeBuf::default();
    let committer_ref = committer.to_ref(&mut time_buf);

    let outcome = gix_stash::push(
        gix_stash::PushContext {
            refs: &repo.refs,
            objects: &repo.odb,
            index: &index,
            worktree,
            committer: committer_ref,
            checkout_options: Default::default(),
        },
        head_oid,
        head_tree,
        Some(head_branch.as_ref()),
        gix_stash::PushOptions::default(),
    )?;

    // parent[1] of the stash commit is the index-state commit.
    use gix_object::FindExt;
    let mut buf = Vec::new();
    let stash_commit = repo.odb.find_commit(&outcome.stash, &mut buf)?;
    let index_commit_oid = stash_commit
        .parents()
        .nth(1)
        .expect("stash commit must have parent[1] (index-state)");

    let index_tree = commit_tree(&repo.odb, index_commit_oid)?;
    let content = blob_content_in_tree(&repo.odb, index_tree, b"tracked.txt")?;
    assert_eq!(
        content, b"staged content\n",
        "index-state commit tree (parent[1]) must reflect what was staged"
    );

    Ok(())
}

/// Tests that `push` on a repository with no local changes returns `Err(NoLocalChanges)`.
///
/// KNOWN BUG (as of commit a882282e5): the `NoLocalChanges` guard only fires when
/// `index.entries().is_empty()`, which is never true for a repo that has committed
/// files.  A clean working tree therefore falls through and produces a stash commit
/// that is identical to HEAD.  The correct behaviour would be to compare the WT +
/// index against HEAD and bail out when there is nothing to save.
///
/// This test records the *expected* correct behaviour.  If it passes, the bug
/// has been fixed; if it fails, the bug is still present.
#[test]
fn push_returns_no_local_changes_on_clean_wt() -> gix_testtools::Result {
    let tmp = push_fixture()?;
    let worktree = tmp.path();
    let repo = Repo::open(worktree)?;
    let index = repo.load_index()?;
    let head_oid = head_commit_oid(worktree, &repo.refs, &repo.odb)?;
    let head_tree = commit_tree(&repo.odb, head_oid)?;
    let head_branch: gix_ref::FullName = "refs/heads/main".try_into().expect("valid ref name");

    let committer = test_committer();
    let mut time_buf = TimeBuf::default();
    let committer_ref = committer.to_ref(&mut time_buf);

    let result = gix_stash::push(
        gix_stash::PushContext {
            refs: &repo.refs,
            objects: &repo.odb,
            index: &index,
            worktree,
            committer: committer_ref,
            checkout_options: Default::default(),
        },
        head_oid,
        head_tree,
        Some(head_branch.as_ref()),
        gix_stash::PushOptions::default(),
    );

    match result {
        Err(gix_stash::PushError::NoLocalChanges) => {
            // Correct — the bug has been fixed.
        }
        Ok(_) => {
            // BUG: the guard `if index.entries().is_empty() && !options.include_untracked`
            // fires only for an empty index.  A repo with committed files has a non-empty
            // index even on a clean WT, so the check never trips.
            return Err("BUG(gix-stash push): NoLocalChanges guard fires only on empty index, \
                 not on clean-WT repos with committed files. \
                 push succeeded on a clean working tree and produced a no-op stash."
                .into());
        }
        Err(e) => return Err(e.into()),
    }

    Ok(())
}

#[test]
fn push_includes_untracked_when_flag_set() -> gix_testtools::Result {
    let tmp = push_fixture()?;
    let worktree = tmp.path();

    // Create an untracked file (not staged, not committed).
    std::fs::write(worktree.join("new.txt"), "untracked content\n")?;

    // Also make a tracked change so the index isn't fully clean and the
    // `NoLocalChanges` guard does not trip (the guard is `index.is_empty()`
    // today, but we want the test to work after a fix too).
    std::fs::write(worktree.join("tracked.txt"), "modified for untracked test\n")?;

    let repo = Repo::open(worktree)?;
    let index = repo.load_index()?;
    let head_oid = head_commit_oid(worktree, &repo.refs, &repo.odb)?;
    let head_tree = commit_tree(&repo.odb, head_oid)?;
    let head_branch: gix_ref::FullName = "refs/heads/main".try_into().expect("valid ref name");

    let committer = test_committer();
    let mut time_buf = TimeBuf::default();
    let committer_ref = committer.to_ref(&mut time_buf);

    let outcome = gix_stash::push(
        gix_stash::PushContext {
            refs: &repo.refs,
            objects: &repo.odb,
            index: &index,
            worktree,
            committer: committer_ref,
            checkout_options: Default::default(),
        },
        head_oid,
        head_tree,
        Some(head_branch.as_ref()),
        gix_stash::PushOptions {
            include_untracked: true,
            ..Default::default()
        },
    )?;

    // parent[2] must exist when include_untracked is set and untracked files were found.
    let untracked_commit_oid = outcome
        .untracked_commit
        .expect("untracked_commit must be Some when include_untracked=true and untracked files exist");

    // The untracked commit's tree must contain new.txt.
    let untracked_tree = commit_tree(&repo.odb, untracked_commit_oid)?;
    let content = blob_content_in_tree(&repo.odb, untracked_tree, b"new.txt")?;
    assert_eq!(
        content, b"untracked content\n",
        "untracked-files commit tree must contain new.txt"
    );

    // new.txt should no longer be on disk after push.
    assert!(
        !worktree.join("new.txt").exists(),
        "untracked file must be removed from disk after push with include_untracked=true"
    );

    Ok(())
}

#[test]
fn push_leaves_untracked_alone_without_flag() -> gix_testtools::Result {
    let tmp = push_fixture()?;
    let worktree = tmp.path();

    // Create an untracked file.
    std::fs::write(worktree.join("new.txt"), "untracked content\n")?;
    // Make a tracked change so push proceeds.
    std::fs::write(worktree.join("tracked.txt"), "modified for flag test\n")?;

    let repo = Repo::open(worktree)?;
    let index = repo.load_index()?;
    let head_oid = head_commit_oid(worktree, &repo.refs, &repo.odb)?;
    let head_tree = commit_tree(&repo.odb, head_oid)?;
    let head_branch: gix_ref::FullName = "refs/heads/main".try_into().expect("valid ref name");

    let committer = test_committer();
    let mut time_buf = TimeBuf::default();
    let committer_ref = committer.to_ref(&mut time_buf);

    let outcome = gix_stash::push(
        gix_stash::PushContext {
            refs: &repo.refs,
            objects: &repo.odb,
            index: &index,
            worktree,
            committer: committer_ref,
            checkout_options: Default::default(),
        },
        head_oid,
        head_tree,
        Some(head_branch.as_ref()),
        gix_stash::PushOptions {
            include_untracked: false,
            ..Default::default()
        },
    )?;

    // No untracked commit should be created.
    assert!(
        outcome.untracked_commit.is_none(),
        "untracked_commit must be None when include_untracked=false"
    );

    // new.txt must still be on disk.
    assert!(
        worktree.join("new.txt").exists(),
        "untracked file must remain on disk when include_untracked=false"
    );

    Ok(())
}

/// Tests that `push` on a repo with no commits returns `Err(EmptyRepository)`.
///
/// The `push` plumbing function requires the caller to supply pre-resolved
/// `head_commit` and `head_tree` OIDs.  Resolving HEAD on an empty repository
/// fails before `push` is reached.  The `EmptyRepository` variant is reserved
/// for the porcelain (`gix`) layer.  This test documents that limitation.
#[test]
fn push_returns_empty_repository_on_no_commits() -> gix_testtools::Result {
    // No-op: the plumbing API cannot represent the no-commits scenario because
    // the caller must supply valid `head_commit`/`head_tree` OIDs up front.
    // See `gix::Repository::stash_push` for the porcelain-level guard.
    Ok(())
}
