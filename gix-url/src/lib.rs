//! A library implementing a URL for use in git with access to its special capabilities.
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(rust_2018_idioms, missing_docs)]
#![forbid(unsafe_code)]

use std::{borrow::Cow, path::PathBuf};

use bstr::{BStr, BString};

///
pub mod expand_path;

mod scheme;
pub use scheme::Scheme;
mod impls;

///
pub mod parse;

/// Minimal URL parser to replace the `url` crate dependency
mod simple_url;

/// Parse the given `bytes` as a [git url](Url).
///
/// # Note
///
/// We cannot and should never have to deal with UTF-16 encoded windows strings, so bytes input is acceptable.
/// For file-paths, we don't expect UTF8 encoding either.
pub fn parse(input: &BStr) -> Result<Url, parse::Error> {
    use parse::InputScheme;
    match parse::find_scheme(input) {
        InputScheme::Local => parse::local(input),
        InputScheme::Url { protocol_end } if input[..protocol_end].eq_ignore_ascii_case(b"file") => {
            parse::file_url(input, protocol_end)
        }
        InputScheme::Url { protocol_end } => parse::url(input, protocol_end),
        InputScheme::Scp { colon } => parse::scp(input, colon),
    }
}

/// Expand `path` for the given `user`, which can be obtained by [`parse()`], resolving the home directories
/// of `user` automatically.
///
/// If more precise control of the resolution mechanism is needed, then use the [expand_path::with()] function.
pub fn expand_path(user: Option<&expand_path::ForUser>, path: &BStr) -> Result<PathBuf, expand_path::Error> {
    expand_path::with(user, path, |user| match user {
        expand_path::ForUser::Current => gix_path::env::home_dir(),
        expand_path::ForUser::Name(user) => {
            gix_path::env::home_dir().and_then(|home| home.parent().map(|home_dirs| home_dirs.join(user.to_string())))
        }
    })
}

