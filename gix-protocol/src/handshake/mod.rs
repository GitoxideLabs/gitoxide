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
        use crate::{Handshake, command::Feature, fetch::RefMap, ls_refs::RefPrefixes};
        use gix_features::progress::Progress;

        /// Intermediate state while potentially fetching a refmap after the handshake.
        pub enum ObtainRefMap<'a> {
            /// We already got a refmap to use from the V1 handshake, which always sends them.
            Existing(RefMap),
            /// We need to invoke another `ls-refs` command to retrieve the refmap as part of the V2 protocol.
            LsRefsCommand(crate::LsRefsCommand<'a>, crate::fetch::refmap::init::Context),
        }

        macro_rules! fetch {
            ($name:ident, $bisync:path, $transport:path, $invoke:ident, $mode:literal) => {
                /// Fetch the refmap, either by returning the existing one or invoking the `ls-refs` command.
                #[$bisync]
                pub async fn $name(
                    self,
                    mut progress: impl Progress,
                    transport: &mut impl $transport,
                    trace_packetlines: bool,
                ) -> Result<RefMap, crate::fetch::refmap::init::Error> {
                    let (cmd, cx) = match self {
                        ObtainRefMap::Existing(map) => return Ok(map),
                        ObtainRefMap::LsRefsCommand(cmd, cx) => (cmd, cx),
                    };

                    let _span = gix_trace::coarse!(
                        "gix_protocol::handshake::ObtainRefMap::fetch()",
                        mode = $mode
                    );
                    let capabilities = cmd.capabilities;
                    let remote_refs = cmd
                        .$invoke(transport, &mut progress, trace_packetlines)
                        .await?;
                    RefMap::from_refs(remote_refs, capabilities, cx)
                }
            };
        }

        impl ObtainRefMap<'_> {
            #[cfg(feature = "async-client")]
            fetch!(
                fetch_async,
                ::bisync::asynchronous::bisync,
                crate::transport::client::async_io::Transport,
                invoke_async,
                "async"
            );

            #[cfg(feature = "blocking-client")]
            fetch!(
                fetch_blocking,
                ::bisync::synchronous::bisync,
                crate::transport::client::blocking_io::Transport,
                invoke_blocking,
                "blocking"
            );
        }

        impl Handshake {
            /// Prepare fetching a [refmap](RefMap) if not present in the handshake.
            pub fn prepare_lsrefs_or_extract_refmap(
                &mut self,
                user_agent: Feature,
                prefix_from_spec_as_filter_on_remote: bool,
                refmap_context: crate::fetch::refmap::init::Context,
            ) -> Result<ObtainRefMap<'_>, crate::fetch::refmap::init::Error> {
                if let Some(refs) = self.refs.take() {
                    return Ok(ObtainRefMap::Existing(RefMap::from_refs(
                        refs,
                        &self.capabilities,
                        refmap_context,
                    )?));
                }

                let prefix_refs = prefix_from_spec_as_filter_on_remote.then(|| {
                    let all_refspecs = refmap_context.aggregate_refspecs();
                    RefPrefixes::from_refspecs(&all_refspecs)
                });
                Ok(ObtainRefMap::LsRefsCommand(
                    crate::LsRefsCommand::new(prefix_refs, &self.capabilities, user_agent),
                    refmap_context,
                ))
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
    // TODO(review): hand-written impls preserve the `thiserror` semantics. `Transport`/`ParseRefs`
    //                are `#[error(transparent)]`; `Credentials` (`#[from]`) and the `source`-named
    //                field of `InvalidCredentials` surface as `source()` like before.
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        Credentials(credentials::protocol::Error),
        EmptyCredentials,
        InvalidCredentials { url: BString, source: std::io::Error },
        Transport(client::Error),
        TransportProtocolPolicyViolation { actual_version: gix_transport::Protocol },
        ParseRefs(refs::parse::Error),
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Credentials(_) => f.write_str("Failed to obtain credentials"),
                Error::EmptyCredentials => f.write_str(
                    "No credentials were returned at all as if the credential helper isn't functioning unknowingly",
                ),
                Error::InvalidCredentials { url, .. } => {
                    write!(f, "Credentials provided for \"{url}\" were not accepted by the remote")
                }
                Error::Transport(err) => std::fmt::Display::fmt(err, f),
                Error::TransportProtocolPolicyViolation { actual_version } => write!(
                    f,
                    "The transport didn't accept the advertised server version {actual_version:?} and closed the connection client side"
                ),
                Error::ParseRefs(err) => std::fmt::Display::fmt(err, f),
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::Credentials(err) => Some(err),
                Error::EmptyCredentials | Error::TransportProtocolPolicyViolation { .. } => None,
                Error::InvalidCredentials { source, .. } => Some(source),
                Error::Transport(err) => err.source(),
                Error::ParseRefs(err) => err.source(),
            }
        }
    }

    impl From<credentials::protocol::Error> for Error {
        fn from(err: credentials::protocol::Error) -> Self {
            Error::Credentials(err)
        }
    }

    impl From<client::Error> for Error {
        fn from(err: client::Error) -> Self {
            Error::Transport(err)
        }
    }

    impl From<refs::parse::Error> for Error {
        fn from(err: refs::parse::Error) -> Self {
            Error::ParseRefs(err)
        }
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
