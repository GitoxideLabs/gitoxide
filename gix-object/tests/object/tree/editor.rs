use gix_object::tree::{Entry, EntryKind};
use gix_object::Tree;

#[test]
fn from_empty_cursor() -> crate::Result {
    let (storage, mut write, num_writes_and_clear) = new_inmemory_writes();
    let odb = StorageOdb::new(storage.clone());
    let mut edit = gix_object::tree::Editor::new(Tree::default(), &odb, gix_hash::Kind::Sha1);

    edit.upsert(Some("root-file"), EntryKind::Blob, any_blob())?.upsert(
        ["nested", "from", "root"],
        EntryKind::BlobExecutable,
        any_blob(),
    )?;
    let cursor_path = ["some", "deeply", "nested", "path"];
    let mut cursor = edit.cursor_at(cursor_path)?;
    let actual = cursor
        .upsert(Some("file"), EntryKind::Blob, any_blob())?
        .upsert(Some("empty-dir-via-cursor"), EntryKind::Tree, empty_tree())?
        .upsert(["with-subdir", "dir", "file"], EntryKind::Blob, any_blob())?
        .upsert(["with-subdir2", "dir", "file"], EntryKind::Blob, any_blob())?
        .remove(Some("file"))?
        .remove(["with-subdir", "dir", "file"])?
        .remove(Some("with-subdir2"))?
        .remove(Some("with-subdir2"))?
        .write(&mut write)?;
    assert_eq!(
        display_tree(actual, &storage),
        "e2339a3f62e2f3fc54a406739a62a4173ee3b5ac
└── empty-dir-via-cursor (empty)
",
        "only one item is left in the tree, which also keeps it alive"
    );
    assert_eq!(num_writes_and_clear(), 1, "root tree");
    assert_eq!(
        cursor.get(None::<&str>),
        None,
        "the 'root' can't be obtained, no entry exists for it, ever"
    );
    assert_eq!(
        cursor.get(Some("empty-dir-via-cursor")),
        Some(&Entry {
            mode: EntryKind::Tree.into(),
            filename: "empty-dir-via-cursor".into(),
            oid: gix_hash::ObjectId::empty_tree(gix_hash::Kind::Sha1),
        }),
    );

    let actual = edit.write(&mut write)?;
    assert_eq!(
        display_tree(actual, &storage),
        "76e0729de84047d19711d90cfcbb4e60bb432682
├── nested
│   └── from
│       └── root bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100755
├── root-file bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100644
└── some
    └── deeply
        └── nested
            └── path
                └── empty-dir-via-cursor (empty)
"
    );

    let mut cursor = edit.cursor_at(cursor_path)?;
    let actual = cursor.remove(Some("empty-dir-via-cursor"))?.write(&mut write)?;
    assert_eq!(actual, empty_tree(), "it keeps the empty tree like the editor would");
    assert_eq!(
        edit.get(["some", "deeply", "nested", "path"]),
        Some(&Entry {
            mode: EntryKind::Tree.into(),
            filename: "path".into(),
            oid: gix_hash::Kind::Sha1.null(),
        }),
        "the directory leading to the removed one is still present"
    );
    assert_eq!(
        edit.get(["some", "deeply", "nested", "path", "empty-dir-via-cursor"]),
        None,
        "but the removed entry is indee removed"
    );

    let actual = edit.write(&mut write)?;
    assert_eq!(
        display_tree(actual, &storage),
        "6cc592046dcaac06d3c619b4892d9ac78738fb5d
├── nested
│   └── from
│       └── root bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100755
└── root-file bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100644
",
        "now the editor naturally prunes all empty trees thus far, removing the cursor root"
    );

    let mut cursor = edit.cursor_at(cursor_path)?;
    let actual = cursor
        .upsert(Some("root-file"), EntryKind::BlobExecutable, any_blob())?
        .upsert(["nested", "from"], EntryKind::BlobExecutable, any_blob())?
        .write(&mut write)?;

    assert_eq!(
        display_tree(actual, &storage),
        "4580ae6d4c22b407cee521d7575e69708ff980a1
├── nested
│   └── from bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100755
└── root-file bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100755
",
        "it is able to write the sub-tree, even though names from the top-level tree are re-used"
    );

    let actual = edit.write(&mut write)?;
    assert_eq!(
        display_tree(actual, &storage),
        "8febb45a1c34e405d70a7ae059d57abdd8254063
├── nested
│   └── from
│       └── root bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100755
├── root-file bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100644
└── some
    └── deeply
        └── nested
            └── path
                ├── nested
                │   └── from bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100755
                └── root-file bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100755
",
        "it places the subtree exactly where it's expected"
    );
    Ok(())
}