/// Classification of a portion of a URL by whether it is *syntactically* safe to pass as an argument to a command-line program.
///
/// Various parts of URLs can be specified to begin with `-`. If they are used as options to a command-line application
/// such as an SSH client, they will be treated as options rather than as non-option arguments as the developer intended.
/// This is a security risk, because URLs are not always trusted and can often be composed or influenced by an attacker.
/// See <https://secure.phabricator.com/T12961> for details.
///
/// # Security Warning
///
/// This type only expresses known *syntactic* risk. It does not cover other risks, such as passing a personal access
/// token as a username rather than a password in an application that logs usernames.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum ArgumentSafety<'a> {
    /// May be safe. There is nothing to pass, so there is nothing dangerous.
    Absent,
    /// May be safe. The argument does not begin with a `-` and so will not be confused as an option.
    Usable(&'a str),
    /// Dangerous! Begins with `-` and could be treated as an option. Use the value in error messages only.
    Dangerous(&'a str),
}

/// A URL with support for specialized git related capabilities.
///
/// Additionally, there is support for [deserialization](Url::from_bytes()) and [serialization](Url::to_bstring()).
///
/// # Mutability Warning
///
/// Due to the mutability of this type, it's possible that the URL serializes to something invalid
/// when fields are modified directly. URLs should always be parsed to this type from string or byte
/// parameters, but never be accepted as an instance of this type and then reconstructed, to maintain
/// validity guarantees.
///
/// # Serialization
///
/// This type does not implement `Into<String>`, `From<Url> for String` because URLs
/// can contain non-UTF-8 sequences in the path component when parsed from raw bytes.
/// Use [to_bstring()](Url::to_bstring()) for lossless serialization, or use the [`Display`](std::fmt::Display)
/// trait for a UTF-8 representation that redacts passwords for safe logging.
///
/// When the `serde` feature is enabled, this type implements `serde::Serialize` and `serde::Deserialize`,
/// which will serialize *all* fields, including the password.
///
/// # Security Warning
///
/// URLs may contain passwords and using standard [formatting](std::fmt::Display) will redact
/// such password, whereas [lossless serialization](Url::to_bstring()) will contain all parts of the
/// URL.
/// **Beware that some URLs still print secrets if they use them outside of the designated password fields.**
///
/// Also note that URLs that fail to parse are typically stored in [the resulting error](parse::Error) type
/// and printed in full using its display implementation.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Url {
    /// The URL scheme.
    pub scheme: Scheme,
    /// The user to impersonate on the remote.
    ///
    /// Stored in decoded form: percent-encoded characters are decoded during parsing.
    /// Re-encoded during canonical serialization, but written as-is in alternative form.
    pub user: Option<String>,
    /// The password associated with a user.
    ///
    /// Stored in decoded form: percent-encoded characters are decoded during parsing.
    /// Re-encoded during canonical serialization. Cannot be serialized in alternative form (will panic in debug builds).
    pub password: Option<String>,
    /// The host to which to connect. Localhost is implied if `None`.
    ///
    /// IPv6 addresses are stored *without* brackets for SSH schemes, but *with* brackets for other schemes.
    /// Brackets are automatically added during serialization when needed (e.g., when a port is specified with an IPv6 host).
    pub host: Option<String>,
    /// When serializing, use the alternative forms as it was parsed as such.
    ///
    /// Alternative forms include SCP-like syntax (`user@host:path`) and bare file paths.
    /// When `true`, password and port cannot be serialized (will panic in debug builds).
    pub serialize_alternative_form: bool,
    /// The port to use when connecting to a host. If `None`, standard ports depending on `scheme` will be used.
    pub port: Option<u16>,
    /// The path portion of the URL, usually the location of the git repository.
    ///
    /// Paths are stored in decoded form: percent-encoded characters are decoded during parsing
    /// and re-encoded during canonical serialization (e.g., `%20` becomes a space in this field).
    ///
    /// Path normalization during parsing:
    /// - SSH/Git schemes: Leading `/~` is stripped (e.g., `/~repo` becomes `~repo`)
    /// - SSH/Git schemes: Empty paths are rejected as errors
    /// - HTTP/HTTPS schemes: Empty paths are normalized to `/`
    ///
    /// During serialization, SSH/Git URLs prepend `/` to paths not starting with `/`.
    ///
    /// # Security Warning
    ///
    /// URLs allow paths to start with `-` which makes it possible to mask command-line arguments as path which then leads to
    /// the invocation of programs from an attacker controlled URL. See <https://secure.phabricator.com/T12961> for details.
    ///
    /// If this value is ever going to be passed to a command-line application, call [Self::path_argument_safe()] instead.
    pub path: BString,
}

/// Instantiation
impl Url {
    /// Create a new instance from the given parts, including a password, which will be validated by parsing them back.
    pub fn from_parts(
        scheme: Scheme,
        user: Option<String>,
        password: Option<String>,
        host: Option<String>,
        port: Option<u16>,
        path: BString,
        serialize_alternative_form: bool,
    ) -> Result<Self, parse::Error> {
        parse(
            Url {
                scheme,
                user,
                password,
                host,
                port,
                path,
                serialize_alternative_form,
            }
            .to_bstring()
            .as_ref(),
        )
    }
}

/// Modification
impl Url {
    /// Set the given `user`, or unset it with `None`. Return the previous value.
    pub fn set_user(&mut self, user: Option<String>) -> Option<String> {
        let prev = self.user.take();
        self.user = user;
        prev
    }

    /// Set the given `password`, or unset it with `None`. Return the previous value.
    pub fn set_password(&mut self, password: Option<String>) -> Option<String> {
        let prev = self.password.take();
        self.password = password;
        prev
    }
}

/// Builder
impl Url {
    /// Enable alternate serialization for this url, e.g. `file:///path` becomes `/path`.
    ///
    /// This is automatically set correctly for parsed URLs, but can be set here for urls
    /// created by constructor.
    pub fn serialize_alternate_form(mut self, use_alternate_form: bool) -> Self {
        self.serialize_alternative_form = use_alternate_form;
        self
    }

