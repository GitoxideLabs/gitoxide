use crate::Result;
pub fn submodule_repo(name: &str) -> Result<gix::Repository> {
    use crate::util::named_subrepo_opts;
    Ok(named_subrepo_opts(
        "make_submodules.sh",
        name,
        gix::open::Options::isolated(),
    )?)
}

pub fn repo(name: &str) -> Result<gix::Repository> {
    use crate::util::named_subrepo_opts;
    Ok(named_subrepo_opts(
        "make_status_repos.sh",
        name,
        gix::open::Options::isolated(),
    )?)
}

#[test]
fn default_thread_limit() -> gix_error::TestResult {
    let expected = cfg!(target_os = "macos").then_some(3);
    assert_eq!(
        gix::status::index_worktree::Options::default().thread_limit,
        expected,
        "the default caps threads at three on macOS and leaves them unlimited elsewhere"
    );
    repo("untracked-only")?
        .status(gix::progress::Discard)?
        .index_worktree_options_mut(|opts| {
            assert_eq!(
                opts.thread_limit, expected,
                "the status platform preserves the default thread limit"
            );
        });
    Ok(())
}

mod into_iter {
    use gix::status::{Item, Submodule, tree_index::TrackRenames};
    use gix_diff::Rewrites;
    use gix_testtools::TestResult;
    use gix_testtools::size_ok;

    use crate::status::{repo, submodule_repo};

