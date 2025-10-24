use std::{collections::BTreeMap, path::PathBuf};

use gix_blame::BlameRanges;
use gix_hash::ObjectId;
use gix_object::bstr;

struct Baseline<'a> {
    lines: bstr::Lines<'a>,
    filenames: BTreeMap<ObjectId, bstr::BString>,
}

mod baseline {
    use std::{collections::BTreeMap, path::Path};

    use gix_blame::BlameEntry;
    use gix_hash::ObjectId;
    use gix_ref::bstr::ByteSlice;

    use super::Baseline;

    // These fields are used by `git` in its porcelain output.
    const HEADER_FIELDS: [&str; 12] = [
        // https://github.com/git/git/blob/6258f68c3c1092c901337895c864073dcdea9213/builtin/blame.c#L256-L280
        "author",
        "author-mail",
        "author-time",
        "author-tz",
        "committer",
        "committer-mail",
        "committer-time",
        "committer-tz",
        "summary",
        "boundary",
        // https://github.com/git/git/blob/6258f68c3c1092c901337895c864073dcdea9213/builtin/blame.c#L239-L248
        "previous",
        "filename",
    ];

    fn is_known_header_field(field: &&str) -> bool {
        HEADER_FIELDS.contains(field)
    }

    impl Baseline<'_> {
        pub fn collect(
            baseline_path: impl AsRef<Path>,
            source_file_name: gix_object::bstr::BString,
        ) -> std::io::Result<Vec<BlameEntry>> {
            let content = std::fs::read(baseline_path)?;
            let baseline = Baseline {
                lines: content.lines(),
                filenames: BTreeMap::default(),
            };

            Ok(baseline
                .map(|entry| {
                    let source_file_name = if entry.source_file_name.as_ref() == Some(&source_file_name) {
                        None
                    } else {
                        entry.source_file_name
                    };

                    BlameEntry {
                        source_file_name,
                        ..entry
                    }
                })
                .collect())
        }
    }

    impl Iterator for Baseline<'_> {
        type Item = BlameEntry;

        fn next(&mut self) -> Option<Self::Item> {
            let mut ranges = None;
            let mut commit_id = gix_hash::Kind::Sha1.null();
            let mut skip_lines: u32 = 0;
            let mut source_file_name: Option<gix_object::bstr::BString> = None;

            for line in self.lines.by_ref() {
                if line.starts_with(b"\t") {
                    // Each group consists of a header and one or more lines. We break from the
                    // loop, thus returning a `BlameEntry` from `next` once we have seen the number
                    // of lines starting with "\t" as indicated in the group’s header.
                    skip_lines -= 1;

                    if skip_lines == 0 {
                        break;
                    } else {
                        continue;
                    }
                }

                let fields: Vec<&str> = line.to_str().unwrap().split(' ').collect();
                if fields.len() == 4 {
                    // We’re possibly dealing with a group header.
                    // If we can’t parse the first field as an `ObjectId`, we know this is not a
                    // group header, so we continue. This can yield false positives, but for
                    // testing purposes, we don’t bother.
                    commit_id = match ObjectId::from_hex(fields[0].as_bytes()) {
                        Ok(id) => id,
                        Err(_) => continue,
                    };

                    let line_number_in_source_file = fields[1].parse::<u32>().unwrap();
                    let line_number_in_final_file = fields[2].parse::<u32>().unwrap();
                    // The last field indicates the number of lines this group contains info for
                    // (this is not equal to the number of lines in git blame’s porcelain output).
                    let number_of_lines_in_group = fields[3].parse::<u32>().unwrap();

                    skip_lines = number_of_lines_in_group;

                    let source_range =
                        (line_number_in_source_file - 1)..(line_number_in_source_file + number_of_lines_in_group - 1);
                    let blame_range =
                        (line_number_in_final_file - 1)..(line_number_in_final_file + number_of_lines_in_group - 1);
                    assert!(ranges.is_none(), "should not overwrite existing ranges");
                    ranges = Some((blame_range, source_range));
                } else if fields[0] == "filename" {
                    // We need to store `source_file_name` as it is not repeated for subsequent
                    // hunks that have the same `commit_id`.
                    source_file_name = Some(fields[1].into());

                    self.filenames.insert(commit_id, fields[1].into());
                } else if !is_known_header_field(&fields[0]) && ObjectId::from_hex(fields[0].as_bytes()).is_err() {
                    panic!("unexpected line: '{:?}'", line.as_bstr());
                }
            }

            let Some((range_in_blamed_file, range_in_source_file)) = ranges else {
                // No new lines were parsed, so we assume the iterator is finished.
                return None;
            };
            Some(BlameEntry::new(
                range_in_blamed_file,
                range_in_source_file,
                commit_id,
                source_file_name.or_else(|| self.filenames.get(&commit_id).cloned()),
            ))
        }
    }
}

