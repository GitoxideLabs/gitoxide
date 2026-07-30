mod keys {
    use gix::config::tree::{Key, Section};
    use gix_object::bstr::{BStr, ByteSlice};

    #[test]
    fn string() -> crate::Result {
        assert_eq!(gix::config::tree::Http::USER_AGENT.try_into_string("agent")?, "agent");
        assert!(gix::config::tree::Http::USER_AGENT.validate("agent".into()).is_ok());

        let invalid = b"\xF0\x80\x80".as_bstr();
        assert_eq!(
            gix::config::tree::Http::USER_AGENT
                .try_into_string(invalid)
                .unwrap_err()
                .to_string(),
            "The utf-8 string at \"http.userAgent=���\" could not be decoded"
        );
        assert!(gix::config::tree::Http::USER_AGENT.validate(invalid).is_err());

        Ok(())
    }

    #[test]
    fn any() {
        assert!(
            !gix::config::Tree.sections().is_empty(),
            "the root has at least one section"
        );
        assert_eq!(gix::config::Tree::AUTHOR.name(), "author");
        assert_eq!(gix::config::tree::Author.keys().len(), 2);
        assert_eq!(gix::config::tree::Author::NAME.name(), "name");
        assert_eq!(gix::config::tree::Author::EMAIL.name(), "email");
        assert_eq!(
            gix::config::tree::Author::NAME
                .validated_assignment("user".into())
                .unwrap(),
            "author.name=user"
        );
        assert_eq!(
            gix::config::tree::Author::NAME
                .validated_assignment("user".into())
                .unwrap(),
            "author.name=user"
        );
    }

    #[test]
    fn default_values() {
        for (key, expected) in [
            (
                &gix::config::tree::Clone::DEFAULT_REMOTE_NAME as &dyn Key,
                BStr::new(b"origin"),
            ),
            (&gix::config::tree::Core::NOTES_REF, BStr::new(b"refs/notes/commits")),
            (&gix::config::tree::Diff::ALGORITHM, BStr::new(b"myers")),
            (&gix::config::tree::Gpg::FORMAT, BStr::new(b"openpgp")),
            (&gix::config::tree::Gpg::PROGRAM, BStr::new(b"gpg")),
            (&gix::config::tree::gpg::OpenPgp::PROGRAM, BStr::new(b"gpg")),
            (&gix::config::tree::gpg::X509::PROGRAM, BStr::new(b"gpgsm")),
            (&gix::config::tree::gpg::Ssh::PROGRAM, BStr::new(b"ssh-keygen")),
            (&gix::config::tree::Commit::GPG_SIGN, BStr::new(b"false")),
            (&gix::config::tree::gitoxide::Core::SHALLOW_FILE, BStr::new(b"shallow")),
            (
                &gix::config::tree::gitoxide::Objects::REPLACE_REF_BASE,
                BStr::new(b"refs/replace/"),
            ),
        ] {
            assert_eq!(key.default_value(), Some(expected), "default for {key:?}");
            assert_eq!(key.default_value_or_panic(), expected, "default for {key:?}");
        }
        assert_eq!(gix::config::tree::Author::NAME.default_value(), None);
    }

    #[test]
    #[should_panic(expected = "BUG: default value must be set")]
    fn missing_default_panics() {
        let _ = gix::config::tree::Author::NAME.default_value_or_panic();
    }

    #[test]
    fn remote_name() {
        assert!(
            gix::config::tree::Remote::PUSH_DEFAULT
                .validate("origin".into())
                .is_ok()
        );
        assert!(
            gix::config::tree::Remote::PUSH_DEFAULT
                .validate("https://github.com/byron/gitoxide".into())
                .is_ok()
        );
    }

    #[test]
    fn unsigned_integer() {
        for valid in [0, 1, 100_124] {
            assert!(
                gix::config::tree::Core::DELTA_BASE_CACHE_LIMIT
                    .validate(valid.to_string().as_bytes().into())
                    .is_ok()
            );
        }

        for invalid in [-1, -100] {
            assert_eq!(
                gix::config::tree::Core::DELTA_BASE_CACHE_LIMIT
                    .validate(invalid.to_string().as_str().into())
                    .unwrap_err()
                    .to_string(),
                "cannot use sign for unsigned integer"
            );
        }

        let out_of_bounds = ((i64::MAX as u64) + 1).to_string();
        assert_eq!(
            gix::config::tree::Core::DELTA_BASE_CACHE_LIMIT
                .validate(out_of_bounds.as_bytes().into())
                .unwrap_err()
                .to_string(),
            "Could not decode '9223372036854775808': Integers needs to be positive or negative numbers which may have a suffix like 1k, 42, or 50G"
        );
    }
}

mod compression {
    use gix::config::tree::Key;

    #[test]
    fn validate_and_convert() {
        for key in [
            &gix::config::tree::Core::COMPRESSION,
            &gix::config::tree::Core::LOOSE_COMPRESSION,
            &gix::config::tree::Pack::COMPRESSION,
        ] {
            for valid in -1..=9 {
                assert!(key.validate(valid.to_string().as_str().into()).is_ok());
            }
            for invalid in [-2, 10, 100] {
                assert!(key.validate(invalid.to_string().as_str().into()).is_err());
            }
        }

        assert_eq!(
            gix::config::tree::Core::COMPRESSION
                .try_into_compression(Ok(Some(-1)))
                .expect("git maps -1 to the zlib default"),
            Some(gix::zlib::Compression::DEFAULT)
        );
        assert_eq!(
            gix::config::tree::Core::COMPRESSION
                .try_into_compression(Ok(Some(1)))
                .unwrap(),
            Some(gix::zlib::Compression::BEST_SPEED)
        );
        assert_eq!(
            gix::config::tree::Pack::COMPRESSION
                .try_into_compression(Ok(Some(9)))
                .unwrap(),
            Some(gix::zlib::Compression::BEST)
        );
        assert!(
            gix::config::tree::Pack::COMPRESSION
                .try_into_compression(Ok(Some(10)))
                .is_err()
        );
    }
}

