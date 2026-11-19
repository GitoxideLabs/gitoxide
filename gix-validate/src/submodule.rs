use bstr::{BStr, ByteSlice};

///
pub mod name {
    /// The error used in [name()](super::name()).
    #[derive(Debug)]
    #[expect(missing_docs)]
    #[non_exhaustive]
    pub enum Error {
        Empty,
        NulByte,
        ParentComponent,
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Empty => write!(f, "Submodule names cannot be empty"),
                Error::NulByte => write!(f, "Submodule names cannot contain NUL bytes"),
                Error::ParentComponent => write!(f, "Submodules names must not contains '..'"),
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(const { &gix_error::ClassificationMarker::VALIDATION })
        }
    }
}

/// Return the original `name` if it is valid, or the respective error indicating what was wrong with it.
/// Empty names, NUL bytes, and parent path components are rejected.
pub fn name(name: &BStr) -> Result<&BStr, name::Error> {
    if name.is_empty() {
        return Err(name::Error::Empty);
    }
    if name.find_byte(0).is_some() {
        return Err(name::Error::NulByte);
    }
    for component in name.as_bytes().split(|b| *b == b'/' || *b == b'\\') {
        if component == b".." {
            return Err(name::Error::ParentComponent);
        }
    }
    Ok(name)
}
