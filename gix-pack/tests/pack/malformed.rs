use crate::Result;
use std::{
    io::Write,
    panic::{AssertUnwindSafe, catch_unwind},
    path::PathBuf,
};

use gix_error::{ErrorExt, TestResult, message};
use gix_pack::{cache, data};

const FIRST_ENTRY_OFFSET: data::Offset = data::header::SIZE as data::Offset;

#[test]
fn ref_delta_resolver_error_is_propagated_unchanged() -> TestResult {
    let bytes = ref_delta_pack(&[1, 1, 0x90, 1])?;
    let pack = data::File::from_data(bytes, PathBuf::from("resolver-error.pack"), gix_hash::Kind::Sha1)?;
    let callback_error = std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        message("base storage is inaccessible").with("path", "external-base"),
    )
    .and_raise(message("could not resolve delta base").with("operation", "base lookup"));
    let original_sources: Vec<_> = callback_error
        .iter_errors_with_locations()
        .map(|source| (source.error().to_string(), source.location()))
        .collect();
    let original_classes: Vec<_> = callback_error
        .classify()
        .map(|classification| classification.class())
        .collect();
    let callback_error = std::cell::RefCell::new(Some(callback_error));
    let resolutions = std::cell::Cell::new(0);
    let err = pack
        .decode_entry(
            pack.entry(FIRST_ENTRY_OFFSET)?,
            &mut Vec::new(),
            &mut Default::default(),
            &|base_id, _| {
                resolutions.set(resolutions.get() + 1);
                assert_eq!(
                    base_id,
                    gix_hash::Kind::Sha1.null().as_ref(),
                    "the resolver receives the synthetic external base ID"
                );
                Err(callback_error
                    .borrow_mut()
                    .take()
                    .expect("the failing resolver is called only once"))
            },
            &mut cache::Never,
        )
        .expect_err("a resolver failure must abort decoding instead of becoming an unresolved base");

    assert_eq!(resolutions.get(), 1, "a failed base lookup must not be retried");
    assert_eq!(
        err.iter_errors_with_locations()
            .map(|source| (source.error().to_string(), source.location()))
            .collect::<Vec<_>>(),
        original_sources,
        "the callback's context, native sources, and locations must survive without added decode context"
    );
    assert_eq!(
        err.classify()
            .map(|classification| classification.class())
            .collect::<Vec<_>>(),
        original_classes,
        "decoding must preserve every callback error classification"
    );
    let native = err
        .downcast_any_ref::<std::io::Error>()
        .expect("the resolver's concrete I/O error remains available for recovery");
    assert_eq!(
        native.kind(),
        std::io::ErrorKind::PermissionDenied,
        "the native failure must not be replaced with a missing-base error"
    );
    assert!(
        native
            .get_ref()
            .expect("the native I/O error retains its payload")
            .is::<gix_error::Message>(),
        "the native source payload retains its concrete type"
    );
    assert_eq!(
        err.dominant_class(),
        Some(gix_error::Class::PermissionDenied),
        "base-resolution permissions remain actionable at the decode boundary"
    );
    assert!(!err.is_not_found(), "a resolver failure is not a missing delta base");
    assert!(
        err.downcast_any_ref::<data::decode::DeltaBaseUnresolved>().is_none(),
        "DeltaBaseUnresolved must only be constructed for a successful lookup returning None"
    );
    Ok(())
}

#[test]
fn ref_delta_resolver_none_is_an_unresolved_base() -> TestResult {
    let bytes = ref_delta_pack(&[1, 1, 0x90, 1])?;
    let pack = data::File::from_data(bytes, PathBuf::from("missing-base.pack"), gix_hash::Kind::Sha1)?;
    let resolutions = std::cell::Cell::new(0);
    let err = pack
        .decode_entry(
            pack.entry(FIRST_ENTRY_OFFSET)?,
            &mut Vec::new(),
            &mut Default::default(),
            &|_, _| {
                resolutions.set(resolutions.get() + 1);
                Ok(None)
            },
            &mut cache::Never,
        )
        .expect_err("Ok(None) means the ref-delta base could not be found");

    assert_eq!(resolutions.get(), 1, "a missing base must not trigger repeated lookups");
    assert!(err.is_not_found(), "an unresolved delta base remains a missing object");
    assert!(
        err.probable_cause().is::<data::decode::DeltaBaseUnresolved>(),
        "the unresolved base remains the typed probable cause"
    );
    assert_eq!(
        err.downcast_any_ref::<data::decode::DeltaBaseUnresolved>()
            .expect("the missing-base error retains its concrete type")
            .0,
        gix_hash::Kind::Sha1.null(),
        "the unresolved error identifies the requested synthetic base"
    );
    Ok(())
}