#[test]
fn from_existing_cursor() -> crate::Result {
    let (storage, mut write, num_writes_and_clear) = new_inmemory_writes();
    let odb = StorageOdb::new_with_odb(storage.clone(), tree_odb()?);
    let root_tree_id = hex_to_id("ff7e7d2aecae1c3fb15054b289a4c58aa65b8646");
    let root_tree = find_tree(&odb, root_tree_id)?;
    odb.access_count_and_clear();
    let mut edit = gix_object::tree::Editor::new(root_tree.clone(), &odb, gix_hash::Kind::Sha1);

    let mut cursor = edit.to_cursor();
    let actual = cursor
        .remove(Some("bin"))?
        .remove(Some("bin.d"))?
        .remove(Some("file.to"))?
        .remove(Some("file.toml"))?
        .remove(Some("file.toml.bin"))?
        .upsert(["some", "nested", "file"], EntryKind::Blob, any_blob())?
        .write(&mut write)?;
    assert_eq!(
        num_writes_and_clear(),
        1 + 2,
        "just the altered root tree, and two of trees towards `some/tested/file`"
    );
    assert_eq!(
        display_tree_with_odb(actual, &storage, &odb),
        "10364deb76aeee372eb486c1216dca2a98dbd379
├── file
│   └── a e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
├── file0 e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
└── some
    └── nested
        └── file bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100644
",
        "a cursor at '' is equivalent to 'as_cursor()', or the editor itself"
    );
    let mut cursor = edit.cursor_at(["some", "nested"])?;
    let actual = cursor
        .upsert(Some("hello-from-cursor"), EntryKind::Blob, any_blob())?
        .remove(Some("file"))?
        .write(&mut write)?;
    assert_eq!(
        display_tree_with_odb(actual, &storage, &odb),
        "0f090b7c09c94f7895d0d8ce63c1da7693c026b3
└── hello-from-cursor bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100644
"
    );

    let mut cursor = edit.set_root(root_tree).to_cursor();
    let actual = cursor
        .remove(Some("bin"))?
        .remove(Some("bin.d"))?
        .remove(Some("file.to"))?
        .remove(Some("file.toml"))?
        .remove(Some("file.toml.bin"))?
        .upsert(["some", "nested", "file"], EntryKind::Blob, any_blob())?
        .write(&mut write)?;
    assert_eq!(
        display_tree_with_odb(actual, &storage, &odb),
        "10364deb76aeee372eb486c1216dca2a98dbd379
├── file
│   └── a e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
├── file0 e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
└── some
    └── nested
        └── file bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100644
",
        "this cursor is the same as the editor"
    );
    let actual = cursor.remove(["some", "nested", "file"])?.write(&mut write)?;
    assert_eq!(
        display_tree_with_odb(actual, &storage, &odb),
        "6e2806ab1e4d4ae2c9d24ce113a9bb54f8eff97b
├── file
│   └── a e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
└── file0 e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
",
        "it's possible to delete a deeply nested file"
    );
    Ok(())
}