mod branch {
    use gix::config::tree::{Branch, Key, branch};

    #[test]
    fn merge() {
        assert!(branch::Merge::try_into_fullrefname("refs/heads/main").is_ok());
        assert!(branch::Merge::try_into_fullrefname("main").is_err());
        assert!(
            Branch::MERGE.validate("refs/heads/main".into()).is_ok(),
            "a fully qualified merge reference is valid"
        );
        assert!(
            Branch::MERGE.validate("main".into()).is_err(),
            "a partial merge reference is invalid"
        );
        assert!(
            Branch::MERGE.validate("".into()).is_err(),
            "a merge reference cannot be empty"
        );

        assert!(Branch::MERGE.full_name(None).is_err());
        assert_eq!(
            Branch::MERGE.full_name(Some("name".into())).expect("valid"),
            "branch.name.merge"
        );
    }
}

mod ssh {

    #[test]
    #[cfg(feature = "blocking-network-client")]
    fn variant() -> crate::Result {
        use gix::config::tree::Ssh;
        use gix_protocol::transport::client::blocking_io::ssh::ProgramKind;

        for (actual, expected) in [
            ("auto", None),
            ("ssh", Some(ProgramKind::Ssh)),
            ("simple", Some(ProgramKind::Simple)),
            ("plink", Some(ProgramKind::Plink)),
            ("putty", Some(ProgramKind::Putty)),
            ("tortoiseplink", Some(ProgramKind::TortoisePlink)),
        ] {
            assert_eq!(Ssh::VARIANT.try_into_variant(actual)?, expected);
        }

        assert_eq!(
            Ssh::VARIANT.try_into_variant("SSH").unwrap_err().to_string(),
            "The key \"ssh.variant=SSH\" (possibly from GIT_SSH_VARIANT) was invalid",
            "case-sensitive comparisons"
        );
        Ok(())
    }
}

#[cfg(feature = "status")]
mod status {
    use gix::{config::tree::Status, status::UntrackedFiles};

    #[test]
    fn default() -> crate::Result {
        for (actual, expected) in [
            ("no", UntrackedFiles::None),
            ("normal", UntrackedFiles::Collapsed),
            ("all", UntrackedFiles::Files),
        ] {
            assert_eq!(
                Status::SHOW_UNTRACKED_FILES.try_into_show_untracked_files(actual)?,
                expected
            );
        }

        assert_eq!(
            Status::SHOW_UNTRACKED_FILES
                .try_into_show_untracked_files("NO")
                .unwrap_err()
                .to_string(),
            "The key \"status.showUntrackedFiles=NO\" was invalid",
            "case-sensitive comparisons"
        );
        Ok(())
    }
}

mod push {
    use gix::{config::tree::Push, push};

    #[test]
    fn default() -> crate::Result {
        for (actual, expected) in [
            ("nothing", push::Default::Nothing),
            ("current", push::Default::Current),
            ("upstream", push::Default::Upstream),
            ("tracking", push::Default::Upstream),
            ("simple", push::Default::Simple),
            ("matching", push::Default::Matching),
        ] {
            assert_eq!(Push::DEFAULT.try_into_default(actual)?, expected);
        }

        assert_eq!(
            Push::DEFAULT.try_into_default("Nothing").unwrap_err().to_string(),
            "The key \"push.default=Nothing\" was invalid",
            "case-sensitive comparisons"
        );
        Ok(())
    }
}

mod fetch {

    #[test]
    #[cfg(feature = "credentials")]
    fn algorithm() -> crate::Result {
        use gix::{
            config::tree::{Fetch, Key},
            remote::fetch::negotiate::Algorithm,
        };

        for (actual, expected) in [
            ("noop", Algorithm::Noop),
            ("consecutive", Algorithm::Consecutive),
            ("skipping", Algorithm::Skipping),
            ("default", Algorithm::Consecutive), // actually, default can be Skipping of `feature.experimental` is true, but we don't deal with that yet until we implement `skipping`
        ] {
            assert_eq!(
                Fetch::NEGOTIATION_ALGORITHM.try_into_negotiation_algorithm(actual)?,
                expected
            );
            assert!(Fetch::NEGOTIATION_ALGORITHM.validate(actual.into()).is_ok());
        }
        assert_eq!(
            Fetch::NEGOTIATION_ALGORITHM
                .try_into_negotiation_algorithm("foo")
                .unwrap_err()
                .to_string(),
            "The key \"fetch.negotiationAlgorithm=foo\" was invalid"
        );
        Ok(())
    }

    #[test]
    #[cfg(feature = "attributes")]
    fn recurse_submodule() -> crate::Result {
        use gix::{
            bstr::ByteSlice,
            config::tree::{Fetch, Key},
        };

        for (actual, expected) in [
            ("true", gix_submodule::config::FetchRecurse::Always),
            ("false", gix_submodule::config::FetchRecurse::Never),
            ("on-demand", gix_submodule::config::FetchRecurse::OnDemand),
        ] {
            assert_eq!(
                Fetch::RECURSE_SUBMODULES.try_into_recurse_submodules(
                    gix_config::Boolean::try_from(actual.as_bytes().as_bstr()).map(|b| Some(b.0))
                )?,
                Some(expected)
            );
            assert!(Fetch::RECURSE_SUBMODULES.validate(actual.into()).is_ok());
        }
        assert_eq!(
            Fetch::RECURSE_SUBMODULES
                .try_into_recurse_submodules(gix_config::Boolean::try_from(b"foo".as_bstr()).map(|b| Some(b.0)))
                .unwrap_err()
                .to_string(),
            "The key \"fetch.recurseSubmodules=foo\" was invalid"
        );
        Ok(())
    }
}

#[cfg(feature = "blob-diff")]
mod diff {
    use gix::{
        config::tree::{Diff, Key},
        diff::rename::Tracking,
    };
    use gix_diff::blob::Algorithm;

