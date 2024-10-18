use std::fmt;

use bstr::{BStr, BString, ByteSlice, ByteVec};

/// The arguments passed to a server command.
#[derive(Debug)]
pub struct Arguments {
    /// The active features/capabilities of the fetch invocation
    #[cfg(any(feature = "async-client", feature = "blocking-client"))]
    features: Vec<crate::command::Feature>,

    args: Vec<BString>,
    haves: Vec<BString>,

    filter: bool,
    shallow: bool,
    deepen_since: bool,
    deepen_not: bool,
    deepen_relative: bool,
    ref_in_want: bool,
    supports_include_tag: bool,

    features_for_first_want: Option<Vec<String>>,
    #[cfg(any(feature = "async-client", feature = "blocking-client"))]
    version: gix_transport::Protocol,

    trace: bool,
}

impl Arguments {
    /// Return true if there is no argument at all.
    ///
    /// This can happen if callers assure that they won't add 'wants' if their 'have' is the same, i.e. if the remote has nothing
    /// new for them.
    pub fn is_empty(&self) -> bool {
        self.haves.is_empty() && !self.args.iter().rev().any(|arg| arg.starts_with_str("want "))
    }
    /// Return true if ref filters is supported.
    pub fn can_use_filter(&self) -> bool {
        self.filter
    }
    /// Return true if shallow refs are supported.
    ///
    /// This is relevant for partial clones when using `--depth X`.
    pub fn can_use_shallow(&self) -> bool {
        self.shallow
    }
    /// Return true if the 'deepen' capability is supported.
    ///
    /// This is relevant for partial clones when using `--depth X` and retrieving additional history.
    pub fn can_use_deepen(&self) -> bool {
        self.shallow
    }
    /// Return true if the '`deepen_since`' capability is supported.
    ///
    /// This is relevant for partial clones when using `--depth X` and retrieving additional history
    /// based on a date beyond which all history should be present.
    pub fn can_use_deepen_since(&self) -> bool {
        self.deepen_since
    }
    /// Return true if the '`deepen_not`' capability is supported.
    ///
    /// This is relevant for partial clones when using `--depth X`.
    pub fn can_use_deepen_not(&self) -> bool {
        self.deepen_not
    }
    /// Return true if the '`deepen_relative`' capability is supported.
    ///
    /// This is relevant for partial clones when using `--depth X`.
    pub fn can_use_deepen_relative(&self) -> bool {
        self.deepen_relative
    }
    /// Return true if the 'ref-in-want' capability is supported.
    ///
    /// This can be used to bypass 'ls-refs' entirely in protocol v2.
    pub fn can_use_ref_in_want(&self) -> bool {
        self.ref_in_want
    }
    /// Return true if the 'include-tag' capability is supported.
    pub fn can_use_include_tag(&self) -> bool {
        self.supports_include_tag
    }
    /// Return true if we will use a stateless mode of operation, which can be decided in conjunction with `transport_is_stateless`.
    ///
    /// * we are always stateless if the transport is stateless, i.e. doesn't support multiple interactions with a single connection.
    /// * we are always stateless if the protocol version is `2`
    /// * otherwise we may be stateful.
    pub fn is_stateless(&self, transport_is_stateless: bool) -> bool {
        #[cfg(any(feature = "async-client", feature = "blocking-client"))]
        let res = transport_is_stateless || self.version == gix_transport::Protocol::V2;
        #[cfg(not(any(feature = "async-client", feature = "blocking-client")))]
        let res = transport_is_stateless;
        res
    }

    /// Add the given `id` pointing to a commit to the 'want' list.
    ///
    /// As such it should be included in the server response as it's not present on the client.
    pub fn want(&mut self, id: impl AsRef<gix_hash::oid>) {
        match self.features_for_first_want.take() {
            Some(features) => self.prefixed("want ", format!("{} {}", id.as_ref(), features.join(" "))),
            None => self.prefixed("want ", id.as_ref()),
        }
    }
    /// Add the given ref to the 'want-ref' list.
    ///
    /// The server should respond with a corresponding 'wanted-refs' section if it will include the
    /// wanted ref in the packfile response.
    pub fn want_ref(&mut self, ref_path: &BStr) {
        let mut arg = BString::from("want-ref ");
        arg.push_str(ref_path);
        self.args.push(arg);
    }
    /// Add the given `id` pointing to a commit to the 'have' list.
    ///
    /// As such it should _not_ be included in the server response as it's already present on the client.
    pub fn have(&mut self, id: impl AsRef<gix_hash::oid>) {
        self.haves.push(format!("have {}", id.as_ref()).into());
    }
    /// Add the given `id` pointing to a commit to the 'shallow' list.
    pub fn shallow(&mut self, id: impl AsRef<gix_hash::oid>) {
        debug_assert!(self.shallow, "'shallow' feature required for 'shallow <id>'");
        if self.shallow {
            self.prefixed("shallow ", id.as_ref());
        }
    }
    /// Deepen the commit history by `depth` amount of commits.
    pub fn deepen(&mut self, depth: usize) {
        debug_assert!(self.shallow, "'shallow' feature required for deepen");
        if self.shallow {
            self.prefixed("deepen ", depth);
        }
    }
    /// Deepen the commit history to include all commits from now to (and including) `seconds` as passed since UNIX epoch.
    pub fn deepen_since(&mut self, seconds: gix_date::SecondsSinceUnixEpoch) {
        debug_assert!(self.deepen_since, "'deepen-since' feature required");
        if self.deepen_since {
            self.prefixed("deepen-since ", seconds);
        }
    }
    /// Deepen the commit history in a relative instead of absolute fashion.
    pub fn deepen_relative(&mut self) {
        debug_assert!(self.deepen_relative, "'deepen-relative' feature required");
        if self.deepen_relative {
            self.args.push("deepen-relative".into());
        }
    }
    /// Do not include commits reachable by the given `ref_path` when deepening the history.
    pub fn deepen_not(&mut self, ref_path: &BStr) {
        debug_assert!(self.deepen_not, "'deepen-not' feature required");
        if self.deepen_not {
            let mut line = BString::from("deepen-not ");
            line.extend_from_slice(ref_path);
            self.args.push(line);
        }
    }
    /// Set the given filter `spec` when listing references.
    pub fn filter(&mut self, spec: &str) {
        debug_assert!(self.filter, "'filter' feature required");
        if self.filter {
            self.prefixed("filter ", spec);
        }
    }
    /// Permanently allow the server to include tags that point to commits or objects it would return.
    ///
    /// Needs to only be called once.
    pub fn use_include_tag(&mut self) {
        debug_assert!(self.supports_include_tag, "'include-tag' feature required");
        if self.supports_include_tag {
            self.add_feature("include-tag");
        }
    }

