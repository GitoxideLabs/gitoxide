use std::borrow::Cow;

use bstr::{BStr, BString, ByteSlice};

/// A signature like [`gix_actor::Signature`], but with all string fields being a `Cow`.
#[derive(Default, PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Signature<'a> {
    /// The possibly mapped name.
    pub name: Cow<'a, BStr>,
    /// The possibly mapped email.
    pub email: Cow<'a, BStr>,
    /// The time stamp at which the signature is performed.
    pub time: Cow<'a, BStr>,
}

impl<'a> From<Signature<'a>> for gix_actor::Signature {
    fn from(s: Signature<'a>) -> Self {
        gix_actor::Signature {
            name: s.name.into_owned(),
            email: s.email.into_owned(),
            time: s.time.into_owned(),
        }
    }
}

impl<'a> From<gix_actor::SignatureRef<'a>> for Signature<'a> {
    fn from(s: gix_actor::SignatureRef<'a>) -> Self {
        Signature {
            name: s.name.into(),
            email: s.email.into(),
            time: s.time.into(),
        }
    }
}

/// A resolved signature with borrowed fields for a mapped `name` and/or `email`.
pub struct ResolvedSignature<'a> {
    /// The mapped name.
    pub name: Option<&'a BStr>,
    /// The mapped email.
    pub email: Option<&'a BStr>,
}

impl<'a> ResolvedSignature<'a> {
    pub(crate) fn try_new(
        new_email: Option<&'a BString>,
        matched_email: &'a BStr,
        current_email: &'_ BStr,
        new_name: Option<&'a BString>,
    ) -> Option<Self> {
        let new_email = new_email
            .map(|n| n.as_bstr())
            .or_else(|| (matched_email != current_email).then_some(matched_email));
        match (new_email, new_name) {
            (None, None) => None,
            (new_email, new_name) => Some(ResolvedSignature {
                email: new_email.map(|v| v.as_bstr()),
                name: new_name.map(|v| v.as_bstr()),
            }),
        }
    }
}
