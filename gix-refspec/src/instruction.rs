use bstr::BStr;

use crate::{Instruction, parse::Operation};

impl Instruction<'_> {
    /// Derive the mode of operation from this instruction.
    pub fn operation(&self) -> Operation {
        match self {
            Instruction::Push(_) => Operation::Push,
            Instruction::Fetch(_) => Operation::Fetch,
        }
    }
}

/// Note that all sources can either be a ref-name, partial or full, or a rev-spec, unless specified otherwise, on the local side.
/// Destinations can only be a partial or full ref names on the remote side.
#[derive(PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Hash, Debug)]
pub enum Push<'a> {
    /// Push all local branches to the matching destination on the remote, which has to exist to be updated.
    AllMatchingBranches {
        /// If true, allow non-fast-forward updates of the matched destination branch.
        allow_non_fast_forward: bool,
    },
    /// Delete the destination ref or glob pattern, with only a single `*` allowed.
    Delete {
        /// The reference or pattern to delete on the remote.
        ref_or_pattern: &'a BStr,
    },
    /// Push a single ref or refspec to a known destination ref.
    Matching {
        /// The source ref or refspec to push. If pattern, it contains a single `*`.
        /// Examples are refnames like `HEAD` or `refs/heads/main`, or patterns like `refs/heads/*`.
        src: &'a BStr,
        /// The ref to update with the object from `src`. If `src`  is a pattern, this is a pattern too.
        /// Examples are refnames like `HEAD` or `refs/heads/main`, or patterns like `refs/heads/*`.
        dst: &'a BStr,
        /// If true, allow non-fast-forward updates of `dest`.
        allow_non_fast_forward: bool,
    },
    /// Exclude a single ref.
    Exclude {
        /// A full or partial ref name to exclude, or multiple if a single `*` is used.
        src: &'a BStr,
    },
}

/// Any source can either be a ref name (full or partial) or a fully spelled out hex-sha for an object, on the remote side.
///
/// Destinations can only be a partial or full ref-names on the local side.
#[derive(PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Hash, Debug)]
pub enum Fetch<'a> {
    /// Fetch a ref or refs, without updating local branches.
    Only {
        /// The partial or full ref name to fetch on the remote side or the full object hex-name, without updating the local side.
        /// Note that this may not be a glob pattern, as those need to be matched by a destination which isn't present here.
        src: &'a BStr,
    },
    /// Exclude a single ref.
    Exclude {
        /// A single partial or full ref name to exclude on the remote, or a pattern with a single `*`. It cannot be a spelled out object hash.
        src: &'a BStr,
    },
    /// Fetch from `src` and update the corresponding destination branches in `dst` accordingly.
    AndUpdate {
        /// The ref name to fetch on the remote side, or a pattern with a single `*` to match against, or the full object hex-name.
        src: &'a BStr,
        /// The local destination to update with what was fetched, or a pattern whose single `*` will be replaced with the matching portion
        /// of the `*` from `src`.
        dst: &'a BStr,
        /// If true, allow non-fast-forward updates of `dest`.
        allow_non_fast_forward: bool,
    },
}
