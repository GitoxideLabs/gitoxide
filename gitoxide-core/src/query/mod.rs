pub struct Engine {
    repo: gix::Repository,
    con: rusqlite::Connection,
    commits: Vec<gix::ObjectId>,
}

pub struct Options {
    pub object_cache_size_mb: usize,
    pub find_copies_harder: bool,
    pub threads: Option<usize>,
}

mod db;

mod engine;
pub use engine::Command;

pub fn prepare(
    repo_dir: &std::path::Path,
    mut progress: impl gix::NestedProgress,
    err: impl std::io::Write,
    opts: Options,
) -> anyhow::Result<Engine> {
    let repo = gix::discover(repo_dir)?;
    let mut con = db::create(
        repo.git_dir().join("ein.query"),
        repo.refs.shared_repository_permissions,
    )?;
    let commits = engine::update(&repo, &mut con, &mut progress, err, opts)?;
    Ok(Engine { repo, con, commits })
}
