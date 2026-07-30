use std::io;

use gix_error::{Class, ClassificationMarker, Error, Message, MetadataValue, classify};
use gix_object::{Kind, Write};

#[test]
fn write_failures_preserve_custom_sources_and_metadata() -> gix_testtools::TestResult {
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
                classify(err.probable_cause()).is_not_found(),
                "the original not-found cause remains available"
            );
            let context = err.metadata().next().expect("the write adds context");
            let diagnostic = err.downcast_any_ref::<Message>().expect("write context");
            assert_eq!(
                *context,
                [("path".into(), MetadataValue::from(dir.path()))]
                    .into_iter()
                    .collect::<gix_error::Metadata>(),
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
        Could not stream loose object data, path="<objects>"
        
        Caused by:
            0: I/O error (Other)
            1: custom reader failed
            2: temporarily missing input,
        Could not stream loose object data, path="<objects>"
        
        Caused by:
            0: I/O error (Other)
            1: custom reader failed
            2: temporarily missing input,
        Could not stream loose object data, path="<objects>"
        
        Caused by:
            0: I/O error (Other)
            1: custom reader failed
            2: temporarily missing input,
        Could not stream loose object data, path="<objects>"
        
        Caused by:
            0: I/O error (Other)
            1: custom reader failed
            2: temporarily missing input,
    ]
    "#);
    Ok(())
}

#[test]
fn delta_lookup_distinguishes_missing_bases_from_recursion_limits() -> gix_testtools::TestResult {
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
                !err.can_retry(),
                "neither absence nor a recursion limit establishes retryability"
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
                [
                    ("base_id".into(), MetadataValue::from(base_id.to_string())),
                    ("object_id".into(), MetadataValue::from(blob_id.to_string())),
                ]
                .into_iter()
                .collect::<gix_error::Metadata>(),
                "delta-resolution metadata retains both object IDs as hex text"
            );
            if max_depth == 0 {
                assert!(!err.is_corrupted(), "a recursion limit does not establish corruption");
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
                    [
                        ("max_depth".into(), MetadataValue::U64(0)),
                        ("object_id".into(), MetadataValue::from(blob_id.to_string())),
                    ]
                    .into_iter()
                    .collect::<gix_error::Metadata>(),
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
                classify(err.probable_cause()).any(|classification| classification.class() == class),
                "the original zlib error retains its classification"
            );
        }
    }
    insta::assert_debug_snapshot!(error_snapshots, "delta lookup distinguishes missing bases from recursion limits", @r#"
    [
        Could not resolve delta base object: delta base object is missing, base_id="Oid(1)", object_id="Oid(2)",
        Could not resolve delta base object: delta base object is missing, base_id="Oid(1)", object_id="Oid(2)",
        Could not resolve delta base object, base_id="Oid(1)", object_id="Oid(2)"
        
        Caused by:
            0: Reached recursion limit while resolving ref delta bases, max_depth=0, object_id="Oid(2)",
        Could not resolve delta base object, base_id="Oid(1)", object_id="Oid(2)"
        
        Caused by:
            0: Reached recursion limit while resolving ref delta bases, max_depth=0, object_id="Oid(2)",
        Could not resolve delta base object, base_id="Oid(1)", object_id="Oid(2)"
        
        Caused by:
            0: Could not read loose object, path="<base-object-path>"
            1: Could not decode zip stream
            2: Invalid input data,
        Could not resolve delta base object, base_id="Oid(1)", object_id="Oid(2)"
        
        Caused by:
            0: Could not read loose object header, path="<base-object-path>"
            1: Could not decode zip stream
            2: Invalid input data,
        Could not resolve delta base object, base_id="Oid(1)", object_id="Oid(2)"
        
        Caused by:
            0: Could not read loose object, path="<base-object-path>"
            1: Could not decode zip stream
            2: Decompressing this input requires a dictionary,
        Could not resolve delta base object, base_id="Oid(1)", object_id="Oid(2)"
        
        Caused by:
            0: Could not read loose object header, path="<base-object-path>"
            1: Could not decode zip stream
            2: Decompressing this input requires a dictionary,
    ]
    "#);
    Ok(())
}

