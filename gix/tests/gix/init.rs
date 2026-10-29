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

mod templates {
    use std::{fs, path::Path};

    use gix_testtools::{TestResult, tempfile};

    fn init(path: &Path, template: &Path) -> gix::Result<gix::Repository> {
        Ok(gix::ThreadSafeRepository::init_opts(
            path,
            gix::create::Kind::WithWorktree,
            gix::create::Options::default(),
            gix::open::Options::isolated().config_overrides([
                format!("init.templateDir={}", template.display()),
                "init.defaultBranch=configured".into(),
            ]),
        )?
        .into())
    }

    fn git_init(path: &Path, template: &Path) -> TestResult {
        fs::create_dir_all(path)?;
        let output = gix_testtools::git_command(path)
            .arg("-c")
            .arg(format!("init.templateDir={}", template.display()))
            .args(["-c", "init.defaultBranch=configured", "init", "--quiet"])
            .output()?;
        assert!(output.status.success(), "Git accepts the template: {output:?}");
        Ok(())
    }

    #[test]
    fn files_config_and_head_follow_git() -> TestResult {
        let tmp = tempfile::tempdir()?;
        let template = tmp.path().join("template");
        fs::create_dir_all(template.join("hooks/nested"))?;
        fs::write(template.join("hooks/post-checkout"), "#!/bin/sh\n")?;
        fs::write(template.join("hooks/nested/data"), "nested\n")?;
        fs::write(template.join("hooks/.hidden"), "hidden\n")?;
        fs::write(template.join(".hidden"), "hidden\n")?;
        fs::write(template.join("HEAD"), "ref: refs/heads/template\n")?;
        fs::write(
            template.join("config"),
            "[custom]
                value = preserved
             [core]
                bare = true
                logAllRefUpdates = false
            ",
        )?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::{PermissionsExt, symlink};
            fs::set_permissions(template.join("hooks/post-checkout"), fs::Permissions::from_mode(0o755))?;
            symlink("nested", template.join("hooks/link"))?;
        }
        let native = init(&tmp.path().join("native"), &template)?;
        let git = tmp.path().join("git");
        git_init(&git, &template)?;
        for name in ["HEAD", "hooks/post-checkout", "hooks/nested/data"] {
            assert_eq!(
                fs::read(native.git_dir().join(name))?,
                fs::read(git.join(".git").join(name))?,
                "Git agrees for {name}"
            );
        }
        for name in [
            ".hidden",
            "hooks/.hidden",
            "hooks/pre-commit.sample",
            "description",
            "info/exclude",
        ] {
            assert!(
                !native.git_dir().join(name).exists(),
                "custom templates replace the built-in template: {name}"
            );
        }
        let config = native.config_snapshot();
        assert_eq!(config.string("custom.value"), Some("preserved".into()));
        assert_eq!(config.boolean("core.logAllRefUpdates"), Some(false));
        assert!(
            !native.is_bare(),
            "the requested repository kind overrides template config"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(native.git_dir().join("hooks/post-checkout"))?
                    .permissions()
                    .mode()
                    & 0o777,
                fs::metadata(git.join(".git/hooks/post-checkout"))?.permissions().mode() & 0o777,
                "executable hook modes are retained"
            );
            assert_eq!(fs::read_link(native.git_dir().join("hooks/link"))?, Path::new("nested"));
        }
        Ok(())
    }

    #[test]
    fn empty_missing_and_unsupported_templates_do_not_fall_back_to_builtin() -> TestResult {
        let tmp = tempfile::tempdir()?;
        let unsupported = tmp.path().join("unsupported");
        let unknown = tmp.path().join("unknown-extension");
        for (template, config) in [
            (
                &unsupported,
                "[core]
                repositoryFormatVersion = 100
            ",
            ),
            (
                &unknown,
                "[core]
                    repositoryFormatVersion = 1
                 [extensions]
                    unknown = true
                ",
            ),
        ] {
            fs::create_dir_all(template)?;
            fs::write(template.join("config"), config)?;
            fs::write(template.join("marker"), "must not copy\n")?;
        }
        let missing = tmp.path().join("does-not-exist");
        for (case, template) in [
            ("empty", Path::new("")),
            ("missing", missing.as_path()),
            ("unsupported", unsupported.as_path()),
            ("unknown", unknown.as_path()),
        ] {
            let native = init(&tmp.path().join(format!("native-{case}")), template)?;
            let git = tmp.path().join(format!("git-{case}"));
            git_init(&git, template)?;
            for name in ["marker", "description", "hooks", "info"] {
                assert_eq!(
                    native.git_dir().join(name).exists(),
                    git.join(".git").join(name).exists(),
                    "{case}: {name}"
                );
                assert!(
                    !native.git_dir().join(name).exists(),
                    "skipping templates doesn't copy the default: {case}: {name}"
                );
            }
            assert_eq!(
                native.head()?.referent_name().expect("unborn symbolic HEAD"),
                "refs/heads/configured"
            );
        }
        Ok(())
    }

    #[test]
    fn environment_precedence_and_permissions() -> TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let tmp = tempfile::tempdir()?;
        let configured = tmp.path().join("configured-template");
        let environment = tmp.path().join("environment-template");
        fs::create_dir_all(&configured)?;
        fs::create_dir_all(&environment)?;
        fs::write(configured.join("marker"), "configured\n")?;
        fs::write(environment.join("marker"), "environment\n")?;
        let _env = gix_testtools::Env::new().set("GIT_TEMPLATE_DIR", environment.to_string_lossy());
        for (case, permission, marker) in [
            ("allowed", gix::sec::Permission::Allow, "environment\n"),
            ("denied", gix::sec::Permission::Deny, "configured\n"),
        ] {
            let mut options =
                gix::open::Options::isolated().config_overrides([format!("init.templateDir={}", configured.display())]);
            options.permissions.env.git_prefix = permission;
            let repo = gix::ThreadSafeRepository::init_opts(
                tmp.path().join(case),
                gix::create::Kind::WithWorktree,
                Default::default(),
                options,
            )?
            .to_thread_local();
            assert_eq!(fs::read_to_string(repo.git_dir().join("marker"))?, marker, "{case}");
        }
        let mut forbidden = gix::open::Options::isolated();
        forbidden.permissions.env.git_prefix = gix::sec::Permission::Forbid;
        assert!(
            gix::ThreadSafeRepository::init_opts(
                tmp.path().join("forbidden"),
                gix::create::Kind::WithWorktree,
                Default::default(),
                forbidden,
            )
            .is_err(),
            "forbidden template environment is an error"
        );
        assert!(
            !tmp.path().join("forbidden").exists(),
            "permissions are checked before creating the repository"
        );
        let _empty = gix_testtools::Env::new().set("GIT_TEMPLATE_DIR", "");
        let mut options =
            gix::open::Options::isolated().config_overrides([format!("init.templateDir={}", configured.display())]);
        options.permissions.env.git_prefix = gix::sec::Permission::Allow;
        let repo = gix::ThreadSafeRepository::init_opts(
            tmp.path().join("empty"),
            gix::create::Kind::WithWorktree,
            Default::default(),
            options,
        )?
        .to_thread_local();
        assert!(
            !repo.git_dir().join("marker").exists(),
            "empty environment overrides configured templates"
        );
        assert!(
            !repo.git_dir().join("hooks").exists(),
            "empty environment disables templates"
        );
        Ok(())
    }

    #[test]
    fn configured_paths_expand_home_but_environment_paths_are_literal() -> TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let tmp = tempfile::tempdir()?;
        let home = tmp.path().join("home");
        fs::create_dir_all(home.join("templates"))?;
        fs::create_dir_all(tmp.path().join("~/templates"))?;
        fs::write(home.join("templates/marker"), "home\n")?;
        fs::write(tmp.path().join("~/templates/marker"), "literal\n")?;
        let _cwd = gix_testtools::set_current_dir(tmp.path())?;
        let _home = gix_testtools::Env::new()
            .set("HOME", home.to_string_lossy())
            .unset("GIT_TEMPLATE_DIR");
        let mut options = gix::open::Options::isolated().config_overrides(["init.templateDir=~/templates"]);
        options.permissions.env.home = gix::sec::Permission::Allow;
        options.permissions.env.git_prefix = gix::sec::Permission::Allow;
        for (case, expected) in [("configured", "home\n"), ("environment", "literal\n")] {
            let _env =
                (case == "environment").then(|| gix_testtools::Env::new().set("GIT_TEMPLATE_DIR", "~/templates"));
            let native = gix::ThreadSafeRepository::init_opts(
                tmp.path().join(case),
                gix::create::Kind::WithWorktree,
                Default::default(),
                options.clone(),
            )?
            .to_thread_local();
            assert_eq!(fs::read_to_string(native.git_dir().join("marker"))?, expected, "{case}");
            let git_path = tmp.path().join(format!("git-{case}"));
            fs::create_dir_all(git_path.join("~/templates"))?;
            fs::write(git_path.join("~/templates/marker"), "literal\n")?;
            let mut git = gix_testtools::git_command(&git_path);
            git.env("HOME", &home);
            if case == "environment" {
                git.env("GIT_TEMPLATE_DIR", "~/templates");
            }
            let output = git
                .args(["-c", "init.templateDir=~/templates", "init", "--quiet"])
                .output()?;
            assert!(output.status.success(), "Git accepts {case}: {output:?}");
            assert_eq!(fs::read_to_string(git_path.join(".git/marker"))?, expected);
        }
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn required_paths_do_not_write_through_template_symlinks() -> TestResult {
        use std::os::unix::fs::symlink;

        let tmp = tempfile::tempdir()?;
        let template = tmp.path().join("template");
        let outside = tmp.path().join("outside");
        fs::create_dir_all(&template)?;
        fs::create_dir_all(&outside)?;
        let contents = "[custom]
            value = preserved
        ";
        fs::write(outside.join("config"), contents)?;
        symlink(outside.join("config"), template.join("config"))?;
        symlink(&outside, template.join("objects"))?;
        symlink(&outside, template.join("refs"))?;
        let native = init(&tmp.path().join("native"), &template)?;
        assert_eq!(
            fs::read_to_string(outside.join("config"))?,
            contents,
            "updating repository config leaves the template target untouched"
        );
        assert!(
            !native.git_dir().join("config").symlink_metadata()?.is_symlink(),
            "updated configuration replaces the copied link"
        );
        assert_eq!(
            native.config_snapshot().string("custom.value"),
            Some("preserved".into())
        );
        for path in ["info", "pack", "heads", "tags"] {
            assert!(
                !outside.join(path).exists(),
                "required directories stay inside the new repository: {path}"
            );
        }
        Ok(())
    }

    #[test]
    fn bare_and_clone_creation_share_templates() -> TestResult {
        let tmp = tempfile::tempdir()?;
        let template = tmp.path().join("template");
        fs::create_dir_all(&template)?;
        fs::write(template.join("marker"), "custom\n")?;
        fs::write(
            template.join("config"),
            "[core]
            logAllRefUpdates
        ",
        )?;
        let options =
            gix::open::Options::isolated().config_overrides([format!("init.templateDir={}", template.display())]);
        let bare = gix::ThreadSafeRepository::init_opts(
            tmp.path().join("bare"),
            gix::create::Kind::Bare,
            Default::default(),
            options.clone(),
        )?
        .to_thread_local();
        assert_eq!(
            bare.config_snapshot().boolean("core.logAllRefUpdates"),
            Some(true),
            "implicit template values are preserved even for bare repositories"
        );
        let repo: gix::Repository = gix::clone::PrepareFetch::new(
            bare.git_dir(),
            tmp.path().join("clone"),
            gix::create::Kind::WithWorktree,
            Default::default(),
            options,
        )?
        .into();
        assert_eq!(fs::read_to_string(repo.git_dir().join("marker"))?, "custom\n");
        assert_eq!(repo.config_snapshot().boolean("core.logAllRefUpdates"), Some(true));
        let filtered = gix::ThreadSafeRepository::init_opts(
            tmp.path().join("filtered"),
            gix::create::Kind::WithWorktree,
            Default::default(),
            gix::open::Options::isolated()
                .config_overrides([format!("init.templateDir={}", template.display())])
                .filter_config_section(|_| false),
        )?
        .to_thread_local();
        assert!(
            !filtered.git_dir().join("marker").exists(),
            "excluded config cannot select templates"
        );
        assert!(
            filtered.git_dir().join("description").exists(),
            "filtered template config leaves the built-in default selected"
        );
        Ok(())
    }

    #[test]
    #[cfg(feature = "sha256")]
    fn sha256_does_not_activate_unknown_template_extensions() -> TestResult {
        let tmp = tempfile::tempdir()?;
        let template = tmp.path().join("template");
        fs::create_dir_all(&template)?;
        fs::write(
            template.join("config"),
            "[core]
            repositoryFormatVersion = 0
            [extensions]
            unknown = true
        ",
        )?;
        fs::write(template.join("marker"), "must not copy\n")?;
        let repo = gix::ThreadSafeRepository::init_opts(
            tmp.path().join("repo"),
            gix::create::Kind::WithWorktree,
            gix::create::Options {
                object_hash: Some(gix::hash::Kind::Sha256),
                ..Default::default()
            },
            gix::open::Options::isolated().config_overrides([format!("init.templateDir={}", template.display())]),
        )?
        .to_thread_local();
        assert!(
            !repo.git_dir().join("marker").exists(),
            "SHA256 requires v1, where the template's ignored extension would become significant"
        );
        assert_eq!(repo.object_hash(), gix::hash::Kind::Sha256);
        assert_eq!(
            gix_testtools::git(repo.workdir().expect("non-bare"), "rev-parse --show-object-format")?,
            "sha256\n"
        );
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
