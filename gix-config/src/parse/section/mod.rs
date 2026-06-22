use std::fmt::Display;

use crate::parse::{SectionData, Span};

///
pub mod header;

pub(crate) mod unvalidated;

/// A parsed section header, containing a name and optionally a subsection name.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Header {
    /// The name of the header.
    pub(crate) name: Name,
    /// The separator used to determine if the section contains a subsection.
    /// This is either a period `.` or a string of whitespace. Note that
    /// reconstruction of subsection format is dependent on this value. If this
    /// is all whitespace, then the subsection name needs to be surrounded by
    /// quotes to have perfect reconstruction.
    pub(crate) separator: Option<Span>,
    pub(crate) subsection_name: Option<Span>,
}

impl Display for SectionData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.header)?;
        for event in &self.events {
            event.fmt(f)?;
        }
        Ok(())
    }
}

mod types {
    use bstr::ByteSlice;

    macro_rules! generate_case_insensitive {
        ($name:ident, $module:ident, $err_doc:literal, $validate:ident, $cow_inner_type:ty, $comment:literal) => {
            ///
            pub mod $module {
                /// The error returned when `TryFrom` is invoked to create an instance.
                #[derive(Debug, thiserror::Error, Copy, Clone)]
                #[error($err_doc)]
                pub struct Error;
            }

            #[doc = $comment]
            #[derive(Clone, Eq, Debug, Default)]
            pub struct $name(pub(crate) crate::parse::Span);

            impl $name {
                pub(crate) fn from_str_unchecked(s: &str) -> Self {
                    $name(s.into())
                }
                #[allow(dead_code)]
                pub(crate) fn as_bstr_in<'a>(&'a self, backing: &'a [u8]) -> &'a bstr::BStr {
                    self.0.as_bstr_in(backing)
                }
                #[allow(dead_code)]
                pub(crate) fn eq_ignore_ascii_case_in(&self, backing: &[u8], other: &Self) -> bool {
                    self.as_bstr_in(backing).eq_ignore_ascii_case(other.0.as_bstr())
                }
                /// Clone this instance.
                #[must_use]
                pub fn to_owned(&self) -> $name {
                    self.clone()
                }
            }

            impl PartialEq for $name {
                fn eq(&self, other: &Self) -> bool {
                    match (&self.0, &other.0) {
                        (crate::parse::Span::Owned(lhs), crate::parse::Span::Owned(rhs)) => {
                            lhs.eq_ignore_ascii_case(rhs)
                        }
                        _ => self.0 == other.0,
                    }
                }
            }

            impl std::fmt::Display for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    self.0.fmt(f)
                }
            }

            impl PartialOrd for $name {
                fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                    Some(self.cmp(other))
                }
            }

            impl Ord for $name {
                fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                    match (&self.0, &other.0) {
                        (crate::parse::Span::Owned(lhs), crate::parse::Span::Owned(rhs)) => {
                            let a = lhs.iter().map(|c| c.to_ascii_lowercase());
                            let b = rhs.iter().map(|c| c.to_ascii_lowercase());
                            a.cmp(b)
                        }
                        _ => self.0.cmp(&other.0),
                    }
                }
            }

            impl std::hash::Hash for $name {
                fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                    match &self.0 {
                        crate::parse::Span::Owned(bytes) => {
                            for b in bytes.iter() {
                                b.to_ascii_lowercase().hash(state);
                            }
                        }
                        _ => self.0.hash(state),
                    }
                }
            }

            impl std::convert::TryFrom<&str> for $name {
                type Error = $module::Error;

                fn try_from(s: &str) -> Result<Self, Self::Error> {
                    Self::try_from(bstr::ByteSlice::as_bstr(s.as_bytes()))
                }
            }

            impl std::convert::TryFrom<String> for $name {
                type Error = $module::Error;

                fn try_from(s: String) -> Result<Self, Self::Error> {
                    Self::try_from(bstr::BString::from(s))
                }
            }

            impl std::convert::TryFrom<bstr::BString> for $name {
                type Error = $module::Error;

                fn try_from(s: bstr::BString) -> Result<Self, Self::Error> {
                    if $validate(s.as_slice().as_bstr()) {
                        Ok(Self(s.into()))
                    } else {
                        Err($module::Error)
                    }
                }
            }

            impl std::convert::TryFrom<&bstr::BStr> for $name {
                type Error = $module::Error;

                fn try_from(s: &bstr::BStr) -> Result<Self, Self::Error> {
                    if $validate(s) {
                        Ok(Self(s.into()))
                    } else {
                        Err($module::Error)
                    }
                }
            }

            impl std::ops::Deref for $name {
                type Target = $cow_inner_type;

                fn deref(&self) -> &Self::Target {
                    self.0.as_bstr()
                }
            }

            impl std::convert::AsRef<str> for $name {
                fn as_ref(&self) -> &str {
                    std::str::from_utf8(self.0.as_ref()).expect("only valid UTF8 makes it through our validation")
                }
            }
        };
    }

    fn is_valid_name(n: &bstr::BStr) -> bool {
        !n.is_empty() && n.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'-')
    }
    fn is_valid_value_name(n: &bstr::BStr) -> bool {
        is_valid_name(n) && n[0].is_ascii_alphabetic()
    }

    generate_case_insensitive!(
        Name,
        name,
        "Valid names consist of alphanumeric characters or dashes.",
        is_valid_name,
        bstr::BStr,
        "Wrapper struct for section header names, like `remote`, since these are case-insensitive."
    );

    generate_case_insensitive!(
        ValueName,
        value_name,
        "Valid value names consist of alphanumeric characters or dashes, starting with an alphabetic character.",
        is_valid_value_name,
        bstr::BStr,
        "Wrapper struct for value names, like `path` in `include.path`, since keys are case-insensitive."
    );
}
pub use types::{Name, ValueName, name, value_name};
