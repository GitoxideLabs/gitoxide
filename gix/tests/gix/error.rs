use gix_error::{ErrorExt, ExnResult, Result};

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
fn public_traits_preserve_typed_internal_errors() {
    use gix_object::Find;

    fn read_object<'a>() -> ExnResult<Option<gix_object::Data<'a>>, std::io::Error> {
        Err(std::io::Error::from(std::io::ErrorKind::TimedOut).raise_typed())
    }

    struct Objects;
    impl Find for Objects {
        fn try_find<'a>(&self, _id: &gix_hash::oid, _buffer: &'a mut Vec<u8>) -> Result<Option<gix_object::Data<'a>>> {
            read_object().map_err(Into::into)
        }
    }

    assert_eq!(
        read_object().expect_err("the internal read failed").error().kind(),
        std::io::ErrorKind::TimedOut,
        "internal results retain their specific error type"
    );
    let err = Objects
        .try_find(&gix_hash::ObjectId::null(gix_hash::Kind::Sha1), &mut Vec::new())
        .expect_err("the public lookup propagates the read failure");
    assert_eq!(
        err.downcast_any_ref::<std::io::Error>()
            .expect("the concrete I/O error survives the boundary")
            .kind(),
        std::io::ErrorKind::TimedOut,
        "public results preserve concrete I/O errors"
    );
    assert!(err.can_retry(), "conversion preserves retry detection");
    assert!(
        err.iter_errors_with_locations()
            .any(|source| source.location().is_some()),
        "conversion preserves the original caller location"
    );
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
