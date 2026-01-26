mod blob;
mod commit;
mod tree;

use gix_testtools::size_ok;

#[test]
fn object_ref_size_in_memory() {
    let actual = std::mem::size_of::<gix::Object<'_>>();
    let expected = 72;
    assert!(
        size_ok(actual, expected),
        "the size of this structure should not change unexpectedly: {actual} <~ {expected}"
    );
}

#[test]
fn oid_size_in_memory() {
    let actual = std::mem::size_of::<gix::Id<'_>>();
    let expected = 48;
    assert!(
        size_ok(actual, expected),
        "the size of this structure should not change unexpectedly: {actual} <~ {expected}"
    );
}
