use gix_error::ExnMessageResult;
use gix_hash::{Hasher, ObjectId};
use gix_testtools::size_ok;

#[test]
fn interruption_preserves_its_io_error_kind() {
    let err = gix_hash::bytes(
        &mut &b"x"[..],
        1,
        gix_hash::Kind::shortest(),
        &mut gix_features::progress::Discard,
        &std::sync::atomic::AtomicBool::new(true),
    )
    .expect_err("the interrupt flag is observed after reading a chunk");
    insta::assert_debug_snapshot!(err, "interruption preserves its io error kind", @"
    I/O error (Interrupted)
    |
    └─ Interrupted
    ");
    assert_eq!(
        err.downcast_any_ref::<std::io::Error>().map(std::io::Error::kind),
        Some(std::io::ErrorKind::Interrupted)
    );
}

#[test]
fn size_of_hasher_sha1_only() {
    let actual = std::mem::size_of::<Hasher>();
    let expected = 112;
    assert!(
        size_ok(actual, expected),
        "The size of this type may be relevant when hashing millions of objects, and shouldn't\
        change unnoticed: {actual} <~ {expected}\
        (The DetectionState alone clocked in at 724 bytes when last examined.)"
    );
}

#[test]
#[cfg(all(feature = "sha256", feature = "sha1"))]
fn size_of_hasher_sha1_and_sha256() {
    let actual = std::mem::size_of::<Hasher>();
    let expected = 112;
    assert!(
        size_ok(actual, expected),
        "The size of this type may be relevant when hashing millions of objects, and shouldn't\
        change unnoticed: {actual} <~ {expected}\
        (The DetectionState alone clocked in at 724 bytes when last examined.)"
    );
}

#[test]
fn size_of_try_finalize_return_type() {
    let actual = std::mem::size_of::<ExnMessageResult<ObjectId>>();
    assert!(
        size_ok(actual, 40),
        "The return value should stay within its 40-byte 64-bit baseline: {actual}"
    );
}
