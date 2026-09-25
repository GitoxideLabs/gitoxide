use gix::{
    bstr::ByteSlice,
    config::tree::{Core, Http, keys},
};
use gix_error::MetadataValue;

pub(super) fn assert_config_error(
    error: &gix::Error,
    key: &str,
    input: Option<MetadataValue>,
    environment: Option<&str>,
) {
    assert!(
        error.is_validation(),
        "invalid configuration is classified as validation"
    );
    let metadata = error.metadata().next().expect("a configuration error has metadata");
    assert_eq!(
        metadata.get("key"),
        Some(&MetadataValue::from(key)),
        "the context identifies the key"
    );
    assert_eq!(
        metadata.get("input"),
        input.as_ref(),
        "the context retains the available input"
    );
    assert_eq!(
        metadata.get("environment_override"),
        environment.map(MetadataValue::from).as_ref(),
        "the context identifies the possible environment override"
    );
}

#[test]
fn integer_metadata_and_cause_survive_wrapping() {
    let err: gix::Error = Core::DELTA_BASE_CACHE_LIMIT
        .try_into_usize(Ok(Some(-1)))
        .expect_err("negative values cannot be cache sizes");
    assert!(err.is_validation(), "invalid configuration is classified as validation");
    assert!(
        err.downcast_any_ref::<std::num::TryFromIntError>().is_some(),
        "the integer conversion error remains available"
    );
    let metadata = err
        .metadata()
        .find(|metadata| metadata.contains_key("key"))
        .expect("the configuration context exposes its key as metadata");
    assert_eq!(metadata["key"], MetadataValue::from("core.deltaBaseCacheLimit"));
    assert_eq!(
        metadata["input"],
        MetadataValue::I64(-1),
        "parsed numbers retain their type"
    );
    assert_eq!(
        metadata["environment_override"],
        MetadataValue::from("GIX_PACK_CACHE_MEMORY")
    );
}

#[test]
fn string_metadata_preserves_invalid_bytes_and_cause() {
    let input = b"\xF0\x80\x80".as_bstr();
    let err: gix::Error = Http::USER_AGENT
        .try_into_string(input)
        .expect_err("overlong UTF-8 is rejected");
    assert!(err.is_validation(), "invalid configuration is classified as validation");
    assert!(
        err.downcast_any_ref::<gix::bstr::Utf8Error>().is_some(),
        "the UTF-8 conversion error remains available"
    );
    let metadata = err
        .metadata()
        .find(|metadata| metadata.contains_key("key"))
        .expect("the configuration context exposes its key as metadata");
    assert_eq!(metadata["key"], MetadataValue::from("http.userAgent"));
    assert_eq!(
        metadata["input"],
        MetadataValue::from(input),
        "input bytes are not decoded lossily"
    );
    assert!(
        !metadata.contains_key("environment_override"),
        "keys without an environment override omit it"
    );
}

#[test]
fn boolean_parser_metadata_remains_in_its_own_context() {
    let input = b"bogus".as_bstr();
    let error = Core::BARE
        .enrich_error(gix::config::Boolean::try_from(input).map(|boolean| Some(boolean.0)))
        .expect_err("the value is not a boolean");
    assert_config_error(&error, "core.bare", None, None);
    let source = error
        .metadata()
        .nth(1)
        .expect("the parser retains its own input context");
    assert_eq!(source.get("input"), Some(&MetadataValue::from(input)));
    assert!(!source.contains_key("key"), "contexts are not merged across causes");
}

#[test]
fn date_conversion_retains_key_metadata() {
    let error = keys::Time::new_time("date", &gix::config::tree::Author)
        .with_environment_override("GIT_AUTHOR_DATE")
        .try_into_time("not a date", None)
        .expect_err("the date is invalid");
    assert_config_error(
        &error,
        "author.date",
        Some(b"not a date".as_bstr().into()),
        Some("GIT_AUTHOR_DATE"),
    );
    assert!(
        error.metadata().count() > 1,
        "the date parser's context remains available"
    );
}

#[cfg(any(
    feature = "blocking-http-transport-reqwest",
    feature = "blocking-http-transport-curl"
))]
#[test]
fn http_callback_can_return_a_concrete_cause() {
    use gix_error::ErrorExt;

    let error = Http::FOLLOW_REDIRECTS
        .try_into_follow_redirects("bad", || {
            Err(std::io::Error::from(std::io::ErrorKind::TimedOut).raise().into())
        })
        .expect_err("callback failures are propagated");
    assert_config_error(&error, "http.followRedirects", Some(b"bad".as_bstr().into()), None);
    assert_eq!(
        error
            .downcast_any_ref::<std::io::Error>()
            .expect("the callback's concrete cause is retained")
            .kind(),
        std::io::ErrorKind::TimedOut
    );
}
