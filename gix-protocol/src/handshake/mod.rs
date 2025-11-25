use bstr::BString;

///
pub mod refs;

/// A git reference, commonly referred to as 'ref', as returned by a git server before sending a pack.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Ref {
    /// A ref pointing to a `tag` object, which in turns points to an `object`, usually a commit
    Peeled {
        /// The name at which the ref is located, like `refs/tags/1.0`.
        full_ref_name: BString,
        /// The hash of the tag the ref points to.
        tag: gix_hash::ObjectId,
        /// The hash of the object the `tag` points to.
        object: gix_hash::ObjectId,
    },
    /// A ref pointing to a commit object
    Direct {
        /// The name at which the ref is located, like `refs/heads/main` or `refs/tags/v1.0` for lightweight tags.
        full_ref_name: BString,
        /// The hash of the object the ref points to.
        object: gix_hash::ObjectId,
    },
    /// A symbolic ref pointing to `target` ref, which in turn, ultimately after possibly following `tag`, points to an `object`
    Symbolic {
        /// The name at which the symbolic ref is located, like `HEAD`.
        full_ref_name: BString,
        /// The path of the ref the symbolic ref points to, like `refs/heads/main`.
        ///
        /// See issue [#205] for details
        ///
        /// [#205]: https://github.com/GitoxideLabs/gitoxide/issues/205
        target: BString,
        /// The hash of the annotated tag the ref points to, if present.
        ///
        /// Note that this field is also `None` if `full_ref_name` is a lightweight tag.
        tag: Option<gix_hash::ObjectId>,
        /// The hash of the object the `target` ref ultimately points to.
        object: gix_hash::ObjectId,
    },
    /// A ref is unborn on the remote and just points to the initial, unborn branch, as is the case in a newly initialized repository
    /// or dangling symbolic refs.
    Unborn {
        /// The name at which the ref is located, typically `HEAD`.
        full_ref_name: BString,
        /// The path of the ref the symbolic ref points to, like `refs/heads/main`, even though the `target` does not yet exist.
        target: BString,
    },
}

#[cfg(feature = "handshake")]
pub(crate) mod hero {
    use crate::handshake::Ref;

    /// The result of the [`handshake()`](crate::handshake()) function.
    #[derive(Default, Debug, Clone)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct Handshake {
        /// The protocol version the server responded with. It might have downgraded the desired version.
        pub server_protocol_version: gix_transport::Protocol,
        /// The references reported as part of the `Protocol::V1` handshake, or `None` otherwise as V2 requires a separate request.
        pub refs: Option<Vec<Ref>>,
        /// Shallow updates as part of the `Protocol::V1`, to shallow a particular object.
        /// Note that unshallowing isn't supported here.
        pub v1_shallow_updates: Option<Vec<crate::fetch::response::ShallowUpdate>>,
        /// The server capabilities.
        pub capabilities: gix_transport::client::Capabilities,
    }

    #[cfg(feature = "fetch")]
    mod fetch {
        #[cfg(feature = "async-client")]
        use crate::transport::client::async_io::Transport;
        #[cfg(feature = "blocking-client")]
        use crate::transport::client::blocking_io::Transport;
        use crate::{fetch::RefMap, Handshake};
        use gix_features::progress::Progress;
        use std::borrow::Cow;

        impl Handshake {
            /// Obtain a [refmap](crate::fetch::RefMap) either from this instance, taking it out in the process, if the handshake was
            /// created from a V1 connection, or use `transport` to fetch the refmap as a separate command invocation.
            #[allow(clippy::result_large_err)]
            #[maybe_async::maybe_async]
            pub async fn fetch_or_extract_refmap<T>(
                &mut self,
                mut progress: impl Progress,
                transport: &mut T,
                user_agent: (&'static str, Option<Cow<'static, str>>),
                trace_packetlines: bool,
                prefix_from_spec_as_filter_on_remote: bool,
                refmap_context: crate::fetch::refmap::init::Context,
            ) -> Result<crate::fetch::RefMap, crate::fetch::refmap::init::Error>
            where
                T: Transport,
            {
                if let Some(refs) = self.refs.take() {
                    return crate::fetch::RefMap::from_refs(refs, &self.capabilities, refmap_context);
                }

                let _span = gix_trace::coarse!("gix_protocol::handshake::fetch_or_extract_refmap()");
                let all_refspecs = refmap_context.aggregate_refspecs();
                let prefix_refspecs = prefix_from_spec_as_filter_on_remote.then_some(&all_refspecs[..]);
                let remote_refs = crate::LsRefsCommand::new(prefix_refspecs, &self.capabilities, user_agent)
                    .invoke(transport, &mut progress, trace_packetlines)
                    .await?;
                RefMap::from_refs(remote_refs, &self.capabilities, refmap_context)
            }
        }
    }
}

#[cfg(feature = "handshake")]
mod error {
    use bstr::BString;
    use gix_transport::client;

    use crate::{credentials, handshake::refs};

    /// The error returned by [`handshake()`][crate::handshake()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Failed to obtain credentials")]
        Credentials(#[from] credentials::protocol::Error),
        #[error("No credentials were returned at all as if the credential helper isn't functioning unknowingly")]
        EmptyCredentials,
        #[error("Credentials provided for \"{url}\" were not accepted by the remote")]
        InvalidCredentials { url: BString, source: std::io::Error },
        #[error(transparent)]
        Transport(#[from] client::Error),
        #[error("The transport didn't accept the advertised server version {actual_version:?} and closed the connection client side")]
        TransportProtocolPolicyViolation { actual_version: gix_transport::Protocol },
        #[error(transparent)]
        ParseRefs(#[from] refs::parse::Error),
    }

    impl gix_transport::IsSpuriousError for Error {
        fn is_spurious(&self) -> bool {
            match self {
                Error::Transport(err) => err.is_spurious(),
                _ => false,
            }
        }
    }
}
#[cfg(feature = "handshake")]
pub use error::Error;

#[cfg(any(feature = "blocking-client", feature = "async-client"))]
#[cfg(feature = "handshake")]
pub(crate) mod function;
