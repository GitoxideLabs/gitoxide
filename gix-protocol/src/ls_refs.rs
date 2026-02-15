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
pub use self::function::RefPrefixes;

#[cfg(any(feature = "blocking-client", feature = "async-client"))]
pub(crate) mod function {
    use std::{borrow::Cow, collections::HashSet};

    use bstr::{BString, ByteVec};
    use gix_features::progress::Progress;
    use gix_transport::client::Capabilities;

    use super::Error;
    #[cfg(feature = "async-client")]
    use crate::transport::client::async_io::{self, TransportV2Ext as _};
    #[cfg(feature = "blocking-client")]
    use crate::transport::client::blocking_io::{self, TransportV2Ext as _};
    use crate::{
        handshake::{refs::from_v2_refs, Ref},
        Command,
    };

    /// [`RefPrefixes`] are the set of prefixes that are sent to the server for
    /// filtering purposes.
    ///
    /// These are communicated by sending zero or more `ref-prefix` values, and
    /// are documented in [gitprotocol-v2.adoc#ls-refs].
    ///
    /// These prefixes can be constructed from a set of [`RefSpec`]'s using
    /// [`RefPrefixes::from_refspecs`].
    ///
    /// Alternatively, they can be constructed using [`RefsPrefixes::new`] and
    /// using [`RefPrefixes::extend`] to add new prefixes. Note that any
    /// references not starting with `refs/` will be filtered out.
    ///
    /// [`RefSpec`]: gix_refspec::RefSpec
    /// [gitprotocol-v2.adoc#ls-refs]: https://github.com/git/git/blob/master/Documentation/gitprotocol-v2.adoc#ls-refs
    pub struct RefPrefixes {
        prefixes: HashSet<BString>,
    }

    impl RefPrefixes {
        /// Create an empty set of [`RefPrefixes`].
        pub fn new() -> RefPrefixes {
            RefPrefixes {
                prefixes: HashSet::new(),
            }
        }

        /// Convert a series of [`RefSpec`]'s into a set of [`RefPrefixes`].
        ///
        /// It attempts to expand each [`RefSpec`] into prefix references, e.g.
        /// `refs/heads/`, `refs/remotes/`, `refs/namespaces/foo/`, etc.
        ///
        /// [`RefSpec`]: gix_refspec::RefSpec
        pub fn from_refspecs<'a>(refspecs: impl IntoIterator<Item = &'a gix_refspec::RefSpec>) -> Self {
            let mut seen = HashSet::new();
            let mut prefixes = HashSet::new();
            for spec in refspecs.into_iter() {
                let spec = spec.to_ref();
                if seen.insert(spec.instruction()) {
                    let mut out = Vec::with_capacity(1);
                    spec.expand_prefixes(&mut out);
                    prefixes.extend(out);
                }
            }
            Self { prefixes }
        }

        fn into_args(self) -> impl Iterator<Item = BString> {
            self.prefixes.into_iter().map(|mut prefix| {
                prefix.insert_str(0, "ref-prefix ");
                prefix
            })
        }
    }

    impl Extend<BString> for RefPrefixes {
        fn extend<T: IntoIterator<Item = BString>>(&mut self, iter: T) {
            self.prefixes
                .extend(iter.into_iter().filter(|prefix| prefix.starts_with(b"refs/")));
        }
    }

    /// A command to list references from a remote Git repository.
    ///
    /// It acts as a utility to separate the invocation into the shared blocking portion,
    /// and the one that performs IO either blocking or `async`.
    pub struct LsRefsCommand<'a> {
        pub(crate) capabilities: &'a Capabilities,
        features: Vec<(&'static str, Option<Cow<'static, str>>)>,
        arguments: Vec<BString>,
    }

    impl<'a> LsRefsCommand<'a> {
        /// Build a command to list refs from the given server `capabilities`,
        /// using `agent` information to identify ourselves.
        pub fn new(
            prefix_refspecs: Option<RefPrefixes>,
            capabilities: &'a Capabilities,
            agent: (&'static str, Option<Cow<'static, str>>),
        ) -> Self {
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
                arguments.extend(refspecs.into_args());
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
        #[cfg(feature = "async-client")]
        pub async fn invoke_async(
            self,
            mut transport: impl async_io::Transport,
            progress: &mut impl Progress,
            trace: bool,
        ) -> Result<Vec<Ref>, Error> {
            let _span = gix_features::trace::detail!("gix_protocol::LsRefsCommand::invoke_async()");
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

        /// Invoke a ls-refs V2 command on `transport`.
        ///
        /// `progress` is used to provide feedback.
        /// If `trace` is `true`, all packetlines received or sent will be passed to the facilities of the `gix-trace` crate.
        #[cfg(feature = "blocking-client")]
        pub fn invoke_blocking(
            self,
            mut transport: impl blocking_io::Transport,
            progress: &mut impl Progress,
            trace: bool,
        ) -> Result<Vec<Ref>, Error> {
            let _span = gix_features::trace::detail!("gix_protocol::LsRefsCommand::invoke_blocking()");
            Command::LsRefs.validate_argument_prefixes(
                gix_transport::Protocol::V2,
                self.capabilities,
                &self.arguments,
                &self.features,
            )?;

            progress.step();
            progress.set_name("list refs".into());
            let mut remote_refs = transport.invoke(
                Command::LsRefs.as_str(),
                self.features.into_iter(),
                if self.arguments.is_empty() {
                    None
                } else {
                    Some(self.arguments.into_iter())
                },
                trace,
            )?;
            Ok(from_v2_refs(&mut remote_refs)?)
        }
    }
}
