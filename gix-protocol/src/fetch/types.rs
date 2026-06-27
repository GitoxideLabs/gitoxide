use std::path::PathBuf;

use crate::fetch::response::{Acknowledgement, ShallowUpdate, WantedRef};

/// Options for use in [`fetch()`](`crate::fetch()`)
#[derive(Debug, Clone)]
pub struct Options<'a> {
    /// The path to the file containing the shallow commit boundary.
    ///
    /// When needed, it will be locked in preparation for being modified.
    pub shallow_file: PathBuf,
    /// How to deal with shallow repositories. It does affect how negotiations are performed.
    pub shallow: &'a Shallow,
    /// Describe how to handle tags when fetching.
    pub tags: Tags,
    /// If `true`, if we fetch from a remote that only offers shallow clones, the operation will fail with an error
    /// instead of writing the shallow boundary to the shallow file.
    pub reject_shallow_remote: bool,
}

/// For use in [`crate::Handshake::prepare_lsrefs_or_extract_refmap()`] and [`fetch`](crate::fetch()).
#[cfg(feature = "handshake")]
pub struct Context<'a, T> {
    /// The outcome of the handshake performed with the remote.
    ///
    /// Note that it's mutable as depending on the protocol, it may contain refs that have been sent unconditionally.
    pub handshake: &'a mut crate::Handshake,
    /// The transport to use when making an `ls-refs` or `fetch` call.
    ///
    /// This is always done if the underlying protocol is V2, which is implied by the absence of refs in the `handshake` outcome.
    pub transport: &'a mut T,
    /// How to self-identify during the `ls-refs` call in [`crate::Handshake::prepare_lsrefs_or_extract_refmap()`] or the `fetch` call in [`fetch()`](crate::fetch()).
    ///
    /// This could be read from the `gitoxide.userAgent` configuration variable.
    pub user_agent: (&'static str, Option<std::borrow::Cow<'static, str>>),
    /// If `true`, output all packetlines using the `gix-trace` machinery.
    pub trace_packetlines: bool,
}

#[cfg(feature = "fetch")]
mod with_fetch {
    use bstr::ByteSlice;

    use crate::fetch::{self, negotiate, refmap};

    /// For use in [`fetch`](crate::fetch()).
    pub struct NegotiateContext<'a, 'b, 'c, Objects, Alternates, AlternatesOut, AlternatesErr, Find>
    where
        Objects: gix_object::Find + gix_object::FindHeader + gix_object::Exists,
        Alternates: FnOnce() -> Result<AlternatesOut, AlternatesErr>,
        AlternatesErr: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
        AlternatesOut: Iterator<Item = (gix_ref::file::Store, Find)>,
        Find: gix_object::Find,
    {
        /// Access to the object database.
        /// *Note* that the `exists()` calls must not trigger a refresh of the ODB packs as plenty of them might fail, i.e. find on object.
        pub objects: &'a Objects,
        /// Access to the git references database.
        pub refs: &'a gix_ref::file::Store,
        /// A function that returns an iterator over `(refs, objects)` for each alternate repository, to assure all known objects are added also according to their tips.
        pub alternates: Alternates,
        /// The implementation that performs the negotiation later, i.e. prepare wants and haves.
        pub negotiator: &'a mut dyn gix_negotiate::Negotiator,
        /// The commit-graph for use by the `negotiator` - we populate it with tips to initialize the graph traversal.
        pub graph: &'a mut gix_negotiate::Graph<'b, 'c>,
    }

    /// A trait to encapsulate steps to negotiate the contents of the pack.
    ///
    /// Typical implementations use the utilities found in the [`negotiate`] module.
    pub trait Negotiate {
        /// Typically invokes [`negotiate::mark_complete_and_common_ref()`].
        fn mark_complete_and_common_ref(&mut self) -> Result<negotiate::Action, negotiate::Error>;
        /// Typically invokes [`negotiate::add_wants()`].
        /// Returns `true` if wants were added, or `false` if the negotiation should be aborted.
        #[must_use]
        fn add_wants(&mut self, arguments: &mut fetch::Arguments, remote_ref_target_known: &[bool]) -> bool;
        /// Typically invokes [`negotiate::one_round()`].
        fn one_round(
            &mut self,
            state: &mut negotiate::one_round::State,
            arguments: &mut fetch::Arguments,
            previous_response: Option<&fetch::Response>,
        ) -> Result<(negotiate::Round, bool), negotiate::Error>;
    }

    /// The outcome of [`fetch()`](crate::fetch()).
    #[derive(Debug, Clone)]
    pub struct Outcome {
        /// The most recent server response.
        ///
        /// Useful to obtain information about new shallow boundaries.
        pub last_response: fetch::Response,
        /// Information about the negotiation to receive the new pack.
        pub negotiate: NegotiateOutcome,
    }

