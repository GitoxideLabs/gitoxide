use bstr::ByteSlice;
use gix_diff::{blob::pipeline::WorktreeRoots, rewrites::CopySource};
use gix_index::entry;
use gix_status::{
    index_as_worktree::{Change, EntryStatus, traits::FastEq},
    index_as_worktree_with_renames,
    index_as_worktree_with_renames::{Context, DirwalkContext, Entry, Options, Outcome, Recorder, Sorting, Summary},
};
use pretty_assertions::assert_eq;

use crate::{fixture_path, fixture_path_rw_slow};

#[test]
fn changed_and_untracked_and_renamed() {
    let expectations_with_dirwalk = [
        // Not always will we match the right source to destinations, there is ambiguity.
        Expectation::Rewrite {
            source_rela_path: "dir/content",
            dest_rela_path: "content-copy",
            dest_dirwalk_status: gix_dir::entry::Status::Untracked,
            diff: None,
            copy: false,
        },
        Expectation::DirwalkEntry {
            rela_path: "content-copy-with-rewrite",
            status: gix_dir::entry::Status::Untracked,
            disk_kind: Some(gix_dir::entry::Kind::File),
        },
        Expectation::Rewrite {
            source_rela_path: "dir/content2",
            dest_rela_path: "content-with-rewrite",
            dest_dirwalk_status: gix_dir::entry::Status::Untracked,
            diff: Some(gix_diff::blob::DiffLineStats {
                removals: 0,
                insertions: 1,
                before: 1,
                after: 2,
                similarity: 0.72,
            }),
            copy: false,
        },
        Expectation::Rewrite {
            source_rela_path: "empty",
            dest_rela_path: "dir/untracked",
            dest_dirwalk_status: gix_dir::entry::Status::Untracked,
            diff: None,
            copy: true,
        },
        // This is just detected as untracked, related to how the rename-tracker matches pairs
        Expectation::DirwalkEntry {
            rela_path: "plainly-renamed-content",
            status: gix_dir::entry::Status::Untracked,
            disk_kind: Some(gix_dir::entry::Kind::File),
        },
        Expectation::Rewrite {
            source_rela_path: "executable",
            dest_rela_path: "rewritten-executable",
            dest_dirwalk_status: gix_dir::entry::Status::Untracked,
            diff: Some(gix_diff::blob::DiffLineStats {
                removals: 0,
                insertions: 1,
                before: 1,
                after: 2,
                similarity: 0.53333336,
            }),
            copy: false,
        },
        Expectation::Rewrite {
            source_rela_path: "empty",
            dest_rela_path: "untracked",
            dest_dirwalk_status: gix_dir::entry::Status::Untracked,
            diff: None,
            copy: true,
        },
    ];
    let rewrites = gix_diff::Rewrites {
        copies: Some(gix_diff::rewrites::Copies {
            source: CopySource::FromSetOfModifiedFiles,
            percentage: Some(0.3),
        }),
        percentage: Some(0.3),
        limit: 0,
        track_empty: true,
    };
    let out = fixture_filtered_detailed(
        "status_many.sh",
        "changed-and-untracked-and-renamed",
        &[],
        &expectations_with_dirwalk,
        Some(rewrites),
        Some(Default::default()),
        Fixture::ReadOnly,
    );
    // The amount of checks currently depends on hashes as they are sorted,
    // and with changes in order come changes in checks. This shold go away
    // with proper, non-hash dependent heuristics.
    let num_similarity_checks = match gix_testtools::object_hash() {
        gix_hash::Kind::Sha1 => 11,
        gix_hash::Kind::Sha256 => 10,
        _ => unimplemented!(),
    };
    assert_eq!(
        out.rewrites,
        Some(gix_diff::rewrites::Outcome {
            options: rewrites,
            num_similarity_checks,
            num_similarity_checks_skipped_for_rename_tracking_due_to_limit: 0,
            num_similarity_checks_skipped_for_copy_tracking_due_to_limit: 0,
        })
    );
}

#[test]
#[cfg(unix)]
fn nonfile_untracked_are_not_visible() {
    fixture_filtered_detailed(
        "status_nonfile.sh",
        "untracked",
        &[],
        &[],
        None,
        Some(Default::default()),
        Fixture::ReadOnly,
    );
}

#[test]
#[cfg(unix)]
fn tracked_changed_to_non_file() {
    fixture_filtered_detailed(
        "status_nonfile.sh",
        "tracked-swapped",
        &[],
        &[Expectation::Modification {
            rela_path: "file",
            status: Change::Type {
                worktree_mode: entry::Mode::FILE,
            }
            .into(),
        }],
        None,
        Some(Default::default()),
        Fixture::ReadOnly,
    );
}

