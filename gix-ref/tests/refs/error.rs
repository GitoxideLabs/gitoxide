use crate::Result;
use gix_error::{Class, Error, ErrorExt, ExnResult, Message, MetadataValue};
use gix_ref::{file::ReferenceExt, packed, transaction::PreviousValue};

#[test]
fn missing_references_retain_their_name_and_classification() -> Result {
    let mut error_snapshots = Vec::new();
    let store = crate::file::store_with_packed_refs()?;
    let packed = store.open_packed_buffer()?.expect("the fixture has packed refs");
    let missing = std::ffi::OsStr::new("missing");
    for err in [
        store.find(missing).expect_err("the reference is absent"),
        store.find_loose(missing).expect_err("the reference is absent"),
        store
            .find_packed(missing, Some(&packed))
            .expect_err("the reference is absent"),
        packed.find(missing).expect_err("the reference is absent"),
    ] {
        error_snapshots.push(gix_testtools::redact_debug_snapshot(&(err), &[]));
        assert!(
            err.is_not_found(),
            "missing loose and packed refs are classified: {err}"
        );
        assert!(
            err.probable_cause().is::<gix_ref::file::find::NotFound>(),
            "the missing reference, not its classification marker, is the probable cause"
        );
        assert_eq!(
            err.downcast_any_ref::<gix_ref::file::find::NotFound>()
                .expect("the concrete reference lookup error is retained")
                .name
                .as_os_str(),
            missing,
            "the lookup failure retains the missing reference name"
        );
    }
    insta::assert_debug_snapshot!(error_snapshots, "missing references remain classified after erasure", @r#"
    [
        The ref partially named "missing" could not be found,
        The ref partially named "missing" could not be found,
        The ref partially named "missing" could not be found,
        The ref partially named "missing" could not be found,
    ]
    "#);
    Ok(())
}

#[test]
fn peeling_missing_targets_is_classified() -> Result {
    let mut error_snapshots = Vec::new();
    use gix_lock::acquire::Fail;
    use gix_ref::{file::transaction::PackedRefs, transaction::RefEdit};

    let (_keep, packed_store) = crate::file::transaction::prepare_and_commit::empty_store()?;
    let blob_id = crate::fixture_hash_kind().empty_blob();
    let name = "refs/tags/missing";
    let packed_err = packed_store
        .transaction()
        .packed_refs(PackedRefs::DeletionsAndNonSymbolicUpdates(Box::new(
            gix_object::find::Never,
        )))
        .prepare(
            [RefEdit::update(
                name.try_into()?,
                gix_ref::Target::Object(blob_id),
                PreviousValue::Any,
                "",
            )],
            Fail::Immediately,
            Fail::Immediately,
        )
        .expect_err("the object to pack is absent")
        .into_error();
    let details = packed_err.metadata().next().expect("packed peeling context");
    let diagnostic = packed_err
        .downcast_any_ref::<Message>()
        .expect("packed peeling diagnostic");
    assert_eq!(
        details.len(),
        2,
        "peeling context contains only the object and reference"
    );
    assert_eq!(
        details["object_id"],
        gix_error::MetadataValue::String(blob_id.to_string()),
        "object ids remain hex text"
    );
    assert_eq!(
        details["reference"],
        gix_error::MetadataValue::Bytes(name.into()),
        "reference names remain bytes"
    );
    assert_eq!(
        diagnostic.class,
        Some(Class::NotFound),
        "the missing-object classification remains accessible on the diagnostic"
    );

    let store = crate::file::store_at("make_ref_repository.sh")?;
    let mut symbolic = store.find("HEAD")?;
    symbolic.target = gix_ref::Target::Symbolic("refs/heads/missing".try_into()?);
    let mut direct = store.find("main")?;
    for err in [
        Error::from(
            symbolic
                .follow(&store)
                .expect("HEAD is symbolic")
                .expect_err("the referent is absent"),
        ),
        Error::from(
            symbolic
                .peel_to_id(&store, &gix_object::find::Never)
                .expect_err("the symbolic target is absent"),
        ),
        Error::from(
            direct
                .peel_to_id(&store, &gix_object::find::Never)
                .expect_err("the object database is empty"),
        ),
        packed_err,
    ] {
        error_snapshots.push(gix_testtools::redact_debug_snapshot(&(err), &[]));
        assert!(
            err.is_not_found(),
            "missing referents and objects are classified: {err}"
        );
        assert!(!err.is_corrupted(), "absence alone does not imply corruption");
    }
    insta::assert_debug_snapshot!(error_snapshots, "peeling missing targets is classified", @r#"
    [
        The ref partially named "refs/heads/missing" could not be found,
        The ref partially named "refs/heads/missing" could not be found,
        Could not peel reference to an object: object could not be found, "object_id"="Oid(1)", "reference"="refs/heads/main",
        Could not peel packed reference: object could not be found, "object_id"="Oid(1)", "reference"="refs/tags/missing",
    ]
    "#);
    Ok(())
}

#[test]
fn peeling_missing_objects_has_one_classified_diagnostic() -> Result {
    let mut error_snapshots = Vec::new();
    for err in peeling_errors(gix_object::find::Never)? {
        error_snapshots.push(gix_testtools::redact_debug_snapshot(&(err), &[]));
        assert!(err.is_not_found(), "an absent object is classified as not found");
        assert_eq!(err.iter_errors().count(), 1, "absence does not need a synthetic cause");
        let details = err.metadata().next().expect("peeling details survive conversion");
        let diagnostic = err.downcast_any_ref::<Message>().expect("peeling diagnostic");
        assert_eq!(
            diagnostic.class,
            Some(Class::NotFound),
            "the diagnostic carries its class"
        );
        assert_eq!(
            details["object_id"],
            MetadataValue::from(crate::fixture_hash_kind().null().to_string()),
            "the missing object is identified"
        );
        assert_eq!(
            details["reference"],
            MetadataValue::from(b"refs/tags/tag".as_slice()),
            "the reference being peeled is identified"
        );
        assert!(
            err.probable_cause().is::<Message>(),
            "the classified diagnostic itself is the probable cause"
        );
    }
    insta::assert_debug_snapshot!(error_snapshots, "peeling missing objects has one classified diagnostic", @r#"
    [
        Could not peel reference to an object: object could not be found, "object_id"="Oid(1)", "reference"="refs/tags/tag",
        Could not peel packed reference: object could not be found, "object_id"="Oid(1)", "reference"="refs/tags/tag",
    ]
    "#);
    Ok(())
}

#[test]
fn object_lookup_failures_retain_their_causes() -> Result {
    let mut error_snapshots = Vec::new();
    struct UnavailableObjects(std::io::ErrorKind);
    impl gix_object::Find for UnavailableObjects {
        fn try_find<'a>(
            &self,
            _object_id: &gix_hash::oid,
            _buffer: &'a mut Vec<u8>,
        ) -> ExnResult<Option<gix_object::Data<'a>>> {
            Err(std::io::Error::new(self.0, gix_error::message("object database unavailable")).raise_erased())
        }
    }

    for kind in [std::io::ErrorKind::PermissionDenied, std::io::ErrorKind::TimedOut] {
        for err in peeling_errors(UnavailableObjects(kind))? {
            error_snapshots.push(gix_testtools::redact_debug_snapshot(&(err), &[]));
            assert!(!err.is_not_found(), "a failed lookup does not imply an absent object");
            assert!(!err.is_corrupted(), "a failed lookup does not imply corrupt data");
            assert_eq!(
                err.can_retry(),
                kind == std::io::ErrorKind::TimedOut,
                "retryability still comes from the callee"
            );
            assert_eq!(
                err.iter_errors().count(),
                3,
                "the context retains the real I/O error and its payload"
            );
            assert_eq!(
                err.downcast_any_ref::<std::io::Error>()
                    .expect("the I/O error remains available")
                    .kind(),
                kind,
                "the callee's concrete error remains intact"
            );
            let details = err.metadata().next().expect("peeling context");
            assert_eq!(
                err.downcast_any_ref::<Message>().expect("peeling context").class,
                None,
                "the context does not reclassify the lookup failure"
            );
            assert_eq!(
                details["object_id"],
                MetadataValue::from(crate::fixture_hash_kind().null().to_string()),
                "failed lookups retain the requested object"
            );
            assert_eq!(
                details["reference"],
                MetadataValue::from(b"refs/tags/tag".as_slice()),
                "failed lookups retain the reference being peeled"
            );
        }
    }
    insta::assert_debug_snapshot!(error_snapshots, "object lookup failures retain their causes", @r#"
    [
        Could not peel reference to an object, "object_id"="Oid(1)", "reference"="refs/tags/tag"
        |
        └─ I/O error (PermissionDenied)
        |
        └─ object database unavailable,
        Could not peel packed reference, "object_id"="Oid(1)", "reference"="refs/tags/tag"
        |
        └─ I/O error (PermissionDenied)
        |
        └─ object database unavailable,
        Could not peel reference to an object, "object_id"="Oid(1)", "reference"="refs/tags/tag"
        |
        └─ I/O error (TimedOut)
        |
        └─ object database unavailable,
        Could not peel packed reference, "object_id"="Oid(1)", "reference"="refs/tags/tag"
        |
        └─ I/O error (TimedOut)
        |
        └─ object database unavailable,
    ]
    "#);
    Ok(())
}

fn peeling_errors(objects: impl gix_object::Find) -> Result<[gix_error::Exn; 2]> {
    use gix_lock::acquire::Fail;
    use gix_ref::{file::transaction::PackedRefs, transaction::RefEdit};

    let (_keep, store) = crate::file::transaction::prepare_and_commit::empty_store()?;
    let mut reference = gix_ref::Reference {
        name: "refs/tags/tag".try_into()?,
        target: gix_ref::Target::Object(crate::fixture_hash_kind().null()),
        peeled: None,
    };
    let peel_error = reference
        .peel_to_id(&store, &objects)
        .expect_err("the supplied object finder fails");
    let prepare_error = store
        .transaction()
        .packed_refs(PackedRefs::DeletionsAndNonSymbolicUpdates(Box::new(objects)))
        .prepare(
            [RefEdit::update(
                reference.name,
                reference.target,
                PreviousValue::Any,
                "",
            )],
            Fail::Immediately,
            Fail::Immediately,
        )
        .expect_err("the supplied object finder also fails while preparing packed refs");
    Ok([peel_error, prepare_error])
}

#[test]
fn malformed_tags_are_corruption_instead_of_missing_objects() -> Result {
    struct MalformedTag;
    impl gix_object::Find for MalformedTag {
        fn try_find<'a>(
            &self,
            object_id: &gix_hash::oid,
            _buffer: &'a mut Vec<u8>,
        ) -> ExnResult<Option<gix_object::Data<'a>>> {
            Ok(Some(gix_object::Data {
                kind: gix_object::Kind::Tag,
                object_hash: object_id.kind(),
                data: b"object invalid\n",
            }))
        }
    }

    let store = crate::file::store_at("make_ref_repository.sh")?;
    let err = Error::from(
        store
            .find("main")?
            .peel_to_id(&store, &MalformedTag)
            .expect_err("the tag target cannot be decoded"),
    );
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&(err), &[]), "malformed stored tag data is corruption", @r#"
    Could not decode tag Oid(1) as referred to by "refs/heads/main"
    |
    └─ object parsing failed
    "#);
    assert!(err.is_corrupted(), "malformed stored tag data is corruption");
    assert!(
        !err.is_not_found(),
        "the malformed tag was found in the object database"
    );
    assert!(err.is_validation(), "the tag parser's original error remains available");
    Ok(())
}

