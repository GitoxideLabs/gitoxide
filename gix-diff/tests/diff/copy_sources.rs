use std::path::Path;

use bstr::{BStr, BString, ByteSlice};
use gix_diff::{Rewrites, rewrites::Copies};
use gix_error::TestResult;
use gix_object::FindExt;

#[test]
fn tree_and_index_copies_use_the_original_source() -> TestResult {
    let root = crate::scripted_fixture_read_only("make_copy_sources.sh")?;
    let objects = crate::open_odb(root.join(".git/objects"))?;
    for (case, source) in [
        ("modified", gix_diff::rewrites::CopySource::FromSetOfModifiedFiles),
        (
            "harder",
            gix_diff::rewrites::CopySource::FromSetOfModifiedFilesAndAllSources,
        ),
    ] {
        let before_tree_id = read_tree_id(&root, case, "before")?;
        let after_tree_id = read_tree_id(&root, case, "after")?;
        let before = gix_index::State::from_tree(&before_tree_id, &objects, Default::default())?;
        let after = gix_index::State::from_tree(&after_tree_id, &objects, Default::default())?;
        let source_index = before
            .entry_index_by_path("z-source".into())
            .expect("fixture source exists");
        let source_entry = &before.entries()[source_index];
        let rewrites = Rewrites {
            copies: Some(Copies {
                source,
                percentage: None,
            }),
            percentage: None,
            ..Default::default()
        };
        let mut cache = new_cache(&root);
        let mut expected = std::fs::read_to_string(root.join(format!(".git/{case}.git")))?
            .lines()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        expected.sort();

        let mut index_changes = Vec::new();
        gix_diff::index(
            &before,
            &after,
            |change| {
                use gix_diff::index::ChangeRef;
                index_changes.push(match change {
                    ChangeRef::Rewrite {
                        source_location,
                        source_index: actual_index,
                        source_entry_mode,
                        source_id,
                        location,
                        copy,
                        ..
                    } => {
                        assert!(copy, "a surviving source is a copy");
                        assert_eq!(
                            actual_index, source_index,
                            "the source index belongs to the original index"
                        );
                        assert_eq!(
                            source_entry_mode, source_entry.mode,
                            "copies retain the original source mode"
                        );
                        assert_eq!(
                            source_id.as_ref(),
                            source_entry.id.as_ref(),
                            "copies retain the original source object"
                        );
                        format!("C100\t{source_location}\t{location}")
                    }
                    ChangeRef::Addition { location, .. } => format!("A\t{location}"),
                    ChangeRef::Deletion { location, .. } => format!("D\t{location}"),
                    ChangeRef::Modification { location, .. } => format!("M\t{location}"),
                });
                Ok::<_, gix_error::Error>(std::ops::ControlFlow::Continue(()))
            },
            Some(gix_diff::index::RewriteOptions {
                rewrites,
                resource_cache: &mut cache,
                find: &objects,
            }),
            &mut gix_pathspec::Search::from_specs([], None, &root)?,
            &mut |_, _, _, _| false,
        )?;
        index_changes.sort();
        assert_eq!(index_changes, expected, "index copies match Git for {case}");

        let (mut before_buf, mut after_buf) = (Vec::new(), Vec::new());
        let before_tree = objects.find_tree_iter(&before_tree_id, &mut before_buf)?;
        let after_tree = objects.find_tree_iter(&after_tree_id, &mut after_buf)?;
        let mut tree_changes = Vec::new();
        gix_diff::tree_with_rewrites(
            before_tree,
            after_tree,
            &mut cache,
            &mut Default::default(),
            &objects,
            |change| {
                use gix_diff::tree_with_rewrites::ChangeRef;
                tree_changes.push(match change {
                    ChangeRef::Rewrite {
                        source_location,
                        source_entry_mode,
                        source_id,
                        location,
                        copy,
                        ..
                    } => {
                        assert!(copy, "a surviving source is a copy");
                        assert_eq!(
                            source_entry_mode,
                            source_entry.mode.to_tree_entry_mode().expect("valid fixture mode"),
                            "copies retain the original source mode"
                        );
                        assert_eq!(source_id, source_entry.id, "copies retain the original source object");
                        format!("C100\t{source_location}\t{location}")
                    }
                    ChangeRef::Addition { location, .. } => format!("A\t{location}"),
                    ChangeRef::Deletion { location, .. } => format!("D\t{location}"),
                    ChangeRef::Modification { location, .. } => format!("M\t{location}"),
                });
                Ok(std::ops::ControlFlow::Continue(()))
            },
            gix_diff::tree_with_rewrites::Options {
                location: Some(gix_diff::tree::recorder::Location::Path),
                rewrites: Some(rewrites),
            },
        )?;
        tree_changes.sort();
        assert_eq!(tree_changes, expected, "tree copies match Git for {case}");
    }
    Ok(())
}