/// Corrupt one OOFF reference while keeping all chunk sizes and the checksum valid.
/// LOFF has one entry, so ordinal one is out of bounds rather than a truncated chunk.
fn corrupt_multi_index_reference(
    path: &std::path::Path,
    object_id: &gix_hash::oid,
    large_offset: bool,
) -> gix_error::TestResult {
    let original = std::fs::read(path)?;
    let index = gix_pack::multi_index::File::from_data(original.clone(), path.to_owned(), None)?;
    let entry_index = index.lookup(object_id).expect("the object is indexed") as usize;
    let num_indices = index.num_indices();
    let object_hash = index.object_hash();
    drop(index);

    let mut chunks = Vec::new();
    for chunk_index in 0..usize::from(original[6]) {
        let table_offset = 12 + chunk_index * 12;
        let chunk_id: [u8; 4] = original[table_offset..table_offset + 4].try_into()?;
        let start = usize::try_from(u64::from_be_bytes(
            original[table_offset + 4..table_offset + 12].try_into()?,
        ))?;
        let end = usize::try_from(u64::from_be_bytes(
            original[table_offset + 16..table_offset + 24].try_into()?,
        ))?;
        let mut data = original[start..end].to_vec();
        if chunk_id == *b"OOFF" {
            let offset = entry_index * 8 + if large_offset { 4 } else { 0 };
            let invalid = if large_offset { (1 << 31) | 1 } else { num_indices };
            data[offset..offset + 4].copy_from_slice(&invalid.to_be_bytes());
        }
        assert_ne!(chunk_id, *b"LOFF", "these small packs do not need large offsets");
        chunks.push((chunk_id, data));
    }
    if large_offset {
        chunks.push((*b"LOFF", 12u64.to_be_bytes().to_vec()));
    }

    let mut corrupted = original[..12].to_vec();
    corrupted[6] = chunks.len().try_into()?;
    let mut offset = 12 + (chunks.len() + 1) * 12;
    for (chunk_id, data) in &chunks {
        corrupted.extend_from_slice(chunk_id);
        corrupted.extend_from_slice(&(offset as u64).to_be_bytes());
        offset += data.len();
    }
    corrupted.extend_from_slice(&[0; 4]);
    corrupted.extend_from_slice(&(offset as u64).to_be_bytes());
    for (_, data) in chunks {
        corrupted.extend(data);
    }
    let mut hasher = gix_hash::hasher(object_hash);
    hasher.update(&corrupted);
    corrupted.extend_from_slice(hasher.try_finalize()?.as_slice());
    std::fs::write(path, corrupted)?;
    Ok(())
}

