use bstr::{BString, ByteSlice};
use gix_index::entry::Stage;

use crate::index::Fixture;

fn icase_fixture() -> gix_index::File {
    Fixture::Generated("v2_icase_name_clashes").open()
}

#[test]
fn entry_by_path() {
    let file = icase_fixture();
    for entry in file.entries() {
        let path = entry.path(&file);
        assert_eq!(file.entry_by_path(path), Some(entry));
        assert_eq!(file.entry_by_path_and_stage(path, Stage::Unconflicted), Some(entry));
    }
}

#[test]
fn dirwalk_api_and_icase_support() {
    let file = Fixture::Loose("ignore-case-realistic").open();
    let icase = file.prepare_icase_backing();
    for entry in file.entries() {
        let entry_path = entry.path(&file);
        let a = file.entry_by_path_icase(entry_path, false, &icase);
        let b = file.entry_by_path_icase(entry_path, true, &icase);
        let c = file.entry_by_path_icase(entry_path.to_ascii_uppercase().as_bstr(), true, &icase);
        assert_eq!(
            a,
            b,
            "{entry_path}: an index without clashes produces exactly the same result, found {:?} and icase {:?}",
            a.map(|e| e.path(&file)),
            b.map(|e| e.path(&file))
        );
        assert_eq!(
            a,
            c,
            "{entry_path}: lower-case lookups work as well, found {:?} and icase {:?}",
            a.map(|e| e.path(&file)),
            c.map(|e| e.path(&file))
        );

        let mut last_pos = 0;
        while let Some(slash_idx) = entry_path[last_pos..].find_byte(b'/') {
            last_pos += slash_idx;
            let dir = entry_path[..last_pos].as_bstr();
            last_pos += 1;

            let entry = file
                .entry_closest_to_directory(dir)
                .unwrap_or_else(|| panic!("didn't find {dir}"));
            assert!(
                entry.path(&file).starts_with(dir),
                "entry must actually be inside of directory"
            );

            let dir_upper: BString = dir.to_ascii_uppercase().into();
            let other_entry = file
                .entry_closest_to_directory_icase(dir_upper.as_bstr(), true, &icase)
                .unwrap_or_else(|| panic!("didn't find upper-cased {dir_upper}"));
            assert_eq!(other_entry, entry, "the first entry is always the same, no matter what kind of search is conducted (as there are no clashes/ambiguities here)");
        }
    }
}

#[test]
fn ignorecase_clashes_and_order() {
    let file = icase_fixture();
    let icase = file.prepare_icase_backing();
    for entry in file.entries() {
        let entry_path = entry.path(&file);
        let a = file.entry_by_path_icase(entry_path, false, &icase);
        assert_eq!(
            a,
            Some(entry),
            "{entry_path}: in a case-sensitive search, we get exact matches, found {:?} ",
            a.map(|e| e.path(&file)),
        );

        let mut last_pos = 0;
        while let Some(slash_idx) = entry_path[last_pos..].find_byte(b'/') {
            last_pos += slash_idx;
            let dir = entry_path[..last_pos].as_bstr();
            last_pos += 1;

            let entry = file
                .entry_closest_to_directory(dir)
                .unwrap_or_else(|| panic!("didn't find {dir}"));
            assert!(
                entry.path(&file).starts_with(dir),
                "entry must actually be inside of directory"
            );
        }
    }
    assert_eq!(
        file.entry_by_path_icase("file_x".into(), true, &icase)
            .map(|e| e.path(&file))
            .expect("in index"),
        "FILE_X",
        "it finds the entry that was inserted first"
    );

    assert_eq!(
        file.entry_by_path_icase("x".into(), true, &icase)
            .map(|e| e.path(&file))
            .expect("in index"),
        "X",
        "the file 'X' was inserted first, no way to see the symlink under 'x'"
    );

    assert!(
        file.entry_closest_to_directory("d".into()).is_none(),
        "this is a file, and this directory search isn't case-sensitive"
    );
    let entry = file.entry_closest_to_directory("D".into());
    assert_eq!(
        entry.map(|e| e.path(&file)).expect("present"),
        "D/B",
        "this is a directory, indeed, we find the first file in it"
    );
    let entry_icase = file.entry_closest_to_directory_icase("d".into(), true, &icase);
    assert_eq!(
        entry_icase, entry,
        "case-insensitive searches don't confuse directories and files, so `d` finds `D`, the directory."
    );
}

#[test]
fn prefixed_entries_icase_with_name_clashes() {
    let file = icase_fixture();
    assert_eq!(
        file.prefixed_entries_range("file".into()),
        Some(7..9),
        "case sensitive search yields only two: file_x and file_X"
    );
}

