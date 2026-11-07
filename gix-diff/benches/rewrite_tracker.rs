use std::{convert::Infallible, fmt::Write, hint::black_box};

use bstr::{BString, ByteSlice};
use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use gix_diff::{
    Rewrites,
    rewrites::{Copies, CopySource, Outcome, Tracker, tracker::ChangeKind},
    tree::visit::{Action, ChangeId, Relation},
};
use gix_hash::{ObjectId, oid};
use gix_object::{
    Kind, Write as ObjectWrite,
    tree::{EntryKind, EntryMode},
};

const EXACT_RENAME_COUNTS: &[usize] = &[128, 1_024, 4_096];
const AMBIGUOUS_IDENTICAL_COUNTS: &[usize] = &[16, 128, 512];
const SIMILARITY_RENAME_COUNTS: &[usize] = &[16, 64, 128, 512];
const EXHAUSTIVE_COPY_COUNTS: &[usize] = &[16, 128, 512];
const RENAMED_DIRECTORY_COUNTS: &[usize] = &[128, 1_024, 4_096];
/// The files of each renamed directory that keep their content, so their renames can be matched by id.
const UNCHANGED_FILES_PER_DIRECTORY: usize = 4;

#[derive(Clone, Copy)]
struct Change {
    id: ObjectId,
    kind: ChangeKind,
    mode: EntryMode,
    relation: Option<Relation>,
}

impl Change {
    fn new(id: ObjectId, kind: ChangeKind) -> Self {
        Change {
            id,
            kind,
            mode: EntryKind::Blob.into(),
            relation: None,
        }
    }
}

impl gix_diff::rewrites::tracker::Change for Change {
    fn id(&self) -> &oid {
        &self.id
    }

    fn relation(&self) -> Option<Relation> {
        self.relation
    }

    fn kind(&self) -> ChangeKind {
        self.kind
    }

    fn entry_mode(&self) -> EntryMode {
        self.mode
    }
}

type ObjectDb = gix_odb::memory::Proxy<gix_object::find::Never>;

fn object_db() -> ObjectDb {
    gix_odb::memory::Proxy::new(gix_object::find::Never, gix_hash::Kind::Sha1)
}

fn insert_blob(objects: &ObjectDb, data: &str) -> ObjectId {
    objects
        .write_buf(Kind::Blob, data.as_bytes())
        .expect("in-memory object writes succeed")
}

struct Scenario {
    rewrites: Rewrites,
    changes: Vec<(Change, BString)>,
    source_tree_changes: Vec<(Change, BString)>,
    objects: ObjectDb,
    expected_matches: usize,
    expected_similarity_checks: Option<usize>,
}

