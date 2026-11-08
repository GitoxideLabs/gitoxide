use std::{ops::ControlFlow, path::Path};

use bstr::{BString, ByteSlice};
use gix_diff::{
    Rewrites,
    rewrites::{Copies, CopySource, Tracker, tracker::visit::SourceKind},
};
use gix_error::TestResult;
use gix_object::tree::{EntryKind, EntryMode};

use super::{Change, ChangeKind};

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
struct Record {
    status: BString,
    source: Option<BString>,
    destination: BString,
}

fn renames(percentage: f32) -> Rewrites {
    Rewrites {
        percentage: Some(percentage),
        limit: 0,
        ..Default::default()
    }
}

fn copies(harder: bool) -> Rewrites {
    Rewrites {
        copies: Some(Copies {
            source: if harder {
                CopySource::FromSetOfModifiedFilesAndAllSources
            } else {
                CopySource::FromSetOfModifiedFiles
            },
            percentage: Some(0.5),
        }),
        ..renames(0.5)
    }
}

macro_rules! baseline {
    ($test:ident, $case:literal, $variant:literal, $rewrites:expr) => {
        #[test]
        fn $test() -> TestResult {
            assert_baseline($case, $variant, $rewrites)
        }
    };
}

baseline!(exact_basename, "exact-basename", "default", renames(0.5));
baseline!(
    exact_ignores_limit,
    "exact-basename",
    "exact",
    Rewrites {
        limit: 1,
        ..renames(1.0)
    }
);
baseline!(unique_basename_prepass, "basename-prepass", "default", renames(0.5));
baseline!(basename_respects_threshold, "basename-prepass", "high", renames(0.85));
baseline!(
    basename_precedes_limit,
    "basename-prepass",
    "limited",
    Rewrites {
        limit: 1,
        ..renames(0.5)
    }
);
baseline!(globally_best_pairs, "global-best", "default", renames(0.5));
baseline!(
    exhaustive_limit,
    "global-best",
    "limited",
    Rewrites {
        limit: 1,
        ..renames(0.5)
    }
);
baseline!(reordered_lines, "reordered", "default", renames(0.5));
baseline!(reordered_threshold, "reordered", "high", renames(0.95));
baseline!(exact_threshold, "reordered", "exact", renames(1.0));
baseline!(crlf_bytes, "crlf", "default", renames(0.5));
baseline!(crlf_threshold, "crlf", "high", renames(0.9));
baseline!(long_line_spans, "long-lines", "default", renames(0.5));
baseline!(binary_spans, "binary", "default", renames(0.5));
baseline!(unterminated_span, "partial-span", "default", renames(0.5));
baseline!(modified_copy_source, "modified-copy", "copies", copies(false));
baseline!(deleted_source_reuse, "deleted-copy", "copies", copies(false));
baseline!(unchanged_copy_source_excluded, "harder-copy", "copies", copies(false));
baseline!(unchanged_copy_source_included, "harder-copy", "harder", copies(true));
baseline!(
    single_addition_copy_harder,
    "single-harder-copy",
    "harder",
    copies(true)
);
baseline!(
    copy_harder_limit_fallback,
    "harder-limit",
    "harder",
    Rewrites {
        limit: 1,
        ..copies(true)
    }
);
baseline!(empty_excluded, "empty", "default", renames(0.5));
baseline!(
    empty_included,
    "empty",
    "tracked",
    Rewrites {
        track_empty: true,
        ..renames(0.5)
    }
);
baseline!(symlink_and_executable_modes, "modes", "default", renames(0.5));
baseline!(size_delta_rejected, "size-delta", "default", renames(0.5));
baseline!(size_delta_threshold, "size-delta", "low", renames(0.4));
baseline!(
    exact_copy_competition,
    "exact-copy-competition",
    "copies",
    Rewrites {
        copies: Some(Copies {
            percentage: Some(1.0),
            ..Default::default()
        }),
        ..renames(1.0)
    }
);
baseline!(
    exact_candidate_cap,
    "exact-candidate-cap-100",
    "exact",
    Rewrites {
        limit: 1,
        ..renames(1.0)
    }
);
baseline!(
    exact_candidate_cap_boundary,
    "exact-candidate-cap-99",
    "exact",
    Rewrites {
        limit: 1,
        ..renames(1.0)
    }
);
baseline!(four_candidate_ties, "matrix-ties", "default", renames(0.5));
baseline!(
    four_candidate_replacement,
    "matrix-replace-worst",
    "default",
    renames(0.5)
);
baseline!(
    below_threshold_candidates_preserve_tie_order,
    "matrix-below-threshold",
    "default",
    renames(0.5)
);
baseline!(
    size_rejected_candidates_preserve_tie_order,
    "matrix-size-rejected",
    "default",
    renames(0.5)
);

