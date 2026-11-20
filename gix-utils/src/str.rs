use std::{borrow::Cow, ffi::OsStr, path::Path};

/// Assure that `s` is precomposed, i.e. `ä` is a single code-point, and not two i.e. `a` and `<umlaut>`.
///
/// Strings containing characters outside the Basic Multilingual Plane are left unchanged, matching Git's
/// fallback when macOS's `UTF-8-MAC` conversion rejects them.
///
/// Returns the original input when unchanged.
pub fn precompose(s: Cow<'_, str>) -> Cow<'_, str> {
    precompose_impl::<false>(s)
}

fn precompose_impl<const IS_PATH: bool>(s: Cow<'_, str>) -> Cow<'_, str> {
    use unicode_normalization::{IsNormalized, char, is_nfc_quick};
    if s.is_ascii() {
        return s;
    }
    let mut chars = s.chars();
    let mut non_bmp = false;
    let normalized = is_nfc_quick(chars.by_ref().take_while(|ch| {
        non_bmp = *ch > '\u{ffff}';
        !non_bmp
    }));
    // On a path, stopping at a non-BMP character leaves later components to normalize.
    if normalized == IsNormalized::Yes && (!IS_PATH || !non_bmp) {
        return s;
    }
    // Quick-check can stop before visiting a non-BMP character. Finish the fallback check if needed.
    // Avoid `is_nfc()`: an inconclusive quick-check would normalize the string before we compose it again.
    if non_bmp || chars.any(|ch| ch > '\u{ffff}') {
        if !IS_PATH {
            return s;
        }
        // Split only when conversion can fail: ASCII separators already keep BMP compositions independent.
        let mut out: Option<String> = None;
        let mut offset = 0;
        #[cfg(windows)]
        let components = s.split_inclusive(['/', '\\']);
        #[cfg(not(windows))]
        let components = s.split_inclusive('/');
        for component in components {
            match precompose(component.into()) {
                Cow::Borrowed(component) => {
                    if let Some(out) = &mut out {
                        out.push_str(component);
                    }
                }
                Cow::Owned(component) => out
                    .get_or_insert_with(|| {
                        let mut out = String::with_capacity(s.len());
                        out.push_str(&s[..offset]);
                        out
                    })
                    .push_str(&component),
            }
            offset += component.len();
        }
        return out.map_or(s, Cow::Owned);
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
        let class = if ch.is_ascii() {
            0
        } else {
            char::canonical_combining_class(ch)
        };
        // ASCII always starts a new sequence and cannot be the second character in a composition.
        if !ch.is_ascii()
            && let Some(starter) = *starter
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

    // The byte length is a cheap capacity estimate and avoids another character-counting pass.
    let mut out = Vec::with_capacity(s.len());
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
    if changed && !out.iter().copied().eq(s.chars()) {
        let mut precomposed = String::with_capacity(s.len());
        precomposed.extend(out);
        Cow::Owned(precomposed)
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
/// Apply [`precompose()`] to each component independently, preserving the path's spelling otherwise.
/// Thus, a non-BMP character in one filename does not prevent composing unrelated components.
pub fn precompose_path(path: Cow<'_, Path>) -> Cow<'_, Path> {
    if path.as_os_str().as_encoded_bytes().is_ascii() {
        return path;
    }
    let Some(input) = path.to_str() else {
        return path;
    };
    match precompose_impl::<true>(input.into()) {
        Cow::Borrowed(_) => path,
        Cow::Owned(precomposed) => Cow::Owned(precomposed.into()),
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

/// Replace `output` with `input` normalized like `git stripspace`.
///
/// Trim Git whitespace from line endings, keep at most one empty line between paragraphs,
/// and terminate non-empty output with a newline. Other bytes are preserved. When provided,
/// `comment_prefix` removes lines starting with that byte sequence before normalization.
pub fn stripspace(input: &[u8], comment_prefix: Option<&[u8]>, output: &mut Vec<u8>) {
    output.clear();
    let mut blank_line = false;
    for line in input.split(|byte| *byte == b'\n') {
        if comment_prefix.is_some_and(|prefix| line.starts_with(prefix)) {
            continue;
        }
        let Some(last) = line.iter().rposition(|byte| !crate::git_is_space(*byte)) else {
            blank_line = !output.is_empty();
            continue;
        };
        if blank_line {
            output.push(b'\n');
            blank_line = false;
        }
        output.extend_from_slice(&line[..=last]);
        output.push(b'\n');
    }
}

/// Replace `output` with `input` commented like `git stripspace --comment-lines`.
///
/// Prefix every line with `comment_prefix`, followed by a space unless the line is empty or
/// starts with a tab. Preserve all input bytes and terminate non-empty output with a newline.
pub fn comment_lines(input: &[u8], comment_prefix: &[u8], output: &mut Vec<u8>) {
    output.clear();
    for line in input.split_inclusive(|byte| *byte == b'\n') {
        output.extend_from_slice(comment_prefix);
        if !matches!(line.first(), Some(b'\n' | b'\t')) {
            output.push(b' ');
        }
        output.extend_from_slice(line);
        if !line.ends_with(b"\n") {
            output.push(b'\n');
        }
    }
}