fn multi_index_corrupt_reference_is_fallible(large_offset: bool) -> gix_error::TestResult {
    use gix_pack::Find as _;

    let dir = crate::scripted_fixture_writable("make_repo_multi_index.sh")?;
    let objects = dir.path().join(".git/objects");
    let path = objects.join("pack/multi-pack-index");
    let index = gix_pack::multi_index::File::at(&path, None)?;
    let object_id = index.oid_at_index(0).to_owned();
    let (pack_index, _) = index.pack_id_and_pack_offset_at_index(0)?;
    let valid_id = index
        .iter()
        .collect::<gix_error::Result<Vec<_>>>()?
        .into_iter()
        .find(|entry| entry.pack_index == pack_index && entry.oid != object_id)
        .expect("the same pack has another object")
        .oid;
    drop(index);
    corrupt_multi_index_reference(&path, &object_id, large_offset)?;

    // Loading must not reject the MIDX and silently fall back to the valid plain indices.
    let index = gix_pack::multi_index::File::at(&path, None)?;
    assert_eq!(
        index.oid_at_index(0),
        object_id,
        "object IDs need no pack-reference validation"
    );
    drop(index);
    let mut handle = crate::odb_at(&objects)?.into_inner();
    handle.prevent_pack_unload();
    let cache = gix_odb::Cache::from(handle.clone());
    let location = handle
        .location_by_oid(&valid_id, &mut Vec::new())?
        .expect("an unrelated valid reference remains usable");
    for err in [
        handle
            .location_by_oid(&object_id, &mut Vec::new())
            .expect_err("the invalid reference must fail location lookup"),
        handle
            .pack_offsets_and_oid(location.pack_id)
            .expect_err("pack iteration must reject corrupt references instead of returning partial results"),
        cache
            .location_by_oid(&object_id, &mut Vec::new())
            .expect_err("the cache forwards location lookup errors"),
        cache
            .pack_offsets_and_oid(location.pack_id)
            .expect_err("the cache forwards pack iteration errors"),
        gix_object::Find::try_find(&handle, &object_id, &mut Vec::new())
            .expect_err("the invalid reference must fail object lookup"),
        gix_odb::Header::try_header(&handle, &object_id).expect_err("the invalid reference must fail header lookup"),
        gix_object::FindHeader::try_header(&handle, &object_id)
            .expect_err("the object header trait also reports corruption"),
    ] {
        assert!(
            err.is_corrupted(),
            "MIDX reference errors retain their corruption classification: {err}"
        );
        assert!(!err.is_not_found(), "corruption is not object absence");
    }
    let missing_id = gix_object::compute_hash(gix_testtools::object_hash(), Kind::Blob, b"not in the MIDX fixture")?;
    assert!(
        !handle.contains(&missing_id),
        "the missing object is absent from both packs and loose storage"
    );
    assert!(
        handle.location_by_oid(&missing_id, &mut Vec::new())?.is_none(),
        "true object absence is Ok(None), not corruption"
    );
    assert!(
        cache.location_by_oid(&missing_id, &mut Vec::new())?.is_none(),
        "the cache preserves Ok(None) for true object absence"
    );
    assert_eq!(
        cache.location_by_oid(&valid_id, &mut Vec::new())?,
        Some(location),
        "the cache forwards successful location lookups"
    );
    assert!(
        gix_object::Find::try_find(&handle, &valid_id, &mut Vec::new())?.is_some(),
        "unrelated valid object lookup still works"
    );
    let lexical = handle.iter()?.collect::<std::result::Result<Vec<_>, _>>()?;
    let ordered = handle
        .iter()?
        .with_ordering(gix_odb::store::iter::Ordering::PackAscendingOffsetThenLooseLexicographical)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    assert_eq!(
        ordered, lexical,
        "invalid offset metadata falls back to object-ID ordering without dropping objects"
    );
    assert!(
        lexical.contains(&object_id),
        "ID-only iteration includes the corrupt reference"
    );
    Ok(())
}

#[test]
fn multi_index_invalid_pack_reference() -> gix_testtools::TestResult {
    multi_index_corrupt_reference_is_fallible(false)
}

#[test]
fn multi_index_invalid_large_offset_reference() -> gix_testtools::TestResult {
    multi_index_corrupt_reference_is_fallible(true)
}

