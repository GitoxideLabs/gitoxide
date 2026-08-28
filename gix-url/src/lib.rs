//! A library implementing a URL for use in git with access to its special capabilities.
//!
//! ## Examples
//!
//! ```
//! let mut url = gix_url::parse("ssh://git@example.com/gitoxide").unwrap();
//! assert_eq!(url.user(), Some("git"));
//! assert_eq!(url.host(), Some("example.com"));
//! assert_eq!(url.to_bstring(), "ssh://git@example.com/gitoxide");
//!
//! assert_eq!(url.set_user(Some("byron".into())), Some("git".into()));
//! assert_eq!(url.user_argument_safe(), Some("byron"));
//! assert_eq!(url.to_bstring(), "ssh://byron@example.com/gitoxide");
//!
//! let suspicious = gix_url::parse("ssh://-Fconfig@host/repo").unwrap();
//! assert_eq!(suspicious.user_argument_safe(), None, "The user isn't returned as it looks like an argument");
//! ```
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

use std::{borrow::Cow, path::PathBuf};

use bstr::{BStr, BString};
use gix_utils::AsBStr;

const HTTP_PATH_ENCODE_SET: &percent_encoding::AsciiSet = &percent_encoding::CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'`')
    .add(b'{')
    .add(b'}');

/// User-home expansion for repository paths.
pub mod expand_path;

mod scheme;
pub use scheme::Scheme;
mod impls;

/// Parsing errors and input classifications.
pub mod parse;

/// Minimal URL parser to replace the `url` crate dependency
mod simple_url;

/// Parse a Git remote location from `input`.
///
/// This accepts standard URLs, SCP-like SSH locations, remote-helper locations and local paths. URL and SCP-like
/// inputs must be UTF-8; remote-helper addresses and local paths retain arbitrary bytes.
///
/// Locations of the `<helper>::<address>` form described in
/// [`gitremote-helpers`](https://git-scm.com/docs/gitremote-helpers) are recognized before any URL
/// syntax, so an address may itself contain `://`. They are represented as
/// [`Scheme::Helper`] holding the helper name, with the address kept verbatim in [`Url::path`] as only the helper
/// program can interpret it. The command-executing `ext` helper is represented as [`Scheme::Ext`] in both spellings;
/// `ext://<address>` retains the entire URL as its command. Other unknown URL transports in the
/// `<helper>://<address>` form are represented as [`Scheme::HelperUrl`].
///
/// # Deviation
///
/// Unlike Git, this rejects textual and overflowing ports in SSH and Git URLs. Git treats such port text as part of
/// the hostname, which hides the malformed port and causes a less useful hostname-resolution error later.
///
/// Also unlike Git, an empty remote-helper name as in `::address` is not accepted, as the `git-remote-` program it
/// would name cannot meaningfully exist.
pub fn parse(input: impl AsBStr) -> Result<Url, parse::Error> {
    use parse::InputScheme;
    let input = input.as_bstr();
    match parse::find_scheme(input) {
        InputScheme::RemoteHelper { helper_end } => Ok(parse::remote_helper(input, helper_end)),
        InputScheme::Local => parse::local(input),
        InputScheme::Url { protocol_end } if input[..protocol_end] == *b"file" => parse::file_url(input, protocol_end),
        InputScheme::Url { protocol_end } => parse::url(input, protocol_end),
        InputScheme::Scp { colon } => parse::scp(input, colon),
    }
}

