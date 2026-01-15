use filetime::FileTime;
use gix_index::{entry, extension, write, write::Options, State, Version};

use crate::index::Fixture::*;

/// Round-trips should eventually be possible for all files we have, as we write them back exactly as they were read.
#[test]
fn roundtrips() -> crate::Result {
    let input = [
        (Loose("extended-flags"), only_tree_ext()),
        (Loose("conflicting-file"), only_tree_ext()),
        (Loose("very-long-path"), only_tree_ext()),
        (
            Generated("v2"),
            options_with(write::Extensions::Given {
                tree_cache: true,
                end_of_index_entry: true,
            }),
        ),
        (Generated("v2_empty"), only_tree_ext()),
        (Generated("v2_more_files"), only_tree_ext()),
        (Generated("v2_all_file_kinds"), only_tree_ext()),
    ];

    for (fixture, options) in input {
        let expected = fixture.open();
        let expected_bytes = std::fs::read(fixture.to_path())?;
        let mut out_bytes = Vec::new();

        let (actual_version, _digest) = expected.write_to(&mut out_bytes, options)?;
        let (actual, _) = State::from_bytes(&out_bytes, FileTime::now(), gix_hash::Kind::Sha1, Default::default())?;

        let name = fixture.to_name();
        compare_states_against_baseline(&actual, actual_version, &expected, options, name);
        compare_raw_bytes(&out_bytes, &expected_bytes, name);
    }
    Ok(())
}

#[test]
fn skip_hash() -> crate::Result {
    let tmp = gix_testtools::tempfile::TempDir::new()?;
    let path = tmp.path().join("index");
    let mut expected = Loose("conflicting-file").open();
    assert!(expected.checksum().is_some());

    expected.set_path(&path);
    expected.write(Options {
        extensions: Default::default(),
        skip_hash: false,
    })?;

    let actual = gix_index::File::at(
        &path,
        expected.checksum().expect("present").kind(),
        false,
        Default::default(),
    )?;
    assert_eq!(
        actual.checksum(),
        expected.checksum(),
        "a hash is written by default and it matches"
    );

    expected.write(Options {
        extensions: Default::default(),
        skip_hash: true,
    })?;

    let actual = gix_index::File::at(
        &path,
        expected.checksum().expect("present").kind(),
        false,
        Default::default(),
    )?;
    assert_eq!(actual.checksum(), None, "no hash is produced in this case");

    Ok(())
}

#[test]
fn roundtrips_sparse_index() -> crate::Result {
    // NOTE: I initially tried putting these fixtures into the main roundtrip test above,
    // but the call to `compare_raw_bytes` panics. It seems like git is using a different
    // ordering when it comes to writing the tree extension. Need to investigate more, hence
    // the separate test for now.
    //
    //          git                     gitoxide
    //
    //          treeroot                treeroot
    //            | d                     | c1
    //            | d/c4                  | c1/c2
    //            | c1                    | c1/c3
    //            | c1/c2                 | d
    //            | c1/c3                 | d/c4
    //

    let input = [
        ("v3_skip_worktree", only_tree_ext()),
        ("v3_sparse_index_non_cone", only_tree_ext()),
        ("v3_sparse_index", only_tree_ext()),
        ("v2_sparse_index_no_dirs", only_tree_ext()),
    ];

    for (fixture, options) in input {
        let fixture = Generated(fixture);
        let expected = fixture.open();
        let _expected_bytes = std::fs::read(fixture.to_path())?;
        let mut out_bytes = Vec::new();

        let (actual_version, _) = expected.write_to(&mut out_bytes, options)?;
        let (actual, _) = State::from_bytes(&out_bytes, FileTime::now(), gix_hash::Kind::Sha1, Default::default())?;

        compare_states_against_baseline(&actual, actual_version, &expected, options, fixture.to_name());
        // TODO: make this work and re-enable it, once this is done the fixtures can be merged into the main "roundtrip" test
        // compare_raw_bytes(&out_bytes, &_expected_bytes, fixture);
    }
    Ok(())
}

#[test]
fn state_comparisons_with_various_extension_configurations() {
    for fixture in [
        Loose("extended-flags"),
        Loose("conflicting-file"),
        Loose("very-long-path"),
        Loose("FSMN"),
        Loose("REUC"),
        Loose("UNTR-with-oids"),
        Loose("UNTR"),
        Generated("v2_empty"),
        Generated("v2"),
        Generated("v2_more_files"),
        Generated("v2_all_file_kinds"),
        Generated("v2_split_index"),
        // TODO: this fails because git allows to configure the index version while gitoxide doesn't
        //       the fixture artificially sets the version to V4 and gitoxide writes it back out as the lowest required version, V2
        // Generated("v4_more_files_IEOT"),
        Generated("v3_skip_worktree"),
        Generated("v3_added_files"),
        Generated("v3_sparse_index_non_cone"),
        Generated("v3_sparse_index"),
        // TODO: this fails because git writes the sdir extension in this case while gitoxide doesn't
        // Generated("v2_sparse_index_no_dirs"),
    ] {
        for options in [
            options_with(write::Extensions::None),
            options_with(write::Extensions::All),
            options_with(write::Extensions::Given {
                tree_cache: true,
                end_of_index_entry: false,
            }),
            options_with(write::Extensions::Given {
                tree_cache: false,
                end_of_index_entry: true,
            }),
        ] {
            let expected = fixture.open();
            let fixture = fixture.to_name();

            let mut out = Vec::<u8>::new();
            let (actual_version, _digest) = expected.write_to(&mut out, options).unwrap();

            let (actual, _) =
                State::from_bytes(&out, FileTime::now(), gix_hash::Kind::Sha1, Default::default()).unwrap();
            compare_states(&actual, actual_version, &expected, options, fixture);
        }
    }
}

