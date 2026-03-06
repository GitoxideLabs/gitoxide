use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

use criterion::{criterion_group, criterion_main, Criterion};
use gix_blame::{BlameEntry, BlameRanges, BlameSink};
use gix_object::bstr::BString;

const DEFAULT_BENCH_PATH: &str = "gix-blame/src/file/function.rs";

struct DiscardSink;

impl BlameSink for DiscardSink {
    fn push(&mut self, _entry: BlameEntry) {}
}

fn incremental_options() -> gix_blame::Options {
    gix_blame::Options {
        diff_algorithm: gix_diff::blob::Algorithm::Histogram,
        ranges: BlameRanges::default(),
        since: None,
        rewrites: Some(gix_diff::Rewrites::default()),
        debug_track_path: false,
    }
}

fn benchmark_incremental_blame(c: &mut Criterion) {
    let repo_path = env::var_os("GIX_BLAME_BENCH_REPO").map_or_else(
        || {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("gix-blame crate has a parent directory")
                .to_path_buf()
        },
        PathBuf::from,
    );
    let source_file_path: BString = env::var("GIX_BLAME_BENCH_PATH")
        .unwrap_or_else(|_| DEFAULT_BENCH_PATH.into())
        .into_bytes()
        .into();
    let commit_spec = env::var("GIX_BLAME_BENCH_COMMIT").unwrap_or_else(|_| "HEAD".into());
    let git_dir = repo_path.join(".git");

    if !has_commit_graph_cache(&git_dir) {
        write_changed_path_commit_graph(&repo_path).expect("commit-graph should be writable for benchmark repository");
    }

    let repo = gix::open(&repo_path).expect("repository can be opened");
    let suspect = repo
        .rev_parse_single(commit_spec.as_str())
        .expect("commit spec resolves to one object")
        .detach();

    let mut group = c.benchmark_group("gix-blame::incremental");
    group.bench_function("without-commit-graph", |b| {
        let mut resource_cache = repo
            .diff_resource_cache_for_tree_diff()
            .expect("tree-diff resource cache can be created");

        b.iter(|| {
            let mut sink = DiscardSink;
            gix_blame::incremental(
                &repo,
                suspect,
                None,
                &mut resource_cache,
                source_file_path.as_ref(),
                &mut sink,
                incremental_options(),
            )
            .expect("incremental blame should succeed");
        });
    });
    group.bench_function("with-commit-graph", |b| {
        let mut resource_cache = repo
            .diff_resource_cache_for_tree_diff()
            .expect("tree-diff resource cache can be created");
        let cache = repo.commit_graph().expect("commit-graph can be loaded from repository");
        b.iter(|| {
            let mut sink = DiscardSink;
            gix_blame::incremental(
                &repo,
                suspect,
                Some(&cache),
                &mut resource_cache,
                source_file_path.as_ref(),
                &mut sink,
                incremental_options(),
            )
            .expect("incremental blame should succeed");
        });
    });
    group.finish();
}

fn has_commit_graph_cache(git_dir: &Path) -> bool {
    let info_dir = git_dir.join("objects/info");
    info_dir.join("commit-graph").is_file() || info_dir.join("commit-graphs").is_dir()
}

fn write_changed_path_commit_graph(worktree_path: &Path) -> std::io::Result<()> {
    let config_status = Command::new("git")
        .args(["config", "commitGraph.changedPathsVersion", "2"])
        .current_dir(worktree_path)
        .status()?;
    assert!(
        config_status.success(),
        "setting commitGraph.changedPathsVersion should succeed"
    );

    let write_status = Command::new("git")
        .args([
            "commit-graph",
            "write",
            "--no-progress",
            "--reachable",
            "--changed-paths",
        ])
        .current_dir(worktree_path)
        .status()?;
    assert!(
        write_status.success(),
        "writing changed-path commit-graph should succeed"
    );
    Ok(())
}

criterion_group!(benches, benchmark_incremental_blame);
criterion_main!(benches);