/// Expand `path` for the given `user`, which can be obtained from [`expand_path::parse()`], resolving the home
/// directory automatically.
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
/// Public fields can be modified into combinations that do not serialize or parse. Use [`parse()`] or
/// [`Url::from_parts()`] at trust boundaries; do not assume an accepted `Url` remains valid without revalidation.
///
/// # Serialization
///
/// This type does not implement `Into<String>`, `From<Url> for String` because URLs
/// can contain non-UTF-8 sequences in the path component when parsed from raw bytes.
/// Use [to_bstring()](Url::to_bstring()) for complete serialization, including non-UTF-8 path bytes, or use the
/// [`Display`](std::fmt::Display) trait for a UTF-8 representation that redacts passwords for safe logging.
///
/// When the `serde` feature is enabled, this type implements `serde::Serialize` and `serde::Deserialize`,
/// which will serialize *all* fields, including the password.
///
/// # Security Warning
///
/// URLs may contain passwords and using standard [formatting](std::fmt::Display) will redact such passwords,
/// whereas [`Url::to_bstring()`] includes all URL parts.
/// **Beware that some URLs still print secrets if they use them outside of the designated password fields.**
///
/// Also note that URLs that fail to parse are typically stored in [the resulting error](parse::Error) type
/// and printed in full using its display implementation.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
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
    /// Re-encoded during canonical serialization. Its presence makes serialization use canonical rather than alternative
    /// form because SCP-like and local-path syntax cannot represent a password.
    pub password: Option<String>,
    /// The host to which to connect, or `None` for locations without a host, such as local paths.
    ///
    /// Brackets are stripped from parsed SSH hosts and otherwise preserved as parsed. Serialization adds brackets to
    /// unbracketed colon-containing hosts when needed to disambiguate a port or scoped IPv6 address.
    /// DNS-like ASCII hosts are lowercased. Non-HTTP hosts are percent-decoded for Git compatibility, while HTTP and
    /// HTTPS host escapes remain encoded.
    pub host: Option<String>,
    /// Request alternative serialization, generally because the location was parsed in that form.
    ///
    /// Alternative forms include SCP-like syntax (`user@host:path`), bare file paths, and the `<helper>::<address>`
    /// syntax of [`gitremote-helpers`](https://git-scm.com/docs/gitremote-helpers).
    /// It is used only for SSH or file locations without a password or port. [`Scheme::Helper`] and [`Scheme::Ext`]
    /// always use remote-helper form, and SCP-like form is also retained when canonical SSH form would change a
    /// relative repository path.
    pub serialize_alternative_form: bool,
    /// The explicit port, if parsed or assigned.
    ///
    /// Git accepts port zero in SSH and Git URLs, so this field may contain `0`. Textual and overflowing ports are
    /// rejected unlike Git; see the deviation documented on [`parse()`]. Use [`Self::port_or_default()`] to obtain a
    /// scheme default when this is `None`.
    pub port: Option<u16>,
    /// The path portion of the URL, usually the location of the git repository.
    ///
    /// Percent-encoded characters are decoded during parsing (e.g., `%20` becomes a space in this field). An unchanged
    /// parsed path may retain its original encoded spelling for serialization. Constructed or modified HTTP paths are
    /// percent-encoded during canonical serialization; other schemes write the path bytes as stored.
    ///
    /// Path normalization during parsing:
    /// - SSH/Git schemes: Leading `/~` is stripped (e.g., `/~repo` becomes `~repo`)
    /// - SSH/Git schemes: Empty paths are rejected as errors
    /// - HTTP/HTTPS schemes: Empty paths are normalized to `/`
    ///
    /// This type has no separate query or fragment fields. For HTTP and HTTPS, `?`, `#`, and everything after them are
    /// stored in this field. For other URL schemes, Git treats `?` and `#` before the first slash as authority text.
    ///
    /// For locations in the `<helper>::<address>` form of
    /// [`gitremote-helpers`](https://git-scm.com/docs/gitremote-helpers), this holds the address verbatim,
    /// uninterpreted and possibly empty, as only the helper program can make sense of it.
    ///
    /// During serialization, SSH/Git URLs prepend `/` to paths not starting with `/`.
    ///
    /// # Security Warning
    ///
    /// URLs allow paths to start with `-` which makes it possible to mask command-line arguments as path which then leads to
    /// the invocation of programs from an attacker controlled URL. See <https://secure.phabricator.com/T12961> for details.
    ///
    /// For a slash-prefixed path that will be passed intact to a command-line application, call
    /// [`Self::path_argument_safe()`]. Other path forms require validation appropriate to how they will be passed.
    pub path: BString,
    /// The original parsed path when it contains percent escapes.
    ///
    /// This lets serialization retain the encoded spelling while [`Self::path`] remains decoded. It is reused only
    /// while decoding it still produces the public path; constructing or mutating the public path encodes percent signs.
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) path_with_percent_escapes: Option<BString>,
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Url {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Fields {
            scheme: Scheme,
            user: Option<String>,
            password: Option<String>,
            host: Option<String>,
            serialize_alternative_form: bool,
            port: Option<u16>,
            path: BString,
            #[serde(default)]
            path_with_percent_escapes: Option<BString>,
        }

        let mut fields = Fields::deserialize(deserializer)?;
        if fields.path_with_percent_escapes.as_ref() == Some(&fields.path) {
            fields.path_with_percent_escapes = Some(encode_legacy_http_path(&fields.path));
            fields.path = percent_encoding::percent_decode(&fields.path)
                .collect::<Vec<_>>()
                .into();
        }
        Ok(Url {
            scheme: fields.scheme,
            user: fields.user,
            password: fields.password,
            host: fields.host,
            serialize_alternative_form: fields.serialize_alternative_form,
            port: fields.port,
            path: fields.path,
            path_with_percent_escapes: fields.path_with_percent_escapes,
        })
    }
}

