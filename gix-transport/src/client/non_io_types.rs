/// Configure how a `RequestWriter` behaves when writing bytes.
#[derive(Default, PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WriteMode {
    /// Each [write()][std::io::Write::write()] call writes the bytes verbatim as one or more packet lines.
    ///
    /// This mode also indicates to the transport that it should try to stream data as it is unbounded. This mode is typically used
    /// for sending packs whose exact size is not necessarily known in advance.
    Binary,
    /// Each [write()][std::io::Write::write()] call assumes text in the input, assures a trailing newline and writes it as single packet line.
    ///
    /// This mode also indicates that the lines written fit into memory, hence the transport may chose to not stream it but to buffer it
    /// instead. This is relevant for some transports, like the one for HTTP.
    #[default]
    OneLfTerminatedLinePerWriteCall,
}

/// The kind of packet line to write when transforming a `RequestWriter` into an `ExtendedBufRead`.
///
/// Both the type and the trait have different implementations for blocking vs async I/O.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MessageKind {
    /// A `flush` packet.
    Flush,
    /// A V2 delimiter.
    Delimiter,
    /// The end of a response.
    ResponseEnd,
    /// The given text.
    Text(&'static [u8]),
}

#[cfg(any(feature = "blocking-client", feature = "async-client"))]
pub(crate) mod connect {
    /// Options for connecting to a remote.
    #[derive(Debug, Default, Clone)]
    pub struct Options {
        /// Use `version` to set the desired protocol version to use when connecting, but note that the server may downgrade it.
        pub version: crate::Protocol,
        #[cfg(feature = "blocking-client")]
        /// Options to use if the scheme of the URL is `ssh`.
        pub ssh: crate::client::blocking_io::ssh::connect::Options,
        /// If `true`, all packetlines received or sent will be passed to the facilities of the `gix-trace` crate.
        pub trace: bool,
    }

    /// The error used in `connect()`.
    ///
    /// (Both blocking and async I/O use the same error type.)
    // TODO(review): this implementation hand-preserves `#[error(transparent)]` semantics for `Url`:
    //                `Display` passes the formatter through and `source()` forwards to the inner error's
    //                source, exactly like the `thiserror`-generated code did. The same pattern is used
    //                for `client::Error::{Http, SshInvocation}` and the http backend errors
    //                (`http::Error`, curl, reqwest).
    #[derive(Debug)]
    #[allow(missing_docs)]
    pub enum Error {
        Url(gix_url::parse::Error),
        PathConversion(bstr::Utf8Error),
        Connection(Box<dyn std::error::Error + Send + Sync>),
        UnsupportedUrlTokens {
            url: bstr::BString,
            scheme: gix_url::Scheme,
        },
        UnsupportedScheme(gix_url::Scheme),
        #[cfg(not(any(feature = "http-client-curl", feature = "http-client-reqwest")))]
        CompiledWithoutHttp(gix_url::Scheme),
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Url(err) => std::fmt::Display::fmt(err, f),
                Error::PathConversion(_) => f.write_str("The git repository path could not be converted to UTF8"),
                Error::Connection(_) => f.write_str("connection failed"),
                Error::UnsupportedUrlTokens { url, scheme } => write!(
                    f,
                    "The url {url:?} contains information that would not be used by the {scheme} protocol"
                ),
                Error::UnsupportedScheme(scheme) => write!(f, "The '{scheme}' protocol is currently unsupported"),
                #[cfg(not(any(feature = "http-client-curl", feature = "http-client-reqwest")))]
                Error::CompiledWithoutHttp(scheme) => write!(
                    f,
                    "'{scheme}' is not compiled in. Compile with the 'http-client-curl' or 'http-client-reqwest' cargo feature"
                ),
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::Url(err) => err.source(),
                Error::PathConversion(err) => Some(err),
                Error::Connection(err) => Some(&**err),
                Error::UnsupportedUrlTokens { .. } | Error::UnsupportedScheme(_) => None,
                #[cfg(not(any(feature = "http-client-curl", feature = "http-client-reqwest")))]
                Error::CompiledWithoutHttp(_) => None,
            }
        }
    }

    impl From<gix_url::parse::Error> for Error {
        fn from(err: gix_url::parse::Error) -> Self {
            Error::Url(err)
        }
    }

    impl From<bstr::Utf8Error> for Error {
        fn from(err: bstr::Utf8Error) -> Self {
            Error::PathConversion(err)
        }
    }

    impl From<Box<dyn std::error::Error + Send + Sync>> for Error {
        fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
            Error::Connection(err)
        }
    }

    // TODO: maybe fix this workaround: want `IsSpuriousError`  in `Connection(…)`
    impl crate::IsSpuriousError for Error {
        fn is_spurious(&self) -> bool {
            match self {
                Error::Connection(err) => {
                    #[cfg(feature = "blocking-client")]
                    if let Some(err) = err.downcast_ref::<crate::client::git::blocking_io::connect::Error>() {
                        return err.is_spurious();
                    }
                    if let Some(err) = err.downcast_ref::<crate::client::Error>() {
                        return err.is_spurious();
                    }
                    false
                }
                _ => false,
            }
        }
    }
}

