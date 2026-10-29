use gix::bstr::BString;

use crate::remote;

fn remote_names<'a>(it: impl IntoIterator<Item = &'a str>) -> Vec<BString> {
    it.into_iter().map(Into::into).collect()
}

fn remote_name(name: &str) -> BString {
    name.into()
}

#[test]
fn local_branches_tracking_ref_matches_git_without_requiring_remote_refs() -> gix_testtools::TestResult {
    let path = gix_testtools::scripted_fixture_read_only("make_local_tracking_branches.sh")?;
    let mut repo = gix::open_opts(&path, gix::open::Options::isolated())?;
    let tracking_ref: &gix::refs::FullNameRef = "refs/remotes/cache/topic".try_into()?;
    assert!(
        repo.try_find_reference(tracking_ref)?.is_none(),
        "the tracking ref need not exist"
    );
    let expected: Vec<_> = std::fs::read_to_string(path.join("branch-upstreams"))?
        .lines()
        .filter_map(|line| line.split_once(' '))
        .filter(|(_, upstream)| *upstream == tracking_ref.as_bstr())
        .map(|(branch, _)| BString::from(branch))
        .collect();
    let actual: Vec<_> = repo
        .local_branches_tracking_ref(tracking_ref)?
        .into_iter()
        .map(|branch| branch.name().as_bstr().to_owned())
        .collect();
    assert_eq!(
        actual, expected,
        "all existing configured trackers match Git's upstream query"
    );
    assert_eq!(
        actual,
        ["refs/heads/renamed", "refs/heads/second"],
        "branch names and stale branch config do not select trackers"
    );
    assert!(
        repo.local_branches_tracking_ref("refs/remotes/cache/missing".try_into()?)?
            .is_empty(),
        "no configured tracker is a valid empty result"
    );

    repo.config_snapshot_mut()
        .set_raw_value("branch.second.merge", "refs/heads/invalid name")?;
    assert!(
        repo.local_branches_tracking_ref(tracking_ref).is_err(),
        "invalid upstream configuration is not treated as absence"
    );
    Ok(())
}

#[test]
fn legacy_names_can_be_used_for_lookup() -> gix_testtools::TestResult {
    let path = gix_testtools::scripted_fixture_read_only("make_legacy_subsection_repo.sh")?;
    let repo = gix::open_opts(&path, gix::open::Options::isolated())?;
    assert_eq!(
        repo.remote_names().into_iter().collect::<Vec<_>>(),
        remote_names(std::fs::read_to_string(path.join("remote-names"))?.lines()),
        "remote enumeration uses the same canonical names as Git"
    );
    let remote = repo
        .find_default_remote(gix::remote::Direction::Fetch)
        .expect("one configured remote")?;
    assert_eq!(
        remote
            .url(gix::remote::Direction::Fetch)
            .expect("configured URL")
            .to_bstring(),
        std::fs::read_to_string(path.join("remote-url"))?.trim(),
        "the enumerated default remote retains its URL"
    );
    assert_eq!(repo.branch_names().into_iter().collect::<Vec<_>>(), ["main"]);
    assert!(
        repo.try_find_remote("Origin").transpose()?.is_none(),
        "the raw legacy spelling is not a separate configured remote"
    );
    Ok(())
}

#[test]
fn remote_and_branch_names() {
    let repo = remote::repo("base");
    assert_eq!(repo.remote_names().len(), 0, "there are no remotes");
    assert_eq!(repo.branch_names().len(), 0, "there are no configured branches");
    assert_eq!(repo.remote_default_name(gix::remote::Direction::Fetch), None);
    assert_eq!(repo.remote_default_name(gix::remote::Direction::Push), None);

    let repo = remote::repo("clone");
    assert_eq!(
        Vec::from_iter(repo.remote_names().into_iter()),
        remote_names(["myself", "origin"])
    );
    assert_eq!(
        repo.remote_default_name(gix::remote::Direction::Fetch),
        Some(remote_name("origin"))
    );
    assert_eq!(
        repo.remote_default_name(gix::remote::Direction::Push),
        Some(remote_name("origin"))
    );
    assert_eq!(Vec::from_iter(repo.branch_names()), vec!["main"]);
}

#[test]
fn remote_default_name() {
    let repo = remote::repo("push-default");

    assert_eq!(
        repo.remote_default_name(gix::remote::Direction::Push),
        Some(remote_name("myself")),
        "overridden via remote.pushDefault"
    );

    assert_eq!(
        repo.remote_default_name(gix::remote::Direction::Fetch),
        None,
        "none if name origin, and there are multiple"
    );
}

