use std::{
    io::Write,
    panic::{AssertUnwindSafe, catch_unwind},
    path::PathBuf,
};

use gix_features::zlib;
use gix_pack::{cache, data};

/// Reproducer for GHSA-x494-mj8g-cj27: malformed delta copy instructions currently reach
/// `gix_pack::data::File::decode_entry()` and panic while slicing the base object instead of
/// returning an error for attacker-controlled pack data.
#[test]
fn delta_copy_is_reported_without_panicking() -> crate::Result {
    let pack_data = ref_delta_pack(&[1, 2, 0x90, 0x02])?;
    let pack = data::File::from_data(pack_data, PathBuf::from("malformed.pack"), gix_hash::Kind::Sha1)?;
    let entry = pack.entry(12)?;
    let mut out = Vec::new();
    let mut inflate = zlib::Inflate::default();

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
fn oversized_delta_result_is_rejected_without_panicking() -> crate::Result {
    let mut delta = encode_delta_size(1);
    delta.extend(encode_delta_size(isize::MAX as u64 + 1));

    let pack_data = ref_delta_pack(&delta)?;
    let pack = data::File::from_data(pack_data, PathBuf::from("malformed.pack"), gix_hash::Kind::Sha1)?;
    let entry = pack.entry(12)?;
    let mut out = Vec::new();
    let mut inflate = zlib::Inflate::default();

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
fn truncated_delta_header_ignores_zero_filled_remainder() -> crate::Result {
    let pack_data = ref_delta_pack_with_declared_size(&[1, 0x80], 3)?;
    let pack = data::File::from_data(pack_data, PathBuf::from("malformed.pack"), gix_hash::Kind::Sha1)?;
    let entry = pack.entry(12)?;
    let mut out = Vec::new();
    let mut inflate = zlib::Inflate::default();

    let res = pack.decode_entry(entry, &mut out, &mut inflate, &resolve_external_blob, &mut cache::Never);

    assert!(
        res.is_err(),
        "truncated delta headers should not be completed by zero-filled output"
    );
    Ok(())
}

#[test]
fn complete_delta_with_mismatched_declared_size_is_rejected() -> crate::Result {
    for (name, delta, decompressed_size) in [
        ("shorter", &[1, 1, 0x90, 1][..], 5),
        ("longer", &[1, 1, 0x90, 1, 0][..], 4),
    ] {
        let pack_data = ref_delta_pack_with_declared_size(delta, decompressed_size)?;
        let pack = data::File::from_data(pack_data, PathBuf::from("malformed.pack"), gix_hash::Kind::Sha1)?;
        let entry = pack.entry(12)?;
        let mut out = Vec::new();
        let mut inflate = zlib::Inflate::default();

        let res = pack.decode_entry(entry, &mut out, &mut inflate, &resolve_external_blob, &mut cache::Never);

        assert!(
            res.is_err(),
            "delta streams {name} than their declared size should be rejected"
        );
    }
    Ok(())
}

#[test]
fn plain_object_with_mismatched_declared_size_is_rejected() -> crate::Result {
    for (blob, decompressed_size) in [(b"A".as_slice(), 2), (b"AB".as_slice(), 1)] {
        let pack_data = blob_pack_with_declared_size(blob, decompressed_size)?;
        let pack = data::File::from_data(pack_data, PathBuf::from("malformed.pack"), gix_hash::Kind::Sha1)?;
        let entry = pack.entry(12)?;
        let mut out = Vec::new();
        let mut inflate = zlib::Inflate::default();

        let res = pack.decode_entry(entry, &mut out, &mut inflate, &resolve_external_blob, &mut cache::Never);

        assert_err_message(
            res,
            "Pack entry is truncated: pack entry decompressed size does not match entry header",
        );
    }
    Ok(())
}

#[test]
fn empty_plain_object_is_accepted() -> crate::Result {
    let pack_data = blob_pack_with_declared_size(b"", 0)?;
    let pack = data::File::from_data(pack_data, PathBuf::from("malformed.pack"), gix_hash::Kind::Sha1)?;
    let entry = pack.entry(12)?;
    let mut out = Vec::new();
    let mut inflate = zlib::Inflate::default();

    let res = pack.decode_entry(entry, &mut out, &mut inflate, &resolve_external_blob, &mut cache::Never)?;

    assert_eq!(res.kind, gix_object::Kind::Blob);
    assert_eq!(res.object_size, 0);
    assert!(out.is_empty());
    Ok(())
}

#[test]
fn in_pack_delta_base_with_mismatched_declared_size_is_rejected() -> crate::Result {
    let (pack_data, delta_offset) = ofs_delta_pack_with_mismatched_base_size()?;
    let pack = data::File::from_data(
        pack_data.as_slice(),
        PathBuf::from("malformed.pack"),
        gix_hash::Kind::Sha1,
    )?;
    let entry = pack.entry(delta_offset)?;
    let mut out = Vec::new();
    let mut inflate = zlib::Inflate::default();

    let res = pack.decode_entry(entry, &mut out, &mut inflate, &resolve_external_blob, &mut cache::Never);

    assert_err_message(
        res,
        "Pack entry is truncated: pack entry decompressed size does not match entry header",
    );
    Ok(())
}

#[test]
fn decode_header_ignores_zero_filled_delta_remainder() -> crate::Result {
    let pack_data = ref_delta_pack_with_declared_size(&[1, 0x80], 3)?;
    let pack = data::File::from_data(pack_data, PathBuf::from("malformed.pack"), gix_hash::Kind::Sha1)?;
    let entry = pack.entry(12)?;
    let mut inflate = zlib::Inflate::default();

    let res = pack.decode_header(entry, &mut inflate, &resolve_external_header_blob);

    assert_err_message(
        res,
        "Pack entry is truncated: pack entry decompressed to fewer bytes than declared in the entry header",
    );
    Ok(())
}

#[test]
fn decode_header_with_mismatched_declared_delta_size_is_rejected() -> crate::Result {
    for (delta, decompressed_size, message) in [
        (
            &[1, 1, 0x90, 1][..],
            5,
            "Pack entry is truncated: pack entry decompressed to fewer bytes than declared in the entry header",
        ),
        (
            &[1, 1, 0x90, 1, 0][..],
            4,
            "Pack entry is truncated: pack entry decompressed to more bytes than declared in the entry header",
        ),
    ] {
        let pack_data = ref_delta_pack_with_declared_size(delta, decompressed_size)?;
        let pack = data::File::from_data(pack_data, PathBuf::from("malformed.pack"), gix_hash::Kind::Sha1)?;
        let entry = pack.entry(12)?;
        let mut inflate = zlib::Inflate::default();

        let res = pack.decode_header(entry, &mut inflate, &resolve_external_header_blob);

        assert_err_message(res, message);
    }
    Ok(())
}

#[test]
fn large_mismatched_declared_delta_size_is_rejected_during_full_decode() -> crate::Result {
    for (delta_len, decompressed_size) in [(33, 34), (34, 33)] {
        let delta = padded_delta(delta_len);
        let pack_data = ref_delta_pack_with_declared_size(&delta, decompressed_size)?;
        let pack = data::File::from_data(pack_data, PathBuf::from("malformed.pack"), gix_hash::Kind::Sha1)?;
        let entry = pack.entry(12)?;
        let mut inflate = zlib::Inflate::default();

        let outcome = pack.decode_header(entry.clone(), &mut inflate, &resolve_external_header_blob)?;

        assert_eq!(outcome.object_size, 1);

        let mut out = Vec::new();
        let res = pack.decode_entry(entry, &mut out, &mut inflate, &resolve_external_blob, &mut cache::Never);

        assert_err_message(
            res,
            "Pack entry is truncated: pack entry decompressed size does not match entry header",
        );
    }
    Ok(())
}

fn assert_err_message<T, E>(res: Result<T, E>, expected: &str)
where
    E: std::fmt::Debug + std::fmt::Display,
{
    match res {
        Ok(_) => panic!("operation should fail"),
        Err(err) => assert_eq!(err.to_string(), expected),
    }
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

fn deflate(bytes: &[u8]) -> crate::Result<Vec<u8>> {
    let mut write = gix_features::zlib::stream::deflate::Write::new(Vec::new());
    write.write_all(bytes)?;
    write.flush()?;
    Ok(write.into_inner())
}

/// Build a one-entry blob pack whose zlib payload comes from `blob`, while the pack entry header
/// declares `decompressed_size`.
///
/// This creates malformed plain-object fixtures that exercise the same header-vs-stream size
/// validation as delta fixtures, but without involving base-object resolution or delta parsing.
fn blob_pack_with_declared_size(blob: &[u8], decompressed_size: u64) -> crate::Result<Vec<u8>> {
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
fn ofs_delta_pack_with_mismatched_base_size() -> crate::Result<(Vec<u8>, data::Offset)> {
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

fn ref_delta_pack(delta: &[u8]) -> crate::Result<Vec<u8>> {
    ref_delta_pack_with_declared_size(delta, delta.len() as u64)
}

/// Build a one-entry ref-delta pack whose zlib payload comes from `delta`, while the pack entry
/// header declares `decompressed_size`.
///
/// Malformed packs can lie in either direction: the header may promise more bytes than inflate
/// produces, leaving zero-filled slack in the caller's output buffer, or it may promise fewer
/// bytes than the stream actually contains. The dedicated helper keeps those tests explicit,
/// while `ref_delta_pack()` remains the shorthand for internally consistent fixtures.
fn ref_delta_pack_with_declared_size(delta: &[u8], decompressed_size: u64) -> crate::Result<Vec<u8>> {
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

fn resolve_external_blob(_id: &gix_hash::oid, out: &mut Vec<u8>) -> Option<data::decode::entry::ResolvedBase> {
    out.clear();
    out.extend_from_slice(b"A");
    Some(data::decode::entry::ResolvedBase::OutOfPack {
        kind: gix_object::Kind::Blob,
        end: 1,
    })
}

/// Resolve the synthetic ref-delta base for `decode_header()` tests.
///
/// Header decoding uses a resolver that only reports base metadata, unlike `decode_entry()`,
/// which also needs the base bytes in `out`. Providing this resolver lets malformed ref-delta
/// fixtures reach delta-header parsing without failing earlier on the unresolved `_id`.
fn resolve_external_header_blob(_id: &gix_hash::oid) -> Option<data::decode::header::ResolvedBase> {
    Some(data::decode::header::ResolvedBase::OutOfPack {
        kind: gix_object::Kind::Blob,
        num_deltas: None,
    })
}
