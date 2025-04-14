use bstr::BStr;
use bstr::BString;
use bstr::ByteSlice;
use gix_validate::path::component::Options;
use std::borrow::Cow;

use crate::os_str_into_bstr;
use crate::try_from_bstr;
use crate::try_from_byte_slice;

/// A wrapper for `BStr`. It is used to enforce the following constraints:
///
/// - The path separator always is `/`, independent of the platform.
/// - Only normal components are allowed.
/// - It is always represented as a bunch of bytes.
#[derive()]
pub struct RelativePath {
    inner: BStr,
}

impl RelativePath {
    fn new_unchecked(value: &BStr) -> Result<&RelativePath, Error> {
        // SAFETY: `RelativePath` is transparent and equivalent to a `&BStr` if provided as reference.
        #[allow(unsafe_code)]
        unsafe {
            std::mem::transmute(value)
        }
    }

    /// TODO
    /// Needs docs.
    pub fn ends_with(&self, needle: &[u8]) -> bool {
        self.inner.ends_with(needle)
    }
}

/// The error used in [`RelativePath`](RelativePath).
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    ContainsInvalidComponent(#[from] gix_validate::path::component::Error),
    #[error(transparent)]
    IllegalUtf8(#[from] crate::Utf8Error),
}

impl<'a> TryFrom<&'a BStr> for &'a RelativePath {
    type Error = Error;

    fn try_from(value: &'a BStr) -> Result<Self, Self::Error> {
        let path: &std::path::Path = &try_from_bstr(value)?;
        let options: Options = Default::default();

        for component in path.components() {
            let component = os_str_into_bstr(component.as_os_str())?;

            gix_validate::path::component(component, None, options)?;
        }

        RelativePath::new_unchecked(value)
    }
}

impl<'a, const N: usize> TryFrom<&'a [u8; N]> for &'a RelativePath {
    type Error = Error;

    #[inline]
    fn try_from(value: &'a [u8; N]) -> Result<Self, Self::Error> {
        let path: &std::path::Path = try_from_byte_slice(value)?;

        let options: Options = Default::default();

        for component in path.components() {
            let component = os_str_into_bstr(component.as_os_str())?;

            gix_validate::path::component(component, None, options)?;
        }

        RelativePath::new_unchecked(value.into())
    }
}

impl<'a> TryFrom<&'a BString> for &'a RelativePath {
    type Error = Error;

    fn try_from(value: &'a BString) -> Result<Self, Self::Error> {
        let path: &std::path::Path = &try_from_bstr(value.as_bstr())?;

        let options: Options = Default::default();

        for component in path.components() {
            let component = os_str_into_bstr(component.as_os_str())?;

            gix_validate::path::component(component, None, options)?;
        }

        RelativePath::new_unchecked(value.as_bstr())
    }
}

/// This is required by a trait bound on [`from_str`](crate::from_bstr).
impl<'a> From<&'a RelativePath> for Cow<'a, BStr> {
    #[inline]
    fn from(value: &'a RelativePath) -> Cow<'a, BStr> {
        Cow::Borrowed(&value.inner)
    }
}

impl AsRef<[u8]> for RelativePath {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.inner.as_bytes()
    }
}