fn multi_index_pack_generation_is_fallible(large_offset: bool) -> gix_error::TestResult {
    use gix_pack::{Find as _, data::output};

    let dir = crate::scripted_fixture_writable("make_repo_multi_index.sh")?;
    let objects = dir.path().join(".git/objects");
    let path = objects.join("pack/multi-pack-index");
    let index = gix_pack::multi_index::File::at(&path, None)?;
    let entries = index.iter().collect::<gix_error::Result<Vec<_>>>()?;
    let (delta_id, base_id, unrelated_id) = 'packs: {
        for (pack_index, name) in index.index_names().iter().enumerate() {
            let pack = gix_pack::data::File::at(
                objects.join("pack").join(name).with_extension("pack"),
                index.object_hash(),
            )?;
            for entry in entries.iter().filter(|entry| entry.pack_index as usize == pack_index) {
                if let gix_pack::data::entry::Header::OfsDelta { base_distance } = pack.entry(entry.pack_offset)?.header
                {
                    let base_offset =
                        gix_pack::data::entry::Header::verified_base_pack_offset(entry.pack_offset, base_distance)
                            .expect("the fixture's OFS delta has a valid base offset");
                    let Some(base) = entries
                        .iter()
                        .find(|base| base.pack_index == entry.pack_index && base.pack_offset == base_offset)
                    else {
                        continue;
                    };
                    if let Some(unrelated) = entries.iter().find(|other| {
                        other.pack_index == entry.pack_index && other.oid != entry.oid && other.oid != base.oid
                    }) {
                        break 'packs Some((entry.oid, base.oid, unrelated.oid));
                    }
                }
            }
        }
        None
    }
    .expect("the fixture contains an OFS delta, its indexed base, and an unrelated object in the same pack");
    drop(index);

    // Construct the shared store directly: converting an Rc-backed handle with
    // into_arc() disables MIDX support, which would bypass the corrupt metadata.
    let store = std::sync::Arc::new(gix_odb::Store::at_opts(
        objects.clone(),
        gix_testtools::object_hash(),
        &mut std::iter::empty(),
        Default::default(),
    )?);
    let mut db = store.to_cache_arc();
    db.prevent_pack_unload();
    let options = gix_pack::data::output::entry::iter_from_counts::Options {
        allow_thin_pack: true,
        thread_limit: Some(1),
        ..Default::default()
    };
    let count = output::Count {
        id: delta_id,
        entry_pack_location: output::count::PackLocation::NotLookedUp,
    };
    let mut valid_entries =
        output::entry::iter_from_counts(vec![count.clone()], db, Box::new(gix_utils::progress::Discard), options)?;
    let (_, generated) = valid_entries.next().expect("one input object produces one chunk")?;
    assert_eq!(
        generated.len(),
        1,
        "thin-pack generation emits only the requested delta"
    );
    assert_eq!(
        generated[0].kind,
        output::entry::Kind::DeltaOid { id: base_id },
        "the omitted OFS base is looked up through pack_offsets_and_oid and becomes a thin-pack ref delta"
    );
    assert!(valid_entries.next().is_none(), "there are no additional entry chunks");
    drop(valid_entries);
    drop(store);

    corrupt_multi_index_reference(&path, &unrelated_id, large_offset)?;
    let store = std::sync::Arc::new(gix_odb::Store::at_opts(
        objects,
        gix_testtools::object_hash(),
        &mut std::iter::empty(),
        Default::default(),
    )?);
    let mut db = store.to_cache_arc();
    db.prevent_pack_unload();
    assert!(
        db.location_by_oid(&delta_id, &mut Vec::new())?.is_some(),
        "unrelated MIDX corruption does not prevent locating the requested delta"
    );
    assert!(
        db.location_by_oid(&base_id, &mut Vec::new())?.is_some(),
        "the delta base reference remains valid too"
    );

    let err = output::entry::iter_from_counts(
        vec![output::Count {
            id: unrelated_id,
            entry_pack_location: output::count::PackLocation::NotLookedUp,
        }],
        db.clone(),
        Box::new(gix_utils::progress::Discard),
        options,
    )
    .err()
    .expect("resolving a corrupt MIDX reference fails the iterator constructor without panicking");
    assert!(err.is_corrupted(), "constructor errors retain MIDX corruption: {err}");
    assert!(!err.is_not_found(), "constructor corruption is not object absence");

    let mut entries = output::entry::iter_from_counts(
        vec![count.clone()],
        db.clone(),
        Box::new(gix_utils::progress::Discard),
        options,
    )?;
    let err = entries
        .next()
        .expect("the requested delta produces an iteration result")
        .expect_err("thin-pack base lookup rejects unrelated corrupt metadata without panicking");
    assert!(err.is_corrupted(), "iteration errors retain MIDX corruption: {err}");
    assert!(!err.is_not_found(), "iteration corruption is not a missing delta base");
    drop(entries);

    let entries = output::entry::iter_from_counts(vec![count], db, Box::new(gix_utils::progress::Discard), options)?;
    let mut writer = output::bytes::FromEntriesIter::new(
        entries.map(|chunk| chunk.map(|(_, entries)| entries)),
        Vec::new(),
        1,
        options.version,
        gix_testtools::object_hash(),
    );
    let err = writer
        .next()
        .expect("pack writing consumes the requested delta")
        .expect_err("pack writing propagates the thin-pack iterator error without panicking");
    assert!(
        err.is_corrupted(),
        "pack-writing context retains MIDX corruption: {err}"
    );
    assert!(!err.is_not_found(), "pack-writing corruption is not object absence");
    assert!(
        writer.digest().is_none(),
        "a failed pack is never finalized with a checksum"
    );
    assert!(writer.next().is_none(), "pack writing stops after the corrupted input");
    Ok(())
}

#[test]
fn multi_index_pack_generation_invalid_pack_reference() -> gix_testtools::TestResult {
    multi_index_pack_generation_is_fallible(false)
}

#[test]
fn multi_index_pack_generation_invalid_large_offset_reference() -> gix_testtools::TestResult {
    multi_index_pack_generation_is_fallible(true)
}