fn assert_baseline(case: &str, variant: &str, rewrites: Rewrites) -> TestResult {
    let root = crate::scripted_fixture_read_only("make_rewrite_git_baseline.sh")?;
    let case_dir = root.join(".git/baseline").join(case);
    if case == "partial-span" && !case_dir.join("counts-partial-span").is_file() {
        // Older Git did not score unterminated partial spans; private tests cover the current algorithm.
        return Ok(());
    }
    let odb = crate::open_odb(root.join(".git/objects"))?;
    let (mut tracker, mut actual) = input_tracker(&case_dir, rewrites)?;
    let sources = std::fs::read(case_dir.join("sources"))?;
    tracker.emit(
        |destination, source| {
            actual.push(match source {
                Some(source) => {
                    let kind = match source.kind {
                        SourceKind::Rename => 'R',
                        SourceKind::Copy => 'C',
                    };
                    let score = source
                        .similarity
                        .map_or(100, gix_diff::rewrites::Similarity::percentage);
                    Record {
                        status: format!("{kind}{score:03}").into(),
                        source: Some(source.location.into()),
                        destination: destination.location.into(),
                    }
                }
                None => plain_record(destination.change.kind, destination.location),
            });
            ControlFlow::Continue(())
        },
        &mut platform(&root),
        &odb,
        |push| -> Result<(), std::io::Error> {
            for entry in sources.split(|b| *b == 0).filter(|entry| !entry.is_empty()) {
                let (header, path) = entry.split_once_str(b"\t").expect("ls-tree separates paths with a tab");
                let fields: Vec<_> = header.split(|b| *b == b' ').collect();
                push(
                    Change {
                        kind: ChangeKind::Modification,
                        mode: EntryMode::from_bytes(fields[0]).expect("ls-tree writes valid modes"),
                        id: gix_hash::ObjectId::from_hex(fields[2]).expect("ls-tree writes valid object IDs"),
                    },
                    path.as_bstr(),
                );
            }
            Ok(())
        },
    )?;
    let expected_data = std::fs::read(case_dir.join(variant))?;
    let mut expected: Vec<_> = raw_records(&expected_data)
        .into_iter()
        .map(|(header, first, second)| Record {
            status: header.rsplit(|b| *b == b' ').next().expect("raw status").into(),
            source: second.map(|_| first.into()),
            destination: second.unwrap_or(first).into(),
        })
        .collect();
    actual.sort();
    expected.sort();
    pretty_assertions::assert_eq!(actual, expected, "Git baseline for {case}/{variant}");
    Ok(())
}

fn input_tracker(case_dir: &Path, rewrites: Rewrites) -> TestResult<(Tracker<Change>, Vec<Record>)> {
    let input = std::fs::read(case_dir.join("input"))?;
    let mut tracker = Tracker::<Change>::new(rewrites);
    let mut actual = Vec::new();
    for (header, path, _) in raw_records(&input) {
        let fields: Vec<_> = header[1..].split(|b| *b == b' ').collect();
        assert_eq!(fields.len(), 5, "Git raw diff headers contain modes, IDs, and status");
        let (kind, mode, blob_id) = match fields[4] {
            b"A" => (ChangeKind::Addition, fields[1], fields[3]),
            b"D" => (ChangeKind::Deletion, fields[0], fields[2]),
            b"M" => (ChangeKind::Modification, fields[0], fields[2]),
            status => panic!("unexpected input status: {:?}", status.as_bstr()),
        };
        let change = Change {
            kind,
            mode: EntryMode::from_bytes(mode).expect("Git writes valid nonzero modes"),
            id: gix_hash::ObjectId::from_hex(blob_id)?,
        };
        if tracker.try_push_change(change, path.as_bstr()).is_some() {
            actual.push(plain_record(kind, path));
        }
    }
    Ok((tracker, actual))
}

