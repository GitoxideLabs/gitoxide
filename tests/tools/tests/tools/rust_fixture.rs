use gix_testtools::{Creation, FixtureState, Result};

#[test]
fn rust_fixture_read_only_creates_and_caches_fixture() -> Result {
    // First call should create the fixture
    let (dir, (a, b, c)) = gix_testtools::rust_fixture_read_only(
        "test_fixture_read_only",
        1,
        |fixture| {
            let dir = fixture.path();
            let a = dir.join("test_file.txt");
            let b = dir.join("subdir");
            let c = dir.join("subdir/nested.txt");
            if fixture.is_uninitialized() {
                std::fs::write(&a, "test content")?;
                std::fs::create_dir(&b)?;
                std::fs::write(&c, "nested content")?;
            }
            Ok((a, b, c))
        },
        gix_hash::Kind::Sha1,
    )?;

    // Verify the fixture was created correctly
    assert!(dir.is_dir());
    assert!(a.exists());
    assert_eq!(std::fs::read_to_string(&a)?, "test content");
    assert!(b.is_dir());
    assert!(c.exists());
    assert_eq!(std::fs::read_to_string(&c)?, "nested content");

    // Second call with same version should return cached result
    // The closure is still called but knows that it's fresh.
    let (dir2, _) = gix_testtools::rust_fixture_read_only(
        "test_fixture_read_only",
        1,
        |fixture| {
            assert!(
                matches!(fixture, gix_testtools::FixtureState::Fresh(_)),
                "Expected cached fixture on second call"
            );
            Ok(())
        },
        gix_hash::Kind::Sha1,
    )?;

    // Both should point to the same directory
    assert_eq!(dir, dir2);

    Ok(())
}

#[test]
fn rust_fixture_read_only_version_change_invalidates_cache() -> Result {
    // Create fixture with version 1
    let (dir1, _) = gix_testtools::rust_fixture_read_only(
        "test_fixture_version",
        1,
        |fixture| {
            if let FixtureState::Uninitialized(dir) = fixture {
                std::fs::write(dir.join("version.txt"), "v1")?;
            }
            Ok(())
        },
        gix_hash::Kind::Sha1,
    )?;

    // Version 2 should create a new fixture in a different directory
    let (dir2, _) = gix_testtools::rust_fixture_read_only(
        "test_fixture_version",
        2,
        |fixture| {
            if let FixtureState::Uninitialized(dir) = fixture {
                std::fs::write(dir.join("version.txt"), "v2")?;
            }
            Ok(())
        },
        gix_hash::Kind::Sha1,
    )?;

    assert_ne!(
        dir1, dir2,
        "Directories should be different (different version subdirectories)"
    );

    // Each should have its own content
    assert_eq!(std::fs::read_to_string(dir1.join("version.txt"))?, "v1");
    assert_eq!(std::fs::read_to_string(dir2.join("version.txt"))?, "v2");

    Ok(())
}

#[test]
fn rust_fixture_writable() -> Result {
    for creation in [Creation::CopyFromReadOnly, Creation::Execute] {
        let (tmp, _) = gix_testtools::rust_fixture_writable(
            "test_fixture_writable_copy",
            1,
            creation,
            |fixture| {
                if let FixtureState::Uninitialized(dir) = fixture {
                    std::fs::write(dir.join("original.txt"), "original content")?;
                }
                Ok(())
            },
            gix_hash::Kind::Sha1,
        )?;

        // Verify the fixture was created
        let original_path = tmp.path().join("original.txt");
        assert!(original_path.exists());
        assert_eq!(std::fs::read_to_string(&original_path)?, "original content");

        // Verify we can write to the directory (it's writable)
        let new_file = tmp.path().join("new_file.txt");
        std::fs::write(&new_file, "new content")?;
        assert!(new_file.exists());
        assert_eq!(std::fs::read_to_string(&new_file)?, "new content");
    }
    Ok(())
}

#[test]
fn rust_fixture_closure_error_propagates() {
    // Test that errors from the closure are properly propagated
    let res = gix_testtools::rust_fixture_read_only(
        "test_fixture_error",
        1,
        |_fixture| Err::<(), _>("intentional error".into()),
        gix_hash::Kind::Sha1,
    );

    let err_msg = res.unwrap_err().to_string();
    assert!(
        err_msg.contains("intentional error"),
        "Error message should contain the original error, got: {err_msg}"
    );
}

#[test]
fn rust_fixture_standalone_uses_fixtures_directory() -> Result {
    let (dir, _) = gix_testtools::rust_fixture_read_only_standalone(
        "test_fixture_standalone",
        1,
        |fixture| {
            if let FixtureState::Uninitialized(dir) = fixture {
                std::fs::write(dir.join("standalone.txt"), "standalone")?;
            }
            Ok(())
        },
        gix_hash::Kind::Sha1,
    )?;

    // Standalone fixtures are stored in fixtures/generated-do-not-edit, not tests/fixtures/...
    let dir_str = dir.to_string_lossy();
    assert!(
        dir_str.contains("fixtures") && dir_str.contains("generated-do-not-edit"),
        "Standalone fixture should be in fixtures/generated-do-not-edit directory, got: {dir_str}"
    );
    assert!(
        !dir_str.contains("tests/fixtures"),
        "Standalone fixture should NOT be in tests/fixtures directory, got: {dir_str}"
    );

    assert!(dir.join("standalone.txt").exists());
    Ok(())
}
