pub fn repo(name: &str) -> crate::Result<gix::Repository> {
    use crate::util::named_subrepo_opts;
    Ok(named_subrepo_opts(
        "make_submodules.sh",
        name,
        gix::open::Options::isolated(),
    )?)
}

mod open {
    use gix::submodule;

    use crate::submodule::repo;

    #[test]
    fn various() -> crate::Result {
        for (name, expected) in [
            (
                "with-submodules",
                &[
                    (
                        "m1",
                        gix::submodule::State {
                            repository_exists: true,
                            is_old_form: false,
                            worktree_checkout: true,
                            superproject_configuration: true,
                        },
                        Some(false),
                    ),
                    (
                        "dir/m1",
                        gix::submodule::State {
                            repository_exists: true,
                            is_old_form: false,
                            worktree_checkout: true,
                            superproject_configuration: true,
                        },
                        Some(false),
                    ),
                ] as &[_],
            ),
            (
                "with-submodules-after-clone",
                &[(
                    "m1",
                    gix::submodule::State {
                        repository_exists: false,
                        is_old_form: false,
                        worktree_checkout: false,
                        superproject_configuration: true,
                    },
                    None,
                )],
            ),
            (
                "not-a-submodule",
                &[(
                    "m1",
                    gix::submodule::State {
                        repository_exists: true,
                        is_old_form: false,
                        worktree_checkout: false,
                        superproject_configuration: true,
                    },
                    None,
                )],
            ),
        ] {
            let repo = repo(name)?;
            for (sm, (sm_name, expected, _expected_is_dirty)) in
                repo.submodules()?.expect("modules present").zip(expected)
            {
                assert_eq!(sm.name(), sm_name);
                let state = sm.state()?;
                assert_eq!(&state, expected);

                let sm_repo = sm.open()?;
                assert_eq!(sm_repo.is_some(), state.repository_exists);
                if let Some(sm_repo) = sm_repo {
                    assert_eq!(
                        sm_repo.kind(),
                        gix::repository::Kind::Submodule,
                        "Submodules are properly detected"
                    );
                    assert!(sm_repo.workdir().is_some(), "the workdir is always configured");
                    let worktree = sm_repo
                        .worktree()
                        .expect("worktrees are always returned even if there seems to not be a checkout");
                    assert_eq!(
                        worktree.dot_git_exists(),
                        state.worktree_checkout,
                        "there is a way to check for indicators that a submodule worktree isn't checked out though"
                    );
                }
                #[cfg(feature = "status")]
                for check_dirty in [false, true] {
                    let status = match sm.status(gix::submodule::config::Ignore::None, check_dirty) {
                        Ok(status) => status,
                        Err(err) => {
                            assert_eq!(
                                name, "not-a-submodule",
                                "{name}: BUG: only one submodule is expected to fail, got '{err:?}'"
                            );
                            continue;
                        }
                    };
                    assert_eq!(
                        &status.state, expected,
                        "no matter what status configuration, the state is always obtained"
                    );
                    assert_eq!(
                        status.is_dirty(),
                        *_expected_is_dirty,
                        "none of these submodules are dirty, but some aren't checked out"
                    );
                }
            }
            assert_eq!(
                repo.modules()?.expect("present").names().count(),
                expected.len(),
                "an expectation per submodule"
            );
        }
        Ok(())
    }

    #[cfg(feature = "status")]
    mod status {
        use crate::submodule::repo;
        use crate::util::hex_to_id;

