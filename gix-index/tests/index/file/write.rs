use filetime::FileTime;
use gix_index::{State, Version, entry, extension, write, write::Options};
use gix_testtools::TestResult;

use crate::Fixture::*;

/// Round-trips should eventually be possible for all files we have, as we write them back exactly as they were read.
#[test]
fn roundtrips() -> TestResult {
    let input = [
        (Loose("extended-flags"), only_tree_ext()),
        (Loose("conflicting-file"), only_tree_ext()),
        (Loose("very-long-path"), only_tree_ext()),
        (
            Generated("v2"),
            options_with(write::Extensions::Given {
                tree_cache: true,
                untracked_cache: false,
                end_of_index_entry: true,
            }),
        ),
        (Generated("v2_empty"), only_tree_ext()),
        (Generated("v2_more_files"), only_tree_ext()),
        (Generated("v2_all_file_kinds"), only_tree_ext()),
    ];

    for (fixture, options) in input {
        // Loose fixtures only exist as SHA-1 version.
        if gix_testtools::object_hash() != gix_hash::Kind::Sha1 && matches!(fixture, Loose(_)) {
            continue;
        }

        let expected = fixture.open();
        let expected_bytes = std::fs::read(fixture.to_path())?;
        let mut out_bytes = Vec::new();

        let (actual_version, _digest) = expected.write_to(&mut out_bytes, options)?;
        let (actual, _) = State::from_bytes(
            &out_bytes,
            FileTime::now(),
            gix_testtools::object_hash(),
            Default::default(),
        )?;

        let name = fixture.to_name();
        compare_states_against_baseline(&actual, actual_version, &expected, options, name);
        compare_raw_bytes(&out_bytes, &expected_bytes, name);
    }
    Ok(())
}

