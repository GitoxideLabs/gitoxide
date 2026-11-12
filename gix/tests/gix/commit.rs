#[cfg(feature = "revision")]
mod describe {
    use gix::commit::describe::SelectRef::{AllRefs, AllTags, AnnotatedTags};
    use gix_testtools::TestResult;

    use crate::named_repo;

    #[test]
    fn nested_annotated_tags_match_git() -> TestResult {
        let fixture = gix_testtools::scripted_fixture_read_only("make_commit_describe_multiple_tags.sh")?;
        for case in ["nested-both", "nested-only"] {
            let root = fixture.join(case);
            let repo = gix::open_opts(&root, crate::restricted())?;
            let baseline = std::fs::read_to_string(root.join("describe.git"))?;
            for (select, expected) in [AnnotatedTags, AllTags].into_iter().zip(baseline.lines()) {
                assert_eq!(
                    repo.head_commit()?.describe().names(select).format()?.to_string(),
                    expected,
                    "{case}: {select:?} prefers the newest outer tag, as Git does"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn malformed_nested_tags_are_skipped() -> TestResult {
        let fixture = gix_testtools::scripted_fixture_read_only("make_commit_describe_multiple_tags.sh")?;
        let repo = gix::open_opts(fixture.join("nested-malformed"), crate::restricted())?;
        assert_eq!(
            repo.head_commit()?.describe().format()?.to_string(),
            "v2",
            "an invalid inner tag must not prevent describing with valid tags"
        );
        Ok(())
    }

    #[cfg(feature = "status")]
    mod with_dirty_suffix {
        use gix::commit::describe::SelectRef;
        use gix_testtools::TestResult;

        use crate::util::named_subrepo_opts;

        #[test]
        fn dirty_suffix_applies_automatically_if_dirty() -> TestResult {
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
        fn dirty_suffix_does_not_apply_if_not_dirty() -> TestResult {
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
    fn tags_are_sorted_by_date_and_lexicographically() -> TestResult {
        let repo = named_repo("make_commit_describe_multiple_tags.sh")?;
        let mut describe = repo.head_commit()?.describe();
        for filter in &[AnnotatedTags, AllTags, AllRefs] {
            describe = describe.names(*filter);
            assert_eq!(describe.format()?.to_string(), "v4", "{filter:?}");
        }
        Ok(())
    }

    #[test]
    fn tags_are_sorted_by_priority() -> TestResult {
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
    fn lightweight_tags_are_sorted_lexicographically() -> TestResult {
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
    use gix::config::tree::{Gpg, Key, User, gpg};
    use gix_testtools::TestResult;
    use gix_testtools::signature;
    use serial_test::serial;

    #[test]
    // Git inherits process-global date variables modified by fixture setup in other tests, which can otherwise fail with
    // `fatal: invalid date format: 42 +0030` while creating the reference signature.
    #[serial]
    #[cfg_attr(windows, ignore = "TODO: requires direct investigation on Windows")]
    fn verifies_a_commit_signed_by_git_with_ssh() -> TestResult {
        if !signature::program_available("ssh-keygen") {
            return Ok(());
        }
        let (_key_home, key) = signature::ssh_private_key()?;
        // Git writes the signed commit.
        let fixture = gix_testtools::scripted_fixture_writable("make_basic_repo.sh")?;
        let email = User::EMAIL.validated_assignment_fmt(&signature::IDENTITY)?;
        // Git for Windows passes this native path to `ssh-keygen`, where forward slashes avoid backslash
        // interpretation while retaining the drive prefix. An MSYS path like `/c/...` isn't accepted here.
        let key_for_git = gix_path::to_unix_separators_on_windows(gix_path::into_bstr(&key)?);
        let signing_key = User::SIGNING_KEY.validated_assignment(key_for_git.as_ref())?;
        let output = gix_testtools::git_command(fixture.path())
            .args(["-c", "user.name=Gitoxide Signing Fixture", "-c"])
            .arg(gix_path::from_bstring(email)?.into_os_string())
            .args(["-c", "gpg.format=ssh", "-c"])
            .arg(gix_path::from_bstring(signing_key)?.into_os_string())
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
    fn sign_write_and_verify_an_ssh_commit() -> TestResult {
        let (_key_home, key) = signature::ssh_private_key()?;
        let options = gix::open::Options::isolated().config_overrides([
            User::NAME.validated_assignment_fmt(&"Gitoxide Signing Fixture")?,
            User::EMAIL.validated_assignment_fmt(&signature::IDENTITY)?,
            Gpg::FORMAT.validated_assignment_fmt(&"ssh")?,
            User::SIGNING_KEY.validated_assignment_fmt(&key.display())?,
            gpg::Ssh::ALLOWED_SIGNERS_FILE
                .validated_assignment_fmt(&signature::fixture("ssh-allowed-signers").display())?,
        ]);
        let mut repo = crate::util::repo_opts("make_basic_repo.sh", options)?
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
        let commit_id = signed.id;
        drop(signed);

        repo.config_snapshot_mut()
            .append_config(["gpg.minTrustLevel=ultimate"], gix_config::Source::Api)?;
        for untrusted in [None, Some("undefined"), Some("invalid")] {
            if let Some(value) = untrusted {
                let mut config = repo.config_snapshot_mut();
                config.append(gix_config::File::from_bytes_no_includes(
                    format!(
                        "[gpg]
                            minTrustLevel = {value}
                        "
                    )
                    .as_bytes(),
                    gix_config::file::Metadata::from(gix_config::Source::Local).with(gix::sec::Trust::Reduced),
                    Default::default(),
                )?)?;
                config.commit()?;
            }
            let outcome = repo
                .find_commit(commit_id)?
                .verify_signature()?
                .expect("the written commit has a signature");
            assert_eq!(outcome.trust_level, gix::commit::verify::TrustLevel::Fully);
            assert!(
                !outcome.is_valid(),
                "untrusted {untrusted:?} must not lower the trusted minimum or cause a parse error"
            );
        }
        Ok(())
    }

    #[test]
    fn signing_policy_respects_the_section_filter() -> TestResult {
        let options = gix::open::Options::isolated()
            .filter_config_section(|meta| meta.source != gix_config::Source::Local)
            .config_overrides([
                "commit.gpgSign=true",
                "gpg.format=openpgp",
                "user.signingKey=trusted-key",
            ]);
        let mut repo = crate::util::repo_opts("make_basic_repo.sh", options)?.to_thread_local();
        for untrusted in ["ssh", "invalid"] {
            repo.config_snapshot_mut().append_config(
                [
                    format!("gpg.format={untrusted}"),
                    "user.signingKey=untrusted-key".into(),
                ],
                gix_config::Source::Local,
            )?;
            let options = repo.commit_signing_options()?;
            assert_eq!(
                options.format,
                gix::commit::sign::Format::OpenPgp,
                "the trusted format wins"
            );
            assert_eq!(options.signing_key, "trusted-key", "the trusted key wins");
        }
        for untrusted in ["false", "invalid"] {
            repo.config_snapshot_mut()
                .append_config([format!("commit.gpgSign={untrusted}")], gix_config::Source::Local)?;
            assert!(
                repo.commit_signing_options_if_enabled()?.is_some(),
                "untrusted values cannot disable trusted automatic signing"
            );
        }
        Ok(())
    }

    #[test]
    fn default_signing_key_uses_trusted_identity() -> TestResult {
        for (name, email) in [
            ("user.name", "user.email"),
            ("committer.name", "committer.email"),
            ("user.name", "gitoxide.user.emailFallback"),
            ("gitoxide.committer.nameFallback", "gitoxide.committer.emailFallback"),
        ] {
            let options = gix::open::Options::isolated()
                .filter_config_section(|meta| meta.source != gix_config::Source::Local)
                .config_overrides([format!("{name}=Trusted Signer"), format!("{email}=trusted@example.com")]);
            let mut repo = crate::util::repo_opts("make_basic_repo.sh", options)?.to_thread_local();
            repo.config_snapshot_mut().append_config(
                [
                    format!("{name}=Untrusted Signer"),
                    format!("{email}=untrusted@example.com"),
                ],
                gix_config::Source::Local,
            )?;
            assert_eq!(
                repo.commit_signing_options()?.signing_key,
                "Trusted Signer <trusted@example.com>",
                "untrusted {name} and {email} cannot select the default signing key"
            );
        }
        Ok(())
    }

    #[test]
    fn resolves_format_defaults_and_program_paths() -> TestResult {
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
    fn expands_verification_program_paths() -> TestResult {
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
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&(err), &[(&(home).to_string_lossy(), "<home>")]), "the configured verifier path is expanded relative to home", @r#"
        Could not verify the commit signature

        Caused by:
            0: Could not execute signature verifier, program="<home>/bin/missing-gpg"
            1: NotFound
        "#);
        Ok(())
    }

    #[test]
    fn resolves_signing_options_only_when_enabled() -> TestResult {
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
    fn resolves_the_default_ssh_key_command() -> TestResult {
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