mod branch_remote {
    use gix::{config::tree::Push, remote};
    use gix_testtools::TestResult;

    use crate::util::named_subrepo_opts;

    mod name {
        use gix::remote;
        use gix_testtools::TestResult;

        use crate::repository::config::remote::branch_remote::repo;

        #[test]
        fn push() -> TestResult {
            {
                let repo = repo("push-remote")?;

                assert_eq!(
                    repo.branch_remote_name("main", remote::Direction::Push)
                        .expect("Remote name exists")
                        .as_ref(),
                    "push-origin",
                    "branch.main.pushRemote is set"
                );
            }

            let repo = repo("push-remote-default")?;

            assert_eq!(
                repo.branch_remote_name("main", remote::Direction::Push)
                    .expect("Remote name exists")
                    .as_ref(),
                "push-origin",
                "remote.pushDefault is set"
            );

            Ok(())
        }
    }

    #[test]
    fn fetch() -> TestResult {
        let repo = repo("fetch")?;

        assert_eq!(
            repo.branch_remote_ref_name("refs/heads/main".try_into()?, remote::Direction::Fetch)
                .expect("Remote Merge ref exists")?
                .shorten(),
            "main"
        );
        assert_eq!(
            repo.branch_remote_tracking_ref_name("refs/heads/main".try_into()?, remote::Direction::Fetch)
                .expect("Remote Merge ref exists")?,
            "refs/remotes/remote_repo/main"
        );
        let (upstream, remote_name) = repo
            .upstream_branch_and_remote_for_tracking_branch("refs/remotes/remote_repo/main".try_into()?)?
            .expect("mapping exists");
        assert_eq!(upstream, "refs/heads/main");
        assert_eq!(remote_name.name().expect("non-anonymous remote"), "remote_repo");

        assert_eq!(
            repo.upstream_branch_and_remote_for_tracking_branch("refs/remotes/missing-remote/main".try_into()?)?,
            None,
            "It's OK to find nothing"
        );

        for direction in [remote::Direction::Fetch, remote::Direction::Push] {
            assert_eq!(
                repo.branch_remote_name("main", direction).expect("Remote name exists"),
                "remote_repo"
            );
            assert_eq!(
                repo.branch_remote_name("broken", direction)
                    .expect("Remote name exists"),
                "remote_repo"
            );
        }

        assert_eq!(
            repo.branch_remote_ref_name("refs/heads/broken".try_into()?, remote::Direction::Fetch)
                .expect("Remote Merge ref exists")?,
            "refs/heads/not_a_valid_merge_ref",
            "short names are simply turned into branch names - this doesn't always work, but sometimes."
        );
        assert!(
            repo.branch_remote_ref_name("refs/heads/missing".try_into()?, remote::Direction::Fetch)
                .is_none()
        );
        for direction in [remote::Direction::Fetch, remote::Direction::Push] {
            assert_eq!(
                repo.branch_remote_name("broken", direction).expect("is set"),
                "remote_repo"
            );
        }
        assert_eq!(
            repo.branch_remote_tracking_ref_name("refs/heads/broken".try_into()?, remote::Direction::Fetch)
                .expect("no error")?,
            "refs/remotes/remote_repo/not_a_valid_merge_ref",
            "the merge ref is broken, but we turned it into a full ref name from which everything else was derived",
        );

        Ok(())
    }

