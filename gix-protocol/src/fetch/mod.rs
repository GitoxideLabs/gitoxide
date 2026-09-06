/// A module providing low-level primitives to flexibly perform various `fetch` related activities. Note that the typesystem isn't used
/// to assure they are always performed in the right order, the caller has to follow some parts of the protocol itself.
///
/// ### Order for receiving a pack
///
/// * [handshake](handshake())
/// * **ls-refs**
///     * [get available refs by refspecs](RefMap::fetch())
/// * **fetch pack**
///     * `negotiate` until a pack can be received (TBD)
/// * [officially terminate the connection](crate::indicate_end_of_interaction())
///     - Consider wrapping the transport in [`SendFlushOnDrop`](crate::SendFlushOnDrop) to be sure the connection is terminated
///       gracefully even if there is an application error.
///
/// Note that this flow doesn't involve actually writing the pack, or indexing it. Nor does it contain machinery
/// to write or update references based on the fetched remote references.
///
/// Also, when the server supports [version 2](crate::transport::Protocol::V2) of the protocol, then each of the listed commands,
/// `ls-refs` and `fetch` can be invoked multiple times in any order.
// Note: for ease of use, this is tested in `gix` itself. The test-suite here uses a legacy implementation.
mod arguments;
pub use arguments::Arguments;

///
pub mod response;

#[cfg(any(feature = "blocking-client", feature = "async-client"))]
#[cfg(feature = "fetch")]
pub(crate) mod function;

#[cfg(feature = "fetch")]
pub mod negotiate;

///
#[cfg(feature = "fetch")]
pub mod refmap;

mod types;
pub use types::*;
