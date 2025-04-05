mod set_namespace {
    use gix::refs::transaction::PreviousValue;
    use gix_testtools::tempfile;

    fn easy_repo_rw() -> crate::Result<(gix::Repository, tempfile::TempDir)> {
        crate::repo_rw("make_references_repo.sh")
    }

    #[test]
    fn affects_edits_and_iteration() -> crate::Result {
        let (mut repo, _keep) = easy_repo_rw()?;
        assert_eq!(
            repo.references()?.all()?.count(),
            17,
            "there are plenty of references in the default namespace"
        );
        assert!(repo.namespace().is_none(), "no namespace is set initially");
        assert!(repo.set_namespace("foo")?.is_none(), "there is no previous namespace");

        assert_eq!(
            repo.references()?.all()?.filter_map(Result::ok).count(),
            0,
            "no references are in the namespace yet"
        );

        repo.tag_reference(
            "new-tag",
            gix::ObjectId::empty_tree(gix::hash::Kind::Sha1),
            PreviousValue::MustNotExist,
        )?;

        repo.reference(
            "refs/heads/new-branch",
            gix::ObjectId::empty_tree(gix::hash::Kind::Sha1),
            PreviousValue::MustNotExist,
            "message",
        )?;

        assert_eq!(
            repo.references()?
                .all()?
                .filter_map(Result::ok)
                .map(|r| r.name().as_bstr().to_owned())
                .collect::<Vec<_>>(),
            vec!["refs/heads/new-branch", "refs/tags/new-tag"],
            "namespaced references appear like normal ones"
        );

        assert_eq!(
            repo.references()?
                .prefixed("refs/tags/")?
                .filter_map(Result::ok)
                .map(|r| r.name().as_bstr().to_owned())
                .collect::<Vec<_>>(),
            vec!["refs/tags/new-tag"],
            "namespaced references appear like normal ones"
        );
        let fully_qualified_tag_name = "refs/tags/new-tag";
        assert_eq!(
            repo.find_reference(fully_qualified_tag_name)?.name().as_bstr(),
            fully_qualified_tag_name,
            "fully qualified (yet namespaced) names work"
        );
        assert_eq!(
            repo.find_reference("new-tag")?.name().as_bstr(),
            fully_qualified_tag_name,
            "namespaces are transparent"
        );

        let previous_ns = repo.clear_namespace().expect("namespace set");
        assert_eq!(previous_ns.as_bstr(), "refs/namespaces/foo/");
        assert!(repo.clear_namespace().is_none(), "it doesn't invent namespaces");

        assert_eq!(
            repo.references()?.all()?.count(),
            19,
            "it lists all references, also the ones in namespaces"
        );
        Ok(())
    }
}

mod iter_references {

    use crate::util::hex_to_id;

    fn repo() -> crate::Result<gix::Repository> {
        crate::repo("make_references_repo.sh").map(|r| r.to_thread_local())
    }

    #[test]
    fn all() -> crate::Result {
        let repo = repo()?;
        assert_eq!(
            repo.references()?
                .all()?
                .filter_map(Result::ok)
                .map(|r| r.name().as_bstr().to_owned())
                .collect::<Vec<_>>(),
            vec![
                "refs/d1",
                "refs/heads/d1",
                "refs/heads/dt1",
                "refs/heads/main",
                "refs/heads/multi-link-target1",
                "refs/loop-a",
                "refs/loop-b",
                "refs/multi-link",
                "refs/remotes/origin/HEAD",
                "refs/remotes/origin/main",
                "refs/remotes/origin/multi-link-target3",
                "refs/tags/dt1",
                "refs/tags/dt2",
                "refs/tags/dt3",
                "refs/tags/multi-link-target2",
                "refs/tags/t1"
            ]
        );
        Ok(())
    }

    #[test]
    fn prefixed() -> crate::Result {
        let repo = repo()?;
        assert_eq!(
            repo.references()?
                .prefixed("refs/heads/")?
                .filter_map(Result::ok)
                .map(|r| (
                    r.name().as_bstr().to_string(),
                    r.target().try_id().map(ToOwned::to_owned)
                ))
                .collect::<Vec<_>>(),
            vec![
                (
                    "refs/heads/d1".to_string(),
                    Some(hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03"))
                ),
                (
                    "refs/heads/dt1".into(),
                    hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03").into()
                ),
                (
                    "refs/heads/main".into(),
                    hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03").into()
                ),
                ("refs/heads/multi-link-target1".into(), None),
            ]
        );
        Ok(())
    }

    #[test]
    fn prefixed_and_peeled() -> crate::Result {
        let repo = repo()?;
        assert_eq!(
            repo.references()?
                .prefixed("refs/heads/")?
                .peeled()?
                .filter_map(Result::ok)
                .map(|r| (
                    r.name().as_bstr().to_string(),
                    r.target().try_id().map(ToOwned::to_owned)
                ))
                .collect::<Vec<_>>(),
            vec![
                (
                    "refs/heads/d1".to_string(),
                    Some(hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03"))
                ),
                (
                    "refs/heads/dt1".into(),
                    hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03").into()
                ),
                (
                    "refs/heads/main".into(),
                    hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03").into()
                ),
                (
                    "refs/remotes/origin/multi-link-target3".into(),
                    hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03").into()
                ),
            ]
        );
        Ok(())
    }
}

mod head {

    use gix_ref::transaction::PreviousValue;

    use crate::util::hex_to_id;

    #[test]
    fn symbolic() -> crate::Result {
        let repo = crate::basic_repo()?;
        let head = repo.head()?;
        match &head.kind {
            gix::head::Kind::Symbolic(r) => {
                assert_eq!(
                    r.target.try_id().map(ToOwned::to_owned),
                    Some(hex_to_id("3189cd3cb0af8586c39a838aa3e54fd72a872a41"))
                );
            }
            _ => panic!("unexpected head kind"),
        }
        assert_eq!(head.referent_name().expect("born").as_bstr(), "refs/heads/main");
        assert!(!head.is_detached());
        Ok(())
    }

    #[test]
    fn detached() -> crate::Result {
        let (repo, _keep) = crate::basic_rw_repo()?;
        repo.reference(
            "HEAD",
            hex_to_id("3189cd3cb0af8586c39a838aa3e54fd72a872a41"),
            PreviousValue::Any,
            "",
        )?;

        let head = repo.head()?;
        assert!(head.is_detached(), "head is detached");
        assert!(head.referent_name().is_none());
        Ok(())
    }
}