    #[test]
    fn renames() -> crate::Result {
        assert_eq!(Diff::RENAMES.try_into_renames(Ok(Some(true)))?, Some(Tracking::Renames));
        assert!(Diff::RENAMES.validate("1".into()).is_ok());
        assert_eq!(
            Diff::RENAMES.try_into_renames(Ok(Some(false)))?,
            Some(Tracking::Disabled)
        );
        assert!(Diff::RENAMES.validate("0".into()).is_ok());
        assert_eq!(
            Diff::RENAMES.try_into_renames(Err(gix_config::value::Error::new("err", "copy")))?,
            Some(Tracking::RenamesAndCopies)
        );
        assert!(Diff::RENAMES.validate("copy".into()).is_ok());
        assert_eq!(
            Diff::RENAMES.try_into_renames(Err(gix_config::value::Error::new("err", "copies")))?,
            Some(Tracking::RenamesAndCopies)
        );
        assert!(Diff::RENAMES.validate("copies".into()).is_ok());

        assert_eq!(
            Diff::RENAMES
                .try_into_renames(Err(gix_config::value::Error::new("err", "foo")))
                .unwrap_err()
                .to_string(),
            "The value of key \"diff.renames=foo\" was invalid"
        );
        Ok(())
    }

    #[test]
    fn driver_binary() -> crate::Result {
        assert_eq!(
            Diff::DRIVER_BINARY.try_into_binary(Some("auto"))?,
            None,
            "this is as good as not setting it, but it's a valid value that would fail if it was just a boolean. It's undocumented though…"
        );
        assert!(Diff::DRIVER_BINARY.validate("auto".into()).is_ok());

        for (actual, expected) in [
            (Some("true"), Some(true)),
            (Some("false"), Some(false)),
            (None, Some(true)),
        ] {
            assert_eq!(Diff::DRIVER_BINARY.try_into_binary(actual)?, expected);
            if let Some(value) = actual {
                assert!(Diff::DRIVER_BINARY.validate(value.into()).is_ok());
            }
        }

        assert_eq!(
            Diff::DRIVER_BINARY
                .try_into_binary(Some("something"))
                .unwrap_err()
                .to_string(),
            "The key \"diff.<driver>.binary=something\" was invalid",
        );
        assert!(Diff::DRIVER_BINARY.validate("foo".into()).is_err());
        Ok(())
    }

    #[test]
    fn algorithm() -> crate::Result {
        for (actual, expected) in [
            ("myers", Algorithm::Myers),
            ("Myers", Algorithm::Myers),
            ("default", Algorithm::Myers),
            ("Default", Algorithm::Myers),
            ("minimal", Algorithm::MyersMinimal),
            ("histogram", Algorithm::Histogram),
        ] {
            assert_eq!(Diff::ALGORITHM.try_into_algorithm(actual)?, expected);
            assert!(Diff::ALGORITHM.validate(actual.into()).is_ok());
        }
        assert_eq!(
            Diff::ALGORITHM.try_into_algorithm("patience").unwrap_err().to_string(),
            "The 'patience' algorithm is not yet implemented"
        );
        assert_eq!(
            Diff::ALGORITHM.try_into_algorithm("foo").unwrap_err().to_string(),
            "Unknown diff algorithm named 'foo'"
        );
        Ok(())
    }
}

#[cfg(feature = "merge")]
mod merge {
    use gix::config::tree::{Key, Merge};
    use gix_merge::blob::builtin_driver::text::ConflictStyle;

    #[test]
    fn conflict_style() -> crate::Result {
        for (actual, expected) in [
            ("merge", ConflictStyle::Merge),
            ("diff3", ConflictStyle::Diff3),
            ("zdiff3", ConflictStyle::ZealousDiff3),
        ] {
            assert_eq!(Merge::CONFLICT_STYLE.try_into_conflict_style(actual)?, expected);
            assert!(Merge::CONFLICT_STYLE.validate(actual.into()).is_ok());
        }
        assert_eq!(
            Merge::CONFLICT_STYLE
                .try_into_conflict_style("foo")
                .unwrap_err()
                .to_string(),
            "The key \"merge.conflictStyle=foo\" was invalid"
        );
        Ok(())
    }
}

mod core {
    use std::time::Duration;

    use gix::config::tree::{Core, Key};
    use gix_lock::acquire::Fail;

    fn signed(value: i64) -> Result<Option<i64>, gix_config::value::Error> {
        Ok(Some(value))
    }

    #[test]
    fn notes_ref_is_a_full_reference_or_empty() {
        assert!(
            Core::NOTES_REF.validate("refs/notes/review".into()).is_ok(),
            "a fully qualified notes reference is valid"
        );
        assert!(
            Core::NOTES_REF.validate("review".into()).is_err(),
            "a partial notes reference is invalid"
        );
        assert!(
            Core::NOTES_REF.validate("".into()).is_ok(),
            "an empty value disables the default notes reference"
        );
    }

    #[test]
    fn timeouts() -> crate::Result {
        assert_eq!(
            Core::FILES_REF_LOCK_TIMEOUT.try_into_lock_timeout(Ok(Some(0)))?,
            Some(Fail::Immediately)
        );
        assert!(Core::FILES_REF_LOCK_TIMEOUT.validate("0".into()).is_ok());
        assert_eq!(
            Core::FILES_REF_LOCK_TIMEOUT.try_into_lock_timeout(Ok(Some(-5)))?,
            Some(Fail::AfterDurationWithBackoff(Duration::from_secs(u64::MAX)))
        );
        assert!(Core::FILES_REF_LOCK_TIMEOUT.validate("-1".into()).is_ok());

        assert_eq!(
            Core::FILES_REF_LOCK_TIMEOUT.try_into_lock_timeout(Ok(Some(2500)))?,
            Some(Fail::AfterDurationWithBackoff(Duration::from_millis(2500)))
        );
        assert!(Core::FILES_REF_LOCK_TIMEOUT.validate("2500".into()).is_ok());
        assert_eq!(
            Core::FILES_REF_LOCK_TIMEOUT
                .try_into_lock_timeout(Err(gix_config::value::Error::new("err", "bogus")))
                .unwrap_err()
                .to_string(),
            "The timeout at key \"core.filesRefLockTimeout\" was invalid"
        );
        Ok(())
    }

