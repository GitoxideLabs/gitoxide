use std::path::Path;

use bstr::{BStr, BString, ByteSlice};
use gix_error::{ErrorExt, ResultExt, ValidationError};
use gix_validate::path::component::Options;

use crate::{os_str_into_bstr, try_from_bstr, try_from_byte_slice};

pub(super) mod types {
    use bstr::{BStr, ByteSlice};
    /// A wrapper for `BStr`. It is used to enforce the following constraints:
    ///
    /// - The path separator always is `/`, independent of the platform.
    /// - Only normal components are allowed.
    /// - It is always represented as a bunch of bytes.
    #[repr(transparent)]
    pub struct RelativePath {
        inner: BStr,
    }

    impl AsRef<[u8]> for RelativePath {
        #[inline]
        fn as_ref(&self) -> &[u8] {
            self.inner.as_bytes()
        }
    }
}
use types::RelativePath;

impl RelativePath {
    fn new_unchecked(value: &BStr) -> Result<&RelativePath, Error> {
        // SAFETY: `RelativePath` is transparent and equivalent to a `&BStr` if provided as reference.
        #[expect(unsafe_code)]
        unsafe {
            Ok(std::mem::transmute::<&BStr, &RelativePath>(value))
        }
    }
}

/// The error used in [`RelativePath`].
pub type Error = gix_error::Exn<ValidationError>;

fn relative_path_from_value_and_path<'a>(path_bstr: &'a BStr, path: &Path) -> Result<&'a RelativePath, Error> {
    if path.is_absolute() {
        return Err(ValidationError::new("A RelativePath is not allowed to be absolute").raise());
    }

    let options = Options::default();

    for component in path.components() {
        let component = os_str_into_bstr(component.as_os_str())?;
        gix_validate::path::component(component, None, options)
            .or_raise(|| ValidationError::new_with_input("Relative path contains an invalid component", component))?;
    }

    RelativePath::new_unchecked(BStr::new(path_bstr.as_bytes()))
}

impl<'a> TryFrom<&'a str> for &'a RelativePath {
    type Error = Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        relative_path_from_value_and_path(value.into(), Path::new(value))
    }
}

impl<'a> TryFrom<&'a BStr> for &'a RelativePath {
    type Error = Error;

    fn try_from(value: &'a BStr) -> Result<Self, Self::Error> {
        let path = try_from_bstr(value)?;
        relative_path_from_value_and_path(value, &path)
    }
}

impl<'a> TryFrom<&'a [u8]> for &'a RelativePath {
    type Error = Error;

    #[inline]
    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        let path = try_from_byte_slice(value)?;
        relative_path_from_value_and_path(value.as_bstr(), path)
    }
}

impl<'a, const N: usize> TryFrom<&'a [u8; N]> for &'a RelativePath {
    type Error = Error;

    #[inline]
    fn try_from(value: &'a [u8; N]) -> Result<Self, Self::Error> {
        let path = try_from_byte_slice(value.as_bstr())?;
        relative_path_from_value_and_path(value.as_bstr(), path)
    }
}

impl<'a> TryFrom<&'a BString> for &'a RelativePath {
    type Error = Error;

    fn try_from(value: &'a BString) -> Result<Self, Self::Error> {
        let path = try_from_bstr(value.as_bstr())?;
        relative_path_from_value_and_path(value.as_bstr(), &path)
    }
}
