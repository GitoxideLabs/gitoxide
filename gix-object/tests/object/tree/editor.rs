use gix_object::{
    Tree,
    tree::{Entry, EntryKind},
};

fn hash_kind() -> gix_hash::Kind {
    crate::fixture_hash_kind()
}

fn null_id() -> gix_hash::ObjectId {
    hash_kind().null()
}

#[test]
fn from_empty_cursor() -> crate::Result {
    let (storage, mut write, num_writes_and_clear) = new_inmemory_writes();
    let odb = StorageOdb::new(storage.clone());
    let mut edit = gix_object::tree::Editor::new(Tree::default(), &odb, hash_kind());

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
    insta::assert_snapshot!(
        crate::normalize_tree_snapshot(&display_tree(actual, &storage)),
        "only one item is left in the tree, which also keeps it alive",
        @r#"
        Oid(1)
        └── empty-dir-via-cursor (empty)
    "#
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
            oid: empty_tree(),
        }),
    );

    let actual = edit.write(&mut write)?;
    insta::assert_snapshot!(crate::normalize_tree_snapshot(&display_tree(actual, &storage)), @r#"
        Oid(1)
        ├── nested
        │   └── from
        │       └── root Oid(2).100755
        ├── root-file Oid(2).100644
        └── some
            └── deeply
                └── nested
                    └── path
                        └── empty-dir-via-cursor (empty)
    "#);

    let mut cursor = edit.cursor_at(cursor_path)?;
    let actual = cursor.remove(Some("empty-dir-via-cursor"))?.write(&mut write)?;
    assert_eq!(actual, empty_tree(), "it keeps the empty tree like the editor would");
    assert_eq!(
        edit.get(["some", "deeply", "nested", "path"]),
        Some(&Entry {
            mode: EntryKind::Tree.into(),
            filename: "path".into(),
            oid: null_id(),
        }),
        "the directory leading to the removed one is still present"
    );
    assert_eq!(
        edit.get(["some", "deeply", "nested", "path", "empty-dir-via-cursor"]),
        None,
        "but the removed entry is indee removed"
    );

    let actual = edit.write(&mut write)?;
    insta::assert_snapshot!(
        crate::normalize_tree_snapshot(&display_tree(actual, &storage)),
        "now the editor naturally prunes all empty trees thus far, removing the cursor root",
        @r#"
        Oid(1)
        ├── nested
        │   └── from
        │       └── root Oid(2).100755
        └── root-file Oid(2).100644
    "#
    );

    let mut cursor = edit.cursor_at(cursor_path)?;
    let actual = cursor
        .upsert(Some("root-file"), EntryKind::BlobExecutable, any_blob())?
        .upsert(["nested", "from"], EntryKind::BlobExecutable, any_blob())?
        .write(&mut write)?;

    insta::assert_snapshot!(
        crate::normalize_tree_snapshot(&display_tree(actual, &storage)),
        "it is able to write the sub-tree, even though names from the top-level tree are re-used",
        @r#"
        Oid(1)
        ├── nested
        │   └── from Oid(2).100755
        └── root-file Oid(2).100755
    "#
    );

    let actual = edit.write(&mut write)?;
    insta::assert_snapshot!(
        crate::normalize_tree_snapshot(&display_tree(actual, &storage)),
        "it places the subtree exactly where it's expected",
        @r#"
        Oid(1)
        ├── nested
        │   └── from
        │       └── root Oid(2).100755
        ├── root-file Oid(2).100644
        └── some
            └── deeply
                └── nested
                    └── path
                        ├── nested
                        │   └── from Oid(2).100755
                        └── root-file Oid(2).100755
    "#
    );
    Ok(())
}
#[test]
fn from_existing_cursor() -> crate::Result {
    let (storage, mut write, num_writes_and_clear) = new_inmemory_writes();
    let odb = StorageOdb::new_with_odb(storage.clone(), tree_odb()?);
    let root_tree_id = crate::generated_tree_root_id()?;
    let root_tree = find_tree(&odb, root_tree_id)?;
    odb.access_count_and_clear();
    let mut edit = gix_object::tree::Editor::new(root_tree.clone(), &odb, hash_kind());

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
    insta::assert_snapshot!(
        crate::normalize_tree_snapshot(&(display_tree_with_odb(actual, &storage, &odb))),
        "a cursor at '' is equivalent to 'as_cursor()', or the editor itself",
        @r#"
        Oid(1)
        ├── file
        │   └── a Oid(2).100644
        ├── file0 Oid(2).100644
        └── some
            └── nested
                └── file Oid(3).100644
    "#
    );
    let mut cursor = edit.cursor_at(["some", "nested"])?;
    let actual = cursor
        .upsert(Some("hello-from-cursor"), EntryKind::Blob, any_blob())?
        .remove(Some("file"))?
        .write(&mut write)?;
    insta::assert_snapshot!(crate::normalize_tree_snapshot(&(display_tree_with_odb(actual, &storage, &odb))), @r#"
        Oid(1)
        └── hello-from-cursor Oid(2).100644
    "#);

    let mut cursor = edit.set_root(root_tree).to_cursor();
    let actual = cursor
        .remove(Some("bin"))?
        .remove(Some("bin.d"))?
        .remove(Some("file.to"))?
        .remove(Some("file.toml"))?
        .remove(Some("file.toml.bin"))?
        .upsert(["some", "nested", "file"], EntryKind::Blob, any_blob())?
        .write(&mut write)?;
    insta::assert_snapshot!(
        crate::normalize_tree_snapshot(&(display_tree_with_odb(actual, &storage, &odb))),
        "this cursor is the same as the editor",
        @r#"
        Oid(1)
        ├── file
        │   └── a Oid(2).100644
        ├── file0 Oid(2).100644
        └── some
            └── nested
                └── file Oid(3).100644
    "#
    );
    let actual = cursor.remove(["some", "nested", "file"])?.write(&mut write)?;
    insta::assert_snapshot!(
        crate::normalize_tree_snapshot(&(display_tree_with_odb(actual, &storage, &odb))),
        "it's possible to delete a deeply nested file",
        @r#"
        Oid(1)
        ├── file
        │   └── a Oid(2).100644
        └── file0 Oid(2).100644
    "#
    );
    Ok(())
}
#[test]
fn from_empty_removal() -> crate::Result {
    let (storage, mut write, num_writes_and_clear) = new_inmemory_writes();
    let odb = StorageOdb::new(storage.clone());
    let mut edit = gix_object::tree::Editor::new(Tree::default(), &odb, hash_kind());

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
    insta::assert_snapshot!(crate::normalize_tree_snapshot(&display_tree(actual, &storage)), @r#"
        Oid(1)
        ├── empty-dir (empty)
        ├── file Oid(2).100644
        ├── with-subdir
        │   └── dir
        │       └── file Oid(2).100644
        └── with-subdir2
            └── dir
                └── file Oid(2).100644
    "#);
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
    insta::assert_snapshot!(
        crate::normalize_tree_snapshot(&display_tree(actual, &storage)),
        "only one file remains, empty dirs are removed automatically",
        @r#"
        Oid(1)
        └── with-subdir
            └── file Oid(2).100644
    "#
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
    let root_tree_id = crate::generated_tree_root_id()?;
    let root_tree = find_tree(&odb, root_tree_id)?;
    odb.access_count_and_clear();
    let mut edit = gix_object::tree::Editor::new(root_tree.clone(), &odb, hash_kind());

    let actual = edit
        .remove(["file"])?
        .remove(Some("does not exist"))?
        .remove(["also", "does", "not", "exist"])?
        .remove(Some("bin.d"))?
        .remove(Some("file.toml.bin"))?
        .remove(Some("file.0"))?
        .write(&mut write)?;
    insta::assert_snapshot!(crate::normalize_tree_snapshot(&(display_tree_with_odb(actual, &storage, &odb))), @r#"
        Oid(1)
        ├── bin Oid(2).100644
        ├── file.to Oid(2).100644
        ├── file.toml Oid(2).100644
        └── file0 Oid(2).100644
    "#);
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
    insta::assert_snapshot!(
        crate::normalize_tree_snapshot(&(display_tree_with_odb(actual, &storage, &odb))),
        "`file` is removed as it remains empty",
        @r#"
        Oid(1)
        ├── bin Oid(2).100644
        ├── bin.d Oid(2).100644
        ├── file.to Oid(2).100644
        ├── file.toml Oid(2).100644
        ├── file.toml.bin Oid(2).100644
        └── file0 Oid(2).100644
    "#
    );

    Ok(())
}
#[test]
fn from_empty_invalid_write() -> crate::Result {
    let (storage, mut write, _num_writes_and_clear) = new_inmemory_writes();
    let odb = StorageOdb::new(storage.clone());
    let mut edit = gix_object::tree::Editor::new(Tree::default(), &odb, hash_kind());

    let actual = edit
        .upsert(["a", "\n"], EntryKind::Blob, any_blob())?
        .write(&mut write)
        .expect("no validation is performed");
    insta::assert_snapshot!(crate::normalize_tree_snapshot(&display_tree(actual, &storage)), @r#"
        Oid(1)
        └── a
            └── 
         Oid(2).100644
    "#);

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
    insta::assert_snapshot!(
        crate::normalize_tree_snapshot(&display_tree(actual, &storage)),
        "after removing invalid entries, it can write again",
        @r#"
        Oid(1)
        └── works Oid(2).100644
    "#
    );
    Ok(())
}
#[test]
fn from_empty_add() -> crate::Result {
    let (storage, mut write, num_writes_and_clear) = new_inmemory_writes();
    let odb = StorageOdb::new(storage.clone());
    let mut edit = gix_object::tree::Editor::new(Tree::default(), &odb, hash_kind());

    let actual = edit.write(&mut write).expect("no changes are fine");
    assert_eq!(actual, empty_tree(), "empty stays empty");
    assert_eq!(num_writes_and_clear(), 1, "the empty tree was written");
    insta::assert_snapshot!(crate::normalize_tree_snapshot(&display_tree(actual, &storage)), @"Oid(1)");
    assert_eq!(odb.access_count_and_clear(), 0);
    assert_eq!(
        edit.get(None::<&str>),
        None,
        "the 'root' can't be obtained, no entry exists for it, ever"
    );

    let actual = edit
        .upsert(Some("hi"), EntryKind::Blob, null_id())?
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
        .upsert(["a", "b", "c"], EntryKind::Blob, null_id())?
        .upsert(["a", "b", "d", "e"], EntryKind::Blob, null_id())?
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
            oid: empty_tree(),
        }),
        "before writing, entries are still present, just like they were written"
    );
    assert_eq!(
        edit.get(["a", "b", "c"]),
        Some(&Entry {
            mode: EntryKind::Tree.into(),
            filename: "c".into(),
            oid: empty_tree(),
        }),
    );

    let actual = edit.write(&mut write).expect("it's OK to write empty trees");
    insta::assert_snapshot!(
        crate::normalize_tree_snapshot(&display_tree(actual, &storage)),
        "one can write through trees, and empty trees are also fine",
        @r#"
        Oid(1)
        └── a
            └── b
                ├── c (empty)
                └── d
                    └── e (empty)
    "#
    );
    assert_eq!(num_writes_and_clear(), 4, "it wrote the trees it needed to write");
    assert_eq!(odb.access_count_and_clear(), 0);
    assert_eq!(edit.get(["a", "b"]), None, "nothing exists here");
    let oid = gix_hash::ObjectId::from_hex(match hash_kind() {
        gix_hash::Kind::Sha1 => &b"850bf83c26003cb0541318718bc9217c4a5bde6d"[..],
        gix_hash::Kind::Sha256 => &b"76be2e1aa5ce87f85b81d707c3a5f91c37d09fd064e28e13442b657d419e15f4"[..],
        _ => unreachable!("tests only support sha1 and sha256 fixtures"),
    })
    .expect("valid object id");
    assert_eq!(
        edit.get(Some("a")),
        Some(&Entry {
            mode: EntryKind::Tree.into(),
            filename: "a".into(),
            oid,
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
    insta::assert_snapshot!(
        crate::normalize_tree_snapshot(&display_tree(actual, &storage)),
        "it's possible to write through previously added blobs",
        @r#"
        Oid(1)
        ├── a
        │   └── b
        │       └── c Oid(2).100755
        └── x
            └── z Oid(2).100644
    "#
    );
    assert_eq!(num_writes_and_clear(), 4);
    assert_eq!(odb.access_count_and_clear(), 0);

    let actual = edit.upsert(["x"], EntryKind::Blob, any_blob())?.write(&mut write)?;
    insta::assert_snapshot!(crate::normalize_tree_snapshot(&display_tree(actual, &storage)), @r#"
        Oid(1)
        ├── a
        │   └── b
        │       └── c Oid(2).100755
        └── x Oid(2).100644
    "#);
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
    insta::assert_snapshot!(
        crate::normalize_tree_snapshot(&display_tree(actual, &storage)),
        "now a tree was once again changed into a blob",
        @r#"
        Oid(1)
        ├── a Oid(2).100644
        └── x Oid(2).100644
    "#
    );
    assert_eq!(num_writes_and_clear(), 1, "only the root-tree changes effectively");
    assert_eq!(odb.access_count_and_clear(), 2, "`a` and `a/b`");

    let actual = edit
        .set_root(Tree::default())
        .upsert(["a", "b", "c"], EntryKind::Blob, any_blob())?
        .upsert(["a"], EntryKind::BlobExecutable, any_blob())?
        .write(&mut write)?;
    insta::assert_snapshot!(
        crate::normalize_tree_snapshot(&display_tree(actual, &storage)),
        "now the root is back to a well-known state, so edits are more intuitive",
        @r#"
        Oid(1)
        └── a Oid(2).100755
    "#
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
    insta::assert_snapshot!(
        crate::normalize_tree_snapshot(&display_tree(actual, &storage)),
        "the intermediate tree is rewritten to be suitable to hold the blob",
        @r#"
        Oid(1)
        └── a
            └── b
                └── c Oid(2).100755
    "#
    );
    assert_eq!(num_writes_and_clear(), 3, "root, and two child-trees");
    assert_eq!(odb.access_count_and_clear(), 0);

    Ok(())
}
#[test]
fn from_existing_add() -> crate::Result {
    let (storage, mut write, num_writes_and_clear) = new_inmemory_writes();
    let odb = StorageOdb::new_with_odb(storage.clone(), tree_odb()?);
    let root_tree_id = crate::generated_tree_root_id()?;
    let root_tree = find_tree(&odb, root_tree_id)?;
    odb.access_count_and_clear();
    let mut edit = gix_object::tree::Editor::new(root_tree.clone(), &odb, hash_kind());
    assert!(edit.get(["bin"]).is_some(), "the root is immediately available");

    let actual = edit.write(&mut write).expect("no changes are fine");
    assert_eq!(actual, root_tree_id, "it rewrites the same tree");
    assert_eq!(odb.access_count_and_clear(), 0);
    insta::assert_snapshot!(crate::normalize_tree_snapshot(&(display_tree_with_odb(actual, &storage, &odb))), @r#"
        Oid(1)
        ├── bin Oid(2).100644
        ├── bin.d Oid(2).100644
        ├── file.to Oid(2).100644
        ├── file.toml Oid(2).100644
        ├── file.toml.bin Oid(2).100644
        ├── file
        │   └── a Oid(2).100644
        └── file0 Oid(2).100644
    "#);
    assert_eq!(
        num_writes_and_clear(),
        1,
        "only the root is written - there is no change tracking"
    );

    let actual = edit
        .upsert(["file", "hi"], EntryKind::Blob, null_id())?
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
        .upsert(["a", "b", "c"], EntryKind::Blob, null_id())?
        .upsert(["a", "b", "d", "e"], EntryKind::Blob, null_id())?
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
    insta::assert_snapshot!(
        crate::normalize_tree_snapshot(&(display_tree_with_odb(actual, &storage, &odb))),
        "one can write through trees and blobs, and empty leaf trees are also fine",
        @r#"
        Oid(1)
        ├── a
        │   └── b
        │       └── d
        │           └── e (empty)
        ├── bin.d Oid(2).100644
        ├── bin
        │   └── b
        │       └── c (empty)
        ├── file.to Oid(2).100644
        ├── file.toml Oid(2).100644
        ├── file.toml.bin Oid(2).100644
        ├── file
        │   └── a Oid(2).100644
        └── file0 Oid(2).100644
    "#
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
    insta::assert_snapshot!(
        crate::normalize_tree_snapshot(&(display_tree_with_odb(actual, &storage, &odb))),
        "it properly sorts entries after type-changes",
        @r#"
        Oid(1)
        ├── a
        │   └── b Oid(2).100644
        ├── bin.d Oid(3).100644
        ├── bin
        │   └── b
        │       └── c (empty)
        ├── file Oid(2).100755
        ├── file.to Oid(3).100644
        ├── file.toml Oid(3).100644
        ├── file.toml.bin Oid(3).100644
        └── file0 Oid(3).100644
    "#
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
    insta::assert_snapshot!(
        crate::normalize_tree_snapshot(&(display_tree_with_odb(actual, &storage, &odb))),
        "now the root is back to a well-known state",
        @r#"
        Oid(1)
        ├── bin Oid(2).100644
        ├── bin.d Oid(2).100644
        ├── file.to Oid(2).100644
        ├── file.toml Oid(2).100644
        ├── file.toml.bin Oid(2).100644
        ├── file
        │   ├── a Oid(2).100644
        │   └── subdir
        │       └── exe Oid(3).100755
        └── file0 Oid(2).100644
    "#
    );
    assert_eq!(num_writes_and_clear(), 1 + 2, "the root and one subtree with directory");
    Ok(())
}

mod utils {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    use bstr::{BStr, ByteSlice};
    use gix_hash::ObjectId;
    use gix_object::{Tree, WriteTo};

    type TreeStore = Rc<RefCell<gix_hashtable::HashMap<ObjectId, Tree>>>;
    pub(super) struct StorageOdb(TreeStore, Option<gix_odb::Handle>, Cell<usize>);

    pub(super) fn tree_odb() -> gix_testtools::Result<gix_odb::Handle> {
        let root = gix_testtools::scripted_fixture_read_only("make_trees.sh")?;
        Ok(gix_odb::at_opts(
            root.join(".git/objects"),
            Vec::new(),
            gix_odb::store::init::Options {
                object_hash: crate::fixture_hash_kind(),
                ..Default::default()
            },
        )?)
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
                let id = gix_object::compute_hash(crate::fixture_hash_kind(), gix_object::Kind::Tree, &buf)?;
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
                        object_hash: id.kind(),
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
        ObjectId::empty_tree(crate::fixture_hash_kind())
    }

    pub(super) fn any_blob() -> ObjectId {
        ObjectId::from_hex(&vec![b'b'; crate::fixture_hash_kind().len_in_hex()]).expect("valid repeated hex")
    }
}
use utils::{
    StorageOdb, any_blob, display_tree, display_tree_with_odb, empty_tree, find_tree, new_inmemory_writes, tree_odb,
};