#[cfg(feature = "serde")]
fn encode_legacy_http_path(path: &[u8]) -> BString {
    let mut out = Vec::with_capacity(path.len());
    let mut start = 0;
    let mut pos = 0;
    while pos + 2 < path.len() {
        if path[pos] == b'%' && path[pos + 1].is_ascii_hexdigit() && path[pos + 2].is_ascii_hexdigit() {
            out.extend(
                percent_encoding::percent_encode(&path[start..pos], HTTP_PATH_ENCODE_SET)
                    .to_string()
                    .bytes(),
            );
            out.extend_from_slice(&path[pos..pos + 3]);
            pos += 3;
            start = pos;
        } else {
            pos += 1;
        }
    }
    out.extend(
        percent_encoding::percent_encode(&path[start..], HTTP_PATH_ENCODE_SET)
            .to_string()
            .bytes(),
    );
    out.into()
}

/// Instantiation
impl Url {
    /// Create an instance from the given parts and validate it by serializing and parsing it back.
    ///
    /// For HTTP and HTTPS, `path` is decoded data: literal percent signs are encoded during serialization, and an empty
    /// path is normalized to `/`. Other schemes interpret `path` according to their serialized syntax.
    /// `serialize_alternative_form` merely requests alternative form; passwords, ports, and unsupported schemes force
    /// canonical URL serialization.
    ///
    /// # Panics
    ///
    /// Panics if the supplied parts cannot be serialized before validation, such as a user without a host.
    pub fn from_parts(
        scheme: Scheme,
        user: Option<String>,
        password: Option<String>,
        host: Option<String>,
        port: Option<u16>,
        path: BString,
        serialize_alternative_form: bool,
    ) -> Result<Self, parse::Error> {
        if let Scheme::Helper(name) = &scheme
            && !parse::is_valid_remote_helper_name(name.as_bytes())
        {
            return Err(parse::Error::InvalidRemoteHelperName { name: name.clone() });
        }
        let is_http = matches!(scheme, Scheme::Http | Scheme::Https);
        let mut parsed = parse(
            Url {
                scheme,
                user,
                password,
                host,
                port,
                path: path.clone(),
                serialize_alternative_form,
                path_with_percent_escapes: None,
            }
            .to_bstring(),
        )?;
        if is_http {
            // Preserve the caller's path as decoded data, except for an empty path normalized to `/` above. In
            // particular, percent escapes supplied through `from_parts()` are literal text and must be encoded.
            if !path.is_empty() {
                parsed.path = path;
            }
            parsed.path_with_percent_escapes = None;
        }
        Ok(parsed)
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
    /// Request alternative serialization, e.g. `file:///path` becomes `/path`.
    ///
    /// Parsed URLs set this automatically. Alternative form is used only for SSH or file locations without a password
    /// or port; all other values serialize in canonical URL form.
    ///
    /// Setting `use_alternate_form` to `false` requests canonical, URL-like, serialization. SCP-like form is retained if
    /// canonical SSH form would change a relative repository path. Remote-helper form is always retained because URL
    /// form would change the address Git passes to the helper.
    pub fn with_request_alternate_form(mut self, use_alternate_form: bool) -> Self {
        self.serialize_alternative_form = use_alternate_form;
        self
    }

    /// Resolve the path of a file location against `current_dir` and normalize it in place.
    ///
    /// Other schemes are unchanged.
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
    /// Use this method instead of [Self::user()] if the username is going to be passed to a command-line application.
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
    /// Use this method or [Self::user_as_argument()] instead of [Self::user()] if the username is going to be
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

    fn path_with_percent_escapes(&self) -> Option<&BStr> {
        let encoded = self.path_with_percent_escapes.as_ref()?;
        percent_encoding::percent_decode(encoded)
            .eq(self.path.iter().copied())
            .then_some(encoded.as_ref())
    }

    /// Return the original percent-escaped path if [Self::path] wasn't changed in the meantime, or [`Self::path`] otherwise.
    pub fn original_path(&self) -> &BStr {
        self.path_with_percent_escapes().unwrap_or(self.path.as_ref())
    }

    /// Return a slash-prefixed path if the bytes after the slash can't be mistaken for a command-line argument.
    ///
    /// The leading slash must be present and must be passed to the command. Empty and non-slash-prefixed paths return
    /// `None`; validate those according to how they will be passed.
    pub fn path_argument_safe(&self) -> Option<&BStr> {
        self.path
            .strip_prefix(b"/")
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
                File | Ext | Helper(_) | HelperUrl(_) => return None,
            })
        })
    }
}

