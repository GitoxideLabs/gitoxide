use std::{path::PathBuf, sync::atomic::AtomicBool};

use gix_features::progress;
use gix_odb::loose::{Options, Store};
use gix_testtools::fixture_path;
use pretty_assertions::assert_eq;

use crate::hex_to_id;

fn ldb() -> Store {
    ldb_at(fixture_path("objects"))
}

fn ldb_at(path: impl Into<PathBuf>) -> Store {
    ldb_at_opts(path, gix_hash::Kind::Sha1)
}

fn ldb_at_opts(path: impl Into<PathBuf>, object_hash: gix_hash::Kind) -> Store {
    Store::at(path, object_hash)
}

fn limited_ldb(limit: usize) -> Store {
    Store::at_opts(
        fixture_path("objects"),
        gix_hash::Kind::Sha1,
        Options {
            alloc_limit_bytes: Some(limit),
            ..Default::default()
        },
    )
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
    let mut oids = ldb().iter().map(std::result::Result::unwrap).collect::<Vec<_>>();
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
        .expect("fixture objects pass integrity checks");
    assert_eq!(outcome.num_objects, 7, "all loose fixture objects were verified");
    let err = db
        .verify_integrity(&mut progress::Discard, &AtomicBool::new(true))
        .expect_err("verification was interrupted");
    assert!(
        err.is_retryable() && err.can_retry(),
        "interrupted verification can be retried"
    );
    let io_error = err
        .downcast_any_ref::<std::io::Error>()
        .expect("verification preserves the typed I/O interruption");
    insta::assert_debug_snapshot!(io_error, "verification retains the interruption kind", @"
    Kind(
        Interrupted,
    )
    ");
    assert_eq!(
        io_error.kind(),
        std::io::ErrorKind::Interrupted,
        "verification retains the interruption kind"
    );
    insta::assert_debug_snapshot!(err.probable_cause(), "verification retains the original I/O diagnostic", @"
    Kind(
        Interrupted,
    )
    ");
    assert!(
        err.downcast_any_ref::<gix_error::ClassificationMarker>().is_none(),
        "classification markers are not diagnostic errors"
    );
    assert_eq!(
        err.iter_errors().count(),
        1,
        "the I/O interruption is the only diagnostic"
    );
    assert_eq!(err.metadata().count(), 0, "real I/O errors need no generic replacement");
    assert!(
        err.classify().any(|classification| {
            classification.class() == gix_error::Class::Retryable
                && classification.io_kind() == Some(std::io::ErrorKind::Interrupted)
        }),
        "retryability identifies the original I/O interruption"
    );
}

mod write {
    use crate::Result;
    use gix_object::Write;
    use gix_odb::loose;

    use crate::store::loose::{ldb_at, ldb_at_opts, locate_oid, object_ids};

    #[test]
    fn compression_level_is_respected() -> Result {
        use gix_zlib::Compression;
        let data: Vec<u8> = (0..64 * 1024).map(|i| (i % 100) as u8).collect();
        let mut sizes = Vec::new();
        for level in [Compression::NONE, Compression::BEST] {
            let dir = gix_testtools::tempfile::tempdir()?;
            let db = loose::Store::at_opts(
                dir.path(),
                gix_testtools::object_hash(),
                loose::Options {
                    compression: level,
                    ..Default::default()
                },
            );
            let id = db.write_buf(gix_object::Kind::Blob, &data)?;
            sizes.push(db.object_path(&id).metadata()?.len());

            let mut buf = Vec::new();
            assert_eq!(
                db.try_find(&id, &mut buf)?.expect("just written").data,
                data,
                "written objects can be read back regardless of level"
            );
        }
        assert!(
            sizes[0] > sizes[1],
            "the best level compresses better than no compression at all: {sizes:?}"
        );
        Ok(())
    }

    #[test]
    fn read_and_write() -> Result {
        let dir = gix_testtools::tempfile::tempdir()?;
        let db = ldb_at(dir.path());
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
            let actual = db.write_buf_with_known_id(obj.kind, obj.data, oid)?;
            assert_eq!(actual, oid);
            assert_eq!(
                db.try_find(&oid, &mut buf2)?.expect("id present").decode()?,
                obj.decode()?
            );
            let mut from = obj.data;
            let actual = db.write_stream_with_known_id(obj.kind, obj.data.len() as u64, &mut from, oid)?;
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
    fn it_writes_objects_with_similar_permissions() -> Result {
        let object_hash = gix_testtools::object_hash();
        let git_store = loose::Store::at(
            crate::scripted_fixture_read_only("repo_with_loose_objects.sh")?.join(".git/objects"),
            object_hash,
        );
        let expected_perm = git_store
            .object_path(&object_hash.empty_blob())
            .metadata()?
            .permissions();

        let tmp = gix_testtools::tempfile::TempDir::new()?;
        let store = loose::Store::at(tmp.path(), object_hash);
        store.write_buf(gix_object::Kind::Blob, &[])?;
        let actual_perm = store.object_path(&object_hash.empty_blob()).metadata()?.permissions();
        assert_eq!(
            actual_perm, expected_perm,
            "we explicitly equalize permissions to be similar to what `git` would do"
        );
        Ok(())
    }

    #[test]
    fn collisions_do_not_cause_failure() -> Result {
        let dir = gix_testtools::tempfile::tempdir()?;

        fn write_empty_trees(dir: &std::path::Path) {
            let db = ldb_at_opts(dir, gix_testtools::object_hash());
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
        for oid in store.iter().map(std::result::Result::unwrap) {
            assert!(store.contains(&oid));
        }
    }
}

mod lookup_prefix {
    use std::collections::HashSet;

    use gix_testtools::fixture_path;
    use maplit::hashset;

    use crate::{
        hex_to_id,
        store::loose::{ldb, ldb_at},
    };

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
        gix_testtools::copy_recursively_into_existing_dir(fixture_path("objects"), &objects_dir).unwrap();
        std::fs::write(
            objects_dir
                .path()
                .join("37")
                .join("d4ffffffffffffffffffffffffffffffffffff"),
            b"fake",
        )
        .unwrap();
        let store = ldb_at(objects_dir.path());
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
        for (index, oid) in store.iter().map(std::result::Result::unwrap).enumerate() {
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
    use crate::Result;
    use gix_error::{Class, Message, MetadataValue, ResourceExhaustionKind};
    use gix_object::{BlobRef, CommitRef, Kind, TagRef, TreeRef, bstr::ByteSlice, tree::EntryKind};

    use crate::{
        hex_to_id, hex_to_id_for_hash,
        store::loose::{ldb, ldb_at_opts, limited_ldb, locate_oid},
    };

    fn find<'a>(hex: &str, buf: &'a mut Vec<u8>) -> gix_object::Data<'a> {
        locate_oid(hex_to_id(hex), buf)
    }

    #[test]
    fn invalid_object_does_not_trigger_panics() -> Result {
        let tmp = gix_testtools::tempfile::tempdir()?;
        let base = tmp.path().join("aa");
        std::fs::create_dir(&base)?;
        std::fs::write(
            base.join(match gix_testtools::object_hash() {
                gix_hash::Kind::Sha1 => "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                gix_hash::Kind::Sha256 => "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                _ => unimplemented!(),
            }),
            [],
        )?;
        let db = ldb_at_opts(tmp.path(), gix_testtools::object_hash());

        let mut buf = Vec::new();
        let id = hex_to_id_for_hash(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        assert!(db.try_find(&id, &mut buf).is_err(), "it must not panic");
        assert!(db.try_header(&id).is_err(), "it must not panic");
        let err = db
            .verify_integrity(
                &mut gix_features::progress::Discard,
                &std::sync::atomic::AtomicBool::new(false),
            )
            .expect_err("verification must report the invalid object");
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&(err), &[(&db.object_path(&id).to_string_lossy(), "<object-path>")]), "corrupt objects do not become valid when retried", @r#"
        Could not read loose object during verification, "object_id"="Oid(1)"
        |
        └─ Could not read loose object, "path"="<object-path>"
        |
        └─ Empty loose object file
        "#);
        assert!(!err.can_retry(), "corrupt objects do not become valid when retried");
        assert!(err.is_corrupted(), "verification preserves the original lookup error");

        Ok(())
    }

    #[test]
    fn completed_object_size_is_validated_before_allocation() -> Result {
        let mut error_snapshots = Vec::new();
        use std::io::Write;

        let tmp = gix_testtools::tempfile::tempdir()?;
        let object_hash = gix_testtools::object_hash();
        let db = ldb_at_opts(tmp.path(), object_hash);
        let blob_id = object_hash.empty_blob();
        let path = db.object_path(&blob_id);
        std::fs::create_dir(path.parent().expect("loose objects have a parent directory"))?;

        for size in [1048576, usize::MAX] {
            let mut writer = gix_zlib::stream::deflate::Write::new(Vec::new(), gix_zlib::Compression::DEFAULT);
            write!(writer, "blob {size}\0")?;
            writer.flush()?;
            std::fs::write(&path, writer.into_inner())?;

            let mut buf = Vec::new();
            let err = db
                .try_find(&blob_id, &mut buf)
                .expect_err("the completed stream contains no body despite its advertised size");
            error_snapshots.push(gix_testtools::redact_debug_snapshot(
                &(err),
                &[
                    (&path.to_string_lossy(), "<object-path>"),
                    (&usize::MAX.to_string(), "<usize::MAX>"),
                ],
            ));
            assert!(
                err.is_corrupted(),
                "a completed object with an oversized header is corrupt: {err}"
            );
            let sizes = err
                .metadata()
                .find(|context| context.contains_key("expected"))
                .expect("the mismatch records both sizes");
            let diagnostic = err
                .iter_errors()
                .filter_map(|error| error.downcast_ref::<Message>())
                .find(|diagnostic| diagnostic.values.contains_key("expected"))
                .expect("the size mismatch has a diagnostic");
            assert_eq!(
                sizes["expected"],
                MetadataValue::from(size),
                "the advertised size is retained"
            );
            assert_eq!(
                sizes["actual"],
                MetadataValue::U64(0),
                "the completed stream has no body"
            );
            assert_eq!(
                diagnostic.class,
                Some(Class::Corruption),
                "the size diagnostic supplies the class"
            );
            assert_eq!(
                err.iter_errors().count(),
                2,
                "the mismatch and path context are the only nodes"
            );
            assert!(
                err.probable_cause().is::<Message>(),
                "no synthetic corruption source remains"
            );
            assert_eq!(
                err.metadata().next().expect("the lookup records its path")["path"],
                MetadataValue::from(path.as_path()),
                "the native object path is retained separately from the size mismatch"
            );
            assert_eq!(
                err.classify()
                    .map(|classification| classification.class())
                    .collect::<Vec<_>>(),
                [Class::Corruption],
                "the mismatch supplies exactly one corruption classification"
            );
            assert_eq!(buf.capacity(), 0, "invalid sizes must be rejected before allocation");
        }
        insta::assert_debug_snapshot!(error_snapshots, "completed object size is validated before allocation", @r#"
        [
            Could not read loose object, "path"="<object-path>"
            |
            └─ Loose object size mismatch: invalid size of inflated loose object, "actual"=0, "expected"=1048576,
            Could not read loose object, "path"="<object-path>"
            |
            └─ Loose object size mismatch: invalid size of inflated loose object, "actual"=0, "expected"=<usize::MAX>,
        ]
        "#);
        Ok(())
    }

    #[test]
    fn tag() -> Result {
        let mut buf = Vec::new();
        let o = find("722fe60ad4f0276d5a8121970b5bb9dccdad4ef9", &mut buf);
        assert_eq!(o.kind, Kind::Tag);
        assert_eq!(o.data.len(), 1024);
        let expected = TagRef {
            target: b"ffa700b4aca13b80cb6b98a078e7c96804f8e0ec".as_bstr(),
            name: b"1.0.0".as_bstr(),
            target_kind: Kind::Commit,
            message: b"for the signature".as_bstr(),
            signature: Some(
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
    fn commit() -> Result {
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
    fn blob_data() -> Result {
        let mut buf = Vec::new();
        let o = find("37d4e6c5c48ba0d245164c4e10d5f41140cab980", &mut buf);
        assert_eq!(o.data.as_bstr(), b"hi there\n".as_bstr());
        Ok(())
    }

    #[test]
    fn blob() -> Result {
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
    fn blob_big() -> Result {
        let mut buf = Vec::new();
        let o = find("a706d7cd20fc8ce71489f34b50cf01011c104193", &mut buf);
        assert_eq!(
            o.decode()?.as_blob().expect("blob").data.len(),
            o.data.len(),
            "erm, blobs are the same as raw data?"
        );
        Ok(())
    }

    #[test]
    fn blob_big_respects_alloc_limit_bytes() -> Result {
        let id = hex_to_id("a706d7cd20fc8ce71489f34b50cf01011c104193");
        let db = limited_ldb(1);
        let mut buf = Vec::new();

        assert_eq!(
            db.try_header(&id)?.expect("header present"),
            (56915, Kind::Blob),
            "header-only reads remain available"
        );
        let err = db
            .try_find(&id, &mut buf)
            .expect_err("the object exceeds the configured allocation limit");
        let allocation = err
            .metadata()
            .find(|context| context.contains_key("size"))
            .expect("the allocation limit retains its byte counts");
        let diagnostic = err
            .iter_errors()
            .filter_map(|error| error.downcast_ref::<Message>())
            .find(|diagnostic| diagnostic.values.contains_key("size"))
            .expect("the allocation limit has a diagnostic");
        assert_eq!(
            *allocation,
            maplit::btreemap! {
                "limit".into() => MetadataValue::U64(1),
                "size".into() => MetadataValue::U64(56915),
            },
            "allocation limits add the unsigned byte limit to the requested size"
        );
        assert_eq!(
            diagnostic.class,
            Some(Class::ResourceExhaustion(ResourceExhaustionKind::AllocationLimit)),
            "the context itself supplies the allocation-limit class"
        );
        assert_eq!(
            err.iter_errors().count(),
            2,
            "the allocation diagnostic and lookup context are the only nodes"
        );
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[(&db.object_path(&id).to_string_lossy(), "<object-path>")]), "the allocation limit retains the lookup context and requested byte count", @r#"
        Could not read loose object, "path"="<object-path>"
        |
        └─ Cannot store loose object in memory: the object exceeds the configured allocation limit, "limit"=1, "size"=56915
        "#);
        assert!(
            err.probable_cause().is::<Message>(),
            "no synthetic resource-exhaustion cause remains"
        );
        assert_eq!(
            err.metadata().next().expect("the lookup records its path")["path"],
            MetadataValue::from(db.object_path(&id)),
            "the native object path remains separate from allocation details"
        );
        assert_eq!(
            err.classify()
                .map(|classification| classification.class())
                .collect::<Vec<_>>(),
            [gix_error::Class::ResourceExhaustion(
                gix_error::ResourceExhaustionKind::AllocationLimit
            )],
            "configured limits are classified only as resource exhaustion"
        );
        assert!(!err.is_corrupted(), "an allocation limit does not establish corruption");
        assert!(!err.can_retry(), "retrying does not change the configured limit");
        Ok(())
    }

    #[test]
    fn unrepresentable_allocation_preserves_the_original_cause() -> Result {
        use std::io::Write;

        let tmp = gix_testtools::tempfile::tempdir()?;
        let object_hash = gix_testtools::object_hash();
        let db = ldb_at_opts(tmp.path(), object_hash);
        let blob_id = object_hash.empty_blob();
        let path = db.object_path(&blob_id);
        std::fs::create_dir(path.parent().expect("loose objects have a parent directory"))?;
        let size = u64::MAX;
        let mut writer = gix_zlib::stream::deflate::Write::new(Vec::new(), gix_zlib::Compression::DEFAULT);
        write!(writer, "blob {size}\0")?;
        // Fill the initial header buffer so inflation is incomplete and allocation is attempted.
        // The size fails usize conversion on 32-bit targets and Vec capacity checks on 64-bit targets.
        writer.write_all(&[0; 64])?;
        writer.flush()?;
        std::fs::write(&path, writer.into_inner())?;

        let mut buf = Vec::new();
        let err = db
            .try_find(&blob_id, &mut buf)
            .expect_err("the advertised allocation cannot be represented");
        assert!(
            err.is_resource_exhausted(),
            "unrepresentable allocations are resource exhaustion"
        );
        assert_eq!(
            err.iter_errors().count(),
            3,
            "only path context, allocation context, and the real cause remain"
        );
        assert_eq!(
            err.classify()
                .map(|classification| classification.class())
                .collect::<Vec<_>>(),
            [Class::ResourceExhaustion(ResourceExhaustionKind::AllocationFailure)],
            "both integer conversion and reservation failures retain their allocation-failure class"
        );
        let allocation = err
            .metadata()
            .find(|context| context.contains_key("size"))
            .expect("the failed allocation records its requested size");
        let diagnostic = err
            .iter_errors()
            .filter_map(|error| error.downcast_ref::<Message>())
            .find(|diagnostic| diagnostic.values.contains_key("size"))
            .expect("the failed allocation has a diagnostic");
        assert_eq!(
            allocation["size"],
            MetadataValue::U64(size),
            "the full unrepresentable size is retained"
        );
        if usize::try_from(size).is_err() {
            insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[(&path.to_string_lossy(), "<object-path>")]), "unrepresentable sizes retain the failed integer conversion", @r#"
            Could not read loose object, "path"="<object-path>"
            |
            └─ Cannot store loose object in memory: the object size cannot be represented in memory, "size"=18446744073709551615
            |
            └─ out of range integral type conversion attempted
            "#);
            assert!(
                err.probable_cause().is::<std::num::TryFromIntError>(),
                "the actual integer-conversion error remains the cause"
            );
            assert_eq!(
                diagnostic.class,
                Some(Class::ResourceExhaustion(ResourceExhaustionKind::AllocationFailure)),
                "the generic context classifies the unrepresentable object size"
            );
        } else {
            insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[(&path.to_string_lossy(), "<object-path>")]), "impossible allocations retain the failed capacity reservation", @r#"
            Could not read loose object, "path"="<object-path>"
            |
            └─ Cannot store loose object in memory, "size"=18446744073709551615
            |
            └─ memory allocation failed because the computed capacity exceeded the collection's maximum
            "#);
            assert!(
                err.probable_cause().is::<std::collections::TryReserveError>(),
                "the actual capacity error remains the cause"
            );
            assert_eq!(
                diagnostic.class, None,
                "the real reservation error supplies its own classification"
            );
        }
        assert_eq!(
            err.metadata().next().expect("the lookup records its path")["path"],
            MetadataValue::from(path.as_path()),
            "the object path is retained separately from allocation details"
        );
        assert_eq!(err.metadata().count(), 2, "only the two caller contexts carry metadata");
        assert!(
            !err.is_corrupted() && !err.can_retry(),
            "an unrepresentable allocation cannot be repaired by retrying"
        );
        assert_eq!(buf.capacity(), 0, "the test must not attempt an actual huge allocation");
        Ok(())
    }

    fn try_locate<'a>(hex: &str, buf: &'a mut Vec<u8>) -> Option<gix_object::Data<'a>> {
        ldb().try_find(&hex_to_id(hex), buf).ok().flatten()
    }

    pub fn as_id(id: &[u8; 20]) -> &gix_hash::oid {
        id.into()
    }

    #[test]
    fn tree() -> Result {
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
        use crate::Result;
        use crate::{hex_to_id, store::loose::ldb};

        #[test]
        fn existing() -> Result {
            let db = ldb();
            assert_eq!(
                db.try_header(&hex_to_id("a706d7cd20fc8ce71489f34b50cf01011c104193"))?
                    .expect("present"),
                (56915, gix_object::Kind::Blob)
            );
            Ok(())
        }

        #[test]
        fn non_existing() -> Result {
            let db = ldb();
            assert_eq!(
                db.try_header(&hex_to_id("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"))?,
                None,
                "it does not exist"
            );
            Ok(())
        }

        #[test]
        fn all() -> Result {
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