#[test]
fn changed_and_untracked() {
    let out = fixture_filtered_detailed(
        "status_many.sh",
        "changed-and-untracked",
        &[],
        &[Expectation::Modification {
            rela_path: "executable",
            status: EntryStatus::Change(Change::Modification {
                executable_bit_changed: false,
                content_change: Some(()),
                set_entry_stat_size_zero: false,
            }),
        }],
        None,
        None,
        Fixture::ReadOnly,
    );
    assert_eq!(out.tracked_file_modification.entries_processed, 4);
    assert_eq!(
        out.dirwalk, None,
        "we didn't configure the dirwalk, so it's just like a modification check"
    );
    assert_eq!(out.rewrites, None, "rewrite checking isn't configured either");

    let expectations_with_dirwalk = [
        Expectation::DirwalkEntry {
            rela_path: "dir/untracked",
            status: gix_dir::entry::Status::Untracked,
            disk_kind: Some(gix_dir::entry::Kind::File),
        },
        Expectation::Modification {
            rela_path: "executable",
            status: EntryStatus::Change(Change::Modification {
                executable_bit_changed: false,
                content_change: Some(()),
                set_entry_stat_size_zero: false,
            }),
        },
        Expectation::DirwalkEntry {
            rela_path: "untracked",
            status: gix_dir::entry::Status::Untracked,
            disk_kind: Some(gix_dir::entry::Kind::File),
        },
    ];
    let out = fixture_filtered_detailed(
        "status_many.sh",
        "changed-and-untracked",
        &[],
        &expectations_with_dirwalk,
        None,
        Some(gix_dir::walk::Options::default()),
        Fixture::ReadOnly,
    );

    let dirwalk = out.dirwalk.expect("configured thus has output");
    assert_eq!(
        dirwalk,
        gix_dir::walk::Outcome {
            read_dir_calls: 3,
            returned_entries: 2,
            seen_entries: 8,
        }
    );
    assert_eq!(out.rewrites, None, "rewrites are still not configured");

    let out = fixture_filtered_detailed(
        "status_many.sh",
        "changed-and-untracked",
        &[],
        &expectations_with_dirwalk,
        Some(Default::default()),
        Some(gix_dir::walk::Options::default()),
        Fixture::ReadOnly,
    );

    let rewrites = out.rewrites.expect("configured thus has output");
    assert_eq!(
        rewrites,
        gix_diff::rewrites::Outcome::default(),
        "there actually is no candidates pairs as there are no deletions"
    );
}

#[cfg(unix)]
#[test]
fn unreadable_untracked() {
    let expectations_with_dirwalk = [Expectation::DirwalkEntry {
        rela_path: "unreadable",
        status: gix_dir::entry::Status::Untracked,
        disk_kind: Some(gix_dir::entry::Kind::File),
    }];
    let out = fixture_filtered_detailed(
        "unreadable_untracked.sh",
        "",
        &[],
        &expectations_with_dirwalk,
        Some(Default::default()),
        Some(gix_dir::walk::Options::default()),
        Fixture::WritableExecuted,
    );

    let dirwalk = out.dirwalk.expect("configured thus has output");
    assert_eq!(
        dirwalk,
        gix_dir::walk::Outcome {
            read_dir_calls: 1,
            returned_entries: 1,
            seen_entries: 3,
        }
    );
}

enum Fixture {
    ReadOnly,
    WritableExecuted,
}