#[test]
fn ref_delta_resolver_some_decodes_external_base() -> TestResult {
    let bytes = ref_delta_pack(&[1, 1, 0x90, 1])?;
    let pack = data::File::from_data(bytes, PathBuf::from("external-base.pack"), gix_hash::Kind::Sha1)?;
    let mut out = Vec::new();
    let resolutions = std::cell::Cell::new(0);
    let decoded = pack.decode_entry(
        pack.entry(FIRST_ENTRY_OFFSET)?,
        &mut out,
        &mut Default::default(),
        &|base_id, out| {
            resolutions.set(resolutions.get() + 1);
            resolve_external_blob(base_id, out)
        },
        &mut cache::Never,
    )?;

    assert_eq!(resolutions.get(), 1, "the successful callback resolves the base once");
    assert_eq!(
        decoded.kind,
        gix_object::Kind::Blob,
        "the base determines the object kind"
    );
    assert_eq!(
        decoded.num_deltas, 1,
        "the external base completes the one-entry delta chain"
    );
    assert_eq!(
        decoded.object_size, 1,
        "the decoded size matches the delta result header"
    );
    assert_eq!(out, b"A", "Ok(Some(base)) preserves successful delta application");
    Ok(())
}

#[test]
fn ref_delta_header_resolver_error_is_propagated_unchanged() -> TestResult {
    let bytes = ref_delta_pack(&[1, 1, 0x90, 1])?;
    let pack = data::File::from_data(bytes, PathBuf::from("header-resolver-error.pack"), gix_hash::Kind::Sha1)?;
    let callback_error = std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        message("base storage is inaccessible").with("path", "external-base"),
    )
    .and_raise(message("could not resolve delta base header").with("operation", "base header lookup"));
    let original_sources: Vec<_> = callback_error
        .iter_errors_with_locations()
        .map(|source| (source.error().to_string(), source.location()))
        .collect();
    let original_classes: Vec<_> = callback_error
        .classify()
        .map(|classification| classification.class())
        .collect();
    let callback_error = std::cell::RefCell::new(Some(callback_error));
    let resolutions = std::cell::Cell::new(0);
    let err = pack
        .decode_header(pack.entry(FIRST_ENTRY_OFFSET)?, &mut Default::default(), &|base_id| {
            resolutions.set(resolutions.get() + 1);
            assert_eq!(
                base_id,
                gix_hash::Kind::Sha1.null().as_ref(),
                "the header resolver receives the synthetic external base ID"
            );
            Err(callback_error
                .borrow_mut()
                .take()
                .expect("the failing header resolver is called only once"))
        })
        .expect_err("a resolver failure must abort header decoding instead of becoming an unresolved base");

    assert_eq!(resolutions.get(), 1, "a failed base header lookup must not be retried");
    assert_eq!(
        err.iter_errors_with_locations()
            .map(|source| (source.error().to_string(), source.location()))
            .collect::<Vec<_>>(),
        original_sources,
        "the callback's context, native sources, and locations must survive without added header decode context"
    );
    assert_eq!(
        err.classify()
            .map(|classification| classification.class())
            .collect::<Vec<_>>(),
        original_classes,
        "header decoding must preserve every callback error classification"
    );
    let native = err
        .downcast_any_ref::<std::io::Error>()
        .expect("the header resolver's concrete I/O error remains available for recovery");
    assert_eq!(
        native.kind(),
        std::io::ErrorKind::PermissionDenied,
        "the native failure must not be replaced with a missing-base error"
    );
    assert!(
        native
            .get_ref()
            .expect("the native I/O error retains its payload")
            .is::<gix_error::Message>(),
        "the native source payload retains its concrete type"
    );
    assert_eq!(
        err.dominant_class(),
        Some(gix_error::Class::PermissionDenied),
        "base-resolution permissions remain actionable at the header decode boundary"
    );
    assert!(
        !err.is_not_found(),
        "a header resolver failure is not a missing delta base"
    );
    assert!(
        err.downcast_any_ref::<data::decode::DeltaBaseUnresolved>().is_none(),
        "DeltaBaseUnresolved must only be constructed for a successful header lookup returning None"
    );
    Ok(())
}

