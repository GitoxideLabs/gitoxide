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
        assert_eq!(gix::open_opts(repo.git_dir(), crate::restricted())?, repo);
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
        assert_eq!(gix::open_opts(repo.git_dir(), crate::restricted())?, repo);
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
        assert_eq!(repo.head()?.referent_name().expect("name"), "refs/heads/special");
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
        assert_eq!(repo.head()?.referent_name().expect("name"), "refs/heads/special");
        assert_eq!(
            repo.is_pristine()?,
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
        assert_eq!(gix::open_opts(repo.git_dir(), crate::restricted())?, repo);
        assert_eq!(
            gix::open_opts(repo.workdir().as_ref().expect("non-bare repo"), crate::restricted())?,
            repo
        );
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

mod reftable {
    fn assert_git(git_dir: &std::path::Path, args: &[&str], expected: &str) -> crate::Result {
        let mut arguments = vec![std::ffi::OsString::from("--git-dir"), git_dir.as_os_str().to_owned()];
        arguments.extend(args.iter().map(|arg| std::ffi::OsString::from(*arg)));
        let output = gix_testtools::isolated_git_output_checked(None, arguments)?;
        assert_eq!(
            String::from_utf8(output.stdout)?.trim(),
            expected,
            "Git reports the expected value for {args:?}"
        );
        Ok(())
    }

    fn init_and_verify(kind: gix::create::Kind, object_hash: gix_hash::Kind) -> crate::Result {
        let tmp = gix_testtools::tempfile::TempDir::new()?;
        let destination = tmp.path().join(match kind {
            gix::create::Kind::WithWorktree => "worktree",
            gix::create::Kind::Bare => "bare.git",
        });
        let repo = gix::ThreadSafeRepository::init(
            &destination,
            kind,
            gix::create::Options {
                object_hash: Some(object_hash),
                reference_storage: gix::create::ReferenceStorage::Reftable,
                ..Default::default()
            },
        )?
        .to_thread_local();

        assert_eq!(
            repo.object_hash(),
            object_hash,
            "initialization configures the requested object hash"
        );
        assert_eq!(
            repo.head()?.referent_name().expect("symbolic HEAD"),
            "refs/heads/main",
            "initialization seeds the default symbolic branch in reftable"
        );
        let reference_storage = repo
            .config_snapshot()
            .string("extensions.refStorage")
            .expect("written by reftable initialization");
        assert_eq!(
            reference_storage.as_slice(),
            b"reftable",
            "initialization records the authoritative reference storage"
        );
        assert!(
            repo.git_dir().join("reftable/tables.list").is_file(),
            "initialization publishes the seed reftable generation"
        );
        assert!(
            repo.git_dir().join("refs/heads").is_file(),
            "reftable initialization creates Git's required refs/heads marker file"
        );
        assert_eq!(
            std::fs::read(repo.git_dir().join("HEAD"))?,
            b"ref: refs/heads/.invalid\n",
            "the compatibility HEAD must not be authoritative"
        );

        if !gix_testtools::should_skip_as_git_version_is_smaller_than(2, 45, 0) {
            assert_git(repo.git_dir(), &["symbolic-ref", "HEAD"], "refs/heads/main")?;
            assert_git(
                repo.git_dir(),
                &["rev-parse", "--show-object-format"],
                match object_hash {
                    #[cfg(feature = "sha1")]
                    gix_hash::Kind::Sha1 => "sha1",
                    #[cfg(feature = "sha256")]
                    gix_hash::Kind::Sha256 => "sha256",
                    _ => unreachable!("all enabled object hashes are covered"),
                },
            )?;
            let mut arguments = vec![
                std::ffi::OsString::from("--git-dir"),
                repo.git_dir().as_os_str().to_owned(),
            ];
            arguments.extend(
                ["symbolic-ref", "HEAD", "refs/heads/from-git"]
                    .into_iter()
                    .map(std::ffi::OsString::from),
            );
            gix_testtools::isolated_git_output_checked(None, arguments)?;
            assert_eq!(
                repo.head()?.referent_name().expect("Git keeps HEAD symbolic"),
                "refs/heads/from-git",
                "gix observes Git's update to the initialized reftable repository"
            );
        }
        Ok(())
    }

    #[cfg(feature = "sha1")]
    #[test]
    fn bare_and_non_bare_sha1_repositories_interoperate_with_git() -> crate::Result {
        for kind in [gix::create::Kind::Bare, gix::create::Kind::WithWorktree] {
            init_and_verify(kind, gix_hash::Kind::Sha1)?;
        }
        Ok(())
    }

    #[cfg(feature = "sha256")]
    #[test]
    fn bare_and_non_bare_sha256_repositories_interoperate_with_git() -> crate::Result {
        for kind in [gix::create::Kind::Bare, gix::create::Kind::WithWorktree] {
            init_and_verify(kind, gix_hash::Kind::Sha256)?;
        }
        Ok(())
    }
}