#[test]
fn pack_location_allocation_failures_are_resource_exhaustion() -> gix_testtools::TestResult {
    use gix_error::ResourceExhaustionKind;
    use gix_pack::{Find as _, data};
    use std::io::Write as _;

    for (size, limit, classification) in [
        (8192, Some(4096), ResourceExhaustionKind::AllocationLimit),
        (u64::MAX, None, ResourceExhaustionKind::AllocationFailure),
    ] {
        let dir = gix_testtools::tempfile::tempdir()?;
        let pack_dir = dir.path().join("pack");
        std::fs::create_dir(&pack_dir)?;
        let object_hash = gix_testtools::object_hash();
        let blob_id = gix_object::compute_hash(object_hash, Kind::Blob, b"blob")?;
        // The advertised size is intentionally larger than the body. Location lookup must
        // reject it before allocating or decompressing, regardless of any existing capacity.
        let mut pack = data::header::encode(data::Version::V2, 1).to_vec();
        let blob_offset = pack.len() as u32;
        data::entry::Header::Blob.write_to(size, &mut pack)?;
        let mut compressed = gix_zlib::stream::deflate::Write::new(Vec::new(), gix_zlib::Compression::DEFAULT);
        compressed.write_all(b"blob")?;
        compressed.flush()?;
        pack.extend(compressed.into_inner());
        let mut hasher = gix_hash::hasher(object_hash);
        hasher.update(&pack);
        let pack_id = hasher.try_finalize()?;
        pack.extend_from_slice(pack_id.as_slice());
        std::fs::write(pack_dir.join(format!("pack-{pack_id}.pack")), pack)?;

        let mut index = Vec::new();
        for first_byte in 0..=255 {
            index.extend_from_slice(&u32::from(blob_id.first_byte() <= first_byte).to_be_bytes());
        }
        index.extend_from_slice(&blob_offset.to_be_bytes());
        index.extend_from_slice(blob_id.as_slice());
        index.extend_from_slice(pack_id.as_slice());
        let mut hasher = gix_hash::hasher(object_hash);
        hasher.update(&index);
        index.extend_from_slice(hasher.try_finalize()?.as_slice());
        std::fs::write(pack_dir.join(format!("pack-{pack_id}.idx")), index)?;

        let mut handle = gix_odb::at_opts(
            dir.path(),
            object_hash,
            [],
            gix_odb::store::init::Options {
                alloc_limit_bytes: limit,
                ..Default::default()
            },
        )?;
        handle.prevent_pack_unload();
        let err = handle
            .location_by_oid(&blob_id, &mut vec![0; 16384])
            .expect_err("oversized on-disk metadata must fail location lookup without panicking");
        assert!(
            err.is_resource_exhausted()
                && err
                    .classify()
                    .all(|marker| marker.class() == Class::ResourceExhaustion(classification)),
            "size conversion, reservation, and configured limits are classified only as resource exhaustion"
        );
        assert!(
            !err.is_corrupted(),
            "allocation failures are not classified as corruption"
        );
        assert!(!err.is_not_found(), "allocation failures are not object absence");
        assert!(
            err.metadata().next().is_none(),
            "allocation diagnostics do not imply a size or limit metadata schema"
        );
        let diagnostic = err
            .iter_errors()
            .filter_map(|error| error.downcast_ref::<Message>())
            .find(|diagnostic| diagnostic.message.starts_with("Cannot store pack entry"))
            .expect("the failed allocation has a diagnostic");
        if limit.is_some() {
            assert_eq!(
                err.to_string(),
                "Cannot store pack entry in memory: allocation limit exceeded",
                "allocation limits retain their diagnostic without metadata suffixes"
            );
        } else if usize::try_from(size).is_err() {
            assert_eq!(
                diagnostic.message,
                format!(
                    "Cannot store pack entry of {size} bytes in memory: the entry size cannot be represented in memory"
                ),
                "conversion diagnostics retain the requested entry size in prose"
            );
            assert!(
                err.probable_cause().is::<std::num::TryFromIntError>(),
                "the original size conversion failure remains accessible"
            );
        } else {
            assert_eq!(
                diagnostic.message,
                format!("Cannot store pack entry of {size} bytes in memory"),
                "reservation diagnostics retain the requested entry size in prose"
            );
            assert!(
                err.probable_cause().is::<std::collections::TryReserveError>(),
                "the original buffer reservation failure remains accessible"
            );
        }
    }
    Ok(())
}