#[test]
fn cancellation_stops_file_pairs_and_unmatched_changes() -> TestResult {
    let root = crate::scripted_fixture_read_only("make_rewrite_git_baseline.sh")?;
    let odb = crate::open_odb(root.join(".git/objects"))?;
    for (case, expects_source) in [("exact-basename", true), ("global-best", true), ("empty", false)] {
        let (mut tracker, immediate) = input_tracker(&root.join(".git/baseline").join(case), renames(0.5))?;
        assert!(
            immediate.is_empty(),
            "these cases only contain tracked additions and deletions"
        );
        let mut calls = 0;
        tracker.emit(
            |_, source| {
                calls += 1;
                assert_eq!(source.is_some(), expects_source, "the callback exercises {case}");
                ControlFlow::Break(())
            },
            &mut platform(&root),
            &odb,
            |_| Ok::<_, std::io::Error>(()),
        )?;
        assert_eq!(calls, 1, "cancellation stops all further callbacks for {case}");
        tracker.emit(
            |_, _| panic!("a cancelled tracker must not retain changes for another emit"),
            &mut platform(&root),
            &odb,
            |_| Ok::<_, std::io::Error>(()),
        )?;
    }
    Ok(())
}

#[test]
fn stricter_copy_threshold_does_not_crowd_out_a_valid_rename() -> TestResult {
    let root = crate::scripted_fixture_read_only("make_rewrite_git_baseline.sh")?;
    let odb = crate::util::object_db();
    let mut tracker = Tracker::<Change>::new(Rewrites {
        copies: Some(Copies {
            percentage: Some(0.95),
            ..Default::default()
        }),
        ..renames(0.5)
    });
    for idx in 0..4 {
        let mut change = Change::modification();
        change.id = crate::util::insert(&odb, &format!("{}other{idx}\n", "same\n".repeat(9)))?;
        assert!(
            tracker
                .try_push_change(change, format!("modified{idx}").as_bytes().as_bstr())
                .is_none(),
            "modified sources participate in copy detection"
        );
    }
    for (mut change, path, data) in [
        (
            Change::deletion(),
            "z-deleted",
            format!("{}{}", "same\n".repeat(8), "other\n".repeat(2)),
        ),
        (Change::addition(), "destination", "same\n".repeat(10)),
    ] {
        change.id = crate::util::insert(&odb, &data)?;
        assert!(
            tracker.try_push_change(change, path.into()).is_none(),
            "rename inputs are tracked"
        );
    }
    let mut renamed = false;
    let mut modifications = 0;
    tracker.emit(
        |destination, source| {
            if let Some(source) = source {
                assert_eq!(source.kind, SourceKind::Rename, "only the rename is eligible");
                assert_eq!(
                    source.location, "z-deleted",
                    "ineligible copies must not consume candidate slots"
                );
                assert_eq!(
                    destination.location, "destination",
                    "the eligible deleted source is renamed"
                );
                assert!(!renamed, "the destination is emitted once");
                renamed = true;
            } else {
                assert_eq!(
                    destination.change.kind,
                    ChangeKind::Modification,
                    "only source modifications remain"
                );
                modifications += 1;
            }
            ControlFlow::Continue(())
        },
        &mut platform(&root),
        &odb,
        |_| Ok::<_, std::io::Error>(()),
    )?;
    assert!(renamed, "the lower rename threshold admits the deleted source");
    assert_eq!(modifications, 4, "copy-source modifications are all preserved");
    Ok(())
}

#[test]
fn a_later_weaker_copy_does_not_take_the_rename_label() -> TestResult {
    let root = crate::scripted_fixture_read_only("make_rewrite_git_baseline.sh")?;
    let odb = crate::util::object_db();
    for high_lines in [19, 20] {
        let mut tracker = Tracker::<Change>::new(Rewrites {
            copies: Some(Copies::default()),
            ..renames(if high_lines == 20 { 1.0 } else { 0.9 })
        });
        for (mut change, path, data) in [
            (Change::deletion(), "source", "same\n".repeat(20)),
            (
                Change::addition(),
                "a-high",
                format!("{}{}", "same\n".repeat(high_lines), "high\n".repeat(20 - high_lines)),
            ),
            (
                Change::addition(),
                "z-low",
                format!("{}{}", "same\n".repeat(12), "else\n".repeat(8)),
            ),
        ] {
            change.id = crate::util::insert(&odb, &data)?;
            assert!(
                tracker.try_push_change(change, path.into()).is_none(),
                "rewrite inputs are tracked"
            );
        }
        let mut calls = 0;
        tracker.emit(
            |destination, source| {
                let source = source.expect("both destinations have an eligible source");
                assert_eq!(source.location, "source", "both destinations reuse the deleted file");
                assert_eq!(
                    source.kind,
                    if destination.location == "a-high" {
                        SourceKind::Rename
                    } else {
                        SourceKind::Copy
                    },
                    "the rename must meet its own threshold, even when a weaker copy sorts last"
                );
                calls += 1;
                ControlFlow::Continue(())
            },
            &mut platform(&root),
            &odb,
            |_| Ok::<_, std::io::Error>(()),
        )?;
        assert_eq!(calls, 2, "the source is emitted as one rename and one copy");
    }
    Ok(())
}

