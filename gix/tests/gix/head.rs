mod peel {
    use crate::util::{hex_to_id, named_subrepo_opts};

    #[test]
    fn all_cases() -> crate::Result {
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
            assert_eq!(repo.head()?.peel_to_object_in_place()?.id, expected_commit);
            assert_eq!(repo.head()?.try_peel_to_id_in_place()?.expect("born"), expected_commit);
        }
        Ok(())
    }
}

mod into_remote {
    use crate::remote;

    #[test]
    fn unborn_is_none() -> crate::Result {
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
    fn detached_is_none() -> crate::Result {
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
