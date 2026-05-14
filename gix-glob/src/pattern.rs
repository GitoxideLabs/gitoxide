use std::fmt;

use bitflags::bitflags;
use bstr::{BStr, ByteSlice};

use crate::{Pattern, pattern, wildmatch};

bitflags! {
    /// Information about a [`Pattern`].
    ///
    /// Its main purpose is to accelerate pattern matching, or to negate the match result or to
    /// keep special rules only applicable when matching paths.
    ///
    /// The mode is typically created when parsing the pattern by inspecting it and isn't typically handled by the user.
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Ord, PartialOrd)]
    pub struct Mode: u32 {
        /// The pattern does not contain a sub-directory and - it doesn't contain slashes after removing the trailing one.
        const NO_SUB_DIR = 1 << 0;
        /// A pattern that is '*literal', meaning that it ends with what's given here
        const ENDS_WITH = 1 << 1;
        /// The pattern must match a directory, and not a file.
        const MUST_BE_DIR = 1 << 2;
        /// The pattern matches, but should be negated. Note that this mode has to be checked and applied by the caller.
        const NEGATIVE = 1 << 3;
        /// The pattern starts with a slash and thus matches only from the beginning.
        const ABSOLUTE = 1 << 4;
    }
}

/// Describes whether to match a path case sensitively or not.
///
/// Used in [`Pattern::matches_repo_relative_path()`].
#[derive(Default, Debug, PartialOrd, PartialEq, Copy, Clone, Hash, Ord, Eq)]
pub enum Case {
    /// The case affects the match
    #[default]
    Sensitive,
    /// Ignore the case of ascii characters.
    Fold,
}

/// Instantiation
impl Pattern {
    /// Parse the given `text` as pattern, or return `None` if `text` was empty.
    pub fn from_bytes(text: &[u8]) -> Option<Self> {
        crate::parse::pattern(text, true).map(|(text, mode, first_wildcard_pos)| Pattern {
            text: text.into(),
            mode,
            first_wildcard_pos,
        })
    }

    /// Parse the given `text` as pattern without supporting leading `!` or `\\!` , or return `None` if `text` was empty.
    ///
    /// This assures that `text` remains entirely unaltered, but removes built-in support for negation as well.
    pub fn from_bytes_without_negation(text: &[u8]) -> Option<Self> {
        crate::parse::pattern(text, false).map(|(text, mode, first_wildcard_pos)| Pattern {
            text: text.into(),
            mode,
            first_wildcard_pos,
        })
    }
}

/// Access
impl Pattern {
    /// Return true if a match is negated.
    pub fn is_negative(&self) -> bool {
        self.mode.contains(Mode::NEGATIVE)
    }

    /// Match the given `path` which takes slashes (and only slashes) literally, and is relative to the repository root.
    /// Note that `path` is assumed to be relative to the repository.
    ///
    /// We may take various shortcuts which is when `basename_start_pos` and `is_dir` come into play.
    /// `basename_start_pos` is the index at which the `path`'s basename starts.
    ///
    /// `case` folding can be configured as well.
    /// `mode` is used to control how [`crate::wildmatch()`] should operate.
    pub fn matches_repo_relative_path(
        &self,
        path: &BStr,
        basename_start_pos: Option<usize>,
        is_dir: Option<bool>,
        case: Case,
        mode: wildmatch::Mode,
    ) -> bool {
        let is_dir = is_dir.unwrap_or(false);
        if !is_dir && self.mode.contains(pattern::Mode::MUST_BE_DIR) {
            return false;
        }

        let flags = mode
            | match case {
                Case::Fold => wildmatch::Mode::IGNORE_CASE,
                Case::Sensitive => wildmatch::Mode::empty(),
            };
        #[cfg(debug_assertions)]
        {
            if basename_start_pos.is_some() {
                debug_assert_eq!(
                    basename_start_pos,
                    path.rfind_byte(b'/').map(|p| p + 1),
                    "BUG: invalid cached basename_start_pos provided"
                );
            }
        }
        debug_assert!(!path.starts_with(b"/"), "input path must be relative");

        if self.mode.contains(pattern::Mode::NO_SUB_DIR) && !self.mode.contains(pattern::Mode::ABSOLUTE) {
            let basename = &path[basename_start_pos.unwrap_or_default()..];
            self.matches(basename, flags)
        } else {
            self.matches(path, flags)
        }
    }

    /// See if `value` matches this pattern in the given `mode`.
    ///
    /// `mode` can identify `value` as path which won't match the slash character, and can match
    /// strings with cases ignored as well. Note that the case folding performed here is ASCII only.
    ///
    /// Note that this method uses some shortcuts to accelerate simple patterns, but falls back to
    /// [wildmatch()][crate::wildmatch()] if these fail.
    pub fn matches(&self, value: &BStr, mode: wildmatch::Mode) -> bool {
        match self.first_wildcard_pos {
            // "*literal" case, overrides starts-with
            Some(pos)
                if self.mode.contains(pattern::Mode::ENDS_WITH)
                    && (!mode.contains(wildmatch::Mode::NO_MATCH_SLASH_LITERAL) || !value.contains(&b'/')) =>
            {
                let text = &self.text[pos + 1..];
                if mode.contains(wildmatch::Mode::IGNORE_CASE) {
                    value
                        .len()
                        .checked_sub(text.len())
                        .is_some_and(|start| text.eq_ignore_ascii_case(&value[start..]))
                } else {
                    value.ends_with(text.as_ref())
                }
            }
            Some(pos) => {
                if mode.contains(wildmatch::Mode::IGNORE_CASE) {
                    if !value
                        .get(..pos)
                        .is_some_and(|value| value.eq_ignore_ascii_case(&self.text[..pos]))
                    {
                        return false;
                    }
                } else if !value.starts_with(&self.text[..pos]) {
                    return false;
                }
                crate::wildmatch(self.text.as_bstr(), value, mode)
            }
            None => {
                if mode.contains(wildmatch::Mode::IGNORE_CASE) {
                    self.text.eq_ignore_ascii_case(value)
                } else {
                    self.text == value
                }
            }
        }
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.mode.contains(Mode::NEGATIVE) {
            "!".fmt(f)?;
        }
        if self.mode.contains(Mode::ABSOLUTE) {
            "/".fmt(f)?;
        }
        self.text.fmt(f)?;
        if self.mode.contains(Mode::MUST_BE_DIR) {
            "/".fmt(f)?;
        }
        Ok(())
    }
}