fn looks_like_command_line_option(b: &[u8]) -> bool {
    b.first() == Some(&b'-')
}

/// Transformation
impl Url {
    /// Return a clone whose file path is resolved against `current_dir` and normalized.
    ///
    /// Other schemes are returned unchanged.
    pub fn canonicalized(&self, current_dir: &std::path::Path) -> Result<Self, gix_path::realpath::Error> {
        let mut res = self.clone();
        res.canonicalize(current_dir)?;
        Ok(res)
    }
}

/// Serialization
impl Url {
    /// Write all URL components, including the password, to `out` in a form suitable for parsing again.
    ///
    /// Parsed escapes for reserved path characters retain their spelling while [`Self::path`] is unchanged, but other
    /// escaping and canonicalization can change the original input spelling. Invalid combinations created through
    /// public field mutation may return an error.
    pub fn write_to(&self, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        if matches!(self.scheme, Scheme::Ext | Scheme::Helper(_)) {
            if let Scheme::Helper(name) = &self.scheme
                && !parse::is_valid_remote_helper_name(name.as_bytes())
            {
                return Err(std::io::Error::other("invalid remote-helper name"));
            }
            if self.user.is_some() || self.password.is_some() || self.host.is_some() || self.port.is_some() {
                return Err(std::io::Error::other(
                    "remote-helper form cannot represent user, password, host or port",
                ));
            }
            return self.write_remote_helper_form_to(out);
        }

        if self.scheme == Scheme::Ssh
            && !self.path.is_empty()
            && !self.path.starts_with(b"/")
            && !self.path.starts_with(b"~")
        {
            if self.password.is_none() && self.port.is_none() && self.host.is_some() {
                return self.write_alternative_form_to(out);
            }
            return Err(std::io::Error::other(
                "relative SSH paths cannot be serialized canonically without changing their meaning",
            ));
        }

        // Since alternative form doesn't employ any escape syntax, password and
        // port number cannot be encoded.
        if self.serialize_alternative_form && self.password.is_none() && self.port.is_none() {
            match &self.scheme {
                Scheme::File | Scheme::Ssh => return self.write_alternative_form_to(out),
                _ => {}
            }
        }
        self.write_canonical_form_to(out)
    }

