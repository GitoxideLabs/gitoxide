use std::sync::atomic::AtomicBool;

use gix_features::progress;
use gix_odb::loose::Store;
use gix_testtools::fixture_path_standalone;
use pretty_assertions::assert_eq;

use crate::hex_to_id;

fn ldb() -> Store {
    Store::at(fixture_path_standalone("objects"), gix_hash::Kind::Sha1)
}

pub fn object_ids() -> Vec<gix_hash::ObjectId> {
    vec![
        hex_to_id("37d4e6c5c48ba0d245164c4e10d5f41140cab980"), // blob
        hex_to_id("595dfd62fc1ad283d61bb47a24e7a1f66398f84d"), // blob
        hex_to_id("6ba2a0ded519f737fd5b8d5ccfb141125ef3176f"), // tree
        hex_to_id("722fe60ad4f0276d5a8121970b5bb9dccdad4ef9"), // tag
        hex_to_id("96ae868b3539f551c88fd5f02394d022581b11b0"), // tree
        hex_to_id("a706d7cd20fc8ce71489f34b50cf01011c104193"), // blob (big)
        hex_to_id("ffa700b4aca13b80cb6b98a078e7c96804f8e0ec"), // commit
    ]
}

#[test]
fn iter() {
    let mut oids = ldb().iter().map(Result::unwrap).collect::<Vec<_>>();
    oids.sort();
    assert_eq!(oids, object_ids());
}
pub fn locate_oid(id: gix_hash::ObjectId, buf: &mut Vec<u8>) -> gix_object::Data<'_> {
    ldb().try_find(&id, buf).expect("read success").expect("id present")
}

#[test]
fn verify_integrity() {
    let db = ldb();
    let outcome = db
        .verify_integrity(&mut progress::Discard, &AtomicBool::new(false))
        .unwrap();
    assert_eq!(outcome.num_objects, 7);
}

mod write {
    use gix_object::Write;
    use gix_odb::loose;

    use crate::store::loose::{locate_oid, object_ids};

    #[test]
    fn read_and_write() -> crate::Result {
        let dir = gix_testtools::tempfile::tempdir()?;
        let db = loose::Store::at(dir.path(), gix_hash::Kind::Sha1);
        let mut buf = Vec::new();
        let mut buf2 = Vec::new();

        for oid in object_ids() {
            let obj = locate_oid(oid, &mut buf);
            let actual = db.write(&obj.decode()?)?;
            assert_eq!(actual, oid);
            assert_eq!(
                db.try_find(&oid, &mut buf2)?.expect("id present").decode()?,
                obj.decode()?
            );
            let actual = db.write_buf(obj.kind, obj.data)?;
            assert_eq!(actual, oid);
            assert_eq!(
                db.try_find(&oid, &mut buf2)?.expect("id present").decode()?,
                obj.decode()?
            );
        }
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn it_writes_objects_with_similar_permissions() -> crate::Result {
        let hk = gix_hash::Kind::Sha1;
        let git_store = loose::Store::at(
            gix_testtools::scripted_fixture_read_only_standalone("repo_with_loose_objects.sh")?.join(".git/objects"),
            hk,
        );
        let expected_perm = git_store
            .object_path(&gix_hash::ObjectId::empty_blob(hk))
            .metadata()?
            .permissions();

        let tmp = gix_testtools::tempfile::TempDir::new()?;
        let store = loose::Store::at(tmp.path(), hk);
        store.write_buf(gix_object::Kind::Blob, &[])?;
        let actual_perm = store
            .object_path(&gix_hash::ObjectId::empty_blob(hk))
            .metadata()?
            .permissions();
        assert_eq!(
            actual_perm, expected_perm,
            "we explicitly equalize permissions to be similar to what `git` would do"
        );
        Ok(())
    }

    #[test]
    fn collisions_do_not_cause_failure() -> crate::Result {
        let dir = gix_testtools::tempfile::tempdir()?;

        fn write_empty_trees(dir: &std::path::Path) {
            let db = loose::Store::at(dir, gix_hash::Kind::Sha1);
            let empty_tree = gix_object::Tree::empty();
            for _ in 0..2 {
                let id = db.write(&empty_tree).expect("works");
                assert!(db.contains(&id), "written objects are actually available");

                let empty_blob = db.write_buf(gix_object::Kind::Blob, &[]).expect("works");
                assert!(db.contains(&empty_blob), "written objects are actually available");
                let id = db
                    .write_stream(gix_object::Kind::Blob, 0, &mut [].as_slice())
                    .expect("works");
                assert_eq!(id, empty_blob);
                assert!(db.contains(&empty_blob), "written objects are actually available");
            }
        }

        gix_features::parallel::threads(|scope| {
            scope.spawn(|| write_empty_trees(dir.path()));
            scope.spawn(|| write_empty_trees(dir.path()));
        });

        Ok(())
    }
}

mod contains {
    use crate::store::loose::ldb;

