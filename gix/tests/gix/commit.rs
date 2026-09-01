#[cfg(feature = "revision")]
mod describe {
    use gix::commit::describe::SelectRef::{AllRefs, AllTags, AnnotatedTags};

    use crate::named_repo;

    #[cfg(feature = "status")]
    mod with_dirty_suffix {
        use gix::commit::describe::SelectRef;

        use crate::util::named_subrepo_opts;

        #[test]
        fn dirty_suffix_applies_automatically_if_dirty() -> crate::Result {
            let repo = named_subrepo_opts(
                "make_submodules.sh",
                "submodule-head-changed",
                gix::open::Options::isolated(),
            )?;

            let actual = repo
                .head_commit()?
                .describe()
                .names(SelectRef::AllRefs)
                .try_resolve()?
                .expect("resolution")
                .format_with_dirty_suffix("dirty".to_owned())?
                .to_string();
            assert_eq!(actual, "main-dirty");
            Ok(())
        }

        #[test]
        fn dirty_suffix_does_not_apply_if_not_dirty() -> crate::Result {
            let repo = named_subrepo_opts("make_submodules.sh", "module1", gix::open::Options::isolated())?;

            let actual = repo
                .head_commit()?
                .describe()
                .names(SelectRef::AllRefs)
                .try_resolve()?
                .expect("resolution")
                .format_with_dirty_suffix("dirty".to_owned())?
                .to_string();
            assert_eq!(actual, "main");
            Ok(())
        }
    }

    #[test]
    fn tags_are_sorted_by_date_and_lexicographically() -> crate::Result {
        let repo = named_repo("make_commit_describe_multiple_tags.sh")?;
        let mut describe = repo.head_commit()?.describe();
        for filter in &[AnnotatedTags, AllTags, AllRefs] {
            describe = describe.names(*filter);
            assert_eq!(describe.format()?.to_string(), "v4", "{filter:?}");
        }
        Ok(())
    }

    #[test]
    fn tags_are_sorted_by_priority() -> crate::Result {
        let repo = named_repo("make_commit_describe_multiple_tags.sh")?;
        let commit = repo.find_reference("refs/tags/v0")?.id().object()?.into_commit();
        let mut describe = commit.describe();
        for filter in &[AnnotatedTags, AllTags, AllRefs] {
            describe = describe.names(*filter);
            assert_eq!(describe.format()?.to_string(), "v1", "{filter:?}");
        }
        Ok(())
    }

    #[test]
    fn lightweight_tags_are_sorted_lexicographically() -> crate::Result {
        let repo = named_repo("make_commit_describe_multiple_tags.sh")?;
        let commit = repo.find_reference("refs/tags/l0")?.id().object()?.into_commit();
        let mut describe = commit.describe();
        for filter in &[AnnotatedTags, AllTags, AllRefs] {
            describe = describe.names(*filter);
            let expected = match filter {
                AnnotatedTags => None,
                _ => Some("l0"),
            };
            let actual = describe.try_format()?.map(|f| f.to_string());
            assert_eq!(actual.as_deref(), expected, "{filter:?}");
        }
        Ok(())
    }
}

#[cfg(feature = "command")]
mod signature {
    use std::process::Command;

    use gix::config::tree::{Gpg, Key, User, gpg};
    use gix_testtools::signature;
    use serial_test::serial;

