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
    /// Sources preserve classifications when raised or converted to [`gix_error::Error`]. Unsafe arguments and
    /// unexpected packet lines expose classification-only [`gix_error::ClassificationMarker`] sources.
    /// Use [`gix_error::classify()`] or the classification predicates on [`gix_error::Exn`] and [`gix_error::Error`],
    /// rather than downcasting these markers to concrete classifier errors. Use [`gix_error::classify()`] with
    /// [`can_retry()`](gix_error::types::Classifications::can_retry) or
    /// [`can_retry_lenient()`](gix_error::types::Classifications::can_retry_lenient) to inspect retryability directly.
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        MissingHandshake,
        Io(std::io::Error),
        Capabilities {
            err: gix_error::Error,
        },
        LineDecode {
            err: gix_error::Message,
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
                Error::SshInvocation(_) => f.write_str("Failed to prepare SSH invocation"),
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
                Error::AmbiguousPath { .. } => Some(const { &gix_error::ClassificationMarker::VALIDATION }),
                Error::ExpectedLine(_) | Error::ExpectedDataLine => {
                    Some(const { &gix_error::ClassificationMarker::CORRUPTION })
                }
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

    impl From<gix_error::Error> for Error {
        fn from(err: gix_error::Error) -> Self {
            Error::Capabilities { err }
        }
    }

    impl From<gix_error::Message> for Error {
        fn from(err: gix_error::Message) -> Self {
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
        use gix_error::{Class, ClassificationMarker, message};

        #[test]
        fn io_classification_is_independent_of_conversion() {
            let mut diagnostics = Vec::new();
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
                let can_retry = gix_error::classify(&make_error()).can_retry();
                let can_retry_lenient = gix_error::classify(&make_error()).can_retry_lenient();
                let err = gix_error::Error::from(make_error());
                assert_eq!(
                    err.can_retry(),
                    can_retry,
                    "the transport conversion preserves the conservative policy for {kind:?}"
                );
                assert_eq!(
                    err.can_retry_lenient(),
                    can_retry_lenient,
                    "the transport conversion preserves the lenient policy for {kind:?}"
                );
                assert!(
                    !err.is_retryable(),
                    "the transport conversion does not add an explicit retry marker for {kind:?}"
                );
                assert_eq!(err.is_not_found(), kind == std::io::ErrorKind::NotFound);
                assert_eq!(err.is_resource_exhausted(), kind == std::io::ErrorKind::OutOfMemory);
                diagnostics.push((kind, gix_error::TestError::from(err)));
            }
            insta::assert_debug_snapshot!(diagnostics, "transport I/O errors retain the original cause and retry policy", @"
            [
                (
                    Interrupted,
                    An IO error occurred when talking to the server
                    |
                    └─ operation interrupted,
                ),
                (
                    UnexpectedEof,
                    An IO error occurred when talking to the server
                    |
                    └─ unexpected end of file,
                ),
                (
                    TimedOut,
                    An IO error occurred when talking to the server
                    |
                    └─ timed out,
                ),
                (
                    BrokenPipe,
                    An IO error occurred when talking to the server
                    |
                    └─ broken pipe,
                ),
                (
                    AddrInUse,
                    An IO error occurred when talking to the server
                    |
                    └─ address in use,
                ),
                (
                    ConnectionAborted,
                    An IO error occurred when talking to the server
                    |
                    └─ connection aborted,
                ),
                (
                    ConnectionReset,
                    An IO error occurred when talking to the server
                    |
                    └─ connection reset,
                ),
                (
                    ConnectionRefused,
                    An IO error occurred when talking to the server
                    |
                    └─ connection refused,
                ),
                (
                    OutOfMemory,
                    An IO error occurred when talking to the server
                    |
                    └─ out of memory,
                ),
                (
                    NotFound,
                    An IO error occurred when talking to the server
                    |
                    └─ entity not found,
                ),
                (
                    PermissionDenied,
                    An IO error occurred when talking to the server
                    |
                    └─ permission denied,
                ),
            ]
            ");
        }

        #[cfg(feature = "http-client")]
        #[test]
        fn http_keeps_retryable_sources() {
            let err = super::Error::Http(
                std::io::Error::new(std::io::ErrorKind::BrokenPipe, "retry me")
                    .and_raise(message("HTTP failed"))
                    .into_error(),
            );
            insta::assert_debug_snapshot!(err, "http keeps retryable sources", @"
            Http(
                HTTP failed
                |
                └─ I/O error (BrokenPipe)
                |
                └─ retry me,
            )
            ");

            assert!(gix_error::classify(&err).can_retry_lenient());
            assert!(!gix_error::classify(&err).can_retry());
            let source = std::error::Error::source(&err)
                .and_then(|err| err.downcast_ref::<gix_error::Error>())
                .expect("HTTP errors retain their gix-error wrapper");
            assert!(
                source
                    .iter_errors()
                    .any(<dyn std::error::Error + 'static>::is::<std::io::Error>)
            );

            let explicit = super::Error::Http(
                ClassificationMarker::with_source(Class::Retryable, message("retry me"))
                    .and_raise(message("HTTP failed"))
                    .into_error(),
            );
            insta::assert_debug_snapshot!(explicit, "HTTP errors retain an explicit retryable source", @"
            Http(
                HTTP failed
                |
                └─ retry me,
            )
            ");
            assert!(gix_error::classify(&explicit).can_retry());
            assert!(gix_error::Error::from(explicit).is_retryable());

            let out_of_memory = super::Error::Http(
                std::io::Error::from(std::io::ErrorKind::OutOfMemory)
                    .and_raise(message("HTTP failed"))
                    .into_error(),
            );
            insta::assert_debug_snapshot!(out_of_memory, "HTTP errors retain the allocation failure as their source", @"
            Http(
                HTTP failed
                |
                └─ out of memory,
            )
            ");
            assert!(!gix_error::classify(&out_of_memory).can_retry());
            assert!(gix_error::classify(&out_of_memory).can_retry_lenient());
            assert!(gix_error::Error::from(out_of_memory).is_resource_exhausted());
        }

        #[test]
        fn custom_errors_expose_classifications() {
            use gix_error::Class;

            fn check<Cause: std::error::Error + 'static>(
                err: impl std::error::Error + Send + Sync + 'static,
                class: Class,
            ) -> gix_error::Exn {
                let err = err.raise();
                assert_eq!(
                    err.is_validation(),
                    class == Class::Validation,
                    "unsafe transport arguments are invalid input"
                );
                assert_eq!(
                    err.is_corrupted(),
                    class == Class::Corruption,
                    "unexpected packet lines are malformed responses"
                );
                assert!(
                    err.downcast_any_ref::<Cause>().is_some(),
                    "the concrete transport error remains available"
                );
                assert!(
                    err.probable_cause().is::<Cause>(),
                    "the concrete transport error, not its classification marker, is the probable cause"
                );
                err.erased()
            }

            let mut diagnostics = vec![check::<super::Error>(
                super::Error::AmbiguousPath { path: "-arg".into() },
                Class::Validation,
            )];
            for err in [super::Error::ExpectedLine("version"), super::Error::ExpectedDataLine] {
                diagnostics.push(check::<super::Error>(err, Class::Corruption));
            }
            insta::assert_debug_snapshot!(diagnostics, "unsafe paths and malformed protocol lines retain their concrete diagnostics", @"
            [
                The repository path '-arg' could be mistaken for a command-line argument,
                A version line was expected, but there was none,
                Expected a data line, but got a delimiter,
            ]
            ");

            #[cfg(feature = "blocking-client")]
            {
                use crate::client::blocking_io::ssh;

                let mut diagnostics = Vec::new();
                for err in [
                    ssh::invocation::Error::AmbiguousUserName { user: "-arg".into() },
                    ssh::invocation::Error::AmbiguousHostName { host: "-arg".into() },
                ] {
                    diagnostics.push(check::<ssh::invocation::Error>(
                        super::Error::SshInvocation(err),
                        Class::Validation,
                    ));
                }
                diagnostics.push(check::<ssh::Error>(
                    ssh::Error::AmbiguousHostName { host: "-arg".into() },
                    Class::Validation,
                ));
                insta::assert_debug_snapshot!(diagnostics, "SSH invocation context preserves the rejected argument as its cause", @"
                [
                    Failed to prepare SSH invocation
                    |
                    └─ Username '-arg' could be mistaken for a command-line argument,
                    Failed to prepare SSH invocation
                    |
                    └─ Host name '-arg' could be mistaken for a command-line argument,
                    Host name '-arg' could be mistaken for a command-line argument,
                ]
                ");
            }
        }
    }
}

pub use error::{AuthenticationRequired, Error};
