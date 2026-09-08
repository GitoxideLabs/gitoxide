use std::{borrow::Cow, ffi::OsString, fmt::Display};

use bstr::{BStr, BString};

use crate::{Boolean, Error, Integer};

fn bool_err(input: impl Into<BString>) -> Error {
    Error::new(
        "Booleans need to be 'no', 'off', 'false', '' or 'yes', 'on', 'true' or any number",
        input,
    )
}

impl TryFrom<OsString> for Boolean {
    type Error = Error;

    fn try_from(value: OsString) -> Result<Self, Self::Error> {
        let value = gix_path::os_str_into_bstr(&value)
            .map_err(|_| Error::new("Illformed UTF-8", std::path::Path::new(&value).display().to_string()))?;
        Self::try_from(value)
    }
}

/// # Warning
///
/// The direct usage of `try_from("string")` is discouraged as it will produce the wrong result for values
/// obtained from `core.bool-implicit-true`, which have no separator and are implicitly true.
/// This method chooses to work correctly for `core.bool-empty=`, which is an empty string and resolves
/// to being `false`.
///
/// Instead of this, obtain booleans with `config.boolean(…)`, which handles the case were no separator is
/// present correctly.
impl TryFrom<&BStr> for Boolean {
    type Error = Error;

    fn try_from(value: &BStr) -> Result<Self, Self::Error> {
        if parse_true(value) {
            Ok(Boolean(true))
        } else if parse_false(value) {
            Ok(Boolean(false))
        } else if let Some(integer) = parse_as_git_int(value) {
            Ok(Boolean(integer != 0))
        } else {
            Err(bool_err(value))
        }
    }
}

impl TryFrom<&str> for Boolean {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(BStr::new(value))
    }
}

impl Boolean {
    /// Return true if the boolean is a true value.
    ///
    /// Note that the inner value is accessible directly as well.
    pub fn is_true(self) -> bool {
        self.0
    }
}

impl TryFrom<Cow<'_, BStr>> for Boolean {
    type Error = Error;
    fn try_from(c: Cow<'_, BStr>) -> Result<Self, Self::Error> {
        Self::try_from(c.as_ref())
    }
}

impl TryFrom<BString> for Boolean {
    type Error = Error;
    fn try_from(value: BString) -> Result<Self, Self::Error> {
        Self::try_from(BStr::new(&value))
    }
}

impl Display for Boolean {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<Boolean> for bool {
    fn from(b: Boolean) -> Self {
        b.0
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Boolean {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bool(self.0)
    }
}

/// Parse the numeric fallback the way `git_parse_maybe_bool_text()` does, which hands
/// the value to `git_parse_int()`: the same bases and `k`/`m`/`g` suffixes that
/// [`Integer`] accepts, but bounded to a C `int` rather than to 64 bits.
///
/// So `git config --type=bool` reads `0x0` as false and `1k` as true, and refuses
/// `2g` and `08`.
///
/// The lower bound is `i32::MIN` as of git 2.50, which changed `-max / factor` to
/// `(-max - 1) / factor` in `git_parse_signed()`; before that `-2147483648` was out
/// of range. That value already parsed here in its decimal spelling, so it is kept.
fn parse_as_git_int(value: &BStr) -> Option<i32> {
    Integer::try_from(value).ok()?.to_decimal()?.try_into().ok()
}

fn parse_true(value: &BStr) -> bool {
    value.eq_ignore_ascii_case(b"yes") || value.eq_ignore_ascii_case(b"on") || value.eq_ignore_ascii_case(b"true")
}

fn parse_false(value: &BStr) -> bool {
    value.eq_ignore_ascii_case(b"no")
        || value.eq_ignore_ascii_case(b"off")
        || value.eq_ignore_ascii_case(b"false")
        || value.is_empty()
}