#[test]
fn malformed_reference_data_is_classified() -> Result {
    let mut error_snapshots = Vec::new();
    let store = crate::file::store_at("make_ref_repository.sh")?;
    let hash = crate::fixture_hash_kind();
    let packed = packed::Buffer::from_bytes(
        b"# pack-refs with: peeled fully-peeled sorted\nbogus refs/heads/main\n",
        hash,
    )?;
    for err in [
        Error::from(
            gix_ref::file::loose::Reference::try_from_path("HEAD".try_into()?, b"invalid", hash)
                .expect_err("the loose ref is malformed"),
        ),
        Error::from(packed::Buffer::from_bytes(b"# invalid\n", hash).expect_err("the header is malformed")),
        Error::from(packed.find("main").expect_err("the packed ref is malformed")),
        Error::from(
            packed
                .iter()?
                .next()
                .expect("one packed ref")
                .expect_err("the ref is malformed"),
        ),
        Error::from(
            store
                .iter_packed(Some(&packed))?
                .find_map(std::result::Result::err)
                .expect("the overlay encounters the malformed packed ref"),
        ),
        Error::from(
            store
                .find("loop-a")?
                .peel_to_id(&store, &gix_object::find::Never)
                .expect_err("the symbolic refs form a cycle"),
        ),
        Error::from(gix_ref::file::log::LineRef::from_bytes(b"invalid").expect_err("the reflog line is malformed")),
        Error::from(
            gix_ref::file::log::iter::forward(b"invalid\n")
                .next()
                .expect("one reflog line")
                .expect_err("the reflog line is malformed"),
        ),
    ] {
        error_snapshots.push(gix_testtools::redact_debug_snapshot(
            &(err),
            &[(&(store.git_dir()).to_string_lossy(), "<git-dir>")],
        ));
        assert!(err.is_corrupted(), "malformed stored data is classified: {err}");
        assert!(!err.is_not_found(), "malformed stored data is present");
    }
    insta::assert_debug_snapshot!(error_snapshots, "malformed reference data is classified", @r#"
    [
        Reference content could not be parsed, "input"="invalid",
        The header could not be parsed, even though first line started with '#',
        Could not decode packed reference, "name"="refs/main"
        |
        └─ Malformed packed reference record,
        Invalid packed reference, "input"="bogus refs/heads/main", "line"=1
        |
        └─ Malformed packed reference,
        Invalid packed reference, "input"="bogus refs/heads/main", "line"=1
        |
        └─ Malformed packed reference,
        Aborting symbolic reference cycle, "path"="<git-dir>/refs/loop-a",
        Could not decode reflog line, "input"="invalid"
        |
        └─ Malformed reflog line,
        Invalid reflog entry, "from_end"=false, "line"=1
        |
        └─ Could not decode reflog line, "input"="invalid"
        |
        └─ Malformed reflog line,
    ]
    "#);
    Ok(())
}

#[test]
fn missing_transaction_targets_are_classified() -> Result {
    let mut error_snapshots = Vec::new();
    use gix_lock::acquire::Fail;
    use gix_ref::transaction::RefEdit;

    let (_keep, store) = crate::file::transaction::prepare_and_commit::empty_store()?;
    for edit in [
        RefEdit::delete("refs/heads/missing".try_into()?, PreviousValue::MustExist),
        RefEdit::update(
            "refs/heads/missing".try_into()?,
            gix_ref::Target::Object(crate::fixture_hash_kind().null()),
            PreviousValue::MustExist,
            "update",
        ),
    ] {
        let err = Error::from(
            store
                .transaction()
                .prepare([edit], Fail::Immediately, Fail::Immediately)
                .expect_err("the edit requires an existing reference"),
        );
        error_snapshots.push(gix_testtools::redact_debug_snapshot(&(err), &[]));
        assert!(err.is_not_found(), "updates and deletions classify missing refs: {err}");
    }
    insta::assert_debug_snapshot!(error_snapshots, "missing transaction targets are classified", @r#"
    [
        Could not prepare reference edit, "reference"="refs/heads/missing", "referent"="refs/heads/missing"
        |
        └─ The reference to delete must exist,
        Could not prepare reference edit, "reference"="refs/heads/missing", "referent"="refs/heads/missing"
        |
        └─ The reference to update must exist,
    ]
    "#);
    Ok(())
}

#[test]
fn invalid_reflog_input_is_classified() -> Result {
    let mut error_snapshots = Vec::new();
    use crate::file::transaction::prepare_and_commit::{committer, create_at, empty_store};
    use gix_lock::acquire::Fail;

    let (_keep, store) = empty_store()?;
    let line = gix_ref::log::Line {
        previous_oid: crate::fixture_hash_kind().null(),
        new_oid: crate::fixture_hash_kind().null(),
        signature: committer(),
        message: "invalid\nmessage".into(),
    };
    let missing_committer = store
        .transaction()
        .prepare([create_at("refs/heads/new")], Fail::Immediately, Fail::Immediately)?
        .commit(None)
        .expect_err("writing a reflog requires a committer");
    assert!(
        missing_committer.is_validation(),
        "a missing reflog identity is invalid input"
    );
    assert!(
        missing_committer
            .probable_cause()
            .is::<gix_ref::file::log::create_or_update::MissingCommitter>(),
        "the missing committer, not its classification marker, is the probable cause"
    );
    assert!(
        missing_committer
            .downcast_any_ref::<gix_ref::file::log::create_or_update::MissingCommitter>()
            .is_some(),
        "callers can request an identity without parsing the diagnostic message"
    );
    for err in [
        Error::from_error(line.write_to(&mut Vec::new()).expect_err("newlines are forbidden")),
        missing_committer.into_error(),
    ] {
        error_snapshots.push(gix_testtools::redact_debug_snapshot(&(err), &[]));
        assert!(err.is_validation(), "invalid reflog input is classified: {err}");
        assert!(!err.is_corrupted(), "invalid input does not imply corrupt stored data");
    }
    insta::assert_debug_snapshot!(error_snapshots, "invalid reflog input is classified", @r#"
    [
        Custom {
            kind: Other,
            error: Messages must not contain newlines (\n),
        },
        Could not update reflog, "reference"="refs/heads/new"
        |
        └─ reflog messages need a committer which isn't set,
    ]
    "#);
    Ok(())
}

#[test]
fn malformed_packed_names_and_reflog_signatures_retain_parser_errors() -> Result {
    let hash = crate::fixture_hash_kind();
    let packed = packed::Buffer::from_bytes(
        format!("# pack-refs with: sorted\n{} refs/heads/bad..name\n", hash.null()).as_bytes(),
        hash,
    )?;
    let err = Error::from(
        packed
            .iter()?
            .next()
            .expect("one packed ref")
            .expect_err("the name is invalid"),
    );
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&(err), &[]), "malformed packed names and reflog signatures retain parser errors", @r#"
    Invalid packed reference, "input"="Oid(1) refs/heads/bad..name", "line"=1
    |
    └─ Malformed packed reference
    |
    └─ Reference name cannot contain repeated dots
    "#);
    assert!(err.is_corrupted());
    assert!(
        err.downcast_any_ref::<gix_ref::name::Error>().is_some(),
        "packed iteration retains the name validator's error"
    );

    let line = format!("{0} {0} invalid signature\tmessage", hash.null());
    let err =
        Error::from(gix_ref::file::log::LineRef::from_bytes(line.as_bytes()).expect_err("the signature is invalid"));
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&(err), &[]), "malformed packed names and reflog signatures retain parser errors", @r#"
    Could not decode reflog line, "input"="Oid(1) Oid(1) invalid signature\tmessage"
    |
    └─ Invalid reflog signature
    |
    └─ Closing '>' not found
    "#);
    assert!(err.is_corrupted());
    assert!(
        err.is_validation(),
        "reflog decoding retains the signature validator's error"
    );
    Ok(())
}

#[test]
fn custom_name_conversion_errors_keep_their_sources() -> Result {
    let mut error_snapshots = Vec::new();
    struct Name<E>(E);
    impl<E> TryInto<&'static gix_ref::PartialNameRef> for Name<E> {
        type Error = E;
        fn try_into(self) -> std::result::Result<&'static gix_ref::PartialNameRef, E> {
            Err(self.0)
        }
    }
    #[derive(Debug)]
    struct Custom(std::io::Error);
    impl std::fmt::Display for Custom {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("custom name conversion")
        }
    }
    impl std::error::Error for Custom {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.0)
        }
    }
    let store = crate::file::store_at("make_ref_repository.sh")?;
    for kind in [std::io::ErrorKind::TimedOut, std::io::ErrorKind::NotFound] {
        for err in [
            store.try_find(Name(Custom(kind.into()))).expect_err("conversion fails"),
            store
                .try_find(Name(Custom(kind.into()).raise()))
                .expect_err("conversion fails"),
        ] {
            error_snapshots.push(gix_testtools::redact_debug_snapshot(&(err), &[]));
            assert!(err.downcast_any_ref::<Custom>().is_some(), "custom type is preserved");
            assert_eq!(err.can_retry(), kind == std::io::ErrorKind::TimedOut);
            assert_eq!(err.is_not_found(), kind == std::io::ErrorKind::NotFound);
        }
    }
    insta::assert_debug_snapshot!(error_snapshots, "custom name conversion errors keep their sources", @"
    [
        The ref name or path is not a valid ref name
        |
        └─ custom name conversion
        |
        └─ timed out,
        The ref name or path is not a valid ref name
        |
        └─ custom name conversion
        |
        └─ timed out,
        The ref name or path is not a valid ref name
        |
        └─ custom name conversion
        |
        └─ entity not found,
        The ref name or path is not a valid ref name
        |
        └─ custom name conversion
        |
        └─ entity not found,
    ]
    ");
    Ok(())
}