#[test]
fn ref_delta_header_resolver_none_is_an_unresolved_base() -> TestResult {
    let bytes = ref_delta_pack(&[1, 1, 0x90, 1])?;
    let pack = data::File::from_data(bytes, PathBuf::from("header-missing-base.pack"), gix_hash::Kind::Sha1)?;
    let resolutions = std::cell::Cell::new(0);
    let err = pack
        .decode_header(pack.entry(FIRST_ENTRY_OFFSET)?, &mut Default::default(), &|_| {
            resolutions.set(resolutions.get() + 1);
            Ok(None)
        })
        .expect_err("Ok(None) means the ref-delta base header could not be found");

    assert_eq!(
        resolutions.get(),
        1,
        "a missing base header must not trigger repeated lookups"
    );
    assert!(
        err.is_not_found(),
        "an unresolved delta base header remains a missing object"
    );
    assert!(
        err.probable_cause().is::<data::decode::DeltaBaseUnresolved>(),
        "the unresolved base remains the typed probable cause"
    );
    assert_eq!(
        err.downcast_any_ref::<data::decode::DeltaBaseUnresolved>()
            .expect("the missing-base error retains its concrete type")
            .0,
        gix_hash::Kind::Sha1.null(),
        "the unresolved header error identifies the requested synthetic base"
    );
    Ok(())
}

#[test]
fn ref_delta_header_resolver_some_decodes_external_base() -> TestResult {
    let bytes = ref_delta_pack(&[1, 1, 0x90, 1])?;
    let pack = data::File::from_data(bytes, PathBuf::from("header-external-base.pack"), gix_hash::Kind::Sha1)?;
    let resolutions = std::cell::Cell::new(0);
    let decoded = pack.decode_header(pack.entry(FIRST_ENTRY_OFFSET)?, &mut Default::default(), &|base_id| {
        resolutions.set(resolutions.get() + 1);
        resolve_external_header_blob(base_id)
    })?;

    assert_eq!(
        resolutions.get(),
        1,
        "the successful header callback resolves the base once"
    );
    assert_eq!(
        decoded.kind,
        gix_object::Kind::Blob,
        "the base header determines the object kind"
    );
    assert_eq!(
        decoded.num_deltas, 1,
        "the external base header completes the one-entry delta chain"
    );
    assert_eq!(
        decoded.object_size, 1,
        "the decoded size matches the delta result header"
    );
    Ok(())
}

#[test]
fn ref_delta_header_cycles_are_rejected() -> gix_testtools::TestResult {
    for (num_entries, close_with_ofs) in [(1, false), (2, false), (3, false), (2, true)] {
        let (pack, offsets) = ref_delta_chain(num_entries, &[0, 0], close_with_ofs)?;
        let resolutions = std::cell::Cell::new(0);
        let err = pack
            .decode_header(pack.entry(FIRST_ENTRY_OFFSET)?, &mut Default::default(), &|base_id| {
                resolutions.set(resolutions.get() + 1);
                assert!(resolutions.get() < 20, "a cyclic header lookup must terminate promptly");
                let offset = offsets[usize::from(base_id.as_bytes()[0]) % offsets.len()];
                Ok(Some(data::decode::header::ResolvedBase::InPack(
                    pack.entry(offset).expect("the synthetic delta entry exists"),
                )))
            })
            .expect_err("cyclic ref-delta bases must not be accepted");
        assert!(err.is_corrupted(), "a delta cycle is corrupt pack data: {err}");
    }
    Ok(())
}

