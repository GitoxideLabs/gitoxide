use std::{borrow::Cow, fmt::Display};

use bstr::BStr;

use crate::parse::{Event, Section};

///
pub mod header;

pub(crate) mod unvalidated;

/// A parsed section header, containing a name and optionally a subsection name.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Header<'a> {
    /// The name of the header.
    pub(crate) name: Name<'a>,
    /// The separator used to determine if the section contains a subsection.
    /// This is either a period `.` or a string of whitespace. Note that
    /// reconstruction of subsection format is dependent on this value. If this
    /// is all whitespace, then the subsection name needs to be surrounded by
    /// quotes to have perfect reconstruction.
    pub(crate) separator: Option<Cow<'a, BStr>>,
    pub(crate) subsection_name: Option<Cow<'a, BStr>>,
}

impl Section<'_> {
    /// Turn this instance into a fully owned one with `'static` lifetime.
    #[must_use]
    pub fn to_owned(&self) -> Section<'static> {
        Section {
            header: self.header.to_owned(),
            events: self.events.iter().map(Event::to_owned).collect(),
        }
    }
}

impl Display for Section<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.header)?;
        for event in &self.events {
            event.fmt(f)?;
        }
        Ok(())
    }
}

mod types {
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
            pub struct $name<'a>(pub(crate) std::borrow::Cow<'a, $cow_inner_type>);

            impl<'a> $name<'a> {
                pub(crate) fn from_str_unchecked(s: &'a str) -> Self {
                    $name(std::borrow::Cow::Borrowed(s.into()))
                }
                /// Turn this instance into a fully owned one with `'static` lifetime.
                #[must_use]
                pub fn to_owned(&self) -> $name<'static> {
                    $name(std::borrow::Cow::Owned(self.0.clone().into_owned()))
                }
            }

            impl PartialEq for $name<'_> {
                fn eq(&self, other: &Self) -> bool {
                    self.0.eq_ignore_ascii_case(&other.0)
                }
            }

            impl std::fmt::Display for $name<'_> {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    self.0.fmt(f)
                }
            }

            impl PartialOrd for $name<'_> {
                fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                    Some(self.cmp(other))
                }
            }

            impl Ord for $name<'_> {
                fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                    let a = self.0.iter().map(|c| c.to_ascii_lowercase());
                    let b = other.0.iter().map(|c| c.to_ascii_lowercase());
                    a.cmp(b)
                }
            }

            impl std::hash::Hash for $name<'_> {
                fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                    for b in self.0.iter() {
                        b.to_ascii_lowercase().hash(state);
                    }
                }
            }

            impl<'a> std::convert::TryFrom<&'a str> for $name<'a> {
                type Error = $module::Error;

                fn try_from(s: &'a str) -> Result<Self, Self::Error> {
                    Self::try_from(std::borrow::Cow::Borrowed(bstr::ByteSlice::as_bstr(s.as_bytes())))
                }
            }

            impl<'a> std::convert::TryFrom<String> for $name<'a> {
                type Error = $module::Error;

                fn try_from(s: String) -> Result<Self, Self::Error> {
                    Self::try_from(std::borrow::Cow::Owned(bstr::BString::from(s)))
                }
            }

            impl<'a> std::convert::TryFrom<std::borrow::Cow<'a, bstr::BStr>> for $name<'a> {
                type Error = $module::Error;

                fn try_from(s: std::borrow::Cow<'a, bstr::BStr>) -> Result<Self, Self::Error> {
                    if $validate(s.as_ref()) {
                        Ok(Self(s))
                    } else {
                        Err($module::Error)
                    }
                }
            }

            impl<'a> std::ops::Deref for $name<'a> {
                type Target = $cow_inner_type;

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            impl<'a> std::convert::AsRef<str> for $name<'a> {
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

pub(crate) fn into_cow_bstr(c: Cow<'_, str>) -> Cow<'_, BStr> {
    match c {
        Cow::Borrowed(s) => Cow::Borrowed(s.into()),
        Cow::Owned(s) => Cow::Owned(s.into()),
    }
}
