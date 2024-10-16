use gix_blame::{blame_file, process_change, process_changes, BlameEntry, Change, Offset, UnblamedHunk};
use gix_hash::ObjectId;
use gix_object::bstr;
use std::path::PathBuf;

struct Baseline<'a> {
    lines: bstr::Lines<'a>,
}

mod baseline {
    use std::path::Path;

    use gix_hash::ObjectId;
    use gix_ref::bstr::ByteSlice;

    use super::Baseline;
    use gix_blame::BlameEntry;

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

    impl<'a> Baseline<'a> {
        pub fn collect(baseline_path: impl AsRef<Path>) -> std::io::Result<Vec<BlameEntry>> {
            let content = std::fs::read(baseline_path)?;

            Ok(Baseline { lines: content.lines() }.collect())
        }
    }

    impl<'a> Iterator for Baseline<'a> {
        type Item = BlameEntry;

        fn next(&mut self) -> Option<Self::Item> {
            let mut ranges = None;
            let mut commit_id = gix_hash::Kind::Sha1.null();
            let mut skip_lines: u32 = 0;

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

                    let line_number_in_original_file = fields[1].parse::<u32>().unwrap();
                    let line_number_in_final_file = fields[2].parse::<u32>().unwrap();
                    // The last field indicates the number of lines this group contains info for
                    // (this is not equal to the number of lines in git blame’s porcelain output).
                    let number_of_lines_in_group = fields[3].parse::<u32>().unwrap();

                    skip_lines = number_of_lines_in_group;

                    let original_range = (line_number_in_original_file - 1)
                        ..(line_number_in_original_file + number_of_lines_in_group - 1);
                    let blame_range =
                        (line_number_in_final_file - 1)..(line_number_in_final_file + number_of_lines_in_group - 1);
                    assert!(ranges.is_none(), "should not overwrite existing ranges");
                    ranges = Some((blame_range, original_range));
                } else if !is_known_header_field(&fields[0]) && ObjectId::from_hex(fields[0].as_bytes()).is_err() {
                    panic!("unexpected line: '{:?}'", line.as_bstr());
                }
            }

            let Some((range_in_blamed_file, range_in_original_file)) = ranges else {
                // No new lines were parsed, so we assume the iterator is finished.
                return None;
            };
            Some(BlameEntry::new(range_in_blamed_file, range_in_original_file, commit_id))
        }
    }
}

struct Fixture {
    worktree_path: PathBuf,
    odb: gix_odb::Handle,
    resource_cache: gix_diff::blob::Platform,
    suspect: ObjectId,
    commits: Vec<Result<gix_traverse::commit::Info, gix_traverse::commit::simple::Error>>,
}

impl Fixture {
    fn new() -> gix_testtools::Result<Fixture> {
        Self::for_worktree_path(fixture_path())
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

        // Needed for `peel_to_id_in_place`.
        use gix_ref::file::ReferenceExt;

        let head_id = reference.peel_to_id_in_place(&store, &odb)?;

        let commits: Vec<_> = gix_traverse::commit::Simple::new(Some(head_id), &odb)
            .sorting(gix_traverse::commit::simple::Sorting::ByCommitTime(
                gix_traverse::commit::simple::CommitTimeOrder::NewestFirst,
            ))?
            .collect();

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
            worktree_path,
            resource_cache,
            suspect: head_id,
            commits,
        })
    }
}