#[test]
fn from_empty_removal() -> crate::Result {
    let (storage, mut write, num_writes_and_clear) = new_inmemory_writes();
    let odb = StorageOdb::new(storage.clone());
    let mut edit = gix_object::tree::Editor::new(Tree::default(), &odb, gix_hash::Kind::Sha1);

    let actual = edit
        .remove(Some("non-existing"))?
        .remove(["still", "does", "not", "exist"])?
        .write(&mut write)?;
    assert_eq!(actual, empty_tree(), "nothing was actually done");
    assert_eq!(num_writes_and_clear(), 1, "it has to write the empty tree though");
    assert_eq!(storage.borrow().len(), 1, "the empty tree ends up in storage, too");
    assert_eq!(odb.access_count_and_clear(), 0);

    let actual = edit
        .upsert(Some("file"), EntryKind::Blob, any_blob())?
        .upsert(Some("empty-dir"), EntryKind::Tree, empty_tree())?
        .upsert(["with-subdir", "dir", "file"], EntryKind::Blob, any_blob())?
        .upsert(["with-subdir2", "dir", "file"], EntryKind::Blob, any_blob())?
        .remove(Some("file"))?
        .remove(Some("empty-dir"))?
        .remove(Some("with-subdir"))?
        .remove(["with-subdir2", "dir"])?
        .remove(Some("with-subdir2"))?
        .write(&mut write)?;
    assert_eq!(actual, empty_tree(), "nothing was actually done");
    assert_eq!(num_writes_and_clear(), 1, "still nothing to write but the empty tree");
    assert_eq!(odb.access_count_and_clear(), 0);

    let actual = edit
        .upsert(Some("file"), EntryKind::Blob, any_blob())?
        .upsert(Some("empty-dir"), EntryKind::Tree, empty_tree())?
        .upsert(["with-subdir", "dir", "file"], EntryKind::Blob, any_blob())?
        .upsert(["with-subdir2", "dir", "file"], EntryKind::Blob, any_blob())?
        .write(&mut write)?;
    assert_eq!(
        display_tree(actual, &storage),
        "9e608223b4cbc733abd20fb6d5b8ea80b074be17
├── empty-dir (empty)
├── file bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100644
├── with-subdir
│   └── dir
│       └── file bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100644
└── with-subdir2
    └── dir
        └── file bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100644
"
    );
    assert_eq!(num_writes_and_clear(), 5);
    assert_eq!(odb.access_count_and_clear(), 0);

    let actual = edit
        .remove(Some("file"))?
        .remove(Some("empty-dir"))?
        .remove(Some("with-subdir"))?
        .remove(["with-subdir2", "dir"])?
        .remove(Some("with-subdir2"))?
        .write(&mut write)?;
    assert_eq!(actual, empty_tree(), "everything was removed, leaving nothing");
    assert_eq!(num_writes_and_clear(), 1, "only the empty tree to write");
    assert_eq!(
        odb.access_count_and_clear(),
        1,
        "has to get `with-subdir2` to remove child-entry"
    );

    let actual = edit
        .upsert(["with-subdir", "file"], EntryKind::Blob, any_blob())?
        .upsert(["with-subdir", "dir", "file"], EntryKind::Blob, any_blob())?
        .remove(["with-subdir", "dir"])?
        .write(&mut write)?;
    assert_eq!(
        display_tree(actual, &storage),
        "da2277079f7e9e5a012c9a03d1aac710866ee2c5
└── with-subdir
    └── file bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100644
",
        "only one file remains, empty dirs are removed automatically"
    );
    assert_eq!(num_writes_and_clear(), 1 + 1, "root and one subtree");
    assert_eq!(storage.borrow().len(), 1 + 4, "empty tree and 4 unique trees");
    assert_eq!(odb.access_count_and_clear(), 0);

    Ok(())
}

#[test]
fn from_existing_remove() -> crate::Result {
    let (storage, mut write, num_writes_and_clear) = new_inmemory_writes();
    let odb = StorageOdb::new_with_odb(storage.clone(), tree_odb()?);
    let root_tree_id = hex_to_id("ff7e7d2aecae1c3fb15054b289a4c58aa65b8646");
    let root_tree = find_tree(&odb, root_tree_id)?;
    odb.access_count_and_clear();
    let mut edit = gix_object::tree::Editor::new(root_tree.clone(), &odb, gix_hash::Kind::Sha1);

    let actual = edit
        .remove(["file"])?
        .remove(Some("does not exist"))?
        .remove(["also", "does", "not", "exist"])?
        .remove(Some("bin.d"))?
        .remove(Some("file.toml.bin"))?
        .remove(Some("file.0"))?
        .write(&mut write)?;
    assert_eq!(
        display_tree_with_odb(actual, &storage, &odb),
        "dfd0d048f8e852879ad8e1a6a9b810873de16a9c
├── bin e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
├── file.to e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
├── file.toml e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
└── file0 e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
"
    );
    assert_eq!(num_writes_and_clear(), 1, "only the root tree is written");
    assert_eq!(
        odb.access_count_and_clear(),
        0,
        "no sub-tree has to be queried for removal"
    );

    let actual = edit
        .remove(Some("bin"))?
        .remove(Some("file.to"))?
        .remove(Some("file.toml"))?
        .remove(Some("file0"))?
        .write(&mut write)?;
    assert_eq!(actual, empty_tree(), "nothing is left");
    assert_eq!(num_writes_and_clear(), 1, "only the empty tree is written");
    assert_eq!(odb.access_count_and_clear(), 0);

    let actual = edit.set_root(root_tree).remove(["file", "a"])?.write(&mut write)?;
    assert_eq!(num_writes_and_clear(), 1, "it writes the changed root-tree");
    assert_eq!(
        odb.access_count_and_clear(),
        1,
        "lookup `file` to delete its (only) content"
    );
    assert_eq!(
        display_tree_with_odb(actual, &storage, &odb),
        "38a66b78d1d5dd8daedf6188d2fafd98357a870c
├── bin e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
├── bin.d e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
├── file.to e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
├── file.toml e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
├── file.toml.bin e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
└── file0 e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
",
        "`file` is removed as it remains empty"
    );

    Ok(())
}

