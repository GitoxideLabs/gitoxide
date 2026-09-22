use crate::Result;
use gix_object::{Exists, FindExt, Write, tree};
use gix_testtools::tempfile::TempDir;

use crate::hex_to_id_for_hash;

#[test]
fn without_memory() -> Result {
    use gix_odb::HeaderExt;
    let (mut odb, _tmp) = db_rw()?;
    let mut buf = Vec::new();
    let mem = odb.take_object_memory().expect("it starts out with memory set");
    assert_eq!(mem.len(), 0, "no object is stored initially");
    let existing = hex_to_id_for_hash(
        "21d3ba9a26b790a4858d67754ae05d04dfce4d0c",
        "95997c02e30a106c5413e7a68e7758c6b3c70e951f7471ee48d75c06edc7d234",
    );
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
    assert_eq!(
        new_tree_id,
        hex_to_id_for_hash(
            "249b0b4106a5e9e7875e446a26468e22ec47a05c",
            "e0fcb04b8efaa91993f9880a111c25ae4f2ebd2db8737504e14548a194e31c84",
        )
    );
    let actual = odb.header(new_tree_id).expect("header of new objects can be found");
    assert_eq!(actual.kind(), gix_object::Kind::Tree);
    assert_eq!(actual.size(), expected_new_tree_size());

    let new_tree = odb
        .find_tree(&new_tree_id, &mut buf)
        .expect("new tree is also available as object")
        .to_owned();
    assert_eq!(new_tree, tree);
    assert!(!odb.exists(&gix_testtools::object_hash().null()));

    Ok(())
}

#[test]
fn with_memory() -> Result {
    use gix_object::FindHeader;
    let mut odb = db()?;
    assert_eq!(
        (*odb).iter()?.count(),
        6,
        "let's be sure we didn't accidentally write anything"
    );
    let mut buf = Vec::new();
    let existing = hex_to_id_for_hash(
        "21d3ba9a26b790a4858d67754ae05d04dfce4d0c",
        "95997c02e30a106c5413e7a68e7758c6b3c70e951f7471ee48d75c06edc7d234",
    );
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
    assert_eq!(
        new_tree_id,
        hex_to_id_for_hash(
            "249b0b4106a5e9e7875e446a26468e22ec47a05c",
            "e0fcb04b8efaa91993f9880a111c25ae4f2ebd2db8737504e14548a194e31c84",
        )
    );
    let actual = odb
        .try_header(&new_tree_id)?
        .expect("header of new objects can be found");
    assert_eq!(actual.kind, gix_object::Kind::Tree);
    assert_eq!(actual.size, expected_new_tree_size());

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

    assert!(!odb.exists(&gix_testtools::object_hash().null()));

    Ok(())
}

#[test]
fn with_memory_trusts_known_id() -> Result {
    let odb = db()?;
    let kind = gix_object::Kind::Blob;
    let bytes = b"content";
    let id = gix_testtools::object_hash().null();

    assert_eq!(odb.write_buf_with_known_id(kind, bytes, id)?, id);
    assert_eq!(odb.num_objects_in_memory(), 1);

    let mut buf = Vec::new();
    let object = odb.find(&id, &mut buf)?;
    assert_eq!(object.kind, kind);
    assert_eq!(object.data, bytes);

    let stream_bytes = b"streamed content";
    let stream_id = gix_object::compute_hash(gix_testtools::object_hash(), kind, stream_bytes)?;
    assert_eq!(
        odb.write_stream_with_known_id(kind, stream_bytes.len() as u64, &mut stream_bytes.as_slice(), stream_id)?,
        stream_id
    );
    assert_eq!(odb.num_objects_in_memory(), 2);

    let object = odb.find(&stream_id, &mut buf)?;
    assert_eq!(object.kind, kind);
    assert_eq!(object.data, stream_bytes);

    Ok(())
}

#[test]
fn without_memory_forwards_known_id_writes() -> Result {
    let (mut odb, _tmp) = db_rw()?;
    odb.take_object_memory().expect("it starts out with memory set");

    let kind = gix_object::Kind::Blob;
    let bytes = b"content";
    let id = gix_testtools::object_hash().null();

    assert_eq!(odb.write_buf_with_known_id(kind, bytes, id)?, id);

    let mut buf = Vec::new();
    let object = odb.find(&id, &mut buf)?;
    assert_eq!(object.kind, kind);
    assert_eq!(object.data, bytes);

    let stream_bytes = b"streamed content";
    let stream_id = gix_object::compute_hash(gix_testtools::object_hash(), kind, stream_bytes)?;
    assert_eq!(
        odb.write_stream_with_known_id(kind, stream_bytes.len() as u64, &mut stream_bytes.as_slice(), stream_id)?,
        stream_id
    );

    let object = odb.find(&stream_id, &mut buf)?;
    assert_eq!(object.kind, kind);
    assert_eq!(object.data, stream_bytes);

    Ok(())
}

fn db() -> Result<gix_odb::memory::Proxy<gix_odb::Handle>> {
    let odb = crate::odb_at(crate::scripted_fixture_read_only("repo_with_loose_objects.sh")?.join(".git/objects"))?;
    Ok(gix_odb::memory::Proxy::new(odb, gix_testtools::object_hash()))
}

fn db_rw() -> Result<(gix_odb::memory::Proxy<gix_odb::Handle>, TempDir)> {
    let tmp = crate::scripted_fixture_writable("repo_with_loose_objects.sh")?;
    let odb = crate::odb_at(tmp.path().join(".git/objects"))?;
    Ok((gix_odb::memory::Proxy::new(odb, gix_testtools::object_hash()), tmp))
}

fn expected_new_tree_size() -> u64 {
    104 + 2 * (gix_testtools::object_hash().len_in_bytes() as u64 - 20)
}