macro_rules! mktest {
    ($name:ident, $case:expr, $number_of_lines:literal) => {
        #[test]
        fn $name() {
            let Fixture {
                worktree_path,
                odb,
                mut resource_cache,
                suspect,
                commits,
            } = Fixture::new().unwrap();

            let lines_blamed = blame_file(
                &odb,
                commits,
                &mut resource_cache,
                suspect,
                worktree_path,
                format!("{}.txt", $case).as_str().into(),
            )
            .unwrap();

            assert_eq!(lines_blamed.len(), $number_of_lines);

            let git_dir = fixture_path().join(".git");
            let baseline = Baseline::collect(git_dir.join(format!("{}.baseline", $case))).unwrap();

            assert_eq!(baseline.len(), $number_of_lines);
            assert_eq!(lines_blamed, baseline);
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

mktest!(resolved_conflict, "resolved-conflict", 2);
mktest!(file_in_one_chain_of_ancestors, "file-in-one-chain-of-ancestors", 1);
mktest!(
    different_file_in_another_chain_of_ancestors,
    "different-file-in-another-chain-of-ancestors",
    1
);

#[test]
#[ignore = "TBD: figure out what the problem is"]
// As of 2024-09-24, these tests are expected to fail.
//
// Context: https://github.com/Byron/gitoxide/pull/1453#issuecomment-2371013904
fn diff_disparity() {
    for case in ["empty-lines-myers", "empty-lines-histogram"] {
        let Fixture {
            worktree_path,
            odb,
            mut resource_cache,
            suspect,
            commits,
        } = Fixture::new().unwrap();

        let lines_blamed = blame_file(
            &odb,
            commits,
            &mut resource_cache,
            suspect,
            worktree_path,
            format!("{case}.txt").as_str().into(),
        )
        .unwrap();

        assert_eq!(lines_blamed.len(), 5);

        let git_dir = fixture_path().join(".git");
        let baseline = Baseline::collect(git_dir.join(format!("{case}.baseline"))).unwrap();

        assert_eq!(lines_blamed, baseline, "{case}");
    }
}

#[test]
fn process_change_works() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(0);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        None,
        None,
    );

    assert_eq!(hunk, None);
    assert_eq!(change, None);
    assert_eq!(offset_in_destination, Offset::Added(0));
}

#[test]
fn process_change_works_added_hunk() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(0);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        Some(UnblamedHunk::new(0..5, suspect, Offset::Added(0))),
        Some(Change::Added(0..3, 0)),
    );

    assert_eq!(
        hunk,
        Some(UnblamedHunk {
            range_in_blamed_file: 3..5,
            suspects: [(suspect, 3..5)].into()
        })
    );
    assert_eq!(change, None);
    assert_eq!(
        lines_blamed,
        [BlameEntry {
            range_in_blamed_file: 0..3,
            range_in_original_file: 0..3,
            commit_id: suspect
        }]
    );
    assert_eq!(new_hunks_to_blame, []);
    assert_eq!(offset_in_destination, Offset::Added(3));
}

#[test]
fn process_change_works_added_hunk_2() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(0);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        Some(UnblamedHunk::new(0..5, suspect, Offset::Added(0))),
        Some(Change::Added(2..3, 0)),
    );

    assert_eq!(
        hunk,
        Some(UnblamedHunk {
            range_in_blamed_file: 3..5,
            suspects: [(suspect, 3..5)].into()
        })
    );
    assert_eq!(change, None);
    assert_eq!(
        lines_blamed,
        [BlameEntry {
            range_in_blamed_file: 2..3,
            range_in_original_file: 2..3,
            commit_id: suspect
        }]
    );
    assert_eq!(
        new_hunks_to_blame,
        [UnblamedHunk {
            range_in_blamed_file: 0..2,
            suspects: [(suspect, 0..2)].into()
        }]
    );
    assert_eq!(offset_in_destination, Offset::Added(1));
}

#[test]
fn process_change_works_added_hunk_3() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(5);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        Some(UnblamedHunk::new(10..15, suspect, Offset::Added(0))),
        Some(Change::Added(12..13, 0)),
    );

    assert_eq!(
        hunk,
        Some(UnblamedHunk {
            range_in_blamed_file: 13..15,
            suspects: [(suspect, 13..15)].into()
        })
    );
    assert_eq!(change, None);
    assert_eq!(
        lines_blamed,
        [BlameEntry {
            range_in_blamed_file: 12..13,
            range_in_original_file: 12..13,
            commit_id: suspect
        }]
    );
    assert_eq!(
        new_hunks_to_blame,
        [UnblamedHunk {
            range_in_blamed_file: 10..12,
            suspects: [(suspect, 5..7)].into()
        }]
    );
    assert_eq!(offset_in_destination, Offset::Added(6));
}

#[test]
fn process_change_works_added_hunk_4() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(0);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        // range_in_destination: 7..12
        Some(UnblamedHunk::new(12..17, suspect, Offset::Added(5))),
        Some(Change::Added(9..10, 0)),
    );

    assert_eq!(
        hunk,
        Some(UnblamedHunk {
            range_in_blamed_file: 15..17,
            suspects: [(suspect, 10..12)].into()
        })
    );
    assert_eq!(change, None);
    assert_eq!(
        lines_blamed,
        [BlameEntry {
            range_in_blamed_file: 14..15,
            range_in_original_file: 9..10,
            commit_id: suspect
        }]
    );
    assert_eq!(
        new_hunks_to_blame,
        [UnblamedHunk {
            range_in_blamed_file: 12..14,
            suspects: [(suspect, 7..9)].into()
        }]
    );
    assert_eq!(offset_in_destination, Offset::Added(1));
}

