use std::{
    any::Any,
    borrow::Cow,
    ops::{Deref, DerefMut},
};

use bstr::BStr;

use crate::{client::Error, Protocol};

/// This trait represents all transport related functions that don't require any input/output to be done which helps
/// implementation to share more code across blocking and async programs.
pub trait TransportWithoutIO {
    /// If the handshake or subsequent reads failed with [`std::io::ErrorKind::PermissionDenied`], use this method to
    /// inform the transport layer about the identity to use for subsequent calls.
    /// If authentication continues to fail even with an identity set, consider communicating this to the provider
    /// of the identity in order to mark it as invalid. Otherwise the user might have difficulty updating obsolete
    /// credentials.
    /// Please note that most transport layers are unauthenticated and thus return [an error][Error::AuthenticationUnsupported] here.
    fn set_identity(&mut self, _identity: gix_sec::identity::Account) -> Result<(), Error> {
        Err(Error::AuthenticationUnsupported)
    }

    /// Returns the canonical URL pointing to the destination of this transport.
    fn to_url(&self) -> Cow<'_, BStr>;

    /// If the actually advertised server version is contained in the returned slice or it is empty, continue as normal,
    /// assume the server's protocol version is desired or acceptable.
    ///
    /// Otherwise, abort the fetch operation with an error to avoid continuing any interaction with the transport.
    ///
    /// In V1 this means a potentially large list of advertised refs won't be read, instead the connection is ignored
    /// leaving the server with a client who potentially unexpectedly terminated the connection.
    ///
    /// Note that `transport.close()` is not called explicitly.
    ///
    /// Custom transports can override this to prevent any use of older protocol versions.
    fn supported_protocol_versions(&self) -> &[Protocol] {
        &[]
    }

    /// Returns true if the transport provides persistent connections across multiple requests, or false otherwise.
    /// Not being persistent implies that certain information has to be resent on each 'turn'
    /// of the fetch negotiation or that the end of interaction (i.e. no further request will be made) has to be indicated
    /// to the server for most graceful termination of the connection.
    fn connection_persists_across_multiple_requests(&self) -> bool;

    /// Pass `config` can be cast and interpreted by the implementation, as documented separately.
    ///
    /// The caller must know how that `config` data looks like for the intended implementation.
    fn configure(&mut self, config: &dyn Any) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>;
}

// Would be nice if the box implementation could auto-forward to all implemented traits.
impl<T: TransportWithoutIO + ?Sized> TransportWithoutIO for Box<T> {
    fn set_identity(&mut self, identity: gix_sec::identity::Account) -> Result<(), Error> {
        self.deref_mut().set_identity(identity)
    }

    fn to_url(&self) -> Cow<'_, BStr> {
        self.deref().to_url()
    }

    fn supported_protocol_versions(&self) -> &[Protocol] {
        self.deref().supported_protocol_versions()
    }

    fn connection_persists_across_multiple_requests(&self) -> bool {
        self.deref().connection_persists_across_multiple_requests()
    }

    fn configure(&mut self, config: &dyn Any) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        self.deref_mut().configure(config)
    }
}

impl<T: TransportWithoutIO + ?Sized> TransportWithoutIO for &mut T {
    fn set_identity(&mut self, identity: gix_sec::identity::Account) -> Result<(), Error> {
        self.deref_mut().set_identity(identity)
    }

    fn to_url(&self) -> Cow<'_, BStr> {
        self.deref().to_url()
    }

    fn supported_protocol_versions(&self) -> &[Protocol] {
        self.deref().supported_protocol_versions()
    }

    fn connection_persists_across_multiple_requests(&self) -> bool {
        self.deref().connection_persists_across_multiple_requests()
    }

    fn configure(&mut self, config: &dyn Any) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        self.deref_mut().configure(config)
    }
}
