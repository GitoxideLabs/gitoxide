use std::path::Path;

use gix_testtools::fixture_path;

use crate::{file::store_with_packed_refs, packed::write_packed_refs_with};

const HASH_KIND: gix_hash::Kind = gix_hash::Kind::Sha1;

#[test]
fn sorted_buffer_works() {
    let store = store_with_packed_refs().unwrap();
    store.open_packed_buffer().unwrap();
}

#[test]
fn empty_buffers_should_not_exist_but_are_fine_to_open() -> crate::Result {
    let (_keep, path) = write_packed_refs_with(&[])?;
    assert_eq!(gix_ref::packed::Buffer::open(path, 512, HASH_KIND)?.iter()?.count(), 0);
    Ok(())
}

#[test]
fn unsorted_buffers_or_those_without_a_header_can_be_opened_and_searched() {
    for (fixture, cutoff) in [("without-header", 20u64), ("unsorted", 32 * 1024)] {
        let buffer = gix_ref::packed::Buffer::open(
            fixture_path(Path::new("packed-refs").join(fixture).to_str().expect("utf8")),
            cutoff,
            HASH_KIND,
        )
        .unwrap();
        for packed_ref in buffer.iter().unwrap().map(Result::unwrap) {
            let found_ref = buffer
                .find(packed_ref.name)
                .expect("ref can be found as buffer is sorted");
            assert_eq!(
                found_ref, packed_ref,
                "both reference are definitely equal, they are the same"
            );
        }
    }
}

#[test]
fn bogus_content_triggers_an_error() -> crate::Result {
    let packed_refs_data = b"starts with a bogus record, not a header anyway";
    let (_keep, path) = write_packed_refs_with(packed_refs_data)?;

    match gix_ref::packed::Buffer::open(path, 32, HASH_KIND) {
        Ok(_) => unreachable!("unsorted buffers can't be opened"),
        Err(err) => assert_eq!(
            err.to_string(),
            "The packed-refs file did not have a header or wasn't sorted and could not be iterated"
        ),
    }
    Ok(())
}