    #[test]
    #[cfg(feature = "revision")]
    fn disambiguate() -> crate::Result {
        use gix::revision::spec::parse::ObjectKindHint;
        for (value, expected) in [
            ("none", None),
            ("commit", Some(ObjectKindHint::Commit)),
            ("committish", Some(ObjectKindHint::Committish)),
            ("tree", Some(ObjectKindHint::Tree)),
            ("treeish", Some(ObjectKindHint::Treeish)),
            ("blob", Some(ObjectKindHint::Blob)),
        ] {
            assert_eq!(Core::DISAMBIGUATE.try_into_object_kind_hint(value).unwrap(), expected);
            assert!(Core::DISAMBIGUATE.validate(value.into()).is_ok());
        }
        assert_eq!(
            Core::DISAMBIGUATE
                .try_into_object_kind_hint("CommiT")
                .unwrap_err()
                .to_string(),
            "The key \"core.disambiguate=CommiT\" was invalid"
        );
        Ok(())
    }

    #[test]
    fn log_all_ref_updates() -> crate::Result {
        assert_eq!(
            Core::LOG_ALL_REF_UPDATES.try_into_ref_updates(Ok(Some(true)))?,
            Some(gix_ref::store::WriteReflog::Normal)
        );
        assert!(Core::LOG_ALL_REF_UPDATES.validate("true".into()).is_ok());
        assert_eq!(
            Core::LOG_ALL_REF_UPDATES.try_into_ref_updates(Ok(Some(false)))?,
            Some(gix_ref::store::WriteReflog::Disable)
        );
        assert!(Core::LOG_ALL_REF_UPDATES.validate("0".into()).is_ok());
        let boolean = |value| gix_config::Boolean::try_from(value).map(|b| Some(b.0));
        assert_eq!(
            Core::LOG_ALL_REF_UPDATES.try_into_ref_updates(boolean("always"))?,
            Some(gix_ref::store::WriteReflog::Always)
        );
        assert!(Core::LOG_ALL_REF_UPDATES.validate("always".into()).is_ok());
        assert_eq!(
            Core::LOG_ALL_REF_UPDATES
                .try_into_ref_updates(boolean("invalid"))
                .unwrap_err()
                .to_string(),
            "The key \"core.logAllRefUpdates=invalid\" was invalid"
        );
        assert!(Core::LOG_ALL_REF_UPDATES.validate("invalid".into()).is_err());
        Ok(())
    }

    #[test]
    fn abbrev() -> crate::Result {
        let object_hash = gix_hash::Kind::Sha1;
        assert_eq!(Core::ABBREV.try_into_abbreviation("4", object_hash)?, Some(4));
        assert_eq!(Core::ABBREV.try_into_abbreviation("auto", object_hash)?, None);
        assert_eq!(
            Core::ABBREV.try_into_abbreviation("AUto", object_hash)?,
            None,
            "case-insensitive"
        );
        assert_eq!(
            Core::ABBREV.try_into_abbreviation("false", object_hash)?,
            Some(object_hash.len_in_hex()),
            "turns abbreviations off entirely"
        );

        assert_eq!(
            Core::ABBREV
                .try_into_abbreviation("   ", object_hash)
                .unwrap_err()
                .to_string(),
            "Invalid value for 'core.abbrev' = '   '. It must be between 4 and 40"
        );
        for invalid in ["foo", "3", "41"] {
            assert!(Core::ABBREV.try_into_abbreviation(invalid, object_hash).is_err());
        }
        Ok(())
    }

    #[test]
    fn delta_base_cache_limit() -> crate::Result {
        assert_eq!(Core::DELTA_BASE_CACHE_LIMIT.try_into_usize(signed(1))?, Some(1));
        assert_eq!(Core::DELTA_BASE_CACHE_LIMIT.try_into_usize(signed(0))?, Some(0));
        assert!(Core::DELTA_BASE_CACHE_LIMIT.validate("0".into()).is_ok());
        assert!(Core::DELTA_BASE_CACHE_LIMIT.validate("1".into()).is_ok());
        assert_eq!(
            Core::DELTA_BASE_CACHE_LIMIT
                .try_into_usize(signed(-1))
                .unwrap_err()
                .to_string(),
            "The value of key \"core.deltaBaseCacheLimit\" (possibly from GIX_PACK_CACHE_MEMORY) could not be parsed as unsigned integer"
        );
        assert!(Core::DELTA_BASE_CACHE_LIMIT.validate("-1".into()).is_err());
        Ok(())
    }

    #[test]
    fn check_stat() -> crate::Result {
        assert!(Core::CHECK_STAT.try_into_checkstat("default")?);
        assert!(!Core::CHECK_STAT.try_into_checkstat("minimal")?);
        assert_eq!(
            Core::CHECK_STAT.try_into_checkstat("normal").unwrap_err().to_string(),
            "The key \"core.checkStat=normal\" was invalid"
        );

        assert!(Core::CHECK_STAT.validate("default".into()).is_ok());
        assert!(Core::CHECK_STAT.validate("minimal".into()).is_ok());
        assert!(Core::CHECK_STAT.validate("foo".into()).is_err());
        Ok(())
    }

    #[test]
    #[cfg(feature = "attributes")]
    fn safecrlf() -> crate::Result {
        for (value, expected) in [
            ("false", gix_filter::pipeline::CrlfRoundTripCheck::Skip),
            ("true", gix_filter::pipeline::CrlfRoundTripCheck::Fail),
            ("warn", gix_filter::pipeline::CrlfRoundTripCheck::Warn),
        ] {
            assert_eq!(Core::SAFE_CRLF.try_into_safecrlf(value).unwrap(), expected);
            assert!(Core::SAFE_CRLF.validate(value.into()).is_ok());
        }
        assert_eq!(
            Core::SAFE_CRLF.try_into_safecrlf("WARN").unwrap_err().to_string(),
            "The key \"core.safecrlf=WARN\" was invalid"
        );
        Ok(())
    }

