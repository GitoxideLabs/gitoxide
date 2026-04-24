use filetime::FileTime;
use std::path::PathBuf;

fn decode_fuzzed(data: &[u8]) -> Result<(gix_index::State, Option<gix_hash::ObjectId>), gix_index::decode::Error> {
    gix_index::State::from_bytes(
        data,
        FileTime::from_unix_time(0, 0),
        gix_hash::Kind::Sha1,
        Default::default(),
    )
}

#[test]
fn index_file_artifacts_run_fuzzer() {
    for path in artifact_paths("index_file") {
        let data = std::fs::read(path).expect("artifact is readable");
        let _ = decode_fuzzed(&data);
    }
}

#[test]
fn impossible_v4_entry_count_is_rejected_before_reserving() {
    let err = decode_fuzzed(include_bytes!(
        "../../fuzz/artifacts/index_file/oom-16fb9c25ef3ba2d2012810726a6b6be0c2181b2b"
    ))
    .expect_err("fuzzed input must stay rejected");

    assert!(
        err.to_string()
            .contains("Declared entry count exceeds possible entries for file size"),
        "{err:?}"
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
        Err(gix_index::decode::Error::UnexpectedTrailerLength { .. }) => {}
        Err(err) => panic!("unexpected decode failure: {err:?}"),
    }
}

#[test]
fn malformed_fsmonitor_extension_is_ignored_instead_of_panicking() {
    let (state, _checksum) = decode_fuzzed(include_bytes!(
        "../../fuzz/artifacts/index_file/crash-6fe328e670c3ca54a4dac7a5c0dc1e51501cf1d9"
    ))
    .expect("fuzzed input should decode without panicking");

    assert!(
        state.fs_monitor().is_none(),
        "malformed optional extension must be ignored"
    );
}

#[test]
fn malformed_untracked_cache_extension_is_ignored_instead_of_panicking() {
    let (state, _checksum) = decode_fuzzed(include_bytes!(
        "../../fuzz/artifacts/index_file/crash-b3dc19d67c36fbc5fc4b4f5729df92911dd3a7d5"
    ))
    .expect("fuzzed input should decode without panicking");

    assert!(
        state.untracked().is_none(),
        "malformed optional extension must be ignored"
    );
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
        Err(gix_index::decode::Error::UnexpectedTrailerLength { .. }) => {}
        Err(err) => panic!("unexpected decode failure: {err:?}"),
    }
}

#[test]
fn malformed_entry_padding_is_rejected_instead_of_panicking() {
    let err = decode_fuzzed(include_bytes!(
        "../../fuzz/artifacts/index_file/crash-f5b1c2a323c8b3d9275ce5223cae534d4af0f8e0"
    ))
    .expect_err("fuzzed input must stay rejected");

    assert!(matches!(err, gix_index::decode::Error::Entry { .. }), "{err:?}");
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
        Err(gix_index::decode::Error::UnexpectedTrailerLength { .. }) => {}
        Err(err) => panic!("unexpected decode failure: {err:?}"),
    }
}

#[test]
fn malformed_entry_padding_with_untracked_cache_is_rejected_instead_of_panicking() {
    let err = decode_fuzzed(include_bytes!(
        "../../fuzz/artifacts/index_file/crash-8ee7f3c7bc2d72e3d1fcff7b67b493a8403297e2"
    ))
    .expect_err("fuzzed input must stay rejected");

    assert!(matches!(err, gix_index::decode::Error::Entry { .. }), "{err:?}");
}

#[test]
fn alloc_limit_constructor_rejects_oversized_allocations() {
    let bytes = std::fs::read(crate::fixture_index_path("v4_more_files_IEOT")).expect("fixture index readable");
    let err = gix_index::State::from_bytes(
        &bytes,
        FileTime::from_unix_time(0, 0),
        gix_hash::Kind::Sha1,
        gix_index::decode::Options {
            alloc_limit_bytes: Some(64),
            ..Default::default()
        },
    )
    .expect_err("fixture should exceed tiny allocation limit");

    assert!(matches!(err, gix_index::decode::Error::OutOfMemory), "{err:?}");
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