    /// Turn a file url like `file://relative` into `file:///root/relative`, hence it assures the url's path component is absolute,
    /// using `current_dir` if needed to achieve that.
    pub fn canonicalize(&mut self, current_dir: &std::path::Path) -> Result<(), gix_path::realpath::Error> {
        if self.scheme == Scheme::File {
            let path = gix_path::from_bstr(Cow::Borrowed(self.path.as_ref()));
            let abs_path = gix_path::realpath_opts(path.as_ref(), current_dir, gix_path::realpath::MAX_SYMLINKS)?;
            self.path = gix_path::into_bstr(abs_path).into_owned();
        }
        Ok(())
    }
}

/// Access
impl Url {
    /// Return the username mentioned in the URL, if present.
    ///
    /// # Security Warning
    ///
    /// URLs allow usernames to start with `-` which makes it possible to mask command-line arguments as username which then leads to
    /// the invocation of programs from an attacker controlled URL. See <https://secure.phabricator.com/T12961> for details.
    ///
    /// If this value is ever going to be passed to a command-line application, call [Self::user_argument_safe()] instead.
    pub fn user(&self) -> Option<&str> {
        self.user.as_deref()
    }

    /// Classify the username of this URL by whether it is safe to pass as a command-line argument.
    ///
    /// Use this method instead of [Self::user()] if the host is going to be passed to a command-line application.
    /// If the unsafe and absent cases need not be distinguished, [Self::user_argument_safe()] may also be used.
    pub fn user_as_argument(&self) -> ArgumentSafety<'_> {
        match self.user() {
            Some(user) if looks_like_command_line_option(user.as_bytes()) => ArgumentSafety::Dangerous(user),
            Some(user) => ArgumentSafety::Usable(user),
            None => ArgumentSafety::Absent,
        }
    }

    /// Return the username of this URL if present *and* if it can't be mistaken for a command-line argument.
    ///
    /// Use this method or [Self::user_as_argument()] instead of [Self::user()] if the host is going to be
    /// passed to a command-line application. Prefer [Self::user_as_argument()] unless the unsafe and absent
    /// cases need not be distinguished from each other.
    pub fn user_argument_safe(&self) -> Option<&str> {
        match self.user_as_argument() {
            ArgumentSafety::Usable(user) => Some(user),
            _ => None,
        }
    }

    /// Return the password mentioned in the url, if present.
    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    /// Return the host mentioned in the URL, if present.
    ///
    /// # Security Warning
    ///
    /// URLs allow hosts to start with `-` which makes it possible to mask command-line arguments as host which then leads to
    /// the invocation of programs from an attacker controlled URL. See <https://secure.phabricator.com/T12961> for details.
    ///
    /// If this value is ever going to be passed to a command-line application, call [Self::host_as_argument()]
    /// or [Self::host_argument_safe()] instead.
    pub fn host(&self) -> Option<&str> {
        self.host.as_deref()
    }

    /// Classify the host of this URL by whether it is safe to pass as a command-line argument.
    ///
    /// Use this method instead of [Self::host()] if the host is going to be passed to a command-line application.
    /// If the unsafe and absent cases need not be distinguished, [Self::host_argument_safe()] may also be used.
    pub fn host_as_argument(&self) -> ArgumentSafety<'_> {
        match self.host() {
            Some(host) if looks_like_command_line_option(host.as_bytes()) => ArgumentSafety::Dangerous(host),
            Some(host) => ArgumentSafety::Usable(host),
            None => ArgumentSafety::Absent,
        }
    }

    /// Return the host of this URL if present *and* if it can't be mistaken for a command-line argument.
    ///
    /// Use this method or [Self::host_as_argument()] instead of [Self::host()] if the host is going to be
    /// passed to a command-line application. Prefer [Self::host_as_argument()] unless the unsafe and absent
    /// cases need not be distinguished from each other.
    pub fn host_argument_safe(&self) -> Option<&str> {
        match self.host_as_argument() {
            ArgumentSafety::Usable(host) => Some(host),
            _ => None,
        }
    }

    /// Return the path of this URL *if* it can't be mistaken for a command-line argument.
    /// Note that it always begins with a slash, which is ignored for this comparison.
    ///
    /// Use this method instead of accessing [Self::path] directly if the path is going to be passed to a
    /// command-line application, unless it is certain that the leading `/` will always be included.
    pub fn path_argument_safe(&self) -> Option<&BStr> {
        self.path
            .get(1..)
            .and_then(|truncated| (!looks_like_command_line_option(truncated)).then_some(self.path.as_ref()))
    }

    /// Return true if the path portion of the URL is `/`.
    pub fn path_is_root(&self) -> bool {
        self.path == "/"
    }

    /// Return the actual or default port for use according to the URL scheme.
    /// Note that there may be no default port either.
    pub fn port_or_default(&self) -> Option<u16> {
        self.port.or_else(|| {
            use Scheme::*;
            Some(match self.scheme {
                Http => 80,
                Https => 443,
                Ssh => 22,
                Git => 9418,
                File | Ext(_) => return None,
            })
        })
    }
}