#[test]
fn extended_flags_automatically_upgrade_the_version_to_avoid_data_loss() -> crate::Result {
    let mut expected = Generated("v2").open();
    assert_eq!(expected.version(), Version::V2);
    expected.entries_mut()[0].flags.insert(entry::Flags::EXTENDED);

    let mut buf = Vec::new();
    let (actual_version, _digest) = expected.write_to(&mut buf, Default::default())?;
    assert_eq!(actual_version, Version::V3, "extended flags need V3");

    Ok(())
}

#[test]
fn remove_flag_is_respected() -> crate::Result {
    let mut index = Generated("v4_more_files_IEOT").open();
    let total_entries = 10;
    assert_eq!(index.entries().len(), total_entries);
    let entries_to_remove = 4;
    for entry in &mut index.entries_mut()[..entries_to_remove] {
        entry.flags.toggle(entry::Flags::REMOVE);
    }
    let mut buf = Vec::<u8>::new();
    index.write_to(&mut buf, Default::default())?;

    let (state, _checksum) = State::from_bytes(&buf, FileTime::now(), gix_hash::Kind::Sha1, Default::default())?;
    assert_eq!(
        state.entries().len(),
        total_entries - entries_to_remove,
        "entries are removed when writing"
    );
    assert_eq!(
        state.entries().iter().map(|e| e.path(&state)).collect::<Vec<_>>(),
        index.entries()[entries_to_remove..]
            .iter()
            .map(|e| e.path(&index))
            .collect::<Vec<_>>(),
        "the correct entries are removed"
    );
    Ok(())
}

fn compare_states_against_baseline(
    actual: &State,
    actual_version: Version,
    expected: &State,
    options: Options,
    fixture: &str,
) {
    compare_states(actual, actual_version, expected, options, fixture);

    assert_eq!(
        actual.tree(),
        expected.tree(),
        "tree extension mismatch, actual vs expected in {fixture:?}"
    );
}

fn compare_states(actual: &State, actual_version: Version, expected: &State, options: Options, fixture: &str) {
    actual.verify_entries().expect("valid");
    actual.verify_extensions(false, gix_object::find::Never).expect("valid");

    assert_eq!(
        actual.version(),
        actual_version,
        "version mismatch, read vs written, in {fixture:?}"
    );
    assert_eq!(
        actual.tree(),
        options
            .extensions
            .should_write(extension::tree::SIGNATURE)
            .and_then(|_| expected.tree()),
        "tree extension mismatch, actual vs option in {fixture:?}"
    );

    // As `write_to` does / should not mutate we can test those properties here.
    // Anything that can be configured has to be tested separately when comparing against baseline
    assert_eq!(
        actual.version(),
        expected.version(),
        "version mismatch, actual vs expected, in {fixture:?}"
    );
    assert_eq!(
        actual.is_sparse(),
        expected.is_sparse(),
        "sparse index entries extension mismatch in {fixture:?}"
    );
    assert_eq!(
        actual.entries().len(),
        expected.entries().len(),
        "entry count mismatch in {fixture:?}",
    );
    assert_eq!(actual.entries(), expected.entries(), "entries mismatch in {fixture:?}");
    assert_eq!(
        actual.path_backing(),
        expected.path_backing(),
        "path_backing mismatch in {fixture:?}",
    );
}

fn compare_raw_bytes(generated: &[u8], expected: &[u8], fixture: &str) {
    assert_eq!(generated.len(), expected.len(), "file length mismatch in {fixture:?}");

    let print_range = 10;
    for (index, (a, b)) in generated.iter().zip(expected.iter()).enumerate() {
        if a != b {
            let range_left = index.saturating_sub(print_range);
            let range_right = (index + print_range).min(generated.len());
            let generated = &generated[range_left..range_right];
            let expected = &expected[range_left..range_right];

            panic! {"\n\nRoundtrip failed for index in fixture {:?} at position {:?}\n\
            \t  Actual: ... {:?} ...\n\
            \tExpected: ... {:?} ...\n\n\
            ", &fixture, index, generated, expected}
        }
    }
}

fn only_tree_ext() -> Options {
    Options {
        extensions: write::Extensions::Given {
            end_of_index_entry: false,
            tree_cache: true,
        },
        skip_hash: false,
    }
}

fn options_with(extensions: write::Extensions) -> Options {
    Options {
        extensions,
        skip_hash: false,
    }
}
