mod blob;
mod commit;
mod tree;

use gix_testtools::size_ok;

#[test]
fn object_ref_size_in_memory() {
    let actual = std::mem::size_of::<gix::Object<'_>>();
    let sha1 = 56;
    let sha256_extra = 16;
    let expected = sha1 + sha256_extra;
    assert!(
        size_ok(actual, expected),
        "the size of this structure should not change unexpectedly: {actual} <~ {expected}"
    );
}

#[test]
fn oid_size_in_memory() {
    let actual = std::mem::size_of::<gix::Id<'_>>();
    let sha1 = 32;
    let sha256_extra = 16;
    let expected = sha1 + sha256_extra;
    assert!(
        size_ok(actual, expected),
        "the size of this structure should not change unexpectedly: {actual} <~ {expected}"
    );
}
