use gix_error::Result;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use bstr::BStr;
use filetime::{FileTime, set_file_mtime};
use gix_filter::eol::AutoCrlf;
use gix_index as index;
use gix_index::{Entry, entry};
use gix_status::{
    index_as_worktree,
    index_as_worktree::{
        Change as WorktreeChange, Conflict, Context, EntryStatus as WorktreeEntryStatus, Options, Outcome, Record,
        Recorder,
        traits::{CompareBlobs, FastEq, ReadData, SubmoduleStatus},
    },
};

use crate::{fixture_path, hex_to_id, odb_at};
use gix_index::entry::{Flags, Mode};
use gix_status::index_as_worktree::ConflictIndexEntry;
use pretty_assertions::assert_eq;

// since tests are fixtures a bunch of stat information (like inode number)
// changes when extracting the data so we need to disable all advanced stat
// changes and only look at mtime seconds and file size to properly
// test all code paths (and to trigger racy git).
pub(super) const TEST_OPTIONS: index::entry::stat::Options = index::entry::stat::Options {
    trust_ctime: false,
    check_stat: false,
    use_nsec: false,
    use_stdev: false,
};

type Change = WorktreeChange<(), ()>;
type EntryStatus = WorktreeEntryStatus<(), ()>;
type Expectation<'a> = (&'a BStr, usize, EntryStatus);

fn attributes_stack(worktree: &std::path::Path, index: &gix_index::State) -> gix_worktree::Stack {
    gix_worktree::Stack::from_state_and_ignore_case(
        worktree,
        false,
        gix_worktree::stack::State::AttributesStack(Default::default()),
        index,
        index.path_backing(),
    )
}

