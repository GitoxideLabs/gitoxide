const SHA1_SIZE: usize = gix_hash::Kind::Sha1.len_in_bytes();

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use gix_object::{self as object};
use gix_odb::pack;

use crate::{
    fixture_path, hex_to_id,
    pack::{leaked_fixture_bytes, pack_from_memory_at, INDEX_V1, PACK_FOR_INDEX_V1, SMALL_PACK, SMALL_PACK_INDEX},
};

fn memory_backed_index(at: &str) -> gix_pack::index::File<&'static [u8]> {
    let (data, path) = leaked_fixture_bytes(fixture_path(at));
    gix_pack::index::File::from_data(data, path, gix_hash::Kind::Sha1).expect("valid index file")
}

mod fuzzed;

mod version {
    mod v1 {
        use gix_pack::index;

        use crate::{fixture_path, pack::INDEX_V1};

        #[test]
        fn lookup() -> Result<(), Box<dyn std::error::Error>> {
            let object_hash = gix_hash::Kind::Sha1;
            let file = index::File::at(fixture_path(INDEX_V1), object_hash)?;
            for (id, desired_index, assertion) in &[
                (&b"036bd66fe9b6591e959e6df51160e636ab1a682e"[..], Some(0), "first"),
                (b"f7f791d96b9a34ef0f08db4b007c5309b9adc3d6", Some(65), "close to last"),
                (b"ffffffffffffffffffffffffffffffffffffffff", None, "not in pack"),
            ] {
                assert_eq!(
                    file.lookup(gix_hash::ObjectId::from_hex(id)?),
                    *desired_index,
                    "{assertion}",
                );
            }
            for (entry_index, entry) in file.iter().enumerate() {
                for mut candidates in [None, Some(0..0)] {
                    let index = file.lookup(entry.oid).expect("id present");
                    assert_eq!(entry.oid.as_ref(), file.oid_at_index(index));
                    assert_eq!(entry.pack_offset, file.pack_offset_at_index(index));
                    assert_eq!(entry.crc32, file.crc32_at_index(index));

                    let hex_len = (entry_index % object_hash.len_in_hex()).max(7);
                    let prefix = gix_hash::Prefix::new(&entry.oid, hex_len)?;
                    assert_eq!(
                        file.lookup_prefix(prefix, candidates.as_mut())
                            .expect("object exists")
                            .expect("non-ambiguous"),
                        index
                    );
                    if let Some(candidates) = candidates {
                        assert_eq!(candidates, index..index + 1);
                    }
                }
            }
            Ok(())
        }
    }

    mod v2 {
        use gix_pack::index;

        use crate::{fixture_path, pack::INDEX_V2};

        #[test]
        fn lookup() -> Result<(), Box<dyn std::error::Error>> {
            let object_hash = gix_hash::Kind::Sha1;
            let file = index::File::at(fixture_path(INDEX_V2), object_hash)?;
            for (id, expected, assertion_message, hex_len) in [
                (&b"0ead45fc727edcf5cadca25ef922284f32bb6fc1"[..], Some(0), "first", 4),
                (b"e800b9c207e17f9b11e321cc1fba5dfe08af4222", Some(29), "last", 40),
                (b"ffffffffffffffffffffffffffffffffffffffff", None, "not in pack", 7),
            ] {
                for mut candidates in [None, Some(1..1)] {
                    let id = gix_hash::ObjectId::from_hex(id)?;
                    assert_eq!(file.lookup(id), expected, "{assertion_message}");
                    assert_eq!(
                        file.lookup_prefix(gix_hash::Prefix::new(&id, hex_len)?, candidates.as_mut()),
                        expected.map(Ok)
                    );
                    if let Some(candidates) = candidates {
                        match expected {
                            Some(expected) => assert_eq!(candidates, expected..expected + 1),
                            None => assert_eq!(candidates, 0..0),
                        }
                    }
                }
            }
            for (entry_index, entry) in file.iter().enumerate() {
                for mut candidates in [None, Some(0..0)] {
                    let index = file.lookup(entry.oid).expect("id present");
                    assert_eq!(entry.oid.as_ref(), file.oid_at_index(index));
                    assert_eq!(entry.pack_offset, file.pack_offset_at_index(index));
                    assert_eq!(entry.crc32, file.crc32_at_index(index), "{index} {entry:?}");

                    let hex_len = (entry_index % object_hash.len_in_hex()).max(7);
                    let prefix = gix_hash::Prefix::new(&entry.oid, hex_len)?;
                    assert_eq!(
                        file.lookup_prefix(prefix, candidates.as_mut())
                            .expect("object exists")
                            .expect("non-ambiguous"),
                        index
                    );
                    if let Some(candidates) = candidates {
                        assert_eq!(candidates, index..index + 1);
                    }
                }
            }
            Ok(())
        }
    }

