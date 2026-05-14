use std::path::{Path, PathBuf};

use bstr::ByteSlice;
use gix_index::{
    Version,
    entry::{self, Flags, Mode},
};

use crate::{Fixture, hex_to_id, loose_file_path};

fn verify(index: gix_index::File) -> gix_index::File {
    index.verify_integrity().unwrap();
    index.verify_entries().unwrap();
    index.verify_extensions(false, gix_object::find::Never).unwrap();
    index
}

pub(crate) fn loose_file(name: &str) -> gix_index::File {
    let path = loose_file_path(name);
    let file = gix_index::File::at(path, gix_hash::Kind::Sha1, false, Default::default()).unwrap();
    verify(file)
}
pub(crate) fn try_file(name: &str, needs_archive: bool) -> Result<gix_index::File, gix_index::file::init::Error> {
    let path = if needs_archive {
        crate::fixture_index_path_needs_archive(name)
    } else {
        crate::fixture_index_path(name)
    };
    let file = gix_index::File::at(path, gix_hash::Kind::Sha1, false, Default::default())?;
    Ok(verify(file))
}
pub(crate) fn file(name: &str) -> gix_index::File {
    try_file(name, false).unwrap()
}
/// Needed if we have to freeze the fixture if contents depends on filesystem traversal order
/// This is Ok and similar to our manual copies of indices, except that it can be regenerated.
fn file_needs_archive(name: &str) -> gix_index::File {
    try_file(name, true).unwrap()
}
fn file_opt(name: &str, opts: gix_index::decode::Options) -> gix_index::File {
    let file = gix_index::File::at(crate::fixture_index_path(name), gix_hash::Kind::Sha1, false, opts).unwrap();
    verify(file)
}

