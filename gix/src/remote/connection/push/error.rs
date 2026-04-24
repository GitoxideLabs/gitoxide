/// Errors from [`super::Prepare::transmit`] and
/// [`Connection::prepare_push`](crate::remote::Connection::prepare_push).
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    // ── prepare_push (handshake) errors ───────────────────────────────────
    #[error("failed to configure the transport before connecting to {url:?}")]
    GatherTransportConfig {
        url: crate::bstr::BString,
        source: crate::config::transport::Error,
    },
    #[error("failed to configure the transport layer")]
    ConfigureTransport(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),
    #[error(transparent)]
    Handshake(#[from] gix_protocol::handshake::Error),
    #[error(transparent)]
    ConfigureCredentials(#[from] crate::config::credential_helpers::Error),
    // ── transmit errors ───────────────────────────────────────────────────
    #[error("parse refspec")]
    ParseRefspec(#[from] gix_refspec::parse::Error),
    #[error("enumerate local refs")]
    References(#[from] crate::reference::iter::Error),
    #[error("initialize local-ref iterator")]
    ReferencesInit(#[from] crate::reference::iter::init::Error),
    #[error("resolve local ref to object id")]
    Resolve(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
    #[error("revision walk: {0}")]
    Walk(String),
    #[error("build pack entries: {0}")]
    Pack(String),
    #[error("send-pack")]
    SendPack(#[from] gix_protocol::send_pack::Error),
    #[error("no refspecs given — nothing to push")]
    NoRefspecs,
    #[error("push destination conflict: {0}")]
    RefspecConflict(#[from] gix_refspec::match_group::validate::Error),
    #[error("refspec matched no local refs: {spec:?}")]
    NoMatch { spec: crate::bstr::BString },
    #[error("the receiving end does not support push options")]
    PushOptionsNotSupported,
    #[error("the receiving end does not support --signed push")]
    SignedPushNotSupported,
}
