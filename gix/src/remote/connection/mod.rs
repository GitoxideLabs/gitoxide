#[cfg(feature = "async-network-client")]
use gix_transport::client::async_io::Transport;
#[cfg(feature = "blocking-network-client")]
use gix_transport::client::blocking_io::Transport;

use crate::Remote;

/// A function that performs a given credential action, trying to obtain credentials for an operation that needs it.
pub type AuthenticateFn<'a> = Box<dyn FnMut(gix_credentials::helper::Action) -> gix_credentials::protocol::Result + 'a>;

/// A type to represent an ongoing connection to a remote host, typically with the connection already established.
///
/// It can be used to perform a variety of operations with the remote without worrying about protocol details,
/// much like a remote procedure call.
pub struct Connection<'a, 'repo, T>
where
    T: Transport,
{
    pub(crate) remote: &'a Remote<'repo>,
    pub(crate) authenticate: Option<AuthenticateFn<'a>>,
    pub(crate) transport_options: Option<Box<dyn std::any::Any>>,
    pub(crate) transport: gix_protocol::SendFlushOnDrop<T>,
    pub(crate) handshake: Option<gix_protocol::Handshake>,
    pub(crate) trace: bool,
}

mod access;

///
pub mod ref_map;

///
pub mod fetch;

///
#[cfg(feature = "blocking-network-client")]
pub mod push;
