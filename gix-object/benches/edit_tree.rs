use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use gix_hash::ObjectId;
use gix_hashtable::hash_map::Entry;
use gix_object::tree::EntryKind;
use gix_object::{tree, Tree, WriteTo};
use std::cell::RefCell;
use std::rc::Rc;

fn create_new_tree_add_and_remove(c: &mut Criterion) {
    let (storage, mut write) = new_inmemory_writes();
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

    let odb = StorageOdb(storage);
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

type TreeStore = Rc<RefCell<gix_hashtable::HashMap<ObjectId, Tree>>>;

fn new_inmemory_writes() -> (TreeStore, impl FnMut(&Tree) -> Result<ObjectId, std::io::Error>) {
    let store = TreeStore::default();
    let write_tree = {
        let store = store.clone();
        let mut buf = Vec::with_capacity(512);
        move |tree: &Tree| {
            buf.clear();
            tree.write_to(&mut buf)?;
            let header = gix_object::encode::loose_header(gix_object::Kind::Tree, buf.len() as u64);
            let mut hasher = gix_features::hash::hasher(gix_hash::Kind::Sha1);
            hasher.update(&header);
            hasher.update(&buf);
            let id = hasher.digest().into();
            let mut borrowed = store.borrow_mut();
            match borrowed.entry(id) {
                Entry::Occupied(_) => {}
                Entry::Vacant(e) => {
                    e.insert(tree.clone());
                }
            }
            Ok(id)
        }
    };
    (store, write_tree)
}

struct StorageOdb(TreeStore);

impl gix_object::Find for StorageOdb {
    fn try_find<'a>(
        &self,
        id: &gix_hash::oid,
        buffer: &'a mut Vec<u8>,
    ) -> Result<Option<gix_object::Data<'a>>, gix_object::find::Error> {
        let borrow = self.0.borrow();
        match borrow.get(id) {
            None => Ok(None),
            Some(tree) => {
                buffer.clear();
                tree.write_to(buffer).expect("valid trees can always be serialized");
                Ok(Some(gix_object::Data {
                    kind: gix_object::Kind::Tree,
                    data: &*buffer,
                }))
            }
        }
    }
}

fn any_blob() -> ObjectId {
    ObjectId::from_hex("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".as_bytes()).unwrap()
}
