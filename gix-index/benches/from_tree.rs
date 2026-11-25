use std::hint::black_box;

use bstr::BString;
use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use gix_index::State;
use gix_object::{Tree, Write, tree::EntryKind};

fn from_tree(c: &mut Criterion) {
    let mut group = c.benchmark_group("from_tree");

    let (flat_id, flat_objects, flat_entries) = flat_tree();
    group.throughput(Throughput::Elements(flat_entries));
    group.bench_function("flat 10k files", |b| {
        b.iter(|| {
            let state = State::from_tree(&flat_id, &flat_objects, Default::default()).expect("tree can be read");
            black_box(state);
        });
    });

    let (wide_deep_id, wide_deep_objects, wide_deep_entries) = wide_deep_tree();
    group.throughput(Throughput::Elements(wide_deep_entries));
    group.bench_function("wide 100 x 100 files", |b| {
        b.iter(|| {
            let state =
                State::from_tree(&wide_deep_id, &wide_deep_objects, Default::default()).expect("tree can be read");
            black_box(state);
        });
    });

    let mut cached_index = State::from_tree(&wide_deep_id, &wide_deep_objects, Default::default())
        .expect("tree can be read to populate the cache");
    cached_index.set_tree(Some(gix_index::extension::Tree {
        name: Default::default(),
        id: wide_deep_id,
        num_entries: Some(u32::try_from(wide_deep_entries).expect("benchmark entries fit in u32")),
        children: Vec::new(),
    }));
    group.bench_function("wide 100 x 100 files cached", |b| {
        b.iter(|| {
            let state =
                State::from_tree_with_cache(&wide_deep_id, &wide_deep_objects, Default::default(), &cached_index)
                    .expect("matching cached entries can be reused");
            black_box(state);
        });
    });

    let (sparse_id, sparse_objects, sparse_entries) = sparse_tree();
    group.throughput(Throughput::Elements(sparse_entries));
    group.bench_function("sparse 10k directories", |b| {
        b.iter(|| {
            let state = State::from_tree(&sparse_id, &sparse_objects, Default::default()).expect("tree can be read");
            black_box(state);
        });
    });
}

criterion_group!(benches, from_tree);
criterion_main!(benches);

fn flat_tree() -> (gix_hash::ObjectId, MemoryDb, u64) {
    let objects = memory_db();
    let mut editor = gix_object::tree::Editor::new(Tree::default(), &gix_object::find::Never, gix_hash::Kind::Sha1);
    let id = repeated_id(b'a');
    const FILE_COUNT: u64 = 10_000;
    for idx in 0..FILE_COUNT {
        editor
            .upsert([BString::from(format!("file-{idx:05}"))], EntryKind::Blob, id)
            .expect("valid path");
    }
    let id = editor.write(|tree| objects.write(tree)).expect("tree can be written");
    (id, objects, FILE_COUNT)
}

fn wide_deep_tree() -> (gix_hash::ObjectId, MemoryDb, u64) {
    let objects = memory_db();
    let mut editor = gix_object::tree::Editor::new(Tree::default(), &gix_object::find::Never, gix_hash::Kind::Sha1);
    let id = repeated_id(b'a');
    const DIR_COUNT: u64 = 100;
    const FILE_COUNT: u64 = 100;
    for dir_idx in 0..DIR_COUNT {
        for file_idx in 0..FILE_COUNT {
            editor
                .upsert(
                    [
                        BString::from(format!("dir-{dir_idx:03}")),
                        BString::from(format!("file-{file_idx:03}")),
                    ],
                    EntryKind::Blob,
                    id,
                )
                .expect("valid path");
        }
    }
    let id = editor.write(|tree| objects.write(tree)).expect("tree can be written");
    (id, objects, DIR_COUNT * FILE_COUNT)
}

fn sparse_tree() -> (gix_hash::ObjectId, MemoryDb, u64) {
    let objects = memory_db();
    let empty_tree_id = objects.write(&Tree::default()).expect("empty tree can be written");
    let mut editor = gix_object::tree::Editor::new(Tree::default(), &gix_object::find::Never, gix_hash::Kind::Sha1);
    const SPARSE_DIR_COUNT: u64 = 10_000;
    for idx in 0..SPARSE_DIR_COUNT {
        editor
            .upsert(
                [BString::from(format!("sparse-{idx:05}"))],
                EntryKind::Tree,
                empty_tree_id,
            )
            .expect("valid path");
    }
    let id = editor.write(|tree| objects.write(tree)).expect("tree can be written");
    (id, objects, SPARSE_DIR_COUNT)
}

type MemoryDb = gix_odb::memory::Proxy<gix_object::find::Never>;

fn memory_db() -> MemoryDb {
    gix_odb::memory::Proxy::new(gix_object::find::Never, gix_hash::Kind::Sha1)
}

fn repeated_id(byte: u8) -> gix_hash::ObjectId {
    gix_hash::ObjectId::from_hex(&vec![byte; gix_hash::Kind::Sha1.len_in_hex()]).expect("valid hex")
}