    #[test]
    #[cfg(feature = "attributes")]
    fn autocrlf() -> crate::Result {
        for (value, expected) in [
            ("false", gix_filter::eol::AutoCrlf::Disabled),
            ("true", gix_filter::eol::AutoCrlf::Enabled),
            ("input", gix_filter::eol::AutoCrlf::Input),
        ] {
            assert_eq!(Core::AUTO_CRLF.try_into_autocrlf(value).unwrap(), expected);
            assert!(Core::AUTO_CRLF.validate(value.into()).is_ok());
        }
        assert_eq!(
            Core::AUTO_CRLF.try_into_autocrlf("Input").unwrap_err().to_string(),
            "The key \"core.autocrlf=Input\" was invalid"
        );
        Ok(())
    }

    #[test]
    #[cfg(feature = "attributes")]
    fn eol() -> crate::Result {
        for (value, expected) in [
            ("lf", gix_filter::eol::Mode::Lf),
            ("crlf", gix_filter::eol::Mode::CrLf),
            ("native", gix_filter::eol::Mode::default()),
        ] {
            assert_eq!(Core::EOL.try_into_eol(value).unwrap(), expected);
            assert!(Core::EOL.validate(value.into()).is_ok());
        }
        assert_eq!(
            Core::EOL.try_into_eol("LF").unwrap_err().to_string(),
            "The key \"core.eol=LF\" was invalid"
        );
        Ok(())
    }

    #[test]
    #[cfg(feature = "attributes")]
    fn check_round_trip_encoding() -> crate::Result {
        for (value, expected) in [
            (
                Some("UTF-8 utf-16BE"),
                &[gix_filter::encoding::UTF_8, gix_filter::encoding::UTF_16BE][..],
            ),
            (
                Some("SHIFT-JIS,UTF-8"),
                &[gix_filter::encoding::SHIFT_JIS, gix_filter::encoding::UTF_8],
            ),
            (
                Some("UTF-16LE, SHIFT-JIS"),
                &[gix_filter::encoding::UTF_16LE, gix_filter::encoding::SHIFT_JIS],
            ),
            (None, &[gix_filter::encoding::SHIFT_JIS]),
        ] {
            assert_eq!(
                Core::CHECK_ROUND_TRIP_ENCODING.try_into_encodings(value).unwrap(),
                expected
            );
            if let Some(value) = value {
                assert!(Core::CHECK_ROUND_TRIP_ENCODING.validate(value.into()).is_ok());
            }
        }
        assert_eq!(
            Core::CHECK_ROUND_TRIP_ENCODING
                .try_into_encodings(Some("SOMETHING ELSE"))
                .unwrap_err()
                .to_string(),
            "The encoding named 'SOMETHING' seen in key 'core.checkRoundTripEncoding=SOMETHING ELSE' is unsupported"
        );
        Ok(())
    }
}

mod index {
    use gix::config::tree::{Index, Key};

    #[test]
    fn threads() {
        for (value, expected) in [("false", 1), ("true", 0), ("0", 0), ("1", 1), ("2", 2), ("12", 12)] {
            assert_eq!(
                Index::THREADS.try_into_index_threads(value).unwrap(),
                expected,
                "{value}"
            );
            assert!(Index::THREADS.validate(value.into()).is_ok());
        }
        assert_eq!(
            Index::THREADS
                .try_into_index_threads("nothing")
                .unwrap_err()
                .to_string(),
            "The key \"index.threads=nothing\" was invalid"
        );
    }
}

mod extensions {
    use gix::config::tree::{Extensions, Key};

    #[test]
    fn object_format() -> crate::Result {
        #[cfg(feature = "sha1")]
        {
            assert_eq!(
                Extensions::OBJECT_FORMAT.try_into_object_format("sha1")?,
                gix_hash::Kind::Sha1
            );
            assert_eq!(
                Extensions::OBJECT_FORMAT.try_into_object_format("SHA1")?,
                gix_hash::Kind::Sha1,
                "case-insensitive"
            );
            assert!(Extensions::OBJECT_FORMAT.validate("sha1".into()).is_ok());
        }
        #[cfg(feature = "sha256")]
        {
            assert_eq!(
                Extensions::OBJECT_FORMAT.try_into_object_format("sha256")?,
                gix_hash::Kind::Sha256
            );
            assert_eq!(
                Extensions::OBJECT_FORMAT.try_into_object_format("SHA256")?,
                gix_hash::Kind::Sha256,
                "case-insensitive"
            );
            assert!(Extensions::OBJECT_FORMAT.validate("sha256".into()).is_ok());
        }
        assert_eq!(
            Extensions::OBJECT_FORMAT
                .try_into_object_format("invalid")
                .unwrap_err()
                .to_string(),
            "The key \"extensions.objectFormat=invalid\" was invalid"
        );
        assert!(Extensions::OBJECT_FORMAT.validate("invalid".into()).is_err());
        Ok(())
    }
}

mod checkout {
    use gix::config::tree::{Checkout, Key};

    fn int(value: i64) -> Result<Option<i64>, gix_config::value::Error> {
        Ok(Some(value))
    }

    #[test]
    fn workers() -> crate::Result {
        assert!(Checkout::WORKERS.validate("0".into()).is_ok());
        assert_eq!(Checkout::WORKERS.try_from_workers(int(0))?, Some(0));
        assert!(Checkout::WORKERS.validate("-1".into()).is_ok());
        assert_eq!(Checkout::WORKERS.try_from_workers(int(-1))?, Some(0));
        assert!(Checkout::WORKERS.validate("-2".into()).is_ok());
        assert!(Checkout::WORKERS.validate("3".into()).is_ok());
        assert_eq!(Checkout::WORKERS.try_from_workers(int(2))?, Some(2));
        Ok(())
    }
}

mod pack {
    use gix::config::tree::{Key, Pack};

