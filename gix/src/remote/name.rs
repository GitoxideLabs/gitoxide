use crate::error::{ErrorExt, validation};
use std::borrow::Cow;

use crate::{
    Error, Result,
    bstr::{BStr, BString, ByteSlice, ByteVec},
};

/// Assemble `refs/remotes/<remote>/<suffix>` without validating or interpreting either input as text.
/// An empty suffix leaves the trailing slash for use as a namespace prefix.
pub(crate) fn remote_tracking_ref(remote: &BStr, suffix: &[u8]) -> BString {
    let mut qualified = gix_ref::Category::RemoteBranch.prefix().to_owned();
    qualified.push_str(remote);
    qualified.push(b'/');
    qualified.push_str(suffix);
    qualified
}

/// A validated symbolic remote name, such as `origin`.
///
/// It can be used under `refs/remotes/<name>/`, for example in a default fetch refspec.
/// Use [`ConfiguredName`] to represent an existing config subsection without that restriction.
#[derive(Debug, PartialEq, Eq, Clone, Ord, PartialOrd, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(try_from = "BString")
)]
pub struct Name(BString);

/// Lifecycle
impl Name {
    /// Validate and create a symbolic remote name.
    ///
    /// Names can contain `/` and non-UTF-8 bytes, but `.` is not accepted.
    /// Errors include the original bytes as `input` [metadata](gix_error::Error::metadata()).
    pub fn symbol(name: impl Into<BString>) -> Result<Self> {
        let name = name.into();
        Self::validate_symbol(name.as_bstr())?;
        Ok(Self(name))
    }

    fn validate_symbol(name: &BStr) -> Result<()> {
        let test_ref = remote_tracking_ref(name, b"test");
        gix_validate::reference::name(test_ref.as_bstr()).map_err(|err| {
            err.and_raise(
                validation("remote name cannot be used to construct refs under refs/remotes/<name>/")
                    .with_input(name.to_owned()),
            )
        })?;
        Ok(())
    }
}

impl Name {
    /// Borrow the validated symbolic name as bytes.
    pub fn as_bstr(&self) -> &BStr {
        self.0.as_bstr()
    }
}

impl From<Name> for BString {
    fn from(name: Name) -> Self {
        name.0
    }
}

impl TryFrom<BString> for Name {
    type Error = Error;

    fn try_from(name: BString) -> Result<Self> {
        Self::symbol(name)
    }
}

impl TryFrom<Cow<'_, BStr>> for Name {
    type Error = Error;

    fn try_from(name: Cow<'_, BStr>) -> Result<Self> {
        Self::validate_symbol(name.as_ref())?;
        Ok(Self(name.into_owned()))
    }
}

/// A remote's exact `[remote "…"]` subsection name.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfiguredName {
    /// A validated symbolic name, such as `origin`.
    Symbolic(Name),
    /// The literal `[remote "."]` subsection. Without an explicit fetch URL, it fetches from the local repository:
    ///
    /// ```gitconfig
    /// [remote "."]
    ///     fetch = refs/heads/main:refs/tracking/local-main
    /// ```
    Dot,
    /// A subsection name other than `.` that is not a valid [`Name`], such as a name with spaces or a URL.
    ///
    /// Both `git fetch "backup mirror"` and `git fetch https://example.com/project.git` use the settings
    /// of the matching subsection:
    ///
    /// ```gitconfig
    /// [remote "backup mirror"]
    ///     url = ../backup.git
    ///     fetch = +refs/heads/*:refs/remotes/backup/*
    /// [remote "https://example.com/project.git"]
    ///     url = https://mirror.example.com/project.git
    ///     fetch = +refs/heads/*:refs/remotes/project/*
    /// ```
    ///
    /// The original subsection bytes are preserved for lookup and round-tripping through [`crate::Remote::save_to()`].
    NonSymbolic(BString),
}

/// Lifecycle
impl ConfiguredName {
    /// Classify a `[remote "…"]` subsection name.
    ///
    /// `.` becomes [`Self::Dot`]. Other names become [`Self::Symbolic`] if they pass [`Name::symbol()`] validation,
    /// or [`Self::NonSymbolic`] otherwise.
    pub fn from_config(name: &BStr) -> Self {
        if name == "." {
            return Self::Dot;
        }
        match Name::try_from(Cow::Borrowed(name)) {
            Ok(name) => Self::Symbolic(name),
            Err(_) => Self::NonSymbolic(name.to_owned()),
        }
    }
}

impl ConfiguredName {
    /// Borrow the symbolic name, or return `None` for [`Self::Dot`] and [`Self::NonSymbolic`].
    /// Use [`Self::as_config_key()`] for the subsection name in all cases.
    pub fn as_name(&self) -> Option<&Name> {
        match self {
            Self::Symbolic(name) => Some(name),
            Self::Dot | Self::NonSymbolic(_) => None,
        }
    }

    /// Borrow the exact bytes of the `[remote "…"]` subsection name.
    pub fn as_config_key(&self) -> &BStr {
        match self {
            Self::Symbolic(name) => name.as_bstr(),
            Self::Dot => BStr::new("."),
            Self::NonSymbolic(name) => name.as_bstr(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ByteSlice, remote_tracking_ref};

    #[test]
    fn remote_tracking_ref_preserves_bytes_and_supports_namespace_prefixes() {
        let remote = b"team/upstream\xff".as_bstr();
        assert_eq!(
            remote_tracking_ref(remote, b"topic/branch\xfe").as_bstr(),
            b"refs/remotes/team/upstream\xff/topic/branch\xfe".as_bstr(),
            "both the remote name and the suffix retain their original bytes"
        );
        assert_eq!(
            remote_tracking_ref(remote, b"").as_bstr(),
            b"refs/remotes/team/upstream\xff/".as_bstr(),
            "an empty suffix leaves the trailing slash for prefix matching"
        );
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

    impl_partial_eq_bytes!(Name, value => value.0.as_bstr());
    impl_partial_eq_pair!(&Name, String, name => name.0.as_bytes(), text => text.as_bytes());
    impl_partial_eq_pair!(&Name, BString, name => name.0.as_bytes(), text => text.as_bytes());
}