fn looks_like_command_line_option(b: &[u8]) -> bool {
    b.first() == Some(&b'-')
}

/// Transformation
impl Url {
    /// Turn a file URL like `file://relative` into `file:///root/relative`, hence it assures the URL's path component is absolute, using
    /// `current_dir` if necessary.
    pub fn canonicalized(&self, current_dir: &std::path::Path) -> Result<Self, gix_path::realpath::Error> {
        let mut res = self.clone();
        res.canonicalize(current_dir)?;
        Ok(res)
    }
}

/// Serialization
impl Url {
    /// Write this URL losslessly to `out`, ready to be parsed again.
    pub fn write_to(&self, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        // Since alternative form doesn't employ any escape syntax, password and
        // port number cannot be encoded.
        if self.serialize_alternative_form
            && (self.scheme == Scheme::File || self.scheme == Scheme::Ssh)
            && self.password.is_none()
            && self.port.is_none()
        {
            self.write_alternative_form_to(out)
        } else {
            self.write_canonical_form_to(out)
        }
    }

    fn write_canonical_form_to(&self, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        fn percent_encode(s: &str) -> Cow<'_, str> {
            /// Characters that must be percent-encoded in the userinfo component of a URL.
            ///
            /// According to RFC 3986, userinfo can contain:
            /// - unreserved characters: `A-Z a-z 0-9 - . _ ~`
            /// - percent-encoded characters
            /// - sub-delims: `! $ & ' ( ) * + , ; =`
            /// - `:`
            ///
            /// This encode-set encodes everything else, particularly `@` (userinfo delimiter),
            /// `/` `?` `#` (path/query/fragment delimiters), and various other special characters.
            const USERINFO_ENCODE_SET: &percent_encoding::AsciiSet = &percent_encoding::CONTROLS
                .add(b' ')
                .add(b'"')
                .add(b'#')
                .add(b'%')
                .add(b'/')
                .add(b'<')
                .add(b'>')
                .add(b'?')
                .add(b'@')
                .add(b'[')
                .add(b'\\')
                .add(b']')
                .add(b'^')
                .add(b'`')
                .add(b'{')
                .add(b'|')
                .add(b'}');
            percent_encoding::utf8_percent_encode(s, USERINFO_ENCODE_SET).into()
        }

        out.write_all(self.scheme.as_str().as_bytes())?;
        out.write_all(b"://")?;

        let needs_brackets = self.port.is_some() && self.host_needs_brackets();