struct Fixture {
    odb: gix_odb::Handle,
    resource_cache: gix_diff::blob::Platform,
    suspect: ObjectId,
}

impl Fixture {
    fn new() -> gix_testtools::Result<Fixture> {
        Self::for_worktree_path(fixture_path()?)
    }

    fn for_worktree_path(worktree_path: PathBuf) -> gix_testtools::Result<Fixture> {
        use gix_ref::store::WriteReflog;

        let store = gix_ref::file::Store::at(
            worktree_path.join(".git"),
            gix_ref::store::init::Options {
                write_reflog: WriteReflog::Disable,
                ..Default::default()
            },
        );
        let odb = gix_odb::at(worktree_path.join(".git/objects"))?;

        let mut reference = gix_ref::file::Store::find(&store, "HEAD")?;

        // Needed for `peel_to_id`.
        use gix_ref::file::ReferenceExt;

        let head_id = reference.peel_to_id(&store, &odb)?;

        let git_dir = worktree_path.join(".git");
        let index = gix_index::File::at(git_dir.join("index"), gix_hash::Kind::Sha1, false, Default::default())?;
        let stack = gix_worktree::Stack::from_state_and_ignore_case(
            worktree_path.clone(),
            false,
            gix_worktree::stack::State::AttributesAndIgnoreStack {
                attributes: Default::default(),
                ignore: Default::default(),
            },
            &index,
            index.path_backing(),
        );
        let capabilities = gix_fs::Capabilities::probe(&git_dir);
        let resource_cache = gix_diff::blob::Platform::new(
            Default::default(),
            gix_diff::blob::Pipeline::new(
                gix_diff::blob::pipeline::WorktreeRoots {
                    old_root: None,
                    new_root: None,
                },
                gix_filter::Pipeline::new(Default::default(), Default::default()),
                vec![],
                gix_diff::blob::pipeline::Options {
                    large_file_threshold_bytes: 0,
                    fs: capabilities,
                },
            ),
            gix_diff::blob::pipeline::Mode::ToGit,
            stack,
        );
        Ok(Fixture {
            odb,
            resource_cache,
            suspect: head_id,
        })
    }
}

macro_rules! mktest {
    ($name:ident, $case:expr, $number_of_lines:literal) => {
        #[test]
        fn $name() -> gix_testtools::Result {
            let Fixture {
                odb,
                mut resource_cache,
                suspect,
            } = Fixture::new()?;

            let source_file_name: gix_object::bstr::BString = format!("{}.txt", $case).into();

            let lines_blamed = gix_blame::file(
                &odb,
                suspect,
                None,
                &mut resource_cache,
                source_file_name.as_ref(),
                gix_blame::Options {
                    diff_algorithm: gix_diff::blob::Algorithm::Histogram,
                    ranges: BlameRanges::default(),
                    since: None,
                    rewrites: Some(gix_diff::Rewrites::default()),
                    debug_track_path: false,
                },
            )?
            .entries;

            assert_eq!(lines_blamed.len(), $number_of_lines);

            let git_dir = fixture_path()?.join(".git");
            let baseline = Baseline::collect(git_dir.join(format!("{}.baseline", $case)), source_file_name)?;

            assert_eq!(baseline.len(), $number_of_lines);
            pretty_assertions::assert_eq!(lines_blamed, baseline);
            Ok(())
        }
    };
}

mktest!(simple_case, "simple", 4);
mktest!(multiline_hunks, "multiline-hunks", 3);
mktest!(deleted_lines, "deleted-lines", 1);
mktest!(deleted_lines_multiple_hunks, "deleted-lines-multiple-hunks", 2);
mktest!(changed_lines, "changed-lines", 1);
mktest!(
    changed_line_between_unchanged_lines,
    "changed-line-between-unchanged-lines",
    3
);
mktest!(added_lines, "added-lines", 2);
mktest!(added_lines_around, "added-lines-around", 3);
mktest!(switched_lines, "switched-lines", 4);
mktest!(added_line_before_changed_line, "added-line-before-changed-line", 3);
mktest!(same_line_changed_twice, "same-line-changed-twice", 2);
mktest!(coalesce_adjacent_hunks, "coalesce-adjacent-hunks", 1);

mktest!(sub_directory, "sub-directory/sub-directory", 3);