#[test]
fn copy_threshold_does_not_lower_the_rename_threshold() -> TestResult {
    use gix_diff::tree::visit::Change;
    use gix_object::tree::EntryKind;

    let root = crate::scripted_fixture_read_only("make_copy_sources.sh")?;
    let objects = crate::util::object_db();
    let before_blob_id = crate::util::insert(
        &objects,
        "one
two
three
four
",
    )?;
    let after_blob_id = crate::util::insert(
        &objects,
        "one
two
three
four
five
",
    )?;
    for deleted in [false, true] {
        let mut tracker = gix_diff::rewrites::Tracker::new(Rewrites {
            percentage: None,
            copies: Some(Copies::default()),
            ..Default::default()
        });
        let source = if deleted {
            Change::Deletion {
                entry_mode: EntryKind::Blob.into(),
                oid: before_blob_id,
                relation: None,
            }
        } else {
            Change::Modification {
                previous_entry_mode: EntryKind::Blob.into(),
                previous_oid: before_blob_id,
                entry_mode: EntryKind::Blob.into(),
                oid: after_blob_id,
            }
        };
        assert!(tracker.try_push_change(source, "source".into()).is_none());
        assert!(
            tracker
                .try_push_change(
                    Change::Addition {
                        entry_mode: EntryKind::Blob.into(),
                        oid: after_blob_id,
                        relation: None,
                    },
                    "destination".into(),
                )
                .is_none()
        );
        let mut rewrites = Vec::new();
        let mut unmatched = Vec::new();
        tracker.emit(
            |destination, source| {
                match source {
                    Some(source) => rewrites.push(source.kind),
                    None => unmatched.push(destination.location.to_owned()),
                }
                std::ops::ControlFlow::Continue(())
            },
            &mut new_cache(&root),
            &objects,
            |_| Ok::<_, std::convert::Infallible>(()),
        )?;
        if deleted {
            assert!(
                rewrites.is_empty(),
                "a fuzzy copy threshold must not enable fuzzy renames"
            );
            assert_eq!(
                unmatched,
                ["destination", "source"],
                "the addition and deletion remain visible"
            );
        } else {
            assert_eq!(rewrites, [gix_diff::rewrites::tracker::visit::SourceKind::Copy]);
            assert_eq!(unmatched, ["source"], "copy tracking retains the original modification");
        }
    }
    Ok(())
}

