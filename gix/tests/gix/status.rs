pub fn submodule_repo(name: &str) -> crate::Result<gix::Repository> {
    use crate::util::named_subrepo_opts;
    Ok(named_subrepo_opts(
        "make_submodules.sh",
        name,
        gix::open::Options::isolated(),
    )?)
}

pub fn repo(name: &str) -> crate::Result<gix::Repository> {
    use crate::util::named_subrepo_opts;
    Ok(named_subrepo_opts(
        "make_status_repos.sh",
        name,
        gix::open::Options::isolated(),
    )?)
}

mod into_iter {
    use gix::status::{tree_index::TrackRenames, Item, Submodule};
    use gix_diff::Rewrites;
    use gix_testtools::size_ok;

    use crate::{
        status::{repo, submodule_repo},
        util::hex_to_id,
    };

    #[test]
    fn item_size() {
        let actual = std::mem::size_of::<Item>();
        let expected = 264;
        assert!(
            size_ok(actual, expected),
            "The size is the same as the one for the index-worktree-item: {actual} <~ {expected}"
        );
    }

    #[test]
    fn submodule_tree_index_modification() -> crate::Result {
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
        let mut items: Vec<_> = status.by_ref().filter_map(Result::ok).collect();
        items.sort_by(|a, b| a.location().cmp(b.location()));
        assert_eq!(items.len(), 3, "1 untracked, 1 move, 1 submodule modification");
        insta::assert_debug_snapshot!(&items[1..], @r#"
        [
            TreeIndex(
                Rewrite {
                    source_location: "this",
                    source_index: 2,
                    source_entry_mode: Mode(
                        FILE,
                    ),
                    source_id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
                    location: "that",
                    index: 2,
                    entry_mode: Mode(
                        FILE,
                    ),
                    id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
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
    fn submodule_fully_ignored_by_override() -> crate::Result {
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
        let mut items: Vec<_> = status.by_ref().filter_map(Result::ok).collect();
        items.sort_by(|a, b| a.location().cmp(b.location()));
        assert_eq!(
            items.len(),
            2,
            "1 untracked, 1 move, 0 submodule modification (ignored)"
        );
        insta::assert_debug_snapshot!(&items, @r#"
        [
            TreeIndex(
                Rewrite {
                    source_location: "this",
                    source_index: 2,
                    source_entry_mode: Mode(
                        FILE,
                    ),
                    source_id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
                    location: "that",
                    index: 2,
                    entry_mode: Mode(
                        FILE,
                    ),
                    id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
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
    fn submodule_fully_ignored_by_configuration() -> crate::Result {
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
        let mut items: Vec<_> = status.by_ref().filter_map(Result::ok).collect();
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
    fn tree_index_modification_worktree_modification_racy_git() -> crate::Result {
        let repo = repo("racy-git")?;
        let mut status = repo.status(gix::progress::Discard)?.into_iter(None)?;
        let mut items: Vec<_> = status.by_ref().filter_map(Result::ok).collect();
        items.sort_by(|a, b| a.location().cmp(b.location()));
        assert_eq!(items.len(), 2, "1 modified in index, the same in worktree");
        Ok(())
    }

    #[test]
    fn untracked_unborn() -> crate::Result {
        let repo = repo("untracked-unborn")?;
        let mut status = repo.status(gix::progress::Discard)?.into_iter(None)?;
        let mut items: Vec<_> = status.by_ref().filter_map(Result::ok).collect();
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
    fn untracked_added() -> crate::Result {
        let repo = repo("untracked-added")?;
        let mut status = repo.status(gix::progress::Discard)?.into_iter(None)?;
        let mut items: Vec<_> = status.by_ref().filter_map(Result::ok).collect();
        items.sort_by(|a, b| a.location().cmp(b.location()));
        insta::assert_debug_snapshot!(items, @r#"
        [
            TreeIndex(
                Addition {
                    location: "added",
                    index: 0,
                    entry_mode: Mode(
                        FILE,
                    ),
                    id: Sha1(d95f3ad14dee633a758d2e331151e950dd13e4ed),
                },
            ),
        ]
        "#);
        Ok(())
    }

    #[test]
    fn error_during_tree_traversal_causes_failure() -> crate::Result {
        let repo = repo("untracked-only")?;
        let platform = repo.status(gix::progress::Discard)?.head_tree(hex_to_id(
            "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391", /* empty blob, invalid tree*/
        ));
        let expected_err = "Could not create index from tree at e69de29bb2d1d6434b8b29ae775ad8c2e48c5391";
        if cfg!(feature = "parallel") {
            let mut items: Vec<_> = platform.into_iter(None)?.collect();
            assert_eq!(
                items.len(),
                3,
                "2 untracked and one error, which is detected only in the end."
            );
            assert_eq!(items.pop().expect("last item").unwrap_err().to_string(), expected_err);
        } else {
            match platform.into_iter(None) {
                Ok(_) => {
                    unreachable!("errors would be detected early here as everything is done ahead of time")
                }
                Err(err) => {
                    assert_eq!(err.to_string(), expected_err);
                }
            }
        }
        Ok(())
    }
}

mod index_worktree {
    mod iter {
        use gix::status::index_worktree::Item;
        use gix_testtools::size_ok;
        use pretty_assertions::assert_eq;

        use crate::status::{repo, submodule_repo};

        #[test]
        fn item_size() {
            let actual = std::mem::size_of::<Item>();
            let expected = 264;
            assert!(
                size_ok(actual, expected),
                "The size is pretty huge and goes down ideally: {actual} <~ {expected}"
            );
        }

        #[test]
        fn submodule_modification() -> crate::Result {
            let repo = submodule_repo("modified-untracked-and-submodule-head-changed-and-modified")?;
            let mut status = repo
                .status(gix::progress::Discard)?
                .index_worktree_options_mut(|opts| {
                    opts.sorting =
                        Some(gix::status::plumbing::index_as_worktree_with_renames::Sorting::ByPathCaseSensitive);
                })
                .into_index_worktree_iter(None)?;
            let items: Vec<_> = status.by_ref().filter_map(Result::ok).collect();
            assert_eq!(items.len(), 3, "1 untracked, 1 modified file, 1 submodule modification");
            Ok(())
        }

        #[test]
        fn untracked_files_collapse_by_default() -> crate::Result {
            let repo = repo("untracked-only")?;
            let status = repo
                .status(gix::progress::Discard)?
                .index_worktree_options_mut(|opts| {
                    opts.sorting =
                        Some(gix::status::plumbing::index_as_worktree_with_renames::Sorting::ByPathCaseSensitive);
                })
                .into_index_worktree_iter(None)?;
            let items: Vec<_> = status.filter_map(Result::ok).collect();
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
        fn untracked_files_settings_none() -> crate::Result {
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
            let items: Vec<_> = status.by_ref().filter_map(Result::ok).collect();
            assert_eq!(items, [], "no untracked files are found…");
            assert_eq!(
                status.outcome_mut().expect("iteration done").index_worktree.dirwalk,
                None,
                "…as there was no directory walk"
            );
            Ok(())
        }

        #[test]
        fn early_drop_for_is_dirty_emulation() -> crate::Result {
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
    }
}

mod is_dirty {
    use crate::status::{repo, submodule_repo};

    #[test]
    fn various_changes_positive() -> crate::Result {
        let repo = submodule_repo("modified-untracked-and-submodule-head-changed-and-modified")?;
        assert!(repo.is_dirty()?, "The repository has various changes");
        Ok(())
    }

    #[test]
    fn submodule_changes_are_picked_up() -> crate::Result {
        let repo = submodule_repo("submodule-head-changed")?;
        assert!(repo.is_dirty()?, "head-changes are also discovered");
        Ok(())
    }

    #[test]
    fn untracked_files_are_excluded() -> crate::Result {
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
    fn index_changed() -> crate::Result {
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