    #[test]
    fn iterable_objects_are_contained() {
        let store = ldb();
        for oid in store.iter().map(Result::unwrap) {
            assert!(store.contains(&oid));
        }
    }
}

mod lookup_prefix {
    use std::collections::HashSet;

    use gix_testtools::fixture_path_standalone;
    use maplit::hashset;

    use crate::{odb::hex_to_id, store::loose::ldb};

    #[test]
    fn returns_none_for_prefixes_without_any_match() {
        let store = ldb();
        let prefix = gix_hash::Prefix::new(&gix_hash::ObjectId::null(gix_hash::Kind::Sha1), 7).unwrap();
        assert!(store.lookup_prefix(prefix, None).unwrap().is_none());

        let mut candidates = HashSet::default();
        assert!(
            store.lookup_prefix(prefix, Some(&mut candidates)).unwrap().is_none(),
            "error codes are the same"
        );
        assert!(candidates.is_empty());
    }

    #[test]
    fn returns_some_err_for_prefixes_with_more_than_one_match() {
        let objects_dir = gix_testtools::tempfile::tempdir().unwrap();
        gix_testtools::copy_recursively_into_existing_dir(fixture_path_standalone("objects"), &objects_dir).unwrap();
        std::fs::write(
            objects_dir
                .path()
                .join("37")
                .join("d4ffffffffffffffffffffffffffffffffffff"),
            b"fake",
        )
        .unwrap();
        let store = gix_odb::loose::Store::at(objects_dir.path(), gix_hash::Kind::Sha1);
        let input_id = hex_to_id("37d4e6c5c48ba0d245164c4e10d5f41140cab980");
        let prefix = gix_hash::Prefix::new(&input_id, 4).unwrap();
        assert_eq!(
            store.lookup_prefix(prefix, None).unwrap(),
            Some(Err(())),
            "there are two objects with that prefix"
        );

        let mut candidates = HashSet::default();
        assert_eq!(
            store.lookup_prefix(prefix, Some(&mut candidates)).unwrap(),
            Some(Err(())),
            "the error code is the same"
        );
        assert_eq!(
            candidates,
            hashset! {hex_to_id("37d4ffffffffffffffffffffffffffffffffffff"), input_id},
            "we get both matching objects"
        );
    }

    #[test]
    fn iterable_objects_can_be_looked_up_with_varying_prefix_lengths() {
        let store = ldb();
        let hex_lengths = &[4, 7, 40];
        for (index, oid) in store.iter().map(Result::unwrap).enumerate() {
            for mut candidates in [None, Some(HashSet::default())] {
                let hex_len = hex_lengths[index % hex_lengths.len()];
                let prefix = gix_hash::Prefix::new(&oid, hex_len).unwrap();
                assert_eq!(
                    store
                        .lookup_prefix(prefix, candidates.as_mut())
                        .unwrap()
                        .expect("object exists")
                        .expect("unambiguous"),
                    oid
                );
                if let Some(candidates) = candidates {
                    assert_eq!(candidates, hashset! {oid});
                }
            }
        }
    }
}

mod find {
    use gix_object::{bstr::ByteSlice, tree::EntryKind, BlobRef, CommitRef, Kind, TagRef, TreeRef};
    use gix_odb::loose;

    use crate::{
        hex_to_id,
        store::loose::{ldb, locate_oid},
    };