#[test]
fn process_change_works_added_hunk_5() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(0);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        Some(UnblamedHunk::new(0..5, suspect, Offset::Added(0))),
        Some(Change::Added(0..3, 1)),
    );

    assert_eq!(
        hunk,
        Some(UnblamedHunk {
            range_in_blamed_file: 3..5,
            suspects: [(suspect, 3..5)].into()
        })
    );
    assert_eq!(change, None);
    assert_eq!(
        lines_blamed,
        [BlameEntry {
            range_in_blamed_file: 0..3,
            range_in_original_file: 0..3,
            commit_id: suspect
        }]
    );
    assert_eq!(new_hunks_to_blame, []);
    assert_eq!(offset_in_destination, Offset::Added(2));
}

#[test]
fn process_change_works_added_hunk_6() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(0);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        // range_in_destination: 0..4
        Some(UnblamedHunk::new(1..5, suspect, Offset::Added(1))),
        Some(Change::Added(0..3, 1)),
    );

    assert_eq!(
        hunk,
        Some(UnblamedHunk {
            range_in_blamed_file: 4..5,
            suspects: [(suspect, 3..4)].into()
        })
    );
    assert_eq!(change, None);
    assert_eq!(
        lines_blamed,
        [BlameEntry {
            range_in_blamed_file: 1..4,
            range_in_original_file: 0..3,
            commit_id: suspect
        }]
    );
    assert_eq!(new_hunks_to_blame, []);
    assert_eq!(offset_in_destination, Offset::Added(2));
}

#[test]
fn process_change_works_added_hunk_7() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(2);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        // range_in_destination: 2..6
        Some(UnblamedHunk::new(3..7, suspect, Offset::Added(1))),
        Some(Change::Added(3..5, 1)),
    );

    assert_eq!(
        hunk,
        Some(UnblamedHunk {
            range_in_blamed_file: 6..7,
            suspects: [(suspect, 5..6)].into()
        })
    );
    assert_eq!(change, None);
    assert_eq!(
        lines_blamed,
        [BlameEntry {
            range_in_blamed_file: 4..6,
            range_in_original_file: 3..5,
            commit_id: suspect
        }]
    );
    assert_eq!(
        new_hunks_to_blame,
        [UnblamedHunk {
            range_in_blamed_file: 3..4,
            suspects: [(suspect, 0..1)].into()
        }]
    );
    assert_eq!(offset_in_destination, Offset::Added(3));
}

#[test]
fn process_change_works_added_hunk_8() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(1);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        // range_in_destination: 25..26
        Some(UnblamedHunk::new(23..24, suspect, Offset::Deleted(2))),
        Some(Change::Added(25..27, 1)),
    );

    assert_eq!(hunk, None);
    assert_eq!(change, Some(Change::Added(25..27, 1)));
    assert_eq!(
        lines_blamed,
        [BlameEntry {
            range_in_blamed_file: 23..24,
            range_in_original_file: 25..26,
            commit_id: suspect
        }]
    );
    assert_eq!(new_hunks_to_blame, []);
    assert_eq!(offset_in_destination, Offset::Added(1));
}

#[test]
fn process_change_works_added_hunk_9() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(0);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        // range_in_destination: 21..22
        Some(UnblamedHunk::new(23..24, suspect, Offset::Added(2))),
        Some(Change::Added(18..22, 3)),
    );

    assert_eq!(hunk, None);
    assert_eq!(change, None);
    assert_eq!(
        lines_blamed,
        [BlameEntry {
            range_in_blamed_file: 23..24,
            range_in_original_file: 21..22,
            commit_id: suspect
        }]
    );
    assert_eq!(new_hunks_to_blame, []);
    assert_eq!(offset_in_destination, Offset::Added(1));
}

#[test]
fn process_change_works_added_hunk_10() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(0);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        // range_in_destination: 70..108
        Some(UnblamedHunk::new(71..109, suspect, Offset::Added(1))),
        Some(Change::Added(106..109, 0)),
    );

    assert_eq!(hunk, None);
    assert_eq!(change, Some(Change::Added(106..109, 0)));
    assert_eq!(
        lines_blamed,
        [BlameEntry {
            range_in_blamed_file: 107..109,
            range_in_original_file: 106..108,
            commit_id: suspect
        }]
    );
    assert_eq!(
        new_hunks_to_blame,
        [UnblamedHunk {
            range_in_blamed_file: 71..107,
            suspects: [(suspect, 70..106)].into()
        }]
    );
    assert_eq!(offset_in_destination, Offset::Added(0));
}

#[test]
fn process_change_works_added_hunk_11() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(0);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        // range_in_destination: 137..144
        Some(UnblamedHunk::new(149..156, suspect, Offset::Added(12))),
        Some(Change::Added(143..146, 0)),
    );

    assert_eq!(hunk, None);
    assert_eq!(change, Some(Change::Added(143..146, 0)));
    assert_eq!(
        lines_blamed,
        [BlameEntry {
            range_in_blamed_file: 155..156,
            range_in_original_file: 143..144,
            commit_id: suspect
        }]
    );
    assert_eq!(
        new_hunks_to_blame,
        [UnblamedHunk {
            range_in_blamed_file: 149..155,
            suspects: [(suspect, 137..143)].into()
        }]
    );
    assert_eq!(offset_in_destination, Offset::Added(0));
}

