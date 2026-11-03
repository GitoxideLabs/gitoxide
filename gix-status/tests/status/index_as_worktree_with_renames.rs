use bstr::ByteSlice;
use gix_diff::{blob::pipeline::WorktreeRoots, rewrites::CopySource};
use gix_status::{
    index_as_worktree::{Change, EntryStatus, traits::FastEq},
    index_as_worktree_with_renames,
    index_as_worktree_with_renames::{Context, DirwalkContext, Entry, Options, Outcome, Recorder, Sorting, Summary},
};
use pretty_assertions::assert_eq;

use crate::fixture_path;

#[test]
fn gitlink_type_changes_are_not_also_untracked() -> gix_testtools::TestResult {
    for (name, worktree_mode) in [
        ("type-change", gix_index::entry::Mode::FILE),
        ("type-change-unconfigured-file", gix_index::entry::Mode::FILE),
        ("type-change-unconfigured-symlink", gix_index::entry::Mode::SYMLINK),
    ] {
        let git_dir = fixture_path("status_submodule").join(name).join(".git");
        if worktree_mode == gix_index::entry::Mode::SYMLINK && git_dir.join("symlinks-unavailable").is_file() {
            continue;
        }
        if worktree_mode == gix_index::entry::Mode::FILE {
            assert_eq!(
                std::fs::read_to_string(git_dir.join("status.baseline"))?,
                " T m1\n",
                "Git reports only a type change for {name}"
            );
        }
        assert_eq!(
            std::fs::read_to_string(git_dir.join("untracked.baseline"))?,
            "",
            "Git does not list the replaced gitlink as untracked for {name}"
        );
        fixture_filtered_detailed(
            "status_submodule.sh",
            name,
            &[],
            &[Expectation::Modification {
                rela_path: "m1",
                status: Change::Type { worktree_mode }.into(),
            }],
            None,
            Some(Default::default()),
            Fixture::ReadOnly,
        );
    }
    Ok(())
}