    #[test]
    fn index_version() -> crate::Result {
        assert_eq!(
            Pack::INDEX_VERSION.try_into_index_version(Ok(Some(1)))?,
            Some(gix_pack::index::Version::V1)
        );
        assert!(Pack::INDEX_VERSION.validate("1".into()).is_ok());
        assert_eq!(
            Pack::INDEX_VERSION.try_into_index_version(Ok(Some(2)))?,
            Some(gix_pack::index::Version::V2)
        );
        assert!(Pack::INDEX_VERSION.validate("2".into()).is_ok());
        assert_eq!(
            Pack::INDEX_VERSION.try_into_index_version(Ok(None))?,
            None,
            "an unset key remains distinguishable from an explicitly configured version"
        );
        assert_eq!(
            Pack::INDEX_VERSION
                .try_into_index_version(Ok(Some(3)))
                .unwrap_err()
                .to_string(),
            "The value of key \"pack.indexVersion\" was invalid"
        );
        assert!(Pack::INDEX_VERSION.validate("3".into()).is_err());
        assert!(Pack::INDEX_VERSION.validate("-1".into()).is_err());
        Ok(())
    }
}

mod protocol {
    use gix::config::tree::{Key, Protocol};

    #[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
    #[test]
    fn allow() -> crate::Result {
        use gix::{config::tree::protocol, remote::url::scheme_permission::Allow};

        for (key, protocol_name_parameter) in [
            (&Protocol::ALLOW, None),
            (&protocol::NameParameter::ALLOW, Some("http")),
        ] {
            for (input, expected) in [
                ("always", Allow::Always),
                ("never", Allow::Never),
                ("user", Allow::User),
            ] {
                assert_eq!(key.try_into_allow(input, protocol_name_parameter)?, expected);
                assert!(key.validate(input.into()).is_ok());
            }
            assert_eq!(
                key.try_into_allow("User", protocol_name_parameter)
                    .unwrap_err()
                    .to_string(),
                format!(
                    "The value \"User\" must be allow|deny|user in configuration key {}",
                    protocol_name_parameter
                        .map_or_else(|| "protocol.allow".into(), |key| format!("protocol.{key}.allow"))
                )
            );
        }
        Ok(())
    }

    #[test]
    fn version() {
        for valid in [0, 1, 2] {
            assert!(Protocol::VERSION.validate(valid.to_string().as_str().into()).is_ok());
        }

        assert_eq!(
            Protocol::VERSION.validate("5".into()).unwrap_err().to_string(),
            "protocol version 5 is unknown"
        );

        #[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
        {
            for (valid, expected) in [
                (None, gix_protocol::transport::Protocol::V2),
                (Some(0), gix_protocol::transport::Protocol::V0),
                (Some(1), gix_protocol::transport::Protocol::V1),
                (Some(2), gix_protocol::transport::Protocol::V2),
            ] {
                assert_eq!(
                    Protocol::VERSION
                        .try_into_protocol_version(Ok(valid))
                        .expect("valid version"),
                    expected
                );
            }

            assert_eq!(
                Protocol::VERSION
                    .try_into_protocol_version(Ok(Some(5)))
                    .unwrap_err()
                    .to_string(),
                "The key \"protocol.version=5\" was invalid"
            );
        }
    }
}

mod gpg {
    use gix::{
        bstr::BStr,
        config::tree::{Gpg, Key, Section, gpg},
    };

    #[test]
    fn keys_and_subsections_are_registered() {
        for (actual, expected) in [
            (Gpg::FORMAT.logical_name(), "gpg.format"),
            (Gpg::PROGRAM.logical_name(), "gpg.program"),
            (Gpg::MIN_TRUST_LEVEL.logical_name(), "gpg.minTrustLevel"),
            (gpg::OpenPgp::PROGRAM.logical_name(), "gpg.openpgp.program"),
            (gpg::X509::PROGRAM.logical_name(), "gpg.x509.program"),
            (gpg::Ssh::PROGRAM.logical_name(), "gpg.ssh.program"),
            (
                gpg::Ssh::DEFAULT_KEY_COMMAND.logical_name(),
                "gpg.ssh.defaultKeyCommand",
            ),
            (
                gpg::Ssh::ALLOWED_SIGNERS_FILE.logical_name(),
                "gpg.ssh.allowedSignersFile",
            ),
            (gpg::Ssh::REVOCATION_FILE.logical_name(), "gpg.ssh.revocationFile"),
        ] {
            assert_eq!(actual, expected);
        }
        assert_eq!(
            Gpg.sub_sections()
                .iter()
                .map(|section| section.name())
                .collect::<Vec<_>>(),
            ["openpgp", "x509", "ssh"]
        );
        for (key, expected) in [
            (&Gpg::PROGRAM as &dyn Key, BStr::new(b"gpg")),
            (&gpg::OpenPgp::PROGRAM, BStr::new(b"gpg")),
            (&gpg::X509::PROGRAM, BStr::new(b"gpgsm")),
            (&gpg::Ssh::PROGRAM, BStr::new(b"ssh-keygen")),
        ] {
            assert_eq!(key.default_value(), Some(expected), "default for {key:?}");
        }
        #[cfg(feature = "command")]
        for valid in ["undefined", "NEVER", "Marginal", "fully", " ultimate "] {
            assert!(
                Gpg::MIN_TRUST_LEVEL.validate(valid.into()).is_ok(),
                "Git accepts {valid:?} as a minimum trust level"
            );
        }
        assert!(Gpg::MIN_TRUST_LEVEL.validate("unknown".into()).is_err());
    }
}

mod notes {
    use gix::config::tree::{Key, Notes};
    use gix_object::bstr::BString;

    #[test]
    fn display_ref_metadata() {
        assert_eq!(Notes::DISPLAY_REF.logical_name(), "notes.displayRef");
        assert_eq!(
            Notes::DISPLAY_REF.the_environment_override(),
            "GIT_NOTES_DISPLAY_REF",
            "the key declares its corresponding environment variable"
        );
    }

    #[test]
    fn display_ref_parsing() -> crate::Result {
        assert_eq!(
            Notes::DISPLAY_REF.try_into_display_refs(":refs/notes/review::refs/notes/*:")?,
            vec![BString::from("refs/notes/review"), BString::from("refs/notes/*")],
            "empty fields are ignored while literal and glob references retain their order"
        );
        Ok(())
    }