        match (&self.user, &self.host) {
            (Some(user), Some(host)) => {
                out.write_all(percent_encode(user).as_bytes())?;
                if let Some(password) = &self.password {
                    out.write_all(b":")?;
                    out.write_all(percent_encode(password).as_bytes())?;
                }
                out.write_all(b"@")?;
                if needs_brackets {
                    out.write_all(b"[")?;
                }
                out.write_all(host.as_bytes())?;
                if needs_brackets {
                    out.write_all(b"]")?;
                }
            }
            (None, Some(host)) => {
                if needs_brackets {
                    out.write_all(b"[")?;
                }
                out.write_all(host.as_bytes())?;
                if needs_brackets {
                    out.write_all(b"]")?;
                }
            }
            (None, None) => {}
            (Some(_user), None) => {
                return Err(std::io::Error::other(
                    "Invalid URL structure: user specified without host",
                ));
            }
        }
        if let Some(port) = &self.port {
            write!(out, ":{port}")?;
        }
        // For SSH and Git URLs, add leading '/' if path doesn't start with '/'
        // This handles paths like "~repo" which serialize as "/~repo" in URL form
        if matches!(self.scheme, Scheme::Ssh | Scheme::Git) && !self.path.starts_with(b"/") {
            out.write_all(b"/")?;
        }
        out.write_all(&self.path)?;
        Ok(())
    }

    fn host_needs_brackets(&self) -> bool {
        fn is_ipv6(h: &str) -> bool {
            h.contains(':') && !h.starts_with('[')
        }
        self.host.as_ref().is_some_and(|h| is_ipv6(h))
    }

    fn write_alternative_form_to(&self, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        let needs_brackets = self.host_needs_brackets();

        match (&self.user, &self.host) {
            (Some(user), Some(host)) => {
                out.write_all(user.as_bytes())?;
                out.write_all(b"@")?;
                if needs_brackets {
                    out.write_all(b"[")?;
                }
                out.write_all(host.as_bytes())?;
                if needs_brackets {
                    out.write_all(b"]")?;
                }
            }
            (None, Some(host)) => {
                if needs_brackets {
                    out.write_all(b"[")?;
                }
                out.write_all(host.as_bytes())?;
                if needs_brackets {
                    out.write_all(b"]")?;
                }
            }
            (None, None) => {}
            (Some(_user), None) => {
                return Err(std::io::Error::other(
                    "Invalid URL structure: user specified without host",
                ));
            }
        }
        if self.scheme == Scheme::Ssh {
            out.write_all(b":")?;
        }
        out.write_all(&self.path)?;
        Ok(())
    }

    /// Transform ourselves into a binary string, losslessly, or fail if the URL is malformed due to host or user parts being incorrect.
    pub fn to_bstring(&self) -> BString {
        let mut buf = Vec::with_capacity(
            (5 + 3)
                + self.user.as_ref().map(String::len).unwrap_or_default()
                + 1
                + self.host.as_ref().map(String::len).unwrap_or_default()
                + self.port.map(|_| 5).unwrap_or_default()
                + self.path.len(),
        );
        self.write_to(&mut buf).expect("io cannot fail in memory");
        buf.into()
    }
}

/// Deserialization
impl Url {
    /// Parse a URL from `bytes`.
    pub fn from_bytes(bytes: &BStr) -> Result<Self, parse::Error> {
        parse(bytes)
    }
}

/// This module contains extensions to the [Url] struct which are only intended to be used
/// for testing code. Do not use this module in production! For all intents and purposes, the APIs of
/// all functions and types exposed by this module are considered unstable and are allowed to break
/// even in patch releases!
#[doc(hidden)]
pub mod testing {
    use bstr::BString;

    use crate::{Scheme, Url};

    /// Additional functions for [Url] which are only intended to be used for tests.
    pub trait TestUrlExtension {
        /// Create a new instance from the given parts without validating them.
        ///
        /// This function is primarily intended for testing purposes. For production code please
        /// consider using [Url::from_parts] instead!
        fn from_parts_unchecked(
            scheme: Scheme,
            user: Option<String>,
            password: Option<String>,
            host: Option<String>,
            port: Option<u16>,
            path: BString,
            serialize_alternative_form: bool,
        ) -> Url {
            Url {
                scheme,
                user,
                password,
                host,
                port,
                path,
                serialize_alternative_form,
            }
        }
    }

    impl TestUrlExtension for Url {}
}