        #[test]
        fn changed_head_compared_to_superproject_index() -> crate::Result {
            let repo = repo("submodule-head-changed")?;
            let sm = repo.submodules()?.into_iter().flatten().next().expect("one submodule");
            let mut status = sm.status(gix::submodule::config::Ignore::None, false)?;
            assert_eq!(
                status.is_dirty(),
                Some(true),
                "we could decide that the submodule is dirty"
            );
            assert_eq!(
                status.index_id,
                Some(hex_to_id("e046f3e51d955840619fc7d01fbd9a469663de22"))
            );
            assert_eq!(
                status.checked_out_head_id,
                Some(hex_to_id("362cb5539acbd3c8ca355471f97c6a68d3db0da7")),
                "the checked out head was reset to something else after the superproject commit"
            );
            assert_eq!(
                status.changes,
                Some(Vec::new()),
                "the status check ran, but there were no changes"
            );
            // make it easier to compare this as baseline
            status.changes.take();

            let status_with_ignore = sm.status(gix::submodule::config::Ignore::Dirty, false)?;
            assert_eq!(
                status_with_ignore, status,
                "The lowest status that makes these changes observable"
            );

            let status_with_ignore_check_only = sm.status(gix::submodule::config::Ignore::Dirty, true)?;
            assert_eq!(
                status_with_ignore_check_only, status,
                "dirty-check has no observable influence here yet as there no 'more expensive' changes"
            );

            let status_with_ignore = sm.status(gix::submodule::config::Ignore::All, false)?;
            assert_eq!(
                status_with_ignore.is_dirty(),
                Some(false),
                "no dirty-information is retrieved, it seems clean"
            );
            assert_eq!(
                status_with_ignore.index_id, None,
                "to avoid false-positives, we don't retrieve the value"
            );
            assert_eq!(
                status_with_ignore.checked_out_head_id, None,
                "this check is ignored as it requires opening a repository"
            );
            Ok(())
        }

        #[test]
        fn modified_in_index_only() -> crate::Result {
            let repo = repo("submodule-index-changed")?;
            let sm = repo.submodules()?.into_iter().flatten().next().expect("one submodule");

            for mode in [
                gix::submodule::config::Ignore::Untracked,
                gix::submodule::config::Ignore::None,
            ] {
                for check_dirty in [false, true] {
                    let status = sm.status_opts(mode, check_dirty, &mut |platform| platform)?;
                    assert_eq!(
                        status.is_dirty(),
                        Some(true),
                        "two files were renamed using `git mv` for an HEAD^{{tree}}-index change"
                    );
                    assert_eq!(
                        status.changes.expect("present").len(),
                        if check_dirty { 1 } else { 3 },
                        "in is-dirty mode, we don't collect all changes"
                    );
                }
            }

            assert!(
                repo.is_dirty()?,
                "superproject should see submodule changes in the index as well"
            );
            Ok(())
        }

        #[test]
        fn modified_and_untracked() -> crate::Result {
            let repo = repo("modified-and-untracked")?;
            let sm = repo.submodules()?.into_iter().flatten().next().expect("one submodule");

            let status = sm.status(gix::submodule::config::Ignore::Dirty, false)?;
            assert_eq!(status.is_dirty(), Some(false), "Dirty skips worktree changes entirely");

            let mut status = sm.status_opts(
                gix::submodule::config::Ignore::None,
                false,
                &mut |status: gix::status::Platform<'_, gix::progress::Discard>| {
                    status.index_worktree_options_mut(|opts| {
                        opts.sorting = Some(gix_status::index_as_worktree_with_renames::Sorting::ByPathCaseSensitive);
                    })
                },
            )?;
            assert_eq!(
                status.is_dirty(),
                Some(true),
                "we could decide that the submodule is dirty"
            );
            assert_eq!(status.index_id, status.checked_out_head_id, "the head didn't change");
            assert_eq!(
                status.changes.as_ref().into_iter().flatten().count(),
                2,
                "1 modified, 1 untracked"
            );

            let status_with_dirty_check = sm.status_opts(
                gix::submodule::config::Ignore::None,
                true, /* check-dirty */
                &mut |status: gix::status::Platform<'_, gix::progress::Discard>| {
                    status.index_worktree_options_mut(|opts| {
                        opts.sorting = Some(gix_status::index_as_worktree_with_renames::Sorting::ByPathCaseSensitive);
                    })
                },
            )?;
            status.changes.as_mut().expect("two changes").pop();
            assert_eq!(
                status_with_dirty_check, status,
                "it cannot abort early as the only change it sees is the modification check.\
                However, with check-dirty, it would only gather the changes"
            );

            let status = sm.status(gix::submodule::config::Ignore::Untracked, false)?;
            assert_eq!(
                status.is_dirty(),
                Some(true),
                "we could decide that the submodule is dirty, even though untracked files are missing"
            );
            assert_eq!(status.index_id, status.checked_out_head_id, "the head didn't change");
            assert_eq!(status.changes.as_ref().into_iter().flatten().count(), 1, "1 modified");

