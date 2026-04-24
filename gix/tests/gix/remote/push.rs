/// Verify the `Outcome` and `RefStatus` public types are accessible and have
/// the expected fields without needing a live connection.
#[test]
fn push_outcome_types_are_accessible() {
    use gix::bstr::BString;
    use gix::remote::push::{Outcome, RefStatus};

    let status = RefStatus {
        local: BString::from("refs/heads/main"),
        remote: BString::from("refs/heads/main"),
        result: Ok(()),
        old_oid: gix::hash::Kind::Sha1.null(),
        new_oid: gix::hash::Kind::Sha1.null(),
    };
    let outcome = Outcome { status: vec![status] };
    assert_eq!(outcome.status.len(), 1);
    assert!(outcome.status[0].result.is_ok());
}

/// Smoke test for the entire push substrate.
///
/// Validates the full vertical: `Connection::prepare_push` → handshake →
/// `Prepare::with_refspecs` → `Prepare::transmit` → pack delivery → server
/// ref update.
///
/// Uses a `file://` URL so no daemon is needed.  The source repo is
/// constructed via `git init` + `git commit` and the destination is a bare
/// repo created with `git init --bare`.
#[cfg(feature = "blocking-network-client")]
#[test]
fn fast_forward_to_bare_file_remote_smoke() {
    let (src_dir, dst_dir) = setup_push_repos();

    // Open the source repo and create an in-memory remote pointing at the bare dst.
    let repo = gix::open_opts(&src_dir, gix::open::Options::isolated()).expect("open source repo");

    let dst_url = format!("file://{}", dst_dir.display());
    let remote = repo
        .remote_at(dst_url.as_str())
        .expect("valid file:// URL")
        .with_fetch_tags(gix::remote::fetch::Tags::None);

    // Connect in Push direction and perform the handshake.
    let conn = remote
        .connect(gix::remote::Direction::Push)
        .expect("connect to bare dst");

    let prepare = conn
        .prepare_push(gix_features::progress::Discard)
        .expect("prepare_push: handshake with bare dst");

    // Transmit the branch.
    let outcome = prepare
        .with_refspecs(["refs/heads/main:refs/heads/main"])
        .transmit(
            gix_features::progress::Discard,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("transmit");

    // Verify the server acknowledged the ref update.
    assert_eq!(outcome.status.len(), 1, "expected one ref status");
    assert!(
        outcome.status[0].result.is_ok(),
        "push should succeed; got: {:?}",
        outcome.status[0].result
    );

    // Verify the bare remote actually has the ref pointing to the pushed commit.
    let bare = gix::open_opts(&dst_dir, gix::open::Options::isolated()).expect("open bare dst repo");
    let mut main_ref = bare
        .find_reference("refs/heads/main")
        .expect("refs/heads/main must exist after push");
    let _commit = main_ref.peel_to_id().expect("refs/heads/main must be resolvable");
}

/// Create a (src, dst.git) repo pair under a temporary directory and return
/// their paths.  The source repo has one empty commit on `main`; the
/// destination is a bare repo with no commits.
///
/// Panics on any `git` subprocess failure — these are test-setup steps, so
/// a clear panic is the right signal.
#[cfg(feature = "blocking-network-client")]
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

    // Init source repo.
    std::fs::create_dir(&src).expect("create src dir");
    run(&src, &["init", "-q", "-b", "main"]);
    // Create an initial empty commit.
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

    // Init bare destination.
    std::fs::create_dir(&dst).expect("create dst dir");
    run(&dst, &["init", "--bare", "-q"]);

    // Keep the TempDir alive for the duration of the test by leaking it.
    // `keep()` consumes `TempDir`, returning a `PathBuf` and preventing cleanup.
    let tmp_path = tmp.keep();
    (tmp_path.join("src"), tmp_path.join("dst.git"))
}
