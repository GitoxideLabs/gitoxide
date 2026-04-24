use std::path::PathBuf;

use std::panic::{catch_unwind, AssertUnwindSafe};

use filetime::FileTime;

#[test]
fn untracked_cache_with_out_of_range_bitmap_bits_is_reported_without_panicking() {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _ = gix_index::State::from_bytes(
            include_bytes!("../../fixtures/fuzzed/untracked-cache-out-of-range-bitmap.git-index"),
            FileTime::from_unix_time(0, 0),
            gix_hash::Kind::Sha1,
            Default::default(),
        );
    }));

    assert!(result.is_ok(), "malformed UNTR bitmaps must not panic during decode");
}

#[test]
fn oversized_entry_count_is_reported_without_allocating_absurd_memory() {
    assert!(
        gix_index::State::from_bytes(
            include_bytes!("../../fixtures/fuzzed/oversized-entry-count-out-of-memory.git-index"),
            FileTime::from_unix_time(0, 0),
            gix_hash::Kind::Sha1,
            Default::default(),
        )
        .is_err(),
        "attacker-controlled entry counts must be rejected instead of exhausting memory"
    );
}

#[test]
fn impossible_entry_count_is_rejected_before_any_large_allocation() {
    assert!(
        gix_index::State::from_bytes(
            include_bytes!("../../fixtures/fuzzed/impossible-entry-count.git-index"),
            FileTime::from_unix_time(0, 0),
            gix_hash::Kind::Sha1,
            Default::default(),
        )
        .is_err(),
        "headers that advertise more entries than can fit in the remaining bytes must be rejected up front"
    );
}

#[test]
fn tree_extension_with_trailing_bytes_is_reported_without_panicking() {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _ = gix_index::State::from_bytes(
            include_bytes!("../../fixtures/fuzzed/tree-extension-trailing-bytes.git-index"),
            FileTime::from_unix_time(0, 0),
            gix_hash::Kind::Sha1,
            Default::default(),
        );
    }));

    assert!(result.is_ok(), "malformed TREE extensions must not panic during decode");
}

#[test]
fn fsmonitor_extension_with_out_of_range_ewah_size_is_reported_without_panicking() {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _ = gix_index::State::from_bytes(
            include_bytes!("../../fixtures/fuzzed/fsmonitor-invalid-ewah-size.git-index"),
            FileTime::from_unix_time(0, 0),
            gix_hash::Kind::Sha1,
            Default::default(),
        );
    }));

    assert!(result.is_ok(), "malformed FSMN extensions must not panic during decode");
}

#[test]
fn untracked_cache_with_impossible_directory_counts_is_rejected_without_allocating_absurd_memory() {
    assert!(
        gix_index::State::from_bytes(
            include_bytes!("../../fixtures/fuzzed/untracked-cache-impossible-directory-counts.git-index"),
            FileTime::from_unix_time(0, 0),
            gix_hash::Kind::Sha1,
            Default::default(),
        )
        .is_err(),
        "UNTR directory counts that cannot fit in the remaining bytes must be rejected before growing vectors"
    );
}

#[test]
fn entry_padding_overflow_is_reported_without_panicking() {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _ = gix_index::State::from_bytes(
            include_bytes!("../../fixtures/fuzzed/entry-padding-overflow.git-index"),
            FileTime::from_unix_time(0, 0),
            gix_hash::Kind::Sha1,
            Default::default(),
        );
    }));

    assert!(result.is_ok(), "truncated entry padding must not panic during decode");
}

#[test]
fn untracked_cache_with_truncated_ewah_is_reported_without_panicking() {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _ = gix_index::State::from_bytes(
            include_bytes!("../../fixtures/fuzzed/untracked-cache-truncated-ewah.git-index"),
            FileTime::from_unix_time(0, 0),
            gix_hash::Kind::Sha1,
            Default::default(),
        );
    }));

    assert!(result.is_ok(), "malformed UNTR bitmaps must not panic during iteration");
}

#[test]
fn unpromoted_fuzz_artifacts_do_not_panic_while_parsing() {
    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let artifact_dir = test_dir.join("../fuzz/artifacts/index_file");

    // These are the artifacts in `fuzz/artifacts/index_file` that are not covered by content-identical
    // files in `tests/fixtures/fuzzed` and are not already exercised by dedicated tests in `index::fuzzed`.
    for name in [
        "crash-183d7e59664e77ac486de5ef39a3d223d6235e83",
        "oom-240461da86da2f14cc4554c7a77726285a0ac9be",
        "oom-71f5c01e4874bfe4ab5e8d40107fcdabafb6287f",
        "crash-f8884670b4ff8bba25d7278aff725beb1dec4aa4",
    ] {
        let path = artifact_dir.join(name);
        // Errors are Ok and expected.
        _ = gix_index::File::at(&path, gix_hash::Kind::Sha1, true, Default::default());
    }
}