#[test]
fn process_change_works_no_overlap() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Deleted(3);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        // range_in_destination: 2..5
        Some(UnblamedHunk::new(3..6, suspect, Offset::Added(1))),
        Some(Change::Added(7..10, 1)),
    );

    assert_eq!(hunk, None);
    assert_eq!(change, Some(Change::Added(7..10, 1)));
    assert_eq!(lines_blamed, []);
    assert_eq!(
        new_hunks_to_blame,
        [UnblamedHunk {
            range_in_blamed_file: 3..6,
            suspects: [(suspect, 5..8)].into()
        }]
    );
    assert_eq!(offset_in_destination, Offset::Deleted(3));
}

#[test]
fn process_change_works_no_overlap_2() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(0);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        // range_in_destination: 6..8
        Some(UnblamedHunk::new(9..11, suspect, Offset::Added(3))),
        Some(Change::Added(2..5, 0)),
    );

    assert_eq!(
        hunk,
        Some(UnblamedHunk {
            range_in_blamed_file: 9..11,
            suspects: [(suspect, 6..8)].into()
        })
    );
    assert_eq!(change, None);
    assert_eq!(lines_blamed, []);
    assert_eq!(new_hunks_to_blame, []);
    assert_eq!(offset_in_destination, Offset::Added(3));
}

#[test]
fn process_change_works_no_overlap_3() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(0);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        // range_in_destination: 5..15
        Some(UnblamedHunk::new(4..15, suspect, Offset::Deleted(1))),
        Some(Change::Added(4..5, 1)),
    );

    assert_eq!(
        hunk,
        Some(UnblamedHunk {
            range_in_blamed_file: 4..15,
            suspects: [(suspect, 5..16)].into()
        })
    );
    assert_eq!(change, None);
    assert_eq!(lines_blamed, []);
    assert_eq!(new_hunks_to_blame, []);
    assert_eq!(offset_in_destination, Offset::Added(0));
}

#[test]
fn process_change_works_no_overlap_4() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(1);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        // range_in_destination: 25..27
        Some(UnblamedHunk::new(23..25, suspect, Offset::Deleted(2))),
        Some(Change::Unchanged(21..22)),
    );

    assert_eq!(
        hunk,
        Some(UnblamedHunk {
            range_in_blamed_file: 23..25,
            suspects: [(suspect, 25..27)].into()
        })
    );
    assert_eq!(change, None);
    assert_eq!(lines_blamed, []);
    assert_eq!(new_hunks_to_blame, []);
    assert_eq!(offset_in_destination, Offset::Added(1));
}

#[test]
fn process_change_works_no_overlap_5() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(1);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        // range_in_destination: 17..18
        Some(UnblamedHunk::new(15..16, suspect, Offset::Deleted(2))),
        Some(Change::Deleted(20, 1)),
    );

    assert_eq!(hunk, None);
    assert_eq!(change, Some(Change::Deleted(20, 1)));
    assert_eq!(lines_blamed, []);
    assert_eq!(
        new_hunks_to_blame,
        [UnblamedHunk {
            range_in_blamed_file: 15..16,
            suspects: [(suspect, 16..17)].into()
        }]
    );
    assert_eq!(offset_in_destination, Offset::Added(1));
}

#[test]
fn process_change_works_no_overlap_6() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(0);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        // range_in_destination: 22..24
        Some(UnblamedHunk::new(23..25, suspect, Offset::Added(1))),
        Some(Change::Deleted(20, 1)),
    );

    assert_eq!(
        hunk,
        Some(UnblamedHunk {
            range_in_blamed_file: 23..25,
            suspects: [(suspect, 22..24)].into()
        })
    );
    assert_eq!(change, None);
    assert_eq!(lines_blamed, []);
    assert_eq!(new_hunks_to_blame, []);
    assert_eq!(offset_in_destination, Offset::Deleted(1));
}

#[test]
fn process_change_works_enclosing_addition() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(3);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        // range_in_destination: 5..8
        Some(UnblamedHunk::new(2..5, suspect, Offset::Deleted(3))),
        Some(Change::Added(3..12, 2)),
    );

    assert_eq!(hunk, None);
    assert_eq!(change, Some(Change::Added(3..12, 2)));
    assert_eq!(
        lines_blamed,
        [BlameEntry {
            range_in_blamed_file: 2..5,
            range_in_original_file: 5..8,
            commit_id: suspect
        }]
    );
    assert_eq!(new_hunks_to_blame, []);
    assert_eq!(offset_in_destination, Offset::Added(3));
}