#[test]
fn from_empty_invalid_write() -> crate::Result {
    let (storage, mut write, _num_writes_and_clear) = new_inmemory_writes();
    let odb = StorageOdb::new(storage.clone());
    let mut edit = gix_object::tree::Editor::new(Tree::default(), &odb, gix_hash::Kind::Sha1);

    let actual = edit
        .upsert(["a", "\n"], EntryKind::Blob, any_blob())?
        .write(&mut write)
        .expect("no validation is performed");
    assert_eq!(
        display_tree(actual, &storage),
        "d23290ea39c284156731188dce62c17ac6b71bda
└── a
    └── \n bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100644
"
    );

    let err = edit
        .upsert(Some("with\0null"), EntryKind::Blob, any_blob())?
        .write(&mut write)
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        r#"Nullbytes are invalid in file paths as they are separators: "with\0null""#
    );

    let err = edit.upsert(Some(""), EntryKind::Blob, any_blob()).unwrap_err();
    let expected_msg = "Empty path components are not allowed";
    assert_eq!(err.to_string(), expected_msg);
    let err = edit
        .upsert(["fine", "", "previous is not fine"], EntryKind::Blob, any_blob())
        .unwrap_err();
    assert_eq!(err.to_string(), expected_msg);

    let actual = edit
        .remove(Some("a"))?
        .remove(Some("with\0null"))?
        .upsert(Some("works"), EntryKind::Blob, any_blob())?
        .write(&mut write)?;
    assert_eq!(
        display_tree(actual, &storage),
        "d5b913c39b06507c7c64adb16c268ce1102ef5c1
└── works bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100644
",
        "after removing invalid entries, it can write again"
    );
    Ok(())
}

