//! Compare stock and cached breadth-first revision walks on a local repository.
//!
//! ```sh
//! cargo run --release --no-default-features --features small --example rev-walk-bench -- \
//!   -C /path/to/repository --tip HEAD --hide 'HEAD~1000' --tips-per-start 64
//! ```
//!
//! Each batch walks the starting tip and consecutive first parents as separate walks, deduplicating
//! overlapping tips and applying the same exclusions. Choose a more distant `--hide` for deeper
//! walks, or repeat `--tip` with `--tips-per-start 1` to benchmark exactly the supplied tips.
//! Shallow boundaries and active replacement refs bypass the revision-walk cache.
//!
//! The example checks ordered commit IDs and warms both repository handles before timing. The object
//! caches stay warm across batches. Each cached batch starts with a fresh revision-walk cache, so the
//! timing includes building it and opening the optional commit-graph. The benchmark only reads the
//! repository.

use std::{
    collections::HashSet,
    ffi::{OsStr, OsString},
    hint::black_box,
    num::NonZeroUsize,
    path::PathBuf,
    time::{Duration, Instant},
};

use clap::Parser;
use gix::{
    ObjectId, Repository, Result,
    error::{OptionExt, ResultExt, bail, message},
    revision::walk::Platform,
};

#[derive(Parser)]
#[command(about = "Benchmark hidden revision walks with and without a shared metadata cache")]
struct Args {
    /// Repository to discover, or a directory inside it.
    #[arg(short = 'C', long, default_value = ".")]
    repository: PathBuf,
    /// Starting commit, branch, or tag. May be repeated.
    #[arg(long, default_value = "HEAD", value_name = "REV")]
    tip: Vec<OsString>,
    /// Exclude this commit and its ancestors from every walk. May be repeated.
    #[arg(long, required = true, value_name = "REV")]
    hide: Vec<OsString>,
    /// Number of first-parent commits to sample per starting tip, including the tip itself.
    #[arg(long, default_value = "16")]
    tips_per_start: NonZeroUsize,
    /// Number of timed batches per mode.
    #[arg(long, default_value = "11")]
    rounds: NonZeroUsize,
    /// Decoded object cache size in MiB for both modes. Use 0 to disable it.
    #[arg(long, default_value = "100")]
    object_cache_mib: usize,
    /// Disable the on-disk commit-graph for both modes.
    #[arg(long)]
    no_commit_graph: bool,
}

fn main() -> Result<()> {
    let args = Args::parse_from(gix::env::args_os());
    let repo = gix::discover(&args.repository)?;
    let hidden = args
        .hide
        .iter()
        .map(|rev| resolve_commit(&repo, rev))
        .collect::<Result<Vec<_>>>()?;
    let use_commit_graph = args.no_commit_graph.then_some(false);
    let mut seen = HashSet::new();
    let mut tips = Vec::new();
    for rev in &args.tip {
        let start_commit_id = resolve_commit(&repo, rev)?;
        for info in repo
            .rev_walk([start_commit_id])
            .first_parent_only()
            .use_commit_graph(use_commit_graph)
            .all()?
            .take(args.tips_per_start.get())
        {
            let tip_commit_id = info?.id;
            if seen.insert(tip_commit_id) {
                tips.push(tip_commit_id);
            }
        }
    }
    let batch = Batch {
        tips,
        hidden,
        use_commit_graph,
    };

    let object_cache_bytes = args
        .object_cache_mib
        .checked_mul(1024 * 1024)
        .ok_or_raise(|| message("Object cache size exceeds the address space").validation())?;
    let mut stock_repo = repo.clone();
    let mut cached_repo = repo.clone();
    stock_repo.object_cache_size(object_cache_bytes);
    cached_repo.object_cache_size(object_cache_bytes);

    let commits = batch.verify(&stock_repo, &cached_repo)?;
    if commits == 0 {
        bail!(
            message("The exclusions leave no visible commits. Choose an older or different --hide revision")
                .validation()
        );
    }
    println!("Repository: {}", repo.git_dir().display());
    println!(
        "Batch: {} walks, {} hidden tips per walk, {commits} yielded commits",
        batch.tips.len(),
        batch.hidden.len(),
    );
    println!(
        "Object cache: {} MiB per mode. Commit-graph: {}.",
        args.object_cache_mib,
        if args.no_commit_graph { "disabled" } else { "auto" },
    );

    let mut stock = Vec::new();
    let mut cached = Vec::new();
    for round in 0..args.rounds.get() {
        if round % 2 == 0 {
            stock.push(batch.time(&stock_repo, false)?);
            cached.push(batch.time(&cached_repo, true)?);
        } else {
            cached.push(batch.time(&cached_repo, true)?);
            stock.push(batch.time(&stock_repo, false)?);
        }
    }
    let stock = median(&mut stock).as_secs_f64();
    let cached = median(&mut cached).as_secs_f64();
    println!("Median of {} batches per mode", args.rounds);
    println!("  Stock:  {:9.3} ms", stock * 1000.0);
    println!("  Cached: {:9.3} ms", cached * 1000.0);
    println!("  Speedup (stock / cached): {:.2}x", stock / cached);
    Ok(())
}

fn resolve_commit(repo: &Repository, rev: &OsStr) -> Result<ObjectId> {
    let spec = gix::path::os_str_into_bstr(rev)?;
    repo.rev_parse_single(spec)
        .and_then(|id| id.object()?.peel_to_commit())
        .map(|commit| commit.id)
        .or_raise(|| message!("Could not resolve {spec:?} to a commit"))
}

struct Batch {
    tips: Vec<ObjectId>,
    hidden: Vec<ObjectId>,
    use_commit_graph: Option<bool>,
}

impl Batch {
    fn walk<'repo>(&self, repo: &'repo Repository, tip_commit_id: ObjectId) -> Platform<'repo, 'repo> {
        repo.rev_walk([tip_commit_id])
            .with_hidden(self.hidden.iter().copied())
            .use_commit_graph(self.use_commit_graph)
    }

    fn verify(&self, stock_repo: &Repository, cached_repo: &Repository) -> Result<usize> {
        let mut cache = cached_repo.rev_walk_cache();
        let mut commits = 0;
        for &tip_commit_id in &self.tips {
            let mut stock = self.walk(stock_repo, tip_commit_id).all()?;
            let mut cached = self.walk(cached_repo, tip_commit_id).with_cache(&mut cache).all()?;
            loop {
                match (stock.next().transpose()?, cached.next().transpose()?) {
                    (Some(stock), Some(cached)) if stock.id == cached.id => commits += 1,
                    (None, None) => break,
                    _ => bail!(message!(
                        "Stock and cached walks yielded different commits for tip {tip_commit_id}"
                    )),
                }
            }
        }
        Ok(commits)
    }

    fn time(&self, repo: &Repository, cached: bool) -> Result<Duration> {
        let start = Instant::now();
        let mut cache = cached.then(|| repo.rev_walk_cache());
        for &tip_commit_id in &self.tips {
            for info in self.walk(repo, tip_commit_id).with_cache(cache.as_mut()).all()? {
                black_box(info?.id);
            }
        }
        drop(cache);
        Ok(start.elapsed())
    }
}

fn median(samples: &mut [Duration]) -> Duration {
    samples.sort_unstable();
    let low = samples[(samples.len() - 1) / 2];
    let high = samples[samples.len() / 2];
    low + high.abs_diff(low) / 2
}