#[test]
fn incompatible_sizes_skip_payloads() -> TestResult {
    use gix_diff::tree::visit::Change;
    use gix_object::tree::EntryKind;

    struct HeadersOnly {
        inner: crate::util::ObjectDb,
    }
    impl gix_object::Find for HeadersOnly {
        fn try_find<'a>(
            &self,
            id: &gix_hash::oid,
            _buffer: &'a mut Vec<u8>,
        ) -> gix_error::Result<Option<gix_object::Data<'a>>> {
            panic!("size-incompatible blob {id} must be rejected before fetching its payload");
        }
    }
    impl gix_object::FindHeader for HeadersOnly {
        fn try_header(&self, id: &gix_hash::oid) -> gix_error::Result<Option<gix_object::Header>> {
            self.inner.try_header(id)
        }
    }
    let objects = HeadersOnly {
        inner: crate::util::object_db(),
    };
    let mut tracker = gix_diff::rewrites::Tracker::new(Rewrites::default());
    for idx in 0..2 {
        let blob_id = crate::util::insert(&objects.inner, &format!("small source {idx}"))?;
        assert!(
            tracker
                .try_push_change(
                    Change::Deletion {
                        entry_mode: EntryKind::Blob.into(),
                        oid: blob_id,
                        relation: None,
                    },
                    format!("source-{idx}").as_bytes().as_bstr(),
                )
                .is_none()
        );
    }
    for idx in 0..3 {
        let blob_id = crate::util::insert(&objects.inner, &format!("large destination {idx}").repeat(100))?;
        assert!(
            tracker
                .try_push_change(
                    Change::Addition {
                        entry_mode: EntryKind::Blob.into(),
                        oid: blob_id,
                        relation: None,
                    },
                    format!("destination-{idx}").as_bytes().as_bstr(),
                )
                .is_none()
        );
    }
    let root = crate::scripted_fixture_read_only("make_copy_sources.sh")?;
    let mut changes = 0;
    let outcome = tracker.emit(
        |_, source| {
            assert!(source.is_none(), "the size ratio rules out every rename");
            changes += 1;
            std::ops::ControlFlow::Continue(())
        },
        &mut new_cache(&root),
        &objects,
        |_| Ok::<_, std::convert::Infallible>(()),
    )?;
    assert_eq!(changes, 5, "all additions and deletions remain visible");
    assert_eq!(
        outcome.num_similarity_checks, 6,
        "every source/destination pair is considered"
    );
    Ok(())
}

#[test]
fn identical_directory_does_not_reserve_the_first_copy_as_a_rename() -> TestResult {
    directory_rewrites_match_git("directory-copy-exact", 1.0, true, &[("old", "b")])
}

#[test]
fn identical_directory_does_not_reserve_the_exact_copy_as_a_rename() -> TestResult {
    directory_rewrites_match_git("directory-copy-edited", 0.5, true, &[("old", "b")])
}

#[test]
fn identical_directories_do_not_preempt_duplicate_file_sources() -> TestResult {
    directory_rewrites_match_git(
        "directory-duplicate-sources",
        1.0,
        false,
        &[("old-a", "new-a"), ("old-b", "new-b")],
    )
}

#[test]
fn split_directory_follows_the_unique_highest_destination() -> TestResult {
    directory_rewrites_match_git("directory-split", 1.0, false, &[("old", "winner")])
}

#[test]
fn equally_split_directory_remains_unpaired() -> TestResult {
    directory_rewrites_match_git("directory-tie", 1.0, false, &[])
}

#[test]
fn directories_with_only_empty_files_have_no_renames_to_infer_from() -> TestResult {
    directory_rewrites_match_git("directory-empty", 1.0, false, &[])
}

#[test]
fn nested_directory_move_does_not_imply_a_root_move() -> TestResult {
    directory_rewrites_match_git("directory-different-suffix", 1.0, false, &[("old/sub", "new/other")])
}

#[test]
fn nested_directory_move_is_retained_beside_a_root_move() -> TestResult {
    directory_rewrites_match_git("directory-nested", 1.0, false, &[("a", "b"), ("a/sub", "b/renamed")])
}

#[test]
fn nested_directory_move_into_a_new_parent() -> TestResult {
    directory_rewrites_match_git("directory-deeper", 1.0, false, &[("old", "new/sub")])
}

