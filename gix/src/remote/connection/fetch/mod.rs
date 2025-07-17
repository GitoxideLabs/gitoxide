#[cfg(feature = "async-network-client")]
use gix_transport::client::async_io::Transport;
#[cfg(feature = "blocking-network-client")]
use gix_transport::client::blocking_io::Transport;

use crate::{
    bstr::BString,
    remote,
    remote::{
        fetch::{DryRun, RefMap},
        ref_map, Connection,
    },
    Progress,
};
use gix_hash::ObjectId;

mod error;
pub use error::Error;

use crate::remote::fetch::WritePackedRefs;

/// The way reflog messages should be composed whenever a ref is written with recent objects from a remote.
pub enum RefLogMessage {
    /// Prefix the log with `action` and generate the typical suffix as `git` would.
    Prefixed {
        /// The action to use, like `fetch` or `pull`.
        action: String,
    },
    /// Control the entire message, using `message` verbatim.
    Override {
        /// The complete reflog message.
        message: BString,
    },
}

impl RefLogMessage {
    pub(crate) fn compose(&self, context: &str) -> BString {
        match self {
            RefLogMessage::Prefixed { action } => format!("{action}: {context}").into(),
            RefLogMessage::Override { message } => message.to_owned(),
        }
    }
}

/// The status of the repository after the fetch operation
#[derive(Debug, Clone)]
pub enum Status {
    /// Nothing changed as the remote didn't have anything new compared to our tracking branches, thus no pack was received
    /// and no new object was added.
    ///
    /// As we could determine that nothing changed without remote interaction, there was no negotiation at all.
    NoPackReceived {
        /// If `true`, we didn't receive a pack due to dry-run mode being enabled.
        dry_run: bool,
        /// Information about the pack negotiation phase if negotiation happened at all.
        ///
        /// It's possible that negotiation didn't have to happen as no reference of interest changed on the server.
        negotiate: Option<outcome::Negotiate>,
        /// However, depending on the refspecs, references might have been updated nonetheless to point to objects as
        /// reported by the remote.
        update_refs: refs::update::Outcome,
    },
    /// There was at least one tip with a new object which we received.
    Change {
        /// Information about the pack negotiation phase.
        negotiate: outcome::Negotiate,
        /// Information collected while writing the pack and its index.
        write_pack_bundle: gix_pack::bundle::write::Outcome,
        /// Information collected while updating references.
        update_refs: refs::update::Outcome,
    },
}

/// The outcome of receiving a pack via [`Prepare::receive()`].
#[derive(Debug, Clone)]
pub struct Outcome {
    /// The result of the initial mapping of references, the prerequisite for any fetch.
    pub ref_map: RefMap,
    /// The outcome of the handshake with the server.
    pub handshake: gix_protocol::Handshake,
    /// The status of the operation to indicate what happened.
    pub status: Status,
}

/// Additional types related to the outcome of a fetch operation.
pub mod outcome {
    /// Information about the negotiation phase of a fetch.
    ///
    /// Note that negotiation can happen even if no pack is ultimately produced.
    #[derive(Default, Debug, Clone)]
    pub struct Negotiate {
        /// The negotiation graph indicating what kind of information 'the algorithm' collected in the end.
        pub graph: gix_negotiate::IdMap,
        /// Additional information for each round of negotiation.
        pub rounds: Vec<gix_protocol::fetch::negotiate::Round>,
    }
}

pub use gix_protocol::fetch::ProgressId;

///
pub mod prepare {
    /// The error returned by [`prepare_fetch()`][super::Connection::prepare_fetch()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Cannot perform a meaningful fetch operation without any configured ref-specs")]
        MissingRefSpecs,
        #[error(transparent)]
        RefMap(#[from] crate::remote::ref_map::Error),
    }

    impl gix_protocol::transport::IsSpuriousError for Error {
        fn is_spurious(&self) -> bool {
            match self {
                Error::RefMap(err) => err.is_spurious(),
                _ => false,
            }
        }
    }
}