mktest!(after_rename, "after-rename", 1);
mktest!(after_second_rename, "after-second-rename", 1);
mktest!(after_rewrite, "after-rewrite", 3);
mktest!(
    after_move_to_sub_directory,
    "sub-directory/after-move-to-sub-directory",
    1
);

mktest!(resolved_conflict, "resolved-conflict", 2);
mktest!(file_in_one_chain_of_ancestors, "file-in-one-chain-of-ancestors", 1);
mktest!(
    different_file_in_another_chain_of_ancestors,
    "different-file-in-another-chain-of-ancestors",
    1
);
mktest!(file_only_changed_in_branch, "file-only-changed-in-branch", 2);
mktest!(file_changed_in_two_branches, "file-changed-in-two-branches", 3);
mktest!(
    file_topo_order_different_than_date_order,
    "file-topo-order-different-than-date-order",
    3
);

/// As of 2024-09-24, these tests are expected to fail.
///
/// Context: https://github.com/Byron/gitoxide/pull/1453#issuecomment-2371013904
#[test]
#[should_panic = "empty-lines-myers"]
fn diff_disparity() {
    for case in ["empty-lines-myers", "empty-lines-histogram"] {
        let Fixture {
            odb,
            mut resource_cache,
            suspect,
        } = Fixture::new().unwrap();

        let source_file_name: gix_object::bstr::BString = format!("{case}.txt").into();

        let lines_blamed = gix_blame::file(
            &odb,
            suspect,
            None,
            &mut resource_cache,
            source_file_name.as_ref(),
            gix_blame::Options {
                diff_algorithm: gix_diff::blob::Algorithm::Histogram,
                ranges: BlameRanges::default(),
                since: None,
                rewrites: Some(gix_diff::Rewrites::default()),
                debug_track_path: false,
            },
        )
        .unwrap()
        .entries;

        assert_eq!(lines_blamed.len(), 5);

        let git_dir = fixture_path().unwrap().join(".git");
        let baseline = Baseline::collect(git_dir.join(format!("{case}.baseline")), source_file_name).unwrap();

        pretty_assertions::assert_eq!(lines_blamed, baseline, "{case}");
    }
}

#[test]
fn file_that_was_added_in_two_branches() -> gix_testtools::Result {
    let worktree_path = gix_testtools::scripted_fixture_read_only("make_blame_two_roots_repo.sh")?;

    let Fixture {
        odb,
        mut resource_cache,
        suspect,
    } = Fixture::for_worktree_path(worktree_path.to_path_buf())?;

    let source_file_name = "file-with-two-roots.txt";
    let lines_blamed = gix_blame::file(
        &odb,
        suspect,
        None,
        &mut resource_cache,
        source_file_name.into(),
        gix_blame::Options::default(),
    )?
    .entries;

    assert_eq!(lines_blamed.len(), 4);

    let git_dir = worktree_path.join(".git");
    let baseline = Baseline::collect(git_dir.join("file-with-two-roots.baseline"), source_file_name.into())?;

    pretty_assertions::assert_eq!(lines_blamed, baseline);

    Ok(())
}

#[test]
fn since() -> gix_testtools::Result {
    let Fixture {
        odb,
        mut resource_cache,
        suspect,
    } = Fixture::new()?;

    let source_file_name: gix_object::bstr::BString = "simple.txt".into();

    let lines_blamed = gix_blame::file(
        &odb,
        suspect,
        None,
        &mut resource_cache,
        source_file_name.as_ref(),
        gix_blame::Options {
            diff_algorithm: gix_diff::blob::Algorithm::Histogram,
            ranges: BlameRanges::default(),
            since: Some(gix_date::parse("2025-01-31", None)?),
            rewrites: Some(gix_diff::Rewrites::default()),
            debug_track_path: false,
        },
    )?
    .entries;

    assert_eq!(lines_blamed.len(), 1);

    let git_dir = fixture_path()?.join(".git");
    let baseline = Baseline::collect(git_dir.join("simple-since.baseline"), source_file_name)?;

    pretty_assertions::assert_eq!(lines_blamed, baseline);

    Ok(())
}

mod blame_ranges {
    use crate::{fixture_path, Baseline, Fixture};
    use gix_blame::BlameRanges;