    /// Add the given `feature`, unconditionally.
    ///
    /// Note that sending an unknown or unsupported feature may cause the remote to terminate
    /// the connection. Use this method if you know what you are doing *and* there is no specialized
    /// method for this, e.g. [`Self::use_include_tag()`].
    pub fn add_feature(&mut self, feature: &str) {
        match self.version {
            gix_transport::Protocol::V0 | gix_transport::Protocol::V1 => {
                let features = self
                    .features_for_first_want
                    .as_mut()
                    .expect("call add_feature before first want()");
                features.push(feature.into());
            }
            gix_transport::Protocol::V2 => {
                self.args.push(feature.into());
            }
        }
    }

    fn prefixed(&mut self, prefix: &str, value: impl fmt::Display) {
        self.args.push(format!("{prefix}{value}").into());
    }
    /// Create a new instance to help setting up arguments to send to the server as part of a `fetch` operation
    /// for which `features` are the available and configured features to use.
    /// If `trace` is `true`, all packetlines received or sent will be passed to the facilities of the `gix-trace` crate.
    #[cfg(any(feature = "async-client", feature = "blocking-client"))]
    pub fn new(version: gix_transport::Protocol, features: Vec<crate::command::Feature>, trace: bool) -> Self {
        use crate::Command;
        let has = |name: &str| features.iter().any(|f| f.0 == name);
        let filter = has("filter");
        let shallow = has("shallow");
        let ref_in_want = has("ref-in-want");
        let mut deepen_since = shallow;
        let mut deepen_not = shallow;
        let mut deepen_relative = shallow;
        let supports_include_tag;
        let (initial_arguments, features_for_first_want) = match version {
            gix_transport::Protocol::V0 | gix_transport::Protocol::V1 => {
                deepen_since = has("deepen-since");
                deepen_not = has("deepen-not");
                deepen_relative = has("deepen-relative");
                supports_include_tag = has("include-tag");
                let baked_features = features
                    .iter()
                    .filter(
                        |(f, _)| *f != "include-tag", /* not a capability in that sense, needs to be turned on by caller later */
                    )
                    .map(|(n, v)| match v {
                        Some(v) => format!("{n}={v}"),
                        None => n.to_string(),
                    })
                    .collect::<Vec<_>>();
                (Vec::new(), Some(baked_features))
            }
            gix_transport::Protocol::V2 => {
                supports_include_tag = true;
                (Command::Fetch.initial_v2_arguments(&features), None)
            }
        };

        Arguments {
            features,
            version,
            args: initial_arguments,
            haves: Vec::new(),
            filter,
            shallow,
            supports_include_tag,
            deepen_not,
            deepen_relative,
            ref_in_want,
            deepen_since,
            features_for_first_want,
            trace,
        }
    }
}

#[cfg(any(feature = "blocking-client", feature = "async-client"))]
mod shared {
    use bstr::{BString, ByteSlice};
    use gix_transport::{client, client::MessageKind};

    use crate::fetch::Arguments;

    impl Arguments {
        pub(in crate::fetch::arguments) fn prepare_v1(
            &mut self,
            transport_is_stateful: bool,
            add_done_argument: bool,
        ) -> Result<(MessageKind, Option<Vec<BString>>), client::Error> {
            if self.haves.is_empty() {
                assert!(add_done_argument, "If there are no haves, is_done must be true.");
            }
            let on_into_read = if add_done_argument {
                client::MessageKind::Text(&b"done"[..])
            } else {
                client::MessageKind::Flush
            };
            let retained_state = if transport_is_stateful {
                None
            } else {
                Some(self.args.clone())
            };

            if let Some(first_arg_position) = self.args.iter().position(|l| l.starts_with_str("want ")) {
                self.args.swap(first_arg_position, 0);
            }
            Ok((on_into_read, retained_state))
        }
    }
}

#[cfg(feature = "async-client")]
mod async_io;

#[cfg(feature = "blocking-client")]
mod blocking_io;
