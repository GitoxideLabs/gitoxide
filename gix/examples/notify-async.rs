//! `cargo run -p gix --features async-notify --example notify-async -- /path/to/repository`
//! Register coverage, print the initial invalidations, then wait for and print the next update.
use std::time::Instant;

#[tokio::main(flavor = "current_thread")]
async fn main() -> gix::Result<()> {
    let path = std::env::args_os().nth(1).unwrap_or_else(|| ".".into());
    let repo = gix::discover(path)?;
    let git_dir = repo.git_dir().to_owned();
    let mut monitor = repo.monitor(gix::notify::Options::default())?;
    drop(repo);
    for pass in 0..2 {
        if pass != 0 {
            monitor.ready_async().await;
        }
        let path = git_dir.clone();
        let (next, outcome) = monitor.service_async(Instant::now(), move || gix::open(&path)).await?;
        monitor = next;
        println!(
            "Changes: {:#?}\nOther worktrees: {:#?}\nStatistics: {:#?}",
            outcome.changes, outcome.other_worktrees, outcome.statistics
        );
        for error in outcome.errors {
            eprintln!("{error:?}");
        }
    }
    monitor.shutdown_async().await
}
