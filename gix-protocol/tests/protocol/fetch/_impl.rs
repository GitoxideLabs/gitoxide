mod fetch_fn {
    use std::borrow::Cow;

    use gix_features::progress::NestedProgress;
    use gix_transport::client;
    use maybe_async::maybe_async;

    use super::{Action, Delegate};
    use gix_protocol::{
        credentials,
        fetch::{Arguments, Error, Response},
        indicate_end_of_interaction, Command,
    };

    /// A way to indicate how to treat the connection underlying the transport, potentially allowing to reuse it.
    #[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub enum FetchConnection {
        /// Use this variant if server should be informed that the operation is completed and no further commands will be issued
        /// at the end of the fetch operation or after deciding that no fetch operation should happen after references were listed.
        ///
        /// When indicating the end-of-fetch, this flag is only relevant in protocol V2.
        /// Generally it only applies when using persistent transports.
        ///
        /// In most explicit client side failure modes the end-of-operation' notification will be sent to the server automatically.
        #[default]
        TerminateOnSuccessfulCompletion,

        /// Indicate that persistent transport connections can be reused by _not_ sending an 'end-of-operation' notification to the server.
        /// This is useful if multiple `fetch(…)` calls are used in succession.
        ///
        /// Note that this has no effect in case of non-persistent connections, like the ones over HTTP.
        ///
        /// As an optimization, callers can use `AllowReuse` here as the server will also know the client is done
        /// if the connection is closed.
        AllowReuse,
    }

    /// Perform a 'fetch' operation with the server using `transport`, with `delegate` handling all server interactions.
    /// **Note** that `delegate` has blocking operations and thus this entire call should be on an executor which can handle
    /// that. This could be the current thread blocking, or another thread.
    ///
    /// * `authenticate(operation_to_perform)` is used to receive credentials for the connection and potentially store it
    ///   if the server indicates 'permission denied'. Note that not all transport support authentication or authorization.
    /// * `progress` is used to emit progress messages.
    /// * `name` is the name of the git client to present as `agent`, like `"my-app (v2.0)"`".
    /// * If `trace` is `true`, all packetlines received or sent will be passed to the facilities of the `gix-trace` crate.
    ///
    /// _Note_ that depending on the `delegate`, the actual action performed can be `ls-refs`, `clone` or `fetch`.
    ///
    /// # WARNING - Do not use!
    ///
    /// As it will hang when having multiple negotiation rounds.
    #[allow(clippy::result_large_err)]
    #[maybe_async]
    // TODO: remove this without losing test coverage - we have the same but better in `gix` and it's
    //       not really worth it to maintain the delegates here.
    pub async fn fetch<F, D, T, P>(
        mut transport: T,
        mut delegate: D,
        authenticate: F,
        mut progress: P,
        fetch_mode: FetchConnection,
        agent: impl Into<String>,
        trace: bool,
    ) -> Result<(), Error>
    where
        F: FnMut(credentials::helper::Action) -> credentials::protocol::Result,
        D: Delegate,
        T: client::Transport,
        P: NestedProgress + 'static,
        P::SubProgress: 'static,
    {
        let gix_protocol::handshake::Outcome {
            server_protocol_version: protocol_version,
            refs,
            v1_shallow_updates: _ignored_shallow_updates_as_it_is_deprecated,
            capabilities,
        } = gix_protocol::fetch::handshake(
            &mut transport,
            authenticate,
            delegate.handshake_extra_parameters(),
            &mut progress,
        )
        .await?;

        let agent = gix_protocol::agent(agent);
        let refs = match refs {
            Some(refs) => refs,
            None => {
                gix_protocol::ls_refs(
                    &mut transport,
                    &capabilities,
                    |a, b, c| {
                        let res = delegate.prepare_ls_refs(a, b, c);
                        c.push(("agent", Some(Cow::Owned(agent.clone()))));
                        res
                    },
                    &mut progress,
                    trace,
                )
                .await?
            }
        };

        let fetch = Command::Fetch;
        let mut fetch_features = fetch.default_features(protocol_version, &capabilities);
        match delegate.prepare_fetch(protocol_version, &capabilities, &mut fetch_features, &refs) {
            Ok(Action::Cancel) => {
                return if matches!(protocol_version, gix_transport::Protocol::V1)
                    || matches!(fetch_mode, FetchConnection::TerminateOnSuccessfulCompletion)
                {
                    indicate_end_of_interaction(transport, trace).await.map_err(Into::into)
                } else {
                    Ok(())
                };
            }
            Ok(Action::Continue) => {
                fetch
                    .validate_argument_prefixes(protocol_version, &capabilities, &[], &fetch_features)
                    .expect("BUG: delegates must always produce valid arguments");
            }
            Err(err) => {
                indicate_end_of_interaction(transport, trace).await?;
                return Err(err.into());
            }
        }

        Response::check_required_features(protocol_version, &fetch_features)?;
        let sideband_all = fetch_features.iter().any(|(n, _)| *n == "sideband-all");
        fetch_features.push(("agent", Some(Cow::Owned(agent))));
        let mut arguments = Arguments::new(protocol_version, fetch_features, trace);
        let mut previous_response = None::<Response>;
        let mut round = 1;
        'negotiation: loop {
            progress.step();
            progress.set_name(format!("negotiate (round {round})"));
            round += 1;
            let action = delegate.negotiate(&refs, &mut arguments, previous_response.as_ref())?;
            let mut reader = arguments.send(&mut transport, action == Action::Cancel).await?;
            if sideband_all {
                setup_remote_progress(&mut progress, &mut reader);
            }
            let response = Response::from_line_reader(
                protocol_version,
                &mut reader,
                true,  /* hack, telling us we don't want this delegate approach anymore */
                false, /* just as much of a hack which causes us to expect a pack immediately */
            )
            .await?;
            previous_response = if response.has_pack() {
                progress.step();
                progress.set_name("receiving pack".into());
                if !sideband_all {
                    setup_remote_progress(&mut progress, &mut reader);
                }
                delegate.receive_pack(reader, progress, &refs, &response).await?;
                break 'negotiation;
            } else {
                match action {
                    Action::Cancel => break 'negotiation,
                    Action::Continue => Some(response),
                }
            }
        }
        if matches!(protocol_version, gix_transport::Protocol::V2)
            && matches!(fetch_mode, FetchConnection::TerminateOnSuccessfulCompletion)
        {
            indicate_end_of_interaction(transport, trace).await?;
        }
        Ok(())
    }

    fn setup_remote_progress<P>(
        progress: &mut P,
        reader: &mut Box<dyn gix_transport::client::ExtendedBufRead<'_> + Unpin + '_>,
    ) where
        P: NestedProgress,
        P::SubProgress: 'static,
    {
        reader.set_progress_handler(Some(Box::new({
            let mut remote_progress = progress.add_child("remote");
            move |is_err: bool, data: &[u8]| {
                gix_protocol::RemoteProgress::translate_to_progress(is_err, data, &mut remote_progress);
                gix_transport::packetline::read::ProgressAction::Continue
            }
        }) as gix_transport::client::HandleProgress<'_>));
    }
}
pub use fetch_fn::{fetch, FetchConnection};

mod delegate {
    use std::{
        borrow::Cow,
        io,
        ops::{Deref, DerefMut},
    };

    use bstr::BString;
    use gix_transport::client::Capabilities;

    use gix_protocol::ls_refs;
    use gix_protocol::{
        fetch::{Arguments, Response},
        handshake::Ref,
    };

    /// Defines what to do next after certain [`Delegate`] operations.
    #[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
    pub enum Action {
        /// Continue the typical flow of operations in this flow.
        Continue,
        /// Return at the next possible opportunity without making further requests, possibly after closing the connection.
        Cancel,
    }

    /// The non-IO protocol delegate is the bare minimal interface needed to fully control the [`fetch`][gix_protocol::fetch()] operation, sparing
    /// the IO parts.
    /// Async implementations must treat it as blocking and unblock it by evaluating it elsewhere.
    ///
    /// See [Delegate] for the complete trait.
    pub trait DelegateBlocking {
        /// Return extra parameters to be provided during the handshake.
        ///
        /// Note that this method is only called once and the result is reused during subsequent handshakes which may happen
        /// if there is an authentication failure.
        fn handshake_extra_parameters(&self) -> Vec<(String, Option<String>)> {
            Vec::new()
        }
        /// Called before invoking 'ls-refs' on the server to allow providing it with additional `arguments` and to enable `features`.
        /// If the server `capabilities` don't match the requirements abort with an error to abort the entire fetch operation.
        ///
        /// Note that some arguments are preset based on typical use, and `features` are preset to maximize options.
        /// The `server` capabilities can be used to see which additional capabilities the server supports as per the handshake which happened prior.
        ///
        /// If the delegate returns [`ls_refs::Action::Skip`], no `ls-refs` command is sent to the server.
        ///
        /// Note that this is called only if we are using protocol version 2.
        fn prepare_ls_refs(
            &mut self,
            _server: &Capabilities,
            _arguments: &mut Vec<BString>,
            _features: &mut Vec<(&str, Option<Cow<'_, str>>)>,
        ) -> std::io::Result<ls_refs::Action> {
            Ok(ls_refs::Action::Continue)
        }

        /// Called before invoking the 'fetch' interaction with `features` pre-filled for typical use
        /// and to maximize capabilities to allow aborting an interaction early.
        ///
        /// `refs` is a list of known references on the remote based on the handshake or a prior call to `ls_refs`.
        /// These can be used to abort early in case the refs are already known here.
        ///
        /// As there will be another call allowing to post arguments conveniently in the correct format, i.e. `want hex-oid`,
        /// there is no way to set arguments at this time.
        ///
        /// `version` is the actually supported version as reported by the server, which is relevant in case the server requested a downgrade.
        /// `server` capabilities is a list of features the server supports for your information, along with enabled `features` that the server knows about.
        fn prepare_fetch(
            &mut self,
            _version: gix_transport::Protocol,
            _server: &Capabilities,
            _features: &mut Vec<(&str, Option<Cow<'_, str>>)>,
            _refs: &[Ref],
        ) -> std::io::Result<Action> {
            Ok(Action::Continue)
        }

        /// A method called repeatedly to negotiate the objects to receive in [`receive_pack(…)`][Delegate::receive_pack()].
        ///
        /// The first call has `previous_response` set to `None` as there was no previous response. Every call that follows `previous_response`
        /// will be set to `Some`.
        ///
        /// ### If `previous_response` is `None`…
        ///
        /// Given a list of `arguments` to populate with wants, want-refs, shallows, filters and other contextual information to be
        /// sent to the server. This method is called once.
        /// Send the objects you `have` have afterwards based on the tips of your refs, in preparation to walk down their parents
        /// with each call to `negotiate` to find the common base(s).
        ///
        /// Note that you should not `want` and object that you already have.
        /// `refs` are the tips of on the server side, effectively the latest objects _they_ have.
        ///
        /// Return `Action::Close` if you know that there are no `haves` on your end to allow the server to send all of its objects
        /// as is the case during initial clones.
        ///
        /// ### If `previous_response` is `Some`…
        ///
        /// Populate `arguments` with the objects you `have` starting from the tips of _your_ refs, taking into consideration
        /// the `previous_response` response of the server to see which objects they acknowledged to have. You have to maintain
        /// enough state to be able to walk down from your tips on each call, if they are not in common, and keep setting `have`
        /// for those which are in common if that helps teaching the server about our state and to acknowledge their existence on _their_ end.
        /// This method is called until the other side signals they are ready to send a pack.
        /// Return `Action::Close` if you want to give up before finding a common base. This can happen if the remote repository
        /// has radically changed so there are no bases, or they are very far in the past, causing all objects to be sent.
        fn negotiate(
            &mut self,
            refs: &[Ref],
            arguments: &mut Arguments,
            previous_response: Option<&Response>,
        ) -> io::Result<Action>;
    }

    impl<T: DelegateBlocking> DelegateBlocking for Box<T> {
        fn handshake_extra_parameters(&self) -> Vec<(String, Option<String>)> {
            self.deref().handshake_extra_parameters()
        }

        fn prepare_ls_refs(
            &mut self,
            _server: &Capabilities,
            _arguments: &mut Vec<BString>,
            _features: &mut Vec<(&str, Option<Cow<'_, str>>)>,
        ) -> io::Result<ls_refs::Action> {
            self.deref_mut().prepare_ls_refs(_server, _arguments, _features)
        }

        fn prepare_fetch(
            &mut self,
            _version: gix_transport::Protocol,
            _server: &Capabilities,
            _features: &mut Vec<(&str, Option<Cow<'_, str>>)>,
            _refs: &[Ref],
        ) -> io::Result<Action> {
            self.deref_mut().prepare_fetch(_version, _server, _features, _refs)
        }

        fn negotiate(
            &mut self,
            refs: &[Ref],
            arguments: &mut Arguments,
            previous_response: Option<&Response>,
        ) -> io::Result<Action> {
            self.deref_mut().negotiate(refs, arguments, previous_response)
        }
    }

    impl<T: DelegateBlocking> DelegateBlocking for &mut T {
        fn handshake_extra_parameters(&self) -> Vec<(String, Option<String>)> {
            self.deref().handshake_extra_parameters()
        }

        fn prepare_ls_refs(
            &mut self,
            _server: &Capabilities,
            _arguments: &mut Vec<BString>,
            _features: &mut Vec<(&str, Option<Cow<'_, str>>)>,
        ) -> io::Result<ls_refs::Action> {
            self.deref_mut().prepare_ls_refs(_server, _arguments, _features)
        }

        fn prepare_fetch(
            &mut self,
            _version: gix_transport::Protocol,
            _server: &Capabilities,
            _features: &mut Vec<(&str, Option<Cow<'_, str>>)>,
            _refs: &[Ref],
        ) -> io::Result<Action> {
            self.deref_mut().prepare_fetch(_version, _server, _features, _refs)
        }

        fn negotiate(
            &mut self,
            refs: &[Ref],
            arguments: &mut Arguments,
            previous_response: Option<&Response>,
        ) -> io::Result<Action> {
            self.deref_mut().negotiate(refs, arguments, previous_response)
        }
    }

    #[cfg(feature = "blocking-client")]
    mod blocking_io {
        use std::{
            io::{self, BufRead},
            ops::DerefMut,
        };

        use gix_features::progress::NestedProgress;

        use super::DelegateBlocking;
        use gix_protocol::{fetch::Response, handshake::Ref};

        /// The protocol delegate is the bare minimal interface needed to fully control the [`fetch`][gix_protocol::fetch()] operation.
        ///
        /// Implementations of this trait are controlled by code with intricate knowledge about how fetching works in protocol version V1 and V2,
        /// so you don't have to.
        /// Everything is tucked away behind type-safety so 'nothing can go wrong'©. Runtime assertions assure invalid
        /// features or arguments don't make it to the server in the first place.
        /// Please note that this trait mostly corresponds to what V2 would look like, even though V1 is supported as well.
        pub trait Delegate: DelegateBlocking {
            /// Receive a pack provided from the given `input`.
            ///
            /// Use `progress` to emit your own progress messages when decoding the pack.
            ///
            /// `refs` of the remote side are provided for convenience, along with the parsed `previous_response` response in case you want
            /// to check additional acks.
            fn receive_pack(
                &mut self,
                input: impl io::BufRead,
                progress: impl NestedProgress + 'static,
                refs: &[Ref],
                previous_response: &Response,
            ) -> io::Result<()>;
        }

        impl<T: Delegate> Delegate for Box<T> {
            fn receive_pack(
                &mut self,
                input: impl BufRead,
                progress: impl NestedProgress + 'static,
                refs: &[Ref],
                previous_response: &Response,
            ) -> io::Result<()> {
                self.deref_mut().receive_pack(input, progress, refs, previous_response)
            }
        }

        impl<T: Delegate> Delegate for &mut T {
            fn receive_pack(
                &mut self,
                input: impl BufRead,
                progress: impl NestedProgress + 'static,
                refs: &[Ref],
                previous_response: &Response,
            ) -> io::Result<()> {
                self.deref_mut().receive_pack(input, progress, refs, previous_response)
            }
        }
    }
    #[cfg(feature = "blocking-client")]
    pub use blocking_io::Delegate;

    #[cfg(feature = "async-client")]
    mod async_io {
        use std::{io, ops::DerefMut};

        use async_trait::async_trait;
        use futures_io::AsyncBufRead;
        use gix_features::progress::NestedProgress;

        use super::DelegateBlocking;
        use gix_protocol::{fetch::Response, handshake::Ref};

        /// The protocol delegate is the bare minimal interface needed to fully control the [`fetch`][gix_protocol::fetch()] operation.
        ///
        /// Implementations of this trait are controlled by code with intricate knowledge about how fetching works in protocol version V1 and V2,
        /// so you don't have to.
        /// Everything is tucked away behind type-safety so 'nothing can go wrong'©. Runtime assertions assure invalid
        /// features or arguments don't make it to the server in the first place.
        /// Please note that this trait mostly corresponds to what V2 would look like, even though V1 is supported as well.
        #[async_trait(?Send)]
        pub trait Delegate: DelegateBlocking {
            /// Receive a pack provided from the given `input`, and the caller should consider it to be blocking as
            /// most operations on the received pack are implemented in a blocking fashion.
            ///
            /// Use `progress` to emit your own progress messages when decoding the pack.
            ///
            /// `refs` of the remote side are provided for convenience, along with the parsed `previous_response` response in case you want
            /// to check additional acks.
            async fn receive_pack(
                &mut self,
                input: impl AsyncBufRead + Unpin + 'async_trait,
                progress: impl NestedProgress + 'static,
                refs: &[Ref],
                previous_response: &Response,
            ) -> io::Result<()>;
        }
        #[async_trait(?Send)]
        impl<T: Delegate> Delegate for Box<T> {
            async fn receive_pack(
                &mut self,
                input: impl AsyncBufRead + Unpin + 'async_trait,
                progress: impl NestedProgress + 'static,
                refs: &[Ref],
                previous_response: &Response,
            ) -> io::Result<()> {
                self.deref_mut()
                    .receive_pack(input, progress, refs, previous_response)
                    .await
            }
        }

        #[async_trait(?Send)]
        impl<T: Delegate> Delegate for &mut T {
            async fn receive_pack(
                &mut self,
                input: impl AsyncBufRead + Unpin + 'async_trait,
                progress: impl NestedProgress + 'static,
                refs: &[Ref],
                previous_response: &Response,
            ) -> io::Result<()> {
                self.deref_mut()
                    .receive_pack(input, progress, refs, previous_response)
                    .await
            }
        }
    }
    #[cfg(feature = "async-client")]
    pub use async_io::Delegate;
}
#[cfg(any(feature = "async-client", feature = "blocking-client"))]
pub use delegate::Delegate;
pub use delegate::{Action, DelegateBlocking};