#[test]
fn process_change_works_enclosing_deletion() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(3);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        // range_in_destination: 13..20
        Some(UnblamedHunk::new(12..19, suspect, Offset::Deleted(1))),
        Some(Change::Deleted(15, 2)),
    );

    assert_eq!(
        hunk,
        Some(UnblamedHunk {
            range_in_blamed_file: 14..19,
            suspects: [(suspect, 15..20)].into()
        })
    );
    assert_eq!(change, None);
    assert_eq!(lines_blamed, []);
    assert_eq!(
        new_hunks_to_blame,
        [UnblamedHunk {
            range_in_blamed_file: 12..14,
            suspects: [(suspect, 10..12)].into()
        }]
    );
    assert_eq!(offset_in_destination, Offset::Added(1));
}

#[test]
fn process_change_works_enclosing_unchanged_lines() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(3);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        // range_in_destination: 109..113
        Some(UnblamedHunk::new(110..114, suspect, Offset::Added(1))),
        Some(Change::Unchanged(109..172)),
    );

    assert_eq!(hunk, None);
    assert_eq!(change, Some(Change::Unchanged(109..172)));
    assert_eq!(lines_blamed, []);
    assert_eq!(
        new_hunks_to_blame,
        [UnblamedHunk {
            range_in_blamed_file: 110..114,
            suspects: [(suspect, 106..110)].into()
        }]
    );
    assert_eq!(offset_in_destination, Offset::Added(3));
}

#[test]
fn process_change_works_unchanged_hunk() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(0);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        Some(UnblamedHunk::new(0..5, suspect, Offset::Added(0))),
        Some(Change::Unchanged(0..3)),
    );

    assert_eq!(
        hunk,
        Some(UnblamedHunk {
            range_in_blamed_file: 0..5,
            suspects: [(suspect, 0..5)].into()
        })
    );
    assert_eq!(change, None);
    assert_eq!(lines_blamed, []);
    assert_eq!(new_hunks_to_blame, []);
    assert_eq!(offset_in_destination, Offset::Added(0));
}

#[test]
fn process_change_works_unchanged_hunk_2() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(0);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        Some(UnblamedHunk::new(0..5, suspect, Offset::Added(0))),
        Some(Change::Unchanged(0..7)),
    );

    assert_eq!(hunk, None);
    assert_eq!(change, Some(Change::Unchanged(0..7)));
    assert_eq!(lines_blamed, []);
    assert_eq!(
        new_hunks_to_blame,
        [UnblamedHunk {
            range_in_blamed_file: 0..5,
            suspects: [(suspect, 0..5)].into()
        }]
    );
    assert_eq!(offset_in_destination, Offset::Added(0));
}

#[test]
fn process_change_works_unchanged_hunk_3() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Deleted(2);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        Some(UnblamedHunk {
            range_in_blamed_file: 22..30,
            suspects: [(suspect, 21..29)].into(),
        }),
        Some(Change::Unchanged(21..23)),
    );

    assert_eq!(
        hunk,
        Some(UnblamedHunk {
            range_in_blamed_file: 22..30,
            suspects: [(suspect, 21..29)].into()
        })
    );
    assert_eq!(change, None);
    assert_eq!(lines_blamed, []);
    assert_eq!(new_hunks_to_blame, []);
    assert_eq!(offset_in_destination, Offset::Deleted(2));
}

#[test]
fn process_change_works_deleted_hunk() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(0);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        Some(UnblamedHunk::new(0..5, suspect, Offset::Added(0))),
        Some(Change::Deleted(5, 3)),
    );

    assert_eq!(hunk, None);
    assert_eq!(change, Some(Change::Deleted(5, 3)));
    assert_eq!(lines_blamed, []);
    assert_eq!(
        new_hunks_to_blame,
        [UnblamedHunk {
            range_in_blamed_file: 0..5,
            suspects: [(suspect, 0..5)].into()
        }]
    );
    assert_eq!(offset_in_destination, Offset::Added(0));
}

#[test]
fn process_change_works_deleted_hunk_2() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(0);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        Some(UnblamedHunk::new(2..16, suspect, Offset::Added(0))),
        Some(Change::Deleted(0, 4)),
    );

    assert_eq!(
        hunk,
        Some(UnblamedHunk {
            range_in_blamed_file: 2..16,
            suspects: [(suspect, 2..16)].into()
        })
    );
    assert_eq!(change, None);
    assert_eq!(lines_blamed, []);
    assert_eq!(new_hunks_to_blame, []);
    assert_eq!(offset_in_destination, Offset::Deleted(4));
}

