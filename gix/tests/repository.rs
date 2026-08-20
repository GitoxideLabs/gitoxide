use gix::bstr::ByteSlice;
use serial_test::serial;

#[test]
#[serial]
fn relative_paths_use_the_cwd_captured_when_opening() -> gix_testtools::Result {
    let root = gix::path::realpath(gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")?)?;
    let nested = root.join("some/very");

    let _cwd = gix_testtools::set_current_dir(&nested)?;
    let repo = gix::discover_opts(".", Default::default(), gix::open::Options::isolated())?;
    assert_eq!(
        repo.normalize_path("file")?.as_bstr(),
        "some/very/file",
        "relative paths start at the captured CWD"
    );
    assert_eq!(
        repo.normalize_path("../file")?.as_bstr(),
        "some/file",
        "parent components consume the captured CWD prefix"
    );
    assert_eq!(
        repo.normalize_path("../../file")?.as_bstr(),
        "file",
        "all CWD components can be consumed"
    );
    assert_eq!(
        repo.normalize_path("./file")?.as_bstr(),
        "some/very/file",
        "current-directory components are removed"
    );

    std::env::set_current_dir(&root)?;
    assert_eq!(
        repo.normalize_path("file")?.as_bstr(),
        "some/very/file",
        "Repository keeps the CWD captured when it was opened"
    );
    let repo = gix::discover_opts(".", Default::default(), gix::open::Options::isolated())?;
    assert!(
        matches!(repo.normalize_path("file")?, std::borrow::Cow::Borrowed(path) if path == "file"),
        "unchanged paths are returned without allocation"
    );
    assert_eq!(
        repo.normalize_path("")?.as_bstr(),
        "",
        "an empty path at the repository root remains empty"
    );
    assert_eq!(
        repo.normalize_path(".")?.as_bstr(),
        "",
        "the repository root expressed as a current-directory component normalizes to an empty path"
    );
    Ok(())
}

#[test]
#[serial]
fn paths_cannot_leave_the_repository() -> gix_testtools::Result {
    let root = gix::path::realpath(gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")?)?;
    let nested = root.join("some");

    let _cwd = gix_testtools::set_current_dir(&nested)?;
    let repo = gix::discover_opts(".", Default::default(), gix::open::Options::isolated())?;
    let absolute = gix::path::into_bstr(root.join("some-with-file/very/deeply/nested/subdir/empty-file"));
    assert_eq!(
        repo.normalize_path(&absolute)?.as_bstr(),
        "some-with-file/very/deeply/nested/subdir/empty-file",
        "absolute paths inside the worktree become repository-relative"
    );
    let err = repo
        .normalize_path("../../outside")
        .expect_err("relative paths cannot traverse above the worktree");
    assert!(err.is_validation(), "leaving the worktree is a validation error");
    assert_eq!(
        err.probable_cause().to_string(),
        format!(
            "The path '{}' leaves the repository",
            std::path::Path::new("some").join("../../outside").display()
        )
    );

    assert_eq!(
        repo.normalize_path("")?.as_bstr(),
        "some",
        "an empty path refers to the captured current directory"
    );
    Ok(())
}

#[test]
#[serial]
fn absolute_paths_outside_the_repository_are_rejected() -> gix_testtools::Result {
    let root = gix::path::realpath(gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")?)?;
    let repo = gix::discover_opts(&root, Default::default(), gix::open::Options::isolated())?;
    let outside = root.parent().expect("fixture has a parent").to_owned();
    let outside_as_bstr = gix::path::into_bstr(outside.clone());

    let err = repo
        .normalize_path(&outside_as_bstr)
        .expect_err("an absolute path outside the repository must fail");
    assert!(err.is_validation(), "an outside path is a validation error");
    assert_eq!(
        err.probable_cause().to_string(),
        format!(
            "The absolute path '{}' is not inside the repository at '{}'",
            outside.display(),
            root.display()
        )
    );
    Ok(())
}

#[test]
#[cfg(feature = "status")]
#[serial]
fn is_dirty_sees_index_changes_outside_the_current_working_directory() -> gix_testtools::Result {
    let root = gix::path::realpath(
        gix_testtools::scripted_fixture_read_only("make_status_repos.sh")?.join("index-changed-outside-subdir"),
    )?;

    let _cwd = gix_testtools::set_current_dir(root.join("subdir"))?;
    let repo = gix::discover_opts(".", Default::default(), gix::open::Options::isolated())?;
    assert!(
        repo.is_dirty()?,
        "the index comparison isn't limited to the current working directory"
    );
    Ok(())
}

#[test]
#[cfg(feature = "revision")]
#[serial]
fn revspec_paths_starting_with_a_dot_are_relative_to_the_current_directory() -> gix_testtools::Result {
    let root = gix::path::realpath(gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")?)?;
    let nested = root.join("some/very");

    let _cwd = gix_testtools::set_current_dir(&nested)?;
    let repo = gix::discover_opts(".", Default::default(), gix::open::Options::isolated())?;
    let blob = repo.rev_parse_single("HEAD:this")?.detach();
    let tree = repo.rev_parse_single("HEAD^{tree}")?.detach();
    for spec in [
        "HEAD:../../this",
        "HEAD:./.././../this",
        ":../../this",
        ":..//.././/this",
        ":0:../../this",
    ] {
        assert_eq!(
            repo.rev_parse_single(spec)?.detach(),
            blob,
            "`{spec}` consumes the CWD prefix to name the blob at the worktree root"
        );
    }
    for spec in ["HEAD:../../", "HEAD:../..", "HEAD:./..//.."] {
        assert_eq!(
            repo.rev_parse_single(spec)?.detach(),
            tree,
            "`{spec}` names the tree the CWD components resolve to"
        );
    }

    std::env::set_current_dir(&root)?;
    let repo = gix::discover_opts(".", Default::default(), gix::open::Options::isolated())?;
    for spec in ["HEAD:./this", "HEAD:././this", ":./this", ":0:./this"] {
        assert_eq!(
            repo.rev_parse_single(spec)?.detach(),
            blob,
            "`{spec}` looks the path up relative to the CWD, even if it's root"
        );
    }
    assert_eq!(
        repo.rev_parse_single("HEAD:./")?.detach(),
        tree,
        "`HEAD:./` names the tree of the CWD itself"
    );
    Ok(())
}

#[test]
#[cfg(feature = "revision")]
#[serial]
fn revspec_paths_starting_with_a_dot_need_a_worktree_to_stay_within() -> gix_testtools::Result {
    let root = gix::path::realpath(gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")?)?;

    let _cwd = gix_testtools::set_current_dir(&root)?;
    let repo = gix::discover_opts(".", Default::default(), gix::open::Options::isolated())?;
    for spec in [
        "HEAD:../this",
        ":../this",
        "HEAD:./../this",
        ":./../this",
        ":0:./../this",
        "HEAD:././../",
    ] {
        assert!(
            probable_cause(repo.rev_parse_single(spec)).contains("leaves the repository"),
            "`{spec}` traverses above the worktree and must not resolve"
        );
    }

    let repo = gix::open_opts(root.join("non-bare-without-worktree"), gix::open::Options::isolated())?;
    assert!(
        repo.rev_parse_single("HEAD:this").is_ok(),
        "`HEAD:this` is repository-relative and resolves without a worktree"
    );
    for spec in ["HEAD:./this", ":./this"] {
        assert!(
            probable_cause(repo.rev_parse_single(spec)).contains("can't be used outside of a worktree"),
            "`{spec}` has nothing to be relative to and must not resolve"
        );
    }
    Ok(())
}

#[cfg(feature = "revision")]
fn probable_cause(res: Result<gix::Id<'_>, gix::revision::spec::parse::single::Error>) -> String {
    res.expect_err("the revspec must not resolve")
        .probable_cause()
        .to_string()
}