    #[test]
    fn upstream_branch_and_remote_name_for_tracking_branch() -> TestResult {
        let mut repo = repo("multiple-remotes")?;
        for expected_remote_name in ["other", "with/two"] {
            let (upstream, remote) = repo
                .upstream_branch_and_remote_for_tracking_branch(
                    format!("refs/remotes/{expected_remote_name}/main")
                        .as_str()
                        .try_into()?,
                )?
                .expect("mapping exists");
            assert_eq!(remote.name().expect("named remote"), expected_remote_name);
            assert_eq!(upstream, "refs/heads/main");
        }
        let err = repo
            .upstream_branch_and_remote_for_tracking_branch("refs/remotes/with/two/slashes/main".try_into()?)
            .expect_err("both remotes reverse-map the tracking branch");
        insta::assert_debug_snapshot!(err, "the name is ambiguous because both remotes have fetch refspecs mapping to it", @"Found ambiguous remotes without 1:1 mapping or more than one match: with/two, with/two/slashes");

        let (upstream, remote) = repo
            .upstream_branch_and_remote_for_tracking_branch("refs/remotes/with/two/special".try_into()?)?
            .expect("mapping exists");
        assert_eq!(remote.name().expect("non-anonymous remote"), "with/two");
        assert_eq!(
            upstream, "refs/heads/special",
            "it finds a single mapping even though there are two refspecs"
        );

        repo.config_snapshot_mut().append_config(
            [
                "remote.prefix.url=../fetch",
                "remote.prefix.fetch=+refs/heads/*:refs/remotes/prefix/unrelated/*",
                "remote.fallback.url=../fetch",
                "remote.fallback.fetch=+refs/heads/*:refs/remotes/prefix/*",
            ],
            gix_config::Source::Api,
        )?;
        let (upstream, remote) = repo
            .upstream_branch_and_remote_for_tracking_branch("refs/remotes/prefix/main".try_into()?)?
            .expect("a remote maps the tracking branch");
        assert_eq!(upstream.as_bstr(), "refs/heads/main");
        assert_eq!(
            remote.name().expect("named remote"),
            "fallback",
            "a matching remote-name prefix does not override the actual refspec mapping"
        );

        repo.config_snapshot_mut().append_config(
            [
                "remote.also-fallback.url=../fetch",
                "remote.also-fallback.fetch=+refs/heads/*:refs/remotes/prefix/*",
            ],
            gix_config::Source::Api,
        )?;
        let err = repo
            .upstream_branch_and_remote_for_tracking_branch("refs/remotes/prefix/main".try_into()?)
            .expect_err("multiple remotes are ambiguous");
        insta::assert_debug_snapshot!(err, "all remotes mapping the same tracking branch are reported", @"Found ambiguous remotes without 1:1 mapping or more than one match: also-fallback, fallback");
        Ok(())
    }

    #[test]
    fn upstream_reverse_mapping_preserves_sources_and_matches_destinations_literally() -> TestResult {
        let repo = |name| {
            named_subrepo_opts(
                "make_remote_reverse_mapping_repos.sh",
                name,
                gix::open::Options::isolated(),
            )
        };
        let tracking_branch = "refs/remotes/origin/main".try_into()?;
        for (name, expected) in [
            ("full-source", Some("refs/heads/main")),
            ("partial-source", Some("heads/main")),
            ("head-source", Some("HEAD")),
            ("short-source", None),
            ("object-id-source", None),
        ] {
            let repo = repo(name)?;
            let result = repo.upstream_branch_and_remote_for_tracking_branch(tracking_branch);
            if let Some(expected) = expected {
                let (upstream, remote) = result?.expect("the full destination matches");
                assert_eq!(
                    upstream.as_bstr(),
                    expected,
                    "the parsed source isn't expanded into a branch"
                );
                assert_eq!(
                    remote.name().expect("named remote"),
                    "origin",
                    "the source belongs to origin"
                );
            } else {
                assert!(
                    result.is_err(),
                    "the source in `{name}` cannot be turned into a full reference name offline"
                );
            }
        }
        let repo = repo("abbreviated-destination")?;
        assert!(
            repo.upstream_branch_and_remote_for_tracking_branch(tracking_branch)?
                .is_none(),
            "an abbreviated destination doesn't match a full tracking-reference name"
        );
        Ok(())
    }

    #[test]
    fn push_default() -> TestResult {
        let repo = repo("fetch")?;

        assert_eq!(
            repo.branch_remote_ref_name("refs/heads/main".try_into()?, remote::Direction::Push)
                .expect("exists")?
                .shorten(),
            "main",
            "by default, there is a 1:1 mapping due to `push.default=simple`"
        );

        for direction in [remote::Direction::Fetch, remote::Direction::Push] {
            assert_eq!(
                repo.branch_remote_tracking_ref_name("refs/heads/main".try_into()?, direction)
                    .expect("exists")?,
                "refs/remotes/remote_repo/main",
                "this is a 'simple' mapping of an existing branch, using push.default=simple and the default refspec"
            );
        }

        assert!(
            repo.branch_remote_tracking_ref_name("refs/heads/broken".try_into()?, remote::Direction::Push)
                .is_none(),
            "push.default = simple, hence we need to verify the merge-branch is the same as us, and retrieving it succeeds due to auto-fullnamification but then it doesn't match",
        );

        Ok(())
    }