    /// The negotiation-specific outcome of [`fetch()`](crate::fetch()).
    #[derive(Debug, Clone)]
    pub struct NegotiateOutcome {
        /// The outcome of the negotiation stage of the fetch operation.
        ///
        /// If it is…
        ///
        /// * [`negotiate::Action::MustNegotiate`] there will always be a `pack`.
        /// * [`negotiate::Action::SkipToRefUpdate`] there is no `pack` but references can be updated right away.
        ///
        /// Note that this is never [negotiate::Action::NoChange`] as this would mean there is no negotiation information at all
        /// so this structure wouldn't be present.
        pub action: negotiate::Action,
        /// Additional information for each round of negotiation.
        pub rounds: Vec<negotiate::Round>,
    }

    /// Information about the relationship between our refspecs, and remote references with their local counterparts.
    ///
    /// It's the first stage that offers connection to the server, and is typically required to perform one or more fetch operations.
    #[derive(Default, Debug, Clone)]
    pub struct RefMap {
        /// A mapping between a remote reference and a local tracking branch.
        pub mappings: Vec<refmap::Mapping>,
        /// The explicit refspecs that were supposed to be used for fetching.
        ///
        /// Typically, they are configured by the remote and are referred to by
        /// [`refmap::SpecIndex::ExplicitInRemote`] in [`refmap::Mapping`].
        pub refspecs: Vec<gix_refspec::RefSpec>,
        /// Refspecs which have been added implicitly due to settings of the `remote`, usually pre-initialized from
        /// [`extra_refspecs` in RefMap options](refmap::init::Context).
        /// They are referred to by [`refmap::SpecIndex::Implicit`] in [`refmap::Mapping`].
        ///
        /// They are never persisted nor are they typically presented to the user.
        pub extra_refspecs: Vec<gix_refspec::RefSpec>,
        /// Information about the fixes applied to the `mapping` due to validation and sanitization.
        pub fixes: Vec<gix_refspec::match_group::validate::Fix>,
        /// All refs advertised by the remote.
        pub remote_refs: Vec<crate::handshake::Ref>,
        /// The kind of hash used for all data sent by the server, if understood by this client implementation.
        ///
        /// It was extracted from the `handshake` as advertised by the server.
        pub object_hash: gix_hash::Kind,
    }

    impl RefMap {
        /// Return `true` if the explicit fetch refspecs represented by this mapping failed to match the remote in a way
        /// that should typically be reported as "no mapping".
        ///
        /// Use this before negotiation or reference updates when callers need to reject a fetch early instead of proceeding
        /// with an empty or purely implicit mapping set.
        ///
        /// This is the case if the server advertised refs but none matched at all, or if only implicit mappings were produced
        /// while at least one explicit refspec required an actual ref match, as is the case for exact ref names like `HEAD`
        /// or `refs/heads/main`.
        pub fn is_missing_required_mapping(&self) -> bool {
            let has_explicit_mapping = self
                .mappings
                .iter()
                .any(|mapping| matches!(mapping.spec_index, crate::fetch::refmap::SpecIndex::ExplicitInRemote(_)));

            (self.mappings.is_empty() && !self.remote_refs.is_empty())
                || (!has_explicit_mapping && explicit_fetch_refspecs_require_a_match(&self.refspecs))
        }
    }

    fn explicit_fetch_refspecs_require_a_match(refspecs: &[gix_refspec::RefSpec]) -> bool {
        refspecs.iter().any(|spec| match spec.to_ref().instruction() {
            gix_refspec::Instruction::Fetch(
                gix_refspec::instruction::Fetch::Only { src } | gix_refspec::instruction::Fetch::AndUpdate { src, .. },
            ) => src.find_byteset(b"*?[]\\").is_none() && gix_hash::ObjectId::from_hex(src).is_err(),
            gix_refspec::Instruction::Fetch(gix_refspec::instruction::Fetch::Exclude { .. })
            | gix_refspec::Instruction::Push(_) => false,
        })
    }
}
#[cfg(feature = "fetch")]
pub use with_fetch::*;

/// Describe how shallow clones are handled when fetching.
///
/// Re-exported from `gix-shallow`, where this type lives so it remains available without the network/protocol stack.
pub use gix_shallow::Shallow;

/// Describe how to handle tags when fetching.
///
/// Re-exported from `gix-refspec`, where this type lives so it remains available without the network/protocol stack.
pub use gix_refspec::Tags;


/// A representation of a complete fetch response
#[derive(Debug, Clone)]
pub struct Response {
    pub(crate) acks: Vec<Acknowledgement>,
    pub(crate) shallows: Vec<ShallowUpdate>,
    pub(crate) wanted_refs: Vec<WantedRef>,
    pub(crate) has_pack: bool,
}

/// The progress ids used in during various steps of the fetch operation.
///
/// Note that tagged progress isn't very widely available yet, but support can be improved as needed.
///
/// Use this information to selectively extract the progress of interest in case the parent application has custom visualization.
#[derive(Debug, Copy, Clone)]
pub enum ProgressId {
    /// The progress name is defined by the remote and the progress messages it sets, along with their progress values and limits.
    RemoteProgress,
}

impl From<ProgressId> for gix_features::progress::Id {
    fn from(v: ProgressId) -> Self {
        match v {
            ProgressId::RemoteProgress => *b"FERP",
        }
    }
}
