/// The error returned by [`fetch()`](crate::fetch()).
#[derive(Debug)]
#[expect(missing_docs)]
pub enum Error {
    FetchResponse(crate::fetch::response::Error),
    Negotiate(gix_error::Error),
    Client(crate::transport::client::Error),
    MissingServerFeature {
        feature: &'static str,
        description: &'static str,
    },
    WriteShallowFile(gix_error::Error),
    ReadShallowFile(gix_error::Error),
    LockShallowFile(gix_lock::acquire::Error),
    RejectShallowRemote,
    ConsumePack(Box<dyn std::error::Error + Send + Sync + 'static>),
    ReadRemainingBytes(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::FetchResponse(_) => f.write_str("Could not decode server reply"),
            Error::Negotiate(err) => std::fmt::Display::fmt(err, f),
            Error::Client(err) => std::fmt::Display::fmt(err, f),
            Error::MissingServerFeature { feature, description } => {
                write!(f, "Server lack feature {feature:?}: {description}")
            }
            Error::WriteShallowFile(_) => {
                f.write_str("Could not write 'shallow' file to incorporate remote updates after fetching")
            }
            Error::ReadShallowFile(_) => f.write_str("Could not read 'shallow' file to send current shallow boundary"),
            Error::LockShallowFile(_) => {
                f.write_str("'shallow' file could not be locked in preparation for writing changes")
            }
            Error::RejectShallowRemote => f.write_str(
                "Receiving objects from shallow remotes is prohibited due to the value of `clone.rejectShallow`",
            ),
            Error::ConsumePack(_) => f.write_str("Failed to consume the pack sent by the remote"),
            Error::ReadRemainingBytes(_) => f.write_str("Failed to read remaining bytes in stream"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::FetchResponse(err) => Some(err),
            Error::Negotiate(err) => Some(err),
            Error::Client(err) => err.source(),
            Error::WriteShallowFile(err) | Error::ReadShallowFile(err) => Some(err),
            Error::LockShallowFile(err) => Some(err),
            Error::ConsumePack(err) => Some(err.as_ref()),
            Error::ReadRemainingBytes(err) => Some(err),
            _ => None,
        }
    }
}

impl Error {
    /// Return `true` if retrying the fetch might succeed.
    pub fn can_retry(&self) -> bool {
        use crate::transport::IsSpuriousError;
        match self {
            Error::FetchResponse(err) => err.can_retry(),
            Error::Client(err) => err.is_spurious(),
            _ => gix_error::can_retry(self),
        }
    }
}

impl From<crate::fetch::response::Error> for Error {
    fn from(err: crate::fetch::response::Error) -> Self {
        Error::FetchResponse(err)
    }
}

impl From<crate::fetch::negotiate::Error> for Error {
    fn from(err: crate::fetch::negotiate::Error) -> Self {
        Error::Negotiate(err.into_error())
    }
}

impl From<crate::transport::client::Error> for Error {
    fn from(err: crate::transport::client::Error) -> Self {
        Error::Client(err)
    }
}

impl From<gix_lock::acquire::Error> for Error {
    fn from(err: gix_lock::acquire::Error) -> Self {
        Error::LockShallowFile(err)
    }
}

#[cfg(test)]
mod tests {
    use gix_error::{ErrorExt, message};

    #[test]
    fn negotiation_keeps_retryable_sources() {
        let err = super::Error::from(
            std::io::Error::new(std::io::ErrorKind::TimedOut, "retry me").and_raise(message("negotiation failed")),
        );

        assert!(err.can_retry());
        let source = std::error::Error::source(&err)
            .and_then(|err| err.downcast_ref::<gix_error::Error>())
            .expect("negotiation errors retain their gix-error wrapper");
        assert!(
            source
                .sources()
                .any(<dyn std::error::Error + 'static>::is::<std::io::Error>)
        );
    }
}
