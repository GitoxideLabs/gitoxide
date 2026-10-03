use gix_error::{Class, ErrorExt, ExnResult, Result};

#[test]
fn public_results_preserve_recovery_types() {
    let parsed: Result<gix_hash::ObjectId> = gix_hash::ObjectId::from_hex(b"invalid");
    assert!(
        parsed.expect_err("invalid object id").is_validation(),
        "parsing retains the validation classification"
    );

    let converted: Result<gix_hash::ObjectId> = gix_hash::ObjectId::try_from(&b"invalid"[..]);
    assert!(
        converted.expect_err("invalid binary object id").is_validation(),
        "conversion retains the validation classification"
    );

    let parsed: ExnResult<_, gix_command::parse::Error> = gix_command::parse::command_line(b"'".as_slice().into());
    let err = parsed.expect_err("an opening quote must be closed");
    assert_eq!(
        err.error(),
        &gix_command::parse::Error::MissingClosingQuote,
        "the public error exposes the concrete parser failure directly"
    );
}

#[test]
fn intrinsic_input_and_capability_failures_remain_distinct() {
    use gix::config::tree::Extensions;

    #[cfg(feature = "blob-diff")]
    for (name, class) in [("unknown", Class::Validation), ("patience", Class::Unsupported)] {
        let err = gix::config::tree::Diff::ALGORITHM
            .try_into_algorithm(name)
            .expect_err("the algorithm is unavailable");
        assert_eq!(
            err.classify()
                .map(|classification| classification.class())
                .collect::<Vec<_>>(),
            [class],
            "only the intrinsic classification is present"
        );
        assert!(
            err.downcast_any_ref::<gix::config::diff::algorithm::Error>().is_some(),
            "classification retains the concrete algorithm recovery error"
        );
    }
    let invalid = Extensions::OBJECT_FORMAT
        .try_into_object_format("invalid")
        .expect_err("unknown hash name");
    assert!(invalid.is_validation(), "an unknown hash name is invalid input");
    assert!(
        !invalid.is_unsupported(),
        "unknown input is not a known disabled capability"
    );
    for (name, enabled) in [("sha1", cfg!(feature = "sha1")), ("sha256", cfg!(feature = "sha256"))] {
        if !enabled {
            let err = Extensions::OBJECT_FORMAT
                .try_into_object_format(name)
                .expect_err("the hash is disabled");
            assert!(err.is_unsupported(), "a known disabled hash requires changing strategy");
            assert!(!err.is_validation(), "a known hash name is valid input");
            assert!(
                err.metadata()
                    .any(|metadata| metadata.get("input") == Some(&gix_error::MetadataValue::from(name.as_bytes()))),
                "the unavailable object format retains its raw input"
            );
        }
    }
}

#[test]
fn generic_key_validation_preserves_unknown_callee_classification() {
    use gix::config::tree::{Core, Key, keys};

    struct Unknown;
    impl keys::Validate for Unknown {
        fn validate(&self, _value: &gix::bstr::BStr) -> gix::Result<()> {
            Err(gix_error::message("custom validator failed without a recovery signal").raise())
        }
    }
    let key = keys::Any::new_with_validate("custom", &Core, Unknown);
    let err = key.validate("input".into()).expect_err("the custom validator fails");
    assert_eq!(
        err.classify().count(),
        0,
        "a generic adapter cannot infer invalid input from failure"
    );
    assert!(
        err.metadata()
            .next()
            .is_some_and(|metadata| metadata.contains_key("key") && metadata.contains_key("input")),
        "unclassified context still identifies the key and input"
    );
}