#[test]
fn copy_harder_preserves_distinct_sources_with_truncated_locations() -> TestResult {
    let root = crate::scripted_fixture_read_only("make_rewrite_git_baseline.sh")?;
    let odb = crate::util::object_db();
    let unrelated_blob_id = crate::util::insert(&odb, "unrelated\n")?;
    let copied_blob_id = crate::util::insert(&odb, "copied\n")?;
    for location in ["", "file"] {
        let mut tracker = Tracker::<Change>::new(Rewrites {
            copies: Some(Copies {
                source: CopySource::FromSetOfModifiedFilesAndAllSources,
                percentage: Some(1.0),
            }),
            ..renames(1.0)
        });
        let modification = Change {
            id: unrelated_blob_id,
            ..Change::modification()
        };
        assert!(
            tracker.try_push_change(modification, location.into()).is_none(),
            "modifications are tracked"
        );
        assert!(
            tracker
                .try_push_change(
                    Change {
                        id: copied_blob_id,
                        ..Change::addition()
                    },
                    location.into()
                )
                .is_none(),
            "the destination is tracked"
        );
        let mut copied = false;
        let mut modifications = 0;
        tracker.emit(
            |destination, source| {
                if destination.change.kind == ChangeKind::Addition {
                    let source = source.expect("truncated locations must not hide the matching source");
                    assert_eq!(source.kind, SourceKind::Copy, "the source remains in the old tree");
                    assert_eq!(
                        source.id,
                        Some(copied_blob_id),
                        "the matching source retains its distinct blob ID"
                    );
                    assert_eq!(
                        source.entry_mode,
                        EntryMode::from(EntryKind::Blob),
                        "the regular source is retained too"
                    );
                    copied = true;
                } else {
                    assert!(
                        source.is_none(),
                        "the original modification is emitted without a source"
                    );
                    modifications += 1;
                }
                ControlFlow::Continue(())
            },
            &mut platform(&root),
            &odb,
            |push| {
                push(modification, location.into());
                for mode in [EntryKind::Link, EntryKind::Blob] {
                    push(
                        Change {
                            id: copied_blob_id,
                            mode: mode.into(),
                            ..Change::modification()
                        },
                        location.into(),
                    );
                }
                Ok::<_, std::io::Error>(())
            },
        )?;
        assert!(
            copied,
            "all observable old states remain available at location {location:?}"
        );
        assert_eq!(
            modifications, 1,
            "reimporting the original modification does not duplicate it"
        );
    }
    Ok(())
}

fn plain_record(kind: ChangeKind, path: &[u8]) -> Record {
    Record {
        status: match kind {
            ChangeKind::Addition => "A",
            ChangeKind::Deletion => "D",
            ChangeKind::Modification => "M",
        }
        .into(),
        source: None,
        destination: path.into(),
    }
}

type RawRecord<'a> = (&'a [u8], &'a [u8], Option<&'a [u8]>);

fn raw_records(data: &[u8]) -> Vec<RawRecord<'_>> {
    let mut fields = data.split(|b| *b == 0).filter(|field| !field.is_empty());
    let mut out = Vec::new();
    while let Some(header) = fields.next() {
        let path = fields.next().expect("raw records have a path");
        let status = header.rsplit(|b| *b == b' ').next().expect("raw status");
        let destination = matches!(status.first(), Some(b'R' | b'C'))
            .then(|| fields.next().expect("rewrites have a destination path"));
        out.push((header, path, destination));
    }
    out
}

fn platform(root: &Path) -> gix_diff::blob::Platform {
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
            gix_worktree::stack::State::AttributesStack(gix_worktree::stack::state::Attributes::default()),
            Default::default(),
            Vec::new(),
            Vec::new(),
        ),
    )
}