    fn find<'a>(hex: &str, buf: &'a mut Vec<u8>) -> gix_object::Data<'a> {
        locate_oid(hex_to_id(hex), buf)
    }

    #[test]
    fn invalid_object_does_not_trigger_panics() -> crate::Result {
        let tmp = gix_testtools::tempfile::tempdir()?;
        let base = tmp.path().join("aa");
        std::fs::create_dir(&base)?;
        std::fs::write(base.join("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"), [])?;
        let db = loose::Store::at(tmp.path(), gix_hash::Kind::Sha1);

        let mut buf = Vec::new();
        let id = hex_to_id("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert!(db.try_find(&id, &mut buf).is_err(), "it must not panic");
        assert!(db.try_header(&id).is_err(), "it must not panic");

        Ok(())
    }

    #[test]
    fn tag() -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = Vec::new();
        let o = find("722fe60ad4f0276d5a8121970b5bb9dccdad4ef9", &mut buf);
        assert_eq!(o.kind, Kind::Tag);
        assert_eq!(o.data.len(), 1024);
        let expected = TagRef {
            target: b"ffa700b4aca13b80cb6b98a078e7c96804f8e0ec".as_bstr(),
            name: b"1.0.0".as_bstr(),
            target_kind: Kind::Commit,
            message: b"for the signature".as_bstr(),
            pgp_signature: Some(
                b"-----BEGIN PGP SIGNATURE-----
Comment: GPGTools - https://gpgtools.org

iQIzBAABCgAdFiEEw7xSvXbiwjusbsBqZl+Z+p2ZlmwFAlsapyYACgkQZl+Z+p2Z
lmy6Ug/+KzvzqiNpzz1bMVVAzp8NCbiEO3QGYPyeQc521lBwpaTrRYR+oHJY15r3
OdL5WDysTpjN8N5FNyfmvzkuPdTkK3JlYmO7VRjdA2xu/B6vIZLaOfAowFrhMvKo
8eoqwGcAP3rC5TuWEgzq2qhbjS4JXFLd4NLjWEFqT2Y2UKm+g8TeGOsa/0pF4Nq5
xeW4qCYR0WcQLFedbpkKHxag2GfaXKvzNNJdqYhVQssNa6BeSmsfDvlWYNe617wV
NvsR/zJT0wHb5SSH+h6QmwA7LQIQF//83Vc3aF7kv9D54r3ibXW5TjZ3WoeTUZO7
kefkzJ12EYDCFLPhHvXPog518nO8Ot46dX+okrF0/B4N3RFTvjKr7VAGTzv2D/Dg
DrD531S2F71b+JIRh641eeP7bjWFQi3tWLtrEOtjjsKPJfYRMKpYFnAO4UUJ6Rck
Z5fFXEUCO8d5WT56jzKDjmVoY01lA87O1YsP/J+zQAlc9v1k6jqeQ53LZNgTN+ue
5fJuSPT3T43pSOD1VQSr3aZ2Anc4Qu7K8uX9lkpxF9Sc0tDbeCosFLZMWNVp6m+e
cjHJZXWmV4CcRfmLsXzU8s2cR9A0DBvOxhPD1TlKC2JhBFXigjuL9U4Rbq9tdegB
2n8f2douw6624Tn/6Lm4a7AoxmU+CMiYagDxDL3RuZ8CAfh3bn0=
=aIns
-----END PGP SIGNATURE-----
"
                .as_bstr(),
            ),
            tagger: Some(b"Sebastian Thiel <byronimo@gmail.com> 1528473343 +0200".as_bstr()),
        };
        assert_eq!(o.decode()?.as_tag().expect("tag"), &expected);
        Ok(())
    }

    #[test]
    fn commit() -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = Vec::new();
        let o = find("ffa700b4aca13b80cb6b98a078e7c96804f8e0ec", &mut buf);
        assert_eq!(o.kind, Kind::Commit);
        assert_eq!(o.data.len(), 1084);
        let expected = CommitRef {
            tree: b"6ba2a0ded519f737fd5b8d5ccfb141125ef3176f".as_bstr(),
            parents: vec![].into(),
            author: b"Sebastian Thiel <byronimo@gmail.com> 1528473303 +0200".as_bstr(),
            committer: b"Sebastian Thiel <byronimo@gmail.com> 1528473303 +0200".as_bstr(),
            encoding: None,
            message: b"initial commit\n".as_bstr(),
            extra_headers: vec![(b"gpgsig".as_bstr(), b"-----BEGIN PGP SIGNATURE-----\nComment: GPGTools - https://gpgtools.org\n\niQIzBAABCgAdFiEEw7xSvXbiwjusbsBqZl+Z+p2ZlmwFAlsaptwACgkQZl+Z+p2Z\nlmxXSQ//fj6t7aWoEKeMdFigfj6OXWPUyrRbS0N9kpJeOfA0BIOea/6Jbn8J5qh1\nYRfrySOzHPXR5Y+w4GwLiVas66qyhAbk4yeqZM0JxBjHDyPyRGhjUd3y7WjEa6bj\nP0ACAIkYZQ/Q/LDE3eubmhAwEobBH3nZbwE+/zDIG0i265bD5C0iDumVOiKkSelw\ncr6FZVw1HH+GcabFkeLRZLNGmPqGdbeBwYERqb0U1aRCzV1xLYteoKwyWcYaH8E3\n97z1rwhUO/L7o8WUEJtP3CLB0zuocslMxskf6bCeubBnRNJ0YrRmxGarxCP3vn4D\n3a/MwECnl6mnUU9t+OnfvrzLDN73rlq8iasUq6hGe7Sje7waX6b2UGpxHqwykmXg\nVimD6Ah7svJanHryfJn38DvJW/wOMqmAnSUAp+Y8W9EIe0xVntCmtMyoKuqBoY7T\nJlZ1kHJte6ELIM5JOY9Gx7D0ZCSKZJQqyjoqtl36dsomT0I78/+7QS1DP4S6XB7d\nc3BYH0JkW81p7AAFbE543ttN0Z4wKXErMFqUKnPZUIEuybtlNYV+krRdfDBWQysT\n3MBebjguVQ60oGs06PzeYBosKGQrHggAcwduLFuqXhLTJqN4UQ18RkE0vbtG3YA0\n+XtZQM13vURdfwFI5qitAGgw4EzPVrkWWzApzLCrRPEMbvP+b9A=\n=2qqN\n-----END PGP SIGNATURE-----\n".as_bstr().into())]
        };
        let object = o.decode()?;
        assert_eq!(object.as_commit().expect("commit"), &expected);
        Ok(())
    }

    #[test]
    fn blob_data() -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = Vec::new();
        let o = find("37d4e6c5c48ba0d245164c4e10d5f41140cab980", &mut buf);
        assert_eq!(o.data.as_bstr(), b"hi there\n".as_bstr());
        Ok(())
    }

    #[test]
    fn blob() -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = Vec::new();
        let o = find("37d4e6c5c48ba0d245164c4e10d5f41140cab980", &mut buf);
        assert_eq!(
            o.decode()?.as_blob().expect("blob"),
            &BlobRef {
                data: &[104, 105, 32, 116, 104, 101, 114, 101, 10]
            },
            "small blobs are treated similarly to other object types and are read into memory at once when the header is read"
        );
        Ok(())
    }

    #[test]
    fn blob_not_existing() {
        let mut buf = Vec::new();
        assert_eq!(try_locate("37d4e6c5c48ba0d245164c4e10d5f41140cab989", &mut buf), None);
    }

    #[test]
    fn blob_big() -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = Vec::new();
        let o = find("a706d7cd20fc8ce71489f34b50cf01011c104193", &mut buf);
        assert_eq!(
            o.decode()?.as_blob().expect("blob").data.len(),
            o.data.len(),
            "erm, blobs are the same as raw data?"
        );
        Ok(())
    }

    fn try_locate<'a>(hex: &str, buf: &'a mut Vec<u8>) -> Option<gix_object::Data<'a>> {
        ldb().try_find(&hex_to_id(hex), buf).ok().flatten()
    }

    pub fn as_id(id: &[u8; 20]) -> &gix_hash::oid {
        id.into()
    }

    #[test]
    fn tree() -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = Vec::new();
        let o = find("6ba2a0ded519f737fd5b8d5ccfb141125ef3176f", &mut buf);
        assert_eq!(o.kind, Kind::Tree);
        assert_eq!(o.data.len(), 66);

        let expected = TreeRef {
            entries: vec![
                gix_object::tree::EntryRef {
                    mode: EntryKind::Tree.into(),
                    filename: b"dir".as_bstr(),
                    oid: as_id(&[
                        150, 174, 134, 139, 53, 57, 245, 81, 200, 143, 213, 240, 35, 148, 208, 34, 88, 27, 17, 176,
                    ]),
                },
                gix_object::tree::EntryRef {
                    mode: EntryKind::Blob.into(),
                    filename: b"file.txt".as_bstr(),
                    oid: as_id(&[
                        55, 212, 230, 197, 196, 139, 160, 210, 69, 22, 76, 78, 16, 213, 244, 17, 64, 202, 185, 128,
                    ]),
                },
            ],
        };
        assert_eq!(o.decode()?.as_tree().expect("tree"), &expected);
        Ok(())
    }

    mod header {
        use crate::odb::{hex_to_id, store::loose::ldb};

        #[test]
        fn existing() -> crate::Result {
            let db = ldb();
            assert_eq!(
                db.try_header(&hex_to_id("a706d7cd20fc8ce71489f34b50cf01011c104193"))?
                    .expect("present"),
                (56915, gix_object::Kind::Blob)
            );
            Ok(())
        }

        #[test]
        fn non_existing() -> crate::Result {
            let db = ldb();
            assert_eq!(
                db.try_header(&hex_to_id("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"))?,
                None,
                "it does not exist"
            );
            Ok(())
        }

        #[test]
        fn all() -> crate::Result {
            let db = ldb();
            let mut buf = Vec::new();
            for id in db.iter() {
                let id = id?;
                let expected = db.try_find(&id, &mut buf)?.expect("exists");
                let (size, kind) = db.try_header(&id)?.expect("header exists");
                assert_eq!(size, expected.data.len() as u64);
                assert_eq!(kind, expected.kind);
            }
            Ok(())
        }
    }
}