#[test]
fn ref_delta_entry_cycles_are_rejected() -> gix_testtools::TestResult {
    for delta in [&[0, 0][..], &[][..]] {
        for (num_entries, close_with_ofs) in [(1, false), (2, false), (3, false), (2, true)] {
            let (pack, offsets) = ref_delta_chain(num_entries, delta, close_with_ofs)?;
            let resolutions = std::cell::Cell::new(0);
            let err = pack
                .decode_entry(
                    pack.entry(FIRST_ENTRY_OFFSET)?,
                    &mut Vec::new(),
                    &mut Default::default(),
                    &|base_id, _| {
                        resolutions.set(resolutions.get() + 1);
                        assert!(resolutions.get() < 20, "a cyclic object lookup must terminate promptly");
                        let offset = offsets[usize::from(base_id.as_bytes()[0]) % offsets.len()];
                        Ok(Some(data::decode::entry::ResolvedBase::InPack(
                            pack.entry(offset).expect("the synthetic delta entry exists"),
                        )))
                    },
                    &mut cache::Never,
                )
                .expect_err("even zero-sized cyclic deltas must be rejected");
            assert!(err.is_corrupted(), "a delta cycle is corrupt pack data: {err}");
        }
    }
    Ok(())
}

#[test]
fn forward_ref_delta_chain_is_accepted() -> gix_testtools::TestResult {
    let (pack, offsets) = ref_delta_chain(12, &[0, 0], false)?;
    let entry = pack.entry(FIRST_ENTRY_OFFSET)?;
    let header = pack.decode_header(entry.clone(), &mut Default::default(), &|base_id| {
        Ok(Some(match offsets.get(usize::from(base_id.as_bytes()[0])) {
            Some(&offset) => data::decode::header::ResolvedBase::InPack(
                pack.entry(offset).expect("the synthetic delta entry exists"),
            ),
            None => data::decode::header::ResolvedBase::OutOfPack {
                kind: gix_object::Kind::Blob,
                num_deltas: None,
            },
        }))
    })?;
    let mut out = Vec::new();
    let decoded = pack.decode_entry(
        entry,
        &mut out,
        &mut Default::default(),
        &|base_id, _| {
            Ok(Some(match offsets.get(usize::from(base_id.as_bytes()[0])) {
                Some(&offset) => data::decode::entry::ResolvedBase::InPack(
                    pack.entry(offset).expect("the synthetic delta entry exists"),
                ),
                None => data::decode::entry::ResolvedBase::OutOfPack {
                    kind: gix_object::Kind::Blob,
                    end: 0,
                },
            }))
        },
        &mut cache::Never,
    )?;
    assert_eq!(header.num_deltas, 12, "forward references are valid without a cycle");
    assert_eq!(decoded.num_deltas, 12, "all forward delta bases are resolved");
    assert!(out.is_empty(), "each delta preserves the empty base blob");
    Ok(())
}

/// Each entry refers to the next entry's numbered object ID. Tests resolve the final ID either
/// outside the pack or back to the first entry. An optional backward ofs-delta closes a mixed cycle.
/// The deliberately inflated object count ensures cycle detection cannot trust the pack header.
fn ref_delta_chain(
    num_entries: u8,
    delta: &[u8],
    close_with_ofs: bool,
) -> Result<(data::File<Vec<u8>>, Vec<data::Offset>)> {
    let mut pack = data::header::encode(data::Version::V2, 0).to_vec();
    let mut offsets = Vec::new();
    for index in 0..num_entries {
        let offset = pack.len() as u64;
        offsets.push(offset);
        let header = if close_with_ofs && index + 1 == num_entries {
            data::entry::Header::OfsDelta {
                base_distance: offset - FIRST_ENTRY_OFFSET,
            }
        } else {
            data::entry::Header::RefDelta {
                base_id: gix_hash::ObjectId::from_bytes_or_panic(&[index + 1; 20]),
            }
        };
        header.write_to(delta.len() as u64, &mut pack)?;
        pack.extend(deflate(delta)?);
    }
    let max_entries_for_size = (pack.len() - data::header::SIZE) as u32;
    pack[8..12].copy_from_slice(&max_entries_for_size.to_be_bytes());
    pack.extend([0; 20]);
    Ok((
        data::File::from_data(pack, PathBuf::from("ref-delta-chain.pack"), gix_hash::Kind::Sha1)?,
        offsets,
    ))
}