#[test]
fn process_change_works_deleted_hunk_3() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(0);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        Some(UnblamedHunk::new(2..16, suspect, Offset::Added(0))),
        Some(Change::Deleted(14, 4)),
    );

    assert_eq!(
        hunk,
        Some(UnblamedHunk {
            range_in_blamed_file: 14..16,
            suspects: [(suspect, 14..16)].into()
        })
    );
    assert_eq!(change, None);
    assert_eq!(lines_blamed, []);
    assert_eq!(
        new_hunks_to_blame,
        [UnblamedHunk::new(2..14, suspect, Offset::Added(0))]
    );
    assert_eq!(offset_in_destination, Offset::Deleted(4));
}

#[test]
fn process_change_works_addition_only() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(1);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        None,
        Some(Change::Added(22..25, 1)),
    );

    assert_eq!(hunk, None);
    assert_eq!(change, None);
    assert_eq!(lines_blamed, []);
    assert_eq!(new_hunks_to_blame, []);
    assert_eq!(offset_in_destination, Offset::Added(3));
}

#[test]
fn process_change_works_deletion_only() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(1);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        None,
        Some(Change::Deleted(11, 5)),
    );

    assert_eq!(hunk, None);
    assert_eq!(change, None);
    assert_eq!(lines_blamed, []);
    assert_eq!(new_hunks_to_blame, []);
    assert_eq!(offset_in_destination, Offset::Deleted(4));
}

#[test]
fn process_change_works_unchanged_only() {
    let mut lines_blamed = Vec::new();
    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination: Offset = Offset::Added(1);
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);

    let (hunk, change) = process_change(
        &mut lines_blamed,
        &mut new_hunks_to_blame,
        &mut offset_in_destination,
        suspect,
        None,
        Some(Change::Unchanged(11..13)),
    );

    assert_eq!(hunk, None);
    assert_eq!(change, None);
    assert_eq!(lines_blamed, []);
    assert_eq!(new_hunks_to_blame, []);
    assert_eq!(offset_in_destination, Offset::Added(1));
}
#[test]
fn process_changes_works() {
    let mut lines_blamed = Vec::new();
    let hunks_to_blame = &[];
    let changes = &[];
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);
    let new_hunks_to_blame = process_changes(&mut lines_blamed, hunks_to_blame, changes, suspect);

    assert_eq!(lines_blamed, []);
    assert_eq!(new_hunks_to_blame, []);
}

#[test]
fn process_changes_works_added_hunk() {
    let mut lines_blamed = Vec::new();
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);
    let hunks_to_blame = &[UnblamedHunk::new(0..4, suspect, Offset::Added(0))];
    let changes = &[Change::Added(0..4, 0)];
    let new_hunks_to_blame = process_changes(&mut lines_blamed, hunks_to_blame, changes, suspect);

    assert_eq!(
        lines_blamed,
        [BlameEntry {
            range_in_blamed_file: 0..4,
            range_in_original_file: 0..4,
            commit_id: suspect
        }]
    );
    assert_eq!(new_hunks_to_blame, []);
}

#[test]
fn process_changes_works_added_hunk_2() {
    let mut lines_blamed = Vec::new();
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);
    let hunks_to_blame = &[UnblamedHunk::new(0..6, suspect, Offset::Added(0))];
    let changes = &[Change::Added(0..4, 0), Change::Unchanged(4..6)];
    let new_hunks_to_blame = process_changes(&mut lines_blamed, hunks_to_blame, changes, suspect);

    assert_eq!(
        lines_blamed,
        [BlameEntry {
            range_in_blamed_file: 0..4,
            range_in_original_file: 0..4,
            commit_id: suspect
        }]
    );
    assert_eq!(new_hunks_to_blame, [UnblamedHunk::new(4..6, suspect, Offset::Added(4))]);
}

#[test]
fn process_changes_works_added_hunk_3() {
    let mut lines_blamed = Vec::new();
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);
    let hunks_to_blame = &[UnblamedHunk::new(0..6, suspect, Offset::Added(0))];
    let changes = &[Change::Unchanged(0..2), Change::Added(2..4, 0), Change::Unchanged(4..6)];
    let new_hunks_to_blame = process_changes(&mut lines_blamed, hunks_to_blame, changes, suspect);

    assert_eq!(
        lines_blamed,
        [BlameEntry {
            range_in_blamed_file: 2..4,
            range_in_original_file: 2..4,
            commit_id: suspect
        }]
    );
    assert_eq!(
        new_hunks_to_blame,
        [
            UnblamedHunk::new(0..2, suspect, Offset::Added(0)),
            UnblamedHunk::new(4..6, suspect, Offset::Added(2))
        ]
    );
}

