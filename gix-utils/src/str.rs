use std::{borrow::Cow, ffi::OsStr, path::Path};

/// Assure that `s` is precomposed, i.e. `ä` is a single code-point, and not two i.e. `a` and `<umlaut>`.
///
/// At the expense of extra-compute, it does nothing if there is no work to be done, returning the original input without allocating.
pub fn precompose(s: Cow<'_, str>) -> Cow<'_, str> {
    use unicode_normalization::{UnicodeNormalization, is_nfc};
    if is_nfc(s.as_ref()) {
        s
    } else {
        Cow::Owned(s.as_ref().nfc().collect())
    }
}

/// Assure that `s` is decomposed, i.e. `ä` turns into `a` and `<umlaut>`.
///
/// At the expense of extra-compute, it does nothing if there is no work to be done, returning the original input without allocating.
pub fn decompose(s: Cow<'_, str>) -> Cow<'_, str> {
    use unicode_normalization::{UnicodeNormalization, is_nfd};
    if is_nfd(s.as_ref()) {
        s
    } else {
        Cow::Owned(s.as_ref().nfd().collect())
    }
}

/// Return the precomposed version of `path`, or `path` itself if it contained illformed unicode,
/// or if the unicode version didn't contains decomposed unicode.
/// Otherwise, similar to [`precompose()`]
pub fn precompose_path(path: Cow<'_, Path>) -> Cow<'_, Path> {
    match path.to_str() {
        None => path,
        Some(maybe_decomposed) => match precompose(maybe_decomposed.into()) {
            Cow::Borrowed(_) => path,
            Cow::Owned(precomposed) => Cow::Owned(precomposed.into()),
        },
    }
}

/// Return the precomposed version of `name`, or `name` itself if it contained illformed unicode,
/// or if the unicode version didn't contains decomposed unicode.
/// Otherwise, similar to [`precompose()`]
pub fn precompose_os_string(name: Cow<'_, OsStr>) -> Cow<'_, OsStr> {
    match name.to_str() {
        None => name,
        Some(maybe_decomposed) => match precompose(maybe_decomposed.into()) {
            Cow::Borrowed(_) => name,
            Cow::Owned(precomposed) => Cow::Owned(precomposed.into()),
        },
    }
}

/// Return the precomposed version of `s`, or `s` itself if it contained illformed unicode,
/// or if the unicode version didn't contains decomposed unicode.
/// Otherwise, similar to [`precompose()`]
#[cfg(feature = "bstr")]
pub fn precompose_bstr(s: Cow<'_, bstr::BStr>) -> Cow<'_, bstr::BStr> {
    use bstr::ByteSlice;
    match s.to_str().ok() {
        None => s,
        Some(maybe_decomposed) => match precompose(maybe_decomposed.into()) {
            Cow::Borrowed(_) => s,
            Cow::Owned(precomposed) => Cow::Owned(precomposed.into()),
        },
    }
}