/// Reproducer for GHSA-x494-mj8g-cj27: malformed delta copy instructions currently reach
/// `gix_pack::data::File::decode_entry()` and panic while slicing the base object instead of
/// returning an error for attacker-controlled pack data.
#[test]
fn delta_copy_is_reported_without_panicking() -> gix_testtools::TestResult {
    let pack_data = ref_delta_pack(&[1, 2, 0x90, 0x02])?;
    let pack = data::File::from_data(pack_data, PathBuf::from("malformed.pack"), gix_hash::Kind::Sha1)?;
    let entry = pack.entry(FIRST_ENTRY_OFFSET)?;
    let mut out = Vec::new();
    let mut inflate = gix_zlib::Inflate::default();

    let result = catch_unwind(AssertUnwindSafe(|| {
        pack.decode_entry(entry, &mut out, &mut inflate, &resolve_external_blob, &mut cache::Never)
    }));

    assert!(
        result
            .expect("malformed delta instructions should produce an error instead of panicking")
            .is_err(),
        "malformed delta instructions should be rejected"
    );
    Ok(())
}

/// Reproducer for GHSA-x494-mj8g-cj27: a delta that declares a result size above `isize::MAX`
/// currently reaches `gix_pack::data::File::decode_entry()` and panics with a capacity overflow
/// instead of rejecting the attacker-controlled size header.
#[test]
#[cfg(target_pointer_width = "64")]
fn oversized_delta_result_is_rejected_without_panicking() -> gix_testtools::TestResult {
    let mut delta = encode_delta_size(1);
    delta.extend(encode_delta_size(isize::MAX as u64 + 1));

    let pack_data = ref_delta_pack(&delta)?;
    let pack = data::File::from_data(pack_data, PathBuf::from("malformed.pack"), gix_hash::Kind::Sha1)?;
    let entry = pack.entry(FIRST_ENTRY_OFFSET)?;
    let mut out = Vec::new();
    let mut inflate = gix_zlib::Inflate::default();

    let result = catch_unwind(AssertUnwindSafe(|| {
        pack.decode_entry(entry, &mut out, &mut inflate, &resolve_external_blob, &mut cache::Never)
    }));

    assert!(
        result
            .expect("oversized delta result headers should be rejected instead of panicking")
            .is_err(),
        "oversized delta result headers should not be accepted"
    );
    Ok(())
}

/// A delta entry can declare more decompressed bytes than zlib actually produces. Header parsing
/// must only inspect the produced bytes, not the zero-filled remainder of the output buffer.
#[test]
fn truncated_delta_header_ignores_zero_filled_remainder() -> gix_testtools::TestResult {
    let pack_data = ref_delta_pack_with_declared_size(&[1, 0x80], 3)?;
    let pack = data::File::from_data(pack_data, PathBuf::from("malformed.pack"), gix_hash::Kind::Sha1)?;
    let entry = pack.entry(FIRST_ENTRY_OFFSET)?;
    let mut out = Vec::new();
    let mut inflate = gix_zlib::Inflate::default();

    let res = pack.decode_entry(entry, &mut out, &mut inflate, &resolve_external_blob, &mut cache::Never);

    assert!(
        res.is_err(),
        "truncated delta headers should not be completed by zero-filled output"
    );
    Ok(())
}

#[test]
fn complete_delta_with_mismatched_declared_size_is_rejected() -> gix_testtools::TestResult {
    for (name, delta, decompressed_size) in [
        ("shorter", &[1, 1, 0x90, 1][..], 5),
        ("longer", &[1, 1, 0x90, 1, 0][..], 4),
    ] {
        let pack_data = ref_delta_pack_with_declared_size(delta, decompressed_size)?;
        let pack = data::File::from_data(pack_data, PathBuf::from("malformed.pack"), gix_hash::Kind::Sha1)?;
        let entry = pack.entry(FIRST_ENTRY_OFFSET)?;
        let mut out = Vec::new();
        let mut inflate = gix_zlib::Inflate::default();

        let res = pack.decode_entry(entry, &mut out, &mut inflate, &resolve_external_blob, &mut cache::Never);

        assert!(
            res.is_err(),
            "delta streams {name} than their declared size should be rejected"
        );
    }
    Ok(())
}

