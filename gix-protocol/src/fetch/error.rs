/// The error returned by [`fetch()`](crate::fetch()).
// TODO(review): hand-written impls preserve the `thiserror` semantics. `Negotiate`/`Client` are
//                `#[error(transparent)]`; `ConsumePack` exposes its boxed source via `&**err`.
#[derive(Debug)]
#[expect(missing_docs)]
pub enum Error {
    FetchResponse(crate::fetch::response::Error),
    Negotiate(crate::fetch::negotiate::Error),
    Client(crate::transport::client::Error),
    MissingServerFeature {
        feature: &'static str,
        description: &'static str,
    },
    WriteShallowFile(gix_shallow::write::Error),
    ReadShallowFile(gix_shallow::read::Error),
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
            Error::Negotiate(err) => err.source(),
            Error::Client(err) => err.source(),
            Error::MissingServerFeature { .. } | Error::RejectShallowRemote => None,
            Error::WriteShallowFile(err) => Some(err),
            Error::ReadShallowFile(err) => Some(err),
            Error::LockShallowFile(err) => Some(err),
            Error::ConsumePack(err) => Some(&**err),
            Error::ReadRemainingBytes(err) => Some(err),
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
        Error::Negotiate(err)
    }
}

impl From<crate::transport::client::Error> for Error {
    fn from(err: crate::transport::client::Error) -> Self {
        Error::Client(err)
    }
}

impl From<gix_shallow::write::Error> for Error {
    fn from(err: gix_shallow::write::Error) -> Self {
        Error::WriteShallowFile(err)
    }
}

impl From<gix_shallow::read::Error> for Error {
    fn from(err: gix_shallow::read::Error) -> Self {
        Error::ReadShallowFile(err)
    }
}

impl From<gix_lock::acquire::Error> for Error {
    fn from(err: gix_lock::acquire::Error) -> Self {
        Error::LockShallowFile(err)
    }
}

impl crate::transport::IsSpuriousError for Error {
    fn is_spurious(&self) -> bool {
        match self {
            Error::FetchResponse(err) => err.is_spurious(),
            Error::Client(err) => err.is_spurious(),
            _ => false,
        }
    }
}
