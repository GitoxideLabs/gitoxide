use std::sync::atomic::AtomicBool;

use gix_features::progress;
use maplit::btreemap;

use crate::{multi_index::multi_index, object_hash};

#[test]
fn checksum() -> crate::Result {
    let (file, _) = multi_index(object_hash());
    assert_eq!(
        file.verify_checksum(&mut progress::Discard, &AtomicBool::new(false))?,
        file.checksum()
    );
    Ok(())
}

#[test]
fn integrity() {
    let object_hash = object_hash();
    let (file, _) = multi_index(object_hash);
    let outcome = file
        .verify_integrity(&mut progress::Discard, &AtomicBool::new(false), Default::default())
        .unwrap();
    assert_eq!(outcome.actual_index_checksum, file.checksum());
    assert_eq!(
        outcome.pack_traverse_statistics,
        vec![match object_hash {
            gix_hash::Kind::Sha1 => gix_pack::index::traverse::Statistics {
                average: gix_pack::data::decode::entry::Outcome {
                    kind: gix_object::Kind::Tree,
                    num_deltas: 1,
                    decompressed_size: 47,
                    compressed_size: 46,
                    object_size: 152
                },
                objects_per_chain_length: btreemap! {
                    0 => 326,
                    1 => 106,
                    2 => 326,
                    3 => 108,
                    4 => 2,
                },
                total_compressed_entries_size: 40628,
                total_decompressed_entries_size: 40919,
                total_object_size: 131993,
                pack_size: 42856,
                num_commits: 16,
                num_trees: 40,
                num_tags: 1,
                num_blobs: 811
            },
            gix_hash::Kind::Sha256 => gix_pack::index::traverse::Statistics {
                average: gix_pack::data::decode::entry::Outcome {
                    kind: gix_object::Kind::Tree,
                    num_deltas: 1,
                    decompressed_size: 62,
                    compressed_size: 60,
                    object_size: 176
                },
                objects_per_chain_length: btreemap! {
                    0 => 360,
                    1 => 145,
                    2 => 292,
                    3 => 68,
                    4 => 2,
                    5 => 1,
                },
                total_compressed_entries_size: 52919,
                total_decompressed_entries_size: 54442,
                total_object_size: 153293,
                pack_size: 55100,
                num_commits: 16,
                num_trees: 40,
                num_tags: 1,
                num_blobs: 811
            },
            _ => unimplemented!(),
        }]
    );
}