    #[test]
    fn display_ref_validation() {
        for valid in [
            "",
            "refs/notes/review",
            "refs/notes/*",
            "refs/notes/revie?",
            "refs/notes/[rs]eview",
            "refs/notes/review:refs/notes/*",
        ] {
            assert!(
                Notes::DISPLAY_REF.validate(valid.into()).is_ok(),
                "{valid:?} is a valid display-reference list"
            );
        }
        for invalid in ["review", "refs/notes/review:security", r"refs/notes/review\literal"] {
            assert!(
                Notes::DISPLAY_REF.validate(invalid.into()).is_err(),
                "{invalid:?} contains a reference that is neither fully qualified nor a glob"
            );
        }
    }
}

mod gitoxide {
    mod http {
        use std::time::Duration;

        use gix::config::tree::{Key, gitoxide};

        #[test]
        fn connect_timeout() -> crate::Result {
            assert_eq!(
                gitoxide::Http::CONNECT_TIMEOUT.validated_assignment_fmt(&Duration::from_secs(1).as_millis())?,
                "gitoxide.http.connectTimeout=1000"
            );
            Ok(())
        }
    }
    mod allow {
        use gix::config::tree::{Key, gitoxide};

        #[test]
        fn protocol_from_user() {
            for value in ["1", "true", "yes", "0", "false", "no"] {
                assert!(
                    gitoxide::Allow::PROTOCOL_FROM_USER.validate(value.into()).is_ok(),
                    "Git accepts {value:?} as a boolean"
                );
            }
            assert!(gitoxide::Allow::PROTOCOL_FROM_USER.validate("invalid".into()).is_err());
        }
    }
    mod commit {
        use gix::config::tree::{Key, gitoxide};

        #[test]
        fn author_and_committer_date() {
            assert_eq!(
                gitoxide::Commit::AUTHOR_DATE
                    .validated_assignment("Thu, 1 Aug 2022 12:45:06 +0800".into())
                    .expect("valid"),
                "gitoxide.commit.authorDate=Thu, 1 Aug 2022 12:45:06 +0800"
            );
            assert_eq!(
                gitoxide::Commit::COMMITTER_DATE
                    .validated_assignment("Thu, 1 Aug 2022 12:45:06 +0800".into())
                    .expect("valid"),
                "gitoxide.commit.committerDate=Thu, 1 Aug 2022 12:45:06 +0800"
            );
        }
    }
    mod author {
        use gix::config::tree::{Key, gitoxide};

        #[test]
        fn name_and_email_fallback() {
            assert_eq!(
                gitoxide::Author::NAME_FALLBACK
                    .validated_assignment("name".into())
                    .expect("valid"),
                "gitoxide.author.nameFallback=name"
            );
            assert_eq!(
                gitoxide::Author::EMAIL_FALLBACK
                    .validated_assignment("email".into())
                    .expect("valid"),
                "gitoxide.author.emailFallback=email"
            );
        }
    }
    mod committer {
        use gix::config::tree::{Key, gitoxide};

        #[test]
        fn name_and_email_fallback() {
            assert_eq!(
                gitoxide::Committer::NAME_FALLBACK
                    .validated_assignment("name".into())
                    .expect("valid"),
                "gitoxide.committer.nameFallback=name"
            );
            assert_eq!(
                gitoxide::Committer::EMAIL_FALLBACK
                    .validated_assignment("email".into())
                    .expect("valid"),
                "gitoxide.committer.emailFallback=email"
            );
        }
    }
    mod objects {
        use std::time::Duration;

        use gix::config::tree::{Key, gitoxide};

        #[test]
        fn alloc_limit() -> crate::Result {
            assert_eq!(
                gitoxide::Objects::ALLOC_LIMIT.validated_assignment("16m".into())?,
                "gitoxide.objects.allocLimit=16m"
            );
            Ok(())
        }

        #[test]
        fn alloc_limit_if_reduced_trust() -> crate::Result {
            assert_eq!(
                gitoxide::Objects::ALLOC_LIMIT_IF_REDUCED_TRUST.validated_assignment("16m".into())?,
                "gitoxide.objects.allocLimitIfReducedTrust=16m"
            );
            Ok(())
        }

        #[test]
        fn refresh_after() -> crate::Result {
            assert_eq!(
                gitoxide::Objects::REFRESH_AFTER.validated_assignment_fmt(&Duration::from_secs(1).as_millis())?,
                "gitoxide.objects.refreshAfter=1000"
            );
            Ok(())
        }
    }
}

#[cfg(any(
    feature = "blocking-http-transport-reqwest",
    feature = "blocking-http-transport-curl"
))]
mod http {
    use gix::config::tree::{Http, Key};
    use gix_object::bstr::ByteSlice;

    #[test]
    fn follow_redirects() -> crate::Result {
        use gix_transport::client::blocking_io::http::options::FollowRedirects;
        assert_eq!(
            Http::FOLLOW_REDIRECTS.try_into_follow_redirects("initial", || unreachable!("no call"))?,
            FollowRedirects::Initial
        );
        for (actual, cb_val, expected) in [
            ("true", Ok(Some(true)), FollowRedirects::All),
            ("false", Ok(Some(false)), FollowRedirects::None),
            // even though this is uncommon, with leniency it's possible to force it to internally default
            ("true", Ok(None), FollowRedirects::Initial),
        ] {
            assert_eq!(
                Http::FOLLOW_REDIRECTS.try_into_follow_redirects(actual, || cb_val)?,
                expected
            );
            assert!(Http::FOLLOW_REDIRECTS.validate(actual.into()).is_ok());
        }

        assert_eq!(
            Http::FOLLOW_REDIRECTS
                .try_into_follow_redirects("something", || Err(gix_config::value::Error::new("invalid", "value")))
                .unwrap_err()
                .to_string(),
            "The key \"http.followRedirects=something\" was invalid",
        );
        assert!(Http::FOLLOW_REDIRECTS.validate("foo".into()).is_err());
        Ok(())
    }

