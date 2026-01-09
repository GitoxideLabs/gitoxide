use std::fmt;

use bstr::BString;

/// The main error type for `gix-transport`, with variants that indicate retry-ability.
#[derive(Debug)]
pub enum Error {
    /// An IO error occurred.
    Io(std::io::Error),
    /// A URL could not be parsed.
    UrlParse(gix_url::parse::Error),
    /// UTF-8 conversion failed for a path.
    PathConversion(bstr::Utf8Error),
    /// The scheme is unsupported.
    UnsupportedScheme(gix_url::Scheme),
    /// The URL contains tokens that are incompatible with the scheme.
    UnsupportedUrlTokens {
        /// The URL that was problematic.
        url: BString,
        /// The scheme being used.
        scheme: gix_url::Scheme,
    },
    /// The repository path could be mistaken for a command-line argument.
    AmbiguousPath {
        /// The path that is ambiguous.
        path: BString,
    },
    /// A host name could be mistaken for a command-line argument.
    AmbiguousHostName {
        /// The host name that is ambiguous.
        host: String,
    },
    /// A user name could be mistaken for a command-line argument.
    AmbiguousUserName {
        /// The user name that is ambiguous.
        user: String,
    },
    /// The virtual host specification was invalid.
    VirtualHostInvalid {
        /// The invalid host string.
        host: String,
    },
    /// A request was performed without performing the handshake first.
    MissingHandshake,
    /// Capabilities could not be parsed.
    Capabilities(String),
    /// A packet line could not be decoded.
    LineDecode(String),
    /// A specific line was expected but missing.
    ExpectedLine(&'static str),
    /// Expected a data line, but got a delimiter.
    ExpectedDataLine,
    /// The transport layer does not support authentication.
    AuthenticationUnsupported,
    /// The transport layer refuses to use a given identity.
    AuthenticationRefused(&'static str),
    /// The protocol version is unsupported.
    UnsupportedProtocolVersion(BString),
    /// Failed to invoke a program.
    InvokeProgram {
        /// The command that failed.
        command: std::ffi::OsString,
        /// The underlying IO error.
        source: std::io::Error,
    },
    /// Could not initialize the HTTP client.
    InitHttpClient(String),
    /// An error occurred while uploading the body of a POST request.
    PostBody(std::io::Error),
    /// HTTP transport error with details.
    HttpDetail(String),
    /// Redirect URL could not be reconciled.
    Redirect {
        /// The redirect URL.
        redirect_url: String,
        /// The expected URL.
        expected_url: String,
    },
    /// Could not finish reading all data to post to the remote.
    ReadPostBody(std::io::Error),
    /// Request configuration failed.
    ConfigureRequest(String),
    /// Authentication failed.
    Authenticate(String),
    /// A 'Simple' SSH variant doesn't support a particular function.
    SshUnsupported {
        /// The simple command that should have been invoked.
        command: std::ffi::OsString,
        /// The function that was unsupported.
        function: &'static str,
    },
    /// Capabilities were missing entirely as there was no 0 byte.
    MissingDelimitingNullByte,
    /// There was not a single capability behind the delimiter.
    NoCapabilities,
    /// A version line was expected, but none was retrieved.
    MissingVersionLine,
    /// Expected 'version X', got something else.
    MalformattedVersionLine(BString),
    /// Got unsupported version.
    UnsupportedVersion {
        /// The desired protocol version.
        desired: crate::Protocol,
        /// The actual version received.
        actual: BString,
    },
    /// An HTTP feature is not compiled in.
    #[cfg(not(any(feature = "http-client-curl", feature = "http-client-reqwest")))]
    CompiledWithoutHttp(gix_url::Scheme),
    /// Connection failed with a nested error.
    Connection(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(err) => write!(f, "IO error: {err}"),
            Error::UrlParse(err) => write!(f, "URL parse error: {err}"),
            Error::PathConversion(err) => write!(f, "The git repository path could not be converted to UTF8: {err}"),
            Error::UnsupportedScheme(scheme) => write!(f, "The '{scheme}' protocol is currently unsupported"),
            Error::UnsupportedUrlTokens { url, scheme } => {
                write!(f, "The url {url:?} contains information that would not be used by the {scheme} protocol")
            }
            Error::AmbiguousPath { path } => {
                write!(f, "The repository path '{path}' could be mistaken for a command-line argument")
            }
            Error::AmbiguousHostName { host } => {
                write!(f, "Host name '{host}' could be mistaken for a command-line argument")
            }
            Error::AmbiguousUserName { user } => {
                write!(f, "Username '{user}' could be mistaken for a command-line argument")
            }
            Error::VirtualHostInvalid { host } => {
                write!(f, "Could not parse {host:?} as virtual host with format <host>[:port]")
            }
            Error::MissingHandshake => write!(f, "A request was performed without performing the handshake first"),
            Error::Capabilities(msg) => write!(f, "Capabilities could not be parsed: {msg}"),
            Error::LineDecode(msg) => write!(f, "A packet line could not be decoded: {msg}"),
            Error::ExpectedLine(line_type) => write!(f, "A {line_type} line was expected, but there was none"),
            Error::ExpectedDataLine => write!(f, "Expected a data line, but got a delimiter"),
            Error::AuthenticationUnsupported => write!(f, "The transport layer does not support authentication"),
            Error::AuthenticationRefused(reason) => write!(f, "The transport layer refuses to use a given identity: {reason}"),
            Error::UnsupportedProtocolVersion(version) => {
                write!(f, "The protocol version indicated by {version:?} is unsupported")
            }
            Error::InvokeProgram { command, source } => write!(f, "Failed to invoke program {command:?}: {source}"),
            Error::InitHttpClient(msg) => write!(f, "Could not initialize the http client: {msg}"),
            Error::PostBody(err) => write!(f, "An IO error occurred while uploading the body of a POST request: {err}"),
            Error::HttpDetail(description) => write!(f, "{description}"),
            Error::Redirect { redirect_url, expected_url } => {
                write!(
                    f,
                    "Redirect url {redirect_url:?} could not be reconciled with original url {expected_url} as they don't share the same suffix"
                )
            }
            Error::ReadPostBody(err) => write!(f, "Could not finish reading all data to post to the remote: {err}"),
            Error::ConfigureRequest(msg) => write!(f, "Request configuration failed: {msg}"),
            Error::Authenticate(msg) => write!(f, "Authentication failed: {msg}"),
            Error::SshUnsupported { command, function } => {
                write!(f, "The 'Simple' ssh variant doesn't support {function}: {command:?}")
            }
            Error::MissingDelimitingNullByte => write!(f, "Capabilities were missing entirely as there was no 0 byte"),
            Error::NoCapabilities => write!(f, "there was not a single capability behind the delimiter"),
            Error::MissingVersionLine => write!(f, "a version line was expected, but none was retrieved"),
            Error::MalformattedVersionLine(line) => write!(f, "expected 'version X', got {line:?}"),
            Error::UnsupportedVersion { desired, actual } => {
                write!(f, "Got unsupported version {actual:?}, expected {}", *desired as u8)
            }
            #[cfg(not(any(feature = "http-client-curl", feature = "http-client-reqwest")))]
            Error::CompiledWithoutHttp(scheme) => {
                write!(
                    f,
                    "'{scheme}' is not compiled in. Compile with the 'http-client-curl' or 'http-client-reqwest' cargo feature"
                )
            }
            Error::Connection(msg) => write!(f, "connection failed: {msg}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            Error::UrlParse(err) => Some(err),
            Error::PathConversion(err) => Some(err),
            Error::InvokeProgram { source, .. } => Some(source),
            Error::PostBody(err) => Some(err),
            Error::ReadPostBody(err) => Some(err),
            _ => None,
        }
    }
}

impl crate::IsSpuriousError for Error {
    fn is_spurious(&self) -> bool {
        match self {
            Error::Io(err) => err.is_spurious(),
            Error::PostBody(err) => err.is_spurious(),
            Error::ReadPostBody(err) => err.is_spurious(),
            Error::InvokeProgram { source, .. } => source.is_spurious(),
            Error::Connection(_) => true, // Connection errors are generally retry-able
            _ => false,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<gix_url::parse::Error> for Error {
    fn from(err: gix_url::parse::Error) -> Self {
        Error::UrlParse(err)
    }
}

impl From<bstr::Utf8Error> for Error {
    fn from(err: bstr::Utf8Error) -> Self {
        Error::PathConversion(err)
    }
}

impl From<gix_packetline::decode::Error> for Error {
    fn from(err: gix_packetline::decode::Error) -> Self {
        Error::LineDecode(err.to_string())
    }
}

#[cfg(feature = "http-client-curl")]
impl From<curl::Error> for Error {
    fn from(err: curl::Error) -> Self {
        Error::Connection(err.to_string())
    }
}

#[cfg(feature = "http-client-reqwest")]
impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Connection(err.to_string())
    }
}

#[cfg(feature = "blocking-client")]
impl From<gix_credentials::protocol::Error> for Error {
    fn from(err: gix_credentials::protocol::Error) -> Self {
        Error::Authenticate(err.to_string())
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for Error {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Error::Connection(err.to_string())
    }
}
