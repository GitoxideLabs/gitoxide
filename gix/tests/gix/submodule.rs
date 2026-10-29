use crate::Result;
pub fn repo(name: &str) -> Result<gix::Repository> {
    use crate::util::named_subrepo_opts;
    Ok(named_subrepo_opts(
        "make_submodules.sh",
        name,
        gix::open::Options::isolated(),
    )?)
}

mod url {
    use std::path::{Path, PathBuf};

    use gix::bstr::{BString, ByteSlice};
    use gix_testtools::TestResult;

    #[test]
    fn resolved_urls_match_git_submodule_init() -> TestResult {
        let fixture = fixture()?;
        let cases = Case::load_all(&fixture)?;
        assert_eq!(cases.len(), 26, "all URL input pairs are covered by the fixture");
        for case in cases {
            let remote_url = &case.remote_url;
            let module_url = &case.module_url;
            let repo = case.open("worktree")?;
            let sm = submodule(&repo)?;
            assert_eq!(
                sm.url()?.to_bstring(),
                module_url.as_bstr(),
                "url() preserves the raw configuration"
            );
            let resolved = sm.resolved_url();
            if !case.init_succeeded {
                assert!(resolved.is_err(), "both reject {remote_url:?} + {module_url:?}");
                continue;
            }
            let expected = case.expected_url("worktree")?;
            assert_eq!(
                resolved?, expected,
                "resolution matches Git for {remote_url:?} + {module_url:?}"
            );
            let initialized = case.open("initialized")?;
            assert_eq!(
                submodule(&initialized)?.resolved_url()?,
                case.expected_url("initialized")?,
                "initialized URLs remain suitable for cloning"
            );
        }
        Ok(())
    }

    #[test]
    fn remote_selection_and_overrides_match_git() -> TestResult {
        let fixture = fixture()?;
        for name in [
            "upstream-branch",
            "upstream-detached",
            "missing",
            "own-upstream",
            "upstream-only",
            "multiple",
        ] {
            let case = fixture.join("remote-selection").join(name);
            let repo = gix::open_opts(case.join("worktree"), gix::open::Options::isolated())?;
            let resolved = submodule(&repo)?.resolved_url()?;
            if name.starts_with("upstream-") {
                assert_eq!(
                    resolved.to_bstring(),
                    "ssh://host/org/module1",
                    "the only remote is the fallback even with a detached HEAD"
                );
            }
            // The fixture records whether its Git supports the only-remote fallback.
            if std::fs::read_to_string(case.join("compare-git"))?.trim() == "true" {
                assert_same_url(
                    resolved,
                    expected_url(&case, "worktree")?,
                    &format!("default remote selection matches Git: {name}"),
                )?;
            }
        }
        let repo = gix::open_opts(fixture.join("override-absolute"), gix::open::Options::isolated())?;
        let sm = submodule(&repo)?;
        assert_eq!(
            sm.resolved_url()?.to_bstring(),
            "ssh://other/override.git",
            "local configuration overrides .gitmodules"
        );
        assert_eq!(
            sm.url()?.to_bstring(),
            "ssh://other/override.git",
            "the raw accessor also honors overrides"
        );
        let worktree = fixture.join("override-relative");
        let repo = gix::open_opts(&worktree, gix::open::Options::isolated())?;
        let sm = submodule(&repo)?;
        let mut expected = gix::url::parse("../override")?;
        expected.canonicalize(&worktree)?;
        assert_eq!(
            sm.resolved_url()?,
            expected,
            "relative configuration overrides are local paths, not remote-relative URLs"
        );
        let repo = gix::open_opts(
            &worktree,
            gix::open::Options::isolated()
                .with(gix::sec::Trust::Reduced)
                .config_overrides(["submodule.m1.url=ssh://trusted/override.git"]),
        )?;
        let sm = submodule(&repo)?;
        assert_eq!(
            sm.resolved_url()?.to_bstring(),
            "ssh://trusted/override.git",
            "trusted API overrides remain usable with reduced repository trust"
        );
        Ok(())
    }

