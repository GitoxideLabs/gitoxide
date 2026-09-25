use crate::Result;
use std::io;

use gix_error::{Class, ClassificationMarker, Error, Message, MetadataValue};
use gix_object::{Kind, Write};

#[test]
fn write_failures_preserve_custom_sources_and_metadata() -> Result {
    let mut error_snapshots = Vec::new();
    #[derive(Debug)]
    struct ReadFailure(ClassificationMarker);

    impl std::fmt::Display for ReadFailure {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("custom reader failed")
        }
    }

    impl std::error::Error for ReadFailure {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.0)
        }
    }

    struct FailingRead;
    impl io::Read for FailingRead {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other(ReadFailure(ClassificationMarker::with_source(
                Class::Retryable,
                gix_error::not_found("temporarily missing input"),
            ))))
        }
    }

    let dir = gix_testtools::tempfile::tempdir()?;
    let loose = gix_odb::loose::Store::at(dir.path(), gix_testtools::object_hash());
    let dynamic = crate::odb_at(dir.path())?;
    for store in [&loose as &dyn Write, &dynamic] {
        for err in [
            store
                .write_stream(Kind::Blob, 1, &mut FailingRead)
                .expect_err("the input reader failed"),
            store
                .write_stream_with_known_id(
                    Kind::Blob,
                    1,
                    &mut FailingRead,
                    gix_testtools::object_hash().empty_blob(),
                )
                .expect_err("a known object ID does not mask input failures"),
        ] {
            error_snapshots.push(gix_testtools::redact_debug_snapshot(
                &(err),
                &[(&(dir.path()).to_string_lossy(), "<objects>")],
            ));
            assert!(
                err.is_not_found() && err.can_retry(),
                "custom sources retain both predicates"
            );
            assert!(
                err.downcast_any_ref::<ReadFailure>().is_some(),
                "the custom cause remains accessible"
            );
            assert_eq!(
                err.downcast_any_ref::<io::Error>()
                    .expect("the reader's I/O error remains accessible")
                    .kind(),
                io::ErrorKind::Other,
                "context does not replace the reader's I/O error"
            );
            assert!(
                gix_error::classify(err.probable_cause()).is_not_found(),
                "the original not-found cause remains available"
            );
            let context = err.metadata().next().expect("the write adds context");
            let diagnostic = err.downcast_any_ref::<Message>().expect("write context");
            assert_eq!(
                *context,
                maplit::btreemap! { "path".into() => MetadataValue::from(dir.path()) },
                "the schema retains only the native object directory"
            );
            assert_eq!(
                diagnostic.class, None,
                "write context does not reclassify the reader's failure"
            );
            assert_eq!(
                err.metadata().count(),
                1,
                "only the write context contributes a nonempty dictionary"
            );
        }
    }
    insta::assert_debug_snapshot!(error_snapshots, "write failures preserve custom sources and metadata", @r#"
    [
        Could not stream loose object data, "path"="<objects>"
        |
        └─ I/O error (Other)
        |
        └─ custom reader failed
        |
        └─ temporarily missing input,
        Could not stream loose object data, "path"="<objects>"
        |
        └─ I/O error (Other)
        |
        └─ custom reader failed
        |
        └─ temporarily missing input,
        Could not stream loose object data, "path"="<objects>"
        |
        └─ I/O error (Other)
        |
        └─ custom reader failed
        |
        └─ temporarily missing input,
        Could not stream loose object data, "path"="<objects>"
        |
        └─ I/O error (Other)
        |
        └─ custom reader failed
        |
        └─ temporarily missing input,
    ]
    "#);
    Ok(())
}

