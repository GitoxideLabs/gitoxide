use crate::odb::hex_to_id;
use gix_object::Write;
use gix_object::{tree, Exists, FindExt};
use gix_testtools::tempfile::TempDir;

#[test]
fn without_memory() -> crate::Result {
    use gix_odb::HeaderExt;
    let (mut odb, _tmp) = db_rw()?;
    let mut buf = Vec::new();
    let mem = odb.take_object_memory().expect("it starts out with memory set");
    assert_eq!(mem.len(), 0, "no object is stored initially");
    let existing = hex_to_id("21d3ba9a26b790a4858d67754ae05d04dfce4d0c");
    let tree = odb.find_tree(&existing, &mut buf).expect("present and valid");
    assert_eq!(tree.entries.len(), 1);
    odb.header(existing).expect("header can be found just the same");
    assert!(odb.exists(&existing));

    let mut tree = tree.to_owned();
    tree.entries.push(tree::Entry {
        mode: tree::EntryKind::Blob.into(),
        filename: "z-for-sorting_another-file-with-same-content".into(),
        oid: existing,
    });
    let new_tree_id = odb.write(&tree)?;
    assert_eq!(new_tree_id, hex_to_id("249b0b4106a5e9e7875e446a26468e22ec47a05c"));
    let actual = odb.header(new_tree_id).expect("header of new objects can be found");
    assert_eq!(actual.kind(), gix_object::Kind::Tree);
    assert_eq!(actual.size(), 104);

    let new_tree = odb
        .find_tree(&new_tree_id, &mut buf)
        .expect("new tree is also available as object")
        .to_owned();
    assert_eq!(new_tree, tree);
    assert!(!odb.exists(&gix_hash::Kind::Sha1.null()));

    Ok(())
}

#[test]
fn with_memory() -> crate::Result {
    use gix_object::FindHeader;
    let mut odb = db()?;
    assert_eq!(
        (*odb).iter()?.count(),
        6,
        "let's be sure we didn't accidentally write anything"
    );
    let mut buf = Vec::new();
    let existing = hex_to_id("21d3ba9a26b790a4858d67754ae05d04dfce4d0c");
    let tree = odb.find_tree(&existing, &mut buf).expect("present and valid");
    assert!(odb.exists(&existing));
    assert_eq!(tree.entries.len(), 1);
    odb.try_header(&existing)?.expect("header can be found just the same");
    assert_eq!(
        odb.num_objects_in_memory(),
        0,
        "nothing is stored when fetching objects - it's not an object cache"
    );

    let mut tree = tree.to_owned();
    tree.entries.push(tree::Entry {
        mode: tree::EntryKind::Blob.into(),
        filename: "z-for-sorting_another-file-with-same-content".into(),
        oid: existing,
    });
    let new_tree_id = odb.write(&tree)?;
    assert_eq!(new_tree_id, hex_to_id("249b0b4106a5e9e7875e446a26468e22ec47a05c"));
    let actual = odb
        .try_header(&new_tree_id)?
        .expect("header of new objects can be found");
    assert_eq!(actual.kind, gix_object::Kind::Tree);
    assert_eq!(actual.size, 104);

    let new_tree = odb
        .find_tree(&new_tree_id, &mut buf)
        .expect("new tree is also available as object")
        .to_owned();
    assert_eq!(new_tree, tree);

    let mem = odb.reset_object_memory().expect("memory is still available");
    assert_eq!(mem.len(), 1, "one new object was just written");

    assert_eq!(
        odb.try_header(&new_tree_id)?,
        None,
        "without memory, the object can't be found anymore"
    );

    let prev_mem = odb.set_object_memory(mem).expect("reset means it's just cleared");
    assert_eq!(prev_mem.len(), 0, "nothing was stored after the reset");

    assert_eq!(odb.num_objects_in_memory(), 1, "we put all previous objects back");

    let odb2 = odb.clone();
    assert_eq!(odb2.num_objects_in_memory(), 1, "it also clones the object memory");

    assert!(!odb.exists(&gix_hash::Kind::Sha1.null()));

    Ok(())
}

fn db() -> crate::Result<gix_odb::memory::Proxy<gix_odb::Handle>> {
    let odb = gix_odb::at(
        gix_testtools::scripted_fixture_read_only_standalone("repo_with_loose_objects.sh")?.join(".git/objects"),
    )?;
    Ok(gix_odb::memory::Proxy::new(odb, gix_hash::Kind::Sha1))
}

fn db_rw() -> crate::Result<(gix_odb::memory::Proxy<gix_odb::Handle>, TempDir)> {
    let tmp = gix_testtools::scripted_fixture_writable_standalone("repo_with_loose_objects.sh")?;
    let odb = gix_odb::at(tmp.path().join(".git/objects"))?;
    Ok((gix_odb::memory::Proxy::new(odb, gix_hash::Kind::Sha1), tmp))
}
