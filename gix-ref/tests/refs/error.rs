use gix_error::{Error, ErrorExt};
use gix_ref::{file::ReferenceExt, packed, transaction::PreviousValue};

#[test]
fn missing_references_remain_classified_after_erasure() -> crate::Result {
    let store = crate::file::store_with_packed_refs()?;
    let packed = store.open_packed_buffer()?.expect("the fixture has packed refs");
    let missing = std::ffi::OsStr::new("missing");
    for err in [
        Error::from(store.find(missing).expect_err("the reference is absent")),
        Error::from(store.find_loose(missing).expect_err("the reference is absent")),
        Error::from(
            store
                .find_packed(missing, Some(&packed))
                .expect_err("the reference is absent"),
        ),
        Error::from(packed.find(missing).expect_err("the reference is absent")),
    ] {
        assert!(
            err.is_not_found(),
            "missing loose and packed refs are classified: {err}"
        );
        assert!(
            err.and_raise(gix_error::message("reference lookup failed"))
                .into_error()
                .is_not_found(),
            "additional context preserves the classification"
        );
    }
    Ok(())
}

#[test]
fn peeling_missing_targets_is_classified() -> crate::Result {
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
    assert_eq!(
        details.message, "Could not peel packed reference",
        "the peeling message is unchanged"
    );
    assert_eq!(
        details.values.len(),
        2,
        "peeling context contains only the object and reference"
    );
    assert_eq!(
        details.values["object_id"],
        gix_error::Value::String(blob_id.to_string()),
        "object ids remain hex text"
    );
    assert_eq!(
        details.values["reference"],
        gix_error::Value::Bytes(name.into()),
        "reference names remain bytes"
    );
    assert!(
        packed_err.downcast_any_ref::<gix_error::NotFoundError>().is_some(),
        "the original missing-object error remains accessible"
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
        assert!(
            err.is_not_found(),
            "missing referents and objects are classified: {err}"
        );
        assert!(!err.is_corrupted(), "absence alone does not imply corruption");
    }
    Ok(())
}

#[test]
fn malformed_tags_are_corruption_instead_of_missing_objects() -> crate::Result {
    struct MalformedTag;
    impl gix_object::Find for MalformedTag {
        fn try_find<'a>(
            &self,
            object_id: &gix_hash::oid,
            _buffer: &'a mut Vec<u8>,
        ) -> Result<Option<gix_object::Data<'a>>, gix_error::Exn> {
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
    assert!(err.is_corrupted(), "malformed stored tag data is corruption");
    assert!(
        !err.is_not_found(),
        "the malformed tag was found in the object database"
    );
    assert!(
        err.downcast_any_ref::<gix_error::ValidationError>().is_some(),
        "the tag parser's original error remains available"
    );
    Ok(())
}

#[test]
fn malformed_reference_data_is_classified() -> crate::Result {
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
                .find_map(Result::err)
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
        assert!(err.is_corrupted(), "malformed stored data is classified: {err}");
        assert!(!err.is_not_found(), "malformed stored data is present");
    }
    Ok(())
}

#[test]
fn missing_transaction_targets_are_classified() -> crate::Result {
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
        assert!(err.is_not_found(), "updates and deletions classify missing refs: {err}");
    }
    Ok(())
}

#[test]
fn invalid_reflog_input_is_classified() -> crate::Result {
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
        .expect_err("writing a reflog requires a committer")
        .into_error();
    assert!(
        missing_committer
            .downcast_any_ref::<gix_ref::file::log::create_or_update::MissingCommitter>()
            .is_some(),
        "callers can request an identity without parsing the diagnostic message"
    );
    for err in [
        Error::from_error(line.write_to(&mut Vec::new()).expect_err("newlines are forbidden")),
        missing_committer,
    ] {
        assert!(err.is_validation(), "invalid reflog input is classified: {err}");
        assert!(!err.is_corrupted(), "invalid input does not imply corrupt stored data");
    }
    Ok(())
}

#[test]
fn malformed_packed_names_and_reflog_signatures_retain_parser_errors() -> crate::Result {
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
    assert!(err.is_corrupted());
    assert!(
        err.downcast_any_ref::<gix_ref::name::Error>().is_some(),
        "packed iteration retains the name validator's error"
    );

    let line = format!("{0} {0} invalid signature\tmessage", hash.null());
    let err =
        Error::from(gix_ref::file::log::LineRef::from_bytes(line.as_bytes()).expect_err("the signature is invalid"));
    assert!(err.is_corrupted());
    assert!(
        err.downcast_any_ref::<gix_error::ValidationError>().is_some(),
        "reflog decoding retains the signature validator's error"
    );
    Ok(())
}

#[test]
fn custom_name_conversion_errors_keep_their_sources() -> crate::Result {
    struct Name<E>(E);
    impl<E> TryInto<&'static gix_ref::PartialNameRef> for Name<E> {
        type Error = E;
        fn try_into(self) -> Result<&'static gix_ref::PartialNameRef, E> {
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
            let err = err.into_error();
            assert!(err.downcast_any_ref::<Custom>().is_some(), "custom type is preserved");
            assert_eq!(err.can_retry(), kind == std::io::ErrorKind::TimedOut);
            assert_eq!(err.is_not_found(), kind == std::io::ErrorKind::NotFound);
        }
    }
    Ok(())
}

#[test]
fn a_depth_limit_does_not_imply_corruption() -> crate::Result {
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
    assert!(!err.is_corrupted());
    assert!(!err.is_not_found());
    assert_eq!(
        err.metadata().next().expect("limit details").values["max_depth"],
        gix_error::Value::from(5_usize)
    );
    Ok(())
}
