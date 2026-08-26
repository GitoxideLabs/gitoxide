use super::*;

#[test]
fn internal_win_path_lookup() -> gix_testtools::Result {
    let root = gix_testtools::scripted_fixture_read_only("win_path_lookup.sh")?;
    let mut paths: Vec<_> = std::fs::read_dir(&root)?
        .filter_map(Result::ok)
        .map(|e| e.path().to_str().expect("no illformed UTF8").to_owned())
        .collect();
    paths.sort();
    paths.insert(0, String::new());
    let joined_paths = std::env::join_paths(paths)?;

    assert_eq!(
        win_path_lookup("a/b".as_ref(), &joined_paths),
        None,
        "any path with separator is considered ready to use"
    );
    assert!(
        std::path::Path::new("Cargo.toml").is_file(),
        "the empty-PATH check needs a file in the current directory"
    );
    assert_eq!(
        win_path_lookup("Cargo.toml".as_ref(), std::ffi::OsStr::new("")),
        None,
        "empty PATH entries don't search the current directory"
    );
    assert_eq!(
        win_path_lookup("x".as_ref(), &joined_paths),
        Some(root.join("a").join("x.exe")),
        "exe will be preferred, and it searches left to right thus doesn't find c/x.exe"
    );
    assert_eq!(
        win_path_lookup("x.exe".as_ref(), &joined_paths),
        Some(root.join("a").join("x.exe")),
        "no matter what, a/x won't be found as it's shadowed by an exe file"
    );
    assert_eq!(
        win_path_lookup("exe.com".as_ref(), &joined_paths),
        Some(root.join("b").join("exe.com")),
        "an explicitly requested suffix is preserved"
    );
    assert_eq!(
        win_path_lookup("exe".as_ref(), &joined_paths),
        Some(root.join("b").join("exe")),
        "it finds files further down the path as well"
    );
    Ok(())
}