#[test]
fn from_empty_add() -> crate::Result {
    let (storage, mut write, num_writes_and_clear) = new_inmemory_writes();
    let odb = StorageOdb::new(storage.clone());
    let mut edit = gix_object::tree::Editor::new(Tree::default(), &odb, gix_hash::Kind::Sha1);

    let actual = edit.write(&mut write).expect("no changes are fine");
    assert_eq!(actual, empty_tree(), "empty stays empty");
    assert_eq!(num_writes_and_clear(), 1, "the empty tree was written");
    assert_eq!(
        display_tree(actual, &storage),
        "4b825dc642cb6eb9a060e54bf8d69288fbee4904\n"
    );
    assert_eq!(odb.access_count_and_clear(), 0);
    assert_eq!(
        edit.get(None::<&str>),
        None,
        "the 'root' can't be obtained, no entry exists for it, ever"
    );

    let actual = edit
        .upsert(Some("hi"), EntryKind::Blob, gix_hash::Kind::Sha1.null())?
        .write(&mut write)
        .expect("effectively no changes are fine");
    assert_eq!(
        actual,
        empty_tree(),
        "null-ids are dropped automatically, they act as placeholders"
    );
    assert_eq!(num_writes_and_clear(), 1, "the empty tree was written, nothing new");
    assert_eq!(odb.access_count_and_clear(), 0);

    let actual = edit
        .upsert(["a", "b", "c"], EntryKind::Blob, gix_hash::Kind::Sha1.null())?
        .upsert(["a", "b", "d", "e"], EntryKind::Blob, gix_hash::Kind::Sha1.null())?
        .write(&mut write)
        .expect("effectively no changes are fine");
    assert_eq!(
        actual,
        empty_tree(),
        "null-ids are dropped automatically, recursively, they act as placeholders"
    );
    assert_eq!(
        num_writes_and_clear(),
        1,
        "the empty tree was written as root, nothing new"
    );
    assert_eq!(storage.borrow().len(), 1, "still nothing but empty trees");
    assert_eq!(odb.access_count_and_clear(), 0);

    edit.upsert(["a", "b"], EntryKind::Tree, empty_tree())?
        .upsert(["a", "b", "c"], EntryKind::Tree, empty_tree())?
        .upsert(["a", "b", "d", "e"], EntryKind::Tree, empty_tree())?;
    assert_eq!(
        edit.get(["a", "b"]),
        Some(&Entry {
            mode: EntryKind::Tree.into(),
            filename: "b".into(),
            oid: hex_to_id("4b825dc642cb6eb9a060e54bf8d69288fbee4904"),
        }),
        "before writing, entries are still present, just like they were written"
    );
    assert_eq!(
        edit.get(["a", "b", "c"]),
        Some(&Entry {
            mode: EntryKind::Tree.into(),
            filename: "c".into(),
            oid: gix_hash::ObjectId::empty_tree(gix_hash::Kind::Sha1),
        }),
    );

    let actual = edit.write(&mut write).expect("it's OK to write empty trees");
    assert_eq!(
        display_tree(actual, &storage),
        "bf91a94ae659ac8a9da70d26acf42df1a36adb6e
└── a
    └── b
        ├── c (empty)
        └── d
            └── e (empty)
",
        "one can write through trees, and empty trees are also fine"
    );
    assert_eq!(num_writes_and_clear(), 4, "it wrote the trees it needed to write");
    assert_eq!(odb.access_count_and_clear(), 0);
    assert_eq!(edit.get(["a", "b"]), None, "nothing exists here");
    assert_eq!(
        edit.get(Some("a")),
        Some(&Entry {
            mode: EntryKind::Tree.into(),
            filename: "a".into(),
            oid: hex_to_id("850bf83c26003cb0541318718bc9217c4a5bde6d"),
        }),
        "but the top-level tree is still available and can yield its entries, as written with proper ids"
    );

    let actual = edit
        .upsert(["a"], EntryKind::Blob, any_blob())?
        .upsert(["a", "b"], EntryKind::Blob, any_blob())?
        .upsert(["a", "b", "c"], EntryKind::BlobExecutable, any_blob())?
        .upsert(["x", "z"], EntryKind::Blob, any_blob())?
        .write(&mut write)
        .expect("writing made-up blobs is fine");
    assert_eq!(
        display_tree(actual, &storage),
        "7534352b2f718388b6c6d0f70aaf12399f38258c
├── a
│   └── b
│       └── c bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100755
└── x
    └── z bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100644
",
        "it's possible to write through previously added blobs"
    );
    assert_eq!(num_writes_and_clear(), 4);
    assert_eq!(odb.access_count_and_clear(), 0);

    let actual = edit.upsert(["x"], EntryKind::Blob, any_blob())?.write(&mut write)?;
    assert_eq!(
        display_tree(actual, &storage),
        "c2bb1616d4db21d99a30a1219d7d47e969f42e26
├── a
│   └── b
│       └── c bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100755
└── x bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100644
"
    );
    assert_eq!(num_writes_and_clear(), 1, "just the root tree changed");
    assert_eq!(odb.access_count_and_clear(), 0);

    let prev_tree = actual;
    let actual = edit
        .upsert(["a", "b", "c"], EntryKind::BlobExecutable, any_blob())?
        .write(&mut write)?;
    assert_eq!(actual, prev_tree, "writing the same path again is a no-op");
    assert_eq!(
        num_writes_and_clear(),
        3,
        "it still rewrites all paths that (potentially) changed. \
         There is no actual change tracking as no-changes aren't the default case for an editor"
    );
    assert_eq!(odb.access_count_and_clear(), 2, "`a` and `a/b`");

    let actual = edit
        .upsert(["a", "b", "c"], EntryKind::Blob, any_blob())?
        .upsert(["a"], EntryKind::Blob, any_blob())?
        .write(&mut write)
        .expect("we can turn overwrite a newly added tree (at 'a/') with a blob");
    assert_eq!(
        display_tree(actual, &storage),
        "7c66d7d5cbfdbbb37085ff5c8c6e5b048727cf88
├── a bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100644
└── x bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100644
",
        "now a tree was once again changed into a blob"
    );
    assert_eq!(num_writes_and_clear(), 1, "only the root-tree changes effectively");
    assert_eq!(odb.access_count_and_clear(), 2, "`a` and `a/b`");

    let actual = edit
        .set_root(Tree::default())
        .upsert(["a", "b", "c"], EntryKind::Blob, any_blob())?
        .upsert(["a"], EntryKind::BlobExecutable, any_blob())?
        .write(&mut write)?;
    assert_eq!(
        display_tree(actual, &storage),
        "f7b85940c3afa596829cacf98e98ff8bfd7c68ed
└── a bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100755
",
        "now the root is back to a well-known state, so edits are more intuitive"
    );
    assert_eq!(
        num_writes_and_clear(),
        1,
        "still, only the root-tree changes effectively"
    );
    assert_eq!(odb.access_count_and_clear(), 0);

    let actual = edit
        .upsert(["a", "b"], EntryKind::Tree, empty_tree())?
        .upsert(["a", "b", "c"], EntryKind::BlobExecutable, any_blob())?
        // .upsert(["a", "b", "d"], EntryKind::Blob, any_blob())?
        .write(&mut write)?;
    assert_eq!(
        display_tree(actual, &storage),
        "d8d3f558776965f70452625b72363234f517b290
└── a
    └── b
        └── c bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100755
",
        "the intermediate tree is rewritten to be suitable to hold the blob"
    );
    assert_eq!(num_writes_and_clear(), 3, "root, and two child-trees");
    assert_eq!(odb.access_count_and_clear(), 0);

    Ok(())
}