#[test]
fn pack_lookup_unavailable_is_not_an_error() -> gix_testtools::TestResult {
    let dir = gix_testtools::tempfile::tempdir()?;
    let mut handle = crate::odb_at(dir.path())?.into_inner();
    handle.prevent_pack_unload();
    let cache = gix_odb::Cache::from(handle.clone());
    let missing_id = gix_testtools::object_hash().null();
    for db in [&handle as &dyn gix_pack::Find, &cache] {
        assert!(
            db.location_by_oid(&missing_id, &mut Vec::new())?.is_none(),
            "an empty store has no packed object location"
        );
        assert!(
            db.pack_offsets_and_oid(0)?.is_none(),
            "an unavailable pack has no offsets, rather than an error"
        );
    }
    Ok(())
}

#[test]
fn pack_location_preserves_native_load_errors() -> gix_testtools::TestResult {
    use gix_pack::Find as _;

    let dir = crate::scripted_fixture_writable("make_repo_multi_index.sh")?;
    let objects = dir.path().join(".git/objects");
    let index = gix_pack::multi_index::File::at(objects.join("pack/multi-pack-index"), None)?;
    let object_id = index.oid_at_index(0).to_owned();
    let (pack_index, _) = index.pack_id_and_pack_offset_at_index(0)?;
    let pack_path = objects
        .join("pack")
        .join(&index.index_names()[pack_index as usize])
        .with_extension("pack");
    drop(index);
    // A directory is present instead of a pack file, so this is an I/O failure,
    // not a disappearing pack that can legitimately return Ok(None).
    std::fs::remove_file(&pack_path)?;
    std::fs::create_dir(&pack_path)?;
    let mut handle = crate::odb_at(&objects)?;
    handle.prevent_pack_unload();
    let err = handle
        .location_by_oid(&object_id, &mut Vec::new())
        .expect_err("a native pack-loading failure must not be suppressed");
    assert!(
        err.downcast_any_ref::<io::Error>().is_some(),
        "the original pack-loading I/O error remains accessible"
    );
    assert!(!err.is_not_found(), "a pack-loading I/O failure is not object absence");
    Ok(())
}

// TODO(odb-parallelism): restore shared/concurrent malformed-index coverage for location,
// offsets, object/header lookup and enumeration, including refresh_never and later valid/repaired
// packs. Cover pack/alternate removal clearing failures without refresh loops.
#[test]
fn pack_lookup_preserves_index_load_errors() -> gix_testtools::TestResult {
    use gix_pack::Find as _;

    for lookup_location in [false, true] {
        let dir = gix_testtools::tempfile::tempdir()?;
        let objects = dir.path();
        std::fs::create_dir(objects.join("info"))?;
        // A self-referencing alternate fails disk-state initialization before any
        // index is available. Both packing APIs must propagate load_one_index errors.
        std::fs::write(objects.join("info/alternates"), b".\n")?;
        let object_id = gix_testtools::object_hash().null();
        let mut handle = gix_odb::at_opts(
            objects,
            gix_testtools::object_hash(),
            [],
            gix_odb::store::init::Options {
                slots: gix_odb::store::init::Slots::Limit(32),
                ..Default::default()
            },
        )?;
        handle.prevent_pack_unload();
        let err = if lookup_location {
            handle
                .location_by_oid(&object_id, &mut Vec::new())
                .expect_err("location lookup must propagate index loading failures")
        } else {
            handle
                .pack_offsets_and_oid(0)
                .expect_err("pack iteration must propagate index loading failures")
        };
        assert!(
            err.is_corrupted(),
            "disk-state initialization corruption must remain classified: {err}"
        );
        assert!(!err.is_not_found(), "index-loading corruption is not object absence");
    }
    Ok(())
}

