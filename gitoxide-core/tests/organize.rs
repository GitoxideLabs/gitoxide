use gitoxide_core::organize::{Mode, run};
use gix_testtools::TestResult;

#[cfg(unix)]
use gix::error::MetadataValue;

#[test]
fn url_components_cannot_move_repositories_outside_the_destination() -> TestResult {
    for url in [
        "https://example.com/%2e%2e/%2e%2e/escaped",
        "https://../escaped",
        "ssh://example.com/../../escaped",
    ] {
        for mode in [Mode::Execute, Mode::Simulate] {
            let fixture = gix_testtools::scripted_fixture_writable("organize.sh")?;
            let source = fixture.path().join("source").canonicalize()?;
            let destination = fixture.path().join("destination");
            gix_testtools::git(&source, &format!("config remote.origin.url {url}"))?;

            let err = run(mode, &source, &destination, gix::progress::Discard, Some(1))
                .expect_err("organizing must reject a URL-derived path escape, even in simulation");
            assert!(
                err.is_validation(),
                "URL-derived destination escapes are validation failures: {url}"
            );
            let diagnostic = gix_testtools::redact_debug_snapshot(
                &err,
                &[(&fixture.path().canonicalize()?.to_string_lossy(), "<fixture>")],
            );
            insta::allow_duplicates! {
                insta::assert_debug_snapshot!(
                    diagnostic,
                    "execute and simulate preserve the confinement cause and resolved destination for every URL",
                    @r#"
                Failed to organize 1 repository

                Caused by:
                    0: Failed to organize repository, repository="<fixture>/source"
                    1: the origin URL resolves outside the repository destination, destination="<fixture>/escaped", destination_root="<fixture>/destination"
                "#
                );
            }
            assert_eq!(
                std::fs::read_to_string(source.join("payload"))?,
                "repository contents",
                "a rejected URL leaves the source contents unchanged: {url}"
            );
            assert!(
                !fixture.path().join("escaped").exists(),
                "no destination is created outside the requested root: {url}"
            );
            assert_eq!(
                std::fs::read_dir(&destination)?.count(),
                0,
                "a rejected URL creates no intermediate destination directories: {url}"
            );
        }
    }
    Ok(())
}

