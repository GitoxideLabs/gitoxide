mod bare {
    use gix_testtools::TestResult;
    use gix_testtools::tempfile;

    #[test]
    #[serial_test::serial]
    fn init_into_non_existing_directory_creates_it() -> TestResult {
        let _environment = gix_testtools::isolate_git_environment()?;
        let tmp = tempfile::tempdir()?;
        let git_dir = tmp.path().join("missing/bare.git");
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
    #[serial_test::serial]
    fn init_into_empty_directory_uses_it_directly() -> TestResult {
        let _environment = gix_testtools::isolate_git_environment()?;
        super::assert_init_into_empty_directory(gix::create::Kind::Bare)
    }

    #[test]
    fn init_into_non_empty_directory_is_not_allowed() -> TestResult {
        let tmp = tempfile::tempdir()?;
        std::fs::write(tmp.path().join("existing.txt"), b"I was here before you")?;

        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&(gix::init_bare(tmp.path())
                .expect_err("init into non empty directory is not allowed")), &[(&(tmp.path()).to_string_lossy(), "<destination>")]), "init into non empty directory is not allowed", @r#"Refusing to initialize the non-empty directory as, input="<destination>""#);
        Ok(())
    }
}

mod non_bare {
    use gix_testtools::TestResult;
    use gix_testtools::tempfile;

    #[test]
    fn init_bare_with_custom_branch_name() -> TestResult {
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
    fn init_bare_with_fully_qualified_custom_branch_name_is_not_prefixed_again() -> TestResult {
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
            repo.is_pristine(),
            Some(true),
            "the expected default ref uses the de-duplicated fully qualified branch name"
        );
        Ok(())
    }