fn bench_exact_renames(c: &mut Criterion) {
    let mut group = c.benchmark_group("rewrite-tracker/exact-renames");
    for &count in EXACT_RENAME_COUNTS {
        let scenario = exact_renames(count);
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &scenario, |b, scenario| {
            b.iter_batched(
                || scenario,
                |scenario| black_box(run_tracker(scenario)),
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_ambiguous_identical_content(c: &mut Criterion) {
    let mut group = c.benchmark_group("rewrite-tracker/ambiguous-identical-content");
    for &count in AMBIGUOUS_IDENTICAL_COUNTS {
        let scenario = ambiguous_identical_content(count);
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &scenario, |b, scenario| {
            b.iter_batched(
                || scenario,
                |scenario| black_box(run_tracker(scenario)),
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_similarity_renames(c: &mut Criterion) {
    for (name, make_scenario) in [
        ("similarity-renames", similarity_renames as fn(usize) -> Scenario),
        ("similarity-without-basename", similarity_without_basename),
        ("edited-binary", edited_binary),
        ("reused-copy-candidates", reused_copy_candidates),
    ] {
        let mut group = c.benchmark_group(format!("rewrite-tracker/{name}"));
        for &count in SIMILARITY_RENAME_COUNTS {
            let scenario = make_scenario(count);
            group.throughput(Throughput::Elements(scenario.expected_matches as u64));
            group.bench_with_input(BenchmarkId::from_parameter(count), &scenario, |b, scenario| {
                b.iter_batched(
                    || scenario,
                    |scenario| black_box(run_tracker(scenario)),
                    BatchSize::SmallInput,
                );
            });
        }
        group.finish();
    }
}

fn bench_exhaustive_copy_sources(c: &mut Criterion) {
    let mut group = c.benchmark_group("rewrite-tracker/exhaustive-copy-sources");
    for &count in EXHAUSTIVE_COPY_COUNTS {
        let scenario = exhaustive_copy_sources(count);
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &scenario, |b, scenario| {
            b.iter_batched(
                || scenario,
                |scenario| black_box(run_tracker(scenario)),
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_renamed_directories(c: &mut Criterion) {
    let mut group = c.benchmark_group("rewrite-tracker/renamed-directories");
    for &count in RENAMED_DIRECTORY_COUNTS {
        let scenario = renamed_directories(count);
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &scenario, |b, scenario| {
            b.iter_batched(
                || scenario,
                |scenario| black_box(run_tracker(scenario)),
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn exact_renames(count: usize) -> Scenario {
    let objects = object_db();
    let mut changes = Vec::with_capacity(count * 2);
    for idx in 0..count {
        let content = format!("unique blob {idx}\n");
        let id = insert_blob(&objects, &content);
        changes.push((
            Change::new(id, ChangeKind::Deletion),
            format!("old/file-{idx:05}.txt").into(),
        ));
        changes.push((
            Change::new(id, ChangeKind::Addition),
            format!("new/file-{idx:05}.txt").into(),
        ));
    }
    Scenario {
        rewrites: Rewrites {
            percentage: None,
            limit: 0,
            ..Default::default()
        },
        changes,
        source_tree_changes: Vec::new(),
        objects,
        expected_matches: count,
        expected_similarity_checks: Some(0),
    }
}

fn ambiguous_identical_content(count: usize) -> Scenario {
    let objects = object_db();
    let id = insert_blob(&objects, "same content in many files\n");
    let mut changes = Vec::with_capacity(count * 2);
    for idx in 0..count {
        changes.push((
            Change::new(id, ChangeKind::Deletion),
            format!("old/file-{idx:05}.txt").into(),
        ));
        changes.push((
            Change::new(id, ChangeKind::Addition),
            format!("new/file-{idx:05}.txt").into(),
        ));
    }
    Scenario {
        rewrites: Rewrites {
            percentage: None,
            limit: 0,
            ..Default::default()
        },
        changes,
        source_tree_changes: Vec::new(),
        objects,
        expected_matches: count,
        expected_similarity_checks: Some(0),
    }
}

fn similarity_renames(count: usize) -> Scenario {
    let objects = object_db();
    let mut changes = Vec::with_capacity(count * 2);
    for idx in 0..count {
        let before = text_blob(idx, false);
        let after = text_blob(idx, true);
        let before_id = insert_blob(&objects, &before);
        let after_id = insert_blob(&objects, &after);
        changes.push((
            Change::new(before_id, ChangeKind::Deletion),
            format!("old/file-{idx:05}.txt").into(),
        ));
        changes.push((
            Change::new(after_id, ChangeKind::Addition),
            format!("new/file-{idx:05}.txt").into(),
        ));
    }
    Scenario {
        rewrites: Rewrites {
            percentage: Some(0.8),
            limit: 0,
            track_empty: false,
            copies: None,
        },
        changes,
        source_tree_changes: Vec::new(),
        objects,
        expected_matches: count,
        expected_similarity_checks: None,
    }
}

fn similarity_without_basename(count: usize) -> Scenario {
    let mut scenario = similarity_renames(count);
    for (idx, (_, path)) in scenario.changes.iter_mut().enumerate() {
        if idx % 2 == 1 {
            *path = format!("new/renamed-{:05}.txt", idx / 2).into();
        }
    }
    scenario
}

fn edited_binary(count: usize) -> Scenario {
    let mut scenario = similarity_without_basename(count);
    for (idx, (change, _)) in scenario.changes.iter_mut().enumerate() {
        let data = format!("\0{}", text_blob(idx / 2, idx % 2 == 1));
        change.id = insert_blob(&scenario.objects, &data);
    }
    scenario
}

fn reused_copy_candidates(count: usize) -> Scenario {
    let mut scenario = similarity_without_basename(count);
    scenario.rewrites.copies = Some(Copies {
        percentage: Some(0.8),
        ..Default::default()
    });
    for (change, _) in &mut scenario.changes {
        if change.kind == ChangeKind::Deletion {
            change.kind = ChangeKind::Modification;
        }
    }
    for idx in 0..count {
        scenario.changes.push((
            scenario.changes[idx * 2 + 1].0,
            format!("second/copy-{idx:05}.txt").into(),
        ));
    }
    scenario.expected_matches = 2 * count;
    scenario
}

fn exhaustive_copy_sources(count: usize) -> Scenario {
    let objects = object_db();
    let id = insert_blob(&objects, "same content copied many times\n");
    let mut changes = Vec::with_capacity(count);
    let mut source_tree_changes = Vec::with_capacity(count);
    for idx in 0..count {
        changes.push((
            Change::new(id, ChangeKind::Addition),
            format!("copies/file-{idx:05}.txt").into(),
        ));
        source_tree_changes.push((
            Change::new(id, ChangeKind::Modification),
            format!("sources/file-{idx:05}.txt").into(),
        ));
    }
    Scenario {
        rewrites: Rewrites {
            percentage: None,
            limit: 0,
            track_empty: false,
            copies: Some(Copies {
                source: CopySource::FromSetOfModifiedFilesAndAllSources,
                percentage: None,
            }),
        },
        changes,
        source_tree_changes,
        objects,
        expected_matches: count,
        expected_similarity_checks: Some(0),
    }
}

/// `count` directories that are renamed while one of their files changes, so the directories differ by id
/// and can only be matched through the renames of their unchanged files.
fn renamed_directories(count: usize) -> Scenario {
    let objects = object_db();
    let mut changes = Vec::with_capacity(count * 2 * (UNCHANGED_FILES_PER_DIRECTORY + 2));
    for idx in 0..count {
        let (old_dir, new_dir) = (format!("old/dir-{idx:05}"), format!("new/dir-{idx:05}"));
        let (old_dir_id, new_dir_id) = (2 * idx as ChangeId, 2 * idx as ChangeId + 1);
        for (dir, dir_id, kind) in [
            (&old_dir, old_dir_id, ChangeKind::Deletion),
            (&new_dir, new_dir_id, ChangeKind::Addition),
        ] {
            let tree_id = gix_object::compute_hash(gix_hash::Kind::Sha1, Kind::Tree, dir.as_bytes())
                .expect("hashing in memory succeeds");
            changes.push((
                Change {
                    mode: EntryKind::Tree.into(),
                    relation: Some(Relation::Parent(dir_id)),
                    ..Change::new(tree_id, kind)
                },
                dir.as_str().into(),
            ));
        }
        let child = |blob_id, kind, dir_id| Change {
            relation: Some(Relation::ChildOfParent(dir_id)),
            ..Change::new(blob_id, kind)
        };
        for file in 0..UNCHANGED_FILES_PER_DIRECTORY {
            let blob_id = insert_blob(&objects, &format!("unchanged file {file} in directory {idx}\n"));
            changes.push((
                child(blob_id, ChangeKind::Deletion, old_dir_id),
                format!("{old_dir}/file-{file}.txt").into(),
            ));
            changes.push((
                child(blob_id, ChangeKind::Addition, new_dir_id),
                format!("{new_dir}/file-{file}.txt").into(),
            ));
        }
        let before_blob_id = insert_blob(&objects, &format!("changed file in directory {idx}\n"));
        let after_blob_id = insert_blob(&objects, &format!("changed file in directory {idx}, now edited\n"));
        changes.push((
            child(before_blob_id, ChangeKind::Deletion, old_dir_id),
            format!("{old_dir}/changed.txt").into(),
        ));
        changes.push((
            child(after_blob_id, ChangeKind::Addition, new_dir_id),
            format!("{new_dir}/changed.txt").into(),
        ));
    }
    Scenario {
        rewrites: Rewrites {
            percentage: None,
            limit: 0,
            ..Default::default()
        },
        changes,
        source_tree_changes: Vec::new(),
        objects,
        // The unchanged files and their directory are renames; the changed file is an addition and a deletion.
        expected_matches: count * (UNCHANGED_FILES_PER_DIRECTORY + 1),
        expected_similarity_checks: Some(0),
    }
}

fn text_blob(idx: usize, with_edit: bool) -> String {
    let mut out = String::new();
    for line in 0..48 {
        writeln!(out, "file-{idx:05} stable line {line:02}").expect("writing to string never fails");
        if with_edit && line % 12 == 0 {
            writeln!(out, "file-{idx:05} added line {line:02}").expect("writing to string never fails");
        }
    }
    out
}

fn run_tracker(scenario: &Scenario) -> Outcome {
    let mut tracker = Tracker::new(scenario.rewrites);
    for (change, location) in &scenario.changes {
        assert!(
            tracker.try_push_change(*change, location.as_bstr()).is_none(),
            "benchmark changes must be retained by rewrite tracking"
        );
    }

    let mut matches = 0;
    let outcome = tracker
        .emit(
            |destination, source| {
                black_box(destination);
                if let Some(source) = source {
                    black_box(source);
                    matches += 1;
                }
                Action::Continue(())
            },
            &mut new_diff_platform(),
            &scenario.objects,
            |push| -> Result<(), Infallible> {
                for (change, location) in &scenario.source_tree_changes {
                    push(*change, location.as_bstr());
                }
                Ok(())
            },
        )
        .expect("benchmark rewrite tracking should succeed");

    assert_eq!(
        matches, scenario.expected_matches,
        "benchmark scenario should produce the expected amount of rewrites"
    );
    if let Some(expected) = scenario.expected_similarity_checks {
        assert_eq!(
            outcome.num_similarity_checks, expected,
            "benchmark scenario should run the expected amount of similarity checks"
        );
    } else {
        assert!(
            outcome.num_similarity_checks >= scenario.expected_matches,
            "similarity benchmark should exercise internal diff checks"
        );
    }
    outcome
}

fn new_diff_platform() -> gix_diff::blob::Platform {
    let attributes = gix_worktree::Stack::new(
        std::env::temp_dir(),
        gix_worktree::stack::State::AttributesStack(gix_worktree::stack::state::Attributes::default()),
        gix_worktree::glob::pattern::Case::Sensitive,
        Vec::new(),
        Vec::new(),
    );
    let filter = gix_diff::blob::Pipeline::new(
        Default::default(),
        gix_filter::Pipeline::new(Default::default(), gix_hash::Kind::Sha1, Default::default()),
        Vec::new(),
        Default::default(),
    );
    gix_diff::blob::Platform::new(
        Default::default(),
        filter,
        gix_diff::blob::pipeline::Mode::ToGit,
        attributes,
    )
}

criterion_group!(
    benches,
    bench_exact_renames,
    bench_ambiguous_identical_content,
    bench_similarity_renames,
    bench_exhaustive_copy_sources,
    bench_renamed_directories
);
criterion_main!(benches);
