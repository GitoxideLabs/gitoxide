//! API-shape test for `Repository::push` convenience method (Task 8.1).
//!
//! Validates the thin wrapper over `Connection::prepare_push` + `transmit`
//! using the same `file://` fixture as `tests/gix/remote/push.rs`.

/// Smoke test: `Repository::push` performs a fast-forward push to a bare file remote.
#[test]
fn repository_push_fast_forward_to_bare_file_remote() {
    let (src_dir, dst_dir) = setup_push_repos();

    // Open as isolated so no ambient git config bleeds in.
    let repo = gix::open_opts(&src_dir, gix::open::Options::isolated()).expect("open source repo");

    // Register an in-memory remote named "origin" pointing at the bare dst.
    let dst_url = format!("file://{}", dst_dir.display());
    let remote = repo
        .remote_at(dst_url.as_str())
        .expect("valid file:// URL")
        .with_fetch_tags(gix::remote::fetch::Tags::None);

    // Save the remote under the name "origin" so that `push("origin", ...)` can find it.
    // Because `open_opts(...isolated())` prevents writing to the real config, we
    // exercise the convenience method via `remote_at` + direct chain to keep it simple.
    // We call `prepare_push` + `transmit` through the same path that `Repository::push`
    // would take, verifying the full vertical in the unit test below.
    let conn = remote
        .connect(gix::remote::Direction::Push)
        .expect("connect to bare dst");
    let prepare = conn
        .prepare_push(gix_features::progress::Discard)
        .expect("prepare_push");
    let outcome = prepare
        .with_refspecs(["refs/heads/main:refs/heads/main"])
        .transmit(
            gix_features::progress::Discard,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("transmit");

    assert_eq!(outcome.status.len(), 1, "expected one ref status");
    assert!(
        outcome.status[0].result.is_ok(),
        "push should succeed; got: {:?}",
        outcome.status[0].result
    );

    // Verify the bare remote actually has the ref.
    let bare = gix::open_opts(&dst_dir, gix::open::Options::isolated()).expect("open bare dst");
    let mut main_ref = bare
        .find_reference("refs/heads/main")
        .expect("refs/heads/main must exist after push");
    let _commit = main_ref.peel_to_id().expect("refs/heads/main must resolve");
}

/// Test `Repository::push` convenience method with a named remote saved in config.
///
/// Because `open_opts(...isolated())` strips ambient git config, we build a repo
/// on disk with `git remote add origin <url>` so that `find_remote("origin")` works.
#[test]
fn repository_push_via_named_remote() {
    let (src_dir, dst_dir) = setup_push_repos();

    // Add a named remote "origin" to the source repo via git.
    let dst_url = format!("file://{}", dst_dir.display());
    let status = std::process::Command::new("git")
        .current_dir(&src_dir)
        .args(["remote", "add", "origin", &dst_url])
        .status()
        .expect("spawn git remote add");
    assert!(status.success(), "git remote add failed: {status}");

    // Now open the repo (non-isolated so it reads the .git/config we just wrote).
    let repo = gix::open(&src_dir).expect("open source repo");

    let outcome = repo
        .push("origin", ["refs/heads/main:refs/heads/main"])
        .expect("push via Repository::push");

    assert_eq!(outcome.status.len(), 1, "expected one ref status");
    assert!(
        outcome.status[0].result.is_ok(),
        "push should succeed; got: {:?}",
        outcome.status[0].result
    );

    // Verify the bare remote.
    let bare = gix::open(&dst_dir).expect("open bare dst");
    let mut main_ref = bare
        .find_reference("refs/heads/main")
        .expect("refs/heads/main must exist after push");
    let _commit = main_ref.peel_to_id().expect("refs/heads/main must resolve");
}

/// Create a (src, dst.git) repo pair under a temporary directory.
///
/// Same shape as `tests/gix/remote/push.rs::setup_push_repos`.
fn setup_push_repos() -> (std::path::PathBuf, std::path::PathBuf) {
    let tmp = gix_testtools::tempfile::TempDir::new().expect("create temp dir");

    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst.git");

    let run = |dir: &std::path::Path, args: &[&str]| {
        let status = std::process::Command::new("git")
            .current_dir(dir)
            .args(args)
            .status()
            .unwrap_or_else(|e| panic!("failed to spawn git {args:?}: {e}"));
        assert!(status.success(), "git {args:?} in {dir:?} exited with {status}");
    };

    std::fs::create_dir(&src).expect("create src dir");
    run(&src, &["init", "-q", "-b", "main"]);
    run(
        &src,
        &[
            "-c",
            "user.email=test@example.com",
            "-c",
            "user.name=Test",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--allow-empty",
            "-q",
            "-m",
            "init",
        ],
    );

    std::fs::create_dir(&dst).expect("create dst dir");
    run(&dst, &["init", "--bare", "-q"]);

    let tmp_path = tmp.keep();
    (tmp_path.join("src"), tmp_path.join("dst.git"))
}
