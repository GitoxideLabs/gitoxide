use gix_error::{Error, ErrorExt};
use gix_ref::{file::ReferenceExt, packed, transaction::PreviousValue};

#[test]
fn missing_references_remain_classified_after_erasure() -> crate::Result {
    let store = crate::file::store_with_packed_refs()?;
    let packed = store.open_packed_buffer()?.expect("the fixture has packed refs");
    for err in [
        Error::from_error(store.find("missing").expect_err("the reference is absent")),
        Error::from_error(store.find_loose("missing").expect_err("the reference is absent")),
        Error::from_error(
            store
                .find_packed("missing", Some(&packed))
                .expect_err("the reference is absent"),
        ),
        Error::from_error(packed.find("missing").expect_err("the reference is absent")),
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
    let store = crate::file::store_at("make_ref_repository.sh")?;
    let mut symbolic = store.find("HEAD")?;
    symbolic.target = gix_ref::Target::Symbolic("refs/heads/missing".try_into()?);
    let mut direct = store.find("main")?;
    for err in [
        Error::from_error(
            symbolic
                .follow(&store)
                .expect("HEAD is symbolic")
                .expect_err("the referent is absent"),
        ),
        Error::from_error(
            symbolic
                .peel_to_id(&store, &gix_object::find::Never)
                .expect_err("the symbolic target is absent"),
        ),
        Error::from_error(
            direct
                .peel_to_id(&store, &gix_object::find::Never)
                .expect_err("the object database is empty"),
        ),
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
    let err = Error::from_error(
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
        Error::from_error(
            gix_ref::file::loose::Reference::try_from_path("HEAD".try_into()?, b"invalid", hash)
                .expect_err("the loose ref is malformed"),
        ),
        Error::from_error(packed::Buffer::from_bytes(b"# invalid\n", hash).expect_err("the header is malformed")),
        Error::from_error(packed.find("main").expect_err("the packed ref is malformed")),
        Error::from_error(
            packed
                .iter()?
                .next()
                .expect("one packed ref")
                .expect_err("the ref is malformed"),
        ),
        Error::from_error(
            store
                .iter_packed(Some(&packed))?
                .find_map(Result::err)
                .expect("the overlay encounters the malformed packed ref"),
        ),
        Error::from_error(
            store
                .find("loop-a")?
                .peel_to_id(&store, &gix_object::find::Never)
                .expect_err("the symbolic refs form a cycle"),
        ),
        Error::from_error(
            gix_ref::file::log::LineRef::from_bytes(b"invalid").expect_err("the reflog line is malformed"),
        ),
        Error::from_error(
            gix_ref::file::log::iter::forward(b"invalid\n")
                .next()
                .expect("one reflog line")
                .expect_err("the reflog line is malformed"),
        ),
    ] {
        assert!(err.is_corrupted(), "malformed stored data is classified: {err}");
        assert!(!err.is_not_found(), "malformed stored data is present");
    }
    assert!(
        !Error::from_error(gix_ref::peel::to_object::Error::DepthLimitExceeded { max_depth: 5 }).is_corrupted(),
        "a depth limit can also be reached by a valid symbolic reference chain"
    );
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
        let err = Error::from_error(
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
    for err in [
        Error::from_error(line.write_to(&mut Vec::new()).expect_err("newlines are forbidden")),
        Error::from_error(
            store
                .transaction()
                .prepare([create_at("refs/heads/new")], Fail::Immediately, Fail::Immediately)?
                .commit(None)
                .expect_err("writing a reflog requires a committer"),
        ),
    ] {
        assert!(err.is_validation(), "invalid reflog input is classified: {err}");
        assert!(!err.is_corrupted(), "invalid input does not imply corrupt stored data");
    }
    Ok(())
}

#[test]
fn existing_sources_and_retry_classifications_are_preserved() {
    for kind in [std::io::ErrorKind::PermissionDenied, std::io::ErrorKind::TimedOut] {
        let err = Error::from_error(gix_ref::file::find::existing::Error::Find(
            gix_ref::file::find::Error::ReadFileContents {
                source: std::io::Error::from(kind),
                path: "refs/heads/main".into(),
            },
        ));
        assert_eq!(
            err.downcast_any_ref::<std::io::Error>().map(std::io::Error::kind),
            Some(kind),
            "the concrete I/O cause remains accessible"
        );
        assert_eq!(
            err.can_retry(),
            kind == std::io::ErrorKind::TimedOut,
            "I/O retry policy is retained"
        );
        assert!(!err.is_not_found(), "a failed lookup does not imply absence");
    }
}