    #[test]
    // Git inherits process-global date variables modified by fixture setup in other tests, which can otherwise fail with
    // `fatal: invalid date format: 42 +0030` while creating the reference signature.
    #[serial]
    #[cfg_attr(windows, ignore = "TODO: requires direct investigation on Windows")]
    fn verifies_a_commit_signed_by_git_with_ssh() -> crate::Result {
        if !signature::program_available("ssh-keygen") {
            return Ok(());
        }
        let (_key_home, key) = signature::ssh_private_key()?;
        // Git writes the signed commit.
        let fixture = gix_testtools::scripted_fixture_writable("make_basic_repo.sh")?;
        let email = User::EMAIL.validated_assignment_fmt(&signature::IDENTITY)?;
        // Git for Windows passes this native path to `ssh-keygen`, where forward slashes avoid backslash
        // interpretation while retaining the drive prefix. An MSYS path like `/c/...` isn't accepted here.
        let key_for_git = gix_path::to_unix_separators_on_windows(gix_path::into_bstr(&key));
        let signing_key = User::SIGNING_KEY.validated_assignment(key_for_git.as_ref())?;
        let output = Command::new(gix_path::env::exe_invocation())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", if cfg!(windows) { "NUL" } else { "/dev/null" })
            .env("GIT_CONFIG_COUNT", "0")
            .arg("-C")
            .arg(fixture.path())
            .args(["-c", "user.name=Gitoxide Signing Fixture", "-c"])
            .arg(gix_path::from_bstring(email).into_os_string())
            .args(["-c", "gpg.format=ssh", "-c"])
            .arg(gix_path::from_bstring(signing_key).into_os_string())
            .args(["commit", "--allow-empty", "-S", "-m", "signed by Git"])
            .output()?;
        assert!(
            output.status.success(),
            "Git creates the reference signature: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let allowed_signers = gpg::Ssh::ALLOWED_SIGNERS_FILE
            .validated_assignment_fmt(&signature::fixture("ssh-allowed-signers").display())?;
        let repo = gix::open_opts(
            fixture.path(),
            gix::open::Options::isolated().config_overrides([allowed_signers]),
        )?;
        let outcome = repo
            .head_commit()?
            .verify_signature()?
            .expect("Git created a signed commit");
        assert!(outcome.is_valid(), "the Git-generated signature is valid");
        assert_eq!(outcome.format, gix::commit::verify::Format::Ssh);
        assert_eq!(
            outcome.signer.as_ref().map(|signer| signer.as_slice()),
            Some(signature::IDENTITY.as_bytes())
        );
        Ok(())
    }

    #[test]
    fn sign_write_and_verify_an_ssh_commit() -> crate::Result {
        let (_key_home, key) = signature::ssh_private_key()?;
        let options = gix::open::Options::isolated().config_overrides([
            User::NAME.validated_assignment_fmt(&"Gitoxide Signing Fixture")?,
            User::EMAIL.validated_assignment_fmt(&signature::IDENTITY)?,
            Gpg::FORMAT.validated_assignment_fmt(&"ssh")?,
            User::SIGNING_KEY.validated_assignment_fmt(&key.display())?,
            gpg::Ssh::ALLOWED_SIGNERS_FILE
                .validated_assignment_fmt(&signature::fixture("ssh-allowed-signers").display())?,
        ]);
        let repo = crate::util::repo_opts("make_basic_repo.sh", options)?
            .to_thread_local()
            .with_object_memory();
        let mut signing_options = repo.commit_signing_options()?;
        assert_eq!(signing_options.format, gix::commit::sign::Format::Ssh);
        let expected_program = if cfg!(windows) {
            gix::path::env::installation_program("ssh-keygen").unwrap_or_else(|| "ssh-keygen".into())
        } else {
            "ssh-keygen".into()
        };
        assert_eq!(signing_options.program, expected_program.into_os_string());
        if !signature::program_available(&signing_options.program) {
            return Ok(());
        }
        assert_eq!(signing_options.signing_key, key);
        assert!(signing_options.program_arguments.is_empty());
        signing_options.program_arguments.push("-q".into());
        let signed = repo.head_commit()?.decode()?.sign(signing_options)?;
        let id = repo.write_object(&signed)?;
        assert!(
            repo.find_commit(id)?
                .verify_signature()?
                .expect("the written commit has a signature")
                .is_valid(),
            "the configured SSH verifier accepts plumbing options resolved and adjusted by the caller"
        );

        let signed = repo.head_commit()?.signed()?;
        let outcome = signed.verify_signature()?.expect("the written commit has a signature");
        assert!(outcome.is_valid(), "the configured SSH verifier accepts the signature");
        assert_eq!(outcome.format, gix::commit::verify::Format::Ssh);
        Ok(())
    }

    #[test]
    fn resolves_format_defaults_and_program_paths() -> crate::Result {
        let home = gix::path::env::home_dir().expect("the test environment has a home directory");
        let mut permissions = gix::open::Permissions::isolated();
        permissions.env.home = gix::sec::Permission::Allow;
        let options = gix::open::Options::isolated()
            .permissions(permissions)
            .config_overrides([
                User::NAME.validated_assignment_fmt(&"Gitoxide Signing Fixture")?,
                User::EMAIL.validated_assignment_fmt(&"signing@example.com")?,
                Gpg::FORMAT.validated_assignment_fmt(&"x509")?,
                gpg::X509::PROGRAM.validated_assignment_fmt(&"~/bin/custom-gpgsm")?,
            ]);
        let repo = crate::util::repo_opts("make_basic_repo.sh", options)?.to_thread_local();
        let options = repo.commit_signing_options()?;
        assert_eq!(options.format, gix::commit::sign::Format::X509);
        assert_eq!(options.program, home.join("bin/custom-gpgsm"));
        assert_eq!(options.signing_key, "Gitoxide Signing Fixture <signing@example.com>");
        assert!(options.program_arguments.is_empty());
        assert!(options.environment.is_empty());
        Ok(())
    }

    #[test]
    fn expands_verification_program_paths() -> crate::Result {
        let home = gix::path::env::home_dir().expect("the test environment has a home directory");
        let mut permissions = gix::open::Permissions::isolated();
        permissions.env.home = gix::sec::Permission::Allow;
        let options = gix::open::Options::isolated()
            .permissions(permissions)
            .config_overrides([gpg::OpenPgp::PROGRAM.validated_assignment_fmt(&"~/bin/missing-gpg")?]);
        let repo = crate::util::repo_opts("make_basic_repo.sh", options)?
            .to_thread_local()
            .with_object_memory();
        let mut commit = repo.head_commit()?.decode()?.into_owned()?;
        let signature_field = gix_object::commit::signature_field_name(commit.tree.kind());
        commit
            .extra_headers
            .push((signature_field.into(), "-----BEGIN PGP SIGNATURE-----\n".into()));
        let id = repo.write_object(&commit)?;
        let err = repo
            .find_commit(id)?
            .verify_signature()
            .expect_err("the configured verifier does not exist");
        let gix::commit::verify::Error::Verify(gix_object::signature::verify::Error::Spawn { program, .. }) = err
        else {
            panic!("expected a verifier spawn failure, got {err:?}");
        };
        assert_eq!(
            program,
            home.join("bin/missing-gpg"),
            "the configured verifier path is expanded relative to home"
        );
        Ok(())
    }

    #[test]
    fn resolves_signing_options_only_when_enabled() -> crate::Result {
        let disabled = gix::open_opts(
            gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")?,
            gix::open::Options::isolated().config_overrides(["gpg.format=invalid"]),
        )?;
        assert!(
            disabled.commit_signing_options_if_enabled()?.is_none(),
            "disabled signing does not resolve unrelated signer configuration"
        );

        let enabled = gix::open_opts(
            gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")?,
            gix::open::Options::isolated().config_overrides([
                "commit.gpgSign=true",
                "user.name=Gitoxide Signing Fixture",
                "user.email=signing@example.com",
            ]),
        )?;
        assert!(
            enabled.commit_signing_options_if_enabled()?.is_some(),
            "enabled signing resolves the same options as an explicit request"
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn resolves_the_default_ssh_key_command() -> crate::Result {
        let options = gix::open::Options::isolated().config_overrides([
            "user.name=Gitoxide Signing Fixture",
            "user.email=signing@example.com",
            "gpg.format=ssh",
            "gpg.ssh.defaultKeyCommand=printf 'key::ssh-ed25519 fixture-key\\n'",
        ]);
        let repo = crate::util::repo_opts("make_basic_repo.sh", options)?.to_thread_local();
        let options = repo.commit_signing_options()?;
        assert_eq!(options.signing_key, "key::ssh-ed25519 fixture-key");
        Ok(())
    }
}