#[test]
fn cancellation_stops_file_changes_and_inferred_directories() -> TestResult {
    let root = crate::scripted_fixture_read_only("make_copy_sources.sh")?;
    let objects = crate::open_odb(root.join(".git/objects"))?;
    let before_tree_id = read_tree_id(&root, "directory-split", "before")?;
    let after_tree_id = read_tree_id(&root, "directory-split", "after")?;
    let mut expected = Vec::new();
    let mut cache = new_cache(&root);
    for stop_after in [None, Some(1), Some(3), Some(4)] {
        let (mut before_buf, mut after_buf) = (Vec::new(), Vec::new());
        let mut actual = Vec::new();
        gix_diff::tree_with_rewrites(
            objects.find_tree_iter(&before_tree_id, &mut before_buf)?,
            objects.find_tree_iter(&after_tree_id, &mut after_buf)?,
            &mut cache,
            &mut Default::default(),
            &objects,
            |change| {
                actual.push(change.into_owned());
                Ok(if Some(actual.len()) == stop_after {
                    std::ops::ControlFlow::Break(())
                } else {
                    std::ops::ControlFlow::Continue(())
                })
            },
            gix_diff::tree_with_rewrites::Options {
                location: Some(gix_diff::tree::recorder::Location::Path),
                rewrites: Some(Rewrites {
                    percentage: None,
                    ..Default::default()
                }),
            },
        )?;
        if let Some(stop_after) = stop_after {
            assert_eq!(
                actual,
                expected[..stop_after],
                "cancellation prevents every later callback, including directory inference"
            );
        } else {
            assert_eq!(
                actual.len(),
                5,
                "three file renames, one directory rename and one added directory"
            );
            assert!(
                actual[..3].iter().all(|change| !change.entry_mode().is_tree())
                    && actual[3..].iter().all(|change| change.entry_mode().is_tree()),
                "file matching completes before directory changes are emitted"
            );
            expected = actual;
        }
    }
    Ok(())
}

#[test]
fn location_projection_preserves_file_and_directory_matches() -> TestResult {
    use gix_diff::{tree::recorder::Location, tree_with_rewrites::Change};

    let root = crate::scripted_fixture_read_only("make_copy_sources.sh")?;
    let objects = crate::open_odb(root.join(".git/objects"))?;
    let before_tree_id = read_tree_id(&root, "directory-split", "before")?;
    let after_tree_id = read_tree_id(&root, "directory-split", "after")?;
    let mut cache = new_cache(&root);
    let mut expected_changes = Vec::new();
    for (location, expected_paths) in [
        (
            Some(Location::Path),
            [
                ("old", "winner"),
                ("old/one", "winner/renamed"),
                ("old/three", "other/three"),
                ("old/two", "winner/two"),
                ("other", "other"),
            ],
        ),
        (
            Some(Location::FileName),
            [
                ("old", "winner"),
                ("one", "renamed"),
                ("other", "other"),
                ("three", "three"),
                ("two", "two"),
            ],
        ),
        (None, [("", ""); 5]),
    ] {
        let (mut before_buf, mut after_buf) = (Vec::new(), Vec::new());
        let (mut changes, mut paths) = (Vec::new(), Vec::new());
        gix_diff::tree_with_rewrites(
            objects.find_tree_iter(&before_tree_id, &mut before_buf)?,
            objects.find_tree_iter(&after_tree_id, &mut after_buf)?,
            &mut cache,
            &mut Default::default(),
            &objects,
            |change| {
                let mut change = change.into_owned();
                paths.push((change.source_location().to_owned(), change.location().to_owned()));
                if let Change::Rewrite { source_location, .. } = &mut change {
                    source_location.clear();
                }
                match &mut change {
                    Change::Addition { location, .. }
                    | Change::Deletion { location, .. }
                    | Change::Modification { location, .. }
                    | Change::Rewrite { location, .. } => location.clear(),
                }
                changes.push(change);
                Ok(std::ops::ControlFlow::Continue(()))
            },
            gix_diff::tree_with_rewrites::Options {
                location,
                rewrites: Some(Rewrites {
                    percentage: None,
                    ..Default::default()
                }),
            },
        )?;
        paths.sort();
        assert_eq!(
            paths,
            expected_paths.map(|(source, destination)| (BString::from(source), BString::from(destination))),
            "callback locations follow the requested projection"
        );
        if expected_changes.is_empty() {
            expected_changes = changes;
        } else {
            assert_eq!(
                changes, expected_changes,
                "location projection preserves chosen objects, relations and callback order"
            );
        }
    }
    Ok(())
}

