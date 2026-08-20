use std::borrow::Cow;

use super::Name;
use crate::bstr::{BStr, BString, ByteSlice, ByteVec};

/// The error returned by [validated()].
#[derive(Debug, thiserror::Error)]
#[error("remote names must be valid within refspecs for fetching: {name:?}")]
#[expect(missing_docs)]
pub struct Error {
    pub source: gix_error::Error,
    pub name: BString,
}

/// Return `name` if it is valid as symbolic remote name.
///
/// This means it has to be valid within a the ref path of a tracking branch.
pub fn validated(name: impl Into<BString>) -> Result<BString, Error> {
    let name = name.into();
    match gix_refspec::parse(
        format!("refs/heads/test:refs/remotes/{name}/test").as_str().into(),
        gix_refspec::parse::Operation::Fetch,
    ) {
        Ok(_) => Ok(name),
        Err(err) => Err(Error {
            source: err.into_error(),
            name,
        }),
    }
}

impl Name<'_> {
    /// Obtain the name as string representation.
    pub fn as_bstr(&self) -> &BStr {
        match self {
            Name::Symbol(v) => v.as_ref().into(),
            Name::Url(v) => v.as_ref(),
        }
    }

    /// Return this instance as a symbolic name, if it is one.
    pub fn as_symbol(&self) -> Option<&str> {
        match self {
            Name::Symbol(n) => n.as_ref().into(),
            Name::Url(_) => None,
        }
    }

    /// Return this instance as url, if it is one.
    pub fn as_url(&self) -> Option<&BStr> {
        match self {
            Name::Url(n) => n.as_ref().into(),
            Name::Symbol(_) => None,
        }
    }

    /// Return a fully-owned copy of this instance.
    pub fn to_owned(&self) -> Name<'static> {
        match self {
            Name::Symbol(s) => Name::Symbol(s.clone().into_owned().into()),
            Name::Url(s) => Name::Url(s.clone().into_owned().into()),
        }
    }
}

impl<'a> TryFrom<Cow<'a, BStr>> for Name<'a> {
    type Error = Cow<'a, BStr>;

    fn try_from(name: Cow<'a, BStr>) -> Result<Self, Self::Error> {
        if name.contains(&b'/') || name.as_ref() == "." {
            Ok(Name::Url(name))
        } else {
            match name {
                Cow::Borrowed(n) => n.to_str().ok().map(Cow::Borrowed).ok_or(name),
                Cow::Owned(n) => Vec::from(n)
                    .into_string()
                    .map_err(|err| Cow::Owned(err.into_vec().into()))
                    .map(Cow::Owned),
            }
            .map(Name::Symbol)
        }
    }
}

impl From<BString> for Name<'static> {
    fn from(name: BString) -> Self {
        Self::try_from(Cow::Owned(name)).expect("String is never illformed")
    }
}

impl AsRef<BStr> for Name<'_> {
    fn as_ref(&self) -> &BStr {
        self.as_bstr()
    }
}

mod impls {
    use crate::{
        bstr::{BStr, BString, ByteSlice},
        remote::Name,
    };

    macro_rules! impl_partial_eq {
        ($left_type:ty, $right_type:ty, $left:ident => $left_bytes:expr, $right:ident => $right_bytes:expr) => {
            impl PartialEq<$right_type> for $left_type {
                fn eq(&self, other: &$right_type) -> bool {
                    let $left = self;
                    let $right = other;
                    $left_bytes == $right_bytes
                }
            }
        };
    }

    macro_rules! impl_partial_eq_pair {
        ($left_type:ty, $right_type:ty, $left:ident => $left_bytes:expr, $right:ident => $right_bytes:expr) => {
            impl_partial_eq!($left_type, $right_type, $left => $left_bytes, $right => $right_bytes);
            impl_partial_eq!($right_type, $left_type, $right => $right_bytes, $left => $left_bytes);
        };
    }

    macro_rules! impl_partial_eq_bytes {
        ($type:ty, $value:ident => $bytes:expr) => {
            impl_partial_eq_pair!($type, str, $value => $bytes.as_bytes(), other => other.as_bytes());
            impl_partial_eq_pair!($type, &str, $value => $bytes.as_bytes(), other => other.as_bytes());
            impl_partial_eq_pair!($type, String, $value => $bytes.as_bytes(), other => other.as_bytes());
            impl_partial_eq_pair!($type, BStr, $value => $bytes.as_bytes(), other => other.as_bytes());
            impl_partial_eq_pair!($type, &BStr, $value => $bytes.as_bytes(), other => other.as_bytes());
            impl_partial_eq_pair!($type, BString, $value => $bytes.as_bytes(), other => other.as_bytes());
        };
    }

    impl_partial_eq_bytes!(Name<'_>, value => value.as_bstr());
    impl_partial_eq_pair!(
        &Name<'_>,
        String,
        name => name.as_bstr().as_bytes(),
        text => text.as_bytes()
    );
    impl_partial_eq_pair!(
        &Name<'_>,
        BString,
        name => name.as_bstr().as_bytes(),
        text => text.as_bytes()
    );
}
