/// The error when looking up a value, for example via [`File::try_value()`][crate::File::try_value()].
#[derive(Debug)]
#[expect(missing_docs)]
pub enum Error<E> {
    ValueMissing(existing::Error),
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
            Error::ValueMissing(err) => err.source(),
            Error::FailedConversion(err) => err.source(),
        }
    }
}

impl<E> From<existing::Error> for Error<E> {
    fn from(err: existing::Error) -> Self {
        Error::ValueMissing(err)
    }
}

///
pub mod existing {
    /// The error when looking up a value that doesn't exist, for example via [`File::value()`][crate::File::value()].
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        SectionMissing,
        SubSectionMissing,
        KeyMissing,
        ValueName(crate::parse::section::value_name::Error),
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::SectionMissing => f.write_str("The requested section does not exist"),
                Error::SubSectionMissing => f.write_str("The requested subsection does not exist"),
                Error::KeyMissing => f.write_str("The key does not exist in the requested section"),
                Error::ValueName(err) => std::fmt::Display::fmt(err, f),
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::ValueName(err) => err.source(),
                _ => None,
            }
        }
    }

    impl From<crate::parse::section::value_name::Error> for Error {
        fn from(err: crate::parse::section::value_name::Error) -> Self {
            Error::ValueName(err)
        }
    }
}