#[test]
fn ordinary_urls_still_move_repositories_below_the_destination() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_writable("organize.sh")?;
    let source = fixture.path().join("source").canonicalize()?;
    let destination = fixture.path().join("destination");
    run(Mode::Execute, &source, &destination, gix::progress::Discard, Some(1))?;

    let moved = destination.join("example.com/owner/repository");
    assert_eq!(
        std::fs::read_to_string(moved.join("payload"))?,
        "repository contents",
        "normal URL organization retains its layout and repository contents"
    );
    assert!(
        moved.join(".git/HEAD").is_file(),
        "organization moves Git metadata along with the worktree"
    );
    assert!(!source.exists(), "successful organization moves the repository");
    run(
        Mode::Execute,
        &destination,
        &destination,
        gix::progress::Discard,
        Some(1),
    )?;
    assert_eq!(
        std::fs::read_to_string(moved.join("payload"))?,
        "repository contents",
        "already organized repositories stay in place"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn dangling_destination_symlinks_are_skipped() -> TestResult {
    for mode in [Mode::Execute, Mode::Simulate] {
        let fixture = gix_testtools::scripted_fixture_writable("organize.sh")?;
        let source = fixture.path().join("source").canonicalize()?;
        let destination = fixture.path().join("destination");
        let occupied = destination.join("example.com/owner/repository");
        let missing = destination.join("missing");
        std::fs::create_dir_all(occupied.parent().expect("the destination has a parent"))?;
        std::os::unix::fs::symlink(&missing, &occupied)?;
        let root = gix::progress::tree::Root::new();
        run(mode, &source, &destination, root.add_child("organize"), Some(1))?;
        let mut messages = Vec::new();
        root.copy_messages(&mut messages);
        assert!(
            messages[0].message.starts_with("Skipping repository"),
            "dangling symlinks occupy the destination too"
        );
        assert_eq!(
            std::fs::read_link(&occupied)?,
            missing,
            "the existing symlink is preserved"
        );
        assert!(source.join(".git/HEAD").is_file(), "the source stays intact");
        assert!(!missing.exists(), "skipping cannot create the missing symlink target");
    }
    Ok(())
}

#[test]
fn occupied_destinations_are_skipped_without_changing_either_path() -> TestResult {
    check_occupied_destinations("https://example.com/owner/repository.git")
}

#[test]
fn normalized_occupied_destinations_are_skipped_without_changing_either_path() -> TestResult {
    check_occupied_destinations("https://example.com/missing/../owner/repository.git")
}

fn check_occupied_destinations(url: &str) -> TestResult {
    for mode in [Mode::Execute, Mode::Simulate] {
        for occupied_kind in ["directory", "empty directory", "file"] {
            let fixture = gix_testtools::scripted_fixture_writable("organize.sh")?;
            let source = fixture.path().join("source").canonicalize()?;
            let destination = fixture.path().join("destination");
            let occupied = destination.join("example.com/owner/repository");
            gix_testtools::git(&source, &format!("config remote.origin.url {url}"))?;
            std::fs::create_dir_all(occupied.parent().expect("the destination has a parent"))?;
            match occupied_kind {
                "directory" => {
                    std::fs::create_dir(&occupied)?;
                    std::fs::write(occupied.join("sentinel"), "existing contents")?;
                }
                "empty directory" => std::fs::create_dir(&occupied)?,
                _ => std::fs::write(&occupied, "existing contents")?,
            }
            let root = gix::progress::tree::Root::new();
            run(mode, &source, &destination, root.add_child("organize"), Some(1))?;
            let mut messages = Vec::new();
            root.copy_messages(&mut messages);
            assert_eq!(messages.len(), 1, "occupied destinations only emit a skip message");
            assert_eq!(
                messages[0].level,
                gix::progress::MessageLevel::Info,
                "a collision is an informational skip"
            );
            assert_eq!(
                messages[0].message,
                format!(
                    "Skipping repository \"{}\": destination \"{}\" already exists",
                    source.display(),
                    occupied.canonicalize()?.display()
                ),
                "the skip explains the collision and identifies both paths"
            );
            assert_eq!(
                std::fs::read_to_string(source.join("payload"))?,
                "repository contents",
                "skipping preserves the source"
            );
            assert!(
                source.join(".git/HEAD").is_file(),
                "skipping preserves source Git metadata"
            );
            if occupied_kind == "empty directory" {
                assert_eq!(
                    std::fs::read_dir(&occupied)?.count(),
                    0,
                    "even an empty destination is preserved"
                );
            } else {
                let sentinel = if occupied_kind == "directory" {
                    occupied.join("sentinel")
                } else {
                    occupied
                };
                assert_eq!(
                    std::fs::read_to_string(sentinel)?,
                    "existing contents",
                    "skipping preserves the destination"
                );
            }
        }
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn an_existing_symlink_cannot_redirect_the_destination() -> TestResult {
    for mode in [Mode::Execute, Mode::Simulate] {
        let fixture = gix_testtools::scripted_fixture_writable("organize.sh")?;
        let source = fixture.path().join("source").canonicalize()?;
        let destination = fixture.path().join("destination");
        let outside = fixture.path().join("outside");
        gix_testtools::git(&source, "config remote.origin.url https://example.com/repository.git")?;
        std::fs::create_dir(&outside)?;
        std::fs::write(outside.join("sentinel"), "outside contents")?;
        std::os::unix::fs::symlink(&outside, destination.join("example.com"))?;

        let err = run(mode, &source, &destination, gix::progress::Discard, Some(1))
            .expect_err("existing path components must stay inside the destination, even in simulation");
        assert!(err.is_validation(), "destination escapes are validation failures");
        let diagnostic = gix_testtools::redact_debug_snapshot(
            &err,
            &[(&fixture.path().canonicalize()?.to_string_lossy(), "<fixture>")],
        );
        insta::allow_duplicates! {
            insta::assert_debug_snapshot!(
                diagnostic,
                "execute and simulate report the symlink target outside the allowed destination root",
                @r#"
            Failed to organize 1 repository

            Caused by:
                0: Failed to organize repository, repository="<fixture>/source"
                1: the origin URL resolves outside the repository destination, destination="<fixture>/outside/repository", destination_root="<fixture>/destination"
            "#
            );
        }
        let metadata = err
            .metadata()
            .find(|metadata| metadata.contains_key("destination"))
            .expect("confinement errors include path metadata through the public API");
        assert_eq!(
            metadata["destination"],
            MetadataValue::Path(outside.canonicalize()?.join("repository")),
            "metadata identifies the resolved path outside the allowed root"
        );
        assert_eq!(
            metadata["destination_root"],
            MetadataValue::Path(destination.canonicalize()?),
            "metadata identifies the canonicalized allowed root"
        );
        assert_eq!(
            std::fs::read_to_string(source.join("payload"))?,
            "repository contents",
            "a rejected destination leaves the source contents unchanged"
        );
        assert!(
            !outside.join("repository").exists(),
            "a symlink cannot redirect the move"
        );
        assert_eq!(
            std::fs::read_to_string(outside.join("sentinel"))?,
            "outside contents",
            "confinement validation leaves pre-existing outside contents unchanged"
        );
        assert_eq!(
            std::fs::read_dir(&outside)?.count(),
            1,
            "a rejected destination creates nothing outside the allowed root"
        );
    }
    Ok(())
}

#[test]
fn nested_moves_still_work_and_leave_no_temporary_directory() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_writable("organize.sh")?;
    let source = fixture.path().join("source").canonicalize()?;
    gix_testtools::git(&source, "config remote.origin.url https://source/owner/repository.git")?;
    run(Mode::Execute, &source, fixture.path(), gix::progress::Discard, Some(1))?;
    assert_eq!(
        std::fs::read_to_string(source.join("owner/repository/payload"))?,
        "repository contents",
        "repositories can still move into their former directory"
    );
    assert_eq!(
        std::fs::read_dir(fixture.path())?.count(),
        2,
        "successful moves clean up the temporary staging directory"
    );
    Ok(())
}

#[test]
fn a_blocked_destination_parent_remains_an_error() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_writable("organize.sh")?;
    let source = fixture.path().join("source").canonicalize()?;
    let destination = fixture.path().join("destination");
    std::fs::write(destination.join("example.com"), "blocker")?;
    let err = run(Mode::Execute, &source, &destination, gix::progress::Discard, Some(1))
        .expect_err("a blocked parent is a failed operation, not an existing destination");
    let report = format!("{err:#?}");
    assert!(
        report.contains("destination="),
        "the failed operation identifies its destination: {report}"
    );
    assert!(
        !report.contains("handle"),
        "errors name the operation instead of generic handling: {report}"
    );
    assert!(
        err.downcast_any_ref::<std::io::Error>().is_some(),
        "the original filesystem error is retained"
    );
    assert_eq!(
        std::fs::read_to_string(source.join("payload"))?,
        "repository contents",
        "a blocked parent leaves the source intact"
    );
    Ok(())
}
