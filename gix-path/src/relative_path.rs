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

/// The error used in [`RelativePath`].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("A RelativePath is not allowed to be absolute")]
    IsAbsolute,
    #[error(transparent)]
    ContainsInvalidComponent(#[from] gix_validate::path::component::Error),
    #[error(transparent)]
    IllegalUtf8(#[from] crate::Utf8Error),
}

impl<'a> TryFrom<&'a str> for &'a RelativePath {
    type Error = Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        use std::path::Path;

        let path: &std::path::Path = Path::new(value);

        if path.is_absolute() {
            return Err(Error::IsAbsolute);
        }

        let options: Options = Default::default();

        for component in path.components() {
            let component = os_str_into_bstr(component.as_os_str())?;

            gix_validate::path::component(component, None, options)?;
        }

        RelativePath::new_unchecked(BStr::new(value.as_bytes()))
    }
}

impl<'a> TryFrom<&'a BStr> for &'a RelativePath {
    type Error = Error;

    fn try_from(value: &'a BStr) -> Result<Self, Self::Error> {
        let path: &std::path::Path = &try_from_bstr(value)?;

        if path.is_absolute() {
            return Err(Error::IsAbsolute);
        }

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

        if path.is_absolute() {
            return Err(Error::IsAbsolute);
        }

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

        if path.is_absolute() {
            return Err(Error::IsAbsolute);
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(windows))]
    #[test]
    fn absolute_paths_return_err() {
        let path_str: &str = "/refs/heads";
        let path_bstr: &BStr = path_str.into();
        let path_u8: &[u8; 11] = b"/refs/heads";
        let path_bstring: BString = "/refs/heads".into();

        assert!(matches!(
            TryInto::<&RelativePath>::try_into(path_str),
            Err(Error::IsAbsolute)
        ));
        assert!(matches!(
            TryInto::<&RelativePath>::try_into(path_bstr),
            Err(Error::IsAbsolute)
        ));
        assert!(matches!(
            TryInto::<&RelativePath>::try_into(path_u8),
            Err(Error::IsAbsolute)
        ));
        assert!(matches!(
            TryInto::<&RelativePath>::try_into(&path_bstring),
            Err(Error::IsAbsolute)
        ));
    }

    #[cfg(windows)]
    #[test]
    fn absolute_paths_return_err() {
        let path_str: &str = r"c:\refs\heads";
        let path_bstr: &BStr = path_str.into();
        let path_u8: &[u8; 13] = b"c:\\refs\\heads";
        let path_bstring: BString = r"c:\refs\heads".into();

        assert!(matches!(
            TryInto::<&RelativePath>::try_into(path_str),
            Err(Error::IsAbsolute)
        ));
        assert!(matches!(
            TryInto::<&RelativePath>::try_into(path_bstr),
            Err(Error::IsAbsolute)
        ));
        assert!(matches!(
            TryInto::<&RelativePath>::try_into(path_u8),
            Err(Error::IsAbsolute)
        ));
        assert!(matches!(
            TryInto::<&RelativePath>::try_into(&path_bstring),
            Err(Error::IsAbsolute)
        ));
    }

    #[cfg(not(windows))]
    #[test]
    fn dots_in_paths_return_err() {
        let path_str: &str = "./heads";
        let path_bstr: &BStr = path_str.into();
        let path_u8: &[u8; 7] = b"./heads";
        let path_bstring: BString = "./heads".into();

        assert!(matches!(
            TryInto::<&RelativePath>::try_into(path_str),
            Err(Error::ContainsInvalidComponent(_))
        ));
        assert!(matches!(
            TryInto::<&RelativePath>::try_into(path_bstr),
            Err(Error::ContainsInvalidComponent(_))
        ));
        assert!(matches!(
            TryInto::<&RelativePath>::try_into(path_u8),
            Err(Error::ContainsInvalidComponent(_))
        ));
        assert!(matches!(
            TryInto::<&RelativePath>::try_into(&path_bstring),
            Err(Error::ContainsInvalidComponent(_))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn dots_in_paths_return_err() {
        let path_str: &str = r".\heads";
        let path_bstr: &BStr = path_str.into();
        let path_u8: &[u8; 7] = b".\\heads";
        let path_bstring: BString = r".\heads".into();

        assert!(matches!(
            TryInto::<&RelativePath>::try_into(path_str),
            Err(Error::ContainsInvalidComponent(_))
        ));
        assert!(matches!(
            TryInto::<&RelativePath>::try_into(path_bstr),
            Err(Error::ContainsInvalidComponent(_))
        ));
        assert!(matches!(
            TryInto::<&RelativePath>::try_into(path_u8),
            Err(Error::ContainsInvalidComponent(_))
        ));
        assert!(matches!(
            TryInto::<&RelativePath>::try_into(&path_bstring),
            Err(Error::ContainsInvalidComponent(_))
        ));
    }

    #[cfg(not(windows))]
    #[test]
    fn double_dots_in_paths_return_err() {
        let path_str: &str = "../heads";
        let path_bstr: &BStr = path_str.into();
        let path_u8: &[u8; 8] = b"../heads";
        let path_bstring: BString = "../heads".into();

        assert!(matches!(
            TryInto::<&RelativePath>::try_into(path_str),
            Err(Error::ContainsInvalidComponent(_))
        ));
        assert!(matches!(
            TryInto::<&RelativePath>::try_into(path_bstr),
            Err(Error::ContainsInvalidComponent(_))
        ));
        assert!(matches!(
            TryInto::<&RelativePath>::try_into(path_u8),
            Err(Error::ContainsInvalidComponent(_))
        ));
        assert!(matches!(
            TryInto::<&RelativePath>::try_into(&path_bstring),
            Err(Error::ContainsInvalidComponent(_))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn double_dots_in_paths_return_err() {
        let path_str: &str = r"..\heads";
        let path_bstr: &BStr = path_str.into();
        let path_u8: &[u8; 8] = b"..\\heads";
        let path_bstring: BString = r"..\heads".into();

        assert!(matches!(
            TryInto::<&RelativePath>::try_into(path_str),
            Err(Error::ContainsInvalidComponent(_))
        ));
        assert!(matches!(
            TryInto::<&RelativePath>::try_into(path_bstr),
            Err(Error::ContainsInvalidComponent(_))
        ));
        assert!(matches!(
            TryInto::<&RelativePath>::try_into(path_u8),
            Err(Error::ContainsInvalidComponent(_))
        ));
        assert!(matches!(
            TryInto::<&RelativePath>::try_into(&path_bstring),
            Err(Error::ContainsInvalidComponent(_))
        ));
    }
}