fn with_index_file_snapshot_filters(has_stable_mtimes: bool, run: impl FnOnce()) {
    let mut settings = insta::Settings::clone_current();
    let stat_filter = if has_stable_mtimes {
        (
            r"(?s)Stat \{\s+mtime: Time \{\s+secs: (\d+),\s+nsecs: (\d+),\s+\},\s+ctime: Time \{\s+secs: \d+,\s+nsecs: \d+,\s+\},\s+dev: \d+,\s+ino: \d+,\s+uid: \d+,\s+gid: \d+,\s+size: \d+,\s+\}",
            "Stat { mtime: Time { secs: $1, nsecs: $2 }, ctime: Time { ... }, ... }",
        )
    } else {
        (
            r"(?s)Stat \{\s+mtime: Time \{\s+secs: \d+,\s+nsecs: \d+,\s+\},\s+ctime: Time \{\s+secs: \d+,\s+nsecs: \d+,\s+\},\s+dev: \d+,\s+ino: \d+,\s+uid: \d+,\s+gid: \d+,\s+size: \d+,\s+\}",
            "Stat { ... }",
        )
    };
    let mut filters = vec![
        (r#"(path: )"[^"]*""#, r#"$1"[redacted]""#),
        (r#"(identifier: )"[^"]*""#, r#"$1"[redacted]""#),
        (
            r"(?s)FileTime \{\s+seconds: \d+,\s+nanos: \d+,\s+\}",
            "FileTime { ... }",
        ),
        stat_filter,
    ];
    if !has_stable_mtimes {
        filters.push((r" mtime: Time \{ secs: \d+, nsecs: \d+ \}", ""));
    }
    settings.set_filters(filters);
    settings.bind(run);
}

#[test]
fn v2_with_single_entry_tree_and_eoie_ext() {
    let file_disallow_threaded_loading = file_opt(
        "v2",
        gix_index::decode::Options {
            min_extension_block_in_bytes_for_threading: 100000,
            ..Default::default()
        },
    );
    for file in [file("v2"), file_disallow_threaded_loading] {
        assert_eq!(file.version(), Version::V2);

        assert_eq!(file.entries().len(), 1);

        let entry = &file.entries()[0];
        assert_eq!(entry.id, hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391"));
        assert!(entry.flags.is_empty());
        assert_eq!(entry.mode, entry::Mode::FILE);
        assert_eq!(entry.path(&file), "a");

        let tree = file.tree().unwrap();
        assert_eq!(tree.num_entries.unwrap_or_default(), 1);
        assert_eq!(tree.id, hex_to_id("496d6428b9cf92981dc9495211e6e1120fb6f2ba"));
        assert!(tree.name.is_empty());
        assert!(tree.children.is_empty());
    }
}
#[test]
fn v2_empty() {
    let file = file("v2_empty");
    assert_eq!(file.version(), Version::V2);
    assert_eq!(file.entries().len(), 0);
    let tree = file.tree().unwrap();
    assert_eq!(tree.num_entries.unwrap_or_default(), 0);
    assert!(tree.name.is_empty());
    assert!(tree.children.is_empty());
    assert_eq!(tree.id, hex_to_id("4b825dc642cb6eb9a060e54bf8d69288fbee4904"));
    assert_eq!(
        file.checksum(),
        Some(hex_to_id("72d53f787d86a932a25a8537cee236d81846a8f1")),
        "checksums are read but not validated by default"
    );
}

#[test]
fn v2_empty_skip_hash() {
    let file = loose_file("skip_hash");
    assert_eq!(file.version(), Version::V2);
    assert_eq!(file.entries().len(), 0);
    let tree = file.tree().unwrap();
    assert_eq!(tree.num_entries.unwrap_or_default(), 0);
    assert!(tree.name.is_empty());
    assert!(tree.children.is_empty());
    assert_eq!(tree.id, hex_to_id("4b825dc642cb6eb9a060e54bf8d69288fbee4904"));
    assert_eq!(
        file.checksum(),
        None,
        "unset checksums are represented in the type system"
    );
}

#[test]
fn v2_with_multiple_entries_without_eoie_ext() {
    let file = file_needs_archive("v2_more_files");
    with_index_file_snapshot_filters(true, || {
        insta::assert_snapshot!(format!("{file:#?}"), @r#"
        File {
            path: "[redacted]",
            checksum: Some(
                Sha1(43bcf12743f506ab5fefaf13f8f5a7eed3d747fe),
            ),
            object_hash: Sha1,
            timestamp: FileTime { ... },
            version: V2,
            entries: [
                        Mode(FILE) mtime: Time { secs: 1717397605, nsecs: 248416030 } e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 a,
                        Mode(FILE) mtime: Time { secs: 1717397605, nsecs: 248416030 } e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 b,
                        Mode(FILE) mtime: Time { secs: 1717397605, nsecs: 248416030 } e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 c,
                        Mode(FILE) mtime: Time { secs: 1717397605, nsecs: 256416095 } e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 d/a,
                        Mode(FILE) mtime: Time { secs: 1717397605, nsecs: 256416095 } e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 d/b,
                        Mode(FILE) mtime: Time { secs: 1717397605, nsecs: 256416095 } e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 d/c,
            ],
            path_backing_size_bytes: 12,
            is_sparse: false,
            end_of_index_at_decode_time: false,
            offset_table_at_decode_time: false,
            tree: Some(
                Tree {
                    name: [],
                    id: Sha1(c9b29c3168d8e677450cc650238b23d9390801fb),
                    num_entries: Some(
                        6,
                    ),
                    children: [
                        Tree {
                            name: [
                                100,
                            ],
                            id: Sha1(765b32c65d38f04c4f287abda055818ec0f26912),
                            num_entries: Some(
                                3,
                            ),
                            children: [],
                        },
                    ],
                },
            ),
            has_link: false,
            has_resolve_undo: false,
            untracked: None,
            has_fs_monitor: false,
        }
        "#);
    });
}

fn find_shared_index_for(index: impl AsRef<Path>) -> PathBuf {
    let mut matches = std::fs::read_dir(index.as_ref().parent().unwrap())
        .unwrap()
        .map(Result::unwrap)
        .filter(|e: &std::fs::DirEntry| e.file_name().into_string().unwrap().starts_with("sharedindex."));
    let res = matches.next().unwrap();
    assert!(matches.next().is_none(), "found more than one shared indices");
    res.path()
}

#[test]
fn split_index_without_any_extension() {
    let file = gix_index::File::at(
        find_shared_index_for(crate::fixture_index_path("v2_split_index")),
        gix_hash::Kind::Sha1,
        false,
        Default::default(),
    )
    .unwrap();
    assert_eq!(file.version(), Version::V2);
}

#[test]
fn v3_extended_flags() {
    let file = loose_file("extended-flags");
    assert_eq!(file.version(), Version::V3);
}

#[test]
fn v2_very_long_path() {
    let file = loose_file("very-long-path");
    assert_eq!(file.version(), Version::V2);

    assert_eq!(file.entries().len(), 9);
    assert_eq!(
        file.entries()[0].path(&file),
        std::iter::repeat_n('a', 4096)
            .chain(std::iter::once('q'))
            .collect::<String>()
    );
    assert!(
        file.tree().is_some(),
        "Tree has invalid entries, but that shouldn't prevent us from loading it"
    );
    let tree = file.tree().expect("present");
    assert_eq!(tree.num_entries, None, "root tree has invalid entries actually");
    assert_eq!(tree.name.as_bstr(), "");
    assert_eq!(tree.num_entries, None, "it's marked invalid actually");
    assert!(tree.id.is_null(), "there is no id for the root");
}

#[test]
fn reuc_extension() {
    let file = loose_file("REUC");
    assert_eq!(file.version(), Version::V2);

    assert!(file.resolve_undo().is_some());
}

#[test]
fn untr_extension() {
    let file = loose_file("UNTR");
    assert_eq!(file.version(), Version::V2);

    assert!(file.untracked().is_some());
}

#[test]
fn untr_extension_with_oids() {
    let file = loose_file("UNTR-with-oids");
    assert_eq!(file.version(), Version::V2);

    assert!(file.untracked().is_some());
}

#[test]
fn untr_extension_empty() {
    let file = file_needs_archive("untracked_cache_empty");

    with_index_file_snapshot_filters(false, || {
        insta::assert_debug_snapshot!(&file, @r#"
        File {
            path: "[redacted]",
            checksum: Some(
                Sha1(e6e8bff2dab8feaa4cf41fd352248b0fc10acb56),
            ),
            object_hash: Sha1,
            timestamp: FileTime { ... },
            version: V2,
            entries: [
                        Mode(FILE) e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 tracked-dir/tracked-file,
                        Mode(FILE) e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 tracked-root-one,
                        Mode(FILE) e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 tracked-root-two,
            ],
            path_backing_size_bytes: 56,
            is_sparse: false,
            end_of_index_at_decode_time: false,
            offset_table_at_decode_time: false,
            tree: None,
            has_link: false,
            has_resolve_undo: false,
            untracked: Some(
                UntrackedCache {
                    identifier: "[redacted]",
                    info_exclude: None,
                    excludes_file: None,
                    exclude_filename_per_dir: ".gitignore",
                    dir_flags: 6,
                    directories: [],
                },
            ),
            has_fs_monitor: false,
        }
        "#);
    });
}

#[test]
fn untr_extension_populated() {
    let file = file_needs_archive("untracked_cache_populated");

    with_index_file_snapshot_filters(true, || {
        insta::assert_debug_snapshot!(&file, @r#"
        File {
            path: "[redacted]",
            checksum: Some(
                Sha1(dabefe909b6858676ca56f46db0d9a30ad0d2a97),
            ),
            object_hash: Sha1,
            timestamp: FileTime { ... },
            version: V2,
            entries: [
                        Mode(FILE) mtime: Time { secs: 2147483647, nsecs: 123456789 } e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 tracked-dir/tracked-file,
                        Mode(FILE) mtime: Time { secs: 2147483647, nsecs: 123456789 } e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 tracked-root-one,
                        Mode(FILE) mtime: Time { secs: 2147483647, nsecs: 123456789 } e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 tracked-root-two,
            ],
            path_backing_size_bytes: 56,
            is_sparse: false,
            end_of_index_at_decode_time: false,
            offset_table_at_decode_time: false,
            tree: None,
            has_link: false,
            has_resolve_undo: false,
            untracked: Some(
                UntrackedCache {
                    identifier: "[redacted]",
                    info_exclude: Some(
                        OidStat {
                            stat: Stat { mtime: Time { secs: 2147483647, nsecs: 123456789 }, ctime: Time { ... }, ... },
                            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
                        },
                    ),
                    excludes_file: None,
                    exclude_filename_per_dir: ".gitignore",
                    dir_flags: 6,
                    directories: [
                        Directory {
                            name: "",
                            untracked_entries: [
                                "untracked-root-file",
                                "untracked-dir-3/",
                                "untracked-dir-2/",
                            ],
                            sub_directories: [
                                1,
                                2,
                                3,
                            ],
                            stat: Some(
                                Stat { mtime: Time { secs: 2147483647, nsecs: 123456789 }, ctime: Time { ... }, ... },
                            ),
                            exclude_file_oid: None,
                            check_only: false,
                        },
                        Directory {
                            name: "tracked-dir",
                            untracked_entries: [],
                            sub_directories: [],
                            stat: Some(
                                Stat { mtime: Time { secs: 2147483647, nsecs: 123456789 }, ctime: Time { ... }, ... },
                            ),
                            exclude_file_oid: None,
                            check_only: false,
                        },
                        Directory {
                            name: "untracked-dir-2",
                            untracked_entries: [
                                "untracked-file-two",
                            ],
                            sub_directories: [],
                            stat: Some(
                                Stat { mtime: Time { secs: 2147483647, nsecs: 123456789 }, ctime: Time { ... }, ... },
                            ),
                            exclude_file_oid: None,
                            check_only: true,
                        },
                        Directory {
                            name: "untracked-dir-3",
                            untracked_entries: [
                                "untracked-file-three",
                            ],
                            sub_directories: [],
                            stat: Some(
                                Stat { mtime: Time { secs: 2147483647, nsecs: 123456789 }, ctime: Time { ... }, ... },
                            ),
                            exclude_file_oid: None,
                            check_only: true,
                        },
                    ],
                },
            ),
            has_fs_monitor: false,
        }
        "#);
    });
}

/// This mirrors Git's sparse/subdir untracked-cache coverage: a directory can
/// carry its own exclude-file oid, and nested untracked directories are
/// serialized depth-first while root sub-directory indices still point at
/// the corresponding directory records.
#[test]
fn untr_extension_nested() {
    let file = file_needs_archive("untracked_cache_nested");

    with_index_file_snapshot_filters(true, || {
        insta::assert_debug_snapshot!(&file, @r#"
        File {
            path: "[redacted]",
            checksum: Some(
                Sha1(bf50cd966cc718b67d3a326d01aa111f78901c1e),
            ),
            object_hash: Sha1,
            timestamp: FileTime { ... },
            version: V2,
            entries: [
                        Mode(FILE) mtime: Time { secs: 2147483647, nsecs: 123456789 } 55535cdccae965cd0ea191aa22df1145a983b2f9 tracked-dir-with-ignore/.gitignore,
                        Mode(FILE) mtime: Time { secs: 2147483647, nsecs: 123456789 } e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 tracked-dir-with-ignore/tracked-file,
                        Mode(FILE) mtime: Time { secs: 2147483647, nsecs: 123456789 } e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 tracked-root-one,
                        Mode(FILE) mtime: Time { secs: 2147483647, nsecs: 123456789 } e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 tracked-root-two,
            ],
            path_backing_size_bytes: 102,
            is_sparse: false,
            end_of_index_at_decode_time: false,
            offset_table_at_decode_time: false,
            tree: None,
            has_link: false,
            has_resolve_undo: false,
            untracked: Some(
                UntrackedCache {
                    identifier: "[redacted]",
                    info_exclude: Some(
                        OidStat {
                            stat: Stat { mtime: Time { secs: 2147483647, nsecs: 123456789 }, ctime: Time { ... }, ... },
                            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
                        },
                    ),
                    excludes_file: None,
                    exclude_filename_per_dir: ".gitignore",
                    dir_flags: 6,
                    directories: [
                        Directory {
                            name: "",
                            untracked_entries: [
                                "untracked-root-file",
                                "untracked-dir-3/",
                                "untracked-dir-2/",
                            ],
                            sub_directories: [
                                1,
                                4,
                                5,
                            ],
                            stat: Some(
                                Stat { mtime: Time { secs: 2147483647, nsecs: 123456789 }, ctime: Time { ... }, ... },
                            ),
                            exclude_file_oid: None,
                            check_only: false,
                        },
                        Directory {
                            name: "tracked-dir-with-ignore",
                            untracked_entries: [
                                "visible-untracked-file",
                                "nested-untracked-dir/",
                            ],
                            sub_directories: [
                                2,
                            ],
                            stat: Some(
                                Stat { mtime: Time { secs: 2147483647, nsecs: 123456789 }, ctime: Time { ... }, ... },
                            ),
                            exclude_file_oid: Some(
                                Sha1(55535cdccae965cd0ea191aa22df1145a983b2f9),
                            ),
                            check_only: false,
                        },
                        Directory {
                            name: "nested-untracked-dir",
                            untracked_entries: [
                                "deep-untracked-dir/",
                            ],
                            sub_directories: [
                                3,
                            ],
                            stat: Some(
                                Stat { mtime: Time { secs: 2147483647, nsecs: 123456789 }, ctime: Time { ... }, ... },
                            ),
                            exclude_file_oid: None,
                            check_only: true,
                        },
                        Directory {
                            name: "deep-untracked-dir",
                            untracked_entries: [
                                "deep-untracked-file",
                            ],
                            sub_directories: [],
                            stat: Some(
                                Stat { mtime: Time { secs: 2147483647, nsecs: 123456789 }, ctime: Time { ... }, ... },
                            ),
                            exclude_file_oid: None,
                            check_only: true,
                        },
                        Directory {
                            name: "untracked-dir-2",
                            untracked_entries: [
                                "untracked-file-two",
                            ],
                            sub_directories: [],
                            stat: Some(
                                Stat { mtime: Time { secs: 2147483647, nsecs: 123456789 }, ctime: Time { ... }, ... },
                            ),
                            exclude_file_oid: None,
                            check_only: true,
                        },
                        Directory {
                            name: "untracked-dir-3",
                            untracked_entries: [
                                "untracked-file-three",
                            ],
                            sub_directories: [],
                            stat: Some(
                                Stat { mtime: Time { secs: 2147483647, nsecs: 123456789 }, ctime: Time { ... }, ... },
                            ),
                            exclude_file_oid: None,
                            check_only: true,
                        },
                    ],
                },
            ),
            has_fs_monitor: false,
        }
        "#);
    });
}

#[test]
fn fsmn_v1() {
    let file = loose_file("FSMN");
    assert_eq!(file.version(), Version::V2);

    assert!(file.fs_monitor().is_some());
}

#[test]
fn v3_added_files() {
    let file = Fixture::Generated("v3_added_files").open();
    assert_eq!(file.version(), Version::V3, "uses extended attributes");
    assert_eq!(file.entries().len(), 1);
    assert_eq!(file.entries()[0].flags, Flags::EXTENDED | Flags::INTENT_TO_ADD);
}

#[test]
fn file_with_conflicts() {
    let file = loose_file("conflicting-file");
    assert_eq!(file.version(), Version::V2);
    assert_eq!(file.entries().len(), 3);
}

#[test]
fn v4_with_delta_paths_and_ieot_ext() {
    let file = file("v4_more_files_IEOT");
    assert_eq!(file.version(), Version::V4);
    assert!(file.had_end_of_index_marker());
    assert!(file.had_offset_table());

    assert_eq!(file.entries().len(), 10);
    for (idx, path) in [
        "a",
        "b",
        "c",
        "d/a",
        "d/b",
        "d/c",
        "d/last/123",
        "d/last/34",
        "d/last/6",
        "x",
    ]
    .iter()
    .enumerate()
    {
        let e = &file.entries()[idx];
        assert_eq!(e.path(&file), path);
        assert!(e.flags.is_empty());
        assert_eq!(e.mode, entry::Mode::FILE);
        assert_eq!(e.id, hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391"));
    }
}

#[test]
fn sparse_checkout_non_sparse_index() {
    let file = file("v3_skip_worktree");

    assert_eq!(file.version(), Version::V3);
    assert!(!file.is_sparse());
    file.entries().iter().for_each(|e| {
        assert_eq!(e.mode, Mode::FILE);
        let path = e.path(&file);
        if path.starts_with("d".as_bytes()) || path.starts_with("c1/c3".as_bytes()) {
            assert_eq!(e.flags, Flags::EXTENDED | Flags::SKIP_WORKTREE);
        } else {
            assert_eq!(e.flags, Flags::empty());
        }
    });
}

#[test]
fn sparse_checkout_cone_mode() {
    let file = file("v3_sparse_index");

    assert_eq!(file.version(), Version::V3);
    assert!(file.is_sparse());
    file.entries().iter().for_each(|e| {
        let path = e.path(&file);
        if path.starts_with("c1/c3".as_bytes()) || path.starts_with("d".as_bytes()) {
            assert_eq!(e.mode, Mode::DIR);
            assert_eq!(e.flags, Flags::EXTENDED | Flags::SKIP_WORKTREE);
        } else {
            assert_eq!(e.mode, Mode::FILE);
            assert_eq!(e.flags, Flags::empty());
        }
    });
}

#[test]
fn sparse_checkout_cone_mode_no_dirs() {
    let file = file("v2_sparse_index_no_dirs");

    assert_eq!(file.version(), Version::V2);
    assert!(file.is_sparse());
    file.entries().iter().for_each(|e| {
        assert_eq!(e.mode, Mode::FILE);
        assert_eq!(e.flags, Flags::empty());
    });
}

#[test]
fn sparse_checkout_non_cone_mode() {
    let file = file("v3_sparse_index_non_cone");

    assert_eq!(file.version(), Version::V3);
    assert!(!file.is_sparse());
    file.entries().iter().for_each(|e| {
        assert_eq!(e.mode, Mode::FILE);
        if e.path(&file).starts_with("c1/c2".as_bytes()) {
            assert_eq!(e.flags, Flags::empty());
        } else {
            assert_eq!(e.flags, Flags::EXTENDED | Flags::SKIP_WORKTREE);
        }
    });
}

#[test]
fn v2_split_index() {
    let file = file("v2_split_index");
    assert_eq!(file.version(), Version::V2);
}

#[test]
fn v2_split_index_recursion_is_handled_gracefully() {
    let err = try_file("v2_split_index_recursive", false).expect_err("recursion fails gracefully");
    assert!(matches!(
        err,
        gix_index::file::init::Error::Decode(gix_index::decode::Error::Verify(_))
    ));
}

#[test]
fn split_index_and_regular_index_of_same_content_are_indeed_the_same() {
    let base = crate::scripted_fixture_read_only(Path::new("make_index").join("v2_split_vs_regular_index.sh")).unwrap();

    let split = verify(
        gix_index::File::at(
            base.join("split/.git/index"),
            gix_hash::Kind::Sha1,
            false,
            Default::default(),
        )
        .unwrap(),
    );

    assert!(
        split.link().is_none(),
        "link extension is dissolved, merging the shared index permanently into the split one (for now)"
    );

    let regular = verify(
        gix_index::File::at(
            base.join("regular/.git/index"),
            gix_hash::Kind::Sha1,
            false,
            Default::default(),
        )
        .unwrap(),
    );

    assert_eq!(
        split.entries().len(),
        regular.entries().len(),
        "split and regular index entries must match in length (and be the exact same)"
    );
    split.entries().iter().zip(regular.entries()).for_each(|(s, r)| {
        assert_eq!(s.id, r.id);
        assert_eq!(s.flags, r.flags);
        assert_eq!(s.path_in(split.path_backing()), r.path_in(regular.path_backing()));
    });
}