#[test]
fn from_existing_add() -> crate::Result {
    let (storage, mut write, num_writes_and_clear) = new_inmemory_writes();
    let odb = StorageOdb::new_with_odb(storage.clone(), tree_odb()?);
    let root_tree_id = hex_to_id("ff7e7d2aecae1c3fb15054b289a4c58aa65b8646");
    let root_tree = find_tree(&odb, root_tree_id)?;
    odb.access_count_and_clear();
    let mut edit = gix_object::tree::Editor::new(root_tree.clone(), &odb, gix_hash::Kind::Sha1);
    assert!(edit.get(["bin"]).is_some(), "the root is immediately available");

    let actual = edit.write(&mut write).expect("no changes are fine");
    assert_eq!(actual, root_tree_id, "it rewrites the same tree");
    assert_eq!(odb.access_count_and_clear(), 0);
    assert_eq!(
        display_tree_with_odb(actual, &storage, &odb),
        "ff7e7d2aecae1c3fb15054b289a4c58aa65b8646
├── bin e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
├── bin.d e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
├── file.to e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
├── file.toml e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
├── file.toml.bin e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
├── file
│   └── a e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
└── file0 e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
"
    );
    assert_eq!(
        num_writes_and_clear(),
        1,
        "only the root is written - there is no change tracking"
    );

    let actual = edit
        .upsert(["file", "hi"], EntryKind::Blob, gix_hash::Kind::Sha1.null())?
        .write(&mut write)
        .expect("effectively no changes are fine");
    assert_eq!(
        actual, root_tree_id,
        "null-ids are dropped automatically, they act as placeholders, ultimately the tree is not changed"
    );
    assert_eq!(
        storage.borrow().len(),
        2,
        "it writes two trees, even though none is new"
    );
    assert_eq!(num_writes_and_clear(), 2, "the write-count reflects that");

    odb.access_count_and_clear();
    let actual = edit
        .upsert(["a", "b", "c"], EntryKind::Blob, gix_hash::Kind::Sha1.null())?
        .upsert(["a", "b", "d", "e"], EntryKind::Blob, gix_hash::Kind::Sha1.null())?
        .write(&mut write)
        .expect("effectively no changes are fine");
    assert_eq!(
        actual, root_tree_id,
        "null-ids are dropped automatically, recursively, and empty intermediate trees are removed as well"
    );
    assert_eq!(storage.borrow().len(), 2, "still the same amount of trees");
    assert_eq!(
        num_writes_and_clear(),
        1,
        "but only the root-tree is written (with nulls pruned)"
    );
    assert_eq!(odb.access_count_and_clear(), 0);

    odb.access_count_and_clear();
    let actual = edit
        .upsert(["bin", "b"], EntryKind::Tree, empty_tree())?
        .upsert(["bin", "b", "c"], EntryKind::Tree, empty_tree())?
        .upsert(["a", "b", "d", "e"], EntryKind::Tree, empty_tree())?
        .write(&mut write)
        .expect("it's OK to write empty leaf-trees");
    assert_eq!(
        odb.access_count_and_clear(),
        0,
        "we write through blobs, and thus create trees on the fly"
    );
    assert_eq!(
        display_tree_with_odb(actual, &storage, &odb),
        "a57b69b5c216b8417d332e64a4f3014eb9962099
├── a
│   └── b
│       └── d
│           └── e (empty)
├── bin.d e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
├── bin
│   └── b
│       └── c (empty)
├── file.to e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
├── file.toml e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
├── file.toml.bin e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
├── file
│   └── a e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
└── file0 e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
",
        "one can write through trees and blobs, and empty leaf trees are also fine"
    );
    assert_eq!(
        num_writes_and_clear(),
        1 + 2 + 3,
        "each changed tree is written: root, the two subtrees"
    );

    odb.access_count_and_clear();
    let actual = edit
        .upsert(["a", "b", "c"], EntryKind::Blob, any_blob())?
        .upsert(["a", "b"], EntryKind::Blob, any_blob())?
        .upsert(["file"], EntryKind::BlobExecutable, any_blob())?
        .write(&mut write)?;
    assert_eq!(odb.access_count_and_clear(), 2, "`a` and `a/b`");
    assert_eq!(
        display_tree_with_odb(actual, &storage, &odb),
        "25a0bd46bc61342e17f03fafcc029e9db52d4c64
├── a
│   └── b bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100644
├── bin.d e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
├── bin
│   └── b
│       └── c (empty)
├── file bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100755
├── file.to e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
├── file.toml e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
├── file.toml.bin e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
└── file0 e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
",
        "it properly sorts entries after type-changes"
    );
    assert_eq!(num_writes_and_clear(), 1 + 1, "the root and one subtree");

    odb.access_count_and_clear();
    let actual = edit
        .set_root(root_tree)
        .upsert(["file", "subdir", "exe"], EntryKind::BlobExecutable, any_blob())?
        .write(&mut write)?;
    assert_eq!(
        odb.access_count_and_clear(),
        1,
        "`file` only, everything else is inserted"
    );
    assert_eq!(
        display_tree_with_odb(actual, &storage, &odb),
        "6a5115bfb88b8303b837854199b90232621f8535
├── bin e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
├── bin.d e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
├── file.to e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
├── file.toml e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
├── file.toml.bin e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
├── file
│   ├── a e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
│   └── subdir
│       └── exe bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.100755
└── file0 e69de29bb2d1d6434b8b29ae775ad8c2e48c5391.100644
",
        "now the root is back to a well-known state"
    );
    assert_eq!(num_writes_and_clear(), 1 + 2, "the root and one subtree with directory");
    Ok(())
}