fn fixture_filtered_detailed(
    script: &str,
    subdir: &str,
    pathspecs: &[&str],
    expected: &[Expectation<'_>],
    rewrites: Option<gix_diff::Rewrites>,
    dirwalk: Option<gix_dir::walk::Options>,
    fixture: Fixture,
) -> Outcome {
    fn cleanup(mut out: Outcome) -> Outcome {
        out.tracked_file_modification.worktree_bytes = 0;
        out.tracked_file_modification.worktree_files_read = 0;
        out.tracked_file_modification.entries_to_update = 0;
        out.tracked_file_modification.racy_clean = 0;
        out
    }

    let (worktree, _tmp) = match fixture {
        Fixture::ReadOnly => {
            let dir = fixture_path(script).join(subdir);
            (dir, None)
        }
        Fixture::WritableExecuted => {
            let tmp = fixture_path_rw_slow(script);
            let dir = tmp.path().join(subdir);
            (dir, Some(tmp))
        }
    };
    let git_dir = worktree.join(".git");
    let object_hash = gix_testtools::object_hash();
    let index = gix_index::File::at(git_dir.join("index"), object_hash, false, Default::default()).unwrap();
    let search = gix_pathspec::Search::from_specs(
        crate::index_as_worktree::to_pathspecs(pathspecs),
        None,
        std::path::Path::new(""),
    )
    .expect("valid specs can be normalized");
    let stack = gix_worktree::Stack::from_state_and_ignore_case(
        worktree.clone(),
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
            WorktreeRoots {
                old_root: None,
                new_root: Some(worktree.to_owned()),
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

    let git_dir_real = gix_path::realpath(&git_dir).unwrap();
    let cwd = gix_fs::current_dir(capabilities.precompose_unicode).unwrap();
    let context = Context {
        pathspec: search,
        resource_cache,
        should_interrupt: &Default::default(),
        dirwalk: DirwalkContext {
            git_dir_realpath: &git_dir_real,
            current_dir: &cwd,
            ignore_case_index_lookup: None,
        },
    };
    let options = Options {
        object_hash,
        tracked_file_modifications: gix_status::index_as_worktree::Options {
            fs: capabilities,
            stat: crate::index_as_worktree::TEST_OPTIONS,
            ..Default::default()
        },
        dirwalk,
        sorting: Some(Sorting::ByPathCaseSensitive),
        rewrites,
    };

    let mut recorder = Recorder::default();
    let objects = crate::odb_at(&git_dir, object_hash);
    let outcome = index_as_worktree_with_renames(
        &index,
        &worktree,
        &mut recorder,
        FastEq,
        crate::index_as_worktree::SubmoduleStatusMock { dirty: false },
        objects,
        &mut gix_features::progress::Discard,
        context,
        options,
    )
    .unwrap();

    let actual = records_to_expectations(&recorder.records);
    assert_eq!(actual, expected);
    assert_summary(&recorder.records, expected);
    cleanup(outcome)
}

fn assert_summary(entries: &[Entry<(), ()>], expected: &[Expectation]) {
    let entries: Vec<_> = entries
        .iter()
        .filter(|r| {
            !matches!(
                r,
                Entry::Modification {
                    status: EntryStatus::NeedsUpdate(..),
                    ..
                }
            )
        })
        .collect();
    assert_eq!(entries.len(), expected.len());
    for (entry, expected) in entries.iter().zip(expected) {
        assert_eq!(entry.summary(), expected.summary());
    }
}

fn records_to_expectations<'a>(recs: &'a [Entry<'_, (), ()>]) -> Vec<Expectation<'a>> {
    recs.iter()
        .filter(|r| {
            !matches!(
                r,
                Entry::Modification {
                    status: EntryStatus::NeedsUpdate(..),
                    ..
                }
            )
        })
        .map(|r| match r {
            Entry::Modification { rela_path, status, .. } => Expectation::Modification {
                rela_path: rela_path.to_str().unwrap(),
                status: status.clone(),
            },
            Entry::DirectoryContents { entry, .. } => Expectation::DirwalkEntry {
                rela_path: entry.rela_path.to_str().unwrap(),
                status: entry.status,
                disk_kind: entry.disk_kind,
            },
            Entry::Rewrite {
                source,
                dirwalk_entry,
                diff,
                copy,
                ..
            } => Expectation::Rewrite {
                source_rela_path: source.rela_path().to_str().unwrap(),
                dest_rela_path: dirwalk_entry.rela_path.to_str().unwrap(),
                dest_dirwalk_status: dirwalk_entry.status,
                diff: *diff,
                copy: *copy,
            },
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
enum Expectation<'a> {
    Modification {
        rela_path: &'a str,
        status: EntryStatus<(), ()>,
    },
    DirwalkEntry {
        rela_path: &'a str,
        status: gix_dir::entry::Status,
        disk_kind: Option<gix_dir::entry::Kind>,
    },
    Rewrite {
        source_rela_path: &'a str,
        dest_rela_path: &'a str,
        dest_dirwalk_status: gix_dir::entry::Status,
        diff: Option<gix_diff::blob::DiffLineStats>,
        copy: bool,
    },
}

impl Expectation<'_> {
    pub fn summary(&self) -> Option<Summary> {
        Some(match self {
            Expectation::Modification { status, .. } => match status {
                EntryStatus::Conflict { .. } => Summary::Conflict,
                EntryStatus::Change(change) => match change {
                    Change::Removed => Summary::Removed,
                    Change::Type { .. } => Summary::TypeChange,
                    Change::Modification { .. } | Change::SubmoduleModification(_) => Summary::Modified,
                },
                EntryStatus::NeedsUpdate(_) => return None,
                EntryStatus::IntentToAdd => Summary::IntentToAdd,
            },
            Expectation::DirwalkEntry { status, .. } => {
                if matches!(status, gix_dir::entry::Status::Untracked) {
                    Summary::Added
                } else {
                    return None;
                }
            }
            Expectation::Rewrite { copy, .. } => {
                if *copy {
                    Summary::Copied
                } else {
                    Summary::Renamed
                }
            }
        })
    }
}
