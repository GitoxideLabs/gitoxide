use filetime::FileTime;
use gix_error::Result;
use std::path::PathBuf;

fn decode_fuzzed(data: &[u8]) -> Result<(gix_index::State, Option<gix_hash::ObjectId>)> {
    gix_index::State::from_bytes(
        data,
        FileTime::from_unix_time(0, 0),
        gix_hash::Kind::Sha1,
        Default::default(),
    )
}

#[test]
fn unsupported_version_is_classified() {
    let mut data = vec![0; 12 + gix_hash::Kind::Sha1.len_in_bytes()];
    data[..4].copy_from_slice(b"DIRC");
    data[4..8].copy_from_slice(&5u32.to_be_bytes());
    let err = decode_fuzzed(&data).expect_err("version 5 requires another index implementation");
    assert!(err.is_unsupported(), "the caller can switch index-reading strategies");
    assert!(
        !err.is_validation() && !err.is_corrupted(),
        "an unknown version is not malformed data"
    );
}

#[test]
fn index_file_artifacts_run_fuzzer() {
    for path in artifact_paths("index_file") {
        let data = std::fs::read(path).expect("artifact is readable");
        if let Ok((state, _)) = decode_fuzzed(&data) {
            let file = gix_index::File::from_state(state, PathBuf::from("fuzz-input.index"));
            let mut out = Vec::new();
            let _ = file.write_to(&mut out, Default::default());
        }
    }
}

#[test]
fn impossible_v4_entry_count_is_rejected_before_reserving() {
    let err = decode_fuzzed(include_bytes!(
        "../../fuzz/artifacts/index_file/oom-16fb9c25ef3ba2d2012810726a6b6be0c2181b2b"
    ))
    .expect_err("fuzzed input must stay rejected");

    insta::assert_debug_snapshot!(err, "the entry count cannot fit into the available index bytes", @"Declared entry count exceeds possible entries for file size");
}

#[test]
fn impossible_tree_child_count_is_ignored() {
    let empty = gix_index::extension::tree::decode(b"\0-1 0\n", gix_hash::Kind::Sha1);
    assert!(empty.is_some(), "an invalidated tree can have no children");

    let data = format!("\0-1 {}\n", usize::MAX);
    assert!(
        gix_index::extension::tree::decode(data.as_bytes(), gix_hash::Kind::Sha1).is_none(),
        "a child count which cannot fit into the extension must be ignored"
    );
}

#[test]
fn malformed_tree_extension_is_ignored_instead_of_panicking() {
    match decode_fuzzed(include_bytes!(
        "../../fuzz/artifacts/index_file/crash-92d6786251cf2b3e13cacc9eda01864724aa6b4b"
    )) {
        Ok((state, _checksum)) => {
            assert!(state.tree().is_none(), "malformed optional extension must be ignored");
        }
        Err(err) => {
            assert!(err.is_corrupted(), "a truncated index trailer is corruption");
            insta::assert_debug_snapshot!(err, "the fixture can also be rejected for its truncated index trailer", @"Index trailer should have been 20 bytes long, but was 64");
        }
    }
}

#[test]
fn malformed_fsmonitor_extension_is_ignored_instead_of_panicking() -> gix_testtools::TestResult {
    let (state, _checksum) = decode_fuzzed(include_bytes!(
        "../../fuzz/artifacts/index_file/crash-6fe328e670c3ca54a4dac7a5c0dc1e51501cf1d9"
    ))?;

    assert!(
        state.fs_monitor().is_none(),
        "malformed optional extension must be ignored"
    );
    Ok(())
}

#[test]
fn malformed_untracked_cache_extension_is_ignored_instead_of_panicking() -> gix_testtools::TestResult {
    let (state, _checksum) = decode_fuzzed(include_bytes!(
        "../../fuzz/artifacts/index_file/crash-b3dc19d67c36fbc5fc4b4f5729df92911dd3a7d5"
    ))?;

    assert!(
        state.untracked().is_none(),
        "malformed optional extension must be ignored"
    );
    Ok(())
}

#[test]
fn impossible_untracked_cache_directory_counts_are_rejected_before_reserving() {
    match decode_fuzzed(include_bytes!(
        "../../fuzz/artifacts/index_file/oom-08fa49e4b1e2e3267dbd9adb27a1926003e39418"
    )) {
        Ok((state, _checksum)) => {
            assert!(
                state.untracked().is_none(),
                "malformed optional extension must be ignored"
            );
        }
        Err(err) => {
            assert!(err.is_corrupted(), "a truncated index trailer is corruption");
            insta::assert_debug_snapshot!(err, "the fixture can also be rejected for its truncated index trailer", @"Index trailer should have been 20 bytes long, but was 26");
        }
    }
}

#[test]
fn malformed_entry_padding_is_rejected_instead_of_panicking() {
    let err = decode_fuzzed(include_bytes!(
        "../../fuzz/artifacts/index_file/crash-f5b1c2a323c8b3d9275ce5223cae534d4af0f8e0"
    ))
    .expect_err("fuzzed input must stay rejected");

    assert!(err.is_corrupted(), "invalid entries are corrupt index data");
    insta::assert_debug_snapshot!(err, "malformed index entries identify the failed parser", @"Could not parse entry at index 1");
}

#[test]
fn malformed_untracked_cache_bitmap_is_rejected_instead_of_panicking() {
    match decode_fuzzed(include_bytes!(
        "../../fuzz/artifacts/index_file/crash-41a7b73c58e644d4a605336b387172f6c9101c5b"
    )) {
        Ok((state, _checksum)) => {
            assert!(
                state.untracked().is_none(),
                "malformed optional extension must be ignored"
            );
        }
        Err(err) => {
            assert!(err.is_corrupted(), "a truncated index trailer is corruption");
            insta::assert_debug_snapshot!(err, "the fixture can also be rejected for its truncated index trailer", @"Index trailer should have been 20 bytes long, but was 51");
        }
    }
}

#[test]
fn malformed_entry_padding_with_untracked_cache_is_rejected_instead_of_panicking() {
    let err = decode_fuzzed(include_bytes!(
        "../../fuzz/artifacts/index_file/crash-8ee7f3c7bc2d72e3d1fcff7b67b493a8403297e2"
    ))
    .expect_err("fuzzed input must stay rejected");

    assert!(err.is_corrupted(), "invalid entries are corrupt index data");
    insta::assert_debug_snapshot!(err, "malformed index entries identify the failed parser", @"Could not parse entry at index 1");
}

fn artifact_paths(target: &str) -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../fuzz/artifacts")
        .join(target);
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect::<Vec<_>>();
    paths.sort();
    paths
}
