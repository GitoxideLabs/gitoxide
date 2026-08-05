use crate::parse::{MaybeDecoded, Span};

///
pub mod header;

pub(crate) mod unvalidated;

/// A parsed section header.
#[derive(Clone, Debug)]
pub(crate) struct HeaderData {
    /// The name of the header.
    pub(crate) name: Span,
    /// The separator used to determine if the section contains a subsection.
    /// This is either a period `.` or a string of whitespace. Note that
    /// reconstruction of subsection format is dependent on this value. If this
    /// is all whitespace, then the subsection name needs to be surrounded by
    /// quotes to have perfect reconstruction.
    pub(crate) separator: Option<Span>,
    pub(crate) subsection_name: Option<MaybeDecoded>,
}

mod types {
    use bstr::ByteSlice;

    macro_rules! generate_case_insensitive {
        ($name:ident, $module:ident, $err_doc:literal, $validate:ident, $cow_inner_type:ty, $comment:literal) => {
            ///
            pub mod $module {
                /// The error returned when `TryFrom` is invoked to create an instance.
                #[derive(Debug, Copy, Clone)]
                pub struct Error;

                impl std::fmt::Display for Error {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        f.write_str($err_doc)
                    }
                }

                impl std::error::Error for Error {}
            }

            #[doc = $comment]
            #[derive(Clone, Eq, Debug, Default)]
            pub struct $name(pub(crate) bstr::BString);

            impl $name {
                pub(crate) fn from_str_unchecked(s: &str) -> Self {
                    $name(s.into())
                }
                /// Clone this instance.
                #[must_use]
                pub fn to_owned(&self) -> $name {
                    self.clone()
                }
            }

            impl PartialEq for $name {
                fn eq(&self, other: &Self) -> bool {
                    self.0.eq_ignore_ascii_case(&other.0)
                }
            }

            impl std::fmt::Display for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    std::fmt::Display::fmt(&self.0, f)
                }
            }

            impl PartialOrd for $name {
                fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                    Some(self.cmp(other))
                }
            }

            impl Ord for $name {
                fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                    let a = self.0.iter().map(|c| c.to_ascii_lowercase());
                    let b = other.0.iter().map(|c| c.to_ascii_lowercase());
                    a.cmp(b)
                }
            }

            impl std::hash::Hash for $name {
                fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                    for b in self.0.iter() {
                        b.to_ascii_lowercase().hash(state);
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
                    std::str::from_utf8(self.0.as_slice()).expect("only valid UTF8 makes it through our validation")
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
pub(crate) use types::ValueName;
pub use types::{Name, name, value_name};

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::ValueName;

    fn key(key: &str) -> ValueName {
        ValueName::try_from(key).expect("valid test key")
    }

    #[test]
    fn value_names_reject_invalid_formats() {
        for invalid in ["", "1a", "a.2", "##", "\""] {
            assert!(ValueName::try_from(invalid).is_err(), "{invalid:?} is invalid");
        }
    }

    #[test]
    fn value_names_are_case_insensitive() {
        assert_eq!(key("aB-c"), key("Ab-C"));
        assert_eq!(key("a").cmp(&key("a")), Ordering::Equal);
        assert_eq!(key("aBc").cmp(&key("AbC")), Ordering::Equal);

        fn calculate_hash<T: std::hash::Hash>(value: T) -> u64 {
            use std::hash::Hasher;
            let mut state = std::collections::hash_map::DefaultHasher::new();
            value.hash(&mut state);
            state.finish()
        }
        assert_eq!(calculate_hash(key("aBc")), calculate_hash(key("AbC")));
    }
}
