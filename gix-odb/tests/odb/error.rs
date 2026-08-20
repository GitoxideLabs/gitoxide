use gix_error::{CorruptionError, Error, ErrorExt, message};
use gix_odb::{alternate, loose, store};

#[test]
fn io_classification_survives_custom_wrappers() {
    use std::io::ErrorKind;

    for kind in [
        ErrorKind::NotFound,
        ErrorKind::Interrupted,
        ErrorKind::OutOfMemory,
        ErrorKind::PermissionDenied,
    ] {
        let io = || std::io::Error::from(kind);
        let load_index = || store::load_index::Error::Io(io());
        for err in [
            load_index().raise_erased(),
            store::load_index::Error::Alternate(alternate::Error::Io(io())).raise_erased(),
            alternate::Error::from(io().raise_erased()).raise_erased(),
            store::find::Error::LoadIndex(load_index()).raise_erased(),
            store::find::Error::LoadPack(io()).raise_erased(),
            store::write::Error::Io(io()).raise_erased(),
            store::write::Error::LoadIndex(load_index()).raise_erased(),
            store::write::Error::LooseWrite(loose::write::Error::IoRaw(io())).raise_erased(),
            store::prefix::disambiguate::Error::Lookup(store::prefix::lookup::Error::LoadIndex(load_index()))
                .raise_erased(),
            store::verify::integrity::Error::InitializeODB(load_index()).raise_erased(),
        ] {
            assert_eq!(
                err.can_retry(),
                gix_error::can_retry(&io()),
                "custom wrappers retain the retry policy for {kind:?}"
            );
            assert_eq!(
                err.is_not_found(),
                kind == ErrorKind::NotFound,
                "custom wrappers retain missing-resource failures"
            );
            let err = err.into_error();
            assert_eq!(
                err.can_retry(),
                gix_error::can_retry(&io()),
                "conversion retains the retry policy for {kind:?}"
            );
            assert_eq!(
                err.is_not_found(),
                kind == ErrorKind::NotFound,
                "conversion retains missing-resource failures"
            );
            assert_eq!(
                err.is_resource_exhausted(),
                kind == ErrorKind::OutOfMemory,
                "conversion retains allocation failures"
            );
            assert_eq!(
                err.downcast_any_ref::<std::io::Error>()
                    .expect("retain the immediate I/O cause")
                    .kind(),
                kind
            );
        }
    }
}

#[test]
fn custom_wrappers_retain_classified_errors_and_branches() {
    let corrupt = || Error::from_error(CorruptionError::new("malformed object"));
    for err in [
        loose::find::Error::Decode(corrupt()).raise_erased(),
        store::find::Error::EntryType(CorruptionError::new("invalid pack entry type")).raise_erased(),
        store::verify::integrity::Error::MultiIndexIntegrity(corrupt()).raise_erased(),
        store::verify::integrity::Error::IndexIntegrity(corrupt()).raise_erased(),
        store::verify::integrity::Error::IndexOpen(corrupt()).raise_erased(),
        store::verify::integrity::Error::MultiIndexOpen(corrupt()).raise_erased(),
        store::verify::integrity::Error::PackOpen(
            message("pack failed")
                .raise()
                .chain(message("unrelated cause"))
                .chain(CorruptionError::new("malformed pack"))
                .into_error(),
        )
        .raise_erased(),
    ] {
        assert!(
            err.is_corrupted(),
            "custom wrappers retain classified causes, including branches"
        );
        assert!(err.into_error().is_corrupted(), "conversion retains classified causes");
    }
}

#[test]
fn custom_leaf_errors_expose_their_classification() {
    let blob_id = gix_hash::ObjectId::empty_blob(gix_hash::Kind::Sha1);
    let err = store::find::Error::DeltaBaseMissing {
        base_id: blob_id,
        id: blob_id,
    }
    .raise();
    assert!(err.is_not_found(), "a missing delta base is a missing object");
    assert!(
        err.into_error().is_not_found(),
        "conversion retains the missing-object classification"
    );

    for err in [
        loose::find::Error::SizeMismatch {
            actual: 0,
            expected: 1,
            path: "object".into(),
        }
        .raise_erased(),
        alternate::Error::Cycle(vec!["objects".into()]).raise_erased(),
    ] {
        assert!(
            err.is_corrupted(),
            "inconsistent object sizes and alternate cycles are corruption"
        );
        assert!(err.into_error().is_corrupted());
    }
    let err = alternate::Error::Parse(alternate::parse::Error::PathConversion(vec![0xff])).raise();
    assert!(err.is_validation(), "unrepresentable alternate paths are invalid input");
    assert!(err.into_error().is_validation());

    for err in [
        loose::verify::integrity::Error::Retry.raise_erased(),
        loose::verify::integrity::Error::Interrupted.raise_erased(),
        store::verify::integrity::Error::NeedsRetryDueToChangeOnDisk.raise_erased(),
        store::verify::integrity::Error::LooseObjectStoreIntegrity(loose::verify::integrity::Error::Retry)
            .raise_erased(),
    ] {
        assert!(
            err.can_retry(),
            "verification can retry after interruption or concurrent changes"
        );
        assert!(err.is_retryable(), "custom errors explicitly advertise retryability");
        assert!(err.into_error().can_retry(), "conversion retains retryability");
    }
}