#[test]
fn changed_and_untracked_and_renamed() {
    let expectations_with_dirwalk = [
        // A deleted source can supply copies; its last destination is the rename.
        Expectation::Rewrite {
            source_rela_path: "dir/content",
            dest_rela_path: "content-copy",
            dest_dirwalk_status: gix_dir::entry::Status::Untracked,
            similarity: None,
            copy: true,
        },
        Expectation::Rewrite {
            source_rela_path: "dir/content",
            dest_rela_path: "content-copy-with-rewrite",
            dest_dirwalk_status: gix_dir::entry::Status::Untracked,
            similarity: gix_diff::rewrites::Similarity::from_score(40_000),
            copy: true,
        },
        Expectation::Rewrite {
            source_rela_path: "dir/content2",
            dest_rela_path: "content-with-rewrite",
            dest_dirwalk_status: gix_dir::entry::Status::Untracked,
            similarity: gix_diff::rewrites::Similarity::from_score(43_200),
            copy: false,
        },
        Expectation::Rewrite {
            source_rela_path: "empty",
            dest_rela_path: "dir/untracked",
            dest_dirwalk_status: gix_dir::entry::Status::Untracked,
            similarity: None,
            copy: true,
        },
        Expectation::Rewrite {
            source_rela_path: "dir/content",
            dest_rela_path: "plainly-renamed-content",
            dest_dirwalk_status: gix_dir::entry::Status::Untracked,
            similarity: None,
            copy: false,
        },
        Expectation::Rewrite {
            source_rela_path: "executable",
            dest_rela_path: "rewritten-executable",
            dest_dirwalk_status: gix_dir::entry::Status::Untracked,
            similarity: gix_diff::rewrites::Similarity::from_score(32_000),
            copy: false,
        },
        Expectation::Rewrite {
            source_rela_path: "empty",
            dest_rela_path: "untracked",
            dest_dirwalk_status: gix_dir::entry::Status::Untracked,
            similarity: None,
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
        Fixture::ReadOnlyNeedsArchive,
    );
    assert_eq!(
        out.rewrites,
        Some(gix_diff::rewrites::Outcome {
            options: rewrites,
            num_similarity_checks: 12,
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
                worktree_mode: gix_index::entry::Mode::FILE,
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
            read_dir_errors: 0,
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
            read_dir_errors: 0,
            returned_entries: 1,
            seen_entries: 3,
        }
    );
}

#[test]
fn untracked_file_removed_after_open() -> gix_testtools::TestResult {
    untracked_file_changed_after_open("bash .git/change-untracked.sh remove")
}

#[test]
fn untracked_file_replaced_after_open() -> gix_testtools::TestResult {
    untracked_file_changed_after_open("bash .git/change-untracked.sh replace")
}

fn untracked_file_changed_after_open(process: &'static str) -> gix_testtools::TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let out = fixture_filtered_detailed(
        "status_untracked_race.sh",
        "",
        &[],
        &[Expectation::Rewrite {
            source_rela_path: "source",
            dest_rela_path: "renamed",
            dest_dirwalk_status: gix_dir::entry::Status::Untracked,
            similarity: None,
            copy: false,
        }],
        Some(gix_diff::Rewrites {
            percentage: None,
            ..Default::default()
        }),
        Some(Default::default()),
        Fixture::WritableWithFilterProcess(process),
    );
    assert_eq!(
        out.rewrites.expect("rewrite tracking is enabled").num_similarity_checks,
        0,
        "the opened file's contents and length still identify an exact rename after its path changes"
    );
    Ok(())
}

enum Fixture {
    ReadOnly,
    ReadOnlyNeedsArchive,
    #[cfg(unix)]
    WritableExecuted,
    WritableWithFilterProcess(&'static str),
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

    let mut filter_process = None;
    let (worktree, _tmp) = match fixture {
        Fixture::ReadOnly => {
            let dir = fixture_path(script).join(subdir);
            (dir, None::<gix_testtools::tempfile::TempDir>)
        }
        Fixture::ReadOnlyNeedsArchive => {
            let dir = gix_testtools::scripted_fixture_read_only_needs_archive(script)
                .expect("script works")
                .join(subdir);
            (dir, None)
        }
        #[cfg(unix)]
        Fixture::WritableExecuted => {
            let tmp = crate::fixture_path_rw_slow(script);
            let dir = tmp.path().join(subdir);
            (dir, Some(tmp))
        }
        Fixture::WritableWithFilterProcess(process) => {
            filter_process = Some(process);
            let tmp = crate::scripted_fixture_writable(script).expect("script works");
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
    let mut filter = gix_filter::Pipeline::new(Default::default(), object_hash, Default::default());
    if let Some(process) = filter_process {
        filter.driver_state_mut().context.worktree_dir = Some(worktree.clone());
        filter.options_mut().drivers.push(gix_filter::Driver {
            name: "race".into(),
            clean: None,
            smudge: None,
            process: Some(process.into()),
            required: true,
        });
    }
    let resource_cache = gix_diff::blob::Platform::new(
        Default::default(),
        gix_diff::blob::Pipeline::new(
            WorktreeRoots {
                old_root: None,
                new_root: Some(worktree.to_owned()),
            },
            filter,
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
        fsmonitor: None,
        dirwalk: DirwalkContext {
            git_dir_realpath: &git_dir_real,
            current_dir: &cwd,
            ignore_case_index_lookup: None,
        },
    };
    let options = Options {
        tracked_file_modifications: gix_status::index_as_worktree::Options {
            fs: capabilities,
            stat: crate::index_as_worktree::TEST_OPTIONS,
            ..Default::default()
        },
        fscache: false,
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
        &mut gix_utils::progress::Discard,
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
                similarity,
                copy,
                ..
            } => Expectation::Rewrite {
                source_rela_path: source.rela_path().to_str().unwrap(),
                dest_rela_path: dirwalk_entry.rela_path.to_str().unwrap(),
                dest_dirwalk_status: dirwalk_entry.status,
                similarity: *similarity,
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
        similarity: Option<gix_diff::rewrites::Similarity>,
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
