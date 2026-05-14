mod bare {
    use gix_testtools::tempfile;

    #[test]
    fn init_into_non_existing_directory_creates_it() -> crate::Result {
        let tmp = tempfile::tempdir()?;
        let git_dir = tmp.path().join("bare.git");
        let repo = gix::init_bare(&git_dir)?;
        assert_eq!(repo.kind(), gix::repository::Kind::Common);
        assert!(
            repo.workdir().is_none(),
            "a worktree isn't present in bare repositories"
        );
        assert_eq!(
            repo.git_dir(),
            git_dir,
            "the repository is placed into the given directory without added sub-directories"
        );
        assert_eq!(gix::open(repo.git_dir())?, repo);
        Ok(())
    }

    #[test]
    fn init_into_empty_directory_uses_it_directly() -> crate::Result {
        let tmp = tempfile::tempdir()?;
        let repo = gix::init_bare(tmp.path())?;
        assert_eq!(repo.kind(), gix::repository::Kind::Common);
        assert!(
            repo.workdir().is_none(),
            "a worktree isn't present in bare repositories"
        );
        assert_eq!(
            repo.git_dir(),
            tmp.path(),
            "the repository is placed into the directory itself"
        );
        assert_eq!(gix::open(repo.git_dir())?, repo);
        Ok(())
    }

    #[test]
    fn init_into_non_empty_directory_is_not_allowed() -> crate::Result {
        let tmp = tempfile::tempdir()?;
        std::fs::write(tmp.path().join("existing.txt"), b"I was here before you")?;

        assert!(
            gix::init_bare(tmp.path())
                .unwrap_err()
                .to_string()
                .starts_with("Refusing to initialize the non-empty directory as")
        );
        Ok(())
    }
}

mod non_bare {
    use gix_testtools::tempfile;

    #[test]
    fn init_bare_with_custom_branch_name() -> crate::Result {
        let tmp = tempfile::tempdir()?;
        let repo: gix::Repository = gix::ThreadSafeRepository::init_opts(
            tmp.path(),
            gix::create::Kind::Bare,
            gix::create::Options::default(),
            gix::open::Options::isolated().config_overrides([
                "user.name=a",
                "user.email=b",
                "init.defaultBranch=special",
            ]),
        )?
        .into();
        assert_eq!(
            repo.head()?.referent_name().expect("name").as_bstr(),
            "refs/heads/special"
        );
        Ok(())
    }

    #[test]
    fn init_bare_with_fully_qualified_custom_branch_name_is_not_prefixed_again() -> crate::Result {
        let tmp = tempfile::tempdir()?;
        let repo: gix::Repository = gix::ThreadSafeRepository::init_opts(
            tmp.path(),
            gix::create::Kind::Bare,
            gix::create::Options::default(),
            gix::open::Options::isolated().config_overrides([
                "user.name=a",
                "user.email=b",
                "init.defaultBranch=refs/heads/special",
            ]),
        )?
        .into();
        assert_eq!(
            repo.head()?.referent_name().expect("name").as_bstr(),
            "refs/heads/special"
        );
        assert_eq!(
            repo.is_pristine(),
            Some(true),
            "the expected default ref uses the de-duplicated fully qualified branch name"
        );
        Ok(())
    }

    #[test]
    fn init_bare_rejects_reserved_branch_name() -> crate::Result {
        let tmp = tempfile::tempdir()?;
        let err = gix::ThreadSafeRepository::init_opts(
            tmp.path(),
            gix::create::Kind::Bare,
            gix::create::Options::default(),
            gix::open::Options::isolated().config_overrides(["user.name=a", "user.email=b", "init.defaultBranch=HEAD"]),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            gix::init::Error::InvalidBranchName {
                name,
                source: gix_validate::reference::name::Error::Reserved { name: reserved }
            } if name == "HEAD" && reserved == "refs/heads/HEAD"
        ));
        Ok(())
    }

    #[test]
    fn init_bare_rejects_reserved_fully_qualified_branch_name() -> crate::Result {
        let tmp = tempfile::tempdir()?;
        let err = gix::ThreadSafeRepository::init_opts(
            tmp.path(),
            gix::create::Kind::Bare,
            gix::create::Options::default(),
            gix::open::Options::isolated().config_overrides([
                "user.name=a",
                "user.email=b",
                "init.defaultBranch=refs/heads/HEAD",
            ]),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            gix::init::Error::InvalidBranchName {
                name,
                source: gix_validate::reference::name::Error::Reserved { name: reserved }
            } if name == "refs/heads/HEAD" && reserved == "refs/heads/HEAD"
        ));
        Ok(())
    }

    #[test]
    fn init_into_empty_directory_creates_a_dot_git_dir() -> crate::Result {
        let tmp = tempfile::tempdir()?;
        let repo = gix::init(tmp.path())?;
        assert_eq!(repo.kind(), gix::repository::Kind::Common);
        assert_eq!(repo.workdir(), Some(tmp.path()), "there is a work tree by default");
        assert_eq!(
            repo.git_dir(),
            tmp.path().join(".git"),
            "there is a work tree by default"
        );
        assert_eq!(gix::open(repo.git_dir())?, repo);
        assert_eq!(gix::open(repo.workdir().as_ref().expect("non-bare repo"))?, repo);
        Ok(())
    }

    #[test]
    fn init_into_non_empty_directory_is_allowed_if_option_is_none_or_false() -> crate::Result {
        for destination_must_be_empty in [None, Some(false)] {
            let tmp = tempfile::tempdir()?;
            std::fs::write(tmp.path().join("existing.txt"), b"I was here before you")?;
            let repo: gix::Repository = gix::ThreadSafeRepository::init_opts(
                tmp.path(),
                gix::create::Kind::WithWorktree,
                gix::create::Options {
                    destination_must_be_empty,
                    ..Default::default()
                },
                gix::open::Options::isolated(),
            )?
            .into();
            assert_eq!(repo.workdir().expect("present"), tmp.path());
            assert_eq!(
                repo.git_dir(),
                tmp.path().join(".git"),
                "gitdir is inside of the workdir"
            );
        }
        Ok(())
    }

    #[test]
    fn init_into_non_empty_directory_is_not_allowed_if_option_is_true() -> crate::Result {
        let tmp = tempfile::tempdir()?;
        std::fs::write(tmp.path().join("existing.txt"), b"I was here before you")?;

        let err = gix::ThreadSafeRepository::init_opts(
            tmp.path(),
            gix::create::Kind::WithWorktree,
            gix::create::Options {
                destination_must_be_empty: Some(true),
                ..Default::default()
            },
            gix::open::Options::isolated(),
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .starts_with("Refusing to initialize the non-empty directory as")
        );
        Ok(())
    }
}