fn directory_rewrites_match_git(
    case: &str,
    percentage: f32,
    copies: bool,
    expected_directories: &[(&str, &str)],
) -> TestResult {
    let root = crate::scripted_fixture_read_only("make_copy_sources.sh")?;
    let objects = crate::open_odb(root.join(".git/objects"))?;
    let before_tree_id = read_tree_id(&root, case, "before")?;
    let after_tree_id = read_tree_id(&root, case, "after")?;
    let before = gix_index::State::from_tree(&before_tree_id, &objects, Default::default())?;
    let after = gix_index::State::from_tree(&after_tree_id, &objects, Default::default())?;
    let null_id = crate::fixture_hash_kind().null();
    let rewrites = Rewrites {
        percentage: Some(percentage),
        copies: copies.then_some(Copies {
            percentage: Some(percentage),
            ..Default::default()
        }),
        limit: 0,
        ..Default::default()
    };
    let expected = std::fs::read(root.join(format!(".git/{case}.raw")))?;
    let mut cache = new_cache(&root);
    let mut actual = Vec::new();
    gix_diff::index(
        &before,
        &after,
        |change| {
            use gix_diff::index::ChangeRef;
            actual.push(match change {
                ChangeRef::Rewrite {
                    source_location,
                    source_index,
                    source_entry_mode,
                    source_id,
                    location,
                    entry_mode,
                    id,
                    copy,
                    ..
                } => {
                    assert_eq!(
                        Ok(source_index),
                        before.entry_index_by_path(source_location.as_ref()),
                        "the rewrite source index belongs to its reported old path"
                    );
                    raw_change(
                        format!(
                            ":{:06o} {:06o} {} {} {}",
                            source_entry_mode.bits(),
                            entry_mode.bits(),
                            source_id.as_ref(),
                            id.as_ref(),
                            if copy { 'C' } else { 'R' }
                        ),
                        &[source_location.as_ref(), location.as_ref()],
                    )
                }
                ChangeRef::Addition {
                    location,
                    entry_mode,
                    id,
                    ..
                } => raw_change(
                    format!(":000000 {:06o} {null_id} {} A", entry_mode.bits(), id.as_ref()),
                    &[location.as_ref()],
                ),
                ChangeRef::Deletion {
                    location,
                    entry_mode,
                    id,
                    ..
                } => raw_change(
                    format!(":{:06o} 000000 {} {null_id} D", entry_mode.bits(), id.as_ref()),
                    &[location.as_ref()],
                ),
                ChangeRef::Modification { .. } => panic!("these fixtures only contain additions and deletions"),
            });
            Ok::<_, gix_error::Error>(std::ops::ControlFlow::Continue(()))
        },
        Some(gix_diff::index::RewriteOptions {
            rewrites,
            resource_cache: &mut cache,
            find: &objects,
        }),
        &mut gix_pathspec::Search::from_specs([], None, &root)?,
        &mut |_, _, _, _| false,
    )?;
    actual.sort();
    assert_eq!(
        actual,
        raw_records(&expected, false),
        "index rewrites match Git for {case}"
    );

    let (mut before_buf, mut after_buf) = (Vec::new(), Vec::new());
    let mut actual = Vec::new();
    let mut directories = Vec::new();
    gix_diff::tree_with_rewrites(
        objects.find_tree_iter(&before_tree_id, &mut before_buf)?,
        objects.find_tree_iter(&after_tree_id, &mut after_buf)?,
        &mut cache,
        &mut Default::default(),
        &objects,
        |change| {
            use gix_diff::tree_with_rewrites::ChangeRef;
            if change.entry_mode().is_tree() {
                if let ChangeRef::Rewrite {
                    source_location,
                    location,
                    ..
                } = change
                {
                    directories.push((source_location.to_owned(), location.to_owned()));
                }
                return Ok(std::ops::ControlFlow::Continue(()));
            }
            actual.push(match change {
                ChangeRef::Rewrite {
                    source_location,
                    source_entry_mode,
                    source_id,
                    location,
                    entry_mode,
                    id,
                    similarity,
                    copy,
                    ..
                } => raw_change(
                    format!(
                        ":{source_entry_mode:06o} {entry_mode:06o} {source_id} {id} {}{:03}",
                        if copy { 'C' } else { 'R' },
                        similarity.map_or(100, gix_diff::rewrites::Similarity::percentage)
                    ),
                    &[source_location, location],
                ),
                ChangeRef::Addition {
                    location,
                    entry_mode,
                    id,
                    ..
                } => raw_change(format!(":000000 {entry_mode:06o} {null_id} {id} A"), &[location]),
                ChangeRef::Deletion {
                    location,
                    entry_mode,
                    id,
                    ..
                } => raw_change(format!(":{entry_mode:06o} 000000 {id} {null_id} D"), &[location]),
                ChangeRef::Modification { .. } => panic!("these fixtures only contain additions and deletions"),
            });
            Ok(std::ops::ControlFlow::Continue(()))
        },
        gix_diff::tree_with_rewrites::Options {
            location: Some(gix_diff::tree::recorder::Location::Path),
            rewrites: Some(rewrites),
        },
    )?;
    actual.sort();
    assert_eq!(
        actual,
        raw_records(&expected, true),
        "tree rewrites match Git for {case}"
    );
    directories.sort();
    let expected_directories: Vec<_> = expected_directories
        .iter()
        .map(|&(source, destination)| (BString::from(source), BString::from(destination)))
        .collect();
    assert_eq!(
        directories, expected_directories,
        "the inferred directory renames follow the chosen file sources"
    );
    Ok(())
}

