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
}

mod error {
    use std::ffi::OsString;

    use bstr::BString;

    #[cfg(feature = "blocking-client")]
    use crate::client::blocking_io::ssh;

    #[cfg(feature = "blocking-client")]
    type SshInvocationError = ssh::invocation::Error;
    #[cfg(not(feature = "blocking-client"))]
    type SshInvocationError = std::convert::Infallible;

    /// Details carried by an HTTP authentication failure in a [`std::io::Error`] of kind
    /// [`PermissionDenied`][std::io::ErrorKind::PermissionDenied].
    ///
    /// Callers can downcast [`std::io::Error::get_ref()`] to this type and forward the challenges
    /// to credential helpers as `wwwauth[]` attributes.
    #[derive(Debug, Default)]
    pub struct AuthenticationRequired {
        /// HTTP `WWW-Authenticate` header values in the order supplied by the server.
        pub www_authenticate: Vec<BString>,
    }

    impl std::fmt::Display for AuthenticationRequired {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("Received HTTP status 401")
        }
    }

    impl std::error::Error for AuthenticationRequired {}

    /// The error used in most methods of the [`client`][crate::client] module.
    ///
    /// Sources preserve classifications when raised or converted to [`gix_error::Error`]. Use
    /// [`gix_error::can_retry()`] or [`gix_error::can_retry_lenient()`] to inspect this error directly.
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        MissingHandshake,
        Io(std::io::Error),
        Capabilities {
            err: gix_error::Error,
        },
        LineDecode {
            err: gix_error::ValidationError,
        },
        ExpectedLine(&'static str),
        ExpectedDataLine,
        AuthenticationUnsupported,
        AuthenticationRefused(&'static str),
        UnsupportedProtocolVersion(BString),
        InvokeProgram {
            source: std::io::Error,
            command: OsString,
        },
        #[cfg(feature = "http-client")]
        Http(gix_error::Error),
        #[cfg(not(feature = "http-client"))]
        Http(std::convert::Infallible),
        SshInvocation(SshInvocationError),
        AmbiguousPath {
            path: BString,
        },
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
                Error::InvokeProgram { command, .. } => write!(f, "Failed to invoke program {}", command.display()),
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
                Error::SshInvocation(err) => Some(err),
                Error::AmbiguousPath { .. } => Some(&crate::INVALID_INPUT),
                Error::ExpectedLine(_) | Error::ExpectedDataLine => Some(&crate::CORRUPTION),
                _ => None,
            }
        }
    }

    impl From<std::io::Error> for Error {
        fn from(err: std::io::Error) -> Self {
            Error::Io(err)
        }
    }

    impl From<gix_error::Exn<gix_error::Message>> for Error {
        fn from(err: gix_error::Exn<gix_error::Message>) -> Self {
            Error::Capabilities { err: err.into_error() }
        }
    }

    impl From<gix_error::ValidationError> for Error {
        fn from(err: gix_error::ValidationError) -> Self {
            Error::LineDecode { err }
        }
    }

    impl From<Error> for gix_error::Error {
        fn from(err: Error) -> Self {
            Self::from_error(err)
        }
    }

    #[cfg(test)]
    mod tests {
        use gix_error::ErrorExt;
        #[cfg(feature = "http-client")]
        use gix_error::{RetryableError, message};

        #[test]
        fn io_classification_is_independent_of_conversion() {
            for kind in [
                std::io::ErrorKind::Interrupted,
                std::io::ErrorKind::UnexpectedEof,
                std::io::ErrorKind::TimedOut,
                std::io::ErrorKind::BrokenPipe,
                std::io::ErrorKind::AddrInUse,
                std::io::ErrorKind::ConnectionAborted,
                std::io::ErrorKind::ConnectionReset,
                std::io::ErrorKind::ConnectionRefused,
                std::io::ErrorKind::OutOfMemory,
                std::io::ErrorKind::NotFound,
                std::io::ErrorKind::PermissionDenied,
            ] {
                let make_error = || super::Error::Io(kind.into());
                let can_retry = gix_error::can_retry(&make_error());
                let can_retry_lenient = gix_error::can_retry_lenient(&make_error());
                for err in [
                    gix_error::Error::from(make_error()),
                    gix_error::Error::from_error(make_error()),
                    make_error().raise().into_error(),
                ] {
                    assert_eq!(
                        err.can_retry(),
                        can_retry,
                        "conversion preserves the conservative policy for {kind:?}"
                    );
                    assert_eq!(
                        err.can_retry_lenient(),
                        can_retry_lenient,
                        "conversion preserves the lenient policy for {kind:?}"
                    );
                    assert!(
                        !err.is_retryable(),
                        "conversion does not add an explicit retry marker for {kind:?}"
                    );
                    assert_eq!(err.is_not_found(), kind == std::io::ErrorKind::NotFound);
                    assert_eq!(err.is_resource_exhausted(), kind == std::io::ErrorKind::OutOfMemory);
                }
            }
        }

        #[cfg(feature = "http-client")]
        #[test]
        fn http_keeps_retryable_sources() {
            let err = super::Error::Http(
                std::io::Error::new(std::io::ErrorKind::BrokenPipe, "retry me")
                    .and_raise(message("HTTP failed"))
                    .into_error(),
            );

            assert!(gix_error::can_retry_lenient(&err));
            assert!(!gix_error::can_retry(&err));
            let source = std::error::Error::source(&err)
                .and_then(|err| err.downcast_ref::<gix_error::Error>())
                .expect("HTTP errors retain their gix-error wrapper");
            assert!(
                source
                    .iter_errors()
                    .any(<dyn std::error::Error + 'static>::is::<std::io::Error>)
            );

            let explicit = super::Error::Http(
                RetryableError::new(message("retry me"))
                    .and_raise(message("HTTP failed"))
                    .into_error(),
            );
            assert!(gix_error::can_retry(&explicit));
            assert!(gix_error::Error::from(explicit).is_retryable());

            let out_of_memory = super::Error::Http(
                std::io::Error::from(std::io::ErrorKind::OutOfMemory)
                    .and_raise(message("HTTP failed"))
                    .into_error(),
            );
            assert!(!gix_error::can_retry(&out_of_memory));
            assert!(gix_error::can_retry_lenient(&out_of_memory));
            assert!(gix_error::Error::from(out_of_memory).is_resource_exhausted());
        }

        #[test]
        fn custom_errors_expose_classifications() {
            let err = gix_error::Error::from(super::Error::AmbiguousPath { path: "-arg".into() });
            assert!(err.is_validation(), "unsafe transport arguments are invalid input");
            for err in [super::Error::ExpectedLine("version"), super::Error::ExpectedDataLine] {
                assert!(
                    gix_error::Error::from(err).is_corrupted(),
                    "unexpected packet lines are malformed responses"
                );
            }

            #[cfg(feature = "blocking-client")]
            {
                use crate::client::blocking_io::ssh;

                for err in [
                    ssh::invocation::Error::AmbiguousUserName { user: "-arg".into() },
                    ssh::invocation::Error::AmbiguousHostName { host: "-arg".into() },
                ] {
                    let err = gix_error::Error::from(super::Error::SshInvocation(err));
                    assert!(err.is_validation(), "unsafe SSH arguments are invalid input");
                    assert!(
                        err.downcast_any_ref::<ssh::invocation::Error>().is_some(),
                        "the concrete SSH error remains available"
                    );
                }
                let err = gix_error::Error::from_error(ssh::Error::AmbiguousHostName { host: "-arg".into() });
                assert!(err.is_validation(), "SSH connection errors expose invalid input too");
            }
        }
    }
}

pub use error::{AuthenticationRequired, Error};
