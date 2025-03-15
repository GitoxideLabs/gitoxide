// Clone a repository from any URL or Path to a given target directory

use anyhow::Context;

fn main() -> anyhow::Result<()> {
    let repo_url = std::env::args_os()
        .nth(1)
        .context("The first argument is the repository URL")?;

    let dst = std::env::args_os()
        .nth(2)
        .context("The second argument is the directory to clone the repository into")?;

    // SAFETY: The closure doesn't use mutexes or memory allocation, so it should be safe to call from a signal handler.
    unsafe {
        gix::interrupt::init_handler(1, || {})?;
    }
    std::fs::create_dir_all(&dst)?;
    let url = gix::url::parse(repo_url.to_str().unwrap().into())?;

    println!("Url: {:?}", url.to_bstring());
    let mut prepare_clone = gix::prepare_clone(url, &dst)?;

    println!("Cloning {repo_url:?} into {dst:?}...");
    let (mut prepare_checkout, _) =
        prepare_clone.fetch_then_checkout(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)?;

    println!(
        "Checking out into {:?} ...",
        prepare_checkout.repo().workdir().expect("should be there")
    );

    let (repo, _) = prepare_checkout.main_worktree(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)?;
    println!("Repo cloned into {:?}", repo.workdir().expect("directory pre-created"));

    let remote = repo
        .find_default_remote(gix::remote::Direction::Fetch)
        .expect("always present after clone")?;

    println!(
        "Default remote: {} -> {}",
        remote.name().expect("default remote is always named").as_bstr(),
        remote
            .url(gix::remote::Direction::Fetch)
            .expect("should be the remote URL")
            .to_bstring(),
    );

    Ok(())
}