#[test]
fn delta_lookup_distinguishes_missing_bases_from_recursion_limits() -> Result {
    let mut error_snapshots = Vec::new();
    use std::io::Write;

    use gix_object::Find;
    use gix_pack::data;

    let dir = gix_testtools::tempfile::tempdir()?;
    let pack_dir = dir.path().join("pack");
    std::fs::create_dir(&pack_dir)?;
    let object_hash = gix_testtools::object_hash();
    let blob_id = object_hash.empty_blob();
    let base_id = object_hash.null();

    // A single ref-delta would produce an empty blob, but its base is absent.
    let mut pack = data::header::encode(data::Version::V2, 1).to_vec();
    data::entry::Header::RefDelta { base_id }.write_to(2, &mut pack)?;
    let mut compressed = gix_zlib::stream::deflate::Write::new(Vec::new(), gix_zlib::Compression::DEFAULT);
    compressed.write_all(&[0, 0])?;
    compressed.flush()?;
    pack.extend(compressed.into_inner());
    let mut hasher = gix_hash::hasher(object_hash);
    hasher.update(&pack);
    let pack_id = hasher.try_finalize()?;
    pack.extend_from_slice(pack_id.as_slice());

    // A v1 index names the unresolved object without needing to decode it first.
    let mut index = Vec::new();
    for first_byte in 0..=255 {
        index.extend_from_slice(&u32::from(first_byte >= blob_id.first_byte()).to_be_bytes());
    }
    index.extend_from_slice(&(data::header::SIZE as u32).to_be_bytes());
    index.extend_from_slice(blob_id.as_slice());
    index.extend_from_slice(pack_id.as_slice());
    let mut hasher = gix_hash::hasher(object_hash);
    hasher.update(&index);
    index.extend_from_slice(hasher.try_finalize()?.as_slice());
    std::fs::write(pack_dir.join(format!("pack-{pack_id}.pack")), pack)?;
    std::fs::write(pack_dir.join(format!("pack-{pack_id}.idx")), index)?;

    for max_depth in [8, 0] {
        let mut store = crate::odb_at(dir.path())?;
        store.max_recursion_depth = max_depth;
        for err in [
            store
                .try_find(&blob_id, &mut Vec::new())
                .expect_err("the delta cannot be resolved"),
            gix_odb::Header::try_header(&store, &blob_id).expect_err("the base kind is unavailable"),
        ] {
            let num_nodes = if max_depth == 0 { 2 } else { 1 };
            assert_eq!(
                err.iter_errors().count(),
                num_nodes,
                "only missing bases collapse the synthetic cause and its context"
            );
            error_snapshots.push(gix_testtools::redact_debug_snapshot(
                &(err),
                &[(&(dir.path()).to_string_lossy(), "<objects>")],
            ));
            assert_eq!(err.is_not_found(), max_depth != 0, "only an absent base is not found");
            assert_eq!(
                err.is_not_found(),
                max_depth != 0,
                "hitting a limit does not establish absence, even after conversion"
            );
            assert!(
                !err.is_corrupted() && !err.can_retry(),
                "neither absence nor a recursion limit establishes corruption or retryability"
            );
            assert_eq!(
                err.metadata().count(),
                num_nodes,
                "each generic context keeps its own values"
            );
            let context = err.metadata().next().expect("the failed lookup carries context");
            let diagnostic = err.downcast_any_ref::<Message>().expect("delta resolution diagnostic");
            assert_eq!(
                *context,
                maplit::btreemap! {
                    "base_id".into() => MetadataValue::from(base_id.to_string()),
                    "object_id".into() => MetadataValue::from(blob_id.to_string()),
                },
                "delta-resolution metadata retains both object IDs as hex text"
            );
            if max_depth == 0 {
                assert_eq!(
                    diagnostic.class, None,
                    "lookup context must not classify recursion failures as missing"
                );
                assert_eq!(err.classify().count(), 0, "a recursion limit has no inferred class");
                let limit = err
                    .iter_errors()
                    .filter_map(|error| error.downcast_ref::<Message>())
                    .find(|context| context.values.contains_key("max_depth"))
                    .expect("the nested recursion failure retains its limit");
                assert_eq!(
                    limit.values,
                    maplit::btreemap! {
                        "max_depth".into() => MetadataValue::U64(0),
                        "object_id".into() => MetadataValue::from(blob_id.to_string()),
                    },
                    "recursion-limit metadata retains the original object and unsigned limit"
                );
            } else {
                assert_eq!(
                    diagnostic.class,
                    Some(Class::NotFound),
                    "the missing-base diagnostic supplies its own class"
                );
                assert!(
                    err.probable_cause().is::<Message>(),
                    "the combined diagnostic is the cause"
                );
                let classification = err.classify().next().expect("the missing base is classified");
                assert!(
                    classification.error().is::<Message>(),
                    "there is no synthetic not-found source"
                );
            }
        }
    }

    let loose = gix_odb::loose::Store::at(dir.path(), object_hash);
    let base_path = loose.object_path(&base_id);
    std::fs::create_dir(base_path.parent().expect("loose objects have a parent directory"))?;
    // These bases exist, but zlib rejects them: invalid data and a stream requiring a preset dictionary.
    for (input, class) in [
        (b"invalid zlib".as_slice(), Class::Corruption),
        ([0x78, 0x20, 0, 0, 0, 0].as_slice(), Class::Validation),
    ] {
        std::fs::write(&base_path, input)?;
        let store = crate::odb_at(dir.path())?;
        for err in [
            store
                .try_find(&blob_id, &mut Vec::new())
                .expect_err("the base cannot be decoded"),
            gix_odb::Header::try_header(&store, &blob_id).expect_err("the base header cannot be decoded"),
        ] {
            error_snapshots.push(gix_testtools::redact_debug_snapshot(
                &(err),
                &[(&base_path.to_string_lossy(), "<base-object-path>")],
            ));
            assert!(!err.is_not_found(), "a failed lookup does not establish absence");
            assert_eq!(
                err.classify()
                    .map(|classification| classification.class())
                    .collect::<Vec<_>>(),
                [class],
                "delta resolution retains the actual callee's class"
            );
            let context = err.metadata().next().expect("delta resolution supplies context");
            assert_eq!(
                err.downcast_any_ref::<Message>()
                    .expect("delta resolution context")
                    .class,
                None,
                "only the None branch adds a not-found class"
            );
            assert_eq!(
                context["object_id"],
                MetadataValue::from(blob_id.to_string()),
                "the requested object is retained"
            );
            assert_eq!(
                context["base_id"],
                MetadataValue::from(base_id.to_string()),
                "the failing base is retained"
            );
            assert_eq!(
                err.metadata().nth(1).expect("the loose lookup supplies its path")["path"],
                MetadataValue::from(base_path.as_path()),
                "the callee's metadata remains separate from delta context"
            );
            assert!(
                gix_error::classify(err.probable_cause()).any(|classification| classification.class() == class),
                "the original zlib error retains its classification"
            );
        }
    }
    insta::assert_debug_snapshot!(error_snapshots, "delta lookup distinguishes missing bases from recursion limits", @r#"
    [
        Could not resolve delta base object: delta base object is missing, "base_id"="Oid(1)", "object_id"="Oid(2)",
        Could not resolve delta base object: delta base object is missing, "base_id"="Oid(1)", "object_id"="Oid(2)",
        Could not resolve delta base object, "base_id"="Oid(1)", "object_id"="Oid(2)"
        |
        └─ Reached recursion limit while resolving ref delta bases, "max_depth"=0, "object_id"="Oid(2)",
        Could not resolve delta base object, "base_id"="Oid(1)", "object_id"="Oid(2)"
        |
        └─ Reached recursion limit while resolving ref delta bases, "max_depth"=0, "object_id"="Oid(2)",
        Could not resolve delta base object, "base_id"="Oid(1)", "object_id"="Oid(2)"
        |
        └─ Could not read loose object, "path"="<base-object-path>"
        |
        └─ Could not decode zip stream
        |
        └─ Invalid input data,
        Could not resolve delta base object, "base_id"="Oid(1)", "object_id"="Oid(2)"
        |
        └─ Could not read loose object header, "path"="<base-object-path>"
        |
        └─ Could not decode zip stream
        |
        └─ Invalid input data,
        Could not resolve delta base object, "base_id"="Oid(1)", "object_id"="Oid(2)"
        |
        └─ Could not read loose object, "path"="<base-object-path>"
        |
        └─ Could not decode zip stream
        |
        └─ Decompressing this input requires a dictionary,
        Could not resolve delta base object, "base_id"="Oid(1)", "object_id"="Oid(2)"
        |
        └─ Could not read loose object header, "path"="<base-object-path>"
        |
        └─ Could not decode zip stream
        |
        └─ Decompressing this input requires a dictionary,
    ]
    "#);
    Ok(())
}

#[test]
#[cfg(unix)]
fn disappearing_loose_objects_keep_retryable_diagnostics() -> Result {
    let mut diagnostics = Vec::new();
    use std::{os::unix::fs::symlink, sync::atomic::AtomicBool};

    let dir = gix_testtools::tempfile::tempdir()?;
    let object_hash = gix_testtools::object_hash();
    let loose = gix_odb::loose::Store::at(dir.path(), object_hash);
    let blob_id = object_hash.empty_blob();
    let path = loose.object_path(&blob_id);
    std::fs::create_dir(path.parent().expect("loose objects have a parent directory"))?;
    // A dangling link is enumerated but cannot be read, reproducing disappearance without a race.
    symlink(dir.path().join("deleted-object"), &path)?;
    assert_eq!(
        loose.iter().collect::<std::result::Result<Vec<_>, _>>()?,
        [blob_id],
        "the object path must be visible to iteration before lookup finds it missing"
    );
    let dynamic = crate::odb_at(dir.path())?;
    for (err, num_nodes) in [
        (
            loose
                .verify_integrity(&mut gix_features::progress::Discard, &AtomicBool::new(false))
                .expect_err("the enumerated object is missing"),
            1,
        ),
        (
            dynamic
                .store_ref()
                .verify_integrity(
                    &mut gix_features::progress::Discard,
                    &AtomicBool::new(false),
                    Default::default(),
                )
                .err()
                .expect("dynamic verification also observes the missing loose object"),
            2,
        ),
    ] {
        diagnostics.push(gix_testtools::redact_debug_snapshot(
            &err,
            &[(&dir.path().to_string_lossy(), "<objects>")],
        ));
        assert!(
            err.is_retryable() && err.can_retry(),
            "verification can retry after objects disappear"
        );
        assert_eq!(
            err.iter_errors().filter(|cause| !cause.is::<Error>()).count(),
            num_nodes,
            "retry classification adds no diagnostic across public error boundaries"
        );
        assert_eq!(
            err.metadata().count(),
            num_nodes - 1,
            "metadata skips the retry diagnostic's empty dictionary"
        );
        assert_eq!(
            err.classify()
                .map(|classification| classification.class())
                .collect::<Vec<_>>(),
            [Class::Retryable],
            "a disappearing object is retryable without adding a not-found class"
        );
        let cause = err
            .probable_cause()
            .downcast_ref::<Message>()
            .expect("the message itself carries retryability");
        assert_eq!(
            cause.class,
            Some(Class::Retryable),
            "no marker is needed for a synthetic retry diagnostic"
        );
        assert!(cause.values.is_empty(), "retry-only diagnostics need no scalar values");
        if num_nodes == 2 {
            let context = err.metadata().next().expect("dynamic verification adds path context");
            assert_eq!(
                err.downcast_any_ref::<Message>().expect("verification context").class,
                None,
                "the caller does not reclassify the failure"
            );
            assert_eq!(
                context["path"],
                MetadataValue::from(dir.path()),
                "the object directory remains available"
            );
        }
    }
    insta::assert_debug_snapshot!(diagnostics, "disappearing loose objects keep retryable diagnostics", @r#"
    [
        Objects were deleted during iteration - try again,
        Could not verify loose object database, "path"="<objects>"
        |
        └─ Objects were deleted during iteration - try again,
    ]
    "#);
    Ok(())
}
