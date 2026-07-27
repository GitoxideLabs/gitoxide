use std::{collections::BTreeMap, hint::black_box, path::Path};

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use gix_diff::Rewrites;
use gix_hash::ObjectId;
use gix_merge::tree::{Options, Outcome};
use gix_object::{
    Kind, Tree, Write,
    tree::{EntryKind, EntryMode},
};
use gix_worktree::stack::state::attributes;

const CASE_COUNT: u64 = 34;

type ObjectDb = gix_odb::memory::Proxy<gix_object::find::Never>;
type Entries = BTreeMap<&'static str, (EntryMode, ObjectId)>;

/// A small, shallow tree containing independent examples of the main structural merge cases.
///
/// The numbered paths cover additions, deletions, modifications, modes, types, rename combinations,
/// directory renames, and file/directory replacements. Exact rename detection avoids spending the
/// benchmark on similarity scoring, and only the modify/modify case needs a genuine text merge.
struct Scenario {
    objects: ObjectDb,
    base: ObjectId,
    ours: ObjectId,
    theirs: ObjectId,
}

fn tree_merge(c: &mut Criterion) {
    let scenario = scenario();
    let mut diff_state = gix_diff::tree::State::default();
    let mut diff_resource_cache = new_diff_resource_cache();
    let mut blob_merge = new_blob_merge_platform();
    let options = options();

    let outcome = merge(
        &scenario,
        scenario.ours,
        scenario.theirs,
        &mut diff_state,
        &mut diff_resource_cache,
        &mut blob_merge,
        options.clone(),
    );
    assert!(
        outcome.conflicts.len() >= 12,
        "the mixed scenario should keep exercising many conflict resolutions"
    );

    let mut group = c.benchmark_group("tree-merge/mixed-structural-cases");
    group.throughput(Throughput::Elements(CASE_COUNT));
    group.bench_function("merge", |b| {
        b.iter_batched(
            || options.clone(),
            |options| {
                black_box(merge(
                    &scenario,
                    scenario.ours,
                    scenario.theirs,
                    &mut diff_state,
                    &mut diff_resource_cache,
                    &mut blob_merge,
                    options,
                ))
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn merge<'objects>(
    scenario: &'objects Scenario,
    ours: ObjectId,
    theirs: ObjectId,
    diff_state: &mut gix_diff::tree::State,
    diff_resource_cache: &mut gix_diff::blob::Platform,
    blob_merge: &mut gix_merge::blob::Platform,
    options: Options,
) -> Outcome<'objects> {
    gix_merge::tree(
        &scenario.base,
        &ours,
        &theirs,
        gix_merge::blob::builtin_driver::text::Labels {
            ancestor: Some("BASE".into()),
            current: Some("OURS".into()),
            other: Some("THEIRS".into()),
        },
        &scenario.objects,
        |buf| scenario.objects.write_buf(Kind::Blob, buf),
        diff_state,
        diff_resource_cache,
        blob_merge,
        options,
    )
    .expect("in-memory tree merge succeeds")
}

fn scenario() -> Scenario {
    let objects = ObjectDb::new(gix_object::find::Never, gix_hash::Kind::Sha1);
    let mut base = Entries::new();

    let modify_ours = insert_base(&objects, &mut base, "01-modify-ours", EntryKind::Blob);
    let modify_theirs = insert_base(&objects, &mut base, "02-modify-theirs", EntryKind::Blob);
    let modify_same = insert_base(&objects, &mut base, "03-modify-same", EntryKind::Blob);
    let _modify_both = insert_base(&objects, &mut base, "04-modify-both", EntryKind::Blob);
    let _delete_ours = insert_base(&objects, &mut base, "05-delete-ours", EntryKind::Blob);
    let _delete_both = insert_base(&objects, &mut base, "06-delete-both", EntryKind::Blob);
    let _modify_delete = insert_base(&objects, &mut base, "07-modify-delete", EntryKind::Blob);
    let mode_ours = insert_base(&objects, &mut base, "08-mode-ours", EntryKind::Blob);
    let mode_and_modify = insert_base(&objects, &mut base, "09-mode-and-modify", EntryKind::Blob);
    let _symlink_delete = insert_base(&objects, &mut base, "10-symlink-delete", EntryKind::Link);

    for path in [
        "16-rename-clean/source",
        "17-rename-same/source",
        "18-rename-modify/source",
        "19-rename-delete/source",
        "20-rename-different/source",
        "21-rename-add/source",
        "22-rename-destination/source",
        "22-rename-destination/target",
        "23-rename-destination-delete/source",
        "23-rename-destination-delete/target",
        "24-two-to-one/one",
        "24-two-to-one/two",
        "25-directory-rename/old/existing",
        "26-directory-rename-modify/old/file",
        "27-directory-rename-different/old/file",
        "28-file-to-directory/node",
        "29-directory-to-file/node/child",
        "30-both-file-to-directory/node",
        "31-delete-vs-file-to-directory/node",
        "32-rename-vs-file-to-directory/node",
    ] {
        insert_base(&objects, &mut base, path, EntryKind::Blob);
    }

    let mut ours = base.clone();
    let mut theirs = base.clone();

    set_blob(&objects, &mut ours, "01-modify-ours", b"ours\n", EntryKind::Blob);
    set_blob(&objects, &mut theirs, "02-modify-theirs", b"theirs\n", EntryKind::Blob);
    let same = blob(&objects, b"same\n");
    ours.insert("03-modify-same", (EntryKind::Blob.into(), same));
    theirs.insert("03-modify-same", (EntryKind::Blob.into(), same));
    set_blob(&objects, &mut ours, "04-modify-both", b"ours\n", EntryKind::Blob);
    set_blob(&objects, &mut theirs, "04-modify-both", b"theirs\n", EntryKind::Blob);
    ours.remove("05-delete-ours");
    ours.remove("06-delete-both");
    theirs.remove("06-delete-both");
    set_blob(&objects, &mut ours, "07-modify-delete", b"modified\n", EntryKind::Blob);
    theirs.remove("07-modify-delete");
    ours.insert("08-mode-ours", (EntryKind::BlobExecutable.into(), mode_ours));
    ours.insert(
        "09-mode-and-modify",
        (EntryKind::BlobExecutable.into(), mode_and_modify),
    );
    set_blob(
        &objects,
        &mut theirs,
        "09-mode-and-modify",
        b"modified\n",
        EntryKind::Blob,
    );
    set_blob(
        &objects,
        &mut ours,
        "10-symlink-delete",
        b"new-target\n",
        EntryKind::Link,
    );
    theirs.remove("10-symlink-delete");

    set_blob(&objects, &mut ours, "11-add-ours", b"added\n", EntryKind::Blob);
    let same_addition = blob(&objects, b"same addition\n");
    ours.insert("12-add-same", (EntryKind::Blob.into(), same_addition));
    theirs.insert("12-add-same", (EntryKind::Blob.into(), same_addition));
    let mode_addition = blob(&objects, b"mode addition\n");
    ours.insert("13-add-mode", (EntryKind::Blob.into(), mode_addition));
    theirs.insert("13-add-mode", (EntryKind::BlobExecutable.into(), mode_addition));
    let type_addition = blob(&objects, b"type addition\n");
    ours.insert("14-add-type", (EntryKind::Blob.into(), type_addition));
    theirs.insert("14-add-type", (EntryKind::Link.into(), type_addition));
    set_blob(&objects, &mut ours, "15-add-directory/ours", b"ours\n", EntryKind::Blob);
    set_blob(
        &objects,
        &mut theirs,
        "15-add-directory/theirs",
        b"theirs\n",
        EntryKind::Blob,
    );

    rename(&mut ours, "16-rename-clean/source", "16-rename-clean/target");
    rename(&mut ours, "17-rename-same/source", "17-rename-same/target");
    rename(&mut theirs, "17-rename-same/source", "17-rename-same/target");
    rename(&mut ours, "18-rename-modify/source", "18-rename-modify/target");
    set_blob(
        &objects,
        &mut theirs,
        "18-rename-modify/source",
        b"modified\n",
        EntryKind::Blob,
    );
    rename(&mut ours, "19-rename-delete/source", "19-rename-delete/target");
    theirs.remove("19-rename-delete/source");
    rename(&mut ours, "20-rename-different/source", "20-rename-different/ours");
    rename(&mut theirs, "20-rename-different/source", "20-rename-different/theirs");
    rename(&mut ours, "21-rename-add/source", "21-rename-add/target");
    set_blob(
        &objects,
        &mut theirs,
        "21-rename-add/target",
        b"addition\n",
        EntryKind::Blob,
    );
    replace_destination_with_source(
        &mut ours,
        "22-rename-destination/source",
        "22-rename-destination/target",
    );
    set_blob(
        &objects,
        &mut theirs,
        "22-rename-destination/target",
        b"modified target\n",
        EntryKind::Blob,
    );
    replace_destination_with_source(
        &mut ours,
        "23-rename-destination-delete/source",
        "23-rename-destination-delete/target",
    );
    theirs.remove("23-rename-destination-delete/target");
    rename(&mut ours, "24-two-to-one/one", "24-two-to-one/target");
    rename(&mut theirs, "24-two-to-one/two", "24-two-to-one/target");

    rename(
        &mut ours,
        "25-directory-rename/old/existing",
        "25-directory-rename/new/existing",
    );
    set_blob(
        &objects,
        &mut theirs,
        "25-directory-rename/old/added",
        b"added\n",
        EntryKind::Blob,
    );
    rename(
        &mut ours,
        "26-directory-rename-modify/old/file",
        "26-directory-rename-modify/new/file",
    );
    set_blob(
        &objects,
        &mut theirs,
        "26-directory-rename-modify/old/file",
        b"modified\n",
        EntryKind::Blob,
    );
    rename(
        &mut ours,
        "27-directory-rename-different/old/file",
        "27-directory-rename-different/ours/file",
    );
    rename(
        &mut theirs,
        "27-directory-rename-different/old/file",
        "27-directory-rename-different/theirs/file",
    );

    ours.remove("28-file-to-directory/node");
    set_blob(
        &objects,
        &mut ours,
        "28-file-to-directory/node/child",
        b"child\n",
        EntryKind::Blob,
    );
    set_blob(
        &objects,
        &mut theirs,
        "28-file-to-directory/node",
        b"modified\n",
        EntryKind::Blob,
    );
    ours.remove("29-directory-to-file/node/child");
    set_blob(
        &objects,
        &mut ours,
        "29-directory-to-file/node",
        b"file\n",
        EntryKind::Blob,
    );
    set_blob(
        &objects,
        &mut theirs,
        "29-directory-to-file/node/child",
        b"modified\n",
        EntryKind::Blob,
    );
    ours.remove("30-both-file-to-directory/node");
    theirs.remove("30-both-file-to-directory/node");
    set_blob(
        &objects,
        &mut ours,
        "30-both-file-to-directory/node/ours",
        b"ours\n",
        EntryKind::Blob,
    );
    set_blob(
        &objects,
        &mut theirs,
        "30-both-file-to-directory/node/theirs",
        b"theirs\n",
        EntryKind::Blob,
    );
    ours.remove("31-delete-vs-file-to-directory/node");
    theirs.remove("31-delete-vs-file-to-directory/node");
    set_blob(
        &objects,
        &mut theirs,
        "31-delete-vs-file-to-directory/node/child",
        b"child\n",
        EntryKind::Blob,
    );
    rename(
        &mut ours,
        "32-rename-vs-file-to-directory/node",
        "32-rename-vs-file-to-directory/away",
    );
    theirs.remove("32-rename-vs-file-to-directory/node");
    set_blob(
        &objects,
        &mut theirs,
        "32-rename-vs-file-to-directory/node/child",
        b"child\n",
        EntryKind::Blob,
    );
    set_blob(
        &objects,
        &mut ours,
        "33-add-file-vs-directory/node",
        b"file\n",
        EntryKind::Blob,
    );
    set_blob(
        &objects,
        &mut theirs,
        "33-add-file-vs-directory/node/child",
        b"child\n",
        EntryKind::Blob,
    );
    let executable_addition = blob(&objects, b"executable\n");
    ours.insert(
        "34-add-executable-same",
        (EntryKind::BlobExecutable.into(), executable_addition),
    );
    theirs.insert(
        "34-add-executable-same",
        (EntryKind::BlobExecutable.into(), executable_addition),
    );

    let base = write_tree(&objects, &base);
    let ours = write_tree(&objects, &ours);
    let theirs = write_tree(&objects, &theirs);
    assert_ne!(modify_ours, modify_theirs);
    assert_ne!(modify_same, modify_ours);

    Scenario {
        objects,
        base,
        ours,
        theirs,
    }
}

fn insert_base(objects: &ObjectDb, entries: &mut Entries, path: &'static str, kind: EntryKind) -> ObjectId {
    let id = blob(objects, path.as_bytes());
    entries.insert(path, (kind.into(), id));
    id
}

fn set_blob(objects: &ObjectDb, entries: &mut Entries, path: &'static str, data: &[u8], kind: EntryKind) {
    let mut unique_data = path.as_bytes().to_vec();
    unique_data.push(b'\n');
    unique_data.extend_from_slice(data);
    entries.insert(path, (kind.into(), blob(objects, &unique_data)));
}

fn blob(objects: &ObjectDb, data: &[u8]) -> ObjectId {
    objects
        .write_buf(Kind::Blob, data)
        .expect("in-memory object writes succeed")
}

fn rename(entries: &mut Entries, source: &'static str, destination: &'static str) {
    let entry = entries.remove(source).expect("rename source exists");
    entries.insert(destination, entry);
}

fn replace_destination_with_source(entries: &mut Entries, source: &'static str, destination: &'static str) {
    let source = entries.remove(source).expect("rename source exists");
    entries.insert(destination, source);
}

fn write_tree(objects: &ObjectDb, entries: &Entries) -> ObjectId {
    let mut editor = gix_object::tree::Editor::new(Tree::default(), &gix_object::find::Never, gix_hash::Kind::Sha1);
    for (path, (mode, id)) in entries {
        editor
            .upsert(path.split('/'), mode.kind(), *id)
            .expect("benchmark paths are valid");
    }
    editor
        .write(|tree| objects.write(tree))
        .expect("in-memory tree writes succeed")
}

fn options() -> Options {
    Options {
        rewrites: Some(Rewrites {
            copies: None,
            percentage: Some(1.0),
            limit: 0,
            track_empty: false,
        }),
        ..Default::default()
    }
}

fn new_diff_resource_cache() -> gix_diff::blob::Platform {
    gix_diff::blob::Platform::new(
        Default::default(),
        gix_diff::blob::Pipeline::new(Default::default(), Default::default(), Vec::new(), Default::default()),
        Default::default(),
        gix_worktree::Stack::new(
            Path::new("gix-merge-benchmark-no-worktree"),
            gix_worktree::stack::State::AttributesStack(gix_worktree::stack::state::Attributes::default()),
            Default::default(),
            Vec::new(),
            Vec::new(),
        ),
    )
}

fn new_blob_merge_platform() -> gix_merge::blob::Platform {
    let attributes = gix_worktree::Stack::new(
        Path::new("gix-merge-benchmark-no-worktree"),
        gix_worktree::stack::State::AttributesStack(gix_worktree::stack::state::Attributes::new(
            Default::default(),
            None,
            attributes::Source::WorktreeThenIdMapping,
            Default::default(),
        )),
        gix_worktree::glob::pattern::Case::Sensitive,
        Vec::new(),
        Vec::new(),
    );
    gix_merge::blob::Platform::new(
        gix_merge::blob::Pipeline::new(Default::default(), gix_filter::Pipeline::default(), Default::default()),
        gix_merge::blob::pipeline::Mode::ToGit,
        attributes,
        vec![],
        Default::default(),
    )
}

criterion_group!(benches, tree_merge);
criterion_main!(benches);