#[test]
fn plain_object_with_mismatched_declared_size_is_rejected() -> gix_testtools::TestResult {
    let mut diagnostics = Vec::new();
    for (blob, decompressed_size) in [(b"A".as_slice(), 2), (b"AB".as_slice(), 1)] {
        let pack_data = blob_pack_with_declared_size(blob, decompressed_size)?;
        let pack = data::File::from_data(pack_data, PathBuf::from("malformed.pack"), gix_hash::Kind::Sha1)?;
        let entry = pack.entry(FIRST_ENTRY_OFFSET)?;
        let mut out = Vec::new();
        let mut inflate = gix_zlib::Inflate::default();

        let res = pack.decode_entry(entry, &mut out, &mut inflate, &resolve_external_blob, &mut cache::Never);

        diagnostics.push(res.expect_err("malformed pack data must be rejected"));
    }
    insta::assert_debug_snapshot!(diagnostics, "plain object with mismatched declared size is rejected", @"
    [
        Pack entry is truncated: pack entry decompressed size does not match entry header,
        Pack entry is truncated: pack entry decompressed size does not match entry header,
    ]
    ");
    Ok(())
}

#[test]
fn empty_plain_object_is_accepted() -> gix_testtools::TestResult {
    let pack_data = blob_pack_with_declared_size(b"", 0)?;
    let pack = data::File::from_data(pack_data, PathBuf::from("malformed.pack"), gix_hash::Kind::Sha1)?;
    let entry = pack.entry(FIRST_ENTRY_OFFSET)?;
    let mut out = Vec::new();
    let mut inflate = gix_zlib::Inflate::default();

    let res = pack.decode_entry(entry, &mut out, &mut inflate, &resolve_external_blob, &mut cache::Never)?;

    assert_eq!(res.kind, gix_object::Kind::Blob);
    assert_eq!(res.object_size, 0);
    assert!(out.is_empty());
    Ok(())
}

#[test]
fn delta_with_mismatched_base_size_is_rejected_before_allocating_work_buffers() -> gix_testtools::TestResult {
    let declared_base_size = 1_000_000;
    let mut delta = encode_delta_size(declared_base_size);
    delta.extend([1, 0x90, 1]);
    let pack_data = ref_delta_pack(&delta)?;
    let pack = data::File::from_data(pack_data, PathBuf::from("malformed.pack"), gix_hash::Kind::Sha1)?;
    let entry = pack.entry(FIRST_ENTRY_OFFSET)?;
    let mut out = Vec::new();
    let mut inflate = gix_zlib::Inflate::default();

    let res = pack.decode_entry(entry, &mut out, &mut inflate, &resolve_external_blob, &mut cache::Never);

    insta::assert_debug_snapshot!(res.expect_err("malformed pack data must be rejected"), "delta with mismatched base size is rejected before allocating work buffers", @"Corrupt delta data: delta base size does not match base object size");
    assert!(
        out.capacity() < declared_base_size as usize,
        "the invalid declared base size must not determine work-buffer capacity"
    );
    Ok(())
}

#[test]
fn in_pack_delta_base_with_mismatched_declared_size_is_rejected() -> gix_testtools::TestResult {
    let (pack_data, delta_offset) = ofs_delta_pack_with_mismatched_base_size()?;
    let pack = data::File::from_data(
        pack_data.as_slice(),
        PathBuf::from("malformed.pack"),
        gix_hash::Kind::Sha1,
    )?;
    let entry = pack.entry(delta_offset)?;
    let mut out = Vec::new();
    let mut inflate = gix_zlib::Inflate::default();

    let res = pack.decode_entry(entry, &mut out, &mut inflate, &resolve_external_blob, &mut cache::Never);

    insta::assert_debug_snapshot!(res.expect_err("malformed pack data must be rejected"), "in pack delta base with mismatched declared size is rejected", @"Corrupt delta data: delta base size does not match base object size");
    Ok(())
}

#[test]
fn decode_header_ignores_zero_filled_delta_remainder() -> gix_testtools::TestResult {
    let pack_data = ref_delta_pack_with_declared_size(&[1, 0x80], 3)?;
    let pack = data::File::from_data(pack_data, PathBuf::from("malformed.pack"), gix_hash::Kind::Sha1)?;
    let entry = pack.entry(FIRST_ENTRY_OFFSET)?;
    let mut inflate = gix_zlib::Inflate::default();

    let res = pack.decode_header(entry, &mut inflate, &resolve_external_header_blob);

    insta::assert_debug_snapshot!(res.expect_err("malformed pack data must be rejected"), "decode header ignores zero filled delta remainder", @"Pack entry is truncated: pack entry decompressed to fewer bytes than declared in the entry header");
    Ok(())
}

#[test]
fn decode_header_with_mismatched_declared_delta_size_is_rejected() -> gix_testtools::TestResult {
    let mut diagnostics = Vec::new();
    for (delta, decompressed_size) in [(&[1, 1, 0x90, 1][..], 5), (&[1, 1, 0x90, 1, 0][..], 4)] {
        let pack_data = ref_delta_pack_with_declared_size(delta, decompressed_size)?;
        let pack = data::File::from_data(pack_data, PathBuf::from("malformed.pack"), gix_hash::Kind::Sha1)?;
        let entry = pack.entry(FIRST_ENTRY_OFFSET)?;
        let mut inflate = gix_zlib::Inflate::default();

        let res = pack.decode_header(entry, &mut inflate, &resolve_external_header_blob);

        diagnostics.push(res.expect_err("malformed pack data must be rejected"));
    }
    insta::assert_debug_snapshot!(diagnostics, "decode header with mismatched declared delta size is rejected", @"
    [
        Pack entry is truncated: pack entry decompressed to fewer bytes than declared in the entry header,
        Pack entry is truncated: pack entry decompressed to more bytes than declared in the entry header,
    ]
    ");
    Ok(())
}

#[test]
fn large_mismatched_declared_delta_size_is_rejected_during_full_decode() -> gix_testtools::TestResult {
    let mut diagnostics = Vec::new();
    for (delta_len, decompressed_size) in [(33, 34), (34, 33)] {
        let delta = padded_delta(delta_len);
        let pack_data = ref_delta_pack_with_declared_size(&delta, decompressed_size)?;
        let pack = data::File::from_data(pack_data, PathBuf::from("malformed.pack"), gix_hash::Kind::Sha1)?;
        let entry = pack.entry(FIRST_ENTRY_OFFSET)?;
        let mut inflate = gix_zlib::Inflate::default();

        let outcome = pack.decode_header(entry.clone(), &mut inflate, &resolve_external_header_blob)?;

        assert_eq!(outcome.object_size, 1);

        let mut out = Vec::new();
        let res = pack.decode_entry(entry, &mut out, &mut inflate, &resolve_external_blob, &mut cache::Never);

        diagnostics.push(res.expect_err("malformed pack data must be rejected"));
    }
    insta::assert_debug_snapshot!(diagnostics, "large mismatched declared delta size is rejected during full decode", @"
    [
        Pack entry is truncated: pack entry decompressed size does not match entry header,
        Pack entry is truncated: pack entry decompressed size does not match entry header,
    ]
    ");
    Ok(())
}

fn padded_delta(len: usize) -> Vec<u8> {
    let mut delta = vec![1, 1, 0x90, 1];
    delta.resize(len, 0);
    delta
}

fn encode_delta_size(mut size: u64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let mut byte = (size & 0x7f) as u8;
        size >>= 7;
        if size != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if size == 0 {
            break;
        }
    }
    out
}

fn deflate(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut write = gix_zlib::stream::deflate::Write::new(Vec::new(), gix_zlib::Compression::BEST_SPEED);
    write.write_all(bytes)?;
    write.flush()?;
    Ok(write.into_inner())
}

/// Build a one-entry blob pack whose zlib payload comes from `blob`, while the pack entry header
/// declares `decompressed_size`.
///
/// This creates malformed plain-object fixtures that exercise the same header-vs-stream size
/// validation as delta fixtures, but without involving base-object resolution or delta parsing.
fn blob_pack_with_declared_size(blob: &[u8], decompressed_size: u64) -> Result<Vec<u8>> {
    let mut pack = Vec::new();
    pack.extend_from_slice(&data::header::encode(data::Version::V2, 1));
    data::entry::Header::Blob.write_to(decompressed_size, &mut pack)?;
    pack.extend(deflate(blob)?);
    pack.extend([0; 20]);
    Ok(pack)
}

/// Build a two-entry pack and return the offset of its ofs-delta entry pointing at an in-pack blob base.
///
/// The base blob header declares two decompressed bytes, but its zlib payload only produces `b"A"`.
/// The delta itself declares a base size of one byte, so this fixture verifies that decoding uses
/// the in-pack base entry's declared size when allocating the base buffer and rejects the base
/// stream mismatch instead of slicing past the buffer.
fn ofs_delta_pack_with_mismatched_base_size() -> Result<(Vec<u8>, data::Offset)> {
    let mut pack = Vec::new();
    pack.extend_from_slice(&data::header::encode(data::Version::V2, 2));

    let base_offset = pack.len() as u64;
    data::entry::Header::Blob.write_to(2, &mut pack)?;
    pack.extend(deflate(b"A")?);

    let delta = [1, 1, 0x90, 1];
    let delta_offset = pack.len() as u64;
    data::entry::Header::OfsDelta {
        base_distance: delta_offset - base_offset,
    }
    .write_to(delta.len() as u64, &mut pack)?;
    pack.extend(deflate(&delta)?);
    pack.extend([0; 20]);
    Ok((pack, delta_offset))
}

fn ref_delta_pack(delta: &[u8]) -> Result<Vec<u8>> {
    ref_delta_pack_with_declared_size(delta, delta.len() as u64)
}

/// Build a one-entry ref-delta pack whose zlib payload comes from `delta`, while the pack entry
/// header declares `decompressed_size`.
///
/// Malformed packs can lie in either direction: the header may promise more bytes than inflate
/// produces, leaving zero-filled slack in the caller's output buffer, or it may promise fewer
/// bytes than the stream actually contains. The dedicated helper keeps those tests explicit,
/// while `ref_delta_pack()` remains the shorthand for internally consistent fixtures.
fn ref_delta_pack_with_declared_size(delta: &[u8], decompressed_size: u64) -> Result<Vec<u8>> {
    let mut pack = Vec::new();
    pack.extend_from_slice(&data::header::encode(data::Version::V2, 1));
    data::entry::Header::RefDelta {
        base_id: gix_hash::Kind::Sha1.null(),
    }
    .write_to(decompressed_size, &mut pack)?;
    pack.extend(deflate(delta)?);
    pack.extend([0; 20]);
    Ok(pack)
}

fn resolve_external_blob(
    _id: &gix_hash::oid,
    out: &mut Vec<u8>,
) -> gix_error::Result<Option<data::decode::entry::ResolvedBase>> {
    out.clear();
    out.extend_from_slice(b"A");
    Ok(Some(data::decode::entry::ResolvedBase::OutOfPack {
        kind: gix_object::Kind::Blob,
        end: 1,
    }))
}

/// Resolve the synthetic ref-delta base for `decode_header()` tests.
///
/// Header decoding uses a resolver that only reports base metadata, unlike `decode_entry()`,
/// which also needs the base bytes in `out`. Providing this resolver lets malformed ref-delta
/// fixtures reach delta-header parsing without failing earlier on the unresolved `_id`.
fn resolve_external_header_blob(_id: &gix_hash::oid) -> gix_error::Result<Option<data::decode::header::ResolvedBase>> {
    Ok(Some(data::decode::header::ResolvedBase::OutOfPack {
        kind: gix_object::Kind::Blob,
        num_deltas: None,
    }))
}