#[test]
fn process_changes_works_added_hunk_4_0() {
    let mut lines_blamed = Vec::new();
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);
    let hunks_to_blame = &[UnblamedHunk::new(0..6, suspect, Offset::Added(0))];
    let changes = &[Change::Added(0..1, 0), Change::Added(1..4, 0), Change::Unchanged(4..6)];
    let new_hunks_to_blame = process_changes(&mut lines_blamed, hunks_to_blame, changes, suspect);

    assert_eq!(
        lines_blamed,
        [
            BlameEntry {
                range_in_blamed_file: 0..1,
                range_in_original_file: 0..1,
                commit_id: suspect
            },
            BlameEntry {
                range_in_blamed_file: 1..4,
                range_in_original_file: 1..4,
                commit_id: suspect
            }
        ]
    );
    assert_eq!(new_hunks_to_blame, [UnblamedHunk::new(4..6, suspect, Offset::Added(4))]);
}

#[test]
fn process_changes_works_added_hunk_4_1() {
    let mut lines_blamed = Vec::new();
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);
    let hunks_to_blame = &[UnblamedHunk::new(0..6, suspect, Offset::Added(0))];
    let changes = &[Change::Added(0..1, 0)];
    let new_hunks_to_blame = process_changes(&mut lines_blamed, hunks_to_blame, changes, suspect);

    assert_eq!(
        lines_blamed,
        [BlameEntry {
            range_in_blamed_file: 0..1,
            range_in_original_file: 0..1,
            commit_id: suspect
        }]
    );
    assert_eq!(new_hunks_to_blame, [UnblamedHunk::new(1..6, suspect, Offset::Added(1))]);
}

#[test]
fn process_changes_works_added_hunk_4_2() {
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);
    let suspect_2 = ObjectId::from_hex(b"2222222222222222222222222222222222222222").unwrap();
    let mut lines_blamed: Vec<BlameEntry> = vec![BlameEntry {
        range_in_blamed_file: 0..2,
        range_in_original_file: 0..2,
        commit_id: suspect,
    }];
    let hunks_to_blame = &[UnblamedHunk::new(2..6, suspect_2, Offset::Added(2))];
    let changes = &[Change::Added(0..1, 0)];
    let new_hunks_to_blame = process_changes(&mut lines_blamed, hunks_to_blame, changes, suspect_2);

    assert_eq!(
        lines_blamed,
        [
            BlameEntry {
                range_in_blamed_file: 0..2,
                range_in_original_file: 0..2,
                commit_id: suspect
            },
            BlameEntry {
                range_in_blamed_file: 2..3,
                range_in_original_file: 0..1,
                commit_id: suspect_2
            }
        ]
    );
    assert_eq!(
        new_hunks_to_blame,
        [UnblamedHunk::new(3..6, suspect_2, Offset::Added(3))]
    );
}

#[test]
fn process_changes_works_added_hunk_5() {
    let mut lines_blamed = Vec::new();
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);
    let hunks_to_blame = &[UnblamedHunk::new(0..6, suspect, Offset::Added(0))];
    let changes = &[Change::Added(0..4, 3), Change::Unchanged(4..6)];
    let new_hunks_to_blame = process_changes(&mut lines_blamed, hunks_to_blame, changes, suspect);

    assert_eq!(
        lines_blamed,
        [BlameEntry {
            range_in_blamed_file: 0..4,
            range_in_original_file: 0..4,
            commit_id: suspect
        }]
    );
    assert_eq!(new_hunks_to_blame, [UnblamedHunk::new(4..6, suspect, Offset::Added(1))]);
}

#[test]
fn process_changes_works_added_hunk_6() {
    let mut lines_blamed = Vec::new();
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);
    let hunks_to_blame = &[UnblamedHunk::new(4..6, suspect, Offset::Added(1))];
    let changes = &[Change::Added(0..3, 0), Change::Unchanged(3..5)];
    let new_hunks_to_blame = process_changes(&mut lines_blamed, hunks_to_blame, changes, suspect);

    assert_eq!(lines_blamed, []);
    assert_eq!(new_hunks_to_blame, [UnblamedHunk::new(4..6, suspect, Offset::Added(4))]);
}

