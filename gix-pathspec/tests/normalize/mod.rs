use std::path::Path;

#[test]
fn consuming_the_entire_prefix_does_not_lead_to_a_single_dot() -> crate::Result {
    let spec = normalized_spec("..", "a", "")?;
    assert_eq!(
        spec.path(),
        ".",
        "the top-level of the worktree can take the special value '.' to mean 'everything'"
    );
    assert!(
        spec.is_nil(),
        "single is for the worktree top-level, and since it wouldn't match anything we make it nil so it does"
    );
    assert_eq!(spec.prefix_directory(), "", "there is no prefix left");
    Ok(())
}

#[test]
fn removes_relative_path_components() -> crate::Result {
    for (input_path, expected_path, expected_prefix) in [
        ("..", "a", ""),
        ("c", "a/b/c", "a/b"),
        ("../c", "a/c", "a"),
        ("../b/c", "a/b/c", "a"), // this is a feature - prefix components once consumed by .. are lost. Important as paths can contain globs
        ("../*c/d", "a/*c/d", "a"),
        ("../../c/d", "c/d", ""),
        ("../../c/d/", "c/d", ""),
        ("./c", "a/b/c", "a/b"),
        ("../../c", "c", ""),
        ("../..", ".", ""),
        ("../././c", "a/c", "a"),
        ("././/./c", "a/b/c", "a/b"),
        ("././/./c/", "a/b/c", "a/b"),
        ("././/./../c/d/", "a/c/d", "a"),
    ] {
        let spec = normalized_spec(input_path, "a/b", "")?;
        assert_eq!(spec.path(), expected_path);
        assert_eq!(
            spec.prefix_directory(),
            expected_prefix,
            "{input_path} -> {expected_path}"
        );
    }
    Ok(())
}

#[test]
fn single_dot_is_special_and_directory_is_implied_without_trailing_slash() -> crate::Result {
    for (input_path, expected) in [(".", "."), ("./", ".")] {
        let spec = normalized_spec(input_path, "", "/repo")?;
        assert_eq!(spec.path(), expected);
        assert!(spec.is_nil(), "such a spec has to match everything");
        assert_eq!(spec.prefix_directory(), "");
    }
    Ok(())
}

#[test]
fn absolute_path_made_relative() -> crate::Result {
    for (input_path, expected, prefix_dir) in [
        ("/repo/a", "a", ""),
        ("/repo/a/..//.///b", "b", ""),
        ("/repo/a/", "a", "a"),
        ("/repo/*/", "*", "*"),
        ("/repo/a/b", "a/b", "a"),
        ("/repo/*/b", "*/b", "*"), // we assume literal paths if specs are absolute
        ("/repo/a/*/", "a/*", "a/*"),
        ("/repo/a/b/", "a/b", "a/b"),
        ("/repo/a/b/*", "a/b/*", "a/b"),
        ("/repo/a/b/c/..", "a/b", "a"),
    ] {
        let spec = normalized_spec(input_path, "", "/repo")?;
        assert_eq!(spec.path(), expected);
        assert_eq!(spec.prefix_directory(), prefix_dir, "{input_path}");
    }
    Ok(())
}

#[test]
fn relative_top_patterns_ignore_the_prefix() -> crate::Result {
    let spec = normalized_spec(":(top)c", "a/b", "")?;
    assert_eq!(spec.path(), "c");
    assert_eq!(spec.prefix_directory(), "");
    Ok(())
}

#[test]
fn absolute_top_patterns_ignore_the_prefix_but_are_made_relative() -> crate::Result {
    let spec = normalized_spec(":(top)/a/b", "prefix-ignored", "/a")?;
    assert_eq!(spec.path(), "b");
    assert_eq!(spec.prefix_directory(), "");
    Ok(())
}

#[test]
fn relative_path_breaks_out_of_working_tree() {
    let err = normalized_spec("../a", "", "").unwrap_err();
    assert_eq!(err.to_string(), "The path '../a' leaves the repository");
    let err = normalized_spec("../../b", "a", "").unwrap_err();
    assert_eq!(
        err.to_string(),
        format!(
            "The path '{}' leaves the repository",
            if cfg!(windows) { r"a\../../b" } else { "a/../../b" }
        )
    );
}

#[test]
fn absolute_path_breaks_out_of_working_tree() {
    let err = normalized_spec("/path/to/repo/..///./a", "", "/path/to/repo").unwrap_err();
    assert_eq!(err.to_string(), "The path '..///./a' leaves the repository");
    let err = normalized_spec("/path/to/repo/../../../dev", "", "/path/to/repo").unwrap_err();
    assert_eq!(err.to_string(), "The path '../../../dev' leaves the repository");
}

#[test]
fn absolute_path_escapes_worktree() {
    assert_eq!(
        normalized_spec("/dev", "", "/path/to/repo").unwrap_err().to_string(),
        "The path '/dev' is not inside of the worktree '/path/to/repo'"
    );
}

fn normalized_spec(
    path: &str,
    prefix: &str,
    root: &str,
) -> Result<gix_pathspec::Pattern, gix_pathspec::normalize::Error> {
    let mut spec = gix_pathspec::parse(path.as_bytes(), Default::default()).expect("valid");
    spec.normalize(Path::new(prefix), Path::new(root))?;
    Ok(spec)
}