#[test]
fn clone_revision_classification_preserves_real_sources() {
    let invalid = gix::clone::with_revision::Error::Invalid {
        revision: "main~1".into(),
    }
    .raise_typed();
    assert!(
        invalid.is_validation(),
        "single-revision clone restrictions are invalid input"
    );
    let parsed =
        gix::clone::with_revision::Error::Parse(std::io::Error::from(std::io::ErrorKind::PermissionDenied).raise())
            .raise_typed();
    assert!(
        parsed.is_permission_denied(),
        "the real callee determines the classification"
    );
    assert!(
        !parsed.is_validation(),
        "the parse wrapper does not override a real source"
    );
    assert!(
        parsed.downcast_any_ref::<std::io::Error>().is_some(),
        "the I/O source survives the typed wrapper"
    );
}

#[cfg(feature = "worktree-mutation")]
#[test]
fn worktree_and_branch_rejections_have_intrinsic_recovery_classes() {
    use gix::worktree::{add, remove};

    let cancelled = add::Error::Interrupted.raise_typed();
    assert!(
        cancelled.is_cancelled(),
        "observed cancellation is not retryable I/O interruption"
    );
    assert!(
        !cancelled.can_retry() && !cancelled.can_retry_lenient(),
        "callers must stop rather than retry cancellation"
    );
    for rejection in [
        add::Error::CheckedOut {
            name: "refs/heads/main".try_into().expect("valid branch"),
            worktree_dirs: vec!["main".into()],
        },
        add::Error::DestinationRegistered {
            destination: "linked".into(),
        },
    ] {
        let err = rejection.raise_typed();
        assert!(err.is_conflict(), "occupied state must be reconciled");
        assert!(!err.is_validation(), "valid targets are not malformed input");
        assert_eq!(
            err.iter_errors().count(),
            1,
            "hidden markers do not add diagnostic causes"
        );
    }
    for rejection in [
        remove::Error::Locked {
            path: "linked".into(),
            reason: None,
        },
        remove::Error::Dirty { path: "linked".into() },
        remove::Error::ContainsSubmodule { path: "linked".into() },
    ] {
        assert!(
            rejection.raise_typed().is_conflict(),
            "removal requires reconciling worktree state"
        );
    }
    let err = gix::repository::branch::delete::CheckedOutError {
        name: "refs/heads/main".try_into().expect("valid branch"),
        worktree_dirs: vec!["main".into()],
    }
    .raise_typed();
    assert!(
        err.is_conflict(),
        "deleting an occupied branch requires reconciling state"
    );
    let cleanup = gix::repository::branch::delete::CleanupError {
        references: Vec::new(),
        deleted: Vec::new(),
    }
    .raise_typed();
    assert_eq!(
        cleanup.classify().count(),
        0,
        "partial cleanup alone does not imply a recovery strategy"
    );
}

#[test]
fn opening_keeps_configuration_failure_classification_specific() -> gix_error::TestResult {
    let directory = gix_testtools::tempfile::TempDir::new()?;
    let repo = crate::init_repo_isolated(directory.path(), gix::create::Kind::Bare)?;
    std::fs::write(
        repo.git_dir().join("config"),
        b"[core]\nrepositoryFormatVersion = 0\n[extensions]\nobjectFormat = sha1\n",
    )?;
    let err =
        gix::open_opts(directory.path(), gix::open::Options::isolated()).expect_err("objectFormat requires version 1");
    assert!(err.is_validation(), "the contradictory configuration is invalid input");
    assert!(
        !err.is_corrupted(),
        "loading context does not classify every callee failure as corruption"
    );
    assert_eq!(err.classify().count(), 1, "only the callee supplies classification");
    Ok(())
}

#[test]
fn native_error_adapters_capture_their_call_site() {
    let err = gix::config::tree::branch::Merge::try_into_fullrefname("refs/heads/invalid name")
        .expect_err("reference names cannot contain spaces");
    let source = err.iter_errors_with_locations().next().expect("the error is present");
    let location = source.location().expect("adapting the error captures a location");
    assert!(
        std::path::Path::new(location.file()).ends_with("gix/src/config/tree/sections/branch.rs"),
        "the location must identify the gix adapter, not a function-call shim: {location}"
    );
}
