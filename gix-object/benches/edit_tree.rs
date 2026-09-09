use std::{hint::black_box, rc::Rc};

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use gix_hash::ObjectId;
use gix_object::{Tree, Write, tree, tree::EntryKind};

fn create_new_tree_add_and_remove(c: &mut Criterion) {
    let (odb, mut write) = new_inmemory_writes();
    let mut editor = tree::Editor::new(Tree::default(), &gix_object::find::Never, gix_hash::Kind::Sha1);
    let mut group = c.benchmark_group("editor");
    let small_throughput = Throughput::Elements((1 + 2 + 4) + 3);
    group.throughput(small_throughput.clone());
    group.bench_function("small tree (empty -> full -> empty)", |b| {
        b.iter(|| {
            let tree_id = editor
                .upsert(Some("file"), EntryKind::Blob, any_blob())
                .unwrap()
                .upsert(["dir", "file"], EntryKind::Blob, any_blob())
                .unwrap()
                .upsert(["more", "deeply", "nested", "file"], EntryKind::Blob, any_blob())
                .unwrap()
                .write(&mut write)
                .unwrap();
            black_box(tree_id);
            let actual = editor
                .remove(Some("file"))
                .unwrap()
                .remove(Some("dir"))
                .unwrap()
                .remove(Some("more"))
                .unwrap()
                .write(&mut write)
                .unwrap();
            assert_eq!(actual, gix_hash::ObjectId::empty_tree(gix_hash::Kind::Sha1));
        });
    });

    let mut editor = tree::Editor::new(Tree::default(), &odb, gix_hash::Kind::Sha1);
    let prefixed_throughput = Throughput::Elements((1 + 2 + 4) + 6 * 3 + (3 + 6 * 3));
    group.throughput(prefixed_throughput.clone());
    group.bench_function("deeply nested tree (empty -> full -> empty)", |b| {
        b.iter(|| {
            let tree_id = editor
                .upsert(["a", "b", "c", "d", "e", "f", "file"], EntryKind::Blob, any_blob())
                .unwrap()
                .upsert(
                    ["a", "b", "c", "d", "e", "f", "dir", "file"],
                    EntryKind::Blob,
                    any_blob(),
                )
                .unwrap()
                .upsert(
                    ["a", "b", "c", "d", "e", "f", "more", "deeply", "nested", "file"],
                    EntryKind::Blob,
                    any_blob(),
                )
                .unwrap()
                .write(&mut write)
                .unwrap();
            black_box(tree_id);
            let tree_id = editor
                .remove(["a", "b", "c", "d", "e", "f", "file"])
                .unwrap()
                .remove(["a", "b", "c", "d", "e", "f", "dir"])
                .unwrap()
                .remove(["a", "b", "c", "d", "e", "f", "more"])
                .unwrap()
                .write(&mut write)
                .unwrap();
            black_box(tree_id);
        });
    });

    drop(group);
    let mut group = c.benchmark_group("cursor");
    group.throughput(small_throughput);
    group.bench_function("small tree (empty -> full -> empty)", |b| {
        let mut editor = editor.to_cursor();
        b.iter(|| {
            let tree_id = editor
                .upsert(Some("file"), EntryKind::Blob, any_blob())
                .unwrap()
                .upsert(["dir", "file"], EntryKind::Blob, any_blob())
                .unwrap()
                .upsert(["more", "deeply", "nested", "file"], EntryKind::Blob, any_blob())
                .unwrap()
                .write(&mut write)
                .unwrap();
            black_box(tree_id);
            let actual = editor
                .remove(Some("file"))
                .unwrap()
                .remove(Some("dir"))
                .unwrap()
                .remove(Some("more"))
                .unwrap()
                .write(&mut write)
                .unwrap();
            assert_eq!(actual, gix_hash::ObjectId::empty_tree(gix_hash::Kind::Sha1));
        });
    });

    group.throughput(prefixed_throughput);
    group.bench_function("deeply nested tree (empty -> full -> empty)", |b| {
        let mut editor = editor.cursor_at(["a", "b", "c", "d", "e", "f"]).unwrap();
        b.iter(|| {
            let tree_id = editor
                .upsert(["file"], EntryKind::Blob, any_blob())
                .unwrap()
                .upsert(["dir", "file"], EntryKind::Blob, any_blob())
                .unwrap()
                .upsert(["more", "deeply", "nested", "file"], EntryKind::Blob, any_blob())
                .unwrap()
                .write(&mut write)
                .unwrap();
            black_box(tree_id);
            let actual = editor
                .remove(["file"])
                .unwrap()
                .remove(["dir"])
                .unwrap()
                .remove(["more"])
                .unwrap()
                .write(&mut write)
                .unwrap();
            assert_eq!(actual, gix_hash::ObjectId::empty_tree(gix_hash::Kind::Sha1));
        });
    });
}

criterion_group!(benches, create_new_tree_add_and_remove);
criterion_main!(benches);

type ObjectDb = Rc<gix_odb::memory::Proxy<gix_object::find::Never>>;

fn new_inmemory_writes() -> (ObjectDb, impl FnMut(&Tree) -> Result<ObjectId, gix_error::Exn>) {
    let odb = Rc::new(gix_odb::memory::Proxy::new(
        gix_object::find::Never,
        gix_hash::Kind::Sha1,
    ));
    let write_tree = {
        let odb = odb.clone();
        move |tree: &Tree| odb.write(tree)
    };
    (odb, write_tree)
}

fn any_blob() -> ObjectId {
    ObjectId::from_hex("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".as_bytes()).unwrap()
}