    fn write_remote_helper_form_to(&self, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        out.write_all(self.scheme.as_str().as_bytes())?;
        out.write_all(b"::")?;
        out.write_all(&self.path)
    }

    fn write_canonical_form_to(&self, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        fn percent_encode(s: &str, encode_colon: bool) -> Cow<'_, str> {
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
            const USERNAME_ENCODE_SET: &percent_encoding::AsciiSet = &USERINFO_ENCODE_SET.add(b':');

            let encode_set = if encode_colon {
                USERNAME_ENCODE_SET
            } else {
                USERINFO_ENCODE_SET
            };
            percent_encoding::utf8_percent_encode(s, encode_set).into()
        }

        fn write_host(out: &mut dyn std::io::Write, host: &str, bracket: bool, scheme: &Scheme) -> std::io::Result<()> {
            if bracket {
                out.write_all(b"[")?;
                out.write_all(host.replace('%', "%25").as_bytes())?;
                out.write_all(b"]")
            } else if matches!(scheme, Scheme::File | Scheme::Http | Scheme::Https) {
                out.write_all(host.as_bytes())
            } else {
                out.write_all(percent_encode(host, host.parse::<std::net::Ipv6Addr>().is_err()).as_bytes())
            }
        }

        out.write_all(self.scheme.as_str().as_bytes())?;
        out.write_all(b"://")?;

        let needs_brackets = self.host_needs_brackets()
            && (self.port.is_some() || self.host.as_ref().is_some_and(|host| host.contains('%')));

        match (&self.user, &self.host) {
            (Some(user), Some(host)) => {
                out.write_all(percent_encode(user, true).as_bytes())?;
                if let Some(password) = &self.password {
                    out.write_all(b":")?;
                    out.write_all(percent_encode(password, false).as_bytes())?;
                }
                out.write_all(b"@")?;
                write_host(out, host, needs_brackets, &self.scheme)?;
            }
            (None, Some(host)) => {
                write_host(out, host, needs_brackets, &self.scheme)?;
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
        if let Some(encoded) = self.path_with_percent_escapes() {
            out.write_all(encoded)?;
        } else if matches!(self.scheme, Scheme::Http | Scheme::Https) {
            // We intentionally do not encode '?' and '#': ParsedUrl keeps them in `path`,
            // and encoding would change routed endpoints for already parsed URLs.
            write!(
                out,
                "{}",
                percent_encoding::percent_encode(&self.path, HTTP_PATH_ENCODE_SET)
            )?;
        } else {
            out.write_all(&self.path)?;
        }
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

    /// Serialize all URL components, including the password, into a binary string.
    ///
    /// Parsed escapes for reserved path characters retain their spelling while [`Self::path`] is unchanged, but other
    /// escaping and canonicalization can change the original input spelling.
    ///
    /// # Panics
    ///
    /// Panics if public field mutation created a structure that cannot be serialized, such as a user without a host.
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
#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    #[test]
    fn legacy_encoded_public_path_is_migrated() -> gix_testtools::Result {
        for (input, legacy_path, decoded_path) in [
            ("https://example.com/a%2Fb", "/a%2Fb", "/a/b"),
            ("https://example.com/%20%25", "/ %25", "/ %"),
        ] {
            let mut legacy = crate::parse(input)?;
            legacy.path = legacy_path.into();
            legacy.path_with_percent_escapes = Some(legacy_path.into());

            let migrated: crate::Url = serde_json::from_slice(&serde_json::to_vec(&legacy)?)?;
            assert_eq!(
                migrated.path, decoded_path,
                "the public path is upgraded to decoded form"
            );
            assert_eq!(migrated.to_bstring(), input, "the encoded spelling remains lossless");
        }
        Ok(())
    }
}

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
                path_with_percent_escapes: None,
            }
        }
    }

    impl TestUrlExtension for Url {}
}
