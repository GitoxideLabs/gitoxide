use bstr::{BStr, ByteSlice};

///
pub mod name {
    /// The error used in [name()](super::name()).
    #[derive(Debug)]
    #[allow(missing_docs)]
    #[non_exhaustive]
    pub enum Error {
        Empty,
        ParentComponent,
        Absolute,
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Empty => write!(f, "Submodule names cannot be empty"),
                Error::ParentComponent => write!(f, "Submodules names must not contains '..'"),
                Error::Absolute => write!(f, "Submodule names must not be absolute paths"),
            }
        }
    }

    impl std::error::Error for Error {}
}

/// Return the original `name` if it is valid, or the respective error indicating what was wrong with it.
pub fn name(name: &BStr) -> Result<&BStr, name::Error> {
    if name.is_empty() {
        return Err(name::Error::Empty);
    }
    if is_absolute(name) {
        return Err(name::Error::Absolute);
    }
    for component in name.as_bytes().split(|b| *b == b'/' || *b == b'\\') {
        if component == b".." {
            return Err(name::Error::ParentComponent);
        }
    }
    Ok(name)
}

/// Return `true` if `name` would re-root a `Path::join`, escaping `.git/modules`.
///
/// A leading separator (also covering `//` and `\\` UNC roots) is absolute on all platforms, and a
/// Windows drive prefix like `C:` re-roots a join on Windows even without a following separator.
fn is_absolute(name: &BStr) -> bool {
    match name.first() {
        Some(b'/' | b'\\') => true,
        Some(drive) if drive.is_ascii_alphabetic() => name.get(1) == Some(&b':'),
        _ => false,
    }
}