#[test]
fn multi_index_ref_delta_preserves_corrupt_base_lookup() -> gix_testtools::TestResult {
    use gix_pack::data;
    use std::io::Write;

    let dir = gix_testtools::tempfile::tempdir()?;
    let pack_dir = dir.path().join("pack");
    std::fs::create_dir(&pack_dir)?;
    let object_hash = gix_testtools::object_hash();
    let base_id = gix_object::compute_hash(object_hash, Kind::Blob, b"base")?;
    let blob_id = gix_object::compute_hash(object_hash, Kind::Blob, b"based")?;
    // The second blob copies the four-byte base and appends `d`. Ref-deltas force
    // the decoder to look up the base's MIDX reference instead of using an offset.
    let mut pack = data::header::encode(data::Version::V2, 2).to_vec();
    let base_offset = pack.len() as u32;
    data::entry::Header::Blob.write_to(4, &mut pack)?;
    let mut compressed = gix_zlib::stream::deflate::Write::new(Vec::new(), gix_zlib::Compression::DEFAULT);
    compressed.write_all(b"base")?;
    compressed.flush()?;
    pack.extend(compressed.into_inner());
    let blob_offset = pack.len() as u32;
    let delta = [4, 5, 0x90, 4, 1, b'd'];
    data::entry::Header::RefDelta { base_id }.write_to(delta.len() as u64, &mut pack)?;
    let mut compressed = gix_zlib::stream::deflate::Write::new(Vec::new(), gix_zlib::Compression::DEFAULT);
    compressed.write_all(&delta)?;
    compressed.flush()?;
    pack.extend(compressed.into_inner());
    let mut hasher = gix_hash::hasher(object_hash);
    hasher.update(&pack);
    let pack_id = hasher.try_finalize()?;
    pack.extend_from_slice(pack_id.as_slice());

    let mut entries = [(base_id, base_offset), (blob_id, blob_offset)];
    entries.sort_by_key(|(object_id, _)| *object_id);
    let mut index = Vec::new();
    for first_byte in 0..=255 {
        let count = entries
            .iter()
            .filter(|(object_id, _)| object_id.first_byte() <= first_byte)
            .count() as u32;
        index.extend_from_slice(&count.to_be_bytes());
    }
    for (object_id, offset) in entries {
        index.extend_from_slice(&offset.to_be_bytes());
        index.extend_from_slice(object_id.as_slice());
    }
    index.extend_from_slice(pack_id.as_slice());
    let mut hasher = gix_hash::hasher(object_hash);
    hasher.update(&index);
    index.extend_from_slice(hasher.try_finalize()?.as_slice());
    let index_path = pack_dir.join(format!("pack-{pack_id}.idx"));
    std::fs::write(pack_dir.join(format!("pack-{pack_id}.pack")), pack)?;
    std::fs::write(&index_path, index)?;
    let path = pack_dir.join("multi-pack-index");
    let mut multi_index = std::fs::File::create(&path)?;
    gix_pack::multi_index::write_from_index_paths(
        vec![index_path],
        &mut multi_index,
        &mut gix_utils::progress::Discard,
        &std::sync::atomic::AtomicBool::default(),
        gix_pack::multi_index::write::Options { object_hash },
    )?;
    drop(multi_index);
    let handle = crate::odb_at(dir.path())?;
    let mut buffer = Vec::new();
    assert_eq!(
        gix_object::Find::try_find(&handle, &blob_id, &mut buffer)?
            .expect("the valid delta resolves")
            .data,
        b"based",
        "the synthetic ref-delta is valid before corruption"
    );
    drop(handle);

    for large_offset in [false, true] {
        let original = std::fs::read(&path)?;
        corrupt_multi_index_reference(&path, &base_id, large_offset)?;
        let mut handle = crate::odb_at(dir.path())?;
        // If callback errors are suppressed, recursion would fail with an unclassified
        // depth-limit error instead of the original corruption diagnostic.
        handle.max_recursion_depth = 0;
        for err in [
            gix_object::Find::try_find(&handle, &blob_id, &mut Vec::new())
                .expect_err("the ref-delta base has an invalid MIDX reference"),
            gix_odb::Header::try_header(&handle, &blob_id)
                .expect_err("header decoding must preserve the base lookup error too"),
        ] {
            assert!(
                err.is_corrupted(),
                "decode callbacks must retain MIDX corruption: {err}"
            );
            assert!(!err.is_not_found(), "an invalid base reference is not a missing base");
        }
        drop(handle);
        std::fs::write(&path, original)?;
    }
    Ok(())
}