#[test]
fn entry_by_path_and_stage() {
    let file = Fixture::Generated("v4_more_files_IEOT").open();
    for entry in file.entries() {
        let path = entry.path(&file);
        assert_eq!(
            file.entry_index_by_path_and_stage(path, Stage::Unconflicted)
                .map(|idx| &file.entries()[idx]),
            Some(entry)
        );
        assert_eq!(file.entry_by_path_and_stage(path, Stage::Unconflicted), Some(entry));
    }
}

#[test]
fn entry_by_path_with_conflicting_file() {
    let file = Fixture::Loose("conflicting-file").open();
    for expected_stage in [Stage::Base, Stage::Ours, Stage::Theirs] {
        assert!(
            file.entry_by_path_and_stage("file".into(), expected_stage).is_some(),
            "we have no stage 0 during a conflict, but all other ones. Missed {expected_stage:?}"
        );
    }

    assert_eq!(
        file.entry_by_path("file".into()).expect("found").stage(),
        Stage::Ours,
        "we always find our stage while in a merge"
    );
}

#[test]
fn prefixed_entries_with_multi_stage_file() {
    let file = Fixture::Loose("conflicting-file").open();

    assert_eq!(
        file.prefixed_entries("fil".into()).expect("present"),
        file.entries(),
        "it's possible to get the entire range"
    );
    assert_eq!(
        file.prefixed_entries("file".into()).expect("present"),
        file.entries(),
        "it's possible to get the entire range even if the same path matches multiple times"
    );
    assert_eq!(
        file.prefixed_entries("".into()).expect("present"),
        file.entries(),
        "empty prefix matches all"
    );
    assert_eq!(file.prefixed_entries_range("".into()), Some(0..3));
    assert_eq!(file.prefixed_entries_range("foo".into()), None);
}

#[test]
fn entry_range() {
    let file = Fixture::Loose("conflicting-file").open();

    assert_eq!(
        file.entry_range("file".into()),
        Some(0..3),
        "three stages, all but stage zero"
    );
    assert_eq!(file.entry_range("foo".into()), None, "does not exist");
}

#[test]
fn remove_entries() {
    let mut file = Fixture::Loose("conflicting-file").open();

    file.remove_entries(|idx, _, _| idx == 0);
    assert_eq!(file.entries().len(), 2);
    file.remove_entries(|idx, _, _| idx == 0);
    assert_eq!(file.entries().len(), 1);
    file.remove_entries(|idx, _, _| idx == 0);
    assert_eq!(file.entries().len(), 0);
    file.remove_entries(|_, _, _| unreachable!("should not be called"));
}

#[test]
fn remove_entry_at_index() {
    let mut file = Fixture::Loose("conflicting-file").open();

    file.remove_entry_at_index(0);
    assert_eq!(file.entries().len(), 2);
    file.remove_entry_at_index(0);
    assert_eq!(file.entries().len(), 1);
    file.remove_entry_at_index(0);
    assert_eq!(file.entries().len(), 0);
}

#[test]
fn sort_entries() {
    let mut file = Fixture::Generated("v4_more_files_IEOT").open();
    assert!(file.verify_entries().is_ok());
    let valid_entries = file.entries().len();

    let entry = file.entry(0).clone();
    let new_entry_path = "an initially incorrectly ordered entry".into();
    file.dangerously_push_entry(entry.stat, entry.id, entry.flags, entry.mode, new_entry_path);
    assert!(file.verify_entries().is_err(), "sort order doesn't fit anymore");

    for (idx, entry) in file.entries()[..valid_entries].iter().enumerate() {
        assert_eq!(
            file.entry_index_by_path_and_stage_bounded(entry.path(&file), Stage::Unconflicted, valid_entries),
            Some(idx),
            "we can still find entries in the correctly sorted region"
        );
    }
    assert_eq!(
        file.entry_by_path_and_stage(new_entry_path, Stage::Unconflicted),
        None,
        "new entry can't be found due to incorrect order"
    );

    file.sort_entries();
    assert!(file.verify_entries().is_ok(), "sorting of entries restores invariants");

    assert_eq!(
        file.entry_by_path_and_stage(new_entry_path, Stage::Unconflicted)
            .expect("can be found")
            .path(&file),
        new_entry_path,
        "we can find the correct entry now"
    );

    check_prefix(
        &file,
        "d",
        &["d/a", "d/b", "d/c", "d/last/123", "d/last/34", "d/last/6"],
    );
    check_prefix(
        &file,
        "d/",
        &["d/a", "d/b", "d/c", "d/last/123", "d/last/34", "d/last/6"],
    );
    check_prefix(&file, "d/last", &["d/last/123", "d/last/34", "d/last/6"]);
    check_prefix(&file, "d/last/", &["d/last/123", "d/last/34", "d/last/6"]);
    check_prefix(&file, "d/las", &["d/last/123", "d/last/34", "d/last/6"]);
    check_prefix(&file, "d/last/123", &["d/last/123"]);
    check_prefix(&file, "d/last/34", &["d/last/34"]);
    check_prefix(&file, "d/last/6", &["d/last/6"]);
    check_prefix(&file, "x", &["x"]);
    check_prefix(&file, "a", &["a", "an initially incorrectly ordered entry"]);
}

