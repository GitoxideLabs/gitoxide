use bstr::BString;

use crate::protocol::{Context, ContextOptions};

/// Indicates key or values contain errors that can't be encoded.
#[derive(Debug)]
#[expect(missing_docs)]
pub enum Error {
    Encoding { key: String, value: BString },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Encoding { key, value } => write!(
                f,
                "{key:?}={value:?} must not contain null bytes or newlines neither in key nor in value."
            ),
        }
    }
}

impl std::error::Error for Error {}

impl Context {
    /// Create a context containing `url`, encoded and decoded according to `options`.
    pub fn from_url(url: impl Into<BString>, options: ContextOptions) -> Self {
        Context {
            options,
            url: Some(url.into()),
            ..Default::default()
        }
    }
}

mod access {
    use bstr::BString;

    use crate::protocol::Context;

    impl Context {
        /// Clear all fields that are considered secret.
        pub fn clear_secrets(&mut self) {
            let Context {
                options: _,
                protocol: _,
                host: _,
                path: _,
                username: _,
                password,
                oauth_refresh_token,
                password_expiry_utc: _,
                url: _,
                quit: _,
            } = self;

            *password = None;
            *oauth_refresh_token = None;
        }
        /// Replace existing secrets with the word `<redacted>`.
        pub fn redacted(mut self) -> Self {
            let Context {
                options: _,
                protocol: _,
                host: _,
                path: _,
                username: _,
                password,
                oauth_refresh_token,
                password_expiry_utc: _,
                url: _,
                quit: _,
            } = &mut self;
            for secret in [password, oauth_refresh_token].into_iter().flatten() {
                *secret = "<redacted>".into();
            }
            self
        }

        /// Convert all relevant fields into a URL for consumption.
        pub fn to_url(&self) -> Option<BString> {
            use bstr::{ByteSlice, ByteVec};
            let mut buf: BString = self.protocol.clone()?.into();
            buf.push_str(b"://");
            if let Some(user) = &self.username {
                buf.push_str(user);
                buf.push(b'@');
            }
            if let Some(host) = &self.host {
                buf.push_str(host);
            }
            if let Some(path) = &self.path {
                if !path.starts_with_str("/") {
                    buf.push(b'/');
                }
                buf.push_str(path);
            }
            buf.into()
        }
        /// Compute a prompt to obtain the given value.
        pub fn to_prompt(&self, field: &str) -> String {
            match self.to_url() {
                Some(url) => format!("{field} for {url}: "),
                None => format!("{field}: "),
            }
        }
    }
}

mod mutate {
    use bstr::ByteSlice;

    use crate::{protocol, protocol::Context};

    /// In-place mutation
    impl Context {
        /// Destructure the url at our `url` field into parts like protocol, host, username and path and store
        /// them in our respective fields. If `use_http_path` is set, http paths are significant even though
        /// normally this isn't the case.
        #[expect(
            clippy::result_large_err,
            reason = "will be removed once `gix-error` is used consistently"
        )]
        pub fn destructure_url_in_place(&mut self, use_http_path: bool) -> Result<&mut Self, protocol::Error> {
            if self.url.is_none() {
                self.url = Some(self.to_url().ok_or(protocol::Error::UrlMissing)?);
            }

            let url = gix_url::parse(self.url.as_ref().expect("URL is present after check above"))?;
            self.protocol = Some(url.scheme.as_str().into());
            self.username = url.user().map(ToOwned::to_owned);
            self.password = url.password().map(ToOwned::to_owned);
            self.host = url.host().map(ToOwned::to_owned).map(|mut host| {
                let port = url.port.filter(|port| {
                    url.scheme
                        .default_port()
                        .is_none_or(|default_port| *port != default_port)
                });
                if let Some(port) = port {
                    use std::fmt::Write;
                    write!(host, ":{port}").expect("infallible");
                }
                host
            });
            if !matches!(url.scheme, gix_url::Scheme::Http | gix_url::Scheme::Https) || use_http_path {
                let path = url.path.trim_with(|b| b == '/');
                self.path = (!path.is_empty()).then(|| path.into());
            }
            Ok(self)
        }
    }
}

mod serde;
pub use self::serde::decode;
