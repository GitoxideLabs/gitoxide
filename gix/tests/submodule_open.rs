#[test]
#[cfg(feature = "status")]
#[cfg_attr(
    windows,
    ignore = "for now, windows gets Os { code: 267, kind: NotADirectory, message: 'The directory name is invalid.' } (maybe because of relative symlink)?"
)]
fn on_nested_symlink() -> gix_testtools::Result {
    let symlink_root =
        gix_testtools::scripted_fixture_read_only("make_submodules.sh")?.join("link-to-dir-in-changed-parent-repo");
    // Note: even though this refers to a symlink, the CWD that is actually set will be the resolved directory.
    std::env::set_current_dir(&symlink_root)?;

    assert!(
        gix::open(&symlink_root).is_err(),
        "this is not a repository directly, it's a directory within one"
    );
    // TODO(symlink): This should work though
    // let repo = gix::discover(&repo_root)?;
    let repo = gix::discover(".")?;
    let sm = repo.submodules()?.into_iter().flatten().next().expect("one submodule");
    assert_eq!(
        sm.work_dir()?,
        p("../../m1"),
        "the workdir remains relative and is available"
    );
    let sm_repo = sm.open()?.expect("repo is present and accessible");
    assert_eq!(sm_repo.git_dir(), "../../.git/modules/m1");
    assert_eq!(
        sm_repo.workdir().expect("worktree present as we have one"),
        p("../../m1")
    );

    for item in repo.status(gix::progress::Discard)?.into_iter(None)? {
        assert!(
            item.is_ok(),
            "{item:?}: if there was no worktree, the changed submodule changed would fail the status computation"
        );
    }
    Ok(())
}

#[cfg(feature = "status")]
fn p(path: &str) -> &std::path::Path {
    std::path::Path::new(path)
}