    #[test]
    fn push_mapped() -> TestResult {
        let repo = repo("push-mapped")?;

        assert_eq!(
            repo.branch_remote_ref_name("refs/heads/main".try_into()?, remote::Direction::Push)
                .expect("exists")?
                .shorten(),
            "remapped-main",
            "the first matching push-spec maps the branch to another head"
        );

        assert_eq!(
            repo.branch_remote_tracking_ref_name("refs/heads/main".try_into()?, remote::Direction::Push)
                .expect("exists")?,
            "refs/remotes/origin/remapped-main",
            "the first matching push-spec maps the branch to another head, then it's mapped with fetch-specs"
        );
        assert_eq!(
            repo.branch_remote_tracking_ref_name("refs/heads/main".try_into()?, remote::Direction::Fetch)
                .expect("exists")?,
            "refs/remotes/origin/main",
            "push.simple is set (or the default), hence it's a one-one mapping along with the standard refspec"
        );

        assert_eq!(
            repo.branch_remote_ref_name("refs/heads/feature".try_into()?, remote::Direction::Fetch)
                .expect("exists")?
                .shorten(),
            "main",
            "branch.feature.merge=refs/heads/main is causing the fetch remote to be remapped"
        );

        assert_eq!(
            repo.branch_remote_ref_name("refs/heads/feature".try_into()?, remote::Direction::Push)
                .expect("exists")?
                .shorten(),
            "remapped-feature",
            "this branch is mapped with push-specs, so we don't run into a failing push.default"
        );

        assert_eq!(
            repo.branch_remote_tracking_ref_name("refs/heads/feature".try_into()?, remote::Direction::Push)
                .expect("exists")?,
            "refs/remotes/origin/remapped-feature",
            "this branch is mapped with push-specs, then it's mapped with fetch-specs as well"
        );
        assert_eq!(
            repo.branch_remote_tracking_ref_name("refs/heads/feature".try_into()?, remote::Direction::Fetch)
                .expect("exists")?,
            "refs/remotes/origin/main",
            "remapping by branch.feature.merge=main, then mapped by refspec"
        );

        Ok(())
    }

    #[test]
    fn push_missing() -> TestResult {
        let repo = repo("push-missing")?;

        assert!(
            repo.branch_remote_ref_name("refs/heads/main".try_into()?, remote::Direction::Push)
                .is_none(),
            "there were push specs, but none matched, and we don't regard the push.default in this case, so end up with no match"
        );

        assert!(
            repo.branch_remote_tracking_ref_name("refs/heads/main".try_into()?, remote::Direction::Push)
                .is_none(),
            "the same thing happens when getting the tracking branch - after all it depends on the remote reference"
        );

        assert_eq!(
            repo.branch_remote_tracking_ref_name("refs/heads/main".try_into()?, remote::Direction::Fetch)
                .expect("exists")?
                .shorten(),
            "origin/main",
            "fetch specs are specified, hence we can get the tracking branch"
        );

        Ok(())
    }

    #[test]
    fn push_default_current() -> TestResult {
        let mut repo = repo("push-default-current")?;

        for same_name_default in ["current", "matching"] {
            repo.config_snapshot_mut()
                .set_value(&Push::DEFAULT, same_name_default)?;
            assert_eq!(
                repo.branch_remote_ref_name("refs/heads/main".try_into()?, remote::Direction::Push)
                    .expect("exists")?
                    .shorten(),
                "main",
                "there was no push spec, `branch.main.merge` points to another branch, but we have a config override"
            );
            assert_eq!(
                repo.branch_remote_tracking_ref_name("refs/heads/main".try_into()?, remote::Direction::Push)
                    .expect("exists")?
                    .shorten(),
                "origin/main",
                "same as above, but retrieves the tracking branch that the remote reference would be tracked under"
            );
        }

        repo.config_snapshot_mut().set_value(&Push::DEFAULT, "upstream")?;
        assert_eq!(
            repo.branch_remote_ref_name("refs/heads/main".try_into()?, remote::Direction::Push)
                .expect("exists")?
                .shorten(),
            "other",
            "`branch.main.merge` is configured as `refs/heads/other`, which is what we use with `push.default=upstream`"
        );
        assert_eq!(
            repo.branch_remote_tracking_ref_name("refs/heads/main".try_into()?, remote::Direction::Push)
                .expect("exists")?
                .shorten(),
            "origin/other",
            "as above, but we convert it to the tracking branch"
        );

        repo.config_snapshot_mut().set_value(&Push::DEFAULT, "simple")?;
        assert_eq!(
            repo.branch_remote_ref_name("refs/heads/main".try_into()?, remote::Direction::Push)
                .transpose()?,
            None,
            "simple requires that the upstream matches the current branch, which isn't the case as `branch.main.merge` points to 'other'"
        );
        assert_eq!(
            repo.branch_remote_tracking_ref_name("refs/heads/main".try_into()?, remote::Direction::Push)
                .transpose()?,
            None,
        );
        Ok(())
    }

    fn repo(name: &str) -> std::result::Result<gix::Repository, gix_error::Error> {
        named_subrepo_opts("make_remote_config_repos.sh", name, gix::open::Options::isolated())
    }
}
