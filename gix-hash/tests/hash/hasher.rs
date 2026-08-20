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
    assert_eq!(
        err.downcast_any_ref::<std::io::Error>().map(std::io::Error::kind),
        Some(std::io::ErrorKind::Interrupted)
    );
}

#[test]
fn size_of_hasher_sha1_only() {
    let actual = std::mem::size_of::<Hasher>();
    let expected = 824;
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
    let expected = 824;
    assert!(
        size_ok(actual, expected),
        "The size of this type may be relevant when hashing millions of objects, and shouldn't\
        change unnoticed: {actual} <~ {expected}\
        (The DetectionState alone clocked in at 724 bytes when last examined.)"
    );
}

#[test]
#[cfg(all(not(feature = "sha256"), feature = "sha1"))]
fn size_of_try_finalize_return_type_sha1_only() {
    assert_eq!(
        std::mem::size_of::<Result<ObjectId, gix_hash::hasher::Error>>(),
        32,
        "The size of the return value should remain compact"
    );
}

#[test]
#[cfg(all(feature = "sha256", feature = "sha1"))]
fn size_of_try_finalize_return_type_sha1_and_sha256() {
    assert_eq!(
        std::mem::size_of::<Result<ObjectId, gix_hash::hasher::Error>>(),
        32 + std::mem::size_of::<usize>(),
        "The size of the return value should remain compact"
    );
}
