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
    /// Omit blobs from the received graph, except explicitly requested blob tips.
    pub filter: Option<Filter>,
    /// If `true`, if we fetch from a remote that only offers shallow clones, the operation will fail with an error
    /// instead of writing the shallow boundary to the shallow file.
    pub reject_shallow_remote: bool,
}

/// Object selection for a partial fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    /// Receive commits and trees, omitting blobs unless explicitly wanted.
    BlobNone,
}

impl Filter {
    /// Return the Git protocol spelling of this filter.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BlobNone => "blob:none",
        }
    }
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
    pub user_agent: crate::command::Feature,
    /// If `true`, output all packetlines using the `gix-trace` machinery.
    pub trace_packetlines: bool,
}

#[cfg(feature = "fetch")]
mod with_fetch {
    use bstr::ByteSlice;
    use gix_error::Result;

    use crate::fetch::{self, negotiate, refmap};

    /// For use in [`fetch`](crate::fetch()).
    pub struct NegotiateContext<'a, 'b, 'c, Objects, Alternates, AlternatesOut, Find>
    where
        Objects: gix_object::Find + gix_object::FindHeader + gix_object::Exists,
        Alternates: FnOnce() -> Result<AlternatesOut>,
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
        fn mark_complete_and_common_ref(&mut self) -> Result<negotiate::Action>;
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
        ) -> Result<(negotiate::Round, bool)>;
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
        /// Return the advertised branches that can identify the remote's born `HEAD`.
        ///
        /// A symbolic `HEAD` selects its advertised branch target. A detached `HEAD`
        /// selects every branch with the same direct object ID, preserving ambiguity
        /// as `git remote show` does. Unborn, missing, or hidden targets yield no candidates.
        /// Tags and other namespaces are never candidates.
        ///
        /// Request both `HEAD` and `refs/heads/*` when creating this map, so protocol v2
        /// servers advertise the branches needed for inference. Results retain advertisement order.
        pub fn remote_head_candidates(&self) -> impl Iterator<Item = &crate::handshake::Ref> {
            use crate::handshake::Ref;
            let head = self.remote_refs.iter().find(|reference| reference.unpack().0 == "HEAD");
            self.remote_refs.iter().filter(move |reference| {
                let (name, object_id, _) = reference.unpack();
                if !name.starts_with(b"refs/heads/") || object_id.is_none() {
                    return false;
                }
                match head {
                    Some(Ref::Symbolic { target, .. }) => name == target,
                    Some(Ref::Direct { .. } | Ref::Peeled { .. }) => object_id == head.and_then(|head| head.unpack().1),
                    Some(Ref::Unborn { .. }) | None => false,
                }
            })
        }

        /// Return exact source names that were requested explicitly but are absent from the advertisement.
        ///
        /// Every exact source must exist, even if other refspecs matched. Wildcards, negative
        /// refspecs and literal object IDs do not require an advertised reference. Partial names
        /// use the same expansion as refspec matching; overlapping and excluded mappings do not
        /// make an advertised source appear missing.
        pub fn missing_source_refs(&self) -> impl Iterator<Item = &bstr::BStr> {
            self.refspecs.iter().enumerate().filter_map(|(index, spec)| {
                let spec = spec.to_ref();
                let src = match spec.instruction() {
                    gix_refspec::Instruction::Fetch(
                        gix_refspec::instruction::Fetch::Only { src }
                        | gix_refspec::instruction::Fetch::AndUpdate { src, .. },
                    ) if src.find_byte(b'*').is_none() && gix_hash::ObjectId::from_hex(src).is_err() => src,
                    _ => return None,
                };
                if self.mappings.iter().any(|mapping| {
                    matches!(mapping.spec_index, refmap::SpecIndex::ExplicitInRemote(spec_index) if spec_index == index)
                }) {
                    return None;
                }
                let mut candidates = Vec::new();
                spec.expand_prefixes(&mut candidates);
                (!self.remote_refs.iter().any(|reference| {
                    let name = reference.unpack().0;
                    candidates.iter().any(|candidate| candidate.as_bstr() == name)
                }))
                .then_some(src)
            })
        }

        /// Return whether any explicitly requested exact source is missing.
        ///
        /// Use this before negotiation or reference updates to reject an incomplete selection.
        /// Wildcards may match no references even if unrelated refs were advertised.
        pub fn is_missing_required_mapping(&self) -> bool {
            self.missing_source_refs().next().is_some()
        }
    }
}
#[cfg(feature = "fetch")]
pub use with_fetch::*;

/// Describe how shallow clones are handled when fetching, with variants defining how the *shallow boundary* is handled.
///
/// The *shallow boundary* is a set of commits whose parents are not present in the repository.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum Shallow {
    /// Fetch all changes from the remote without affecting the shallow boundary at all.
    ///
    /// This also means that repositories that aren't shallow will remain like that.
    #[default]
    NoChange,
    /// Receive update to `depth` commits in the history of the refs to fetch (from the viewpoint of the remote),
    /// with the value of `1` meaning to receive only the commit a ref is pointing to.
    ///
    /// This may update the shallow boundary to increase or decrease the amount of available history.
    DepthAtRemote(std::num::NonZeroU32),
    /// Increase the number of commits and thus expand the shallow boundary by `depth` commits as seen from our local
    /// shallow boundary, with a value of `0` having no effect.
    Deepen(u32),
    /// Set the shallow boundary at the `cutoff` time, meaning that there will be no commits beyond that time.
    Since {
        /// The date beyond which there will be no history.
        cutoff: gix_date::Time,
    },
    /// Receive all history excluding all commits reachable from `remote_refs`. These can be long or short
    /// ref names or tag names.
    Exclude {
        /// The ref names to exclude, short or long. Note that ambiguous short names will cause the remote to abort
        /// without an error message being transferred (because the protocol does not support it)
        remote_refs: Vec<gix_ref::PartialName>,
        /// If some, this field has the same meaning as [`Shallow::Since`] which can be used in combination
        /// with excluded references.
        since_cutoff: Option<gix_date::Time>,
    },
}

impl Shallow {
    /// Produce a variant that causes the repository to loose its shallow boundary, effectively by extending it
    /// beyond all limits.
    pub fn undo() -> Self {
        Shallow::DepthAtRemote((i32::MAX as u32).try_into().expect("valid at compile time"))
    }
}

/// Describe how to handle tags when fetching
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub enum Tags {
    /// Fetch all tags from the remote, even if these are not reachable from objects referred to by our refspecs.
    All,
    /// Fetch only the tags that point to the objects being sent.
    /// That way, annotated tags that point to an object we receive are automatically transmitted and their refs are created.
    /// The same goes for lightweight tags.
    #[default]
    Included,
    /// Do not fetch any tags.
    None,
}

impl Tags {
    /// Obtain a refspec that determines whether or not to fetch all tags, depending on this variant.
    ///
    /// The returned refspec is the default refspec for tags, but won't overwrite local tags ever.
    #[cfg(feature = "fetch")]
    pub fn to_refspec(&self) -> Option<gix_refspec::RefSpecRef<'static>> {
        match self {
            Tags::All | Tags::Included => Some(
                gix_refspec::parse("refs/tags/*:refs/tags/*".into(), gix_refspec::parse::Operation::Fetch)
                    .expect("valid"),
            ),
            Tags::None => None,
        }
    }
}

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

impl From<ProgressId> for gix_utils::progress::Id {
    fn from(v: ProgressId) -> Self {
        match v {
            ProgressId::RemoteProgress => *b"FERP",
        }
    }
}
