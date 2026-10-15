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
                Failed to handle 1 repository

                Caused by:
                    0: Failed to handle repository, repository="<fixture>/source"
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
    Ok(())
}

#[test]
fn occupied_destinations_retain_each_repository_and_its_failure() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_writable("organize.sh")?;
    let source = fixture.path().join("source").canonicalize()?;
    let destination = fixture.path().join("destination");
    for name in ["first", "second", "third"] {
        let repository = source.join(name);
        std::fs::create_dir(&repository)?;
        gix_testtools::git(&repository, "init -q")?;
        gix_testtools::git(
            &repository,
            &format!("config remote.origin.url https://example.com/{name}.git"),
        )?;
        std::fs::write(repository.join("payload"), name)?;
        let occupied = destination.join("example.com").join(name);
        std::fs::create_dir_all(&occupied)?;
        std::fs::write(occupied.join("sentinel"), "existing contents")?;
    }
    // The original fixture repository must not hide the three repositories nested below it.
    std::fs::remove_dir_all(source.join(".git"))?;

    let err = run(Mode::Execute, &source, &destination, gix::progress::Discard, Some(1))
        .expect_err("three occupied destinations produce three independent failures");
    assert_eq!(
        err.error().to_string(),
        "Failed to handle 3 repositories",
        "the aggregate counts failed repositories"
    );
    let repositories = err
        .metadata()
        .filter_map(|metadata| metadata.get("repository"))
        .collect::<Vec<_>>();
    assert_eq!(repositories.len(), 3, "each repository's path is retained in metadata");
    for name in ["first", "second", "third"] {
        assert!(
            repositories.contains(&&gix::error::MetadataValue::Path(source.join(name))),
            "the report identifies {name}"
        );
        assert_eq!(
            std::fs::read_to_string(source.join(name).join("payload"))?,
            name,
            "failed moves preserve source contents"
        );
        assert_eq!(
            std::fs::read_to_string(destination.join("example.com").join(name).join("sentinel"))?,
            "existing contents",
            "failed moves preserve destination contents"
        );
    }
    let io_errors = err.iter_errors().filter(|error| error.is::<std::io::Error>()).count();
    assert_eq!(io_errors, 3, "every repository retains its original I/O failure");
    let diagnostic = format!(
        "{:?}",
        gix_testtools::redact_debug_snapshot(
            &err,
            &[(&fixture.path().canonicalize()?.to_string_lossy(), "<fixture>")]
        )
    );
    if diagnostic.contains("Caused by (flattened):") {
        for name in ["first", "second", "third"] {
            assert!(
                diagnostic.contains(&format!("repository=\"<fixture>/source/{name}\"")),
                "flattening retains each repository path"
            );
        }
        return Ok(());
    }
    // OS error wording differs, so snapshot only context and hierarchy while checking all I/O causes above.
    let diagnostic = diagnostic
        .lines()
        .map(|line| {
            if line.contains("repository=\"")
                || line.starts_with("Failed")
                || line.starts_with("Caused")
                || line.is_empty()
            {
                line.to_owned()
            } else if let Some((prefix, _)) = line.split_once("─ ") {
                format!("{prefix}─ <I/O failure>")
            } else {
                format!(
                    "{}: <I/O failure>",
                    line.split_once(": ").expect("cause lines are numbered").0
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!(diagnostic, "the aggregate retains repository paths and independent I/O causes", @r#"
    Failed to handle 3 repositories

    Caused by:
        ├─0: Failed to handle repository, repository="<fixture>/source/first"
        │ └─ <I/O failure>
        ├─1: Failed to handle repository, repository="<fixture>/source/second"
        │ └─ <I/O failure>
        └─2: Failed to handle repository, repository="<fixture>/source/third"
          └─ <I/O failure>
    "#);
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
            Failed to handle 1 repository

            Caused by:
                0: Failed to handle repository, repository="<fixture>/source"
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
