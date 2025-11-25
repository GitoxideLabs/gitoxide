#[cfg(any(feature = "blocking-client", feature = "async-client"))]
mod error {
    use crate::handshake::refs::parse;

    /// The error returned by invoking a [`super::function::LsRefsCommand`].
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
#[cfg(any(feature = "blocking-client", feature = "async-client"))]
pub use error::Error;

#[cfg(any(feature = "blocking-client", feature = "async-client"))]
pub(crate) mod function {
    use std::{borrow::Cow, collections::HashSet};

    use bstr::{BString, ByteVec};
    use gix_features::progress::Progress;
    use gix_transport::client::Capabilities;
    use maybe_async::maybe_async;

    use super::Error;
    #[cfg(feature = "async-client")]
    use crate::transport::client::async_io::{Transport, TransportV2Ext};
    #[cfg(feature = "blocking-client")]
    use crate::transport::client::blocking_io::{Transport, TransportV2Ext};
    use crate::{
        handshake::{refs::from_v2_refs, Ref},
        Command,
    };

    /// A command to list references from a remote Git repository.
    pub struct LsRefsCommand<'a> {
        capabilities: &'a Capabilities,
        features: Vec<(&'static str, Option<Cow<'static, str>>)>,
        arguments: Vec<BString>,
    }

    impl<'a> LsRefsCommand<'a> {
        /// Build a command to list refs from the given server `capabilities`,
        /// using `agent` information to identify ourselves.
        pub fn new(
            prefix_refspecs: Option<&[gix_refspec::RefSpec]>,
            capabilities: &'a Capabilities,
            agent: (&'static str, Option<Cow<'static, str>>),
        ) -> Self {
            let _span =
                gix_features::trace::detail!("gix_protocol::LsRefsCommand::new()", capabilities = ?capabilities);
            let ls_refs = Command::LsRefs;
            let mut features = ls_refs.default_features(gix_transport::Protocol::V2, capabilities);
            features.push(agent);
            let mut arguments = ls_refs.initial_v2_arguments(&features);
            if capabilities
                .capability("ls-refs")
                .and_then(|cap| cap.supports("unborn"))
                .unwrap_or_default()
            {
                arguments.push("unborn".into());
            }

            if let Some(refspecs) = prefix_refspecs {
                let mut seen = HashSet::new();
                for spec in refspecs {
                    let spec = spec.to_ref();
                    if seen.insert(spec.instruction()) {
                        let mut prefixes = Vec::with_capacity(1);
                        spec.expand_prefixes(&mut prefixes);
                        for mut prefix in prefixes {
                            prefix.insert_str(0, "ref-prefix ");
                            arguments.push(prefix);
                        }
                    }
                }
            }

            Self {
                capabilities,
                features,
                arguments,
            }
        }

        /// Invoke a ls-refs V2 command on `transport`.
        ///
        /// `progress` is used to provide feedback.
        /// If `trace` is `true`, all packetlines received or sent will be passed to the facilities of the `gix-trace` crate.
        #[maybe_async]
        pub async fn invoke(
            self,
            mut transport: impl Transport,
            progress: &mut impl Progress,
            trace: bool,
        ) -> Result<Vec<Ref>, Error> {
            Command::LsRefs.validate_argument_prefixes(
                gix_transport::Protocol::V2,
                self.capabilities,
                &self.arguments,
                &self.features,
            )?;

            progress.step();
            progress.set_name("list refs".into());
            let mut remote_refs = transport
                .invoke(
                    Command::LsRefs.as_str(),
                    self.features.into_iter(),
                    if self.arguments.is_empty() {
                        None
                    } else {
                        Some(self.arguments.into_iter())
                    },
                    trace,
                )
                .await?;
            Ok(from_v2_refs(&mut remote_refs).await?)
        }
    }
}
