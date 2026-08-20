/// The error when looking up a value, for example via [`File::try_value()`][crate::File::try_value()].
#[derive(Debug)]
#[expect(missing_docs)]
pub enum Error<E> {
    ValueMissing(gix_error::Error),
    FailedConversion(E),
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

impl<E> From<existing::Error> for Error<E> {
    fn from(err: existing::Error) -> Self {
        Error::ValueMissing(err.into_error())
    }
}

///
pub mod existing {
    /// The error when looking up a value that doesn't exist, for example via [`File::value()`][crate::File::value()].
    pub type Error = gix_error::Exn;

    pub(crate) fn section_missing() -> Error {
        not_found("The requested section does not exist")
    }

    pub(crate) fn subsection_missing() -> Error {
        not_found("The requested subsection does not exist")
    }

    pub(crate) fn key_missing() -> Error {
        not_found("The key does not exist in the requested section")
    }

    fn not_found(message: &'static str) -> Error {
        use gix_error::ErrorExt;
        gix_error::NotFoundError::new(message).raise_erased()
    }
}