#[test]
fn a_depth_limit_does_not_imply_corruption() -> Result {
    let (_keep, store) = crate::file::transaction::prepare_and_commit::empty_store()?;
    let refs = store.git_dir().join("refs/heads");
    std::fs::create_dir_all(&refs)?;
    for index in 0..6 {
        std::fs::write(
            refs.join(format!("r{index}")),
            if index == 5 {
                format!("{}\n", crate::fixture_hash_kind().null())
            } else {
                format!("ref: refs/heads/r{}\n", index + 1)
            },
        )?;
    }
    let err = store
        .find("r0")?
        .follow_to_object_packed(&store, None)
        .expect_err("the valid symbolic chain exceeds the depth limit")
        .into_error();
    insta::assert_debug_snapshot!(err, "a valid symbolic chain need not be corrupted", @r#"Symbolic reference depth limit exceeded, "max_depth"=5"#);
    assert!(!err.is_corrupted(), "a valid symbolic chain need not be corrupted");
    assert!(!err.is_not_found(), "all symbolic targets exist");
    let details = err.metadata().next().expect("limit details");
    assert_eq!(
        err.downcast_any_ref::<Message>().expect("limit diagnostic").class,
        None,
        "the depth limit remains unclassified"
    );
    assert!(
        err.classify().next().is_none(),
        "no classification is added through a cause"
    );
    assert_eq!(
        details["max_depth"],
        MetadataValue::from(5_usize),
        "the configured depth limit is retained"
    );
    Ok(())
}

#[test]
fn loose_reference_diagnostics_keep_input_with_the_failure() -> Result {
    let hash = crate::fixture_hash_kind();
    let contents = b"invalid\xff";
    let err = gix_ref::file::loose::Reference::try_from_path("HEAD".try_into()?, contents, hash)
        .expect_err("a malformed object id is corruption");
    insta::assert_debug_snapshot!(err, "scalar context does not invent a validation failure", @r#"Reference content could not be parsed, "input"="invalid\xff""#);
    assert!(
        err.is_corrupted() && !err.is_validation(),
        "scalar context does not invent a validation failure"
    );
    assert_eq!(
        err.iter_errors().count(),
        1,
        "the syntax failure and its input form one diagnostic"
    );
    assert_eq!(
        err.metadata().next().expect("reference input")["input"],
        gix_error::MetadataValue::from(contents.as_slice()),
        "non-UTF8 reference contents remain bytes"
    );

    let err = gix_ref::file::loose::Reference::try_from_path("HEAD".try_into()?, b"ref: refs/heads/.bad\n", hash)
        .expect_err("a symbolic target must be a valid reference name");
    insta::assert_debug_snapshot!(err, "the callee's validation class remains available", @r#"
    Could not decode reference, "input"="ref: refs/heads/.bad\n"
    |
    └─ Invalid symbolic reference target, "target"="refs/heads/.bad"
    |
    └─ Reference name cannot start with a dot
    "#);
    assert!(err.is_validation(), "the callee's validation class remains available");
    assert!(
        err.downcast_any_ref::<gix_validate::reference::name::Error>().is_some(),
        "the real symbolic-target validator remains a cause"
    );
    assert_eq!(
        err.metadata().next().expect("reference input")["input"],
        gix_error::MetadataValue::from(b"ref: refs/heads/.bad\n".as_slice()),
        "symbolic-target context retains the complete contents"
    );
    Ok(())
}
