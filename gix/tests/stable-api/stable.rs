use gix::error::ErrorExt;
use std::path::Path;

fn inspect(repo: &mut gix::Repository) -> gix::Result<()> {
    let _: &Path = repo.git_dir();
    let _: &Path = repo.common_dir();
    let _: Option<&Path> = repo.workdir();
    let _ = (repo.is_bare(), repo.is_shallow()?, repo.kind());
    let shared: gix::ThreadSafeRepository = repo.clone().into_sync();
    let _: gix::Repository = shared.to_thread_local();
    repo.object_cache_size(Some(1024));
    repo.reload()?;
    Ok(())
}

fn main() -> gix::Result<()> {
    #[cfg(feature = "parallel")]
    {
        fn send<T: Send>() {}
        fn send_sync<T: Send + Sync>() {}
        send::<gix::Repository>();
        send_sync::<gix::ThreadSafeRepository>();
    }
    let options = gix::open::Options::isolated().config_overrides(["core.bare=true"]);
    let _open: fn(std::path::PathBuf) -> gix::Result<gix::Repository> = gix::open;
    let _discover = |path: &Path| gix::discover(path);
    let _inspect = inspect;
    let _ = options;
    let error = gix::error::not_found("missing repository").raise();
    let _ = (error.classify(), error.metadata());
    Ok(())
}