            Ok(())
        }

        #[test]
        fn changed_head_empty_worktree() -> crate::Result {
            let repo = repo("submodule-head-changed-no-worktree")?;
            let sm = repo.submodules()?.into_iter().flatten().next().expect("one submodule");

            let status = sm.status(gix::submodule::config::Ignore::None, false)?;
            assert_eq!(
                status.state,
                gix::submodule::State {
                    repository_exists: true,
                    is_old_form: false,
                    worktree_checkout: false,
                    superproject_configuration: true,
                }
            );
            assert_eq!(
                status.is_dirty(),
                None,
                "a missing worktree counts as no-dirty, even though the checked out HEAD changed. \
                 Git does the same, even though as we express it as 'not determined'"
            );
            assert_ne!(
                status.index_id, status.checked_out_head_id,
                "not considered dirty despite head mismatch"
            );
            assert!(
                status.changes.is_none(),
                "Detailed changes are never done if there is no worktree"
            );

            Ok(())
        }

        #[test]
        fn is_dirty_skips_expensive_checks() -> crate::Result {
            let repo = repo("submodule-head-changed-and-modified")?;
            let sm = repo.submodules()?.into_iter().flatten().next().expect("one submodule");

            let status = sm.status(gix::submodule::config::Ignore::None, true)?;
            assert_eq!(
                status.changes, None,
                "computation was stopped on the first detected change (the index/head)"
            );
            assert_eq!(
                status.index_id,
                Some(hex_to_id("e046f3e51d955840619fc7d01fbd9a469663de22")),
                "the index id was obtained"
            );
            assert_eq!(
                status.checked_out_head_id,
                Some(hex_to_id("362cb5539acbd3c8ca355471f97c6a68d3db0da7")),
                "the checked out head was also obtained to be able to se if it's dirty"
            );
            Ok(())
        }
    }

    #[test]
    fn not_a_submodule() -> crate::Result {
        let repo = repo("not-a-submodule")?;
        let sm = repo.submodules()?.into_iter().flatten().next().expect("one submodule");
        assert!(sm.open()?.is_some(), "repo available as it was cloned");
        assert!(sm.index_id()?.is_none(), "no actual submodule");
        assert!(sm.head_id()?.is_none(), "no actual submodule");
        Ok(())
    }

    #[test]
    #[cfg(feature = "revision")]
    fn submodule_worktrees() -> crate::Result {
        let sm_repo = crate::util::named_subrepo_opts(
            "make_submodule_with_worktree.sh",
            "worktree-of-submodule",
            gix::open::Options::isolated(),
        )?;
        let wd = sm_repo.workdir().expect("workdir is present");
        assert!(
            sm_repo.rev_parse_single(":this").is_ok(),
            "the file is in the submodule"
        );
        assert!(
            wd.join("this").is_file(),
            "The submodule itself has the file, so it should be in the worktree"
        );

        assert_eq!(sm_repo.worktrees()?.len(), 1, "only a single linked worktree");
        Ok(())
    }

    #[test]
    #[cfg(feature = "revision")]
    fn list_submodule_worktrees() -> crate::Result {
        let sm_repo = crate::util::named_subrepo_opts(
            "make_submodule_with_worktree.sh",
            "submodule-with-extra-worktree-host/m1",
            gix::open::Options::isolated(),
        )?;
        let wd = sm_repo.workdir().expect("workdir is present");
        assert!(
            sm_repo.rev_parse_single(":this").is_ok(),
            "the file is in the submodule"
        );
        assert!(
            wd.join("this").is_file(),
            "The submodule itself has the file, so it should be in the worktree"
        );

        assert_eq!(sm_repo.worktrees()?.len(), 1, "only a single linked worktree");
        Ok(())
    }

    #[test]
    fn old_form() -> crate::Result {
        for name in ["old-form-invalid-worktree-path", "old-form"] {
            let repo = repo(name)?;
            let sm = repo
                .submodules()?
                .expect("modules present")
                .next()
                .expect("one submodule");

            assert_ne!(
                sm.git_dir_try_old_form()?,
                sm.git_dir(),
                "compat git dir should be the worktree location"
            );
            let sm_repo = sm.open()?.expect("initialized");
            assert_eq!(
                sm_repo.kind(),
                gix::repository::Kind::WorkTree { is_linked: false },
                "old submodules aren't recognized as such because it would require reading a lot of additional data"
            );
            assert_eq!(
                sm.state()?,
                submodule::State {
                    repository_exists: true,
                    is_old_form: true,
                    worktree_checkout: true,
                    superproject_configuration: true,
                }
            );
        }
        Ok(())
    }
}
