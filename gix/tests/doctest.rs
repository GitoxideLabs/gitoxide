pub fn basic_repo_dir() -> Result<std::path::PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")
}

pub fn open_repo(path: impl Into<std::path::PathBuf>) -> Result<gix::Repository, gix_error::Error> {
    gix::open_opts(path, gix::open::Options::isolated())
}

pub fn discover_repo(path: impl AsRef<std::path::Path>) -> Result<gix::Repository, gix_error::Error> {
    let opts = gix::open::Options::isolated();
    gix::ThreadSafeRepository::discover_opts(
        path,
        Default::default(),
        gix::sec::trust::Mapping {
            full: opts.clone(),
            reduced: opts,
        },
    )
    .map(Into::into)
}

pub fn basic_subrepo_dir(name: &str) -> Result<std::path::PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    Ok(basic_repo_dir()?.join(name))
}

pub fn remote_repo_dir(name: &str) -> Result<std::path::PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    Ok(gix_testtools::scripted_fixture_read_only("make_remote_repos.sh")?.join(name))
}

pub fn worktree_repo_dir() -> Result<std::path::PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    Ok(gix_testtools::scripted_fixture_read_only("make_worktree_repo.sh")?.join("repo"))
}

pub fn tempdir() -> std::io::Result<gix_testtools::tempfile::TempDir> {
    gix_testtools::tempfile::TempDir::new()
}