#[test]
#[cfg(unix)]
fn disappearing_loose_objects_keep_retryable_diagnostics() -> gix_testtools::TestResult {
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
                .verify_integrity(&mut gix_utils::progress::Discard, &AtomicBool::new(false))
                .expect_err("the enumerated object is missing"),
            1,
        ),
        (
            dynamic
                .store_ref()
                .verify_integrity(
                    &mut gix_utils::progress::Discard,
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
        Could not verify loose object database, path="<objects>"
        
        Caused by:
            0: Objects were deleted during iteration - try again,
    ]
    "#);
    Ok(())
}

#[test]
fn in_pack_ref_delta_preserves_malformed_base_entry() -> gix_testtools::TestResult {
    use gix_pack::data;
    use std::io::Write;

    let dir = gix_testtools::tempfile::tempdir()?;
    let pack_dir = dir.path().join("pack");
    std::fs::create_dir(&pack_dir)?;
    let object_hash = gix_testtools::object_hash();
    let base_id = gix_object::compute_hash(object_hash, Kind::Blob, b"base")?;
    let blob_id = gix_object::compute_hash(object_hash, Kind::Blob, b"based")?;
    let mut pack = data::header::encode(data::Version::V2, 2).to_vec();
    let base_offset = pack.len() as u32;
    // Type zero is invalid. The index still locates this base within the same pack,
    // so resolving the ref-delta must propagate pack.entry()'s error, not absence.
    pack.push(0);
    let blob_offset = pack.len() as u32;
    let delta = [4, 5, 0x90, 4, 1, b'd'];
    data::entry::Header::RefDelta { base_id }.write_to(delta.len() as u64, &mut pack)?;
    let mut compressed = gix_zlib::stream::deflate::Write::new(Vec::new(), gix_zlib::Compression::DEFAULT);
    compressed.write_all(&delta)?;
    compressed.flush()?;
    pack.extend(compressed.into_inner());
    let mut hasher = gix_hash::hasher(object_hash);
    hasher.update(&pack);
    let pack_id = hasher.try_finalize()?;
    pack.extend_from_slice(pack_id.as_slice());

    // A v1 index can describe the malformed base without decoding pack entries.
    let mut entries = [(base_id, base_offset), (blob_id, blob_offset)];
    entries.sort_by_key(|(object_id, _)| *object_id);
    let mut index = Vec::new();
    for first_byte in 0..=255 {
        let count = entries
            .iter()
            .filter(|(object_id, _)| object_id.first_byte() <= first_byte)
            .count() as u32;
        index.extend_from_slice(&count.to_be_bytes());
    }
    for (object_id, offset) in entries {
        index.extend_from_slice(&offset.to_be_bytes());
        index.extend_from_slice(object_id.as_slice());
    }
    index.extend_from_slice(pack_id.as_slice());
    let mut hasher = gix_hash::hasher(object_hash);
    hasher.update(&index);
    index.extend_from_slice(hasher.try_finalize()?.as_slice());
    std::fs::write(pack_dir.join(format!("pack-{pack_id}.pack")), pack)?;
    std::fs::write(pack_dir.join(format!("pack-{pack_id}.idx")), index)?;

    let mut handle = crate::odb_at(dir.path())?;
    // Suppressing the callback error would enter recursive base lookup and hit this limit.
    handle.max_recursion_depth = 0;
    for err in [
        gix_object::Find::try_find(&handle, &blob_id, &mut Vec::new())
            .expect_err("the in-pack ref-delta base has a malformed entry header"),
        gix_odb::Header::try_header(&handle, &blob_id)
            .expect_err("header decoding must preserve the malformed base entry error too"),
    ] {
        assert!(
            err.is_corrupted(),
            "base entry corruption must survive the callback: {err}"
        );
        assert!(!err.is_not_found(), "a malformed in-pack base is not absent");
        assert!(
            err.downcast_any_ref::<data::decode::DeltaBaseUnresolved>().is_none(),
            "a failed base entry lookup must not become an unresolved delta"
        );
        assert!(
            !err.metadata().any(|values| values.contains_key("max_depth")),
            "base entry corruption must bypass recursive fallback"
        );
        assert_eq!(
            err.probable_cause().to_string(),
            "Object type 0 is unsupported",
            "both decoders must retain the original pack.entry() diagnostic"
        );
    }
    Ok(())
}
