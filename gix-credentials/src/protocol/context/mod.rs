use bstr::BString;

use crate::protocol::{Context, ContextOptions};

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
    use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_encode, utf8_percent_encode};

    use crate::protocol::Context;

    const COMPONENT: &AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'.').remove(b'_').remove(b'~');
    const PROTOCOL: &AsciiSet = &COMPONENT.remove(b'+');
    const HOST: &AsciiSet = &COMPONENT.remove(b':').remove(b'[').remove(b']');
    const HTTP_HOST: &AsciiSet = &HOST.remove(b'%');
    const PATH: &AsciiSet = &COMPONENT.remove(b'/');

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
                www_authenticate: _,
                capability: _,
                authtype: _,
                credential,
                ephemeral: _,
                url: _,
                quit: _,
            } = self;

            *password = None;
            *oauth_refresh_token = None;
            *credential = None;
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
                www_authenticate: _,
                capability: _,
                authtype: _,
                credential,
                ephemeral: _,
                url: _,
                quit: _,
            } = &mut self;
            for secret in [password, oauth_refresh_token, credential].into_iter().flatten() {
                *secret = "<redacted>".into();
            }
            self
        }

        /// Convert all relevant fields into a URL for consumption, escaping component delimiters.
        /// Passwords are omitted, and a missing protocol yields `None`.
        pub fn to_url(&self) -> Option<BString> {
            use bstr::{ByteSlice, ByteVec};
            let protocol = self.protocol.as_deref()?;
            let mut buf: BString = utf8_percent_encode(protocol, PROTOCOL).to_string().into();
            buf.push_str(b"://");
            if let Some(user) = &self.username {
                buf.push_str(utf8_percent_encode(user, COMPONENT).to_string());
                buf.push(b'@');
            }
            if let Some(host) = &self.host {
                // HTTP and file hosts retain URL escapes in gix-url, while other schemes decode them.
                let encode_set = if matches!(protocol, "http" | "https" | "file") {
                    HTTP_HOST
                } else {
                    HOST
                };
                buf.push_str(utf8_percent_encode(host, encode_set).to_string());
            }
            if let Some(path) = &self.path {
                if !path.starts_with_str("/") {
                    buf.push(b'/');
                }
                buf.push_str(percent_encode(path, PATH).to_string());
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

    use gix_error::Result;
    use gix_error::{OptionExt, validation};

    use crate::protocol::Context;

    /// In-place mutation
    impl Context {
        /// Destructure the url at our `url` field into parts like protocol, host, username and path and store
        /// them in our respective fields. If `use_http_path` is set, http paths are significant even though
        /// normally this isn't the case.
        /// If no URL is supplied, construct and validate one without reinterpreting the supplied host or credentials.
        pub fn destructure_url_in_place(&mut self, use_http_path: bool) -> Result<&mut Self> {
            let from_components = self.url.is_none();
            if from_components {
                self.url = Some(self.to_url().ok_or_raise(|| {
                    validation("Either 'url' field or both 'protocol' and 'host' fields must be provided")
                })?);
            }

            let url = gix_url::parse(self.url.as_ref().expect("URL is present after check above"))?;
            if !matches!(url.scheme, gix_url::Scheme::Http | gix_url::Scheme::Https) || use_http_path {
                let path = url.path.trim_with(|b| b == '/');
                self.path = (!path.is_empty()).then(|| path.into());
            }
            if from_components {
                return Ok(self);
            }
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
            Ok(self)
        }
    }
}

mod serde;
pub use self::serde::decode;
