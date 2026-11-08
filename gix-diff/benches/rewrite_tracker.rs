use std::{convert::Infallible, fmt::Write, hint::black_box};

use bstr::{BString, ByteSlice};
use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use gix_diff::{
    Rewrites,
    rewrites::{Copies, CopySource, Outcome, Tracker, tracker::ChangeKind},
    tree::visit::Action,
    tree_with_rewrites::ChangeRef,
};
use gix_hash::{ObjectId, oid};
use gix_object::{
    FindExt, Kind, Tree, Write as ObjectWrite,
    tree::{EntryKind, EntryMode},
};

const EXACT_RENAME_COUNTS: &[usize] = &[128, 1_024, 4_096];
const AMBIGUOUS_IDENTICAL_COUNTS: &[usize] = &[16, 128, 512];
const SIMILARITY_RENAME_COUNTS: &[usize] = &[16, 64, 128, 512];
const EXHAUSTIVE_COPY_COUNTS: &[usize] = &[16, 128, 512];
const DIRECTORY_COUNTS: &[usize] = &[128, 1_024, 4_096];
/// The files of each renamed directory that keep their content, so their renames can be matched by id.
const UNCHANGED_FILES_PER_DIRECTORY: usize = 4;

#[derive(Clone, Copy)]
struct Change {
    id: ObjectId,
    kind: ChangeKind,
    mode: EntryMode,
}

impl Change {
    fn new(id: ObjectId, kind: ChangeKind) -> Self {
        Change {
            id,
            kind,
            mode: EntryKind::Blob.into(),
        }
    }
}