    #[test]
    fn unborn_and_bare_superprojects_resolve_urls() -> TestResult {
        let fixture = fixture()?;
        let repo = gix::open_opts(fixture.join("unborn-upstream"), gix::open::Options::isolated())?;
        let sm = submodule(&repo)?;
        assert_eq!(
            sm.resolved_url()?.to_bstring(),
            "ssh://host/org/sub",
            "unborn branches can have a configured upstream"
        );
        let worktree = fixture.join("unborn-missing");
        let repo = gix::open_opts(&worktree, gix::open::Options::isolated())?;
        let sm = submodule(&repo)?;
        let mut expected = gix::url::parse("../sub")?;
        expected.canonicalize(&worktree)?;
        assert_same_url(
            sm.resolved_url()?,
            expected,
            "a missing upstream uses the unborn superproject itself",
        )?;

        let repo = gix::open_opts(fixture.join("bare.git"), gix::open::Options::isolated())?;
        let sm = submodule(&repo)?;
        let expected = gix::url::parse(gix::path::into_bstr(fixture.join("sub"))?)?;
        assert_same_url(
            sm.resolved_url()?,
            expected,
            "bare repositories resolve .gitmodules against their local origin",
        )?;
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn own_upstream_accepts_verbatim_repository_paths() -> TestResult {
        let fixture = fixture()?;
        let worktree = fixture.join("unborn-local");
        let repo = gix::open_opts(std::fs::canonicalize(&worktree)?, gix::open::Options::isolated())?;
        let sm = submodule(&repo)?;
        let resolved = sm.resolved_url()?;
        assert_eq!(resolved.scheme, gix::url::Scheme::File, "the fallback remains local");
        assert_eq!(
            std::fs::canonicalize(gix::path::from_bstr(resolved.path.as_bstr())?)?,
            std::fs::canonicalize(fixture.join("sub"))?,
            "verbatim and short paths identify the same local destination"
        );
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn native_relative_urls_match_git() -> TestResult {
        let case = fixture()?.join("native-relative");
        let repo = gix::open_opts(case.join("worktree"), gix::open::Options::isolated())?;
        let resolved = submodule(&repo)?.resolved_url()?;
        let expected = expected_url(&case, "worktree")?;
        assert_eq!(resolved, expected, "native separators resolve like Git on Windows");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn local_urls_preserve_non_utf8_paths() -> TestResult {
        let repo = gix::open_opts(fixture()?.join("non-utf8"), gix::open::Options::isolated())?;
        let sm = submodule(&repo)?;
        let resolved = sm.resolved_url()?;
        assert!(
            resolved.path.ends_with(b"/source-\xff"),
            "resolution preserves arbitrary local path bytes"
        );
        assert!(
            gix::path::from_bstr(resolved.path.as_bstr())?.is_absolute(),
            "local URLs are independent of CWD"
        );
        Ok(())
    }

    struct Case {
        path: PathBuf,
        remote_url: BString,
        module_url: BString,
        init_succeeded: bool,
    }

    impl Case {
        fn load_all(fixture: &Path) -> TestResult<Vec<Self>> {
            std::fs::read_dir(fixture.join("resolution"))?
                .map(|entry| {
                    let path = entry?.path();
                    let remote_url = std::fs::read(path.join("remote-url"))?.trim_end().into();
                    let module_url = std::fs::read(path.join("module-url"))?.trim_end().into();
                    let init_succeeded = std::fs::read_to_string(path.join("init-status"))?
                        .trim()
                        .parse::<i32>()?
                        == 0;
                    Ok(Case {
                        path,
                        remote_url,
                        module_url,
                        init_succeeded,
                    })
                })
                .collect()
        }

        fn open(&self, state: &str) -> TestResult<gix::Repository> {
            Ok(gix::open_opts(self.path.join(state), gix::open::Options::isolated())?)
        }

        fn expected_url(&self, state: &str) -> TestResult<gix::url::Url> {
            expected_url(&self.path, state)
        }
    }

    fn fixture() -> TestResult<PathBuf> {
        Ok(gix::path::realpath(gix_testtools::scripted_fixture_read_only(
            "make_submodule_urls.sh",
        )?)?)
    }

    fn submodule(repo: &gix::Repository) -> TestResult<gix::Submodule<'_>> {
        Ok(repo
            .submodules()?
            .expect("fixture has submodules")
            .next()
            .expect("fixture has one submodule"))
    }

    fn expected_url(case: &Path, state: &str) -> TestResult<gix::url::Url> {
        let expected = std::fs::read(case.join("expected-url"))?;
        let mut expected = gix::url::parse(expected.trim())?;
        expected.canonicalize(&case.join(state))?;
        Ok(expected)
    }

    fn assert_same_url(mut actual: gix::url::Url, mut expected: gix::url::Url, context: &str) -> TestResult {
        for url in [&mut actual, &mut expected] {
            if url.scheme == gix::url::Scheme::File {
                let path = gix::path::from_bstr(url.path.as_bstr())?;
                // Git and Rust can use different Windows short names; the destination need not exist yet.
                let parent = std::fs::canonicalize(path.parent().expect("local test URLs have a parent"))?;
                url.path =
                    gix::path::into_bstr(parent.join(path.file_name().expect("local test URLs name a destination")))?
                        .into_owned();
            }
        }
        assert_eq!(actual, expected, "{context}");
        Ok(())
    }
}

mod open {
    use gix_testtools::TestResult;
    use std::io::Write;

    use gix_sec::Trust;

    use gix::submodule;

    use crate::{submodule::repo, util::named_subrepo_opts};

    #[test]
    fn linked_superproject_submodule_repositories_are_private() -> TestResult {
        let fixture = gix_testtools::scripted_fixture_read_only("make_linked_superproject_submodules.sh")?;
        let main = gix::open_opts(fixture.join("superproject"), crate::restricted())?;
        for (name, repository_exists, worktree_checkout) in [
            ("uninitialized", false, false),
            ("initialized", true, true),
            ("deinitialized", true, false),
        ] {
            let linked_path = fixture.join(name);
            let linked = gix::open_opts(&linked_path, crate::restricted())?;
            let sm = linked
                .submodules()?
                .expect("fixture has submodules")
                .next()
                .expect("fixture has m1");
            let expected_git_dir = linked.git_dir().join("modules/m1");
            assert_eq!(
                sm.git_dir()?,
                expected_git_dir,
                "linked submodule repositories belong to the private superproject Git directory"
            );
            if !repository_exists {
                assert!(
                    linked_path.join("m1").is_dir(),
                    "initial checkout creates the submodule directory"
                );
                assert_eq!(
                    std::fs::read_dir(linked_path.join("m1"))?.count(),
                    0,
                    "initial checkout leaves the submodule unpopulated"
                );
            }
            let state = sm.state()?;
            assert_eq!(state.repository_exists, repository_exists, "repository state in {name}");
            assert_eq!(state.worktree_checkout, worktree_checkout, "checkout state in {name}");
            let opened = sm.open()?;
            assert_eq!(
                opened.is_some(),
                repository_exists,
                "opening follows the linked repository's lifecycle"
            );
            if let Some(opened) = opened {
                assert_eq!(
                    std::fs::canonicalize(opened.git_dir())?,
                    std::fs::canonicalize(&expected_git_dir)?,
                    "opening uses the private submodule repository in {name}"
                );
            }
        }
        let main_sm = main
            .submodules()?
            .expect("fixture has submodules")
            .next()
            .expect("fixture has m1");
        assert_eq!(
            main_sm.git_dir()?,
            main.git_dir().join("modules/m1"),
            "main submodule location is unchanged"
        );
        assert!(
            main_sm.state()?.worktree_checkout,
            "the main submodule remains populated"
        );
        Ok(())
    }

    #[test]
    fn empty_worktree_config_is_not_an_uninitialized_submodule() -> TestResult {
        let fixture = gix_testtools::scripted_fixture_writable("make_submodule_with_worktree.sh")?;
        let repo = gix::open_opts(
            fixture.path().join("submodule-with-extra-worktree-host"),
            gix::open::Options::isolated(),
        )?;
        let worktree = repo.workdir().expect("fixture has a worktree");
        gix_testtools::git(worktree, "submodule deinit -f m1")?;
        let sm = repo
            .submodules()?
            .expect("fixture has submodules")
            .next()
            .expect("fixture has m1");
        assert!(
            sm.open()?.is_some(),
            "deinitializing preserves the submodule repository"
        );
        std::fs::OpenOptions::new()
            .append(true)
            .open(repo.git_dir().join("modules/m1/config"))?
            .write_all(b"\n[core]\n\tworktree = \n")?;

        let err = sm
            .open()
            .expect_err("invalid core.worktree configuration must propagate");
        insta::assert_debug_snapshot!(err, "the nested missing-path error must not hide the configuration failure", @r#"
        The path at the 'core.worktree' configuration could not be interpolated, input=""

        Caused by:
            0: path is missing
        "#);
        assert!(
            err.downcast_any_ref::<gix_error::Message>()
                .is_some_and(|error| error.class == Some(gix_error::Class::Validation)),
            "the failure is classified as invalid configuration: {err:?}"
        );
        assert!(
            err.is_not_found(),
            "the nested missing-path error must not hide the configuration failure"
        );
        Ok(())
    }

    #[test]
    fn various() -> TestResult {
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

    #[test]
    fn absolute_looking_names_remain_below_the_modules_directory() -> TestResult {
        let repo = repo("absolute-looking-submodule-names")?;
        let modules_dir = repo.common_dir().join("modules");
        let mut count = 0;
        for sm in repo.submodules()?.expect("submodule configuration present") {
            let git_dir = sm.git_dir()?;
            assert!(
                git_dir.starts_with(&modules_dir),
                "Git-compatible name {:?} must remain below {modules_dir:?} instead of producing {git_dir:?}",
                sm.name()
            );
            assert!(sm.index_id()?.is_some(), "Git added the submodule to the index");
            assert!(sm.head_id()?.is_some(), "Git committed the submodule as a gitlink");
            count += 1;
        }
        assert_eq!(count, 2, "both absolute-looking names were parsed as submodules");
        Ok(())
    }

    /// Reproducer for GHSA-p3hw-mv63-rf9w: `Submodule::open()` must not inherit
    /// `git_dir_trust` from the parent repository because doing so skips recomputing
    /// trust for the submodule git-dir and can bypass ownership-based trust checks.
    #[test]
    fn trust_is_recomputed_for_opened_submodules() -> TestResult {
        let repo = named_subrepo_opts(
            "make_submodules.sh",
            "with-submodules",
            crate::restricted().with(Trust::Reduced),
        )?;
        assert_eq!(
            repo.git_dir_trust(),
            Trust::Reduced,
            "the parent trust is forced for the test"
        );

        let sm = repo
            .submodules()?
            .expect("modules present")
            .next()
            .expect("one submodule");
        let sm_repo = sm.open()?.expect("submodule repository exists");
        assert_eq!(
            sm_repo.git_dir_trust(),
            Trust::Full,
            "submodule trust must be re-derived from its own git-dir instead of inherited from the parent"
        );
        Ok(())
    }

    #[test]
    fn gitlink_target_takes_precedence_over_name_in_git_dir_resolution() -> TestResult {
        let repo = repo("submodule-with-divergent-gitlink")?;
        let sm = repo
            .submodules()?
            .expect("modules present")
            .next()
            .expect("one submodule");

        let git_dir_from_name = sm.git_dir()?;
        assert!(
            git_dir_from_name.ends_with("modules/outer/inner"),
            "the name-derived path is the fallback location, what it should be per submodule configuration/name"
        );

        let git_dir_from_gitlink = sm.git_dir_try_old_form()?;
        assert!(
            git_dir_from_gitlink.ends_with("modules/inner"),
            "the worktree .git file points at the relocated repository"
        );
        assert_ne!(
            git_dir_from_gitlink, git_dir_from_name,
            "the gitlink target must be authoritative even when it differs from .git/modules/<name>"
        );
        assert_eq!(
            sm.state()?,
            submodule::State {
                repository_exists: true,
                is_old_form: false,
                worktree_checkout: true,
                superproject_configuration: true,
            },
            "a modern gitlink remains modern even when its target differs from the name-derived path"
        );

        #[cfg(feature = "status")]
        {
            let status = sm.status(gix::submodule::config::Ignore::None, false)?;
            assert_eq!(
                status.is_dirty(),
                Some(false),
                "opening the gitlink target avoids a 'phantom' submodule HEAD change"
            );
            assert_eq!(
                status.checked_out_head_id, status.index_id,
                "there are no HEAD changes even though the 'phantom' at modules/outer/inner has its HEAD at @~1"
            );

            let status = sm.status(gix::submodule::config::Ignore::All, false)?;
            assert_eq!(
                status.state,
                submodule::State {
                    repository_exists: true,
                    is_old_form: false,
                    worktree_checkout: true,
                    superproject_configuration: true,
                },
                "ignore=all still follows a parseable divergent gitdir file"
            );
        }

        Ok(())
    }

    #[test]
    fn status_uses_detached_worktree_from_symlinked_git_dir() -> TestResult {
        let root =
            gix_testtools::scripted_fixture_read_only("make_submodules.sh")?.join("linked-git-dir-detached-worktree");
        let repo = gix::open_opts(root.join("home"), gix::open::Options::isolated())?;
        let sm = repo
            .submodules()?
            .expect("modules present")
            .next()
            .expect("one submodule");
        let submodule_workdir = gix_path::realpath(root.join("home/.config/awesome/lain"))?;

        assert_eq!(
            sm.work_dir()?,
            submodule_workdir,
            "submodule workdir must stay anchored in the detached worktree"
        );
        assert_eq!(
            sm.open()?.expect("submodule repository exists").workdir(),
            Some(submodule_workdir.as_path()),
            "opening the submodule must use its actual checkout, not a path derived from the symlink parent"
        );

        #[cfg(feature = "status")]
        {
            let status = sm.status(gix::submodule::config::Ignore::None, false)?;
            assert_eq!(
                status.is_dirty(),
                Some(false),
                "status must succeed and report the clean detached-worktree submodule"
            );
        }

        Ok(())
    }

    #[test]
    #[cfg(unix)] // symlinks are used here, let's not try our luck on Windows.
    fn keeps_callers_path_namespace_when_opened_through_symlinked_ancestor() -> TestResult {
        let link = gix_testtools::scripted_fixture_read_only("make_submodules.sh")?.join("symlinked-ancestor");

        for parent_name in ["with-submodules", "with-submodule-uninitialized-checkout"] {
            let worktree = link.join(parent_name);
            let repo = gix::open_opts(&worktree, gix::open::Options::isolated())?;
            assert_eq!(
                repo.workdir(),
                Some(worktree.as_path()),
                "the parent repository keeps the path it was opened with"
            );
            for sm in repo.submodules()?.expect("modules present") {
                let sm_repo = sm.open()?.expect("submodule repository exists");
                let sm_git_dir = sm_repo.path();
                assert!(
                    sm_git_dir.starts_with(&worktree),
                    "the submodule git dir stays in the namespace of the path the parent was opened with: {sm_git_dir:?}"
                );
                assert_eq!(
                    sm_repo.workdir(),
                    Some(worktree.join(gix_path::from_bstr(sm.path()?)?.as_ref())).as_deref(),
                    "the submodule workdir stays in the same namespace instead of being canonicalized, \
                     so it can be related to the parent worktree and the submodule git dir with prefix logic"
                );
            }
        }

        Ok(())
    }

    #[test]
    fn broken_gitlink_target_is_reported() -> TestResult {
        let mut error_snapshots = Vec::new();
        let repo = repo("submodule-with-missing-gitlink-target")?;
        let work_dir = repo.workdir().expect("fixture has a worktree");
        let sm = repo
            .submodules()?
            .expect("modules present")
            .next()
            .expect("one submodule");

        for err in [
            sm.git_dir_try_old_form().expect_err("the gitlink target is missing"),
            sm.state().expect_err("state validates the gitlink target"),
            sm.open().expect_err("opening validates the gitlink target"),
        ] {
            error_snapshots.push(gix_testtools::redact_debug_snapshot(
                &(err),
                &[(&work_dir.to_string_lossy(), "<repository>")],
            ));
            assert!(err.is_validation());
        }

        #[cfg(feature = "status")]
        {
            let err = sm
                .status(gix::submodule::config::Ignore::None, false)
                .expect_err("ignore=none fails as some submodules can't be opened");
            insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[(&work_dir.to_string_lossy(), "<repository>")]), "status reports the invalid submodule gitdir target", @r#"The gitdir file at "<repository>/m1/.git" contains an invalid gitdir target: "<repository>/m1/../missing""#);
            assert!(err.is_validation());
        }

        #[cfg(feature = "status")]
        {
            let status = sm.status(gix::submodule::config::Ignore::All, false)?;
            assert_eq!(
                status.state,
                submodule::State {
                    repository_exists: false,
                    is_old_form: false,
                    worktree_checkout: true,
                    superproject_configuration: true,
                },
                "ignore=all does not inspect the broken gitdir target"
            );
        }

        insta::assert_debug_snapshot!(error_snapshots, "broken gitlink target is reported", @r#"
        [
            The gitdir file at "<repository>/m1/.git" contains an invalid gitdir target: "<repository>/m1/../missing",
            The gitdir file at "<repository>/m1/.git" contains an invalid gitdir target: "<repository>/m1/../missing",
            The gitdir file at "<repository>/m1/.git" contains an invalid gitdir target: "<repository>/m1/../missing",
        ]
        "#);
        Ok(())
    }

    #[test]
    fn malformed_gitlink_target_is_ignored_by_ignore_all_status() -> TestResult {
        let repo = repo("submodule-with-malformed-gitlink")?;
        let work_dir = repo.workdir().expect("fixture has a worktree");
        let sm = repo
            .submodules()?
            .expect("modules present")
            .next()
            .expect("one submodule");

        let err = sm.git_dir_try_old_form().expect_err("the gitlink target is malformed");
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&(err), &[(&work_dir.to_string_lossy(), "<repository>")]), "malformed gitlink target is ignored by ignore all status", @r#"
        The gitdir file at "<repository>/m1/.git" contains an invalid gitdir target

        Caused by:
            0: Format should be 'gitdir: <path>', but got, input="bogus\n"
        "#);
        assert!(err.is_validation());

        #[cfg(feature = "status")]
        {
            let err = sm
                .status(gix::submodule::config::Ignore::None, false)
                .expect_err("ignore=none fails as some submodules can't be opened");
            insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[(&work_dir.to_string_lossy(), "<repository>")]), "status reports the invalid submodule gitdir target", @r#"
            The gitdir file at "<repository>/m1/.git" contains an invalid gitdir target

            Caused by:
                0: Format should be 'gitdir: <path>', but got, input="bogus\n"
            "#);
            assert!(err.is_validation());

            let status = sm.status(gix::submodule::config::Ignore::All, false)?;
            assert_eq!(
                status.state,
                submodule::State {
                    repository_exists: true,
                    is_old_form: false,
                    worktree_checkout: true,
                    superproject_configuration: true,
                },
                "ignore=all does not parse the malformed gitdir file"
            );
        }

        Ok(())
    }

    #[cfg(feature = "status")]
    mod status {
        use crate::{submodule::repo, util::hex_to_id};
        use gix_testtools::TestResult;

        #[test]
        fn changed_head_compared_to_superproject_index() -> TestResult {
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
        fn modified_in_index_only() -> TestResult {
            let mut repo: gix::Repository = repo("submodule-index-changed")?;
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

            repo.config_snapshot_mut()
                .set_value(&gix::config::tree::Diff::IGNORE_SUBMODULES, "all")?;

            if cfg!(feature = "parallel") {
                assert!(!repo.is_dirty()?, "There is a global flag to deactivate this");
            } else {
                assert!(
                    repo.is_dirty()?,
                    "We still spawn a thread in non-parallel mode, \
                    but then ThreadSafe repository isn't actually threadsafe.\
                    This is why we open a new repo, and can't see in-memory overrides.\
                    Maybe ThreadSafeRepository should be changed to always use thred-safe primitives."
                );
            }

            let sm = repo.submodules()?.into_iter().flatten().next().expect("one submodule");
            let status = sm.status_opts(gix::submodule::config::Ignore::None, true, &mut |platform| platform)?;
            assert_eq!(
                status.is_dirty(),
                Some(true),
                "The global override has no bearing on this specific call"
            );
            Ok(())
        }

        #[test]
        fn modified_and_untracked() -> TestResult {
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
        fn changed_head_empty_worktree() -> TestResult {
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
        fn is_dirty_skips_expensive_checks() -> TestResult {
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
    fn not_a_submodule() -> TestResult {
        let repo = repo("not-a-submodule")?;
        let sm = repo.submodules()?.into_iter().flatten().next().expect("one submodule");
        assert!(sm.open()?.is_some(), "repo available as it was cloned");
        assert!(sm.index_id()?.is_none(), "no actual submodule");
        assert!(sm.head_id()?.is_none(), "no actual submodule");
        Ok(())
    }

    #[test]
    fn in_unborn() -> TestResult {
        let repo = repo("unborn")?;
        assert_eq!(
            repo.submodules()?.into_iter().flatten().count(),
            0,
            "there is nothing, and that is fine"
        );
        Ok(())
    }

    #[test]
    #[cfg(feature = "revision")]
    fn submodule_worktrees() -> TestResult {
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
    fn list_submodule_worktrees() -> TestResult {
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
    fn old_form() -> TestResult {
        for name in ["old-form-invalid-worktree-path", "old-form"] {
            let repo = repo(name)?;
            let sm = repo
                .submodules()?
                .expect("modules present")
                .next()
                .expect("one submodule");

            assert_ne!(
                sm.git_dir_try_old_form()?,
                sm.git_dir()?,
                "compat git dir should be the worktree location"
            );
            let sm_repo = sm.open()?.expect("initialized");
            assert_eq!(
                sm_repo.kind(),
                gix::repository::Kind::Common,
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

#[cfg(unix)]
mod advisory {
    use gix_testtools::TestResult;

    /// Reproducer for GHSA-p3hw-mv63-rf9w and GHSA-fr8x-3vfx-f45h: a crafted submodule name with
    /// traversal components is reused to derive `.git/modules/<name>`, so `Submodule::state()` and
    /// `Submodule::open()` can be redirected to another repository outside the intended modules
    /// directory.
    #[test]
    fn traversal_names_do_not_escape_the_modules_directory() -> TestResult {
        let mut error_snapshots = Vec::new();
        let fixture = gix_testtools::scripted_fixture_writable("make_submodule_traversal_advisory.sh")?;
        let repo_dir = fixture.path().join("victim-repo");

        let repo = gix::open_opts(&repo_dir, crate::restricted())?;
        let sm = repo
            .submodules()?
            .expect("submodule configuration present")
            .next()
            .expect("one malicious submodule");

        assert!(matches!(
            sm.git_dir()
                .expect_err("the traversal name is rejected")
                .downcast_any_ref::<gix_validate::submodule::name::Error>(),
            Some(gix_validate::submodule::name::Error::ParentComponent)
        ));
        for err in [
            sm.git_dir_try_old_form().expect_err("the traversal name is rejected"),
            sm.open().expect_err("opening rejects the traversal name"),
            sm.state().expect_err("state rejects the traversal name"),
        ] {
            error_snapshots.push(gix_testtools::redact_debug_snapshot(&(err), &[]));
            assert!(err.is_validation());
            assert!(
                matches!(
                    err.downcast_any_ref::<gix_validate::submodule::name::Error>(),
                    Some(gix_validate::submodule::name::Error::ParentComponent)
                ),
                "the precise traversal-name failure remains in the erased API's chain"
            );
        }

        let redirected_repo = fixture.path().join("escaped-target.git");
        assert!(
            redirected_repo.is_dir(),
            "the attacker-controlled repository does indeex exist at {redirected_repo:?}"
        );
        insta::assert_debug_snapshot!(error_snapshots, "traversal names do not escape the modules directory", @"
        [
            The submodule name is invalid
            
            Caused by:
                0: Submodules names must not contains '..',
            The submodule name is invalid
            
            Caused by:
                0: Submodules names must not contains '..',
            The submodule name is invalid
            
            Caused by:
                0: Submodules names must not contains '..',
        ]
        ");
        Ok(())
    }

    /// Reproducer for malicious `submodule.<name>.update = !...` commands added in `.gitmodules`
    /// after `git submodule init`: `gix` must reject them from `.gitmodules` instead of exposing
    /// them as executable updates.
    #[test]
    fn update_commands_from_gitmodules_are_rejected_after_init() -> TestResult {
        let fixture = gix_testtools::scripted_fixture_writable("make_submodule_update_advisory.sh")?;
        let victim = fixture.path().join("victim");

        let repo = gix::open_opts(&victim, crate::restricted())?;
        let sm = repo
            .submodules()?
            .expect("submodules present")
            .next()
            .expect("one submodule");
        let err = sm.update().expect_err("commands from `.gitmodules` are forbidden");
        insta::assert_debug_snapshot!(err, "update commands from gitmodules are rejected after init", @r#"The 'update' field of submodule 'sub' tried to set a command to be shared, input="touch pwned""#);
        assert_eq!(
            err.metadata().find_map(|metadata| metadata.get("input")),
            Some(&gix_error::MetadataValue::Bytes("touch pwned".into()))
        );
        Ok(())
    }
}
#[cfg(feature = "worktree-mutation")]
mod restore {
    use std::{fs, sync::atomic::AtomicBool};

    use gix::progress;
    use gix_testtools::{TestResult, git};

    #[test]
    fn restores_head_and_replaces_staged_index_without_recursing() -> TestResult {
        for detached in [false, true] {
            let mut states = Vec::new();
            for use_git in [true, false] {
                let fixture = gix_testtools::scripted_fixture_writable("make_submodule_restore.sh")?;
                let parent = gix::open_opts(fixture.path().join("parent"), crate::restricted())?;
                let module = parent.submodules()?.expect("one module").next().expect("one module");
                let repo = gix::open_opts(module.git_dir()?.canonicalize()?, crate::restricted())?;
                let workdir = repo.workdir().expect("module worktree");
                if detached {
                    git(workdir, "checkout --detach")?;
                }
                let head_before = repo.head()?.referent_name().map(ToOwned::to_owned);
                let head_commit_id = repo.head_id()?.detach();
                fs::write(workdir.join("file"), "staged\n")?;
                fs::write(workdir.join("staged-addition"), "discarded\n")?;
                git(workdir, "add file staged-addition")?;
                fs::remove_dir_all(workdir)?;
                if use_git {
                    git(
                        parent.workdir().expect("parent worktree"),
                        "submodule update --init --no-fetch --checkout dir/module",
                    )?;
                } else {
                    module
                        .restore_worktree(progress::Discard, &AtomicBool::new(false))?
                        .expect("existing local clone is restored");
                    assert_eq!(
                        repo.head()?.referent_name().map(ToOwned::to_owned),
                        head_before,
                        "restoration preserves HEAD attachment"
                    );
                }
                assert_eq!(repo.head_id()?, head_commit_id, "restoration does not move HEAD");
                let reopened = gix::open_opts(workdir, crate::restricted())?;
                assert_eq!(
                    reopened.head_id()?,
                    head_commit_id,
                    "the recreated gitfile opens the clone"
                );
                assert!(
                    workdir.join("nested").is_dir() && !workdir.join("nested/file").exists(),
                    "nested submodules remain unpopulated"
                );
                states.push(gix_testtools::repository::snapshot(workdir)?);
            }
            assert_eq!(states[0].index, states[1].index, "restored index agrees with Git");
            assert_eq!(states[0].worktree, states[1].worktree, "restored files agree with Git");
        }
        Ok(())
    }

    #[test]
    fn restores_empty_gitfile_only_and_old_form_worktrees() -> TestResult {
        for case in ["empty", "gitfile-only", "old-form"] {
            let fixture = gix_testtools::scripted_fixture_writable("make_submodule_restore.sh")?;
            let parent = gix::open_opts(fixture.path().join("parent"), crate::restricted())?;
            let module = parent.submodules()?.expect("one module").next().expect("one module");
            let repo = gix::open_opts(module.git_dir()?.canonicalize()?, crate::restricted())?;
            let workdir = repo.workdir().expect("module worktree").canonicalize()?;
            let head_commit_id = repo.head_id()?.detach();
            for entry in fs::read_dir(&workdir)? {
                let entry = entry?;
                if entry.file_name() != ".git" {
                    if entry.file_type()?.is_dir() {
                        fs::remove_dir_all(entry.path())?;
                    } else {
                        fs::remove_file(entry.path())?;
                    }
                }
            }
            if case != "gitfile-only" {
                fs::remove_file(workdir.join(".git"))?;
            }
            if case == "old-form" {
                fs::rename(repo.git_dir(), workdir.join(".git"))?;
                git(&workdir, "--git-dir=.git --work-tree=. config core.worktree ..")?;
            }
            module
                .restore_worktree(progress::Discard, &AtomicBool::new(false))?
                .expect("local clone is reused");
            let reopened = gix::open_opts(&workdir, crate::restricted())?;
            assert_eq!(reopened.head_id()?, head_commit_id, "{case}: HEAD is unchanged");
            assert_eq!(
                fs::read(workdir.join("file"))?,
                b"tracked\n",
                "{case}: files are restored"
            );
            assert_eq!(
                workdir.join(".git").is_dir(),
                case == "old-form",
                "old-form clones stay in place"
            );
            assert_eq!(
                gix::open_opts(reopened.git_dir(), crate::restricted())?
                    .workdir()
                    .expect("core.worktree")
                    .canonicalize()?,
                workdir,
                "core.worktree reconnects the clone independently of the gitfile"
            );
        }
        Ok(())
    }

    #[test]
    fn missing_clones_remain_uninitialized_and_unborn_clones_get_an_empty_index() -> TestResult {
        for missing_clone in [false, true] {
            let fixture = gix_testtools::scripted_fixture_writable("make_submodule_restore.sh")?;
            let parent = gix::open_opts(fixture.path().join("parent"), crate::restricted())?;
            let module = parent.submodules()?.expect("one module").next().expect("one module");
            let repo = gix::open_opts(module.git_dir()?.canonicalize()?, crate::restricted())?;
            let workdir = repo.workdir().expect("module worktree");
            if missing_clone {
                fs::remove_dir_all(repo.git_dir())?;
            } else {
                git(workdir, "symbolic-ref HEAD refs/heads/unborn")?;
            }
            fs::remove_dir_all(workdir)?;
            let outcome = module.restore_worktree(progress::Discard, &AtomicBool::new(false))?;
            assert_eq!(
                outcome.is_none(),
                missing_clone,
                "restoration never creates a new clone"
            );
            if missing_clone {
                assert!(
                    !workdir.exists(),
                    "an unavailable clone leaves the missing worktree alone"
                );
            } else {
                let reopened = gix::open_opts(workdir, crate::restricted())?;
                assert!(reopened.head()?.is_unborn(), "an unborn branch stays unborn");
                assert!(
                    reopened.open_index()?.entries().is_empty(),
                    "the initial index is empty"
                );
                assert_eq!(fs::read_dir(workdir)?.count(), 1, "only the gitfile is created");
            }
        }
        Ok(())
    }

    #[test]
    fn preflight_refusals_preserve_index_and_destination() -> TestResult {
        for case in ["nonempty", "foreign-link", "index-lock", "config-lock", "cancelled"] {
            let fixture = gix_testtools::scripted_fixture_writable("make_submodule_restore.sh")?;
            let parent = gix::open_opts(fixture.path().join("parent"), crate::restricted())?;
            let module = parent.submodules()?.expect("one module").next().expect("one module");
            let repo = gix::open_opts(module.git_dir()?.canonicalize()?, crate::restricted())?;
            let workdir = repo.workdir().expect("module worktree");
            fs::remove_dir_all(workdir)?;
            match case {
                "nonempty" => {
                    fs::create_dir(workdir)?;
                    fs::write(workdir.join("keep"), "preserved\n")?;
                }
                "foreign-link" => {
                    fs::create_dir(workdir)?;
                    fs::write(
                        workdir.join(".git"),
                        format!("gitdir: {}\n", parent.git_dir().display()),
                    )?;
                }
                "index-lock" => fs::write(repo.index_path().with_extension("lock"), "held\n")?,
                "config-lock" => fs::write(repo.git_dir().join("config.lock"), "held\n")?,
                "cancelled" => {}
                _ => unreachable!(),
            }
            let before = gix_testtools::repository::snapshot(parent.workdir().expect("parent worktree"))?;
            let index_before = fs::read(repo.index_path())?;
            let config_before = fs::read(repo.git_dir().join("config"))?;
            module
                .restore_worktree(progress::Discard, &AtomicBool::new(case == "cancelled"))
                .expect_err("the refusal happens before any mutation");
            assert_eq!(
                before,
                gix_testtools::repository::snapshot(parent.workdir().expect("parent worktree"))?,
                "{case}: worktree contents are preserved"
            );
            assert_eq!(
                fs::read(repo.index_path())?,
                index_before,
                "{case}: staged state is preserved"
            );
            assert_eq!(
                fs::read(repo.git_dir().join("config"))?,
                config_before,
                "{case}: metadata is preserved"
            );
        }
        Ok(())
    }

    #[test]
    fn filters_are_applied_and_failures_preserve_the_previous_index() -> TestResult {
        for fail in [false, true] {
            let fixture = gix_testtools::scripted_fixture_writable("make_submodule_restore.sh")?;
            let parent = gix::open_opts(fixture.path().join("parent"), crate::restricted())?;
            let module = parent.submodules()?.expect("one module").next().expect("one module");
            let repo = gix::open_opts(module.git_dir()?.canonicalize()?, crate::restricted())?;
            let workdir = repo.workdir().expect("module worktree");
            fs::write(repo.git_dir().join("info/attributes"), "file filter=restore\n")?;
            let mut config = repo.config_file_mut(repo.git_dir().join("config"))?;
            config.set_raw_value("filter.restore.smudge", if fail { "false" } else { "printf filtered" })?;
            config.set_raw_value("filter.restore.required", "true")?;
            config.commit()?;
            let index_before = fs::read(repo.index_path())?;
            let head_before = fs::read(repo.git_dir().join("HEAD"))?;
            fs::remove_dir_all(workdir)?;
            let outcome = module.restore_worktree(progress::Discard, &AtomicBool::new(false));
            assert_eq!(outcome.is_err(), fail, "required filter failures are reported");
            assert_eq!(
                fs::read(repo.git_dir().join("HEAD"))?,
                head_before,
                "HEAD never changes"
            );
            assert!(
                !repo.index_path().with_extension("lock").exists(),
                "index locks are released"
            );
            if fail {
                assert_eq!(
                    fs::read(repo.index_path())?,
                    index_before,
                    "the original index survives failure"
                );
            } else {
                assert_eq!(fs::read(workdir.join("file"))?, b"filtered", "the smudge filter runs");
            }
        }
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn destination_and_parent_symlinks_are_never_followed() -> TestResult {
        for case in ["destination", "parent", "gitfile"] {
            let fixture = gix_testtools::scripted_fixture_writable("make_submodule_restore.sh")?;
            let parent = gix::open_opts(fixture.path().join("parent"), crate::restricted())?;
            let module = parent.submodules()?.expect("one module").next().expect("one module");
            let repo = gix::open_opts(module.git_dir()?.canonicalize()?, crate::restricted())?;
            let workdir = repo.workdir().expect("module worktree");
            fs::remove_dir_all(workdir)?;
            let outside = gix_testtools::tempfile::tempdir()?;
            fs::write(outside.path().join("keep"), "preserved\n")?;
            match case {
                "destination" => std::os::unix::fs::symlink(outside.path(), workdir)?,
                "parent" => {
                    fs::remove_dir(workdir.parent().expect("nested path"))?;
                    std::os::unix::fs::symlink(outside.path(), workdir.parent().expect("nested path"))?;
                }
                "gitfile" => {
                    fs::create_dir(workdir)?;
                    std::os::unix::fs::symlink(repo.git_dir(), workdir.join(".git"))?;
                }
                _ => unreachable!(),
            }
            let index_before = fs::read(repo.index_path())?;
            module
                .restore_worktree(progress::Discard, &AtomicBool::new(false))
                .expect_err("symlinks must not redirect checkout");
            assert_eq!(
                fs::read(repo.index_path())?,
                index_before,
                "{case}: the index is untouched"
            );
            assert_eq!(
                fs::read(outside.path().join("keep"))?,
                b"preserved\n",
                "outside content survives"
            );
            assert_eq!(
                fs::read_dir(outside.path())?.count(),
                1,
                "no entries are written outside"
            );
        }
        Ok(())
    }
}
