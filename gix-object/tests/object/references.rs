use gix_error::TestResult;
use gix_object::{Data, Kind, references::Entry};

#[test]
fn graph_edges_ignore_metadata_and_preserve_expected_kinds() -> TestResult {
    for &hash in gix_hash::Kind::all() {
        let tree_id = gix_hash::ObjectId::empty_tree(hash);
        let blob_id = gix_hash::ObjectId::empty_blob(hash);
        let parent_id = gix_object::compute_hash(hash, Kind::Commit, b"parent")?;
        let commit = format!("tree {tree_id}\nparent {parent_id}\nauthor broken\n\nbody\n");
        let object = Data::new(commit.as_bytes(), Kind::Commit, hash);
        assert!(object.decode().is_err(), "identities still require full validation");
        assert_eq!(
            object.references().collect::<gix_error::Result<Vec<_>>>()?,
            vec![
                Entry {
                    id: tree_id,
                    kind: Kind::Tree
                },
                Entry {
                    id: parent_id,
                    kind: Kind::Commit
                }
            ],
            "all commit graph edges precede unrelated metadata"
        );
        let tag = format!("object {blob_id}\ntype blob\nmalformed metadata");
        assert_eq!(
            Data::new(tag.as_bytes(), Kind::Tag, hash)
                .references()
                .collect::<gix_error::Result<Vec<_>>>()?,
            vec![Entry {
                id: blob_id,
                kind: Kind::Blob
            }],
            "tag targets retain their declared object kinds"
        );
        let mut tree = Vec::new();
        for (mode, object_id) in [
            ("0000000100644", blob_id),
            ("040000", tree_id),
            ("0", blob_id),
            ("160000", parent_id),
        ] {
            tree.extend_from_slice(format!("{mode} entry\0").as_bytes());
            tree.extend_from_slice(object_id.as_bytes());
        }
        assert_eq!(
            Data::new(&tree, Kind::Tree, hash)
                .references()
                .collect::<gix_error::Result<Vec<_>>>()?,
            vec![
                Entry {
                    id: blob_id,
                    kind: Kind::Blob
                },
                Entry {
                    id: tree_id,
                    kind: Kind::Tree
                },
                Entry {
                    id: blob_id,
                    kind: Kind::Blob
                }
            ],
            "raw modes retain connectivity; only actual gitlinks are excluded"
        );
        assert!(
            Data::new(b"arbitrary bytes", Kind::Blob, hash)
                .references()
                .next()
                .is_none()
        );
    }
    Ok(())
}

#[test]
fn malformed_edges_fail_and_fuse_even_when_metadata_is_not_validated() -> TestResult {
    for &hash in gix_hash::Kind::all() {
        let tree_id = gix_hash::ObjectId::empty_tree(hash);
        for (kind, data) in [
            (Kind::Commit, String::new().into_bytes()),
            (Kind::Commit, b"tree invalid\n".to_vec()),
            (Kind::Commit, format!("tree {tree_id}\nparent invalid\n").into_bytes()),
            (Kind::Tag, format!("object {tree_id}\ntype invalid\n").into_bytes()),
            (Kind::Tag, format!("object {tree_id}\n").into_bytes()),
            (Kind::Tree, b"100644 entry\0truncated".to_vec()),
            (Kind::Tree, b"invalid entry\0".to_vec()),
        ] {
            let mut iter = Data::new(&data, kind, hash).references();
            iter.by_ref()
                .collect::<gix_error::Result<Vec<_>>>()
                .expect_err("malformed graph edges must fail");
            assert!(iter.next().is_none(), "the first malformed edge ends iteration");
        }
    }
    Ok(())
}