mod error {
    use std::ffi::OsString;

    use bstr::BString;

    #[cfg(feature = "http-client")]
    use crate::client::blocking_io::http;
    #[cfg(feature = "blocking-client")]
    use crate::client::blocking_io::ssh;
    use crate::client::capabilities;

    #[cfg(feature = "http-client")]
    type HttpError = http::Error;
    #[cfg(feature = "blocking-client")]
    type SshInvocationError = ssh::invocation::Error;
    #[cfg(not(feature = "http-client"))]
    type HttpError = std::convert::Infallible;
    #[cfg(not(feature = "blocking-client"))]
    type SshInvocationError = std::convert::Infallible;

    /// The error used in most methods of the [`client`][crate::client] module
    #[derive(Debug)]
    #[allow(missing_docs)]
    pub enum Error {
        MissingHandshake,
        Io(std::io::Error),
        Capabilities { err: capabilities::Error },
        LineDecode { err: gix_packetline::decode::Error },
        ExpectedLine(&'static str),
        ExpectedDataLine,
        AuthenticationUnsupported,
        AuthenticationRefused(&'static str),
        UnsupportedProtocolVersion(BString),
        InvokeProgram { source: std::io::Error, command: OsString },
        Http(HttpError),
        SshInvocation(SshInvocationError),
        AmbiguousPath { path: BString },
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::MissingHandshake => {
                    f.write_str("A request was performed without performing the handshake first")
                }
                Error::Io(_) => f.write_str("An IO error occurred when talking to the server"),
                Error::Capabilities { .. } => f.write_str("Capabilities could not be parsed"),
                Error::LineDecode { .. } => f.write_str("A packet line could not be decoded"),
                Error::ExpectedLine(line) => write!(f, "A {line} line was expected, but there was none"),
                Error::ExpectedDataLine => f.write_str("Expected a data line, but got a delimiter"),
                Error::AuthenticationUnsupported => f.write_str("The transport layer does not support authentication"),
                Error::AuthenticationRefused(identity) => {
                    write!(f, "The transport layer refuses to use a given identity: {identity}")
                }
                Error::UnsupportedProtocolVersion(version) => {
                    write!(f, "The protocol version indicated by {version:?} is unsupported")
                }
                Error::InvokeProgram { command, .. } => write!(f, "Failed to invoke program {command:?}"),
                Error::Http(err) => std::fmt::Display::fmt(err, f),
                Error::SshInvocation(err) => std::fmt::Display::fmt(err, f),
                Error::AmbiguousPath { path } => {
                    write!(
                        f,
                        "The repository path '{path}' could be mistaken for a command-line argument"
                    )
                }
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::Io(err) => Some(err),
                Error::Capabilities { err } => Some(err),
                Error::LineDecode { err } => Some(err),
                Error::InvokeProgram { source, .. } => Some(source),
                Error::Http(err) => err.source(),
                Error::SshInvocation(err) => err.source(),
                Error::MissingHandshake
                | Error::ExpectedLine(_)
                | Error::ExpectedDataLine
                | Error::AuthenticationUnsupported
                | Error::AuthenticationRefused(_)
                | Error::UnsupportedProtocolVersion(_)
                | Error::AmbiguousPath { .. } => None,
            }
        }
    }

    impl From<std::io::Error> for Error {
        fn from(err: std::io::Error) -> Self {
            Error::Io(err)
        }
    }

    impl From<capabilities::Error> for Error {
        fn from(err: capabilities::Error) -> Self {
            Error::Capabilities { err }
        }
    }

    impl From<gix_packetline::decode::Error> for Error {
        fn from(err: gix_packetline::decode::Error) -> Self {
            Error::LineDecode { err }
        }
    }

    impl From<HttpError> for Error {
        fn from(err: HttpError) -> Self {
            Error::Http(err)
        }
    }

    impl crate::IsSpuriousError for Error {
        fn is_spurious(&self) -> bool {
            match self {
                Error::Io(err) => err.is_spurious(),
                Error::Http(err) => err.is_spurious(),
                _ => false,
            }
        }
    }
}

pub use error::Error;