#[test]
fn untracked_caches_roundtrip_when_enabled() -> TestResult {
    for fixture in [
        Generated("untracked_cache_empty"),
        Generated("untracked_cache_populated"),
        Generated("untracked_cache_nested"),
        Loose("UNTR"),
        Loose("UNTR-with-oids"),
    ] {
        let expected = fixture.open();
        assert!(expected.untracked().is_some(), "the fixture has an untracked cache");
        for extensions in [
            write::Extensions::AllUnconditionally,
            write::Extensions::All,
            write::Extensions::Given {
                tree_cache: false,
                untracked_cache: true,
                end_of_index_entry: true,
            },
            write::Extensions::Given {
                tree_cache: true,
                untracked_cache: false,
                end_of_index_entry: true,
            },
            write::Extensions::None,
        ] {
            let mut out = Vec::new();
            expected.write_to(&mut out, options_with(extensions))?;
            let (actual, _) =
                State::from_bytes(&out, expected.timestamp(), expected.object_hash(), Default::default())?;
            if extensions.should_write(extension::untracked_cache::SIGNATURE).is_some() {
                if matches!(extensions, write::Extensions::All) {
                    let cache = actual.untracked().expect("All writes the untracked cache");
                    let original = expected.untracked().expect("the fixture has an untracked cache");
                    assert_eq!(cache.identifier(), original.identifier(), "cache identity is retained");
                    assert_eq!(
                        cache.directories().len(),
                        original.directories().len(),
                        "directory structure is retained"
                    );
                    for (directory, old) in cache.directories().iter().zip(original.directories()) {
                        assert_eq!(directory.name, old.name, "directory names are retained");
                        assert_eq!(
                            directory.sub_directories, old.sub_directories,
                            "child directories are retained"
                        );
                        assert_eq!(
                            directory.exclude_file_oid, old.exclude_file_oid,
                            "ignore hashes are retained"
                        );
                        assert!(
                            directory.stat.is_none() && directory.untracked_entries.is_empty() && !directory.check_only,
                            "All invalidates cached directory listings"
                        );
                    }
                } else {
                    assert_eq!(
                        format!("{:#?}", actual.untracked()),
                        format!("{:#?}", expected.untracked()),
                        "writing {} preserves its untracked cache, including directory validity and ignore hashes",
                        fixture.to_name()
                    );
                }
                if let Generated(_) = &fixture {
                    let root = expected
                        .path()
                        .parent()
                        .and_then(std::path::Path::parent)
                        .expect("generated indexes live in the fixture's .git directory");
                    let temporary = gix_testtools::tempfile::TempDir::new()?;
                    let index_path = temporary.path().join("index");
                    std::fs::write(&index_path, &out)?;
                    let arguments = [
                        "--no-optional-locks",
                        "ls-files",
                        "--others",
                        "--exclude-standard",
                        "-z",
                    ];
                    let expected_listing = gix_testtools::git_command(root).args(arguments).output()?;
                    let actual_listing = gix_testtools::git_command(root)
                        .args(arguments)
                        .env("GIT_INDEX_FILE", &index_path)
                        .output()?;
                    assert!(
                        expected_listing.status.success(),
                        "Git can read the original fixture index"
                    );
                    assert!(
                        actual_listing.status.success(),
                        "Git can read the serialized UNTR: {actual_listing:?}"
                    );
                    assert_eq!(
                        actual_listing.stdout, expected_listing.stdout,
                        "Git produces the same untracked listing from its own index and the serialized cache"
                    );
                }
            } else {
                assert!(
                    actual.untracked().is_none(),
                    "untracked caches can be omitted explicitly"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn default_write_invalidates_the_untracked_cache_after_an_index_edit() -> TestResult {
    let mut index = Generated("untracked_cache_nested").open();
    assert!(index.untracked().is_some(), "the source has a populated cache");
    let original_cache = format!("{:#?}", index.untracked());
    let root = index
        .path()
        .parent()
        .and_then(std::path::Path::parent)
        .expect("the generated index lives in .git")
        .to_owned();
    index.remove_entries(|_, path, _| path == "tracked-dir-with-ignore/tracked-file");
    let temporary = gix_testtools::tempfile::TempDir::new()?;
    index.set_path(temporary.path().join("index"));
    index.write(Options::default())?;
    assert_eq!(
        format!("{:#?}", index.untracked()),
        original_cache,
        "writing does not mutate the in-memory cache"
    );

    let status = gix_testtools::git_command(root)
        .args([
            "--no-optional-locks",
            "status",
            "--porcelain",
            "--untracked-files=normal",
            "-z",
        ])
        .env("GIT_INDEX_FILE", index.path())
        .output()?;
    assert!(status.status.success(), "Git reads the edited index: {status:?}");
    assert!(
        status
            .stdout
            .split(|byte| *byte == 0)
            .any(|line| line == b"?? tracked-dir-with-ignore/tracked-file"),
        "Git lists the file whose index entry was removed"
    );
    let reopened = gix_index::File::at(index.path(), index.object_hash(), false, Default::default())?;
    let cache = reopened.untracked().expect("default writes retain the untracked cache");
    assert!(
        cache.directories().iter().all(|directory| directory.stat.is_none()
            && directory.untracked_entries.is_empty()
            && !directory.check_only),
        "default writes invalidate all cached directory listings"
    );
    Ok(())
}

#[test]
fn default_write_invalidates_all_tree_nodes_after_index_edits() -> TestResult {
    let mut index = Generated("v2_deeper_tree").open();
    let original_tree = index.tree().expect("committing populates TREE").clone();
    let root = index
        .path()
        .parent()
        .and_then(std::path::Path::parent)
        .expect("the generated index lives in .git")
        .to_owned();
    index
        .entry_mut_by_path_and_stage("sub/a/1".into(), entry::Stage::Unconflicted)
        .expect("the fixture tracks this nested file")
        .mode = entry::Mode::FILE_EXECUTABLE;
    index
        .entry_mut_by_path_and_stage("sub/c/d/3".into(), entry::Stage::Unconflicted)
        .expect("the fixture tracks this deeply nested file")
        .flags
        .insert(entry::Flags::REMOVE);

    let temporary = gix_testtools::tempfile::TempDir::new()?;
    index.set_path(temporary.path().join("cached-index"));
    index.write(Options::default())?;
    assert_eq!(
        index.tree(),
        Some(&original_tree),
        "writing does not mutate the in-memory cache"
    );
    let reopened = gix_index::File::at(index.path(), index.object_hash(), false, Default::default())?;
    assert_invalidated_tree(reopened.tree().expect("All retains TREE"), &original_tree);
    reopened.verify_extensions(false, gix_object::find::Never)?;

    let uncached_path = temporary.path().join("uncached-index");
    let mut uncached_bytes = Vec::new();
    index.write_to(&mut uncached_bytes, options_with(write::Extensions::None))?;
    std::fs::write(&uncached_path, uncached_bytes)?;
    // Git may write trees and refresh the index, but must not modify the read-only fixture.
    let objects = temporary.path().join("objects");
    std::fs::create_dir(&objects)?;
    let write_tree = |path| {
        gix_testtools::git_command(&root)
            .arg("write-tree")
            .env("GIT_INDEX_FILE", path)
            .env("GIT_OBJECT_DIRECTORY", &objects)
            .env("GIT_ALTERNATE_OBJECT_DIRECTORIES", root.join(".git/objects"))
            .output()
    };
    let cached = write_tree(index.path())?;
    let uncached = write_tree(&uncached_path)?;
    assert!(
        cached.status.success() && uncached.status.success(),
        "Git reads both indexes: {cached:?}, {uncached:?}"
    );
    assert_eq!(
        cached.stdout, uncached.stdout,
        "invalidated caches produce the same tree as rebuilding from scratch"
    );
    assert_ne!(
        cached.stdout,
        format!("{}\n", original_tree.id).as_bytes(),
        "the rebuilt tree reflects the entry edits"
    );
    Ok(())
}

#[test]
fn optional_extensions_can_be_selected_independently() {
    let signatures = [
        extension::tree::SIGNATURE,
        extension::untracked_cache::SIGNATURE,
        extension::end_of_index_entry::SIGNATURE,
    ];
    for extensions in [write::Extensions::default(), write::Extensions::AllUnconditionally] {
        for signature in signatures {
            assert_eq!(
                extensions.should_write(signature),
                Some(signature),
                "all modes select every writable optional extension"
            );
        }
    }
    for tree_cache in [false, true] {
        for untracked_cache in [false, true] {
            for end_of_index_entry in [false, true] {
                let extensions = write::Extensions::Given {
                    tree_cache,
                    untracked_cache,
                    end_of_index_entry,
                };
                for (signature, enabled) in
                    signatures
                        .into_iter()
                        .zip([tree_cache, untracked_cache, end_of_index_entry])
                {
                    assert_eq!(
                        extensions.should_write(signature),
                        enabled.then_some(signature),
                        "Given selects only enabled extensions"
                    );
                    assert_eq!(
                        write::Extensions::None.should_write(signature),
                        None,
                        "None selects no optional extensions"
                    );
                }
                assert_eq!(
                    extensions.should_write(extension::sparse::SIGNATURE),
                    None,
                    "mandatory extensions cannot be selected"
                );
                assert_eq!(
                    extensions.should_write(*b"REUC"),
                    None,
                    "unsupported extensions cannot be selected"
                );
            }
        }
    }
}

#[test]
fn skip_hash() -> TestResult {
    let tmp = gix_testtools::tempfile::TempDir::new()?;
    let path = tmp.path().join("index");
    let mut expected = Generated("v2_more_files").open();
    assert!(expected.checksum().is_some(), "the fixture starts with a checksum");

    expected.set_path(&path);
    for skip_hash in [false, true, false] {
        expected.write(Options {
            extensions: Default::default(),
            skip_hash,
        })?;
        assert_eq!(
            expected.checksum().is_none(),
            skip_hash,
            "writing without a hash clears the checksum; hashing again restores it"
        );

        let actual = gix_index::File::at(&path, expected.object_hash(), false, Default::default())?;
        assert_eq!(
            actual.checksum(),
            expected.checksum(),
            "checksum availability and value agree immediately after writing and after reopening"
        );
    }

    Ok(())
}

#[test]
fn roundtrips_sparse_index() -> TestResult {
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
        let (actual, _) = State::from_bytes(
            &out_bytes,
            FileTime::now(),
            gix_testtools::object_hash(),
            Default::default(),
        )?;

        compare_states_against_baseline(&actual, actual_version, &expected, options, fixture.to_name());
        // TODO: make this work and re-enable it, once this is done the fixtures can be merged into the main "roundtrip" test
        // compare_raw_bytes(&out_bytes, &_expected_bytes, fixture);
    }
    Ok(())
}

#[test]
fn state_comparisons_with_various_extension_configurations() -> TestResult {
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
        // Loose fixtures only exist as SHA-1 version.
        if gix_testtools::object_hash() != gix_hash::Kind::Sha1 && matches!(fixture, Loose(_)) {
            continue;
        }

        for options in [
            options_with(write::Extensions::None),
            options_with(write::Extensions::All),
            options_with(write::Extensions::AllUnconditionally),
            options_with(write::Extensions::Given {
                tree_cache: true,
                untracked_cache: false,
                end_of_index_entry: false,
            }),
            options_with(write::Extensions::Given {
                tree_cache: false,
                untracked_cache: false,
                end_of_index_entry: true,
            }),
        ] {
            let expected = fixture.open();
            let fixture = fixture.to_name();

            let mut out = Vec::<u8>::new();
            let (actual_version, _digest) = expected.write_to(&mut out, options)?;

            let (actual, _) =
                State::from_bytes(&out, FileTime::now(), gix_testtools::object_hash(), Default::default())?;
            compare_states(&actual, actual_version, &expected, options, fixture);
        }
    }
    Ok(())
}

#[test]
fn extended_flags_automatically_upgrade_the_version_to_avoid_data_loss() -> TestResult {
    let mut expected = Generated("v2").open();
    assert_eq!(expected.version(), Version::V2);
    expected.entries_mut()[0].flags.insert(entry::Flags::EXTENDED);

    let mut buf = Vec::new();
    let (actual_version, _digest) = expected.write_to(&mut buf, Default::default())?;
    assert_eq!(actual_version, Version::V3, "extended flags need V3");

    Ok(())
}

#[test]
fn remove_flag_is_respected() -> TestResult {
    let mut index = Generated("v4_more_files_IEOT").open();
    let total_entries = 10;
    assert_eq!(index.entries().len(), total_entries);
    let entries_to_remove = 4;
    for entry in &mut index.entries_mut()[..entries_to_remove] {
        entry.flags.toggle(entry::Flags::REMOVE);
    }
    let mut buf = Vec::<u8>::new();
    index.write_to(&mut buf, Default::default())?;

    let (state, _checksum) =
        State::from_bytes(&buf, FileTime::now(), gix_testtools::object_hash(), Default::default())?;
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
    if matches!(options.extensions, write::Extensions::All) {
        match (actual.tree(), expected.tree()) {
            (Some(actual), Some(expected)) => assert_invalidated_tree(actual, expected),
            (actual, expected) => assert_eq!(actual, expected, "absent caches are not created in {fixture:?}"),
        }
    } else {
        assert_eq!(
            actual.tree(),
            options
                .extensions
                .should_write(extension::tree::SIGNATURE)
                .and_then(|_| expected.tree()),
            "tree extension mismatch, actual vs option in {fixture:?}"
        );
    }

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

fn assert_invalidated_tree(actual: &extension::Tree, expected: &extension::Tree) {
    assert!(
        actual.num_entries.is_none() && actual.id.is_null(),
        "all cache-tree nodes are invalidated"
    );
    assert_eq!(actual.name, expected.name, "cache-tree names are retained");
    assert_eq!(
        actual.children.len(),
        expected.children.len(),
        "cache-tree structure is retained"
    );
    for (actual, expected) in actual.children.iter().zip(&expected.children) {
        assert_invalidated_tree(actual, expected);
    }
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

            panic! {"\n\nRoundtrip failed for index in fixture {fixture:?} at position {index:?}\n\
            \t  Actual: ... {generated:?} ...\n\
            \tExpected: ... {expected:?} ...\n\n\
            "}
        }
    }
}

fn only_tree_ext() -> Options {
    Options {
        extensions: write::Extensions::Given {
            end_of_index_entry: false,
            tree_cache: true,
            untracked_cache: false,
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
