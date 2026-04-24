use std::path::PathBuf;

use super::multi_index;
use crate::{hex_to_id, pack::leaked_fixture_bytes};

/// Load a multi-index fixture into memory and instantiate a memory-backed multi-index file from it.
fn multi_index_from_memory() -> gix_pack::multi_index::File<&'static [u8]> {
    let (data, path) = leaked_fixture_bytes(
        crate::scripted_fixture_read_only("make_pack_gen_repo_multi_index.sh")
            .expect("test fixture exists")
            .join(".git/objects/pack/multi-pack-index"),
    );
    gix_pack::multi_index::File::from_data(data, path, None).expect("valid multi-index")
}

#[test]
fn lookup_with_ambiguity() {
    let (file, _path) = multi_index();
    let oid = hex_to_id("cfc33fc40413fb3e30ff6b44d03fd8d071cb633b");
    let prefix = gix_hash::Prefix::new(&oid, 4).unwrap();
    assert_eq!(
        file.lookup_prefix(prefix, None),
        Some(Err(())),
        "error code indicates ambiguous result"
    );

    let mut candidates = 0..0;
    assert_eq!(
        file.lookup_prefix(prefix, Some(&mut candidates)),
        Some(Err(())),
        "error code is similar to before"
    );
    assert_eq!(candidates, 682..683 + 1, "we receive a list of all duplicates");
}

#[test]
fn lookup_prefix() {
    let (file, _path) = multi_index();

    for (idx, entry) in file.iter().enumerate() {
        for mut candidates in [None, Some(0..0)] {
            let hex_len = (idx % file.object_hash().len_in_hex()).max(5);
            let hex_oid = entry.oid.to_hex_with_len(hex_len).to_string();
            assert_eq!(hex_oid.len(), hex_len);
            let oid_prefix = gix_hash::Prefix::new(&entry.oid, hex_len).unwrap();
            let entry_index = file
                .lookup_prefix(oid_prefix, candidates.as_mut())
                .expect("object found")
                .expect("non-ambiguous");
            assert_eq!(file.oid_at_index(entry_index), entry.oid);

            if let Some(candidates) = candidates {
                assert_eq!(candidates, entry_index..entry_index + 1);
            }
        }
    }
}

#[test]
fn lookup_missing() {
    let (file, _path) = multi_index();
    let prefix = gix_hash::Prefix::new(&gix_hash::ObjectId::null(gix_hash::Kind::Sha1), 7).unwrap();
    assert!(file.lookup_prefix(prefix, None).is_none());

    let mut candidates = 1..1;
    assert!(file.lookup_prefix(prefix, Some(&mut candidates)).is_none());
    assert_eq!(candidates, 0..0);
}

#[test]
fn general() {
    let (file, path) = multi_index();

    assert_eq!(file.version(), gix_pack::multi_index::Version::V1);
    assert_eq!(file.path(), path);
    assert_eq!(file.num_indices(), 1);
    assert_eq!(file.object_hash(), gix_hash::Kind::Sha1);
    assert_eq!(file.num_objects(), 868);
    assert_eq!(file.checksum(), hex_to_id("39a3804d0a84de609e4fcb49e66dc1297c75ca11"));
    // assert_eq!()
    assert_eq!(
        file.index_names(),
        vec![PathBuf::from("pack-542ad1d1c7c762ea4e36907570ff9e4b5b7dde1b.idx")]
    );

    for (idx, expected_pack_offset, expected_oid) in &[
        (0u32, 25267u64, hex_to_id("000f574443efab4ddbeee3621e49124eb3f8b6d0")),
        (140, 30421, hex_to_id("2935a65b1d69fb33c93dabc4cdf65a6f4d30ce4c")),
        (867, 24540, hex_to_id("ffea360a6a54c1185eeae4f3cfefc927cf7a35a9")),
    ] {
        let actual_oid = file.oid_at_index(*idx);
        assert_eq!(actual_oid, *expected_oid);
        assert_eq!(file.lookup(actual_oid), Some(*idx));
        let (pack_id, pack_offset) = file.pack_id_and_pack_offset_at_index(*idx);
        assert_eq!(pack_id, 0, "we only have one pack here");
        assert_eq!(pack_offset, *expected_pack_offset);
    }

    let mut count = 0;
    for (idx, entry) in file.iter().enumerate() {
        assert_eq!(entry.oid, file.oid_at_index(idx as u32));
        let (pack_index, pack_offset) = file.pack_id_and_pack_offset_at_index(idx as u32);
        assert_eq!(pack_index, entry.pack_index);
        assert_eq!(pack_offset, entry.pack_offset);
        count += 1;
    }
    assert_eq!(count, file.num_objects());
}

#[test]
fn from_memory() {
    let file = multi_index_from_memory();
    assert_eq!(file.version(), gix_pack::multi_index::Version::V1);
    assert_eq!(file.num_indices(), 1);
    assert_eq!(file.object_hash(), gix_hash::Kind::Sha1);
    assert_eq!(file.num_objects(), 868);
    assert_eq!(file.checksum(), hex_to_id("39a3804d0a84de609e4fcb49e66dc1297c75ca11"));
    assert_eq!(
        file.lookup(hex_to_id("000f574443efab4ddbeee3621e49124eb3f8b6d0")),
        Some(0)
    );
}