    #[cfg(feature = "gix-features-parallel")]
    mod any {
        use std::{fs, io, sync::atomic::AtomicBool};

        use gix_features::progress;
        use gix_odb::pack;
        use gix_pack::{data::input, index};

        use crate::{
            fixture_path,
            pack::{INDEX_V2, V2_PACKS_AND_INDICES},
        };

        fn slice_map(entry: gix_pack::data::EntryRange, map: &memmap2::Mmap) -> Option<&[u8]> {
            map.get(entry.start as usize..entry.end as usize)
        }

        #[test]
        fn write_to_stream() -> Result<(), Box<dyn std::error::Error>> {
            fn assert_index_write(
                mode: &input::Mode,
                compressed: &input::EntryDataMode,
                index_path: &&str,
                data_path: &&str,
            ) -> Result<(), Box<dyn std::error::Error>> {
                let mut pack_iter = pack::data::input::BytesToEntriesIter::new_from_header(
                    io::BufReader::new(fs::File::open(fixture_path(data_path))?),
                    *mode,
                    *compressed,
                    gix_hash::Kind::Sha1,
                )?;

                let mut actual = Vec::<u8>::new();
                let desired_kind = pack::index::Version::default();
                let num_objects = pack_iter.len() as u32;
                let pack_version = pack_iter.version();
                let outcome = pack::index::write_data_iter_to_stream(
                    desired_kind,
                    || {
                        let file = std::fs::File::open(fixture_path(data_path))?;
                        let map = unsafe { memmap2::MmapOptions::new().map_copy_read_only(&file)? };
                        Ok((slice_map, map))
                    },
                    &mut pack_iter,
                    None,
                    &mut progress::Discard,
                    &mut actual,
                    &AtomicBool::new(false),
                    gix_hash::Kind::Sha1,
                    pack_version,
                )?;

                let expected = fs::read(fixture_path(index_path))?;
                let end_of_header = 4 * 2;
                assert_eq!(&actual[..end_of_header], &expected[..end_of_header], "header");
                let end_of_fanout_table = end_of_header + 256 * 4;
                assert_eq!(
                    &actual[end_of_header..end_of_fanout_table],
                    &expected[end_of_header..end_of_fanout_table],
                    "fan out table"
                );
                let end_of_ids = end_of_fanout_table + 20 * num_objects as usize;
                assert_eq!(
                    &actual[end_of_fanout_table..end_of_ids],
                    &expected[end_of_fanout_table..end_of_ids],
                    "hashes: sha1"
                );
                let end_of_crc32 = end_of_ids + 4 * num_objects as usize;
                assert_eq!(
                    &actual[end_of_ids..end_of_crc32],
                    &expected[end_of_ids..end_of_crc32],
                    "crc32"
                );
                let end_of_offsets = end_of_crc32 + 4 * num_objects as usize;
                assert_eq!(
                    &actual[end_of_crc32..end_of_offsets],
                    &expected[end_of_crc32..end_of_offsets],
                    "offsets"
                );
                let end_of_pack_hash = end_of_offsets + 20;
                assert_eq!(
                    &actual[end_of_offsets..end_of_pack_hash],
                    &expected[end_of_offsets..end_of_pack_hash],
                    "offsets"
                );
                let end_of_index_hash = end_of_pack_hash + 20;
                assert_eq!(
                    &actual[end_of_pack_hash..end_of_index_hash],
                    &expected[end_of_pack_hash..end_of_index_hash],
                    "index hash"
                );
                assert_eq!(
                    actual, expected,
                    "we should be writing a bit-exact version of the original V2 index"
                );
                assert_eq!(
                    outcome.num_objects, num_objects,
                    "it wrote the entire iterator worth of entries"
                );
                assert_eq!(outcome.index_version, desired_kind);
                assert_eq!(
                    outcome.index_hash,
                    gix_hash::ObjectId::try_from(&expected[end_of_pack_hash..end_of_index_hash])?
                );
                Ok(())
            }
            for mode in &[input::Mode::AsIs, input::Mode::Verify, input::Mode::Restore] {
                for compressed in &[input::EntryDataMode::Crc32, input::EntryDataMode::KeepAndCrc32] {
                    for (index_path, data_path) in V2_PACKS_AND_INDICES {
                        assert_index_write(mode, compressed, index_path, data_path)?;
                    }
                }
            }
            Ok(())
        }

        #[test]
        fn lookup_missing() {
            let file = index::File::at(&fixture_path(INDEX_V2), gix_hash::Kind::Sha1).unwrap();
            let prefix = gix_hash::Prefix::new(&gix_hash::Kind::Sha1.null(), 7).unwrap();
            assert!(file.lookup_prefix(prefix, None).is_none());

            let mut candidates = 1..1;
            assert!(file.lookup_prefix(prefix, Some(&mut candidates)).is_none());
            assert_eq!(candidates, 0..0);
        }
    }
}

