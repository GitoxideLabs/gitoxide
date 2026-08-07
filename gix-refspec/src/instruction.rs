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

/// Sources are local ref names, ref patterns, or revision expressions unless specified otherwise.
/// A non-pattern source with an explicit destination is kept without syntax validation, like Git.
/// Destinations are partial or full ref names on the remote side.
#[derive(PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Hash, Debug)]
pub enum Push<'a> {
    /// Push all local branches to the matching destination on the remote, which has to exist to be updated.
    AllMatchingBranches {
        /// If true, allow non-fast-forward updates of the matched destination branch.
        allow_non_fast_forward: bool,
    },
    /// Delete the destination ref.
    Delete {
        /// The reference to delete on the remote.
        ref_or_pattern: &'a BStr,
    },
    /// Push the object or objects named by `src` to `dst`.
    Matching {
        /// The source expression to push. Non-pattern expressions with an explicit destination are not syntax-checked.
        /// Ref patterns contain exactly one `*`, such as `refs/heads/*`.
        src: &'a BStr,
        /// The ref to update with the object from `src`. If `src`  is a pattern, this is a pattern too.
        /// Examples are refnames like `HEAD` or `refs/heads/main`, or patterns like `refs/heads/*`.
        dst: &'a BStr,
        /// If true, allow non-fast-forward updates of `dst`.
        allow_non_fast_forward: bool,
    },
    /// Exclude a single ref.
    Exclude {
        /// A full or partial ref name to exclude, or a pattern containing a single `*`.
        /// Partial names are matched literally; rev-specs and object hashes are not supported.
        src: &'a BStr,
    },
}

/// Sources are remote ref names or fully spelled object IDs unless specified otherwise.
///
/// Destinations are partial or full ref names on the local side.
#[derive(PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Hash, Debug)]
pub enum Fetch<'a> {
    /// Fetch a ref or refs, without updating local branches.
    Only {
        /// The partial or full ref name to fetch on the remote side or the full object hex-name, without updating the local side.
        /// This cannot be a ref pattern, as a positive fetch pattern requires a patterned destination.
        src: &'a BStr,
    },
    /// Exclude a single ref.
    Exclude {
        /// A partial or full ref name to exclude on the remote, or a pattern containing a single `*`.
        /// Partial names are matched literally; object IDs are not supported.
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
