use std::io;

use gix_error::{NotFoundError, RetryableError, Value};
use gix_object::{Kind, Write};

#[test]
fn write_failures_preserve_custom_sources_and_metadata() -> crate::Result {
    #[derive(Debug)]
    struct ReadFailure(RetryableError);

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
            Err(io::Error::other(ReadFailure(RetryableError::new(NotFoundError::new(
                "temporarily missing input",
            )))))
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
            assert!(
                err.is_not_found() && err.can_retry(),
                "custom sources retain both predicates"
            );
            let err = err.into_error();
            assert!(
                err.is_not_found() && err.can_retry(),
                "conversion preserves both predicates"
            );
            assert!(
                err.downcast_any_ref::<ReadFailure>().is_some(),
                "the custom cause remains accessible"
            );
            let context = err.metadata().next().expect("the write adds context");
            assert_eq!(
                context.message, "Could not stream loose object data",
                "streaming failures share the same operation context"
            );
            assert_eq!(
                context.values,
                maplit::btreemap! { "path".into() => Value::from(dir.path()) },
                "the schema retains only the native object directory"
            );
        }
    }
    Ok(())
}

#[test]
fn delta_lookup_distinguishes_missing_bases_from_recursion_limits() -> crate::Result {
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
            let err = err.into_error();
            assert_eq!(
                err.is_not_found(),
                max_depth != 0,
                "hitting a limit does not establish absence"
            );
            assert!(!err.is_corrupted() && !err.can_retry());
            let context = err.metadata().next().expect("the failed lookup carries context");
            assert_eq!(
                context.message, "Could not resolve delta base object",
                "object and header lookup share the same delta-resolution context"
            );
            assert_eq!(
                context.values,
                maplit::btreemap! {
                    "base_id".into() => Value::from(base_id.to_string()),
                    "object_id".into() => Value::from(blob_id.to_string()),
                },
                "delta-resolution metadata retains both object IDs as hex text"
            );
            if max_depth == 0 {
                let limit = err
                    .metadata()
                    .find(|context| context.values.contains_key("max_depth"))
                    .expect("the nested recursion failure retains its limit");
                assert_eq!(
                    limit.message, "Reached recursion limit while resolving ref delta bases",
                    "both lookup paths report the same recursion-limit context"
                );
                assert_eq!(
                    limit.values,
                    maplit::btreemap! {
                        "max_depth".into() => Value::U64(0),
                        "object_id".into() => Value::from(blob_id.to_string()),
                    },
                    "recursion-limit metadata retains the original object and unsigned limit"
                );
            }
        }
    }
    Ok(())
}
