use gix_reftable::basics::{
    common_prefix_size, get_be16, get_be24, get_be32, get_be64, put_be16, put_be24, put_be32, put_be64,
};

// Upstream mapping: test_reftable_basics__common_prefix_size
#[test]
fn common_prefix() {
    assert_eq!(common_prefix_size(b"refs/heads/a", b"refs/heads/b"), 11);
}

// Upstream mapping: put_get_be64/be32/be24/be16 tests
#[test]
fn big_endian_roundtrip() {
    let mut be64 = [0u8; 8];
    put_be64(&mut be64, 0x0102_0304_0506_0708);
    assert_eq!(get_be64(&be64), 0x0102_0304_0506_0708);

    let mut be32 = [0u8; 4];
    put_be32(&mut be32, 0x0102_0304);
    assert_eq!(get_be32(&be32), 0x0102_0304);

    let mut be24 = [0u8; 3];
    put_be24(&mut be24, 0x01_02_03);
    assert_eq!(get_be24(&be24), 0x01_02_03);

    let mut be16 = [0u8; 2];
    put_be16(&mut be16, 0x0102);
    assert_eq!(get_be16(&be16), 0x0102);
}
