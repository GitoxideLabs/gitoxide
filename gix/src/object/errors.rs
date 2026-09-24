pub(crate) fn existing_error(err: gix_error::Exn) -> gix_error::Error {
    err.into_error()
}
