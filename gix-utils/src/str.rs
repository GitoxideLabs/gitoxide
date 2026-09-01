use std::{borrow::Cow, ffi::OsStr, path::Path};

/// Assure that `s` is precomposed, i.e. `ä` is a single code-point, and not two i.e. `a` and `<umlaut>`.
///
/// At the expense of extra-compute, it does nothing if there is no work to be done, returning the original input without allocating.
pub fn precompose(s: Cow<'_, str>) -> Cow<'_, str> {
    use unicode_normalization::{char, is_nfc};
    if is_nfc(s.as_ref()) {
        return s;
    }

    /// Compose filesystem-decomposed characters without the canonical reordering that full NFC performs.
    /// Non-composable combining marks must retain their byte order to keep matching index entries.
    ///
    /// * `out` holds the characters emitted so far; composition replaces its starter, otherwise `ch` is appended.
    /// * `starter` is the index of the latest class-zero character eligible for composition, if one exists.
    /// * `max_class` is the highest combining class appended since the starter, used to block invalid composition.
    /// * `ch` is the next canonically decomposed character to process.
    ///
    /// Returns `true` if `ch` was composed into the starter, or `false` if it was appended unchanged.
    fn push(out: &mut Vec<char>, starter: &mut Option<usize>, max_class: &mut u8, ch: char) -> bool {
        let class = char::canonical_combining_class(ch);
        if let Some(starter) = *starter
            && (*max_class == 0 || *max_class < class)
            && let Some(composed) = char::compose(out[starter], ch)
        {
            out[starter] = composed;
            return true;
        }
        if class == 0 {
            *starter = Some(out.len());
            *max_class = 0;
        } else {
            *max_class = (*max_class).max(class);
        }
        out.push(ch);
        false
    }

    let mut out = Vec::with_capacity(s.chars().count());
    let mut starter = None;
    let mut max_class = 0;
    let mut changed = false;
    for ch in s.chars() {
        let mut first = true;
        char::decompose_canonical(ch, |decomposed| {
            changed |= !first || decomposed != ch;
            first = false;
            changed |= push(&mut out, &mut starter, &mut max_class, decomposed);
        });
    }
    if changed {
        Cow::Owned(out.into_iter().collect())
    } else {
        s
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
