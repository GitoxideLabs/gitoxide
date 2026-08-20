/// The error returned by [`fetch()`](crate::fetch()).
#[derive(Debug, thiserror::Error)]
#[expect(missing_docs)]
pub enum Error {
    #[error("Could not decode server reply")]
    FetchResponse(#[from] crate::fetch::response::Error),
    #[error(transparent)]
    Negotiate(#[from] crate::fetch::negotiate::Error),
    #[error(transparent)]
    Client(#[from] crate::transport::client::Error),
    #[error("Server lack feature {feature:?}: {description}")]
    MissingServerFeature {
        feature: &'static str,
        description: &'static str,
    },
    #[error("Could not write 'shallow' file to incorporate remote updates after fetching")]
    WriteShallowFile(#[source] gix_error::Error),
    #[error("Could not read 'shallow' file to send current shallow boundary")]
    ReadShallowFile(#[source] gix_error::Error),
    #[error("'shallow' file could not be locked in preparation for writing changes")]
    LockShallowFile(#[source] std::io::Error),
    #[error("Receiving objects from shallow remotes is prohibited due to the value of `clone.rejectShallow`")]
    RejectShallowRemote,
    #[error("Failed to consume the pack sent by the remote")]
    ConsumePack(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
    #[error("Failed to read remaining bytes in stream")]
    ReadRemainingBytes(#[source] std::io::Error),
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