#[test]
fn process_changes_works_added_hunk_7() {
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);
    let suspect_2 = ObjectId::from_hex(b"2222222222222222222222222222222222222222").unwrap();
    let mut lines_blamed: Vec<BlameEntry> = vec![BlameEntry {
        range_in_blamed_file: 0..1,
        range_in_original_file: 0..1,
        commit_id: suspect,
    }];
    let hunks_to_blame = &[UnblamedHunk::new(1..3, suspect_2, Offset::Added(1))];
    let changes = &[Change::Added(0..1, 2)];
    let new_hunks_to_blame = process_changes(&mut lines_blamed, hunks_to_blame, changes, suspect_2);

    assert_eq!(
        lines_blamed,
        [
            BlameEntry {
                range_in_blamed_file: 0..1,
                range_in_original_file: 0..1,
                commit_id: suspect
            },
            BlameEntry {
                range_in_blamed_file: 1..2,
                range_in_original_file: 0..1,
                commit_id: suspect_2
            }
        ]
    );
    assert_eq!(
        new_hunks_to_blame,
        [UnblamedHunk::new(2..3, suspect_2, Offset::Added(0))]
    );
}

#[test]
fn process_changes_works_added_hunk_8() {
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);
    let mut lines_blamed = Vec::new();
    let hunks_to_blame = &[UnblamedHunk::new(0..4, suspect, Offset::Added(0))];
    let changes = &[Change::Added(0..2, 0), Change::Unchanged(2..3), Change::Added(3..4, 0)];
    let new_hunks_to_blame = process_changes(&mut lines_blamed, hunks_to_blame, changes, suspect);

    assert_eq!(
        lines_blamed,
        [
            BlameEntry {
                range_in_blamed_file: 0..2,
                range_in_original_file: 0..2,
                commit_id: suspect
            },
            BlameEntry {
                range_in_blamed_file: 3..4,
                range_in_original_file: 3..4,
                commit_id: suspect
            }
        ]
    );
    assert_eq!(new_hunks_to_blame, [UnblamedHunk::new(2..3, suspect, Offset::Added(2))]);
}

#[test]
fn process_changes_works_added_hunk_9() {
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);
    let mut lines_blamed: Vec<BlameEntry> = vec![BlameEntry {
        range_in_blamed_file: 30..31,
        range_in_original_file: 30..31,
        commit_id: suspect,
    }];
    let hunks_to_blame = &[
        UnblamedHunk {
            range_in_blamed_file: 0..30,
            suspects: [(suspect, 0..30)].into(),
        },
        UnblamedHunk {
            range_in_blamed_file: 31..37,
            suspects: [(suspect, 31..37)].into(),
        },
    ];
    let changes = &[
        Change::Unchanged(0..16),
        Change::Added(16..17, 0),
        Change::Unchanged(17..37),
    ];
    let new_hunks_to_blame = process_changes(&mut lines_blamed, hunks_to_blame, changes, suspect);

    lines_blamed.sort_by(|a, b| a.range_in_blamed_file.start.cmp(&b.range_in_blamed_file.start));

    assert_eq!(
        lines_blamed,
        [
            BlameEntry {
                range_in_blamed_file: 16..17,
                range_in_original_file: 16..17,
                commit_id: suspect
            },
            BlameEntry {
                range_in_blamed_file: 30..31,
                range_in_original_file: 30..31,
                commit_id: suspect
            }
        ]
    );
    assert_eq!(
        new_hunks_to_blame,
        [
            UnblamedHunk {
                range_in_blamed_file: 0..16,
                suspects: [(suspect, 0..16)].into()
            },
            UnblamedHunk {
                range_in_blamed_file: 17..30,
                suspects: [(suspect, 16..29)].into()
            },
            UnblamedHunk {
                range_in_blamed_file: 31..37,
                suspects: [(suspect, 30..36)].into()
            }
        ]
    );
}

#[test]
fn process_changes_works_deleted_hunk() {
    let mut lines_blamed = Vec::new();
    let suspect = ObjectId::null(gix_hash::Kind::Sha1);
    let hunks_to_blame = &[
        UnblamedHunk::new(0..4, suspect, Offset::Added(0)),
        UnblamedHunk::new(4..7, suspect, Offset::Added(0)),
    ];
    let changes = &[Change::Deleted(0, 3), Change::Added(0..4, 0)];
    let new_hunks_to_blame = process_changes(&mut lines_blamed, hunks_to_blame, changes, suspect);

    assert_eq!(
        lines_blamed,
        [BlameEntry {
            range_in_blamed_file: 0..4,
            range_in_original_file: 0..4,
            commit_id: suspect
        }]
    );
    assert_eq!(
        new_hunks_to_blame,
        [UnblamedHunk {
            range_in_blamed_file: 4..7,
            suspects: [(suspect, 3..6)].into()
        }]
    );
}

fn fixture_path() -> PathBuf {
    gix_testtools::scripted_fixture_read_only("make_blame_repo.sh").unwrap()
}
