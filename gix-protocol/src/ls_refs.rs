#[cfg(any(feature = "blocking-client", feature = "async-client"))]
mod error {
    use crate::handshake::refs::parse;

    /// The error returned by invoking a [`super::function::LsRefsCommand`].
    // TODO(review): all four variants were `#[error(transparent)]`: `Display` and `source()`
    //                forward to the wrapped error, exactly like the `thiserror`-generated code did.
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        Io(std::io::Error),
        Transport(gix_transport::client::Error),
        Parse(parse::Error),
        ArgumentValidation(crate::command::validate_argument_prefixes::Error),
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Io(err) => std::fmt::Display::fmt(err, f),
                Error::Transport(err) => std::fmt::Display::fmt(err, f),
                Error::Parse(err) => std::fmt::Display::fmt(err, f),
                Error::ArgumentValidation(err) => std::fmt::Display::fmt(err, f),
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::Io(err) => err.source(),
                Error::Transport(err) => err.source(),
                Error::Parse(err) => err.source(),
                Error::ArgumentValidation(err) => err.source(),
            }
        }
    }

    impl From<std::io::Error> for Error {
        fn from(err: std::io::Error) -> Self {
            Error::Io(err)
        }
    }

    impl From<gix_transport::client::Error> for Error {
        fn from(err: gix_transport::client::Error) -> Self {
            Error::Transport(err)
        }
    }

    impl From<parse::Error> for Error {
        fn from(err: parse::Error) -> Self {
            Error::Parse(err)
        }
    }

    impl From<crate::command::validate_argument_prefixes::Error> for Error {
        fn from(err: crate::command::validate_argument_prefixes::Error) -> Self {
            Error::ArgumentValidation(err)
        }
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
    use std::collections::HashSet;

    use bstr::{BString, ByteVec};
    use gix_features::progress::Progress;
    use gix_transport::client::Capabilities;

    use super::Error;
    #[cfg(feature = "async-client")]
    use crate::transport::client::async_io::TransportV2Ext as _;
    #[cfg(feature = "blocking-client")]
    use crate::transport::client::blocking_io::TransportV2Ext as _;
    use crate::{Command, handshake::Ref};

    /// [`RefPrefixes`] are the set of prefixes that are sent to the server for
    /// filtering purposes.
    ///
    /// These are communicated by sending zero or more `ref-prefix` values, and
    /// are documented in [gitprotocol-v2.adoc#ls-refs].
    ///
    /// These prefixes can be constructed from a set of [`RefSpec`]'s using
    /// [`RefPrefixes::from_refspecs`].
    ///
    /// Alternatively, they can be constructed using [`RefPrefixes::new`] and
    /// using [`RefPrefixes::extend`] to add new prefixes.
    ///
    /// [`RefSpec`]: gix_refspec::RefSpec
    /// [gitprotocol-v2.adoc#ls-refs]: https://github.com/git/git/blob/master/Documentation/gitprotocol-v2.adoc#ls-refs
    pub struct RefPrefixes {
        prefixes: Vec<BString>,
    }

    impl Default for RefPrefixes {
        fn default() -> Self {
            Self::new()
        }
    }

    impl RefPrefixes {
        /// Create an empty set of [`RefPrefixes`].
        pub fn new() -> RefPrefixes {
            RefPrefixes { prefixes: Vec::new() }
        }

        /// Convert a series of [`RefSpec`]'s into a set of [`RefPrefixes`].
        ///
        /// It attempts to expand each [`RefSpec`] into prefix references, e.g.
        /// `refs/heads/`, `refs/remotes/`, `refs/namespaces/foo/`, etc.
        ///
        /// Inputs that aren't fully qualified refs, like `HEAD` or `main`, are
        /// expanded in the same DWIM-style way that Git uses for `ref-prefix`
        /// generation, yielding prefixes like `HEAD`, `refs/heads/main`, and
        /// other rev-parse candidates.
        ///
        /// [`RefSpec`]: gix_refspec::RefSpec
        pub fn from_refspecs<'a>(refspecs: impl IntoIterator<Item = &'a gix_refspec::RefSpec>) -> Self {
            let mut seen = HashSet::new();
            let mut prefixes = Self::new();
            for spec in refspecs.into_iter() {
                let spec = spec.to_ref();
                if seen.insert(spec.instruction()) {
                    let mut out = Vec::with_capacity(1);
                    spec.expand_prefixes(&mut out);
                    prefixes.extend(out);
                }
            }
            prefixes
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
            for prefix in iter {
                if !self.prefixes.iter().any(|existing| existing == &prefix) {
                    self.prefixes.push(prefix);
                }
            }
        }
    }

    /// A command to list references from a remote Git repository.
    ///
    /// Its invocation uses the same implementation with either blocking or asynchronous I/O.
    pub struct LsRefsCommand<'a> {
        pub(crate) capabilities: &'a Capabilities,
        features: Vec<crate::command::Feature>,
        arguments: Vec<BString>,
    }

    macro_rules! invoke {
        ($name:ident, $bisync:path, $transport:path, $from_v2_refs:path, $mode:literal) => {
            /// Invoke a ls-refs V2 command on `transport`.
            ///
            /// `progress` is used to provide feedback.
            /// If `trace` is `true`, all packetlines received or sent will be passed to the facilities of the `gix-trace` crate.
            #[$bisync]
            pub async fn $name(
                self,
                mut transport: impl $transport,
                progress: &mut impl Progress,
                trace: bool,
            ) -> Result<Vec<Ref>, Error> {
                let _span = gix_features::trace::detail!("gix_protocol::LsRefsCommand::invoke()", mode = $mode);
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
                Ok($from_v2_refs(&mut remote_refs).await?)
            }
        };
    }

    impl<'a> LsRefsCommand<'a> {
        /// Build a command to list refs from the given server `capabilities`,
        /// using `agent` information to identify ourselves.
        ///
        /// Use [`crate::ls_refs::RefPrefixes::from_refspecs()`] to construct `ref_prefixes`
        /// from refspecs, or [`crate::ls_refs::RefPrefixes::new()`] to build them manually.
        pub fn new(
            ref_prefixes: Option<RefPrefixes>,
            capabilities: &'a Capabilities,
            agent: crate::command::Feature,
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

            if let Some(prefixes) = ref_prefixes {
                arguments.extend(prefixes.into_args());
            }

            Self {
                capabilities,
                features,
                arguments,
            }
        }

        #[cfg(feature = "async-client")]
        invoke!(
            invoke_async,
            ::bisync::asynchronous::bisync,
            crate::transport::client::async_io::Transport,
            crate::handshake::refs::async_io::from_v2_refs,
            "async"
        );

        #[cfg(feature = "blocking-client")]
        invoke!(
            invoke_blocking,
            ::bisync::synchronous::bisync,
            crate::transport::client::blocking_io::Transport,
            crate::handshake::refs::blocking_io::from_v2_refs,
            "blocking"
        );
    }

    #[cfg(test)]
    mod ref_prefixes {
        use bstr::{BString, ByteSlice};

        use super::RefPrefixes;

        #[test]
        fn extend_preserves_first_seen_order_and_deduplicates_prefixes() {
            let mut prefixes = RefPrefixes::new();
            prefixes.extend(
                [
                    "refs/tags",
                    "HEAD",
                    "main",
                    "refs/heads/main",
                    "refs/tags",
                    "HEAD",
                    "refs/heads/feature",
                    "refs/heads/main",
                ]
                .into_iter()
                .map(|prefix| prefix.as_bytes().as_bstr().to_owned()),
            );

            assert_eq!(
                prefixes.into_args().collect::<Vec<_>>(),
                [
                    "ref-prefix refs/tags",
                    "ref-prefix HEAD",
                    "ref-prefix main",
                    "ref-prefix refs/heads/main",
                    "ref-prefix refs/heads/feature"
                ]
                .into_iter()
                .map(BString::from)
                .collect::<Vec<_>>()
            );
        }

        #[test]
        fn from_refspecs_keeps_exact_refs_and_dwim_expansions() {
            let specs = [
                gix_refspec::parse("HEAD".into(), gix_refspec::parse::Operation::Fetch)
                    .expect("valid")
                    .to_owned(),
                gix_refspec::parse("dwim".into(), gix_refspec::parse::Operation::Fetch)
                    .expect("valid")
                    .to_owned(),
                gix_refspec::parse(
                    "refs/tags/prefix*:refs/tags/prefix*".into(),
                    gix_refspec::parse::Operation::Fetch,
                )
                .expect("valid")
                .to_owned(),
                gix_refspec::parse("refs/heads/main".into(), gix_refspec::parse::Operation::Fetch)
                    .expect("valid")
                    .to_owned(),
            ];

            let prefixes = RefPrefixes::from_refspecs(&specs);

            assert_eq!(
                prefixes.into_args().collect::<Vec<_>>(),
                [
                    "ref-prefix HEAD",
                    "ref-prefix dwim",
                    "ref-prefix refs/dwim",
                    "ref-prefix refs/tags/dwim",
                    "ref-prefix refs/heads/dwim",
                    "ref-prefix refs/remotes/dwim",
                    "ref-prefix refs/remotes/dwim/HEAD",
                    "ref-prefix refs/tags/prefix",
                    "ref-prefix refs/heads/main",
                ]
                .into_iter()
                .map(BString::from)
                .collect::<Vec<_>>()
            );
        }
    }
}