    #[test]
    fn extra_header() -> crate::Result {
        assert_eq!(Http::EXTRA_HEADER.try_into_extra_header(vec!["a", "b"])?, ["a", "b"]);
        assert_eq!(
            Http::EXTRA_HEADER.try_into_extra_header(vec!["a", "b", "", "c", "d"])?,
            ["c", "d"]
        );

        assert!(Http::EXTRA_HEADER.validate("a".into()).is_ok());

        let invalid = b"\xF0\x80\x80";
        assert!(Http::EXTRA_HEADER.validate(invalid.as_bstr()).is_err());
        assert_eq!(
            Http::EXTRA_HEADER
                .try_into_extra_header(vec![invalid.as_bstr()])
                .unwrap_err()
                .to_string(),
            "The utf-8 string at \"http.extraHeader=���\" could not be decoded"
        );
        Ok(())
    }

    #[test]
    fn http_version() -> crate::Result {
        use gix_transport::client::blocking_io::http::options::HttpVersion;

        for (actual, expected) in [("HTTP/1.1", HttpVersion::V1_1), ("HTTP/2", HttpVersion::V2)] {
            assert_eq!(Http::VERSION.try_into_http_version(actual)?, expected);
            assert!(Http::VERSION.validate(actual.into()).is_ok());
        }

        assert_eq!(
            Http::VERSION.try_into_http_version("invalid").unwrap_err().to_string(),
            "The key \"http.version=invalid\" was invalid"
        );
        assert!(Http::VERSION.validate("invalid".into()).is_err());
        Ok(())
    }

    #[test]
    fn ssl_version() -> crate::Result {
        use gix_transport::client::blocking_io::http::options::SslVersion::*;

        for (actual, expected) in [
            ("default", Default),
            ("", Default),
            ("tlsv1", TlsV1),
            ("sslv2", SslV2),
            ("sslv3", SslV3),
            ("tlsv1.0", TlsV1_0),
            ("tlsv1.1", TlsV1_1),
            ("tlsv1.2", TlsV1_2),
            ("tlsv1.3", TlsV1_3),
        ] {
            assert_eq!(Http::SSL_VERSION.try_into_ssl_version(actual)?, expected);
            assert!(Http::SSL_VERSION.validate(actual.into()).is_ok());
        }

        assert_eq!(
            Http::SSL_VERSION
                .try_into_ssl_version("invalid")
                .unwrap_err()
                .to_string(),
            "The ssl version at \"http.sslVersion=invalid\" (possibly from GIT_SSL_VERSION) was invalid"
        );
        assert!(Http::SSL_VERSION.validate("invalid".into()).is_err());
        Ok(())
    }

    #[test]
    fn proxy_auth_method() -> crate::Result {
        use gix_transport::client::blocking_io::http::options::ProxyAuthMethod::*;
        for (actual, expected) in [
            ("anyauth", AnyAuth),
            ("basic", Basic),
            ("digest", Digest),
            ("negotiate", Negotiate),
            ("ntlm", Ntlm),
        ] {
            assert_eq!(Http::PROXY_AUTH_METHOD.try_into_proxy_auth_method(actual)?, expected);
            assert!(Http::PROXY_AUTH_METHOD.validate(actual.into()).is_ok());
        }

        assert_eq!(
            Http::PROXY_AUTH_METHOD
                .try_into_proxy_auth_method("invalid")
                .unwrap_err()
                .to_string(),
            "The key \"http.proxyAuthMethod=invalid\" was invalid"
        );
        assert!(Http::PROXY_AUTH_METHOD.validate("invalid".into()).is_err());
        Ok(())
    }
}

mod remote {
    use gix::{
        config::tree::{Key, Remote},
        remote,
    };

    #[test]
    fn tag_opt() -> crate::Result {
        assert_eq!(Remote::TAG_OPT.try_into_tag_opt("--tags")?, remote::fetch::Tags::All);
        assert!(Remote::TAG_OPT.validate("--tags".into()).is_ok());
        assert_eq!(
            Remote::TAG_OPT.try_into_tag_opt("--no-tags")?,
            remote::fetch::Tags::None
        );
        assert!(Remote::TAG_OPT.validate("--no-tags".into()).is_ok());

        assert_eq!(
            Remote::TAG_OPT.try_into_tag_opt("--unknown").unwrap_err().to_string(),
            "The key \"remote.<name>.tagOpt=--unknown\" was invalid"
        );
        Ok(())
    }

    #[test]
    fn url_and_push_url() {
        assert!(Remote::URL.try_into_url("http://example.org").is_ok());
        assert!(Remote::URL.validate("http://example.org".into()).is_ok());

        assert_eq!(
            Remote::URL.try_into_url("https://").unwrap_err().to_string(),
            "The url at \"remote.<name>.url=https://\" could not be parsed"
        );
        assert!(Remote::URL.validate("http://".into()).is_err());
    }

    #[test]
    fn refspecs() {
        let fetch_spec = "+refs/heads/*:refs/remotes/origin/*";
        assert!(
            Remote::FETCH
                .try_into_refspec(fetch_spec, gix_refspec::parse::Operation::Fetch)
                .is_ok()
        );
        assert!(Remote::FETCH.validate(fetch_spec.into()).is_ok());

        let push_spec = "HEAD:refs/heads/name";
        assert!(
            Remote::PUSH
                .try_into_refspec(push_spec, gix_refspec::parse::Operation::Push)
                .is_ok()
        );
        assert!(Remote::PUSH.validate(push_spec.into()).is_ok());

        assert_eq!(
            Remote::FETCH
                .try_into_refspec("*/*/*:refs/heads/*", gix_refspec::parse::Operation::Fetch)
                .unwrap_err()
                .to_string(),
            "The refspec at \"remote.<name>.fetch=*/*/*:refs/heads/*\" could not be parsed"
        );
        assert_eq!(
            Remote::PUSH
                .try_into_refspec("*/*/*:refs/heads/*", gix_refspec::parse::Operation::Push)
                .unwrap_err()
                .to_string(),
            "The refspec at \"remote.<name>.push=*/*/*:refs/heads/*\" could not be parsed"
        );
    }
}