    #[test]
    fn init_bare_rejects_reserved_branch_name() -> TestResult {
        let tmp = tempfile::tempdir()?;
        let err = gix::ThreadSafeRepository::init_opts(
            tmp.path(),
            gix::create::Kind::Bare,
            gix::create::Options::default(),
            gix::open::Options::isolated().config_overrides(["user.name=a", "user.email=b", "init.defaultBranch=HEAD"]),
        )
        .unwrap_err();
        insta::assert_debug_snapshot!(err, "init bare rejects reserved branch name", @r#"
        Invalid default branch name, input="HEAD"

        Caused by:
            0: Reference name is reserved and cannot be used: "refs/heads/HEAD"
        "#);
        assert!(matches!(
            err.classify().filter(|classification| classification.class() == gix_error::Class::Validation)
                .find_map(|classification| classification.error().downcast_ref::<gix_error::Message>()),
            Some(gix_error::Message { values, .. }) if values.get("input") == Some(&gix_error::MetadataValue::Bytes("HEAD".into()))
        ));
        assert!(matches!(
            err.downcast_any_ref(),
            Some(gix_validate::reference::name::Error::Reserved { name }) if name == "refs/heads/HEAD"
        ));
        Ok(())
    }

    #[test]
    fn init_bare_rejects_reserved_fully_qualified_branch_name() -> TestResult {
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
        insta::assert_debug_snapshot!(err, "init bare rejects reserved fully qualified branch name", @r#"
        Invalid default branch name, input="refs/heads/HEAD"

        Caused by:
            0: Reference name is reserved and cannot be used: "refs/heads/HEAD"
        "#);
        assert!(matches!(
            err.classify().filter(|classification| classification.class() == gix_error::Class::Validation)
                .find_map(|classification| classification.error().downcast_ref::<gix_error::Message>()),
            Some(gix_error::Message { values, .. }) if values.get("input") == Some(&gix_error::MetadataValue::Bytes("refs/heads/HEAD".into()))
        ));
        assert!(matches!(
            err.downcast_any_ref(),
            Some(gix_validate::reference::name::Error::Reserved { name }) if name == "refs/heads/HEAD"
        ));
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn init_into_empty_directory_creates_a_dot_git_dir() -> TestResult {
        let _environment = gix_testtools::isolate_git_environment()?;
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
        super::assert_init_into_empty_directory(gix::create::Kind::WithWorktree)
    }

    #[test]
    fn init_into_non_empty_directory_is_allowed_if_option_is_none_or_false() -> TestResult {
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
    fn init_into_non_empty_directory_is_not_allowed_if_option_is_true() -> TestResult {
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
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&(err), &[(&(tmp.path()).to_string_lossy(), "<destination>")]), "init into non empty directory is not allowed if option is true", @r#"Refusing to initialize the non-empty directory as, input="<destination>""#);
        Ok(())
    }
}

#[test]
fn invalid_default_object_format_is_an_error_unless_explicitly_overridden() -> gix_testtools::TestResult {
    for repo_kind in [gix::create::Kind::Bare, gix::create::Kind::WithWorktree] {
        for &requested_object_hash in OBJECT_FORMATS {
            let tmp = gix_testtools::tempfile::tempdir()?;
            let result = gix::ThreadSafeRepository::init_opts(
                tmp.path(),
                repo_kind,
                gix::create::Options {
                    object_hash: requested_object_hash,
                    ..Default::default()
                },
                crate::restricted().config_overrides(["init.defaultObjectFormat=invalid"]),
            );
            if let Some(expected_object_hash) = requested_object_hash {
                assert_eq!(
                    result?.to_thread_local().object_hash(),
                    expected_object_hash,
                    "an explicit hash bypasses an invalid configured default for {repo_kind:?}"
                );
            } else {
                let err = result.expect_err("an unknown default hash must fail initialization");
                assert!(err.is_validation());
                assert_eq!(
                    err.metadata_merged().get("key"),
                    Some(&gix_error::MetadataValue::from("init.defaultObjectFormat")),
                    "the error identifies the invalid configuration key"
                );
                assert!(
                    !tmp.path().join("HEAD").exists() && !tmp.path().join(".git").exists(),
                    "invalid defaults are rejected before creating a repository"
                );
            }
        }
    }
    Ok(())
}

const OBJECT_FORMATS: &[Option<gix_hash::Kind>] = &[
    None,
    #[cfg(feature = "sha1")]
    Some(gix_hash::Kind::Sha1),
    #[cfg(feature = "sha256")]
    Some(gix_hash::Kind::Sha256),
];

fn assert_init_into_empty_directory(repo_kind: gix::create::Kind) -> gix_testtools::TestResult {
    use gix::config::tree::{Core, Extensions, Init, Key};

    #[cfg(feature = "sha1")]
    let build_default_hash = gix_hash::Kind::Sha1;
    #[cfg(not(feature = "sha1"))]
    let build_default_hash = gix_hash::Kind::Sha256;
    for &default_object_format in OBJECT_FORMATS {
        for &requested_object_hash in OBJECT_FORMATS {
            let case = format!(
                "repo_kind={repo_kind:?}, requested_object_hash={requested_object_hash:?}, default_object_format={default_object_format:?}"
            );
            let tmp = gix_testtools::tempfile::tempdir()?;
            let overrides = default_object_format
                .map(|format| Init::DEFAULT_OBJECT_FORMAT.validated_assignment_fmt(&format))
                .transpose()?;
            let repo = gix::ThreadSafeRepository::init_opts(
                tmp.path(),
                repo_kind,
                gix::create::Options {
                    object_hash: requested_object_hash,
                    ..Default::default()
                },
                crate::restricted().config_overrides(overrides),
            )?
            .to_thread_local();
            assert_eq!(repo.kind(), gix::repository::Kind::Common, "{case}");
            match repo_kind {
                gix::create::Kind::Bare => {
                    assert!(repo.workdir().is_none(), "bare repositories have no worktree: {case}");
                    assert_eq!(
                        repo.git_dir(),
                        tmp.path(),
                        "bare repositories use the directory itself: {case}"
                    );
                }
                gix::create::Kind::WithWorktree => {
                    assert_eq!(
                        repo.workdir(),
                        Some(tmp.path()),
                        "the worktree uses the destination: {case}"
                    );
                    assert_eq!(
                        repo.git_dir(),
                        tmp.path().join(".git"),
                        "the git directory is inside the worktree: {case}"
                    );
                }
            }
            let expected_object_hash = requested_object_hash
                .or(default_object_format)
                .unwrap_or(build_default_hash);
            assert_eq!(
                repo.object_hash(),
                expected_object_hash,
                "explicit request, then configured default, then build default: {case}"
            );
            let (expected_repository_format_version, expected_object_format): (i64, Option<&str>) =
                match expected_object_hash {
                    #[cfg(feature = "sha1")]
                    gix_hash::Kind::Sha1 => (0, None),
                    #[cfg(feature = "sha256")]
                    gix_hash::Kind::Sha256 => (1, Some("sha256")),
                    _ => unreachable!("the cases only use enabled hashes"),
                };
            let config =
                gix::config::File::from_path_no_includes(repo.git_dir().join("config"), gix::config::Source::Local)?;
            assert_eq!(
                config.integer(Core::REPOSITORY_FORMAT_VERSION)?,
                Some(expected_repository_format_version),
                "the on-disk repository version matches its object format: {case}"
            );
            assert_eq!(
                config.string(Extensions::OBJECT_FORMAT),
                expected_object_format.map(Into::into),
                "only SHA-256 needs an object-format extension: {case}"
            );
            let reopened = gix::open_opts(repo.git_dir(), crate::restricted())?;
            assert_eq!(
                reopened.object_hash(),
                expected_object_hash,
                "the object hash survives reopening: {case}"
            );
            assert_eq!(reopened, repo, "the repository paths survive reopening: {case}");
        }
    }
    Ok(())
}
