use gix_testtools::{Creation, Result};

const SCRIPT_NAME: &str = "make_basic.sh";

#[test]
fn scripted_fixture_read_only_with_post_returns_value() -> Result {
    let (dir, value) = gix_testtools::scripted_fixture_read_only_with_post(SCRIPT_NAME, 1, |fixture| {
        let dir = fixture.path();
        // The script should have already created these files
        assert!(dir.join("script_file.txt").exists());
        assert!(dir.join("subdir/nested.txt").exists());

        // Return a computed value
        Ok(std::fs::read_to_string(dir.join("script_file.txt"))?.len())
    })?;

    // Verify the fixture path is valid
    assert!(dir.is_dir());
    assert!(dir.join("script_file.txt").exists());

    // Verify the returned value (always available now since closure is always called)
    assert_eq!(value, "created by script\n".len());

    Ok(())
}

#[test]
fn scripted_fixture_writable_with_post_returns_value() -> Result {
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
fn scripted_fixture_with_post_can_return_complex_types() -> Result {
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
