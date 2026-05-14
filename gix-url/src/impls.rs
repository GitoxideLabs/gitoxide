use std::path::{Path, PathBuf};

use bstr::BStr;

use crate::{Scheme, Url, parse};

impl Default for Url {
    fn default() -> Self {
        Url {
            serialize_alternative_form: false,
            scheme: Scheme::Ssh,
            user: None,
            password: None,
            host: None,
            port: None,
            path: bstr::BString::default(),
        }
    }
}

impl TryFrom<&str> for Url {
    type Error = parse::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_bytes(value.into())
    }
}

impl TryFrom<String> for Url {
    type Error = parse::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_bytes(value.as_str().into())
    }
}

impl TryFrom<PathBuf> for Url {
    type Error = parse::Error;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        gix_path::into_bstr(value).try_into()
    }
}

impl TryFrom<&Path> for Url {
    type Error = parse::Error;

    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        gix_path::into_bstr(value).try_into()
    }
}

impl TryFrom<&std::ffi::OsStr> for Url {
    type Error = parse::Error;

    fn try_from(value: &std::ffi::OsStr) -> Result<Self, Self::Error> {
        gix_path::os_str_into_bstr(value)
            .expect("no illformed UTF-8 on Windows")
            .try_into()
    }
}

impl TryFrom<&BStr> for Url {
    type Error = parse::Error;

    fn try_from(value: &BStr) -> Result<Self, Self::Error> {
        Self::from_bytes(value)
    }
}

impl<'a> TryFrom<std::borrow::Cow<'a, BStr>> for Url {
    type Error = parse::Error;

    fn try_from(value: std::borrow::Cow<'a, BStr>) -> Result<Self, Self::Error> {
        Self::try_from(&*value)
    }
}

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut storage;
        let to_print = if self.password.is_some() {
            storage = self.clone();
            storage.password = Some("redacted".into());
            &storage
        } else {
            self
        };
        to_print.to_bstring().fmt(f)
    }
}