impl<'remote, 'repo, T> Connection<'remote, 'repo, T>
where
    T: Transport,
{
    /// Perform a handshake with the remote and obtain a ref-map with `options`, and from there one
    /// Note that at this point, the `transport` should already be configured using the [`transport_mut()`][Self::transport_mut()]
    /// method, as it will be consumed here.
    ///
    /// From there additional properties of the fetch can be adjusted to override the defaults that are configured via git-config.
    ///
    /// # Async Experimental
    ///
    /// Note that this implementation is currently limited correctly in blocking mode only as it relies on Drop semantics to close the connection
    /// should the fetch not be performed. Furthermore, there the code doing the fetch is inherently blocking and it's not offloaded to a thread,
    /// making this call block the executor.
    /// It's best to unblock it by placing it into its own thread or offload it should usage in an async context be truly required.
    #[allow(clippy::result_large_err)]
    #[gix_protocol::maybe_async::maybe_async]
    pub async fn prepare_fetch(
        mut self,
        progress: impl Progress,
        options: ref_map::Options,
    ) -> Result<Prepare<'remote, 'repo, T>, prepare::Error> {
        if self.remote.refspecs(remote::Direction::Fetch).is_empty() && options.extra_refspecs.is_empty() {
            return Err(prepare::Error::MissingRefSpecs);
        }
        let ref_map = self.ref_map_by_ref(progress, options).await?;
        Ok(Prepare {
            con: Some(self),
            ref_map,
            dry_run: DryRun::No,
            reflog_message: None,
            write_packed_refs: WritePackedRefs::Never,
            shallow: Default::default(),
            filter: None,
            additional_wants: Vec::new(),
        })
    }
}

impl<T> Prepare<'_, '_, T>
where
    T: Transport,
{
    /// Return the `ref_map` (that includes the server handshake) which was part of listing refs prior to fetching a pack.
    pub fn ref_map(&self) -> &RefMap {
        &self.ref_map
    }
}

mod config;
mod receive_pack;
///
#[path = "update_refs/mod.rs"]
pub mod refs;

/// A structure to hold the result of the handshake with the remote and configure the upcoming fetch operation.
pub struct Prepare<'remote, 'repo, T>
where
    T: Transport,
{
    con: Option<Connection<'remote, 'repo, T>>,
    ref_map: RefMap,
    dry_run: DryRun,
    reflog_message: Option<RefLogMessage>,
    write_packed_refs: WritePackedRefs,
    shallow: remote::fetch::Shallow,
    filter: Option<remote::fetch::ObjectFilter>,
    additional_wants: Vec<ObjectId>,
}

/// Builder
impl<T> Prepare<'_, '_, T>
where
    T: Transport,
{
    /// If dry run is enabled, no change to the repository will be made.
    ///
    /// This works by not actually fetching the pack after negotiating it, nor will refs be updated.
    pub fn with_dry_run(mut self, enabled: bool) -> Self {
        self.dry_run = if enabled { DryRun::Yes } else { DryRun::No };
        self
    }

    /// If enabled, don't write ref updates to loose refs, but put them exclusively to packed-refs.
    ///
    /// This improves performance and allows case-sensitive filesystems to deal with ref names that would otherwise
    /// collide.
    pub fn with_write_packed_refs_only(mut self, enabled: bool) -> Self {
        self.write_packed_refs = if enabled {
            WritePackedRefs::Only
        } else {
            WritePackedRefs::Never
        };
        self
    }

    /// Set the reflog message to use when updating refs after fetching a pack.
    pub fn with_reflog_message(mut self, reflog_message: RefLogMessage) -> Self {
        self.reflog_message = reflog_message.into();
        self
    }

    /// Define what to do when the current repository is a shallow clone.
    ///
    /// *Has no effect if the current repository is not as shallow clone.*
    pub fn with_shallow(mut self, shallow: remote::fetch::Shallow) -> Self {
        self.shallow = shallow;
        self
    }

    /// Ask the server to apply `filter` when sending objects.
    pub fn with_filter(mut self, filter: Option<remote::fetch::ObjectFilter>) -> Self {
        self.filter = filter;
        self
    }

    /// Request that the server also sends the objects identified by the given object ids.
    ///
    /// Objects already present locally will be ignored during negotiation.
    pub fn with_additional_wants(mut self, wants: impl IntoIterator<Item = ObjectId>) -> Self {
        for want in wants {
            if !self.additional_wants.contains(&want) {
                self.additional_wants.push(want);
            }
        }
        self
    }
}
