use std::borrow::Borrow;

use bstr::{BStr, ByteSlice};
use gix_features::threading::OwnShared;

use crate::{Name, NameRef};

impl NameRef<'_> {
    /// Turn this ref into its owned counterpart.
    pub fn to_owned(self) -> Name {
        Name(OwnShared::from(self.0))
    }

    /// Return the inner `str`.
    pub fn as_str(&self) -> &str {
        self.0
    }
}

impl AsRef<str> for NameRef<'_> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl<'a> TryFrom<&'a BStr> for NameRef<'a> {
    type Error = Error;

    fn try_from(attr: &'a BStr) -> Result<Self, Self::Error> {
        fn attr_valid(attr: &BStr) -> bool {
            if attr.is_empty() || attr.first() == Some(&b'-') {
                return false;
            }

            attr.bytes()
                .all(|b| matches!(b, b'-' | b'.' | b'_' | b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'))
        }

        attr_valid(attr)
            .then(|| NameRef(attr.to_str().expect("no illformed utf8")))
            .ok_or_else(|| Error::new_with_input("Attribute has non-ascii characters or starts with '-'", attr))
    }
}

impl<'a> Name {
    /// Provide our ref-type.
    pub fn as_ref(&'a self) -> NameRef<'a> {
        NameRef(self.as_str())
    }

    /// Return the inner `str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for Name {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Name {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_newtype_struct("Name", self.as_str())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Name {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(rename = "Name")]
        struct NameDef(String);

        let NameDef(value) = <NameDef as serde::Deserialize>::deserialize(deserializer)?;
        NameRef::try_from(value.as_bytes().as_bstr())
            .map(NameRef::to_owned)
            .map_err(serde::de::Error::custom)
    }
}

/// The error returned by [`parse::Iter`][crate::parse::Iter].
pub type Error = gix_error::ValidationError;
