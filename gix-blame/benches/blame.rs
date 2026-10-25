//! A benchmark for `gix-blame`. It benchmarks blames for files in `gitoxide` itself, in order to
//! exercise somewhat realistic workloads.
use std::path::PathBuf;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use gix_hash::ObjectId;

// These are files whose blames are supposed to be somewhat representative of real-world use cases.
const FILES_TO_BLAME: [&str; 9] = [
    // These files are among the most frequently changed ones in the entire repository. They mostly
    // exercise blob diffs and history traversal.
    "CHANGELOG.md",
    "STABILITY.md",
    "README.md",
    "Cargo.toml",
    // These files live in nested directories. In addition to blob diffs and history traversal,
    // they also exercise tree diffs.
    "gix-blame/src/file/function.rs",
    "gix-path/src/env/mod.rs",
    "gix-index/tests/index/file/write.rs",
    "gix-object/src/lib.rs",
    "gix-odb/src/store_impls/loose/write.rs",
];

// The commit to start blame from. A fixed commit was chosen to make the benchmark start from a
// stable starting point. This commit was created on 2026-10-25.
const COMMIT_TO_BLAME: &str = "d4db00dfdd5874ad13a40eff4d820fb242b986f7";

fn bench_blame(c: &mut Criterion) {
    let crate_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let (repository, _trust) = gix_discover::upwards(&crate_path).expect("did not find a repository");
    let (git_dir, worktree_dir) = repository.into_repository_and_work_tree_directories();

    let common_dir = match gix_discover::path::from_plain_file(&git_dir.join("commondir"))
        .transpose()
        .expect("was not able to resolve common dir")
    {
        Some(path) => git_dir.join(path),
        None => git_dir.clone(),
    }
    .canonicalize()
    .expect("was not able to canonicalize common dir");

    let odb = gix_odb::at(common_dir.join("objects"), gix_hash::Kind::Sha1).expect("object database could not be read");

    let head_id = ObjectId::from_hex(COMMIT_TO_BLAME.as_bytes()).expect("commit to blame to point to an actual commit");

    let index = gix_index::File::at(git_dir.join("index"), gix_hash::Kind::Sha1, false, Default::default())
        .expect("index could not be read");

    let create_resource_cache = || {
        let stack = gix_worktree::Stack::from_state_and_ignore_case(
            worktree_dir.as_deref().unwrap_or(&git_dir),
            false,
            gix_worktree::stack::State::AttributesAndIgnoreStack {
                attributes: Default::default(),
                ignore: Default::default(),
            },
            &index,
            index.path_backing(),
        );
        gix_diff::blob::Platform::new(
            Default::default(),
            gix_diff::blob::Pipeline::new(
                gix_diff::blob::pipeline::WorktreeRoots {
                    old_root: None,
                    new_root: None,
                },
                gix_filter::Pipeline::new(Default::default(), gix_hash::Kind::Sha1, Default::default()),
                vec![],
                gix_diff::blob::pipeline::Options {
                    large_file_threshold_bytes: 0,
                    fs: Default::default(),
                },
            ),
            gix_diff::blob::pipeline::Mode::ToGit,
            stack,
        )
    };

    for file in FILES_TO_BLAME {
        c.bench_function(file, |b| {
            // This benchmark measures `gix_blame::file` alone. Construction and destruction of
            // `resource_cache` is outside the timed section.
            b.iter_batched_ref(
                create_resource_cache,
                |resource_cache| {
                    let start = gix_blame::Start::Commit(head_id);

                    std::hint::black_box(
                        gix_blame::file(&odb, start, None, resource_cache, file.as_ref(), Default::default())
                            .expect("blame to not fail"),
                    )
                },
                BatchSize::SmallInput,
            );
        });
    }
}

criterion_group!(benches, bench_blame);
criterion_main!(benches);
