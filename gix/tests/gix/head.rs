mod indirect {
    use crate::util::named_subrepo_opts;
    use gix_testtools::TestResult;

    #[test]
    fn symbolic_chains_match_git() -> TestResult {
        for state in ["empty", "staged", "committed", "packed"] {
            let repo = named_subrepo_opts("make_indirect_head_repos.sh", state, gix::open::Options::isolated())?;
            let head = repo.head()?;
            let expected_name = std::fs::read_to_string(repo.git_dir().join("head-name.git"))?;
            assert_eq!(
                head.referent_name().expect("symbolic HEAD").as_bstr(),
                expected_name.trim(),
                "HEAD follows symbolic branches to the same referent as Git in {state}"
            );
            assert_eq!(head.is_unborn(), matches!(state, "empty" | "staged"));
            assert_eq!(
                repo.head_tree_id_or_empty()?.to_string(),
                std::fs::read_to_string(repo.git_dir().join("tree.git"))?.trim(),
                "indirect unborn HEADs use the empty tree in {state}"
            );

            #[cfg(feature = "status")]
            {
                use std::fmt::Write;

                let items = repo
                    .status(gix::progress::Discard)?
                    .into_iter(None)?
                    .collect::<gix::Result<Vec<_>>>()?;
                let mut actual = String::new();
                for item in items {
                    match item {
                        gix::status::Item::TreeIndex(gix::diff::index::Change::Addition { location, .. }) => {
                            writeln!(actual, "A  {location}")?;
                        }
                        other => panic!("unexpected status in {state}: {other:?}"),
                    }
                }
                let expected = std::fs::read_to_string(repo.git_dir().join("status.git"))?;
                assert_eq!(actual, expected, "indirect HEAD status agrees with Git in {state}");
                assert_eq!(
                    repo.is_dirty()?,
                    !expected.is_empty(),
                    "staged additions make HEAD dirty"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn symbolic_cycles_are_not_unborn() -> TestResult {
        let repo = named_subrepo_opts("make_indirect_head_repos.sh", "cycle", gix::open::Options::isolated())?;
        assert!(
            repo.head().is_err(),
            "a symbolic cycle cannot be classified as an unborn branch"
        );
        assert!(
            repo.head_tree_id_or_empty().is_err(),
            "a cycle cannot silently become an empty tree"
        );
        Ok(())
    }
}

mod peel {
    use crate::util::{hex_to_id, named_subrepo_opts};
    use gix_testtools::TestResult;

    #[test]
    fn all_cases() -> TestResult {
        let expected_commit = hex_to_id("fafd9d08a839d99db60b222cd58e2e0bfaf1f7b2");
        for name in ["detached", "symbolic", "tag-detached", "tag-symbolic"] {
            let repo = named_subrepo_opts("make_head_repos.sh", name, gix::open::Options::isolated())?;
            assert_eq!(repo.head()?.into_peeled_id()?, expected_commit);
            assert_eq!(repo.head()?.into_peeled_object()?.id, expected_commit);
            assert_eq!(repo.head_id()?, expected_commit);
            let commit = repo.head_commit()?;
            assert_eq!(commit.id, expected_commit);
            assert_eq!(repo.head_tree_id()?, commit.tree_id()?);
            assert_eq!(repo.head_tree_id_or_empty()?, commit.tree_id()?);
            assert_eq!(repo.head()?.try_into_peeled_id()?.expect("born"), expected_commit);
            assert_eq!(repo.head()?.peel_to_object()?.id, expected_commit);
            assert_eq!(repo.head()?.try_peel_to_id()?.expect("born"), expected_commit);
        }
        Ok(())
    }

    #[test]
    fn a_missing_detached_head_object_is_not_an_empty_tree() -> TestResult {
        let (repo, _keep) = crate::basic_rw_repo()?;
        let mut missing_id = repo.object_hash().null();
        missing_id.as_mut_slice()[0] = 1;
        repo.reference("HEAD", missing_id, gix::refs::transaction::PreviousValue::Any, "")?;

        let err = repo
            .head_tree_id_or_empty()
            .expect_err("a born HEAD with a missing object must remain an error");
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&(err), &[]), "a missing detached head object is not an empty tree", @"An object with id Oid(1) could not be found");
        assert!(err.is_not_found());
        Ok(())
    }
}

mod into_remote {
    use crate::remote;
    use gix_testtools::TestResult;

    #[test]
    fn unborn_is_none() -> TestResult {
        let repo = remote::repo("url-rewriting");
        assert_eq!(
            repo.head()?.into_remote(gix::remote::Direction::Fetch).transpose()?,
            None
        );
        assert_eq!(
            repo.find_fetch_remote(None)?.name().expect("present").as_ref(),
            "origin",
            "we can fallback to the only available remote"
        );
        Ok(())
    }

    #[test]
    fn detached_is_none() -> TestResult {
        let repo = remote::repo("detached-head");
        assert_eq!(
            repo.head()?.into_remote(gix::remote::Direction::Fetch).transpose()?,
            None
        );
        assert_eq!(
            repo.find_fetch_remote(None)?.name().expect("present").as_ref(),
            "origin",
            "we can fallback to the only available remote"
        );
        Ok(())
    }
}
