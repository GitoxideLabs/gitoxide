//! An implementation of negotiation algorithms to help the server figure out what we have in common so it can optimize
//! the pack it sends to only contain what we don't have.
#![deny(missing_docs)]
#![forbid(unsafe_code)]
mod consecutive;
mod noop;
mod skipping;

bitflags::bitflags! {
    /// Multi purpose, shared flags that are used by negotiation algorithms and by the caller as well
    ///
    /// However, in this crate we can't implement the calling side, so we marry this type to whatever is needed in downstream crates.
    #[derive(Debug, Default, Copy, Clone)]
    pub struct Flags: u8 {
        /// The object is already available locally and doesn't need to be fetched by the remote.
        const COMPLETE = 1 << 0;
        /// A commit from an alternate object database.
        const ALTERNATE = 1 << 1;

        /// The revision is known to be in common with the remote.
        ///
        /// Used by `consecutive` and `skipping`.
        const COMMON = 1 << 2;
        /// The revision has entered the priority queue.
        ///
        /// Used by `consecutive` and `skipping`.
        const SEEN = 1 << 3;
        /// The revision was popped off our primary priority queue, used to avoid double-counting of `non_common_revs`
        ///
        /// Used by `consecutive` and `skipping`.
        const POPPED = 1 << 4;

        /// The revision is common and was set by merit of a remote tracking ref (e.g. `refs/heads/origin/main`).
        ///
        /// Used by `consecutive`.
        const COMMON_REF = 1 << 5;

        /// The remote let us know it has the object. We still have to tell the server we have this object or one of its descendants.
        /// We won't tell the server about its ancestors.
        ///
        /// Used by `skipping`.
        const ADVERTISED = 1 << 6;
    }
}

/// Additional data to store with each commit when used by any of our algorithms.
///
/// It's shared among those who use the [`Negotiator`] trait, and all implementations of it.
#[derive(Default, Debug, Copy, Clone)]
pub struct Metadata {
    /// Used by `skipping`.
    /// Only used if commit is not COMMON
    pub original_ttl: u16,
    /// Used by `skipping`.
    pub ttl: u16,
    /// Additional information about each commit
    pub flags: Flags,
}

/// The graph our callers use to store traversal information, for (re-)use in the negotiation implementation.
pub type Graph<'find, 'cache> = gix_revwalk::Graph<'find, 'cache, gix_revwalk::graph::Commit<Metadata>>;

/// A map associating an object id with its commit-metadata.
pub type IdMap = gix_revwalk::graph::IdMap<gix_revwalk::graph::Commit<Metadata>>;

/// The way the negotiation is performed.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub enum Algorithm {
    /// Do not send any information at all, which typically leads to complete packs to be sent.
    Noop,
    /// Walk over consecutive commits and check each one. This can be costly be assures packs are exactly the size they need to be.
    #[default]
    Consecutive,
    /// Like `Consecutive`, but skips commits to converge faster, at the cost of receiving packs that are larger than they have to be.
    Skipping,
}

impl std::fmt::Display for Algorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Algorithm::Noop => "noop",
            Algorithm::Consecutive => "consecutive",
            Algorithm::Skipping => "skipping",
        }
        .fmt(f)
    }
}

/// Calculate how many `HAVE` lines we may send in one round, with variation depending on whether the `transport_is_stateless` or not.
/// `window_size` is the previous (or initial) value of the window size.
pub fn window_size(transport_is_stateless: bool, window_size: Option<usize>) -> usize {
    let current_size = match window_size {
        None => return 16,
        Some(cs) => cs,
    };
    const PIPESAFE_FLUSH: usize = 32;
    const LARGE_FLUSH: usize = 16384;

    if transport_is_stateless {
        if current_size < LARGE_FLUSH {
            current_size * 2
        } else {
            current_size * 11 / 10
        }
    } else if current_size < PIPESAFE_FLUSH {
        current_size * 2
    } else {
        current_size + PIPESAFE_FLUSH
    }
}

impl Algorithm {
    /// Create an instance of a negotiator which implements this algorithm.
    pub fn into_negotiator(self) -> Box<dyn Negotiator> {
        match &self {
            Algorithm::Noop => Box::new(noop::Noop) as Box<dyn Negotiator>,
            Algorithm::Consecutive => Box::<consecutive::Algorithm>::default(),
            Algorithm::Skipping => Box::<skipping::Algorithm>::default(),
        }
    }
}

/// A delegate to implement a negotiation algorithm.
pub trait Negotiator {
    /// Mark `id` as common between the remote and us.
    ///
    /// These ids are typically the local tips of remote tracking branches.
    fn known_common(&mut self, id: gix_hash::ObjectId, graph: &mut Graph<'_, '_>) -> Result<(), Error>;

    /// Add `id` as starting point of a traversal across commits that aren't necessarily common between the remote and us.
    ///
    /// These tips are usually the commits of local references whose tips should lead to objects that we have in common with the remote.
    fn add_tip(&mut self, id: gix_hash::ObjectId, graph: &mut Graph<'_, '_>) -> Result<(), Error>;

    /// Produce the next id of an object that we want the server to know we have. It's an object we don't know we have in common or not.
    ///
    /// Returns `None` if we have exhausted all options, which might mean we have traversed the entire commit graph.
    fn next_have(&mut self, graph: &mut Graph<'_, '_>) -> Option<Result<gix_hash::ObjectId, Error>>;

    /// Mark `id` as being common with the remote (as informed by the remote itself) and return `true` if we knew it was common already.
    ///
    /// We can assume to have already seen `id` as we were the one to inform the remote in a prior `have`.
    fn in_common_with_remote(&mut self, id: gix_hash::ObjectId, graph: &mut Graph<'_, '_>) -> Result<bool, Error>;
}

/// An error that happened during any of the methods on a [`Negotiator`].
pub type Error = gix_revwalk::graph::get_or_insert_default::Error;