mod utils {
    use crate::hex_to_id;
    use bstr::{BStr, ByteSlice};
    use gix_hash::ObjectId;
    use gix_object::{Tree, WriteTo};
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    type TreeStore = Rc<RefCell<gix_hashtable::HashMap<ObjectId, Tree>>>;
    pub(super) struct StorageOdb(TreeStore, Option<gix_odb::Handle>, Cell<usize>);

    pub(super) fn tree_odb() -> gix_testtools::Result<gix_odb::Handle> {
        let root = gix_testtools::scripted_fixture_read_only("make_trees.sh")?;
        Ok(gix_odb::at(root.join(".git/objects"))?)
    }

    pub(super) fn find_tree(odb: &impl gix_object::FindExt, id: ObjectId) -> gix_testtools::Result<Tree> {
        let mut buf = Vec::new();
        Ok(odb.find_tree(&id, &mut buf)?.into())
    }

    pub(super) fn new_inmemory_writes() -> (
        TreeStore,
        impl FnMut(&Tree) -> Result<ObjectId, gix_hash::io::Error>,
        impl Fn() -> usize,
    ) {
        let store = TreeStore::default();
        let num_writes = Rc::new(Cell::new(0_usize));
        let write_tree = {
            let store = store.clone();
            let num_writes = num_writes.clone();
            let mut buf = Vec::with_capacity(512);
            move |tree: &Tree| {
                buf.clear();
                tree.write_to(&mut buf)?;
                let id = gix_object::compute_hash(gix_hash::Kind::Sha1, gix_object::Kind::Tree, &buf)?;
                store.borrow_mut().insert(id, tree.clone());
                let old = num_writes.get();
                num_writes.set(old + 1);
                Ok(id)
            }
        };
        let obtain_num_writes = {
            let c = num_writes.clone();
            move || {
                let res = c.get();
                c.set(0);
                res
            }
        };
        (store, write_tree, obtain_num_writes)
    }

