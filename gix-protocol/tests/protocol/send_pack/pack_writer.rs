use gix_protocol::send_pack::pack_writer::write_pack;

#[test]
fn empty_pack_has_header_and_trailer_sha1() {
    let mut out = Vec::new();
    let trailer = write_pack(std::iter::empty(), gix_hash::Kind::Sha1, &mut out).expect("write empty pack");
    // 12-byte header + 20-byte sha1 trailer = 32 bytes.
    assert_eq!(out.len(), 32);
    assert_eq!(&out[..4], b"PACK");
    assert_eq!(&out[4..8], &[0, 0, 0, 2]); // version 2
    assert_eq!(&out[8..12], &[0, 0, 0, 0]); // 0 objects
                                            // Trailer must be the hash of the first 12 bytes.
    let expected = {
        let mut h = gix_hash::hasher(gix_hash::Kind::Sha1);
        h.update(&out[..12]);
        h.try_finalize().expect("finalize")
    };
    assert_eq!(&out[12..], expected.as_slice());
    assert_eq!(trailer, expected);
}

#[cfg(feature = "sha256")]
#[test]
fn empty_pack_trailer_width_is_parametric_for_sha256() {
    let mut out = Vec::new();
    let _trailer = write_pack(std::iter::empty(), gix_hash::Kind::Sha256, &mut out).expect("write empty pack sha256");
    // 12-byte header + 32-byte sha256 trailer = 44 bytes.
    assert_eq!(out.len(), 44);
    assert_eq!(&out[..4], b"PACK");
}
