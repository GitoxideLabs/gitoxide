mod error {
    use crate::handshake::refs::parse;

    /// The error returned by [`ls_refs()`][crate::ls_refs()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        Io(#[from] std::io::Error),
        #[error(transparent)]
        Transport(#[from] gix_transport::client::Error),
        #[error(transparent)]
        Parse(#[from] parse::Error),
        #[error(transparent)]
        ArgumentValidation(#[from] crate::command::validate_argument_prefixes::Error),
    }

    impl gix_transport::IsSpuriousError for Error {
        fn is_spurious(&self) -> bool {
            match self {
                Error::Io(err) => err.is_spurious(),
                Error::Transport(err) => err.is_spurious(),
                _ => false,
            }
        }
    }
}
pub use error::Error;

/// What to do after preparing ls-refs in [`ls_refs()`][crate::ls_refs()].
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub enum Action {
    /// Continue by sending a 'ls-refs' command.
    Continue,
    /// Skip 'ls-refs' entirely.
    ///
    /// This is useful if the `ref-in-want` capability is taken advantage of. When fetching, one must must then send
    /// `want-ref`s during the negotiation phase.
    Skip,
}

pub(crate) mod function {
    use std::borrow::Cow;

    use bstr::BString;
    use gix_features::progress::Progress;
    use gix_transport::client::{Capabilities, Transport, TransportV2Ext};
    use maybe_async::maybe_async;

    use super::{Action, Error};
    use crate::{
        handshake::{refs::from_v2_refs, Ref},
        indicate_end_of_interaction, Command,
    };

    /// Invoke an ls-refs V2 command on `transport`, which requires a prior handshake that yielded
    /// server `capabilities`. `prepare_ls_refs(capabilities, arguments, features)` can be used to alter the _ls-refs_. `progress` is used to provide feedback.
    /// Note that `prepare_ls_refs()` is expected to add the `(agent, Some(name))` to the list of `features`.
    /// If `trace` is `true`, all packetlines received or sent will be passed to the facilities of the `gix-trace` crate.
    #[maybe_async]
    pub async fn ls_refs(
        mut transport: impl Transport,
        capabilities: &Capabilities,
        prepare_ls_refs: impl FnOnce(
            &Capabilities,
            &mut Vec<BString>,
            &mut Vec<(&str, Option<Cow<'static, str>>)>,
        ) -> std::io::Result<Action>,
        progress: &mut impl Progress,
        trace: bool,
    ) -> Result<Vec<Ref>, Error> {
        let _span = gix_features::trace::detail!("gix_protocol::ls_refs()", capabilities = ?capabilities);
        let ls_refs = Command::LsRefs;
        let mut ls_features = ls_refs.default_features(gix_transport::Protocol::V2, capabilities);
        let mut ls_args = ls_refs.initial_v2_arguments(&ls_features);
        if capabilities
            .capability("ls-refs")
            .and_then(|cap| cap.supports("unborn"))
            .unwrap_or_default()
        {
            ls_args.push("unborn".into());
        }
        let refs = match prepare_ls_refs(capabilities, &mut ls_args, &mut ls_features) {
            Ok(Action::Skip) => Vec::new(),
            Ok(Action::Continue) => {
                ls_refs.validate_argument_prefixes(
                    gix_transport::Protocol::V2,
                    capabilities,
                    &ls_args,
                    &ls_features,
                )?;

                progress.step();
                progress.set_name("list refs".into());
                let mut remote_refs = transport
                    .invoke(
                        ls_refs.as_str(),
                        ls_features.into_iter(),
                        if ls_args.is_empty() {
                            None
                        } else {
                            Some(ls_args.into_iter())
                        },
                        trace,
                    )
                    .await?;
                from_v2_refs(&mut remote_refs).await?
            }
            Err(err) => {
                indicate_end_of_interaction(transport, trace).await?;
                return Err(err.into());
            }
        };
        Ok(refs)
    }
}