    #[test]
    fn line_range() -> gix_testtools::Result {
        let Fixture {
            odb,
            mut resource_cache,
            suspect,
        } = Fixture::new()?;

        let source_file_name: gix_object::bstr::BString = "simple.txt".into();

        let lines_blamed = gix_blame::file(
            &odb,
            suspect,
            None,
            &mut resource_cache,
            source_file_name.as_ref(),
            gix_blame::Options {
                diff_algorithm: gix_diff::blob::Algorithm::Histogram,
                ranges: BlameRanges::from_one_based_inclusive_range(1..=2).unwrap(),
                since: None,
                rewrites: Some(gix_diff::Rewrites::default()),
                debug_track_path: false,
            },
        )?
        .entries;

        assert_eq!(lines_blamed.len(), 2);

        let git_dir = fixture_path()?.join(".git");
        let baseline = Baseline::collect(git_dir.join("simple-lines-1-2.baseline"), source_file_name)?;

        pretty_assertions::assert_eq!(lines_blamed, baseline);

        Ok(())
    }

    #[test]
    fn multiple_ranges_using_add_range() -> gix_testtools::Result {
        let Fixture {
            odb,
            mut resource_cache,
            suspect,
        } = Fixture::new()?;

        let ranges = BlameRanges::from_one_based_inclusive_ranges(vec![
            1..=2, // Lines 1-2
            1..=1, // Duplicate range, should be ignored
            4..=4, // Line 4
        ])
        .unwrap();

        let source_file_name: gix_object::bstr::BString = "simple.txt".into();

        let lines_blamed = gix_blame::file(
            &odb,
            suspect,
            None,
            &mut resource_cache,
            source_file_name.as_ref(),
            gix_blame::Options {
                diff_algorithm: gix_diff::blob::Algorithm::Histogram,
                ranges,
                since: None,
                rewrites: None,
                debug_track_path: false,
            },
        )?
        .entries;

        assert_eq!(lines_blamed.len(), 3); // Should have 3 lines total (2 from first range + 1 from second range)

        let git_dir = fixture_path()?.join(".git");
        let baseline = Baseline::collect(
            git_dir.join("simple-lines-multiple-1-2-and-4.baseline"),
            source_file_name,
        )?;

        pretty_assertions::assert_eq!(lines_blamed, baseline);

        Ok(())
    }

    #[test]
    fn multiple_ranges_using_from_ranges() -> gix_testtools::Result {
        let Fixture {
            odb,
            mut resource_cache,
            suspect,
        } = Fixture::new()?;

        let ranges = BlameRanges::from_one_based_inclusive_ranges(vec![1..=2, 1..=1, 4..=4]).unwrap();

        let source_file_name: gix_object::bstr::BString = "simple.txt".into();

        let lines_blamed = gix_blame::file(
            &odb,
            suspect,
            None,
            &mut resource_cache,
            source_file_name.as_ref(),
            gix_blame::Options {
                diff_algorithm: gix_diff::blob::Algorithm::Histogram,
                ranges,
                since: None,
                rewrites: None,
                debug_track_path: false,
            },
        )?
        .entries;

        assert_eq!(lines_blamed.len(), 3); // Should have 3 lines total (2 from first range + 1 from second range)

        let git_dir = fixture_path()?.join(".git");
        let baseline = Baseline::collect(
            git_dir.join("simple-lines-multiple-1-2-and-4.baseline"),
            source_file_name,
        )?;

        pretty_assertions::assert_eq!(lines_blamed, baseline);

        Ok(())
    }
}

mod rename_tracking {
    use gix_blame::BlameRanges;

    use crate::{Baseline, Fixture};

    #[test]
    fn source_file_name_is_tracked_per_hunk() -> gix_testtools::Result {
        let worktree_path = gix_testtools::scripted_fixture_read_only("make_blame_rename_tracking_repo.sh")?;

        let Fixture {
            odb,
            mut resource_cache,
            suspect,
        } = Fixture::for_worktree_path(worktree_path.to_path_buf())?;

        let source_file_name = "after-rename.txt";
        let lines_blamed = gix_blame::file(
            &odb,
            suspect,
            None,
            &mut resource_cache,
            source_file_name.into(),
            gix_blame::Options {
                diff_algorithm: gix_diff::blob::Algorithm::Histogram,
                ranges: BlameRanges::default(),
                since: None,
                rewrites: Some(gix_diff::Rewrites::default()),
                debug_track_path: false,
            },
        )?
        .entries;

        assert_eq!(lines_blamed.len(), 3);

        let git_dir = worktree_path.join(".git");
        let baseline = Baseline::collect(git_dir.join("after-rename.baseline"), source_file_name.into())?;

        pretty_assertions::assert_eq!(lines_blamed, baseline);

        Ok(())
    }
}

fn fixture_path() -> gix_testtools::Result<PathBuf> {
    gix_testtools::scripted_fixture_read_only("make_blame_repo.sh")
}