    #[cfg(unix)]
    #[test]
    fn status_does_not_write_to_the_worktree() -> TestResult {
        let (fixture, ()) = gix_testtools::scripted_fixture_read_only_with_post("make_status_repos.sh", 1, |state| {
            if let gix_testtools::FixtureState::Uninitialized(path) = state {
                // An old timestamp exposes even short-lived probes on coarse filesystems.
                std::fs::File::open(path.join("untracked-only"))?
                    .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000))?;
            }
            Ok(())
        })?;
        let root = fixture.join("untracked-only");
        let repo = gix::open_opts(&root, gix::open::Options::isolated().filter_config_section(|_| false))?;
        let before = std::fs::metadata(&root)?.modified()?;
        for item in repo.status(gix::progress::Discard)?.into_iter(None)? {
            item?;
        }
        assert_eq!(
            std::fs::metadata(&root)?.modified()?,
            before,
            "reading status must not create temporary entries that invalidate the monitor"
        );
        Ok(())
    }

    #[test]
    fn index_writes_respect_checksum_configuration_trust() -> TestResult {
        for trust in [gix_sec::Trust::Reduced, gix_sec::Trust::Full] {
            let (mut repo, _fixture) = crate::basic_rw_repo()?;
            repo.config_snapshot_mut()
                .append_config(["index.skipHash=false"], gix_config::Source::User)?;
            repo.config_snapshot_mut()
                .append(gix_config::File::from_bytes_no_includes(
                    b"[index]
                        skipHash = true
                    ",
                    gix_config::file::Metadata::from(gix_config::Source::Local).with(trust),
                    Default::default(),
                )?)?;
            let mut index = repo.open_index()?;
            index.entries_mut()[0].stat = Default::default();
            let mut status = repo
                .status(gix::progress::Discard)?
                .index(gix::worktree::IndexPersistedOrInMemory::InMemory(index))
                .into_iter(None)?;
            for item in status.by_ref() {
                item?;
            }
            let mut outcome = status.into_outcome().expect("status completed");
            assert!(outcome.has_changes(), "status refreshes the cleared index stat");
            outcome.write_changes().expect("the refreshed index can be written")?;
            assert_eq!(
                repo.open_index()?.checksum().is_none(),
                trust == gix_sec::Trust::Full,
                "only trusted configuration can omit checksums from refreshed indexes"
            );
        }
        Ok(())
    }

    #[test]
    fn conflict_paths_are_reported_once_like_git() -> TestResult {
        use gix::status::{
            UntrackedFiles, index_worktree::Item as WorktreeItem, plumbing::index_as_worktree::EntryStatus,
        };

        let root = gix_testtools::scripted_fixture_read_only("conflicts.sh")?;
        let baseline = std::fs::read_to_string(root.join(".git/baseline-status"))?;
        assert_eq!(
            baseline.lines().count(),
            29,
            "Git reports twenty-seven conflicts and two untracked files"
        );
        for ignore_case in [false, true] {
            let repo = gix::open_opts(
                &root,
                gix::open::Options::isolated().config_overrides([format!("core.ignoreCase={ignore_case}")]),
            )?;
            for untracked in [UntrackedFiles::Files, UntrackedFiles::Collapsed] {
                let items = repo
                    .status(gix::progress::Discard)?
                    .untracked_files(untracked)
                    .into_iter(None)?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                let mut actual = Vec::new();
                for item in items {
                    let code = match &item {
                        Item::IndexWorktree(WorktreeItem::Modification {
                            status: EntryStatus::Conflict { summary, .. },
                            ..
                        }) => summary.as_str(),
                        Item::IndexWorktree(WorktreeItem::DirectoryContents { entry, .. }) => {
                            assert_eq!(
                                entry.status,
                                gix::dir::entry::Status::Untracked,
                                "only untracked controls remain"
                            );
                            "??"
                        }
                        other => panic!("unexpected status item: {other:?}"),
                    };
                    actual.push(format!("{code} {}", item.location()));
                }
                actual.sort();
                let mut expected: Vec<_> = baseline.lines().map(str::to_owned).collect();
                expected.sort();
                assert_eq!(
                    actual, expected,
                    "each conflict appears once, with Git's status (ignore_case={ignore_case})"
                );
            }
        }
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn writing_stat_updates_honors_shared_repository_permissions() -> TestResult {
        use std::{fs, os::unix::fs::PermissionsExt};

        let fixture = gix_testtools::scripted_fixture_writable_with_args(
            "make_shared_repository.sh",
            ["0640", "common"],
            gix_testtools::Creation::CopyFromReadOnly,
        )?;
        let mut repo = gix::open_opts(fixture.path(), crate::restricted().strict_config(true))?;
        let mut config = repo.config_snapshot_mut();
        config.set_raw_value("core.sharedRepository", "0664")?;
        config.set_raw_value("core.deltaBaseCacheLimit", "invalid")?;
        let err = config.commit().expect_err("the object cache limit is invalid");
        assert!(
            err.to_string().contains("core.deltaBaseCacheLimit"),
            "the object cache limit causes the update to fail: {err}"
        );
        let mut index = repo.open_index()?;
        index.entries_mut()[0].stat = Default::default();
        index.write(Default::default(), gix::fs::SharedPermissions::Explicit(0o640))?;
        let mut status = repo.status(gix::progress::Discard)?.into_iter(None)?;
        for item in status.by_ref() {
            item?;
        }
        status
            .into_outcome()
            .expect("status iteration is complete")
            .write_changes()
            .expect("the stale stat data needs to be refreshed")?;
        assert_eq!(
            fs::metadata(repo.index_path())?.permissions().mode() & 0o777,
            0o640,
            "refreshing index stat data uses the sharing policy from before the failed update"
        );
        Ok(())
    }

    #[test]
    fn item_size() {
        let actual = std::mem::size_of::<Item>();
        let sha1 = 240;
        let sha256_extra = 56;
        let expected = sha1 + sha256_extra;
        assert!(
            size_ok(actual, expected),
            "The size is the same as the one for the index-worktree-item: {actual} <~ {expected}"
        );
    }

    #[test]
    fn submodule_tree_index_modification() -> TestResult {
        let repo = submodule_repo("git-mv-and-untracked-and-submodule-head-changed-and-modified")?;
        let mut status = repo
            .status(gix::progress::Discard)?
            .index_worktree_options_mut(|opts| {
                opts.sorting =
                    Some(gix::status::plumbing::index_as_worktree_with_renames::Sorting::ByPathCaseSensitive);
            })
            .tree_index_track_renames(TrackRenames::Given(Rewrites {
                track_empty: true,
                ..Default::default()
            }))
            .into_iter(None)?;
        let mut items: Vec<_> = status.by_ref().filter_map(std::result::Result::ok).collect();
        items.sort_by(|a, b| a.location().cmp(b.location()));
        assert_eq!(items.len(), 3, "1 untracked, 1 move, 1 submodule modification");
        let snapshot = gix_testtools::normalize_debug_snapshot(&&items[1..]).0;
        insta::assert_snapshot!(snapshot, @r#"
        [
            TreeIndex(
                Rewrite {
                    source_location: "this",
                    source_index: 2,
                    source_entry_mode: Mode(
                        FILE,
                    ),
                    source_id: Oid(1),
                    location: "that",
                    index: 2,
                    entry_mode: Mode(
                        FILE,
                    ),
                    id: Oid(1),
                    copy: false,
                },
            ),
            IndexWorktree(
                DirectoryContents {
                    entry: Entry {
                        rela_path: "untracked",
                        status: Untracked,
                        property: None,
                        disk_kind: Some(
                            File,
                        ),
                        index_kind: None,
                        pathspec_match: Some(
                            Always,
                        ),
                    },
                    collapsed_directory_status: None,
                },
            ),
        ]
        "#);
        Ok(())
    }

    #[test]
    fn submodule_fully_ignored_by_override() -> TestResult {
        let repo = submodule_repo("git-mv-and-untracked-and-submodule-head-changed-and-modified")?;
        let mut status = repo
            .status(gix::progress::Discard)?
            .index_worktree_options_mut(|opts| {
                opts.sorting =
                    Some(gix::status::plumbing::index_as_worktree_with_renames::Sorting::ByPathCaseSensitive);
            })
            .tree_index_track_renames(TrackRenames::Given(Rewrites {
                track_empty: true,
                ..Default::default()
            }))
            .index_worktree_submodules(Some(Submodule::Given {
                ignore: gix::submodule::config::Ignore::All,
                check_dirty: false,
            }))
            .into_iter(None)?;
        let mut items: Vec<_> = status.by_ref().filter_map(std::result::Result::ok).collect();
        items.sort_by(|a, b| a.location().cmp(b.location()));
        assert_eq!(
            items.len(),
            2,
            "1 untracked, 1 move, 0 submodule modification (ignored)"
        );
        let snapshot = gix_testtools::normalize_debug_snapshot(&items).0;
        insta::assert_snapshot!(snapshot, @r#"
        [
            TreeIndex(
                Rewrite {
                    source_location: "this",
                    source_index: 2,
                    source_entry_mode: Mode(
                        FILE,
                    ),
                    source_id: Oid(1),
                    location: "that",
                    index: 2,
                    entry_mode: Mode(
                        FILE,
                    ),
                    id: Oid(1),
                    copy: false,
                },
            ),
            IndexWorktree(
                DirectoryContents {
                    entry: Entry {
                        rela_path: "untracked",
                        status: Untracked,
                        property: None,
                        disk_kind: Some(
                            File,
                        ),
                        index_kind: None,
                        pathspec_match: Some(
                            Always,
                        ),
                    },
                    collapsed_directory_status: None,
                },
            ),
        ]
        "#);
        Ok(())
    }
    #[test]
    fn submodule_fully_ignored_by_configuration() -> TestResult {
        let repo = submodule_repo("git-mv-and-untracked-and-submodule-head-changed-and-modified-ignore-all")?;
        let mut status = repo
            .status(gix::progress::Discard)?
            .index_worktree_options_mut(|opts| {
                opts.sorting =
                    Some(gix::status::plumbing::index_as_worktree_with_renames::Sorting::ByPathCaseSensitive);
            })
            .tree_index_track_renames(TrackRenames::Given(Rewrites {
                track_empty: true,
                ..Default::default()
            }))
            .into_iter(None)?;
        let mut items: Vec<_> = status.by_ref().filter_map(std::result::Result::ok).collect();
        items.sort_by(|a, b| a.location().cmp(b.location()));
        assert_eq!(items.len(), 1, "1 untracked, 0 submodule modification (ignored)");
        insta::assert_debug_snapshot!(&items, @r#"
        [
            IndexWorktree(
                DirectoryContents {
                    entry: Entry {
                        rela_path: "untracked",
                        status: Untracked,
                        property: None,
                        disk_kind: Some(
                            File,
                        ),
                        index_kind: None,
                        pathspec_match: Some(
                            Always,
                        ),
                    },
                    collapsed_directory_status: None,
                },
            ),
        ]
        "#);
        Ok(())
    }

    #[test]
    fn tree_index_modification_worktree_modification_racy_git() -> TestResult {
        let repo = repo("racy-git")?;
        let mut status = repo.status(gix::progress::Discard)?.into_iter(None)?;
        let mut items: Vec<_> = status.by_ref().filter_map(std::result::Result::ok).collect();
        items.sort_by(|a, b| a.location().cmp(b.location()));
        assert_eq!(items.len(), 2, "1 modified in index, the same in worktree");
        Ok(())
    }

    #[test]
    fn untracked_unborn() -> TestResult {
        let repo = repo("untracked-unborn")?;
        let mut status = repo.status(gix::progress::Discard)?.into_iter(None)?;
        let mut items: Vec<_> = status.by_ref().filter_map(std::result::Result::ok).collect();
        items.sort_by(|a, b| a.location().cmp(b.location()));
        insta::assert_debug_snapshot!(items, @r#"
        [
            IndexWorktree(
                DirectoryContents {
                    entry: Entry {
                        rela_path: "untracked",
                        status: Untracked,
                        property: None,
                        disk_kind: Some(
                            File,
                        ),
                        index_kind: None,
                        pathspec_match: Some(
                            Always,
                        ),
                    },
                    collapsed_directory_status: None,
                },
            ),
        ]
        "#);
        Ok(())
    }

    #[test]
    fn added_unborn() -> TestResult {
        let repo = repo("added-unborn")?;
        let mut status = repo.status(gix::progress::Discard)?.into_iter(None)?;
        let mut items: Vec<_> = status.by_ref().filter_map(std::result::Result::ok).collect();
        items.sort_by(|a, b| a.location().cmp(b.location()));
        let snapshot = gix_testtools::normalize_debug_snapshot(&items).0;
        insta::assert_snapshot!(snapshot, @r#"
        [
            TreeIndex(
                Addition {
                    location: "added",
                    index: 0,
                    entry_mode: Mode(
                        FILE,
                    ),
                    id: Oid(1),
                },
            ),
            TreeIndex(
                Addition {
                    location: "dir/nested",
                    index: 1,
                    entry_mode: Mode(
                        FILE,
                    ),
                    id: Oid(1),
                },
            ),
        ]
        "#);
        Ok(())
    }

    #[test]
    fn untracked_added() -> TestResult {
        let repo = repo("untracked-added")?;
        let mut status = repo.status(gix::progress::Discard)?.into_iter(None)?;
        let mut items: Vec<_> = status.by_ref().filter_map(std::result::Result::ok).collect();
        items.sort_by(|a, b| a.location().cmp(b.location()));
        let snapshot = gix_testtools::normalize_debug_snapshot(&items).0;
        insta::assert_snapshot!(snapshot, @r#"
        [
            TreeIndex(
                Addition {
                    location: "added",
                    index: 0,
                    entry_mode: Mode(
                        FILE,
                    ),
                    id: Oid(1),
                },
            ),
        ]
        "#);
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn submodule_assume_unchanged_replaced_with_symlink_is_ignored() -> TestResult {
        let repo = repo("submodule-assume-unchanged-symlink")?;
        let mut status = repo.status(gix::progress::Discard)?.into_iter(None)?;
        let items: Vec<_> = status.by_ref().filter_map(std::result::Result::ok).collect();
        assert!(
            items.is_empty(),
            "assume-unchanged submodule should not show up in status"
        );
        Ok(())
    }

    #[test]
    fn uninitialized_submodule_with_files_is_clean() -> TestResult {
        let root = gix_testtools::scripted_fixture_read_only("make_uninitialized_submodule.sh")?.join("uninitialized");
        assert_eq!(
            gix_testtools::git(&root, "--no-optional-locks status --porcelain=v1 --untracked-files=all")?,
            "",
            "Git ignores files in an uninitialized submodule"
        );

        let repo = gix::open_opts(root, gix::open::Options::isolated())?;
        for untracked in [
            gix::status::UntrackedFiles::Collapsed,
            gix::status::UntrackedFiles::Files,
        ] {
            let items = repo
                .status(gix::progress::Discard)?
                .untracked_files(untracked)
                .into_iter(None)?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            assert!(
                items.is_empty(),
                "an uninitialized submodule must stay clean: {items:?}"
            );
        }
        Ok(())
    }

    #[test]
    #[cfg(any(unix, windows))]
    fn unconfigured_submodule_status_rejects_symlinks() -> TestResult {
        use gix::status::plumbing::index_as_worktree::traits::SubmoduleStatus;

        let root = gix_testtools::scripted_fixture_read_only("make_unlisted_gitlink_symlinks.sh")?;
        if !gix_testtools::fixture_has_symlinks(&root)? {
            return Ok(());
        }
        let repo = gix::open_opts(root, gix::open::Options::isolated())?;
        let index = repo.index()?;
        let entry = &index.entries()[0];
        let mut status = gix::status::index_worktree::BuiltinSubmoduleStatus::new(
            repo.into_sync(),
            Submodule::Given {
                ignore: gix::submodule::config::Ignore::None,
                check_dirty: false,
            },
        )?;
        assert!(
            status.status(entry, "d".into())?.is_some(),
            "the unconfigured gitlink's changed commit is reported"
        );
        for path in ["module-link", "alias/d", "alias/../d"] {
            let err = status
                .status(entry, path.into())
                .expect_err("the provider must validate paths independently of worktree traversal");
            assert!(err.is_validation(), "the symlink in {path} is rejected: {err:?}");
        }
        Ok(())
    }

    #[test]
    fn submodule_status_preserves_worktree_overrides() -> TestResult {
        use gix::status::plumbing::index_as_worktree::traits::SubmoduleStatus;

        let root = gix_testtools::scripted_fixture_read_only("make_unlisted_gitlinks.sh")?;
        for modules in ["absent", "listed"] {
            let mut repo = gix::open_opts(root.join(format!("{modules}-commit")), gix::open::Options::isolated())?;
            let index = repo.index()?;
            let entry = index.entry_by_path("d".into()).expect("the fixture has a gitlink");
            for (work_dir, expected_dirty) in [
                (Some(root.join(format!("{modules}-commit"))), true),
                (Some(root.join(format!("{modules}-clean"))), false),
                (None, false),
            ] {
                repo.set_workdir(work_dir.clone())?;
                let mut status = gix::status::index_worktree::BuiltinSubmoduleStatus::new(
                    repo.clone().into_sync(),
                    Submodule::Given {
                        ignore: gix::submodule::config::Ignore::None,
                        check_dirty: false,
                    },
                )?;
                assert_eq!(
                    status.status(entry, "d".into())?.is_some(),
                    expected_dirty,
                    "the provider uses the effective parent worktree, including None: {modules}, {work_dir:?}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn submodule_discovery_skips_missing_paths_but_propagates_invalid_ones() -> TestResult {
        for case in ["missing", "implicit", "empty", "absolute", "escaping"] {
            let repo = submodule_repo(&format!("path-{case}"))?;
            let status = gix::status::index_worktree::BuiltinSubmoduleStatus::new(
                repo.into_sync(),
                Submodule::AsConfigured { check_dirty: false },
            );
            if case == "missing" {
                status?;
            } else {
                let err = status.err().expect("invalid paths must fail discovery");
                assert!(err.is_validation(), "{case}: status preserves path-validation errors");
            }
        }
        Ok(())
    }

    #[test]
    fn submodule_status_preserves_missing_configured_worktree() -> TestResult {
        use gix::status::plumbing::index_as_worktree::traits::SubmoduleStatus;

        let root = gix_testtools::scripted_fixture_read_only("make_unlisted_gitlinks.sh")?;
        let repo = gix::open_opts(
            root.join("missing-worktree"),
            gix::open::Options::isolated()
                .filter_config_section(|_| true)
                .config_overrides(["diff.ignoreSubmodules=all"]),
        )?;
        assert!(
            !repo.workdir().expect("core.worktree is configured").exists(),
            "an eligible configuration section can select a missing checkout"
        );
        let index = repo.index()?;
        let entry = index.entry_by_path("d".into()).expect("the fixture has a gitlink");
        let mut status = gix::status::index_worktree::BuiltinSubmoduleStatus::new(
            repo.into_sync(),
            Submodule::AsConfigured { check_dirty: false },
        )?;
        assert!(
            status.status(entry, "d".into())?.is_none(),
            "configured ignore=all does not require the missing worktree to become accessible"
        );
        Ok(())
    }

    #[test]
    fn gitfile_dot_dot_target_configured_worktree_matches_git() -> TestResult {
        gitlinks_match_git(&["gitfile-worktree-config"])
    }

    #[test]
    fn gitfile_dot_dot_target_empty_worktree_config_is_reported() -> TestResult {
        use gix::status::plumbing::index_as_worktree::traits::SubmoduleStatus;

        let root = gix_testtools::scripted_fixture_read_only("make_unlisted_gitlinks.sh")?;
        for modules in ["absent", "unrelated", "listed"] {
            let worktree = root.join(format!("{modules}-gitfile-empty-worktree-config"));
            assert!(
                worktree.join("d/.git").is_file(),
                "{modules}: the checkout gitfile is present"
            );
            let repo = gix::open_opts(worktree, gix::open::Options::isolated())?;
            let index = repo.index()?;
            let entry = index.entry_by_path("d".into()).expect("the fixture has a gitlink");
            let mut status = gix::status::index_worktree::BuiltinSubmoduleStatus::new(
                repo.into_sync(),
                Submodule::Given {
                    ignore: gix::submodule::config::Ignore::None,
                    check_dirty: false,
                },
            )?;
            let err = status
                .status(entry, "d".into())
                .expect_err("empty core.worktree must propagate for configured and unconfigured gitlinks");
            assert!(
                err.is_validation(),
                "{modules}: empty core.worktree is invalid configuration: {err:?}"
            );
        }
        Ok(())
    }

    fn gitlinks_match_git(states: &[&str]) -> TestResult {
        use gix::status::plumbing::index_as_worktree::{Change, EntryStatus};
        use gix::submodule::config::Ignore;
        use std::fmt::Write;

        let root = gix_testtools::scripted_fixture_read_only("make_unlisted_gitlinks.sh")?;
        for modules in ["absent", "unrelated", "listed"] {
            for state in states {
                let repo = gix::open_opts(root.join(format!("{modules}-{state}")), gix::open::Options::isolated())?;
                for (name, ignore) in [
                    ("none", Ignore::None),
                    ("untracked", Ignore::Untracked),
                    ("dirty", Ignore::Dirty),
                    ("all", Ignore::All),
                ] {
                    let items = repo
                        .status(gix::progress::Discard)?
                        .index_worktree_submodules(Submodule::Given {
                            ignore,
                            check_dirty: false,
                        })
                        .into_iter(None)?
                        .collect::<gix::Result<Vec<_>>>()?;
                    let mut actual = String::new();
                    for item in items {
                        match item {
                            Item::IndexWorktree(gix::status::index_worktree::Item::Modification {
                                entry,
                                rela_path,
                                status: EntryStatus::Change(Change::SubmoduleModification(status)),
                                ..
                            }) => {
                                assert_eq!(status.index_id, Some(entry.id), "compare the indexed gitlink commit");
                                assert_eq!(status.is_dirty(), Some(true), "only dirty gitlinks are emitted");
                                writeln!(actual, " M {rela_path}")?;
                            }
                            other => panic!("unexpected status for {modules}-{state}: {other:?}"),
                        }
                    }
                    let expected = std::fs::read_to_string(repo.git_dir().join(format!("status-{name}.git")))?;
                    assert_eq!(actual, expected, "{modules}-{state} with ignore={name} agrees with Git");
                }
                let expected = std::fs::read_to_string(repo.git_dir().join("status-none.git"))?;
                assert_eq!(
                    repo.is_dirty()?,
                    !expected.is_empty(),
                    "is_dirty includes unlisted gitlinks"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn gitlinks_without_matching_modules_configuration_match_git() -> TestResult {
        gitlinks_match_git(&[
            "clean",
            "commit",
            "modified",
            "staged",
            "untracked",
            "gitfile",
            "gitfile-worktree-inference",
            "uninitialized",
        ])?;
        let root = gix_testtools::scripted_fixture_read_only("make_unlisted_gitlinks.sh")?;
        let repo = gix::open_opts(root.join("global-ignore"), gix::open::Options::isolated())?;
        assert!(
            std::fs::read(repo.git_dir().join("status-configured.git"))?.is_empty(),
            "Git honors the global ignore setting for unlisted gitlinks"
        );
        assert!(
            !repo.is_dirty()?,
            "the global ignore setting applies to unlisted gitlinks"
        );
        let reduced = || gix::open::Options::isolated().with_trust(gix::sec::Trust::Reduced);
        assert!(
            gix::open_opts(root.join("global-ignore"), reduced())?.is_dirty()?,
            "a reduced-trust ignore setting cannot hide a dirty gitlink"
        );
        assert!(
            !gix::open_opts(
                root.join("global-ignore"),
                reduced().config_overrides(["diff.ignoreSubmodules=all"]),
            )?
            .is_dirty()?,
            "an API ignore setting still applies to a reduced-trust repository"
        );
        for modules in ["absent", "listed"] {
            let repo = gix::open_opts(
                root.join(format!("{modules}-commit")),
                gix::open::Options::isolated().config_overrides(["diff.ignoreSubmodules=all"]),
            )?;
            assert!(
                std::fs::read(repo.git_dir().join("status-all.git"))?.is_empty(),
                "Git ignores submodule changes with the same setting"
            );
            assert!(
                !repo.is_dirty()?,
                "API open options must survive serial submodule status checks"
            );
            assert!(
                repo.status(gix::progress::Discard)?
                    .into_iter(None)?
                    .collect::<gix::Result<Vec<_>>>()?
                    .is_empty(),
                "the full status iterator must honor the caller's ignore override"
            );
        }
        Ok(())
    }

    #[test]
    #[cfg_attr(not(target_os = "macos"), ignore = "Needs Git's macOS Unicode precomposition")]
    fn emoji_with_decomposed_unicode_matches_git() -> TestResult {
        let root = gix_testtools::scripted_fixture_read_only_needs_archive("make_emoji_with_decomposed_unicode.sh")?;
        assert_eq!(
            gix_testtools::git(&root, "--no-optional-locks status --porcelain=v1 --untracked-files=all")?,
            "",
            "the filenames from the report are tracked and clean in Git"
        );

        let repo = gix::open_opts(root, gix::open::Options::isolated())?;
        let items = repo
            .status(gix::progress::Discard)?
            .into_iter(None)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        assert!(
            items.is_empty(),
            "Unicode precomposition must agree with Git: {items:?}"
        );
        Ok(())
    }

    mod progress_hook {
        pub(super) enum Hook {
            Init,
            Drop,
        }

        pub(super) struct Progress(pub(super) Box<dyn FnMut(Hook) + Send + Sync>);

        impl gix::progress::Count for Progress {
            fn set(&self, _: usize) {}
            fn step(&self) -> usize {
                0
            }
            fn inc_by(&self, _: usize) {}
            fn counter(&self) -> gix::progress::StepShared {
                gix::progress::Discard.counter()
            }
        }

        impl gix::progress::Progress for Progress {
            fn init(&mut self, _: Option<usize>, _: Option<gix::progress::Unit>) {
                (self.0)(Hook::Init);
            }
            fn set_name(&mut self, _: String) {}
            fn name(&self) -> Option<String> {
                None
            }
            fn id(&self) -> gix::progress::Id {
                gix::progress::UNKNOWN
            }
            fn message(&self, _: gix::progress::MessageLevel, _: String) {}
        }

        impl Drop for Progress {
            fn drop(&mut self) {
                (self.0)(Hook::Drop);
            }
        }
    }

    #[test]
    #[cfg(not(feature = "parallel"))]
    fn serial_status_cancels_before_filtering_queued_rewrites() -> TestResult {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };

        // Other packages in a workspace test can still enable parallel plumbing through feature unification.
        if gix::parallel::num_threads(Some(2)) != 1 {
            return Ok(());
        }
        let fixture = gix_testtools::scripted_fixture_read_only("status-cancel-queued-rewrite.sh")?;
        let repo = gix::open_opts(fixture, gix::open::Options::isolated())?;
        let interrupt = Arc::new(AtomicBool::new(false));
        let scan = |repo: &gix::Repository, progress| {
            repo.status(progress)?
                .untracked_files(gix::status::UntrackedFiles::Files)
                .index_worktree_rewrites(Some(gix_diff::Rewrites::default()))
                .should_interrupt_owned(Arc::clone(&interrupt))
                .into_iter(None)
        };
        let filter_failed = |err: &gix::Error| {
            err.iter_errors()
                .any(|cause| cause.to_string().contains("Driver process failed"))
        };
        let err = scan(&repo, progress_hook::Progress(Box::new(|_| {})))
            .err()
            .expect("the queued candidate's required filter fails in an uninterrupted scan");
        assert!(
            filter_failed(&err),
            "the fixture exercises the required filter: {err:?}"
        );

        let cancel_during_tracked_scan = || {
            let interrupt = Arc::clone(&interrupt);
            progress_hook::Progress(Box::new(move |hook| {
                if matches!(hook, progress_hook::Hook::Init) {
                    interrupt.store(true, Ordering::SeqCst);
                }
            }))
        };
        let err = scan(&repo, cancel_during_tracked_scan())
            .err()
            .expect("the tracked-file scan requested cancellation");
        assert!(err.is_cancelled(), "expected cancellation, got {err:?}");
        assert!(!filter_failed(&err), "the queued filter must be skipped: {err:?}");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let (fixture, ()) =
                gix_testtools::scripted_fixture_read_only_with_post("status-cancel-queued-rewrite.sh", 1, |state| {
                    if let gix_testtools::FixtureState::Uninitialized(path) = state {
                        let ignore = path.join(".gitignore");
                        std::fs::write(&ignore, "ignored\n")?;
                        std::fs::set_permissions(ignore, std::fs::Permissions::from_mode(0o000))?;
                    }
                    Ok(())
                })?;
            // Privileged users can still read this fixture.
            if let Err(err) = std::fs::File::open(fixture.join(".gitignore")) {
                assert_eq!(
                    err.kind(),
                    std::io::ErrorKind::PermissionDenied,
                    "the ignore file must fail with a permission error"
                );
                let repo = gix::open_opts(fixture, gix::open::Options::isolated())?;
                interrupt.store(false, Ordering::SeqCst);
                let err = scan(&repo, cancel_during_tracked_scan())
                    .err()
                    .expect("the directory walk fails before the tracked-file scan requests cancellation");
                assert!(err.is_cancelled(), "expected cancellation, got {err:?}");
                assert!(
                    err.is_permission_denied(),
                    "cancellation retains the directory-walk error: {err:?}"
                );
            }
        }
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn status_cancels_during_a_clean_filter() -> TestResult {
        use std::{
            sync::{
                Arc,
                atomic::{AtomicBool, Ordering},
            },
            thread,
            time::{Duration, Instant},
        };

        let gate = gix_testtools::tempfile::tempdir()?;
        let quoted = gix::quote::single(gix::path::into_bstr(gate.path())?.as_ref());
        let finish = if cfg!(feature = "parallel") { "cat" } else { "exit 7" };
        let clean = format!(
            "gate={quoted}; : > \"$gate/started\"; \
             while [ -d \"$gate\" ] && [ ! -f \"$gate/release\" ]; do sleep 0.001; done; {finish}"
        );
        let fixture = gix_testtools::scripted_fixture_read_only("status-cancel-during-filter.sh")?;
        let repo = gix::open_opts(
            fixture,
            gix::open::Options::isolated()
                .config_overrides([format!("filter.gate.clean={clean}"), "filter.gate.required=true".into()]),
        )?;
        let mut index = repo.open_index()?;
        let file = index
            .entry_index_by_path("file".into())
            .expect("the fixture tracks file");
        index.entries_mut()[file].stat = Default::default();
        let interrupt = Arc::new(AtomicBool::new(false));
        let scan = repo
            .status(gix::progress::Discard)?
            .index(gix::worktree::IndexPersistedOrInMemory::InMemory(index))
            .untracked_files(gix::status::UntrackedFiles::None)
            .should_interrupt_owned(Arc::clone(&interrupt));

        let wait_for_start = || {
            let started = gate.path().join("started");
            let deadline = Instant::now() + Duration::from_secs(5);
            while !started.is_file() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(1));
            }
            started.is_file()
        };
        let cancel_filter = || -> std::io::Result<_> {
            interrupt.store(true, Ordering::SeqCst);
            std::fs::File::create(gate.path().join("release"))
        };
        #[cfg(not(feature = "parallel"))]
        let err = thread::scope(|scope| -> TestResult<_> {
            let cancel = scope.spawn(|| -> TestResult {
                let filter_started = wait_for_start();
                cancel_filter()?;
                assert!(filter_started, "status must start the clean filter before cancellation");
                Ok(())
            });
            let result = scan.into_index_worktree_iter(None);
            cancel.join().expect("the canceller completed")?;
            Ok(result.err().expect("the clean filter failed after cancellation"))
        })?;
        #[cfg(feature = "parallel")]
        let err = thread::scope(|scope| -> TestResult<_> {
            let mut iter = scan.into_index_worktree_iter(None)?;
            assert!(wait_for_start(), "the filter is in flight before polling the iterator");
            let cancel = scope.spawn(|| {
                // Give next() time to wait for the item produced by this filter.
                thread::sleep(Duration::from_millis(5));
                cancel_filter()
            });
            let item = iter.next();
            cancel.join().expect("the canceller completed")?;
            let err = item
                .expect("the iterator reports cancellation")
                .expect_err("cancellation takes precedence over the in-flight change");
            assert!(iter.next().is_none(), "cancellation is reported once");
            assert!(iter.into_outcome().is_none(), "a cancelled scan has no outcome");
            Ok(err)
        })?;
        assert!(err.is_cancelled(), "expected cancellation, got {err:?}");
        #[cfg(not(feature = "parallel"))]
        assert!(
            err.iter_errors()
                .any(|cause| cause.to_string().contains("Driver process failed")),
            "the required filter failure must remain in the error chain: {err:?}"
        );
        Ok(())
    }

    #[test]
    fn error_during_tree_traversal_causes_failure() -> TestResult {
        let mut error_snapshots = Vec::new();
        let repo = repo("untracked-only")?;
        let invalid_tree_id = repo.object_hash().empty_blob();
        let platform = repo.status(gix::progress::Discard)?.head_tree(invalid_tree_id);
        if cfg!(feature = "parallel") {
            let mut items: Vec<_> = platform.into_iter(None)?.collect();
            assert_eq!(
                items.len(),
                3,
                "2 untracked and one error, which is detected only in the end."
            );
            error_snapshots.push(gix_testtools::redact_debug_snapshot(
                &(items.pop().expect("last item").unwrap_err()),
                &[],
            ));
        } else {
            match platform.into_iter(None) {
                Ok(_) => {
                    unreachable!("errors would be detected early here as everything is done ahead of time")
                }
                Err(err) => {
                    error_snapshots.push(gix_testtools::redact_debug_snapshot(&(err), &[]));
                }
            }
        }
        insta::assert_debug_snapshot!(error_snapshots, "error during tree traversal causes failure", @"
        [
            Could not create index from tree at Oid(1)
            
            Caused by:
                0: Expected object of kind tree but got blob at Oid(1),
        ]
        ");
        #[cfg(not(feature = "parallel"))]
        {
            use std::sync::{
                Arc,
                atomic::{AtomicBool, Ordering},
            };

            let interrupt = Arc::new(AtomicBool::new(false));
            let patterns = std::iter::from_fn(|| {
                interrupt.store(true, Ordering::SeqCst);
                None
            });
            let err = repo
                .status(gix::progress::Discard)?
                .head_tree(invalid_tree_id)
                .should_interrupt_owned(Arc::clone(&interrupt))
                .into_iter(patterns)
                .err()
                .expect("the tree scan fails after pattern collection requests cancellation");
            assert!(err.is_cancelled(), "expected cancellation, got {err:?}");
            assert!(
                err.iter_errors()
                    .any(|cause| cause.to_string().contains("Could not create index from tree")),
                "cancellation retains the tree scan failure: {err:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn pre_interrupted_status_returns_cancellation() -> TestResult {
        use std::sync::{Arc, atomic::AtomicBool};

        let repo = repo("racy-git")?;
        for untracked in [gix::status::UntrackedFiles::None, gix::status::UntrackedFiles::Files] {
            let res = repo
                .status(gix::progress::Discard)?
                .untracked_files(untracked)
                .should_interrupt_owned(Arc::new(AtomicBool::new(true)))
                .into_iter(None);
            let err = if cfg!(feature = "parallel") {
                let mut iter = res?;
                let err = iter
                    .next()
                    .expect("cancellation is reported before any status items")
                    .expect_err("a pre-interrupted scan cannot yield status items");
                assert!(iter.next().is_none(), "cancellation is reported only once");
                assert!(iter.into_outcome().is_none(), "a cancelled scan has no outcome");
                err
            } else {
                res.err().expect("serial status reports cancellation from into_iter()")
            };
            assert!(err.is_cancelled(), "expected cancellation, got {err:?}");
        }
        Ok(())
    }

    #[test]
    #[cfg(feature = "parallel")]
    fn stopping_status_preserves_caller_flags() -> TestResult {
        use std::{
            sync::{
                Arc, Mutex,
                atomic::{AtomicBool, Ordering},
                mpsc,
            },
            time::Duration,
        };

        use gix::progress::{Count, Id, MessageLevel, Progress, Step, StepShared, UNKNOWN, Unit};

        struct PausedProgress {
            resume: Mutex<mpsc::Receiver<()>>,
            finished: mpsc::Sender<()>,
            count: StepShared,
        }

        impl Progress for PausedProgress {
            fn init(&mut self, max: Option<Step>, _unit: Option<Unit>) {
                assert_eq!(max, Some(1), "the worker has one tracked entry to check");
                self.resume
                    .get_mut()
                    .expect("the worker has exclusive access to progress")
                    .recv_timeout(Duration::from_secs(5))
                    .ok();
            }
            fn set_name(&mut self, _name: String) {}
            fn name(&self) -> Option<String> {
                None
            }
            fn id(&self) -> Id {
                UNKNOWN
            }
            fn message(&self, _level: MessageLevel, _message: String) {}
        }

        impl Count for PausedProgress {
            fn set(&self, step: Step) {
                self.count.store(step, Ordering::Relaxed);
            }
            fn step(&self) -> Step {
                self.count.load(Ordering::Relaxed)
            }
            fn inc_by(&self, step: Step) {
                self.count.fetch_add(step, Ordering::Relaxed);
            }
            fn counter(&self) -> StepShared {
                Arc::clone(&self.count)
            }
        }

        impl Drop for PausedProgress {
            fn drop(&mut self) {
                self.finished.send(()).ok();
            }
        }

        #[derive(Debug)]
        enum Stop {
            DropWithOwnedFlag,
            DropWithSharedFlag,
            InterruptWhilePaused,
            InterruptBeforeDraining,
        }

        static SHARED: AtomicBool = AtomicBool::new(false);
        let repo = repo("racy-git")?;
        for stop in [
            Stop::DropWithOwnedFlag,
            Stop::DropWithSharedFlag,
            Stop::InterruptWhilePaused,
            Stop::InterruptBeforeDraining,
        ] {
            let owned = Arc::new(AtomicBool::new(false));
            let (resume, paused) = mpsc::channel();
            let (finished, worker) = mpsc::channel();
            let count = StepShared::default();
            let platform = repo
                .status(PausedProgress {
                    resume: Mutex::new(paused),
                    finished,
                    count: Arc::clone(&count),
                })?
                .untracked_files(gix::status::UntrackedFiles::None);
            let platform = if matches!(stop, Stop::DropWithSharedFlag) {
                platform.should_interrupt_shared(&SHARED)
            } else {
                platform.should_interrupt_owned(Arc::clone(&owned))
            };
            let mut iter = platform.into_iter(None)?;
            assert!(
                matches!(iter.next().transpose()?, Some(Item::TreeIndex(_))),
                "the staged change is available while the tracked-file worker is paused"
            );
            let resume_and_wait = || -> TestResult {
                resume.send(())?;
                worker.recv_timeout(Duration::from_secs(5))?;
                Ok(())
            };
            let interrupt = matches!(stop, Stop::InterruptWhilePaused | Stop::InterruptBeforeDraining);
            let wait_before_draining = matches!(stop, Stop::InterruptBeforeDraining);
            if interrupt {
                owned.store(true, Ordering::SeqCst);
                if wait_before_draining {
                    // Let the caller's flag stop the scan before next() can request private shutdown.
                    resume_and_wait()?;
                }
                let err = iter
                    .next()
                    .expect("cancellation is reported regardless of whether the worker has finished")
                    .expect_err("the interrupted scan must not end successfully");
                assert!(err.is_cancelled(), "expected cancellation, got {err:?}");
                assert!(iter.next().is_none(), "cancellation is reported only once");
                assert!(iter.into_outcome().is_none(), "a cancelled scan has no outcome");
            } else {
                drop(iter);
            }
            if !wait_before_draining {
                resume_and_wait()?;
            }
            assert_eq!(
                count.load(Ordering::Relaxed),
                0,
                "the scan stopped before checking tracked entries ({stop:?})"
            );
            assert!(
                !SHARED.load(Ordering::Relaxed),
                "dropping the iterator leaves the shared flag clear"
            );
            assert_eq!(
                owned.load(Ordering::Relaxed),
                interrupt,
                "the caller's flag indicates whether it requested interruption ({stop:?})"
            );
        }
        Ok(())
    }

    #[test]
    #[cfg(feature = "parallel")]
    fn status_worker_exit_without_a_result_is_an_error() -> TestResult {
        let progress = progress_hook::Progress(Box::new(|hook| {
            if matches!(hook, progress_hook::Hook::Init) {
                panic!("the worker exited before publishing its result");
            }
        }));
        let repo = repo("untracked-unborn")?;
        let mut iter = repo
            .status(progress)?
            .untracked_files(gix::status::UntrackedFiles::None)
            .into_index_worktree_iter(None)?;
        let err = iter
            .next()
            .expect("worker exit is reported")
            .expect_err("the scan failed");
        assert!(
            err.to_string().contains("exited without reporting its result"),
            "expected a missing worker result, got {err:?}"
        );
        assert!(iter.next().is_none(), "the error is reported once");
        assert!(iter.into_outcome().is_none(), "an incomplete scan has no outcome");
        Ok(())
    }

    #[test]
    #[cfg(feature = "parallel")]
    fn status_completes_while_worktree_progress_is_dropping() -> TestResult {
        use std::{
            sync::{Mutex, mpsc},
            thread,
            time::Duration,
        };

        let repo = repo("untracked-unborn")?;
        for invalid_tree in [true, false] {
            let (finishing, wait_for_finishing) = mpsc::channel();
            let (release, wait_for_release) = mpsc::channel::<()>();
            let mut wait_for_release = Mutex::new(wait_for_release);
            let progress = progress_hook::Progress(Box::new(move |hook| {
                if matches!(hook, progress_hook::Hook::Drop) {
                    finishing.send(()).ok();
                    wait_for_release.get_mut().expect("exclusive access").recv().ok();
                }
            }));
            let mut platform = repo
                .status(progress)?
                .untracked_files(gix::status::UntrackedFiles::None);
            if invalid_tree {
                platform = platform.head_tree(repo.object_hash().empty_blob());
            }
            let mut iter = platform.into_iter(None)?;
            wait_for_finishing.recv_timeout(Duration::from_secs(5))?;

            let (returned, wait_for_next) = mpsc::channel();
            thread::scope(|scope| {
                let release_cleanup = scope.spawn(move || {
                    let returned = wait_for_next.recv_timeout(Duration::from_secs(2));
                    drop(release);
                    returned
                });
                let item = iter.next();
                returned.send(()).ok();
                release_cleanup
                    .join()
                    .expect("helper thread completed")
                    .expect("next() must return even while worktree cleanup is blocked");
                if invalid_tree {
                    let err = item
                        .expect("the iterator reports the tree error")
                        .expect_err("there are no status entries to report");
                    assert!(
                        err.to_string().contains("Could not create index from tree"),
                        "expected the tree error, got {err:?}"
                    );
                    assert!(iter.next().is_none(), "the error is reported once");
                    assert!(iter.into_outcome().is_none(), "a failed scan has no outcome");
                } else {
                    assert!(item.is_none(), "there are no status entries to report");
                    assert!(iter.into_outcome().is_some(), "both scans completed successfully");
                }
            });
        }
        Ok(())
    }

    #[test]
    #[cfg(not(feature = "parallel"))]
    fn serial_status_reports_cancellation_during_into_iter() -> TestResult {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };

        let repo = repo("racy-git")?;
        let interrupt = Arc::new(AtomicBool::new(false));
        let res = repo
            .status(gix::progress::Discard)?
            .untracked_files(gix::status::UntrackedFiles::None)
            .should_interrupt_owned(Arc::clone(&interrupt))
            .into_iter(std::iter::once_with(|| {
                interrupt.store(true, Ordering::SeqCst);
                gix::bstr::BString::from(".")
            }));
        let err = res.err().expect("status must not return a partially computed scan");
        assert!(err.is_cancelled(), "expected cancellation, got {err:?}");
        Ok(())
    }

    #[test]
    #[cfg(not(feature = "parallel"))]
    fn serial_status_preserves_results_when_interrupted_after_the_scan() -> TestResult {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };

        let repo = repo("racy-git")?;
        let interrupt = Arc::new(AtomicBool::new(false));
        let mut iter = repo
            .status(gix::progress::Discard)?
            .untracked_files(gix::status::UntrackedFiles::None)
            .should_interrupt_owned(Arc::clone(&interrupt))
            .into_iter(None)?;
        interrupt.store(true, Ordering::Relaxed);
        let items: Vec<_> = iter.by_ref().collect::<gix::Result<_>>()?;
        assert!(
            matches!(items.as_slice(), [Item::TreeIndex(_), Item::IndexWorktree(_)]),
            "the staged and unstaged changes were already computed"
        );
        let outcome = iter.into_outcome().expect("the completed scan has an outcome");
        assert_eq!(outcome.index_worktree.tracked_file_modification.entries_processed, 1);
        Ok(())
    }
}

mod index_worktree {
    mod iter {
        use gix::status::index_worktree::Item;
        use gix_testtools::TestResult;
        use gix_testtools::size_ok;
        use pretty_assertions::assert_eq;

        use crate::status::{repo, submodule_repo};

        #[test]
        #[cfg(unix)]
        fn status_reads_only_required_attribute_and_ignore_files() -> TestResult {
            fn assert_symlink(path: &std::path::Path) -> TestResult {
                assert!(
                    path.symlink_metadata()?.file_type().is_symlink(),
                    "{} must be an unreadable symlink loop",
                    path.display()
                );
                Ok(())
            }

            let fixture = gix_testtools::scripted_fixture_read_only("make_status_attribute_and_ignore_repos.sh")?;
            if !gix_testtools::fixture_has_symlinks(&fixture)? {
                return Ok(());
            }
            let tracked_root = fixture.join("tracked");
            assert_symlink(&tracked_root.join(".gitignore"))?;
            let tracked = gix::open_opts(tracked_root, gix::open::Options::isolated())?;
            for (pattern, expected_count) in [("file", 1), (":(attr:visible)*", 1), (":(attr:missing)*", 0)] {
                let items = tracked
                    .status(gix::progress::Discard)?
                    .untracked_files(gix::status::UntrackedFiles::None)
                    .into_index_worktree_iter([pattern.into()])?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                assert_eq!(
                    items.len(),
                    expected_count,
                    "tracked status needs attributes for pathspecs and filters, but never ignores: {pattern}"
                );
                if let Some(item) = items.first() {
                    assert_eq!(item.rela_path(), "file", "the changed tracked file is selected");
                    assert_eq!(
                        item.summary(),
                        Some(gix::status::index_worktree::iter::Summary::Modified),
                        "the equal-sized content change is detected"
                    );
                }
            }

            let untracked_root = fixture.join("untracked");
            assert_symlink(&untracked_root.join(".gitattributes"))?;
            let untracked = gix::open_opts(untracked_root, gix::open::Options::isolated())?;
            let items = untracked
                .status(gix::progress::Discard)?
                .into_index_worktree_iter(None)?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            assert_eq!(
                items.len(),
                1,
                "untracked status applies ignores without reading attributes"
            );
            assert_eq!(items[0].rela_path(), "untracked", "only the unignored file is reported");
            Ok(())
        }

        #[test]
        fn item_size() {
            let actual = std::mem::size_of::<Item>();
            let sha1 = 240;
            let sha256_extra = 56;
            let expected = sha1 + sha256_extra;
            assert!(
                size_ok(actual, expected),
                "The size is pretty huge and goes down ideally: {actual} <~ {expected}"
            );
        }

        #[test]
        fn submodule_modification() -> TestResult {
            let repo = submodule_repo("modified-untracked-and-submodule-head-changed-and-modified")?;
            let mut status = repo
                .status(gix::progress::Discard)?
                .index_worktree_options_mut(|opts| {
                    opts.sorting =
                        Some(gix::status::plumbing::index_as_worktree_with_renames::Sorting::ByPathCaseSensitive);
                })
                .into_index_worktree_iter(None)?;
            let items: Vec<_> = status.by_ref().filter_map(std::result::Result::ok).collect();
            assert_eq!(items.len(), 3, "1 untracked, 1 modified file, 1 submodule modification");
            Ok(())
        }

        #[test]
        fn untracked_files_collapse_by_default() -> TestResult {
            let repo = repo("untracked-only")?;
            let status = repo
                .status(gix::progress::Discard)?
                .index_worktree_options_mut(|opts| {
                    opts.sorting =
                        Some(gix::status::plumbing::index_as_worktree_with_renames::Sorting::ByPathCaseSensitive);
                })
                .into_index_worktree_iter(None)?;
            let items: Vec<_> = status.filter_map(std::result::Result::ok).collect();
            assert_eq!(
                items,
                [
                    Item::DirectoryContents {
                        entry: gix_dir::Entry {
                            rela_path: "new".into(),
                            status: gix_dir::entry::Status::Untracked,
                            property: None,
                            disk_kind: Some(gix_dir::entry::Kind::Directory),
                            index_kind: None,
                            pathspec_match: Some(gix_dir::entry::PathspecMatch::Always),
                        },
                        collapsed_directory_status: None
                    },
                    Item::DirectoryContents {
                        entry: gix_dir::Entry {
                            rela_path: "subdir/untracked".into(),
                            status: gix_dir::entry::Status::Untracked,
                            property: None,
                            disk_kind: Some(gix_dir::entry::Kind::File),
                            index_kind: None,
                            pathspec_match: Some(gix_dir::entry::PathspecMatch::Always),
                        },
                        collapsed_directory_status: None
                    }
                ],
                "'new/untracked' gets collapsed, but the second untracked is in a folder with a tracked file.\
                This collapsing behaviour is the default."
            );
            Ok(())
        }

        #[test]
        fn untracked_files_settings_none() -> TestResult {
            let mut repo = repo("untracked-only")?;
            repo.config_snapshot_mut()
                .set_value(&gix::config::tree::Status::SHOW_UNTRACKED_FILES, "no")?;

            let mut status = repo
                .status(gix::progress::Discard)?
                .index_worktree_options_mut(|opts| {
                    opts.sorting =
                        Some(gix::status::plumbing::index_as_worktree_with_renames::Sorting::ByPathCaseSensitive);
                })
                .into_index_worktree_iter(None)?;
            let items: Vec<_> = status.by_ref().filter_map(std::result::Result::ok).collect();
            assert_eq!(items, [], "no untracked files are found…");
            assert_eq!(
                status.outcome_mut().expect("iteration done").index_worktree.dirwalk,
                None,
                "…as there was no directory walk"
            );
            Ok(())
        }

        #[test]
        fn early_drop_for_is_dirty_emulation() -> TestResult {
            let repo = submodule_repo("modified-untracked-and-submodule-head-changed-and-modified")?;
            let is_dirty = repo
                .status(gix::progress::Discard)?
                .index_worktree_submodules(gix::status::Submodule::AsConfigured { check_dirty: true })
                .index_worktree_options_mut(|opts| {
                    opts.sorting =
                        Some(gix::status::plumbing::index_as_worktree_with_renames::Sorting::ByPathCaseSensitive);
                })
                .into_index_worktree_iter(None)?
                .next()
                .is_some();
            assert!(is_dirty, "this should abort the work as quickly as possible");
            Ok(())
        }

        #[test]
        #[cfg(unix)] // symlinks are used here, let's not try our luck on Windows.
        fn tracked_entries_under_a_symlinked_directory_are_reported_as_removed() -> TestResult {
            let repo = repo("symlink-replaces-tracked-dir")?;
            let mut items: Vec<_> = repo
                .status(gix::progress::Discard)?
                .into_index_worktree_iter(None)?
                .collect::<std::result::Result<_, _>>()?;
            items.sort_by(|a, b| a.rela_path().cmp(b.rela_path()));
            let idx = 2;
            assert_eq!(items[idx].rela_path(), "tracked/file");
            assert_eq!(
                items[idx].summary(),
                Some(gix::status::index_worktree::iter::Summary::Removed)
            );
            // Remove the entry with stat information which can't be stable.
            items.remove(idx);
            // We correctly identify the `tracked` folder as tracked, knowing that in the index
            // it's a directory, and locally it's a symlink.
            insta::assert_debug_snapshot!(&items, @r#"
            [
                DirectoryContents {
                    entry: Entry {
                        rela_path: "target",
                        status: Untracked,
                        property: None,
                        disk_kind: Some(
                            Directory,
                        ),
                        index_kind: None,
                        pathspec_match: Some(
                            Always,
                        ),
                    },
                    collapsed_directory_status: None,
                },
                DirectoryContents {
                    entry: Entry {
                        rela_path: "tracked",
                        status: Untracked,
                        property: None,
                        disk_kind: Some(
                            Symlink,
                        ),
                        index_kind: Some(
                            Directory,
                        ),
                        pathspec_match: Some(
                            Always,
                        ),
                    },
                    collapsed_directory_status: None,
                },
            ]
            "#);
            Ok(())
        }
    }
}

mod is_dirty {
    use crate::status::{repo, submodule_repo};
    use gix_testtools::TestResult;

    #[test]
    fn various_changes_positive() -> TestResult {
        let repo = submodule_repo("modified-untracked-and-submodule-head-changed-and-modified")?;
        assert!(repo.is_dirty()?, "The repository has various changes");
        Ok(())
    }

    #[test]
    fn submodule_changes_are_picked_up() -> TestResult {
        let repo = submodule_repo("submodule-head-changed")?;
        assert!(repo.is_dirty()?, "head-changes are also discovered");
        Ok(())
    }

    #[test]
    fn untracked_files_are_excluded() -> TestResult {
        let repo = submodule_repo("module1")?;
        assert_eq!(
            repo.status(gix::progress::Discard)?
                .into_index_worktree_iter(None)?
                .count(),
            1,
            "there is one untracked file"
        );
        assert!(
            !repo.is_dirty()?,
            "untracked files aren't taken into consideration, just like `git describe` which ignores them"
        );
        Ok(())
    }

    #[test]
    fn unborn_head_is_not_an_error() -> TestResult {
        let repo = repo("untracked-unborn")?;

        assert!(
            !repo.is_dirty()?,
            "untracked files aren't taken into consideration, even if HEAD is unborn"
        );
        Ok(())
    }

    #[test]
    fn added_files_in_unborn_head_are_dirty() -> TestResult {
        let repo = repo("added-unborn")?;

        assert!(
            repo.is_dirty()?,
            "files added to the index make an unborn repository dirty"
        );
        Ok(())
    }

    #[test]
    fn index_changed() -> TestResult {
        let repo = repo("git-mv")?;
        assert!(
            repo.is_dirty()?,
            "the only detectable change is in the index, in comparison to the HEAD^{{tree}}"
        );

        let repo = submodule_repo("with-submodules")?;
        assert!(
            repo.is_dirty()?,
            "the index changed here as well, this time there is also a new file"
        );
        Ok(())
    }
}
