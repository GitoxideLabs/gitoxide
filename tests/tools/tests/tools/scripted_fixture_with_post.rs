use gix_testtools::{Creation, TestResult};

const SCRIPT_NAME: &str = "make_basic.sh";

#[cfg(not(windows))]
#[test]
fn post_processed_archives_do_not_collide() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let source = gix_testtools::tempfile::tempdir()?;
    let _cwd = gix_testtools::set_current_dir(source.path())?;
    let _environment = gix_testtools::Env::new()
        .unset("GIX_TEST_IGNORE_ARCHIVES")
        .unset("GIX_TEST_FIXTURE_CACHE")
        .unset("GIX_TEST_FIXTURE_SOURCE_ROOT")
        .set("GIX_TEST_CREATE_ARCHIVES_EVEN_ON_CI", "1")
        .set(
            "GIX_TESTTOOLS_ARCHIVE_RUNS",
            source.path().join("runs").display().to_string(),
        );
    std::fs::create_dir_all("tests/fixtures")?;
    for (script, single_archive) in [("separate.sh", false), ("single.sh", true)] {
        // Record executions outside the cached payload to distinguish extraction from regeneration.
        std::fs::write(
            gix_testtools::fixture_path(script),
            "printf plain > payload
printf run >> \"$GIX_TESTTOOLS_ARCHIVE_RUNS\"
",
        )?;
        let mut generated_paths = Vec::new();
        let mut generated_runs = Vec::new();
        for from_archive in [false, true] {
            for version in [None, Some(0), Some(1)] {
                let expected = version.map_or_else(|| "plain".to_owned(), |version| format!("post-v{version}"));
                let post = |state: gix_testtools::FixtureState<'_>| -> gix_testtools::PostResult {
                    assert_eq!(
                        state.is_uninitialized(),
                        !from_archive,
                        "each variant is generated once, then its post-processed archive is loaded as Fresh"
                    );
                    if let gix_testtools::FixtureState::Uninitialized(path) = state {
                        std::fs::write(path.join("payload"), &expected)?;
                    }
                    Ok(())
                };
                let path = match (version, single_archive) {
                    (None, false) => gix_testtools::scripted_fixture_read_only_with_args(script, ["argument"])?,
                    (None, true) => {
                        gix_testtools::scripted_fixture_read_only_with_args_single_archive(script, ["argument"])?
                    }
                    (Some(version), false) => {
                        gix_testtools::scripted_fixture_read_only_with_args_with_post(
                            script,
                            ["argument"],
                            version,
                            post,
                        )?
                        .0
                    }
                    (Some(version), true) => {
                        gix_testtools::scripted_fixture_read_only_with_args_single_archive_with_post(
                            script,
                            ["argument"],
                            version,
                            post,
                        )?
                        .0
                    }
                };
                assert_eq!(
                    std::fs::read_to_string(path.join("payload"))?,
                    expected,
                    "plain fixtures and each post-processing version retain their own payload"
                );
                if !from_archive {
                    assert!(
                        !generated_paths.contains(&path),
                        "each variant has a distinct generated directory"
                    );
                    generated_paths.push(path);
                }
            }
            if from_archive {
                assert_eq!(
                    std::fs::read("runs")?,
                    generated_runs,
                    "all variants reload from their own archives without rerunning the script"
                );
            } else {
                generated_runs = std::fs::read("runs")?;
                for path in &generated_paths {
                    std::fs::remove_dir_all(path)?;
                }
            }
        }
    }
    Ok(())
}

#[test]
fn scripted_fixture_read_only_with_post_returns_value() -> TestResult {
    let (dir, value) = gix_testtools::scripted_fixture_read_only_with_post(SCRIPT_NAME, 1, |fixture| {
        let dir = fixture.path();
        assert!(dir.is_absolute(), "post-processing receives an absolute fixture path");
        // The script should have already created these files
        assert!(dir.join("script_file.txt").exists());
        assert!(dir.join("subdir/nested.txt").exists());

        // Return a computed value
        Ok(std::fs::read_to_string(dir.join("script_file.txt"))?.len())
    })?;

    // Verify the fixture path is valid
    assert!(dir.is_absolute(), "read-only fixtures return absolute paths");
    assert!(dir.is_dir());
    assert!(dir.join("script_file.txt").exists());

    // Verify the returned value (always available now since closure is always called)
    assert_eq!(value, "created by script\n".len());

    Ok(())
}