impl gix_diff::rewrites::tracker::Change for Change {
    fn id(&self) -> Option<&oid> {
        (!self.id.is_null()).then_some(self.id.as_ref())
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

fn bench_directories(c: &mut Criterion) {
    for (name, make_scenario, rewrites_enabled) in [
        (
            "renamed-directories",
            (|count| renamed_directories(count, false)) as fn(usize) -> DirectoryScenario,
            true,
        ),
        (
            "renamed-into-existing-directories",
            |count| renamed_directories(count, true),
            true,
        ),
        ("modified-directories", |count| modified_directories(count, 0), true),
        (
            "modified-directories-no-rewrites",
            |count| modified_directories(count, 0),
            false,
        ),
        (
            "modified-directories-with-renames",
            |count| modified_directories(count, 8),
            true,
        ),
        ("one-renamed-directory", one_renamed_directory, true),
    ] {
        let mut group = c.benchmark_group(format!("tree-diff/{name}"));
        for &count in DIRECTORY_COUNTS {
            let scenario = make_scenario(count);
            group.throughput(Throughput::Elements(count as u64));
            group.bench_with_input(BenchmarkId::from_parameter(count), &scenario, |b, scenario| {
                b.iter_batched(
                    || scenario,
                    |scenario| black_box(run_directory_diff(scenario, rewrites_enabled)),
                    BatchSize::SmallInput,
                );
            });
        }
        group.finish();
    }
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

struct DirectoryScenario {
    objects: ObjectDb,
    source_tree_id: ObjectId,
    destination_tree_id: ObjectId,
    expected: CallbackCounts,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct CallbackCounts {
    /// Additions, deletions, modifications, and rewrites, in that order.
    files: [usize; 4],
    /// Additions, deletions, modifications, and rewrites, in that order.
    directories: [usize; 4],
}

/// Independent directory moves with one edited child each require inference from file renames.
/// Existing destinations retain an unchanged child on both sides of the diff.
fn renamed_directories(count: usize, existing_destinations: bool) -> DirectoryScenario {
    let objects = object_db();
    let mut source = gix_object::tree::Editor::new(Tree::default(), &gix_object::find::Never, gix_hash::Kind::Sha1);
    let mut destination =
        gix_object::tree::Editor::new(Tree::default(), &gix_object::find::Never, gix_hash::Kind::Sha1);
    for idx in 0..count {
        let (old_dir, new_dir) = (format!("old-{idx:05}"), format!("new-{idx:05}"));
        if existing_destinations {
            let kept_blob_id = insert_blob(&objects, &format!("retained file in directory {idx}\n"));
            for tree in [&mut source, &mut destination] {
                tree.upsert([new_dir.as_str(), "kept.txt"], EntryKind::Blob, kept_blob_id)
                    .expect("benchmark paths are valid");
            }
        }
        for file in 0..=UNCHANGED_FILES_PER_DIRECTORY {
            let name = format!("file-{file}.txt");
            let source_blob_id = insert_blob(&objects, &format!("file {file} in directory {idx}\n"));
            let destination_blob_id = if file == UNCHANGED_FILES_PER_DIRECTORY {
                insert_blob(&objects, &format!("edited file {file} in directory {idx}\n"))
            } else {
                source_blob_id
            };
            source
                .upsert([old_dir.as_str(), name.as_str()], EntryKind::Blob, source_blob_id)
                .expect("benchmark paths are valid");
            destination
                .upsert([new_dir.as_str(), name.as_str()], EntryKind::Blob, destination_blob_id)
                .expect("benchmark paths are valid");
        }
    }
    DirectoryScenario {
        source_tree_id: source
            .write(|tree| objects.write(tree))
            .expect("in-memory tree writes succeed"),
        destination_tree_id: destination
            .write(|tree| objects.write(tree))
            .expect("in-memory tree writes succeed"),
        objects,
        expected: CallbackCounts {
            files: [count, count, 0, count * UNCHANGED_FILES_PER_DIRECTORY],
            directories: [0, 0, count * usize::from(existing_destinations), count],
        },
    }
}

/// Existing directories each modify one file. An optional directory move exercises inference
/// while leaving all directory modifications unrelated to it.
fn modified_directories(count: usize, renames: usize) -> DirectoryScenario {
    let objects = object_db();
    let mut source = gix_object::tree::Editor::new(Tree::default(), &gix_object::find::Never, gix_hash::Kind::Sha1);
    let mut destination =
        gix_object::tree::Editor::new(Tree::default(), &gix_object::find::Never, gix_hash::Kind::Sha1);
    for idx in 0..count {
        let directory = format!("dir-{idx:05}");
        let source_blob_id = insert_blob(&objects, &format!("original file in directory {idx}\n"));
        let destination_blob_id = insert_blob(&objects, &format!("modified file in directory {idx}\n"));
        source
            .upsert([directory.as_str(), "file.txt"], EntryKind::Blob, source_blob_id)
            .expect("benchmark paths are valid");
        destination
            .upsert([directory.as_str(), "file.txt"], EntryKind::Blob, destination_blob_id)
            .expect("benchmark paths are valid");
    }
    for idx in 0..renames {
        let name = format!("file-{idx}.txt");
        let blob_id = insert_blob(&objects, &format!("exact rename {idx}\n"));
        source
            .upsert(["old-renamed", name.as_str()], EntryKind::Blob, blob_id)
            .expect("benchmark paths are valid");
        destination
            .upsert(["new-renamed", name.as_str()], EntryKind::Blob, blob_id)
            .expect("benchmark paths are valid");
    }
    DirectoryScenario {
        source_tree_id: source
            .write(|tree| objects.write(tree))
            .expect("in-memory tree writes succeed"),
        destination_tree_id: destination
            .write(|tree| objects.write(tree))
            .expect("in-memory tree writes succeed"),
        objects,
        expected: CallbackCounts {
            files: [0, 0, count, renames],
            directories: [0, 0, count, usize::from(renames > 0)],
        },
    }
}

/// Many exact file renames all vote for one destination directory.
fn one_renamed_directory(count: usize) -> DirectoryScenario {
    let objects = object_db();
    let mut source = gix_object::tree::Editor::new(Tree::default(), &gix_object::find::Never, gix_hash::Kind::Sha1);
    let mut destination =
        gix_object::tree::Editor::new(Tree::default(), &gix_object::find::Never, gix_hash::Kind::Sha1);
    for idx in 0..count {
        let name = format!("file-{idx:05}.txt");
        let blob_id = insert_blob(&objects, &format!("unique file {idx}\n"));
        source
            .upsert(["old", name.as_str()], EntryKind::Blob, blob_id)
            .expect("benchmark paths are valid");
        destination
            .upsert(["new", name.as_str()], EntryKind::Blob, blob_id)
            .expect("benchmark paths are valid");
    }
    DirectoryScenario {
        source_tree_id: source
            .write(|tree| objects.write(tree))
            .expect("in-memory tree writes succeed"),
        destination_tree_id: destination
            .write(|tree| objects.write(tree))
            .expect("in-memory tree writes succeed"),
        objects,
        expected: CallbackCounts {
            files: [0, 0, 0, count],
            directories: [0, 0, 0, 1],
        },
    }
}

fn run_directory_diff(scenario: &DirectoryScenario, rewrites_enabled: bool) -> Option<Outcome> {
    let (mut source_buf, mut destination_buf) = (Vec::new(), Vec::new());
    let mut actual = CallbackCounts::default();
    let outcome = gix_diff::tree_with_rewrites(
        scenario
            .objects
            .find_tree_iter(&scenario.source_tree_id, &mut source_buf)
            .expect("source tree exists"),
        scenario
            .objects
            .find_tree_iter(&scenario.destination_tree_id, &mut destination_buf)
            .expect("destination tree exists"),
        &mut new_diff_platform(),
        &mut Default::default(),
        &scenario.objects,
        |change| {
            let (entry_mode, kind) = match change {
                ChangeRef::Addition { entry_mode, .. } => (entry_mode, 0),
                ChangeRef::Deletion { entry_mode, .. } => (entry_mode, 1),
                ChangeRef::Modification { entry_mode, .. } => (entry_mode, 2),
                ChangeRef::Rewrite { entry_mode, .. } => (entry_mode, 3),
            };
            let counts = if entry_mode.is_tree() {
                &mut actual.directories
            } else {
                &mut actual.files
            };
            counts[kind] += 1;
            black_box(change);
            Ok(Action::Continue(()))
        },
        gix_diff::tree_with_rewrites::Options {
            location: Some(gix_diff::tree::recorder::Location::Path),
            rewrites: rewrites_enabled.then_some(Rewrites {
                percentage: None,
                limit: 0,
                ..Default::default()
            }),
        },
    )
    .expect("benchmark tree diff succeeds");
    assert_eq!(
        actual, scenario.expected,
        "each callback kind occurs exactly as expected"
    );
    outcome
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
    bench_directories
);
criterion_main!(benches);
