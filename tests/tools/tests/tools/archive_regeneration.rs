use gix_testtools::{FixtureState, TestResult};

#[test]
fn tracked_ignored_archives_are_regenerated() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    regenerate("sha1", false)
}

#[test]
fn tracked_archives_use_the_source_repository_hash() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    regenerate("sha256", false)
}

#[test]
fn tracked_archives_use_the_linked_worktree_index() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    regenerate("sha1", true)
}

/// Build the simulated source checkout directly instead of using fixture scripts: the fixture
/// helpers can initialize the process-global exclusion cache before we switch into this checkout,
/// causing the test to inspect the wrong repository. Direct setup also keeps this fixture-helper
/// regression test independent of the machinery it exercises and gives linked worktrees valid
/// paths without repairing links after copying or extracting a pre-generated fixture.
fn regenerate(object_format: &str, linked: bool) -> TestResult {
    let temp = gix_testtools::tempfile::tempdir()?;
    let main = temp.path().join("main");
    std::fs::create_dir(&main)?;
    gix_testtools::git(&main, &format!("init --object-format={object_format}"))?;
    let worktree = if linked {
        gix_testtools::git(&main, "commit --allow-empty -m initial")?;
        gix_testtools::git(&main, "worktree add -b linked ../linked")?;
        temp.path().join("linked")
    } else {
        main
    };
    std::fs::write(worktree.join(".gitignore"), b"**/generated-archives/\n")?;
    let crate_root = worktree.join("crate");
    let archive_dir = crate_root.join("tests/fixtures/generated-archives");
    std::fs::create_dir_all(&archive_dir)?;
    let extension = if cfg!(feature = "xz") { "tar.xz" } else { "tar" };
    let tracked_archive = archive_dir.join(format!("rust-tracked.{extension}"));
    std::fs::write(&tracked_archive, b"old archive")?;
    gix_testtools::git(
        &worktree,
        &format!("add -f crate/tests/fixtures/generated-archives/rust-tracked.{extension}"),
    )?;
    std::fs::remove_file(&tracked_archive)?;
    let _cwd = gix_testtools::set_current_dir(&crate_root)?;
    let _environment = gix_testtools::Env::new()
        .set("GIX_TEST_IGNORE_ARCHIVES", "1")
        .set("GIX_TEST_CREATE_ARCHIVES_EVEN_ON_CI", "1");
    for name in ["tracked", "untracked"] {
        let (fixture, ()) = gix_testtools::rust_fixture_read_only(name, 1, |state| {
            if let FixtureState::Uninitialized(path) = state {
                std::fs::write(path.join("payload"), b"generated")?;
            }
            Ok(())
        })?;
        assert_eq!(
            std::fs::read(fixture.join("payload"))?,
            b"generated",
            "missing fixture caches are generated even when archive writing is excluded"
        );
    }
    assert!(
        tracked_archive.is_file(),
        "a tracked archive is recreated despite inherited ignore rules"
    );
    assert!(
        !archive_dir.join(format!("rust-untracked.{extension}")).exists(),
        "untracked archives still obey inherited ignore rules"
    );
    Ok(())
}