#[test]
fn scripted_fixture_writable_with_post_returns_value() -> TestResult {
    let (tmp, value) = gix_testtools::scripted_fixture_writable_with_args_with_post(
        SCRIPT_NAME,
        None::<String>,
        Creation::Execute,
        1,
        |fixture| {
            // Compute something from the fixture
            Ok(std::fs::read_dir(fixture.path())?
                .filter_map(std::result::Result::ok)
                .count())
        },
    )?;

    // Verify the fixture is writable
    std::fs::write(tmp.path().join("new_file.txt"), "test")?;
    assert!(tmp.path().join("new_file.txt").exists());

    // Verify the returned value (should have script_file.txt and subdir)
    assert_eq!(value, 2);

    Ok(())
}

#[test]
fn scripted_fixture_with_post_can_return_complex_types() -> TestResult {
    #[derive(Debug, PartialEq)]
    struct FixtureInfo {
        file_count: usize,
        has_subdir: bool,
    }

    // Use version 2 to force recreation (different from other tests using this script)
    let (dir, info) = gix_testtools::scripted_fixture_read_only_with_post(SCRIPT_NAME, 2, |fixture| {
        let dir = fixture.path();
        Ok(FixtureInfo {
            file_count: std::fs::read_dir(dir)?.count(),
            has_subdir: dir.join("subdir").is_dir(),
        })
    })?;

    assert!(dir.is_dir());
    assert_eq!(
        info,
        FixtureInfo {
            file_count: 2,
            has_subdir: true
        },
        "info is always available since closure is always called"
    );

    Ok(())
}

#[test]
fn version_compatible_writable_fixtures_support_both_creation_modes() -> TestResult {
    let copied = gix_testtools::scripted_fixture_writable_with_args_with_git_version(
        SCRIPT_NAME,
        ["version-compatible-copy"],
        Creation::CopyFromReadOnly,
        |_| true,
    )?
    .expect("compatible Git allows fixture generation");
    assert!(copied.path().join("script_file.txt").is_file());
    std::fs::write(copied.path().join("writable"), b"yes")?;

    let executed = gix_testtools::scripted_fixture_writable_with_args_with_git_version(
        SCRIPT_NAME,
        ["version-compatible-execute"],
        Creation::Execute,
        |_| true,
    )?
    .expect("compatible Git allows fixture generation");
    assert!(executed.path().join("script_file.txt").is_file());
    std::fs::write(executed.path().join("writable"), b"yes")?;

    Ok(())
}

#[test]
fn version_incompatible_writable_fixtures_never_run_the_script() -> TestResult {
    assert!(
        gix_testtools::scripted_fixture_writable_with_args_with_git_version(
            SCRIPT_NAME,
            ["version-incompatible-copy-without-archive"],
            Creation::CopyFromReadOnly,
            |_| false,
        )?
        .is_none(),
        "copy mode reports an unavailable required archive"
    );
    assert!(
        gix_testtools::scripted_fixture_writable_with_args_with_git_version(
            SCRIPT_NAME,
            ["version-incompatible-execute-without-archive"],
            Creation::Execute,
            |_| false,
        )?
        .is_none(),
        "execute mode reports an unavailable required archive instead of running the script"
    );

    let mut post_process_was_called = false;
    let fixture = gix_testtools::scripted_fixture_writable_with_args_with_post_with_git_version(
        SCRIPT_NAME,
        ["version-incompatible-post-process-without-archive"],
        Creation::Execute,
        1,
        |_| {
            post_process_was_called = true;
            Ok(())
        },
        |_| false,
    )?;
    assert!(
        fixture.is_none(),
        "post-processing also reports the unavailable fixture"
    );
    assert!(
        !post_process_was_called,
        "post-processing is not invoked without a compatible fixture"
    );

    Ok(())
}