#[test]
fn matching_reads_each_inexact_blob_once_and_skips_exact_payloads() -> TestResult {
    struct ReadOnce {
        blobs: [(gix_hash::ObjectId, crate::util::ObjectDb, gix_object::Header); 2],
    }
    impl gix_object::Find for ReadOnce {
        fn try_find<'a>(
            &self,
            id: &gix_hash::oid,
            buffer: &'a mut Vec<u8>,
        ) -> gix_error::Result<Option<gix_object::Data<'a>>> {
            let (_, objects, _) = self
                .blobs
                .iter()
                .find(|(blob_id, _, _)| blob_id.as_ref() == id)
                .expect("known blob");
            assert_eq!(
                objects.num_objects_in_memory(),
                1,
                "a fingerprint needs one payload read"
            );
            let data = objects.try_find(id, buffer)?;
            drop(objects.reset_object_memory());
            Ok(data)
        }
    }
    impl gix_object::FindHeader for ReadOnce {
        fn try_header(&self, id: &gix_hash::oid) -> gix_error::Result<Option<gix_object::Header>> {
            Ok(self
                .blobs
                .iter()
                .find(|(blob_id, _, _)| blob_id.as_ref() == id)
                .map(|(_, _, header)| *header))
        }
    }
    let root = crate::scripted_fixture_read_only("make_rewrite_git_baseline.sh")?;
    let before = "one
two
three
";
    let after = "one
two
three
four
";
    for exact in [true, false] {
        let blob = |data: &str| -> gix_error::Result<_> {
            let objects = crate::util::object_db();
            let blob_id = crate::util::insert(&objects, data)?;
            Ok((
                blob_id,
                objects,
                gix_object::Header {
                    kind: gix_object::Kind::Blob,
                    size: data.len() as u64,
                },
            ))
        };
        let objects = ReadOnce {
            blobs: [blob(before)?, blob(if exact { before } else { after })?],
        };
        let mut tracker = Tracker::<Change>::new(renames(0.5));
        for (change, location) in [
            (
                Change {
                    id: objects.blobs[0].0,
                    ..Change::deletion()
                },
                "old",
            ),
            (
                Change {
                    id: objects.blobs[1].0,
                    ..Change::addition()
                },
                "new",
            ),
        ] {
            assert!(tracker.try_push_change(change, location.into()).is_none());
        }
        let mut matches = 0;
        tracker.emit(
            |_, source| {
                let source = source.expect("both versions are similar enough to match");
                assert_eq!(source.similarity.is_none(), exact, "identity matches need no score");
                matches += 1;
                ControlFlow::Continue(())
            },
            &mut platform(&root),
            &objects,
            |_| -> Result<(), std::io::Error> { panic!("rename-only matching must not enumerate copy sources") },
        )?;
        assert_eq!(matches, 1, "the source and destination form one rename");
        assert_eq!(
            objects
                .blobs
                .iter()
                .map(|(_, objects, _)| objects.num_objects_in_memory())
                .sum::<usize>(),
            if exact { 2 } else { 0 },
            "exact matching skips payloads; inexact matching reads each blob once without computing line statistics"
        );
    }
    Ok(())
}

#[test]
fn zero_threshold_does_not_match_edited_symlinks() -> TestResult {
    let root = crate::scripted_fixture_read_only("make_rewrite_git_baseline.sh")?;
    let objects = crate::util::object_db();
    let mut tracker = Tracker::<Change>::new(renames(0.0));
    for (path, change, target) in [
        ("source", Change::deletion(), "old-target"),
        ("destination", Change::addition(), "new-target"),
    ] {
        assert!(
            tracker
                .try_push_change(
                    Change {
                        id: crate::util::insert(&objects, target)?,
                        mode: EntryKind::Link.into(),
                        ..change
                    },
                    path.into(),
                )
                .is_none(),
            "both symlink changes are deferred for identity matching"
        );
    }
    let mut unmatched = 0;
    tracker.emit(
        |_, source| {
            assert!(
                source.is_none(),
                "symlinks require identical contents even at a zero threshold"
            );
            unmatched += 1;
            ControlFlow::Continue(())
        },
        &mut platform(&root),
        &objects,
        |_| Ok::<_, std::io::Error>(()),
    )?;
    assert_eq!(unmatched, 2, "the addition and deletion remain separate changes");
    Ok(())
}