    impl StorageOdb {
        pub fn new(storage: TreeStore) -> Self {
            Self(storage, None, Cell::new(0))
        }
        pub fn new_with_odb(storage: TreeStore, odb: gix_odb::Handle) -> Self {
            Self(storage, Some(odb), Cell::new(0))
        }
        pub fn access_count_and_clear(&self) -> usize {
            let res = self.2.get();
            self.2.set(0);
            res
        }
    }

    impl gix_object::Find for StorageOdb {
        fn try_find<'a>(
            &self,
            id: &gix_hash::oid,
            buffer: &'a mut Vec<u8>,
        ) -> Result<Option<gix_object::Data<'a>>, gix_object::find::Error> {
            let borrow = self.0.borrow();
            let old = self.2.get();
            self.2.set(old + 1);
            match borrow.get(id) {
                None => self.1.as_ref().map_or(Ok(None), |odb| odb.try_find(id, buffer)),
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

    fn display_tree_recursive(
        tree_id: ObjectId,
        storage: &TreeStore,
        odb: Option<&dyn gix_object::FindExt>,
        name: Option<&BStr>,
        buf: &mut Vec<u8>,
    ) -> termtree::Tree<String> {
        let mut tree_storage = None;
        let borrow = storage.borrow();
        let tree = borrow
            .get(&tree_id)
            .or_else(|| {
                if tree_id.is_empty_tree() {
                    tree_storage = Some(Tree::default());
                    tree_storage.as_ref()
                } else {
                    odb.and_then(|odb| {
                        tree_storage = odb.find_tree(&tree_id, buf).map(Into::into).ok();
                        tree_storage.as_ref()
                    })
                }
            })
            .unwrap_or_else(|| panic!("tree {tree_id} is always present"));

        let mut termtree = termtree::Tree::new(if let Some(name) = name {
            if tree.entries.is_empty() {
                format!("{name} (empty)")
            } else {
                name.to_string()
            }
        } else {
            tree_id.to_string()
        });

        for entry in &tree.entries {
            if entry.mode.is_tree() {
                termtree.push(display_tree_recursive(
                    entry.oid,
                    storage,
                    odb,
                    Some(entry.filename.as_bstr()),
                    buf,
                ));
            } else {
                termtree.push(format!(
                    "{} {}.{}",
                    entry.filename,
                    entry.oid,
                    entry.mode.kind().as_octal_str()
                ));
            }
        }
        termtree
    }

    pub(super) fn display_tree(tree_id: ObjectId, storage: &TreeStore) -> String {
        let mut buf = Vec::new();
        display_tree_recursive(tree_id, storage, None, None, &mut buf).to_string()
    }

    pub(super) fn display_tree_with_odb(
        tree_id: ObjectId,
        storage: &TreeStore,
        odb: &impl gix_object::FindExt,
    ) -> String {
        let mut buf = Vec::new();
        display_tree_recursive(tree_id, storage, Some(odb), None, &mut buf).to_string()
    }

    pub(super) fn empty_tree() -> ObjectId {
        ObjectId::empty_tree(gix_hash::Kind::Sha1)
    }

    pub(super) fn any_blob() -> ObjectId {
        hex_to_id("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
    }
}
use crate::hex_to_id;
use utils::{
    any_blob, display_tree, display_tree_with_odb, empty_tree, find_tree, new_inmemory_writes, tree_odb, StorageOdb,
};