fn raw_change(header: String, paths: &[&BStr]) -> BString {
    let mut record = header.into_bytes();
    record.push(0);
    for path in paths {
        record.extend_from_slice(path);
        record.push(0);
    }
    record.into()
}

fn raw_records(data: &[u8], keep_scores: bool) -> Vec<BString> {
    let mut fields = data.split(|byte| *byte == 0);
    let mut records = Vec::new();
    while let Some(header) = fields.next().filter(|header| !header.is_empty()) {
        let status_offset = header.iter().rposition(|byte| *byte == b' ').expect("raw status") + 1;
        let is_rewrite = matches!(header[status_offset], b'R' | b'C');
        let header = if is_rewrite && !keep_scores {
            &header[..status_offset + 1]
        } else {
            header
        };
        let mut paths = vec![fields.next().expect("raw source or destination path").as_bstr()];
        if is_rewrite {
            paths.push(fields.next().expect("raw rewrite destination path").as_bstr());
        }
        records.push(raw_change(
            header.to_str().expect("raw headers are ASCII").to_owned(),
            &paths,
        ));
    }
    records.sort();
    records
}

fn read_tree_id(root: &Path, case: &str, version: &str) -> TestResult<gix_hash::ObjectId> {
    Ok(gix_hash::ObjectId::from_hex(
        std::fs::read(root.join(format!(".git/{case}-{version}.tree")))?.trim(),
    )?)
}

fn new_cache(root: &Path) -> gix_diff::blob::Platform {
    gix_diff::blob::Platform::new(
        Default::default(),
        gix_diff::blob::Pipeline::new(
            Default::default(),
            gix_filter::Pipeline::new(Default::default(), crate::fixture_hash_kind(), Default::default()),
            Vec::new(),
            Default::default(),
        ),
        gix_diff::blob::pipeline::Mode::ToGit,
        gix_worktree::Stack::new(
            root,
            gix_worktree::stack::State::AttributesStack(Default::default()),
            Default::default(),
            Vec::new(),
            Vec::new(),
        ),
    )
}