#[test]
fn traverse_with_index_and_forward_ref_deltas() {
    let index = index::File::at(
        fixture_path("objects/pack-with-forward-delta/pack-0bb5bc1e3d864c617c2539445c832ccdd531cd4e.idx"),
        Default::default(),
    )
    .unwrap();
    let data = pack::data::File::at(index.path().with_extension("pack"), Default::default()).unwrap();
    let count = AtomicUsize::new(0);
    let _it_should_work = index
        .traverse_with_index(
            &data,
            |_, _, _, _| {
                count.fetch_add(1, Ordering::SeqCst);
                Ok::<_, std::io::Error>(())
            },
            &mut progress::Discard,
            &AtomicBool::new(false),
            index::traverse::with_index::Options::default(),
        )
        .unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 9, "we traverse all objects");
}

#[test]
fn from_memory_backing_supports_verification_and_traversal() {
    use gix_features::progress;

    let index = memory_backed_index(SMALL_PACK_INDEX);
    let data = pack_from_memory_at(SMALL_PACK);
    assert_eq!(
        index
            .verify_checksum(&mut progress::Discard, &AtomicBool::new(false))
            .unwrap(),
        index.index_checksum()
    );
    assert_eq!(
        data.verify_checksum(&mut progress::Discard, &AtomicBool::new(false))
            .unwrap(),
        data.checksum()
    );

    let count = AtomicUsize::new(0);
    index
        .traverse_with_index(
            &data,
            |_, _, _, _| {
                count.fetch_add(1, Ordering::SeqCst);
                Ok::<_, std::io::Error>(())
            },
            &mut progress::Discard,
            &AtomicBool::new(false),
            gix_pack::index::traverse::with_index::Options::default(),
        )
        .unwrap();
    assert_eq!(count.load(Ordering::SeqCst), index.num_objects() as usize);
}

use gix_features::progress;
use gix_pack::{cache, data::decode::entry::Outcome, index};
use maplit::btreemap;

use crate::pack::{INDEX_V2, PACK_FOR_INDEX_V2};

static ALGORITHMS: &[index::traverse::Algorithm] = &[
    index::traverse::Algorithm::Lookup,
    index::traverse::Algorithm::DeltaTreeLookup,
];

static MODES: &[index::verify::Mode] = &[
    index::verify::Mode::HashCrc32,
    index::verify::Mode::HashCrc32Decode,
    index::verify::Mode::HashCrc32DecodeEncode,
];