fn fixture(name: &str, expected_status: &[Expectation<'_>]) -> Outcome {
    fixture_filtered(name, &[], expected_status)
}

#[cfg(unix)]
fn nonfile_fixture(name: &str, expected_status: &[Expectation<'_>]) -> Outcome {
    fixture_filtered_detailed(
        "status_nonfile",
        name,
        &[],
        expected_status,
        |_| {},
        false,
        Default::default(),
        false,
        None,
    )
}

fn fixture_with_index(
    name: &str,
    prepare_index: impl FnMut(&mut gix_index::State),
    expected_status: &[Expectation<'_>],
) -> Outcome {
    fixture_filtered_detailed(
        name,
        "",
        &[],
        expected_status,
        prepare_index,
        false,
        Default::default(),
        false,
        None,
    )
}

fn submodule_fixture(name: &str, expected_status: &[Expectation<'_>]) -> Outcome {
    fixture_filtered_detailed(
        "status_submodule",
        name,
        &[],
        expected_status,
        |_| {},
        false,
        Default::default(),
        false,
        None,
    )
}

fn conflict_fixture(name: &str, expected_status: &[Expectation<'_>]) -> Outcome {
    fixture_filtered_detailed(
        "conflicts",
        name,
        &[],
        expected_status,
        |_| {},
        false,
        Default::default(),
        false,
        None,
    )
}

fn submodule_fixture_status(name: &str, expected_status: &[Expectation<'_>], submodule_dirty: bool) -> Outcome {
    fixture_filtered_detailed(
        "status_submodule",
        name,
        &[],
        expected_status,
        |_| {},
        submodule_dirty,
        Default::default(),
        false,
        None,
    )
}

fn fixture_filtered(name: &str, pathspecs: &[&str], expected_status: &[Expectation<'_>]) -> Outcome {
    fixture_filtered_detailed(
        name,
        "",
        pathspecs,
        expected_status,
        |_| {},
        false,
        Default::default(),
        false,
        None,
    )
}

#[expect(clippy::too_many_arguments)]
fn fixture_filtered_detailed(
    name: &str,
    subdir: &str,
    pathspecs: &[&str],
    expected_status: &[Expectation<'_>],
    mut prepare_index: impl FnMut(&mut gix_index::State),
    submodule_dirty: bool,
    auto_crlf: gix_filter::eol::AutoCrlf,
    use_odb: bool,
    fs_capabilities: Option<&dyn Fn(&std::path::Path) -> gix_fs::Capabilities>,
) -> Outcome {
    // This can easily happen in some fixtures, which can cause flakiness. It's time-dependent after all.
    fn ignore_racyclean(mut out: Outcome) -> Outcome {
        out.racy_clean = 0;
        out
    }

    fn ignore_updated(mut out: Outcome) -> Outcome {
        out.entries_to_update = 0;
        out
    }

    fn ignore_worktree_stats(mut out: Outcome) -> Outcome {
        out.worktree_bytes = 0;
        out.worktree_files_read = 0;
        out
    }

    let worktree = fixture_path(name).join(subdir);
    let git_dir = worktree.join(".git");
    let object_hash = gix_testtools::object_hash();
    let mut index = gix_index::File::at(git_dir.join("index"), object_hash, false, Default::default()).unwrap();
    prepare_index(&mut index);
    let mut recorder = Recorder::default();
    let search = gix_pathspec::Search::from_specs(to_pathspecs(pathspecs), None, std::path::Path::new(""))
        .expect("valid specs can be normalized");
    let ctx = Context {
        pathspec: search,
        stack: attributes_stack(&worktree, &index),
        filter: gix_filter::Pipeline::new(
            Default::default(),
            gix_testtools::object_hash(),
            gix_filter::pipeline::Options {
                eol_config: gix_filter::eol::Configuration {
                    auto_crlf,
                    ..Default::default()
                },
                ..Default::default()
            },
        ),
        should_interrupt: &AtomicBool::default(),
    };
    let options = Options {
        fs: fs_capabilities.map_or_else(|| gix_fs::Capabilities::probe(&git_dir), |new| new(&git_dir)),
        stat: TEST_OPTIONS,
        ..Options::default()
    };
    let outcome = if use_odb {
        let odb = odb_at(&git_dir, object_hash);
        index_as_worktree(
            &index,
            &worktree,
            &mut recorder,
            FastEq,
            SubmoduleStatusMock { dirty: submodule_dirty },
            odb,
            &mut gix_utils::progress::Discard,
            ctx,
            options,
        )
    } else {
        let odb = gix_object::find::Never;
        index_as_worktree(
            &index,
            &worktree,
            &mut recorder,
            FastEq,
            SubmoduleStatusMock { dirty: submodule_dirty },
            &odb,
            &mut gix_utils::progress::Discard,
            ctx,
            options,
        )
    }
    .unwrap();
    recorder.records.sort_unstable_by_key(|r| r.relative_path);
    assert_eq!(records_to_tuple(recorder.records), expected_status);
    ignore_racyclean(ignore_updated(ignore_worktree_stats(outcome)))
}

/// Note that we also reset certain information to assure there is no flakiness - everything regarding race-detection otherwise can cause failures.
pub(super) fn records_to_tuple<'index>(
    records: impl IntoIterator<Item = Record<'index, (), ()>>,
) -> Vec<Expectation<'index>> {
    records
        .into_iter()
        .filter_map(|r| deracify_status(r.status).map(|status| (r.relative_path, r.entry_index, status)))
        .collect()
}

fn deracify_status(status: EntryStatus) -> Option<EntryStatus> {
    Some(match status {
        EntryStatus::Conflict { summary, entries } => EntryStatus::Conflict { summary, entries },
        EntryStatus::Change(c) => match c {
            Change::Removed => Change::Removed,
            Change::Type { worktree_mode } => Change::Type { worktree_mode },
            Change::Modification {
                executable_bit_changed,
                content_change,
                set_entry_stat_size_zero: _,
            } => Change::Modification {
                executable_bit_changed,
                content_change,
                set_entry_stat_size_zero: false,
            },
            Change::SubmoduleModification(c) => Change::SubmoduleModification(c),
        }
        .into(),
        EntryStatus::NeedsUpdate(_) => return None,
        EntryStatus::IntentToAdd => EntryStatus::IntentToAdd,
    })
}

#[derive(Clone)]
pub(super) struct SubmoduleStatusMock {
    pub(super) dirty: bool,
}

impl SubmoduleStatus for SubmoduleStatusMock {
    type Output = ();

    fn status(&mut self, _entry: &Entry, _rela_path: &BStr) -> Result<Option<Self::Output>> {
        Ok(self.dirty.then_some(()))
    }
}

pub(super) fn to_pathspecs(input: &[&str]) -> Vec<gix_pathspec::Pattern> {
    input
        .iter()
        .map(|pattern| gix_pathspec::parse(pattern.as_bytes(), Default::default()).expect("known to be valid"))
        .collect()
}

fn status_removed() -> EntryStatus {
    Change::Removed.into()
}

#[test]
fn hash_errors_preserve_io_kinds() {
    let mut diagnostics = Vec::new();
    use std::io::ErrorKind;

    for (stream_len, interrupted, expected_kind) in
        [(2, false, ErrorKind::UnexpectedEof), (1, true, ErrorKind::Interrupted)]
    {
        let err = gix_object::compute_stream_hash(
            gix_testtools::object_hash(),
            gix_object::Kind::Blob,
            &mut &b"x"[..],
            stream_len,
            &mut gix_utils::progress::Discard,
            &AtomicBool::new(interrupted),
        )
        .expect_err("a short stream or requested interruption prevents hashing");
        diagnostics.push(gix_testtools::redact_debug_snapshot(&err, &[]));
        let err = err
            .downcast_any_ref::<std::io::Error>()
            .expect("hashing I/O failures must remain I/O errors");
        assert_eq!(err.kind(), expected_kind, "callers must retain the native I/O kind");
    }
    insta::assert_debug_snapshot!(diagnostics, "stream hashing retains premature EOF and interruption diagnostics", @"
    [
        failed to fill whole buffer,
        I/O error (Interrupted)
        
        Caused by:
            0: Interrupted,
    ]
    ");
}

#[test]
fn hash_errors_without_io_causes_preserve_hashing_failure() {
    let err = gix_hash::io::from_hasher(gix_error::message("hash collision").corrupted_error());
    assert!(err.is_corrupted(), "the hashing failure retains its corruption class");
    assert!(
        err.downcast_any_ref::<std::io::Error>().is_none(),
        "a hashing failure without an I/O cause has no native kind"
    );
    insta::assert_debug_snapshot!(err, "the original hashing failure remains available for diagnostics", @"
    Failed to hash data

    Caused by:
        0: hash collision
    ");
}

#[test]
#[cfg(unix)]
fn nonfile_untracked_are_not_visible() {
    // And generally, untracked aren't visible here.
    nonfile_fixture("untracked", &[]);
}

#[test]
#[cfg(unix)]
fn tracked_changed_to_non_file() {
    nonfile_fixture(
        "tracked-swapped",
        &[(
            BStr::new(b"file"),
            0,
            Change::Type {
                worktree_mode: entry::Mode::FILE,
            }
            .into(),
        )],
    );
}

#[test]
#[cfg(unix)]
fn executable_changed_to_non_file_matches_git() -> gix_testtools::TestResult {
    let root = fixture_path("status_nonfile").join("executable-swapped");
    assert_eq!(
        std::fs::read_to_string(root.join(".git/status.baseline"))?,
        " M file\n",
        "Git reports an executable replaced by a FIFO as modified"
    );
    nonfile_fixture(
        "executable-swapped",
        &[(
            BStr::new(b"file"),
            0,
            Change::Modification {
                executable_bit_changed: true,
                content_change: Some(()),
                set_entry_stat_size_zero: false,
            }
            .into(),
        )],
    );
    Ok(())
}

#[test]
fn removed() {
    let out = fixture(
        "status_removed",
        &[
            (BStr::new(b"dir/content"), 0, status_removed()),
            (BStr::new(b"dir/sub-dir/symlink"), 1, status_removed()),
            (BStr::new(b"empty"), 2, status_removed()),
            (BStr::new(b"executable"), 3, status_removed()),
        ],
    );
    assert_eq!(
        out,
        Outcome {
            entries_to_process: 4,
            entries_processed: 4,
            symlink_metadata_calls: 4,
            ..Default::default()
        }
    );

    let out = fixture_filtered(
        "status_removed",
        &["dir"],
        &[
            (BStr::new(b"dir/content"), 0, status_removed()),
            (BStr::new(b"dir/sub-dir/symlink"), 1, status_removed()),
        ],
    );
    assert_eq!(
        out,
        Outcome {
            entries_to_process: 2,
            entries_processed: 2,
            entries_skipped_by_common_prefix: 2,
            symlink_metadata_calls: 2,
            ..Default::default()
        }
    );
}

#[test]
#[cfg(unix)]
fn removed_if_a_tracked_directory_is_replaced_with_a_symlink() {
    assert_eq!(
        fixture(
            "status_tracked_dir_replaced_with_symlink",
            &[(BStr::new(b"tracked/file"), 0, status_removed())]
        ),
        Outcome {
            entries_to_process: 1,
            entries_processed: 1,
            ..Default::default()
        }
    );
}

#[test]
fn replace_dir_with_file() {
    let out = fixture_filtered_detailed(
        "status_many",
        "replace-dir-with-file",
        &[],
        &[
            (BStr::new(b"dir/content"), 0, status_removed()),
            (BStr::new(b"dir/content2"), 1, status_removed()),
            (BStr::new(b"dir/sub/nested"), 2, status_removed()),
        ],
        |_| {},
        false,
        Default::default(),
        false,
        None,
    );
    assert_eq!(
        out,
        Outcome {
            entries_to_process: 5,
            entries_processed: 5,
            symlink_metadata_calls: if cfg!(windows) { 5 } else { 4 },
            ..Default::default()
        }
    );
}

#[test]
fn subomdule_nochange() {
    assert_eq!(
        submodule_fixture("no-change", &[]),
        Outcome {
            entries_to_process: 2,
            entries_processed: 2,
            symlink_metadata_calls: 2,
            ..Default::default()
        }
    );
}

#[test]
fn subomdule_deleted_dir() {
    assert_eq!(
        submodule_fixture("deleted-dir", &[(BStr::new(b"m1"), 1, status_removed())]),
        Outcome {
            entries_to_process: 2,
            entries_processed: 2,
            symlink_metadata_calls: 2,
            ..Default::default()
        }
    );
}

#[test]
fn subomdule_typechange() {
    assert_eq!(
        submodule_fixture(
            "type-change",
            &[(
                BStr::new(b"m1"),
                1,
                Change::Type {
                    worktree_mode: entry::Mode::FILE
                }
                .into()
            )]
        ),
        Outcome {
            entries_to_process: 2,
            entries_processed: 2,
            symlink_metadata_calls: 2,
            ..Default::default()
        }
    );
}

#[test]
fn subomdule_empty_dir_no_change() {
    assert_eq!(
        submodule_fixture("empty-dir-no-change", &[]),
        Outcome {
            entries_to_process: 2,
            entries_processed: 2,
            symlink_metadata_calls: 2,
            ..Default::default()
        }
    );
}

#[test]
fn subomdule_empty_dir_no_change_is_passed_to_submodule_handler() {
    assert_eq!(
        submodule_fixture_status(
            "empty-dir-no-change",
            &[(BStr::new(b"m1"), 1, Change::SubmoduleModification(()).into())],
            true,
        ),
        Outcome {
            entries_to_process: 2,
            entries_processed: 2,
            symlink_metadata_calls: 2,
            ..Default::default()
        }
    );
}

#[test]
fn intent_to_add() {
    assert_eq!(
        fixture(
            "status_intent_to_add",
            &[(BStr::new(b"content"), 0, EntryStatus::IntentToAdd)],
        ),
        Outcome {
            entries_to_process: 1,
            entries_processed: 1,
            symlink_metadata_calls: 1,
            ..Default::default()
        }
    );
}

#[test]
fn conflict() {
    assert_eq!(
        fixture(
            "status_conflict",
            &[(
                BStr::new(b"content"),
                0,
                EntryStatus::Conflict {
                    summary: Conflict::BothModified,
                    entries: Box::new([
                        Some(ConflictIndexEntry {
                            id: hex_to_id("df967b96a579e45a18b8251732d16804b2e56a55"),
                            flags: Flags::from_bits_retain(0x1000),
                            mode: Mode::FILE,
                        },),
                        Some(ConflictIndexEntry {
                            id: hex_to_id("d244dd0bf67758236f793fd7749a1c814fbfeac4"),
                            flags: Flags::from_bits_retain(0x2000),
                            mode: Mode::FILE,
                        },),
                        Some(ConflictIndexEntry {
                            id: hex_to_id("c7747099cf9e073babc68f52cdfb4d280ba5689f"),
                            flags: Flags::STAGE_MASK,
                            mode: Mode::FILE,
                        }),
                    ])
                }
            )],
        ),
        Outcome {
            entries_to_process: 3,
            entries_processed: 1,
            ..Default::default()
        },
        "2 entries were just related to the conflict, which we don't count as processed then"
    );
}

#[test]
fn conflict_both_deleted_and_added_by_them_and_added_by_us() {
    use Conflict::*;
    assert_eq!(
        conflict_fixture(
            "both-deleted",
            &[
                (
                    BStr::new(b"added-by-them"),
                    0,
                    EntryStatus::Conflict {
                        summary: AddedByThem,
                        entries: Box::new([
                            None,
                            None,
                            Some(ConflictIndexEntry {
                                id: hex_to_id("9daeafb9864cf43055ae93beb0afd6c7d144bfa4"),
                                flags: Flags::STAGE_MASK,
                                mode: Mode::FILE,
                            }),
                        ])
                    }
                ),
                (
                    BStr::new(b"added-by-us"),
                    1,
                    EntryStatus::Conflict {
                        summary: AddedByUs,
                        entries: Box::new([
                            None,
                            Some(ConflictIndexEntry {
                                id: hex_to_id("9daeafb9864cf43055ae93beb0afd6c7d144bfa4"),
                                flags: Flags::from_bits_retain(0x2000),
                                mode: Mode::FILE,
                            },),
                            None,
                        ])
                    }
                ),
                (
                    BStr::new(b"file"),
                    2,
                    EntryStatus::Conflict {
                        summary: BothDeleted,
                        entries: Box::new([
                            Some(ConflictIndexEntry {
                                id: hex_to_id("9daeafb9864cf43055ae93beb0afd6c7d144bfa4"),
                                flags: Flags::from_bits_retain(0x1000),
                                mode: Mode::FILE,
                            }),
                            None,
                            None,
                        ])
                    }
                ),
            ],
        ),
        Outcome {
            entries_to_process: 3,
            entries_processed: 3,
            ..Default::default()
        },
    );
}

/// Extra entries `try_from_entry` tells the status worker to skip after `start_index`.
/// This must be consecutive same-path conflict stages, not `stage - 1`.
#[test]
fn conflict_try_from_entry_skip_count_is_consecutive_same_path_stages() {
    use Conflict::*;
    use gix_index::entry::Stage::{Base, Ours, Theirs, Unconflicted};

    fn assert_conflict_skip_count(
        stages: &[(gix_index::entry::Stage, &str)],
        (expected_conflict, expected_extra, diagnostic): (Conflict, usize, Option<&str>),
    ) {
        fn index_with_stages(stages: &[(gix_index::entry::Stage, &str)]) -> gix_index::State {
            let object_hash = gix_testtools::object_hash();
            let mut state = gix_index::State::new(object_hash);
            let id = object_hash.null();
            for &(stage, path) in stages {
                state.dangerously_push_entry(
                    Default::default(),
                    id,
                    Flags::from_stage(stage),
                    Mode::FILE,
                    BStr::new(path.as_bytes()),
                );
            }
            state
        }

        let index = index_with_stages(stages);
        let path = index.entry(0).path(&index);
        let (actual_summary, actual_extra, _) =
            Conflict::try_from_entry(index.entries(), index.path_backing(), 0, path)
                .expect("the first entry is a conflict stage");
        assert_eq!(
            actual_summary,
            expected_conflict,
            "{}",
            diagnostic.unwrap_or("conflict classification")
        );
        assert_eq!(
            actual_extra, expected_extra,
            "{expected_conflict:?} extra skip count must be consecutive same-path stages after start, not stage-1"
        );
    }

    assert_conflict_skip_count(
        &[(Theirs, "ua"), (Unconflicted, "next"), (Unconflicted, "next2")],
        (AddedByThem, 0, None),
    );
    assert_conflict_skip_count(
        &[(Ours, "au"), (Unconflicted, "next")],
        (AddedByUs, 0, Some("an unconflicted entry terminates the conflict")),
    );
    assert_conflict_skip_count(
        &[(Ours, "a"), (Theirs, "b")],
        (
            AddedByUs,
            0,
            Some("a conflict entry for another path terminates the current conflict"),
        ),
    );
    assert_conflict_skip_count(&[(Base, "dd")], (BothDeleted, 0, None));
    assert_conflict_skip_count(&[(Base, "dt"), (Ours, "dt")], (DeletedByThem, 1, None));
    assert_conflict_skip_count(
        &[(Base, "du"), (Theirs, "du"), (Unconflicted, "next")],
        (DeletedByUs, 1, None),
    );
    assert_conflict_skip_count(
        &[(Ours, "aa"), (Theirs, "aa"), (Unconflicted, "next")],
        (BothAdded, 1, None),
    );
    assert_conflict_skip_count(&[(Base, "bm"), (Ours, "bm"), (Theirs, "bm")], (BothModified, 2, None));
}

#[test]
fn conflict_added_by_them_does_not_skip_following_dirty_path_when_chunked() {
    use Conflict::*;

    fn pad_count_for_chunk_size_gt_one() -> usize {
        // `optimize_chunk_size` uses `(n / (threads * 2)).clamp(1, 1000)`.
        // Need `n >= threads * 6` so chunk_size >= 3 and the two paths after AddedByThem share its chunk.
        let threads = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
        threads.saturating_mul(6).max(32)
    }
    let pad = pad_count_for_chunk_size_gt_one();
    assert_eq!(
        fixture_filtered_detailed(
            "conflicts",
            "both-deleted",
            &[],
            &[
                (
                    BStr::new(b"added-by-them"),
                    0,
                    EntryStatus::Conflict {
                        summary: AddedByThem,
                        entries: Box::new([
                            None,
                            None,
                            Some(ConflictIndexEntry {
                                id: hex_to_id("9daeafb9864cf43055ae93beb0afd6c7d144bfa4"),
                                flags: Flags::STAGE_MASK,
                                mode: Mode::FILE,
                            }),
                        ])
                    }
                ),
                (BStr::new(b"added-by-them-dirty"), 1, status_removed()),
                (
                    BStr::new(b"added-by-us"),
                    2,
                    EntryStatus::Conflict {
                        summary: AddedByUs,
                        entries: Box::new([
                            None,
                            Some(ConflictIndexEntry {
                                id: hex_to_id("9daeafb9864cf43055ae93beb0afd6c7d144bfa4"),
                                flags: Flags::from_bits_retain(0x2000),
                                mode: Mode::FILE,
                            }),
                            None,
                        ])
                    }
                ),
                (
                    BStr::new(b"file"),
                    3,
                    EntryStatus::Conflict {
                        summary: BothDeleted,
                        entries: Box::new([
                            Some(ConflictIndexEntry {
                                id: hex_to_id("9daeafb9864cf43055ae93beb0afd6c7d144bfa4"),
                                flags: Flags::from_bits_retain(0x1000),
                                mode: Mode::FILE,
                            }),
                            None,
                            None,
                        ])
                    }
                ),
            ],
            |index| {
                let proto = index.entry(0);
                let (stat, id, mode) = (proto.stat, proto.id, proto.mode);
                index.dangerously_push_entry(stat, id, Flags::empty(), mode, BStr::new(b"added-by-them-dirty"));
                for i in 0..pad {
                    let path = format!("z-pad-{i:03}");
                    index.dangerously_push_entry(stat, id, Flags::SKIP_WORKTREE, mode, BStr::new(path.as_bytes()));
                }
                index.sort_entries();
            },
            false,
            Default::default(),
            false,
            None,
        ),
        Outcome {
            entries_to_process: 4 + pad,
            entries_processed: 4 + pad,
            entries_skipped_by_entry_flags: pad,
            symlink_metadata_calls: 1,
            ..Default::default()
        },
        "every conflict and dirty entry must be processed; only SKIP_WORKTREE padding entries are skipped",
    );
}

#[test]
fn conflict_both_added_and_deleted_by_them() {
    use Conflict::*;
    assert_eq!(
        conflict_fixture(
            "both-added",
            &[
                (
                    BStr::new(b"both-added"),
                    0,
                    EntryStatus::Conflict {
                        summary: BothAdded,
                        entries: Box::new([
                            None,
                            Some(ConflictIndexEntry {
                                id: hex_to_id("ba2906d0666cf726c7eaadd2cd3db615dedfdf3a"),
                                flags: Flags::from_bits_retain(0x2000),
                                mode: Mode::FILE,
                            },),
                            Some(ConflictIndexEntry {
                                id: hex_to_id("e019be006cf33489e2d0177a3837a2384eddebc5"),
                                flags: Flags::STAGE_MASK,
                                mode: Mode::FILE,
                            },),
                        ])
                    }
                ),
                (
                    BStr::new(b"deleted-by-them"),
                    2,
                    EntryStatus::Conflict {
                        summary: DeletedByThem,
                        entries: Box::new([
                            Some(ConflictIndexEntry {
                                id: hex_to_id("b1b716105590454bfc4c0247f193a04088f39c7f"),
                                flags: Flags::from_bits_retain(0x1000),
                                mode: Mode::FILE,
                            },),
                            Some(ConflictIndexEntry {
                                id: hex_to_id("7d5ae6def200acda76d2ccf7c93170a9d88d6cb1"),
                                flags: Flags::from_bits_retain(0x2000),
                                mode: Mode::FILE,
                            },),
                            None,
                        ])
                    }
                ),
            ],
        ),
        Outcome {
            entries_to_process: 4,
            entries_processed: 2,
            ..Default::default()
        },
    );
}

#[test]
fn conflict_detailed_single() {
    use Conflict::*;
    for (name, expected, expected_entries, entry_index, entries_to_process, entries_processed) in [
        (
            "deleted-by-them",
            DeletedByThem,
            [
                Some(ConflictIndexEntry {
                    id: hex_to_id("dde77be9fbfb155ff0473e7fe31781d56d50e5d3"),
                    flags: Flags::from_bits_retain(0x1000),
                    mode: Mode::FILE,
                }),
                Some(ConflictIndexEntry {
                    id: hex_to_id("e14959721a622239cc8de786a4b8cfcefea8304c"),
                    flags: Flags::from_bits_retain(0x2000),
                    mode: Mode::FILE,
                }),
                None,
            ],
            0,
            2,
            1,
        ),
        (
            "deleted-by-us",
            DeletedByUs,
            [
                Some(ConflictIndexEntry {
                    id: hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391"),
                    flags: Flags::from_bits_retain(0x1000),
                    mode: Mode::FILE,
                }),
                None,
                Some(ConflictIndexEntry {
                    id: hex_to_id("0835e4f9714005ed591f68d306eea0d6d2ae8fd7"),
                    flags: Flags::STAGE_MASK,
                    mode: Mode::FILE,
                }),
            ],
            0,
            2,
            1,
        ),
        (
            "both-modified",
            BothModified,
            [
                Some(ConflictIndexEntry {
                    id: hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391"),
                    flags: Flags::from_bits_retain(0x1000),
                    mode: Mode::FILE,
                }),
                Some(ConflictIndexEntry {
                    id: hex_to_id("e45c9c2666d44e0327c1f9c239a74c508336053e"),
                    flags: Flags::from_bits_retain(0x2000),
                    mode: Mode::FILE,
                }),
                Some(ConflictIndexEntry {
                    id: hex_to_id("aac4af54d6427ef10af2b51a524e7272c4f37c02"),
                    flags: Flags::STAGE_MASK,
                    mode: Mode::FILE,
                }),
            ],
            0,
            3,
            1,
        ),
    ] {
        assert_eq!(
            conflict_fixture(
                name,
                &[(
                    BStr::new(b"file"),
                    entry_index,
                    EntryStatus::Conflict {
                        summary: expected,
                        entries: Box::new(expected_entries)
                    }
                )],
            ),
            Outcome {
                entries_to_process,
                entries_processed,
                ..Default::default()
            },
            "{name}"
        );
    }
}

#[test]
fn submodule_conflict() {
    assert_eq!(
        submodule_fixture(
            "conflict",
            &[(
                BStr::new(b"m1"),
                1,
                EntryStatus::Conflict {
                    summary: Conflict::DeletedByUs,
                    entries: Box::new([
                        Some(ConflictIndexEntry {
                            id: hex_to_id("3189cd3cb0af8586c39a838aa3e54fd72a872a41"),
                            flags: Flags::from_bits_retain(0x1000),
                            mode: Mode::DIR | Mode::SYMLINK
                        }),
                        None,
                        Some(ConflictIndexEntry {
                            id: hex_to_id("e376f96e6a7f1c9335ca16c3f62e172166146bda"),
                            flags: Flags::STAGE_MASK,
                            mode: Mode::DIR | Mode::SYMLINK,
                        }),
                    ])
                }
            )]
        ),
        Outcome {
            entries_to_process: 3,
            entries_processed: 2,
            symlink_metadata_calls: 1,
            ..Default::default()
        },
        "1 metadata call for .gitmodules, conflicting entries are not queried for status anymore."
    );
}

#[test]
fn unchanged() {
    fixture("status_unchanged", &[]);
}

#[test]
fn unchanged_symlinks_present_but_deactivated() {
    fixture_filtered_detailed(
        "status_unchanged",
        "",
        &[],
        &[],
        |_| {},
        false,
        Default::default(),
        false,
        Some(&|dir| gix_fs::Capabilities {
            symlink: false,
            ..gix_fs::Capabilities::probe(dir)
        }),
    );
}

#[test]
fn unchanged_despite_filter() {
    let actual_outcome = fixture_filtered_detailed(
        "status_unchanged_filter",
        "",
        &[],
        &[],
        |_| {},
        false,
        AutoCrlf::Enabled,
        true, /* make ODB available */
        None,
    );

    let expected_outcome = Outcome {
        entries_to_process: 5,
        entries_processed: 5,
        symlink_metadata_calls: 5,
        ..Default::default()
    };
    assert_eq!(actual_outcome, expected_outcome);
}

#[test]
fn clean_filter_receives_worktree_relative_path_from_worktree_root() -> gix_testtools::TestResult {
    // Use an absolute worktree path, as `gix` does.
    let worktree = gix_path::realpath(fixture_path("status_filter_path"))?;
    assert_ne!(
        std::env::current_dir()?,
        worktree,
        "the caller runs outside the worktree"
    );
    let git_dir = worktree.join(".git");
    let object_hash = gix_testtools::object_hash();
    let mut index = gix_index::File::at(git_dir.join("index"), object_hash, false, Default::default())?;
    let entry = index
        .entry_mut_by_path_and_stage("a.txt".into(), gix_index::entry::Stage::Unconflicted)
        .expect("the fixture tracks `a.txt`");
    // A stale `mtime` makes the stat check fail, so the worktree file is cleaned and hashed.
    entry.stat.mtime = Default::default();

    /// Record the output of the clean filter for each compared entry.
    #[derive(Clone, Default)]
    struct RecordCleaned(Arc<std::sync::Mutex<Vec<bstr::BString>>>);
    impl CompareBlobs for RecordCleaned {
        type Output = ();

        fn compare_blobs<'a, 'b>(
            &mut self,
            _entry: &Entry,
            _worktree_file_size: u64,
            data: impl ReadData<'a>,
            buf: &mut Vec<u8>,
        ) -> Result<Option<Self::Output>> {
            use std::io::Read;
            buf.clear();
            data.stream_worktree_file()?
                .read_to_end(buf)
                .map_err(gix_hash::io::from_std_io)?;
            self.0.lock().expect("not poisoned").push(buf.as_slice().into());
            Ok(None)
        }
    }

    let mut ctx = Context {
        pathspec: gix_pathspec::Search::from_specs(to_pathspecs(&["a.txt"]), None, std::path::Path::new(""))?,
        stack: attributes_stack(&worktree, &index),
        filter: gix_filter::Pipeline::new(
            Default::default(),
            object_hash,
            gix_filter::pipeline::Options {
                drivers: vec![gix_filter::Driver {
                    name: "path".into(),
                    // Both the relative path and its sibling configuration must resolve as they do in Git.
                    clean: Some(r#"cat >/dev/null && printf %s %f && cat "$(dirname %f)/.clean-map""#.into()),
                    smudge: None,
                    process: None,
                    // Fail instead of falling back to the unfiltered content, which also matches the index.
                    required: true,
                }],
                ..Default::default()
            },
        ),
        should_interrupt: &AtomicBool::default(),
    };
    ctx.filter.driver_state_mut().context.worktree_dir = Some(worktree.clone());
    let cleaned = RecordCleaned::default();
    index_as_worktree(
        &index,
        &worktree,
        &mut Recorder::default(),
        cleaned.clone(),
        SubmoduleStatusMock { dirty: false },
        odb_at(&git_dir, object_hash),
        &mut gix_utils::progress::Discard,
        ctx,
        Options {
            fs: gix_fs::Capabilities::probe(&git_dir),
            stat: TEST_OPTIONS,
            ..Options::default()
        },
    )?;

    assert_eq!(
        *cleaned.0.lock().expect("not poisoned"),
        [bstr::BString::from("a.txt mapped")],
        "like Git, the clean filter receives a worktree-relative `%f` and resolves sibling files from the worktree root"
    );
    Ok(())
}

#[test]
fn refresh() {
    let expected_outcome = Outcome {
        entries_to_process: 6,
        entries_processed: 6,
        symlink_metadata_calls: 6,
        ..Default::default()
    };
    assert_eq!(
        fixture_with_index(
            "status_changed",
            |index| { index.entries_mut().iter_mut().for_each(|e| e.stat = Default::default()) },
            #[cfg(not(windows))]
            &[
                (
                    BStr::new(b"dir/content"),
                    0,
                    Change::Modification {
                        executable_bit_changed: true,
                        content_change: None,
                        set_entry_stat_size_zero: false
                    }
                    .into(),
                ),
                (
                    BStr::new(b"dir/content2"),
                    1,
                    Change::Modification {
                        executable_bit_changed: false,
                        content_change: Some(()),
                        set_entry_stat_size_zero: false
                    }
                    .into(),
                ),
                (
                    BStr::new(b"empty"),
                    3,
                    Change::Type {
                        worktree_mode: entry::Mode::SYMLINK
                    }
                    .into()
                ),
                (
                    BStr::new(b"executable"),
                    4,
                    Change::Modification {
                        executable_bit_changed: true,
                        content_change: Some(()),
                        set_entry_stat_size_zero: false
                    }
                    .into(),
                ),
                (
                    BStr::new(b"z-executable"),
                    5,
                    Change::Type {
                        worktree_mode: entry::Mode::SYMLINK,
                    }
                    .into(),
                ),
            ],
            #[cfg(windows)]
            &[
                (
                    BStr::new("dir/content2"),
                    1,
                    Change::Modification {
                        executable_bit_changed: false,
                        content_change: Some(()),
                        set_entry_stat_size_zero: false
                    }
                    .into(),
                ),
                (
                    BStr::new("empty"),
                    3,
                    Change::Type {
                        worktree_mode: entry::Mode::SYMLINK
                    }
                    .into()
                ),
                (
                    BStr::new("executable"),
                    4,
                    Change::Modification {
                        executable_bit_changed: false,
                        content_change: Some(()),
                        set_entry_stat_size_zero: false
                    }
                    .into(),
                ),
                (
                    BStr::new("z-executable"),
                    5,
                    Change::Type {
                        worktree_mode: entry::Mode::SYMLINK,
                    }
                    .into(),
                ),
            ],
        ),
        expected_outcome,
    );
}

#[test]
fn modified() -> gix_testtools::TestResult {
    assert_eq!(
        std::fs::read_to_string(fixture_path("status_changed").join(".git/type-changes.baseline"))?,
        " T empty\n T z-executable\n",
        "Git reports both regular-file modes replaced by symlinks as type changes"
    );
    let expected_outcome = Outcome {
        entries_to_process: 6,
        entries_processed: 6,
        symlink_metadata_calls: 6,
        ..Default::default()
    };
    let actual_outcome = fixture(
        "status_changed",
        #[cfg(not(windows))]
        &[
            (
                BStr::new(b"dir/content"),
                0,
                Change::Modification {
                    executable_bit_changed: true,
                    content_change: None,
                    set_entry_stat_size_zero: false,
                }
                .into(),
            ),
            (
                BStr::new(b"dir/content2"),
                1,
                Change::Modification {
                    executable_bit_changed: false,
                    content_change: Some(()),
                    set_entry_stat_size_zero: false,
                }
                .into(),
            ),
            (
                BStr::new(b"empty"),
                3,
                Change::Type {
                    worktree_mode: entry::Mode::SYMLINK,
                }
                .into(),
            ),
            (
                BStr::new(b"executable"),
                4,
                Change::Modification {
                    executable_bit_changed: true,
                    content_change: Some(()),
                    set_entry_stat_size_zero: false,
                }
                .into(),
            ),
            (
                BStr::new(b"z-executable"),
                5,
                Change::Type {
                    worktree_mode: entry::Mode::SYMLINK,
                }
                .into(),
            ),
        ],
        #[cfg(windows)]
        &[
            (
                BStr::new("dir/content2"),
                1,
                Change::Modification {
                    executable_bit_changed: false,
                    content_change: Some(()),
                    set_entry_stat_size_zero: false,
                }
                .into(),
            ),
            (
                BStr::new("empty"),
                3,
                Change::Type {
                    worktree_mode: entry::Mode::SYMLINK,
                }
                .into(),
            ),
            (
                BStr::new("executable"),
                4,
                Change::Modification {
                    executable_bit_changed: false,
                    content_change: Some(()),
                    set_entry_stat_size_zero: false,
                }
                .into(),
            ),
            (
                BStr::new("z-executable"),
                5,
                Change::Type {
                    worktree_mode: entry::Mode::SYMLINK,
                }
                .into(),
            ),
        ],
    );
    assert_eq!(actual_outcome, expected_outcome);
    Ok(())
}

#[test]
fn modified_with_symlinks_disabled() -> gix_testtools::TestResult {
    for executable_bit in [false, true] {
        assert_eq!(
            std::fs::read_to_string(
                fixture_path("status_changed").join(format!(".git/type-change-modes-{executable_bit}.baseline"))
            )?,
            ":100644 120000\n:100755 120000\n",
            "Git preserves actual symlink modes even with core.symlinks=false"
        );
        fixture_filtered_detailed(
            "status_changed",
            "",
            &["empty", "z-executable"],
            &[
                (
                    BStr::new(b"empty"),
                    3,
                    Change::Type {
                        worktree_mode: Mode::SYMLINK,
                    }
                    .into(),
                ),
                (
                    BStr::new(b"z-executable"),
                    5,
                    Change::Type {
                        worktree_mode: Mode::SYMLINK,
                    }
                    .into(),
                ),
            ],
            |_| {},
            false,
            Default::default(),
            false,
            Some(&|dir| gix_fs::Capabilities {
                symlink: false,
                executable_bit,
                ..gix_fs::Capabilities::probe(dir)
            }),
        );
    }
    Ok(())
}

#[test]
fn racy_git() -> gix_testtools::TestResult {
    let timestamp = 940040400;
    // we need a writable fixture because we have to mess with `mtimes` manually, because touch -d
    // respects the locale so the test wouldn't work depending on the timezone you
    // run your test in.
    let dir = crate::scripted_fixture_writable("racy_git.sh")?;
    let worktree = dir.path();
    let git_dir = worktree.join(".git");
    let fs = gix_fs::Capabilities::probe(&git_dir);
    let object_hash = gix_testtools::object_hash();
    let mut index = gix_index::File::at(git_dir.join("index"), object_hash, false, Default::default())?;

    #[derive(Clone)]
    struct CountCalls(Arc<AtomicUsize>, FastEq);
    impl CompareBlobs for CountCalls {
        type Output = ();

        fn compare_blobs<'a, 'b>(
            &mut self,
            entry: &Entry,
            worktree_file_size: u64,
            data: impl ReadData<'a>,
            buf: &mut Vec<u8>,
        ) -> Result<Option<Self::Output>> {
            self.0.fetch_add(1, Ordering::Relaxed);
            self.1.compare_blobs(entry, worktree_file_size, data, buf)
        }
    }

    // We artificially mess with the entry's `mtime` so that it's before the timestamp saved by git.
    // This would usually mean an invalid fs/invalid index file and as a result the racy git
    // mitigation doesn't work and the worktree shows up as unchanged even tough the file did
    // change.
    // This case doesn't happen in the realworld (except for file corruption) but
    // makes sure we are actually hitting the right codepath.
    index.entries_mut()[0].stat.mtime.secs = timestamp;
    set_file_mtime(
        worktree.join("content"),
        FileTime::from_unix_time(i64::from(timestamp), 0),
    )?;
    let mut recorder = Recorder::default();

    let count = Arc::new(AtomicUsize::new(0));
    let counter = CountCalls(count.clone(), FastEq);
    let ctx = Context {
        pathspec: default_pathspec(),
        stack: attributes_stack(worktree, &index),
        filter: gix_filter::Pipeline::new(Default::default(), gix_testtools::object_hash(), Default::default()),
        should_interrupt: &AtomicBool::default(),
    };
    let out = index_as_worktree(
        &index,
        worktree,
        &mut recorder,
        counter.clone(),
        SubmoduleStatusMock { dirty: false },
        gix_object::find::Never,
        &mut gix_utils::progress::Discard,
        ctx.clone(),
        Options {
            fs,
            stat: TEST_OPTIONS,
            ..Options::default()
        },
    )?;
    assert_eq!(
        out,
        Outcome {
            entries_to_process: 1,
            entries_processed: 1,
            symlink_metadata_calls: 1,
            ..Default::default()
        }
    );
    assert_eq!(count.load(Ordering::Relaxed), 0, "no blob content is accessed");
    assert_eq!(
        records_to_tuple(recorder.records),
        &[],
        "the testcase triggers racy git"
    );

    // Now we also backdate the index timestamp to match the artificially created
    // mtime above this is now a realistic realworld race-condition which should trigger racy git
    // and cause proper output.
    index.set_timestamp(FileTime::from_unix_time(i64::from(timestamp), 0));
    let mut recorder = Recorder::default();
    let out = index_as_worktree(
        &index,
        worktree,
        &mut recorder,
        counter,
        SubmoduleStatusMock { dirty: false },
        gix_object::find::Never,
        &mut gix_utils::progress::Discard,
        ctx,
        Options {
            fs,
            stat: TEST_OPTIONS,
            ..Options::default()
        },
    )?;
    assert_eq!(
        out,
        Outcome {
            entries_to_process: 1,
            entries_processed: 1,
            symlink_metadata_calls: 1,
            racy_clean: 1,
            worktree_bytes: 3,
            worktree_files_read: 1,
            ..Default::default()
        }
    );
    assert_eq!(
        count.load(Ordering::Relaxed),
        1,
        "no we needed to access the blob content"
    );
    assert_eq!(
        records_to_tuple(recorder.records),
        &[(
            BStr::new(b"content"),
            0,
            Change::Modification {
                executable_bit_changed: false,
                content_change: Some(()),
                set_entry_stat_size_zero: false
            }
            .into(),
        )],
        "racy change is correctly detected"
    );
    Ok(())
}

fn default_pathspec() -> gix_pathspec::Search {
    gix_pathspec::Search::from_specs(to_pathspecs(&[]), None, std::path::Path::new("")).expect("empty is always valid")
}

#[cfg(windows)]
mod windows {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn fscache_preserves_live_status_on_misses() -> gix_error::TestResult {
        let worktree = gix_testtools::scripted_fixture_read_only("fscache.sh")?;
        let mut index = gix_index::State::new(gix_testtools::object_hash());
        for name in [
            "File",
            "directory",
            "directory/file",
            "file",
            "missing",
            "missing-parent/file",
        ] {
            index.dangerously_push_entry(
                Default::default(),
                gix_testtools::object_hash().null(),
                Flags::INTENT_TO_ADD,
                Mode::FILE,
                BStr::new(name),
            );
        }
        let mut results = Vec::new();
        for fscache in [false, true] {
            results.push(fscache_status(
                &index,
                &worktree,
                Options {
                    fscache,
                    ..Default::default()
                },
            )?);
        }
        assert_eq!(
            results[0].0, results[1].0,
            "cached and live status agree for hits, different casing and missing entries"
        );
        assert_eq!(
            results[0].1.symlink_metadata_calls, 6,
            "uncached status queries every path"
        );
        assert_eq!(
            results[1].1.symlink_metadata_calls, 3,
            "different casing, an absent entry and failed parent enumeration use live metadata"
        );
        Ok(())
    }

    fn fscache_status<'a>(
        index: &'a gix_index::State,
        worktree: &std::path::Path,
        options: Options,
    ) -> Result<(Vec<Expectation<'a>>, Outcome)> {
        let ctx = Context {
            pathspec: default_pathspec(),
            stack: attributes_stack(worktree, index),
            filter: gix_filter::Pipeline::new(Default::default(), gix_testtools::object_hash(), Default::default()),
            should_interrupt: &AtomicBool::default(),
        };
        let mut recorder = Recorder::default();
        let outcome = index_as_worktree(
            index,
            worktree,
            &mut recorder,
            FastEq,
            SubmoduleStatusMock { dirty: false },
            gix_object::find::Never,
            &mut gix_utils::progress::Discard,
            ctx,
            Options {
                thread_limit: Some(1),
                ..options
            },
        )?;
        recorder.records.sort_unstable_by_key(|record| record.relative_path);
        Ok((
            recorder
                .records
                .into_iter()
                .map(|record| (record.relative_path, record.entry_index, record.status))
                .collect(),
            outcome,
        ))
    }

    #[test]
    fn fscache_preserves_stat_and_mode_status() -> gix_error::TestResult {
        let worktree = gix_testtools::scripted_fixture_read_only("fscache.sh")?;
        let object_hash = gix_testtools::object_hash();
        let mut index = gix_index::State::new(object_hash);
        for (name, mode) in [("directory", Mode::FILE), ("directory/file", Mode::FILE_EXECUTABLE)] {
            // Empty index blobs differ in size, so FastEq can detect modifications without an ODB.
            index.dangerously_push_entry(
                Default::default(),
                object_hash.empty_blob(),
                Flags::empty(),
                mode,
                BStr::new(name),
            );
        }
        let metadata = gix_index::fs::Metadata::from_path_no_follow(&worktree.join("file"))?;
        index.dangerously_push_entry(
            gix_index::entry::Stat::from_fs(&metadata)?,
            gix_object::compute_hash(object_hash, gix_object::Kind::Blob, b"hello")?,
            Flags::empty(),
            Mode::FILE,
            BStr::new("file"),
        );
        index.set_timestamp(FileTime::from_system_time(
            std::time::SystemTime::now() + std::time::Duration::from_secs(60),
        ));
        for executable_bit in [false, true] {
            let options = Options {
                fs: gix_fs::Capabilities {
                    executable_bit,
                    ..Default::default()
                },
                stat: gix_index::entry::stat::Options {
                    use_nsec: true,
                    ..Default::default()
                },
                ..Default::default()
            };
            let (live, live_outcome) = fscache_status(
                &index,
                &worktree,
                Options {
                    fscache: false,
                    ..options.clone()
                },
            )?;
            let (cached, cached_outcome) = fscache_status(
                &index,
                &worktree,
                Options {
                    fscache: true,
                    ..options
                },
            )?;
            assert_eq!(
                cached, live,
                "cached and live status use the same stat and mode policies"
            );
            assert_eq!(
                cached,
                vec![
                    (BStr::new("directory"), 0, Change::Removed.into()),
                    (
                        BStr::new("directory/file"),
                        1,
                        Change::Modification {
                            executable_bit_changed: executable_bit,
                            content_change: Some(()),
                            set_entry_stat_size_zero: false,
                        }
                        .into()
                    ),
                ],
                "unchanged files, replaced directories and executable mode changes are distinguished"
            );
            assert_eq!(
                live_outcome.symlink_metadata_calls, 3,
                "live status reads metadata for each entry"
            );
            assert_eq!(
                cached_outcome.symlink_metadata_calls, 0,
                "cache hits avoid per-entry metadata calls"
            );
        }
        Ok(())
    }

    #[test]
    fn fscache_preserves_timestamp_errors_and_early_returns() -> gix_error::TestResult {
        let (worktree, ()) = gix_testtools::scripted_fixture_read_only_with_post("fscache.sh", 3, |fixture| {
            if let gix_testtools::FixtureState::Uninitialized(path) = fixture {
                set_file_mtime(path.join("file"), FileTime::from_unix_time(-1, 0))?;
            }
            Ok(())
        })?;
        for flags in [Flags::empty(), Flags::INTENT_TO_ADD] {
            let mut index = gix_index::State::new(gix_testtools::object_hash());
            index.dangerously_push_entry(
                Default::default(),
                gix_testtools::object_hash().empty_blob(),
                flags,
                Mode::FILE,
                BStr::new("file"),
            );
            for fscache in [false, true] {
                let result = fscache_status(
                    &index,
                    &worktree,
                    Options {
                        fscache,
                        ..Default::default()
                    },
                );
                if flags.is_empty() {
                    let err = result.expect_err("pre-epoch metadata must fail status");
                    assert!(
                        err.to_string().contains("The clock was off when reading file metadata"),
                        "status preserves timestamp error context"
                    );
                    let cause = err
                        .downcast_any_ref::<std::time::SystemTimeError>()
                        .expect("the timestamp error remains downcastable");
                    assert_eq!(
                        cause.duration(),
                        std::time::Duration::from_secs(1),
                        "the timestamp error is not normalized away"
                    );
                } else {
                    let (records, outcome) = result?;
                    assert_eq!(
                        records,
                        vec![(BStr::new("file"), 0, EntryStatus::IntentToAdd)],
                        "intent-to-add returns before stat conversion"
                    );
                    assert_eq!(
                        outcome.symlink_metadata_calls, 1,
                        "unusable cache entries still fall back to live metadata"
                    );
                }
            }
        }
        Ok(())
    }
}
