mod arguments;
pub use arguments::Arguments;

mod error;
pub use error::Error;
///
pub mod response;
pub use response::Response;

mod handshake;
pub use handshake::upload_pack as handshake;

mod refmap;

/// A type to represent an ongoing connection to a remote host, typically with the connection already established.
///
/// It can be used to perform a variety of operations with the remote without worrying about protocol details,
/// much like a remote procedure call.
pub struct Connection<'a, T> {
    // TODO: figure out how to abstract `Remote`.
    pub(crate) authenticate: Option<crate::AuthenticateFn<'a>>,
    pub(crate) transport_options: Option<Box<dyn std::any::Any>>,
    pub(crate) transport: T,
    pub(crate) trace: bool,
}

/// Information about the relationship between our refspecs, and remote references with their local counterparts.
#[derive(Default, Debug, Clone)]
#[cfg(any(feature = "blocking-client", feature = "async-client"))]
pub struct RefMap {
    /// A mapping between a remote reference and a local tracking branch.
    pub mappings: Vec<refmap::Mapping>,
    /// Refspecs which have been added implicitly due to settings of the `remote`, possibly pre-initialized from
    /// [`extra_refspecs` in RefMap options][crate::remote::ref_map::Options::extra_refspecs].
    ///
    /// They are never persisted nor are they typically presented to the user.
    pub extra_refspecs: Vec<gix_refspec::RefSpec>,
    /// Information about the fixes applied to the `mapping` due to validation and sanitization.
    pub fixes: Vec<gix_refspec::match_group::validate::Fix>,
    /// All refs advertised by the remote.
    pub remote_refs: Vec<crate::handshake::Ref>,
    /// Additional information provided by the server as part of the handshake.
    ///
    /// Note that the `refs` field is always `None` as the refs are placed in `remote_refs`.
    pub handshake: crate::handshake::Outcome,
    /// The kind of hash used for all data sent by the server, if understood by this client implementation.
    ///
    /// It was extracted from the `handshake` as advertised by the server.
    pub object_hash: gix_hash::Kind,
}