#[test]
fn prefixed_entries() {
    let mut file = Fixture::Generated("v4_more_files_IEOT").open();
    let entry = file.entry(0).clone();
    let new_entry_path = "an initially incorrectly ordered entry".into();
    file.dangerously_push_entry(entry.stat, entry.id, entry.flags, entry.mode, new_entry_path);
    file.sort_entries();

    check_prefix(&file, "a", &["a", "an initially incorrectly ordered entry"]);
    check_prefix(&file, "an", &["an initially incorrectly ordered entry"]);
    check_prefix(
        &file,
        "an initially incorrectly ordered entry",
        &["an initially incorrectly ordered entry"],
    );
    check_prefix(&file, "x", &["x"]);
    check_prefix(&file, "b", &["b"]);
    check_prefix(&file, "c", &["c"]);

    assert_eq!(
        file.prefixed_entries_range("".into()),
        Some(0..11),
        "empty prefixes match everything"
    );
    assert!(
        file.prefixed_entries_range("foo".into()).is_none(),
        "there is no match for this prefix"
    );
}

fn check_prefix(index: &gix_index::State, prefix: &str, expected: &[&str]) {
    assert_eq!(
        index
            .prefixed_entries(prefix.into())
            .unwrap_or_else(|| panic!("{prefix:?} must match at least one entry"))
            .iter()
            .map(|e| e.path(index))
            .collect::<Vec<_>>(),
        expected,
        "{prefix:?}"
    );
}

#[test]
fn path_is_directory() {
    let file = Fixture::Loose("ignore-case-realistic").open();

    // Test that directories containing entries are detected
    assert!(
        file.path_is_directory("tests".into()),
        "tests is a directory containing entries"
    );
    assert!(
        file.path_is_directory("tests/snapshots".into()),
        "tests/snapshots is a directory containing entries"
    );
    assert!(
        file.path_is_directory("tests/snapshots/porcelain".into()),
        "tests/snapshots/porcelain is a directory"
    );
    assert!(
        file.path_is_directory("tests/tools".into()),
        "tests/tools is a directory"
    );

    // Test that non-existent directories return false
    assert!(
        !file.path_is_directory("nonexistent".into()),
        "nonexistent is not a directory"
    );
    assert!(!file.path_is_directory("z".into()), "z is not a directory");
    assert!(
        !file.path_is_directory("test".into()),
        "test is not a directory (tests is)"
    );

    // Test that files are not directories
    assert!(
        !file.path_is_directory("tests/utilities.sh".into()),
        "tests/utilities.sh is a file, not a directory"
    );

    // Test that partial directory names don't match
    assert!(!file.path_is_directory("".into()), "empty path is not a directory");
}

#[test]
fn path_is_directory_icase() {
    let file = Fixture::Loose("ignore-case-realistic").open();
    let icase = file.prepare_icase_backing();

    // Test case-sensitive matching
    assert!(
        file.path_is_directory_icase("tests".into(), false, &icase),
        "tests is a directory (case-sensitive)"
    );
    assert!(
        file.path_is_directory_icase("tests/tools".into(), false, &icase),
        "tests/tools is a directory (case-sensitive)"
    );

    // Test case-insensitive matching
    assert!(
        file.path_is_directory_icase("TESTS".into(), true, &icase),
        "TESTS is a directory (case-insensitive, matches 'tests')"
    );
    assert!(
        file.path_is_directory_icase("tests/TOOLS".into(), true, &icase),
        "tests/TOOLS is a directory (case-insensitive, matches 'tests/tools')"
    );
    assert!(
        file.path_is_directory_icase("TESTS/SNAPSHOTS".into(), true, &icase),
        "TESTS/SNAPSHOTS is a directory (case-insensitive)"
    );

    // Test that non-existent paths return false even with icase
    assert!(
        !file.path_is_directory_icase("nonexistent".into(), true, &icase),
        "nonexistent is not a directory even with icase"
    );
    assert!(
        !file.path_is_directory_icase("Z".into(), true, &icase),
        "Z is not a directory even with icase"
    );
}

#[test]
fn path_is_directory_icase_with_clashes() {
    let file = icase_fixture();
    let icase = file.prepare_icase_backing();

    // Test directory detection with case clashes
    assert!(
        file.path_is_directory_icase("D".into(), false, &icase),
        "D is a directory (case-sensitive)"
    );
    assert!(
        file.path_is_directory_icase("d".into(), true, &icase),
        "d matches D directory (case-insensitive)"
    );

    // Test that files aren't detected as directories
    assert!(
        !file.path_is_directory_icase("X".into(), false, &icase),
        "X is a file, not a directory"
    );
    assert!(
        !file.path_is_directory_icase("x".into(), false, &icase),
        "x is a symlink, not a directory"
    );
}
