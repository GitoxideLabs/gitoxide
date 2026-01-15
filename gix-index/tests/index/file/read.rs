use std::path::{Path, PathBuf};

use bstr::ByteSlice;
use gix_index::{
    entry::{self, Flags, Mode},
    Version,
};

use crate::{hex_to_id, index::Fixture, loose_file_path};

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
pub(crate) fn try_file(name: &str) -> Result<gix_index::File, gix_index::file::init::Error> {
    let file = gix_index::File::at(
        crate::fixture_index_path(name),
        gix_hash::Kind::Sha1,
        false,
        Default::default(),
    )?;
    Ok(verify(file))
}
pub(crate) fn file(name: &str) -> gix_index::File {
    try_file(name).unwrap()
}
fn file_opt(name: &str, opts: gix_index::decode::Options) -> gix_index::File {
    let file = gix_index::File::at(crate::fixture_index_path(name), gix_hash::Kind::Sha1, false, opts).unwrap();
    verify(file)
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
    let file = file("v2_more_files");
    assert_eq!(file.version(), Version::V2);

    assert_eq!(file.entries().len(), 6);
    for (idx, path) in ["a", "b", "c", "d/a", "d/b", "d/c"].iter().enumerate() {
        let e = &file.entries()[idx];
        assert_eq!(e.path(&file), path);
        assert!(e.flags.is_empty());
        assert_eq!(e.mode, entry::Mode::FILE);
        assert_eq!(e.id, hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391"));
    }

    let tree = file.tree().unwrap();
    assert_eq!(tree.id, hex_to_id("c9b29c3168d8e677450cc650238b23d9390801fb"));
    assert_eq!(tree.num_entries.unwrap_or_default(), 6);
    assert!(tree.name.is_empty());
    assert_eq!(tree.children.len(), 1);

    let tree = &tree.children[0];
    assert_eq!(tree.id, hex_to_id("765b32c65d38f04c4f287abda055818ec0f26912"));
    assert_eq!(tree.num_entries.unwrap_or_default(), 3);
    assert_eq!(tree.name.as_bstr(), "d");
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
    let err = try_file("v2_split_index_recursive").expect_err("recursion fails gracefully");
    assert!(matches!(
        err,
        gix_index::file::init::Error::Decode(gix_index::decode::Error::Verify(_))
    ));
}

#[test]
fn split_index_and_regular_index_of_same_content_are_indeed_the_same() {
    let base = gix_testtools::scripted_fixture_read_only_standalone(
        Path::new("make_index").join("v2_split_vs_regular_index.sh"),
    )
    .unwrap();

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
