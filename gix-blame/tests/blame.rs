use std::path::PathBuf;

use gix_hash::ObjectId;
use gix_object::bstr;

struct Baseline<'a> {
    lines: bstr::Lines<'a>,
}

mod baseline {
    use std::path::Path;

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
        pub fn collect(baseline_path: impl AsRef<Path>) -> std::io::Result<Vec<BlameEntry>> {
            let content = std::fs::read(baseline_path)?;

            Ok(Baseline { lines: content.lines() }.collect())
        }
    }

    impl Iterator for Baseline<'_> {
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
                } else if !is_known_header_field(&fields[0]) && ObjectId::from_hex(fields[0].as_bytes()).is_err() {
                    panic!("unexpected line: '{:?}'", line.as_bstr());
                }
            }

            let Some((range_in_blamed_file, range_in_source_file)) = ranges else {
                // No new lines were parsed, so we assume the iterator is finished.
                return None;
            };
            Some(BlameEntry::new(range_in_blamed_file, range_in_source_file, commit_id))
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
        fn $name() -> gix_testtools::Result<()> {
            let Fixture {
                odb,
                mut resource_cache,
                suspect,
            } = Fixture::new()?;

            let lines_blamed = gix_blame::file(
                &odb,
                suspect,
                None,
                &mut resource_cache,
                format!("{}.txt", $case).as_str().into(),
                gix_blame::Options {
                    range: None,
                    since: None,
                },
            )?
            .entries;

            assert_eq!(lines_blamed.len(), $number_of_lines);

            let git_dir = fixture_path().join(".git");
            let baseline = Baseline::collect(git_dir.join(format!("{}.baseline", $case)))?;

            assert_eq!(baseline.len(), $number_of_lines);
            assert_eq!(lines_blamed, baseline);
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

        let lines_blamed = gix_blame::file(
            &odb,
            suspect,
            None,
            &mut resource_cache,
            format!("{case}.txt").as_str().into(),
            gix_blame::Options {
                range: None,
                since: None,
            },
        )
        .unwrap()
        .entries;

        assert_eq!(lines_blamed.len(), 5);

        let git_dir = fixture_path().join(".git");
        let baseline = Baseline::collect(git_dir.join(format!("{case}.baseline"))).unwrap();

        assert_eq!(lines_blamed, baseline, "{case}");
    }
}

#[test]
fn line_range() {
    let Fixture {
        odb,
        mut resource_cache,
        suspect,
    } = Fixture::new().unwrap();

    let lines_blamed = gix_blame::file(
        &odb,
        suspect,
        None,
        &mut resource_cache,
        "simple.txt".into(),
        gix_blame::Options {
            range: Some(1..2),
            since: None,
        },
    )
    .unwrap()
    .entries;

    assert_eq!(lines_blamed.len(), 2);

    let git_dir = fixture_path().join(".git");
    let baseline = Baseline::collect(git_dir.join("simple-lines-1-2.baseline")).unwrap();

    assert_eq!(lines_blamed, baseline);
}

#[test]
fn since() {
    let Fixture {
        odb,
        mut resource_cache,
        suspect,
    } = Fixture::new().unwrap();

    let lines_blamed = gix_blame::file(
        &odb,
        suspect,
        None,
        &mut resource_cache,
        "simple.txt".into(),
        gix_blame::Options {
            range: None,
            since: Some(gix_date::parse("2025-01-31", None).unwrap()),
        },
    )
    .unwrap()
    .entries;

    assert_eq!(lines_blamed.len(), 1);

    let git_dir = fixture_path().join(".git");
    let baseline = Baseline::collect(git_dir.join("simple-since.baseline")).unwrap();

    assert_eq!(lines_blamed, baseline);
}

fn fixture_path() -> PathBuf {
    gix_testtools::scripted_fixture_read_only("make_blame_repo.sh").unwrap()
}
