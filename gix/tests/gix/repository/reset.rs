use std::{path::Path, process::Command, sync::atomic::AtomicBool};

use gix::progress::Discard;

fn git(cwd: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .status()
        .expect("git available");
    assert!(status.success(), "git {args:?} failed");
}

fn open(path: &Path) -> gix::Repository {
    gix::open_opts(
        path,
        gix::open::Options::isolated().config_overrides(["user.name=gitoxide", "user.email=gitoxide@localhost"]),
    )
    .expect("open repo")
}

#[test]
fn hard_reset_moves_branch_index_and_worktree() -> crate::Result {
    let tmp = gix_testtools::tempfile::tempdir()?;
    let work = tmp.path().join("work");
    git(tmp.path(), &["init", "-b", "master", work.to_str().unwrap()]);

    std::fs::write(work.join("a.txt"), "one\n")?;
    git(&work, &["add", "a.txt"]);
    git(&work, &["commit", "-m", "one"]);
    let first = open(&work).head_id()?.detach();

    std::fs::write(work.join("a.txt"), "two\n")?;
    std::fs::write(work.join("b.txt"), "new\n")?;
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-m", "two"]);

    std::fs::remove_file(work.join("b.txt"))?;
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-m", "three"]);
    let tip = open(&work).head_id()?.detach();

    // Walk back so HEAD/index/worktree look like the intermediate commit that still had b.txt.
    git(&work, &["reset", "--hard", &first.to_hex().to_string()]);
    // Re-introduce a tracked b.txt that the tip no longer has.
    std::fs::write(work.join("b.txt"), "stale\n")?;
    git(&work, &["add", "b.txt"]);
    // Leave a dirty tracked a.txt as well.
    std::fs::write(work.join("a.txt"), "dirty\n")?;

    let repo = open(&work);
    let outcome = repo.reset_hard(
        tip,
        Discard,
        &AtomicBool::default(),
        gix::repository::reset::Options::default(),
    )?;
    assert_eq!(outcome.commit_id, tip);

    assert_eq!(std::fs::read_to_string(work.join("a.txt"))?, "two\n");
    assert!(
        !work.join("b.txt").exists(),
        "tracked path removed by hard reset must disappear from the worktree"
    );
    let head = open(&work).head_id()?.detach();
    assert_eq!(head, tip);
    Ok(())
}

#[test]
fn hard_reset_ff_only_rejects_diverged_history() -> crate::Result {
    let tmp = gix_testtools::tempfile::tempdir()?;
    let work = tmp.path().join("work");
    git(tmp.path(), &["init", "-b", "master", work.to_str().unwrap()]);
    std::fs::write(work.join("a.txt"), "base\n")?;
    git(&work, &["add", "a.txt"]);
    git(&work, &["commit", "-m", "base"]);

    git(&work, &["checkout", "-b", "side"]);
    std::fs::write(work.join("a.txt"), "side\n")?;
    git(&work, &["add", "a.txt"]);
    git(&work, &["commit", "-m", "side"]);
    let side = open(&work).head_id()?.detach();

    git(&work, &["checkout", "master"]);
    std::fs::write(work.join("a.txt"), "main\n")?;
    git(&work, &["add", "a.txt"]);
    git(&work, &["commit", "-m", "main"]);

    let repo = open(&work);
    let err = repo
        .reset_hard(
            side,
            Discard,
            &AtomicBool::default(),
            gix::repository::reset::Options {
                require_fast_forward: true,
                ..Default::default()
            },
        )
        .expect_err("diverged histories must fail with require_fast_forward");
    assert!(
        matches!(err, gix::repository::reset::Error::NotFastForward { .. }),
        "unexpected error: {err:?}"
    );
    Ok(())
}

#[test]
fn hard_reset_leaves_untracked_files_alone() -> crate::Result {
    let tmp = gix_testtools::tempfile::tempdir()?;
    let work = tmp.path().join("work");
    git(tmp.path(), &["init", "-b", "master", work.to_str().unwrap()]);
    std::fs::write(work.join("a.txt"), "one\n")?;
    git(&work, &["add", "a.txt"]);
    git(&work, &["commit", "-m", "one"]);

    std::fs::write(work.join("a.txt"), "dirty\n")?;
    std::fs::write(work.join("untracked.txt"), "keep me\n")?;

    let repo = open(&work);
    let tip = repo.head_id()?.detach();
    // Reset to same commit: discards dirty tracked content, keeps untracked.
    repo.reset_hard(
        tip,
        Discard,
        &AtomicBool::default(),
        gix::repository::reset::Options::default(),
    )?;

    assert_eq!(std::fs::read_to_string(work.join("a.txt"))?, "one\n");
    assert_eq!(std::fs::read_to_string(work.join("untracked.txt"))?, "keep me\n");
    Ok(())
}