#[test]
fn pack_lookup() -> Result<(), Box<dyn std::error::Error>> {
    for (index_path, pack_path, stats) in &[
        (
            INDEX_V2,
            PACK_FOR_INDEX_V2,
            index::traverse::Statistics {
                average: Outcome {
                    kind: object::Kind::Tree,
                    num_deltas: 1,
                    decompressed_size: 3456,
                    compressed_size: 1725,
                    object_size: 9621,
                },
                objects_per_chain_length: btreemap! {
                    0 => 18,
                    1 => 4,
                    2 => 3,
                    3 => 1,
                    4 => 2,
                    5 => 1,
                    6 => 1,
                },
                total_compressed_entries_size: 51753,
                total_decompressed_entries_size: 103701,
                total_object_size: 288658,
                num_commits: 10,
                num_blobs: 5,
                num_tags: 0,
                num_trees: 15,
                pack_size: 51875,
            },
        ),
        (
            INDEX_V1,
            PACK_FOR_INDEX_V1,
            index::traverse::Statistics {
                average: Outcome {
                    kind: object::Kind::Tree,
                    num_deltas: 0,
                    decompressed_size: 1982,
                    compressed_size: 729,
                    object_size: 2093,
                },
                objects_per_chain_length: btreemap! {
                    0 => 64,
                    1 => 3
                },
                total_compressed_entries_size: 48867,
                total_decompressed_entries_size: 132823,
                total_object_size: 140243,
                num_commits: 2,
                num_blobs: 63,
                num_tags: 0,
                num_trees: 2,
                pack_size: 49113,
            },
        ),
        (
            SMALL_PACK_INDEX,
            SMALL_PACK,
            index::traverse::Statistics {
                average: Outcome {
                    kind: object::Kind::Tree,
                    num_deltas: 0,
                    decompressed_size: 118,
                    compressed_size: 85,
                    object_size: 293,
                },
                objects_per_chain_length: btreemap! {
                    0 => 30,
                    1 => 6,
                    2 => 6,
                },
                total_compressed_entries_size: 3604,
                total_decompressed_entries_size: 4997,
                total_object_size: 12307,
                num_commits: 14,
                num_blobs: 14,
                num_tags: 0,
                num_trees: 14,
                pack_size: 3732,
            },
        ),
    ] {
        let idx = index::File::at(fixture_path(index_path), gix_hash::Kind::Sha1)?;
        let pack = pack::data::File::at(fixture_path(pack_path), gix_hash::Kind::Sha1)?;

        assert_eq!(pack.version(), pack::data::Version::V2);
        assert_eq!(pack.num_objects(), idx.num_objects());
        for algo in ALGORITHMS {
            for mode in MODES {
                assert_eq!(
                    idx.verify_integrity(
                        Some(gix_pack::index::verify::PackContext {
                            data: &pack,
                            options: gix_pack::index::verify::integrity::Options {
                                verify_mode: *mode,
                                traversal: *algo,
                                make_pack_lookup_cache: || cache::Never,
                                thread_limit: None
                            }
                        }),
                        &mut progress::Discard,
                        &AtomicBool::new(false)
                    )
                    .map(|o| (o.actual_index_checksum, o.pack_traverse_statistics))?,
                    (idx.index_checksum(), Some(stats.to_owned())),
                    "{algo:?} -> {mode:?}"
                );
            }
        }
        let num_objects = stats
            .objects_per_chain_length
            .values()
            .map(|v| *v as usize)
            .sum::<usize>();
        let sorted_offsets = idx.sorted_offsets();
        assert_eq!(num_objects, sorted_offsets.len());
        for idx_entry in idx.iter() {
            let pack_entry = pack.entry(idx_entry.pack_offset)?;
            assert_ne!(pack_entry.data_offset, idx_entry.pack_offset);
            assert!(sorted_offsets.binary_search(&idx_entry.pack_offset).is_ok());
        }
        for (entry, offset_from_index) in pack.streaming_iter()?.zip(sorted_offsets.iter().copied()) {
            let entry = entry?;
            assert_eq!(
                entry.pack_offset, offset_from_index,
                "iteration should yield the same pack offsets as the index"
            );

            let mut buf = vec![0u8; entry.decompressed_size as usize];
            let pack_entry = pack.entry(offset_from_index)?;
            assert_eq!(
                pack_entry.pack_offset(),
                entry.pack_offset,
                "index entry offset and computed pack offset must match"
            );
            pack.decompress_entry(&pack_entry, &mut Default::default(), &mut buf)?;

            assert_eq!(
                buf.len() as u64,
                entry.decompressed_size,
                "the decompressed length are the same no matter what decompressed them"
            );

            let next_offset_index = sorted_offsets
                .binary_search(&entry.pack_offset)
                .expect("correct offset")
                + 1;
            let next_offset = if next_offset_index == sorted_offsets.len() {
                (pack.data_len() - SHA1_SIZE) as u64
            } else {
                sorted_offsets[next_offset_index]
            };
            assert_eq!(
                entry
                    .compressed
                    .expect("bytes present in default configuration of streaming iter")
                    .len() as u64,
                next_offset - entry.pack_offset - u64::from(entry.header_size),
                "we get the compressed bytes region after the head to the next entry"
            );
        }
    }
    Ok(())
}

#[test]
fn iter() -> Result<(), Box<dyn std::error::Error>> {
    for (path, kind, num_objects, index_checksum, pack_checksum) in &[
        (
            INDEX_V1,
            index::Version::V1,
            67,
            "5a2b20ef73ffe911178532df86232b64830cb536",
            "7ebaef998897d903e6e6b6763d3a6ec4dc5b845b",
        ),
        (
            INDEX_V2,
            index::Version::V2,
            30,
            "560eba66e6b391eb83efc3ec9fc8a3087788911c",
            "f1cd3cc7bc63a4a2b357a475a58ad49b40355470",
        ),
        (
            SMALL_PACK_INDEX,
            index::Version::V2,
            42,
            "544a7204a55f6e9cacccf8f6e191ea8f83575de3",
            "0f3ea84cd1bba10c2a03d736a460635082833e59",
        ),
    ] {
        let idx = index::File::at(fixture_path(path), gix_hash::Kind::Sha1)?;
        assert_eq!(idx.version(), *kind);
        assert_eq!(idx.num_objects(), *num_objects);
        assert_eq!(
            idx.verify_integrity(
                None::<gix_pack::index::verify::PackContext<'_, fn() -> cache::Never>>,
                &mut progress::Discard,
                &AtomicBool::new(false)
            )
            .map(|o| (o.actual_index_checksum, o.pack_traverse_statistics))?,
            (idx.index_checksum(), None)
        );
        assert_eq!(idx.index_checksum(), hex_to_id(index_checksum));
        assert_eq!(idx.pack_checksum(), hex_to_id(pack_checksum));
        assert_eq!(idx.iter().count(), *num_objects as usize);
    }
    Ok(())
}
