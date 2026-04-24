use std::{
    io::Write,
    panic::{catch_unwind, AssertUnwindSafe},
};

use gix_features::zlib;
use gix_pack::{cache, data};
use gix_testtools::tempfile;

/// Reproducer for GHSA-x494-mj8g-cj27: malformed delta copy instructions currently reach
/// `gix_pack::data::File::decode_entry()` and panic while slicing the base object instead of
/// returning an error for attacker-controlled pack data.
#[test]
fn delta_copy_is_reported_without_panicking() -> crate::Result {
    let (_temp, pack, entry) = ref_delta_pack(&[1, 2, 0x90, 0x02])?;
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

    let (_temp, pack, entry) = ref_delta_pack(&delta)?;
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

fn ref_delta_pack(delta: &[u8]) -> crate::Result<(tempfile::TempDir, data::File, data::Entry)> {
    fn deflate(bytes: &[u8]) -> crate::Result<Vec<u8>> {
        let mut write = gix_features::zlib::stream::deflate::Write::new(Vec::new());
        write.write_all(bytes)?;
        write.flush()?;
        Ok(write.into_inner())
    }

    let tmp = tempfile::tempdir()?;
    let path = tmp.path().join("malformed.pack");

    let mut pack = Vec::new();
    pack.extend_from_slice(&data::header::encode(data::Version::V2, 1));
    data::entry::Header::RefDelta {
        base_id: gix_hash::Kind::Sha1.null(),
    }
    .write_to(delta.len() as u64, &mut pack)?;
    pack.extend(deflate(delta)?);
    pack.extend([0; 20]);

    std::fs::write(&path, pack)?;
    let file = data::File::at(&path, gix_hash::Kind::Sha1)?;
    let entry = file.entry(12)?;
    Ok((tmp, file, entry))
}

fn resolve_external_blob(_id: &gix_hash::oid, out: &mut Vec<u8>) -> Option<data::decode::entry::ResolvedBase> {
    out.clear();
    out.extend_from_slice(b"A");
    Some(data::decode::entry::ResolvedBase::OutOfPack {
        kind: gix_object::Kind::Blob,
        end: 1,
    })
}
