/// The error when looking up a value, for example via [`File::try_value()`][crate::File::try_value()].
#[derive(Debug)]
#[expect(missing_docs)]
pub enum Error<E> {
    ValueMissing(gix_error::Error),
    FailedConversion(E),
}

impl<E: Into<gix_error::Error>> Error<E> {
    /// Convert this lookup error into a standard error, retaining the inner exception's context.
    pub fn into_error(self) -> gix_error::Error {
        match self {
            Error::ValueMissing(err) => err,
            Error::FailedConversion(err) => err.into(),
        }
    }
}

impl<E: std::fmt::Display> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ValueMissing(err) => std::fmt::Display::fmt(err, f),
            Error::FailedConversion(err) => std::fmt::Display::fmt(err, f),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for Error<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::ValueMissing(err) => Some(err),
            Error::FailedConversion(err) => Some(err),
        }
    }
}

impl<E> From<gix_error::Exn> for Error<E> {
    fn from(err: gix_error::Exn) -> Self {
        Error::ValueMissing(err.into_error())
    }
}

///
pub mod existing {

    pub(crate) fn section_missing() -> gix_error::Exn {
        not_found("The requested section does not exist")
    }

    pub(crate) fn subsection_missing() -> gix_error::Exn {
        not_found("The requested subsection does not exist")
    }

    pub(crate) fn key_missing() -> gix_error::Exn {
        not_found("The key does not exist in the requested section")
    }

    fn not_found(message: &'static str) -> gix_error::Exn {
        use gix_error::ErrorExt;
        gix_error::NotFoundError::new(message).raise_erased()
    }
}
