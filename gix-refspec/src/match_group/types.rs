use std::borrow::Cow;

use bstr::BStr;
use gix_hash::oid;

use crate::RefSpecRef;

/// A match group is able to match a list of ref specs in order while handling negation, conflicts and one to many mappings.
#[derive(Default, Debug, Clone)]
pub struct MatchGroup<'a> {
    /// The specs that take part in item matching.
    pub specs: Vec<RefSpecRef<'a>>,
}

///
pub mod match_lhs {
    use crate::{MatchGroup, match_group::Mapping};

    /// The outcome of any matching operation of a [`MatchGroup`].
    ///
    /// It's used to validate and process the contained [mappings](Mapping).
    #[derive(Debug, Clone)]
    pub struct Outcome<'spec, 'item> {
        /// The match group that produced this outcome.
        pub group: MatchGroup<'spec>,
        /// The mappings derived from matching [items](crate::match_group::Item).
        pub mappings: Vec<Mapping<'item, 'spec>>,
    }
}

///
pub mod match_rhs {
    use crate::{MatchGroup, match_group::Mapping};

    /// The outcome of any matching operation of a [`MatchGroup`].
    ///
    /// It's used to validate and process the contained [mappings](Mapping).
    #[derive(Debug, Clone)]
    pub struct Outcome<'spec, 'item> {
        /// The match group that produced this outcome.
        pub group: MatchGroup<'spec>,
        /// The mappings derived from matching [items](crate::match_group::Item).
        pub mappings: Vec<Mapping<'spec, 'item>>,
    }
}

/// An item to match, input to various matching operations.
#[derive(Debug, Copy, Clone)]
pub struct Item<'a> {
    /// The full name of the references, like `refs/heads/main`
    pub full_ref_name: &'a BStr,
    /// The id that `full_ref_name` points to, which typically is a commit, but can also be a tag object (or anything else).
    pub target: &'a oid,
    /// The object an annotated tag is pointing to, if `target` is an annotated tag.
    pub object: Option<&'a oid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// The source (or left-hand) side of a mapping.
pub enum SourceRef<'a> {
    /// A full reference name, which is expected to be valid.
    ///
    /// Validity, however, is not enforced here.
    FullName(Cow<'a, BStr>),
    /// The name of an object that is expected to exist on the remote side.
    /// Note that it might not be advertised by the remote but part of the object graph,
    /// and thus gets sent in the pack. The server is expected to fail unless the desired
    /// object is present but at some time it is merely a request by the user.
    ObjectId(gix_hash::ObjectId),
}

impl SourceRef<'_> {
    /// Create a fully owned instance by consuming this one.
    pub fn into_owned(self) -> Source {
        match self {
            SourceRef::ObjectId(id) => Source::ObjectId(id),
            SourceRef::FullName(name) => Source::FullName(name.into_owned().into()),
        }
    }
}

impl std::fmt::Display for SourceRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceRef::FullName(name) => name.fmt(f),
            SourceRef::ObjectId(id) => id.fmt(f),
        }
    }
}

/// The source (or left-hand) side of a mapping, which owns its name.
pub type Source = SourceRef<'static>;

/// A mapping from a remote to a local refs for fetches or local to remote refs for pushes.
///
/// Mappings are like edges in a graph, initially without any constraints.
#[derive(Debug, Clone)]
pub struct Mapping<'a, 'b> {
    /// The index into the initial `items` list that matched against a spec.
    pub item_index: Option<usize>,
    /// The name of the remote side for fetches or the local one for pushes that matched.
    pub lhs: SourceRef<'a>,
    /// The name of the local side for fetches or the remote one for pushes that corresponds to `lhs`, if available.
    pub rhs: Option<Cow<'b, BStr>>,
    /// The index of the matched ref-spec as seen from the match group.
    pub spec_index: usize,
}

impl std::hash::Hash for Mapping<'_, '_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.lhs.hash(state);
        self.rhs.hash(state);
    }
}
