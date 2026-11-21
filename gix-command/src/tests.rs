use super::*;

#[test]
fn internal_win_path_lookup() -> gix_testtools::TestResult {
    let root = gix_testtools::scripted_fixture_read_only("win_path_lookup.sh")?;
    let mut paths: Vec<_> = std::fs::read_dir(&root)?
        .filter_map(std::result::Result::ok)
        .map(|e| e.path().to_str().expect("no illformed UTF8").to_owned())
        .collect();
    paths.sort();
    paths.insert(0, String::new());
    let joined_paths = std::env::join_paths(paths)?;

    assert_eq!(
        win_path_lookup("a/b".as_ref(), &joined_paths, None)?,
        None,
        "any path with separator is considered ready to use"
    );
    for command in ["x/", r"x\", ".", ".."] {
        assert_eq!(
            win_path_lookup(command.as_ref(), &joined_paths, None)?,
            None,
            "a trailing separator makes the command an explicit path"
        );
    }
    assert!(
        std::path::Path::new("Cargo.toml").is_file(),
        "the empty-PATH check needs a file in the current directory"
    );
    assert_eq!(
        win_path_lookup("Cargo.toml".as_ref(), std::ffi::OsStr::new(""), None)?,
        None,
        "empty PATH entries don't search the current directory"
    );
    assert_eq!(
        win_path_lookup("x".as_ref(), &joined_paths, None)?,
        Some(root.join("a").join("x.exe")),
        "exe will be preferred, and it searches left to right thus doesn't find c/x.exe"
    );
    assert_eq!(
        win_path_lookup("x.exe".as_ref(), &joined_paths, None)?,
        Some(root.join("a").join("x.exe")),
        "no matter what, a/x won't be found as it's shadowed by an exe file"
    );
    assert_eq!(
        win_path_lookup("exe.com".as_ref(), &joined_paths, None)?,
        Some(root.join("b").join("exe.com")),
        "an explicitly requested suffix is preserved"
    );
    assert_eq!(
        win_path_lookup("exe".as_ref(), &joined_paths, None)?,
        Some(root.join("b").join("exe")),
        "it finds files further down the path as well"
    );
    Ok(())
}

#[cfg(windows)]
#[test]
fn drive_relative_paths_use_the_childs_drive_directory() -> gix_testtools::TestResult {
    for current_dir in [r"C:\child", r"\\?\C:\child"] {
        let current_dir = Path::new(current_dir);
        for path in [r"C:hook", r"c:hook"] {
            assert_eq!(
                anchor_path(Path::new(path), Some(current_dir))?,
                current_dir.join("hook"),
                "same-drive paths use the child directory, including verbatim roots"
            );
        }
        assert_eq!(
            anchor_path(Path::new(r"C:hook\"), Some(current_dir))?,
            current_dir.join("hook").join(""),
            "anchoring preserves a trailing separator that makes a program invalid"
        );
        assert_eq!(
            anchor_path(Path::new("D:hook"), Some(current_dir))?,
            std::path::absolute("D:hook")?,
            "a different drive is anchored to its own OS directory during preparation"
        );
    }
    assert!(!is_bare_command(Path::new("C:")), "a drive alone is an explicit path");
    Ok(())
}
