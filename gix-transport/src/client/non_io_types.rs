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
    pub type Error = gix_error::Exn<gix_error::Message>;
}

mod error {
    use std::ffi::OsString;

    use bstr::BString;

    #[cfg(feature = "blocking-client")]
    use crate::client::blocking_io::ssh;
    use crate::client::capabilities;

    #[cfg(feature = "http-client")]
    type HttpError = gix_error::Error;
    #[cfg(feature = "blocking-client")]
    type SshInvocationError = ssh::invocation::Error;
    #[cfg(not(feature = "http-client"))]
    type HttpError = std::convert::Infallible;
    #[cfg(not(feature = "blocking-client"))]
    type SshInvocationError = std::convert::Infallible;

    /// The error used in most methods of the [`client`][crate::client] module
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        MissingHandshake,
        Io(std::io::Error),
        Capabilities { err: gix_error::Error },
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
                Error::AuthenticationRefused(reason) => {
                    write!(f, "The transport layer refuses to use a given identity: {reason}")
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
                Error::LineDecode { err } => Some(err),
                Error::InvokeProgram { source, .. } => Some(source),
                Error::Capabilities { err } => Some(err),
                Error::Http(err) => Some(err),
                Error::SshInvocation(err) => err.source(),
                _ => None,
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
            Error::Capabilities { err: err.into_error() }
        }
    }

    impl From<gix_packetline::decode::Error> for Error {
        fn from(err: gix_packetline::decode::Error) -> Self {
            Error::LineDecode { err }
        }
    }

    impl Error {
        /// Return `true` if retrying the failed transport operation might succeed.
        pub fn can_retry(&self) -> bool {
            match self {
                Error::Io(err) => gix_error::can_retry(err),
                #[cfg(feature = "http-client")]
                Error::Http(err) => err.can_retry(),
                _ => false,
            }
        }
    }

    #[cfg(test)]
    mod tests {
        #[cfg(feature = "http-client")]
        use gix_error::{ErrorExt, message};

        #[cfg(feature = "http-client")]
        #[test]
        fn http_keeps_retryable_sources() {
            let err = super::Error::Http(
                std::io::Error::new(std::io::ErrorKind::TimedOut, "retry me")
                    .and_raise(message("HTTP failed"))
                    .into_error(),
            );

            assert!(err.can_retry());
            let source = std::error::Error::source(&err)
                .and_then(|err| err.downcast_ref::<gix_error::Error>())
                .expect("HTTP errors retain their gix-error wrapper");
            assert!(
                source
                    .iter_errors()
                    .any(<dyn std::error::Error + 'static>::is::<std::io::Error>)
            );
        }
    }
}

pub use error::Error;
