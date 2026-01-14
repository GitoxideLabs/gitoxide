use std::borrow::Cow;

use bstr::BStr;

/// The error returned by [`index()`](crate::index()).
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Cannot diff indices that contain sparse entries")]
    IsSparse,
    #[error("Unmerged entries aren't allowed in the left-hand index, only in the right-hand index")]
    LhsHasUnmerged,
    #[error("The callback indicated failure")]
    Callback(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("Failure during rename tracking")]
    RenameTracking(#[from] crate::rewrites::tracker::emit::Error),
}

/// What to do after a [ChangeRef] was passed ot the callback of [`index()`](crate::index()).
///
/// Use [`std::ops::ControlFlow::Continue`] to continue the operation.
/// Use [`std::ops::ControlFlow::Break`] to stop the operation immediately.
/// This is useful if one just wants to determine if something changed or not.
pub type Action = std::ops::ControlFlow<()>;

/// Options to configure how rewrites are tracked as part of the [`index()`](crate::index()) call.
pub struct RewriteOptions<'a, Find>
where
    Find: gix_object::FindObjectOrHeader,
{
    /// The cache to be used when rename-tracking by similarity is enabled, typically the default.
    /// Note that it's recommended to call [`clear_resource_cache()`](`crate::blob::Platform::clear_resource_cache()`)
    /// between the calls to avoid runaway memory usage, as the cache isn't limited.
    pub resource_cache: &'a mut crate::blob::Platform,
    /// A way to lookup objects from the object database, for use in similarity checks.
    pub find: &'a Find,
    /// Configure how rewrites are tracked.
    pub rewrites: crate::Rewrites,
}

/// Identify a change that would have to be applied to `lhs` to obtain `rhs`, as provided in [`index()`](crate::index()).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChangeRef<'lhs, 'rhs> {
    /// An entry was added to `rhs`.
    Addition {
        /// The location of the newly added entry in `rhs`.
        location: Cow<'rhs, BStr>,
        /// The index into the entries array of `rhs` for full access.
        index: usize,
        /// The mode of the entry in `rhs`.
        entry_mode: gix_index::entry::Mode,
        /// The object id of the entry in `rhs`.
        id: Cow<'rhs, gix_hash::oid>,
    },
    /// An entry was removed from `rhs`.
    Deletion {
        /// The location the entry that doesn't exist in `rhs`.
        location: Cow<'lhs, BStr>,
        /// The index into the entries array of `lhs` for full access.
        index: usize,
        /// The mode of the entry in `lhs`.
        entry_mode: gix_index::entry::Mode,
        /// The object id of the entry in `lhs`.
        id: Cow<'rhs, gix_hash::oid>,
    },
    /// An entry was modified, i.e. has changed its content or its mode.
    Modification {
        /// The location of the modified entry both in `lhs` and `rhs`.
        location: Cow<'rhs, BStr>,
        /// The index into the entries array of `lhs` for full access.
        previous_index: usize,
        /// The previous mode of the entry, in `lhs`.
        previous_entry_mode: gix_index::entry::Mode,
        /// The previous object id of the entry, in `lhs`.
        previous_id: Cow<'lhs, gix_hash::oid>,
        /// The index into the entries array of `rhs` for full access.
        index: usize,
        /// The mode of the entry in `rhs`.
        entry_mode: gix_index::entry::Mode,
        /// The object id of the entry in `rhs`.
        id: Cow<'rhs, gix_hash::oid>,
    },
    /// An entry was renamed or copied from `lhs` to `rhs`.
    ///
    /// A rename is effectively fusing together the `Deletion` of the source and the `Addition` of the destination.
    Rewrite {
        /// The location of the source of the rename or copy operation, in `lhs`.
        source_location: Cow<'lhs, BStr>,
        /// The index of the entry before the rename, into the entries array of `rhs` for full access.
        source_index: usize,
        /// The mode of the entry before the rewrite, in `lhs`.
        source_entry_mode: gix_index::entry::Mode,
        /// The object id of the entry before the rewrite.
        ///
        /// Note that this is the same as `id` if we require the [similarity to be 100%](super::Rewrites::percentage), but may
        /// be different otherwise.
        source_id: Cow<'lhs, gix_hash::oid>,

        /// The current location of the entry in `rhs`.
        location: Cow<'rhs, BStr>,
        /// The index of the entry after the rename, into the entries array of `rhs` for full access.
        index: usize,
        /// The mode of the entry after the rename in `rhs`.
        entry_mode: gix_index::entry::Mode,
        /// The object id of the entry after the rename in `rhs`.
        id: Cow<'rhs, gix_hash::oid>,

        /// If true, this rewrite is created by copy, and `source_id` is pointing to its source. Otherwise, it's a rename,
        /// and `source_id` points to a deleted object, as renames are tracked as deletions and additions of the same
        /// or similar content.
        copy: bool,
    },
}

/// The fully-owned version of [`ChangeRef`].
pub type Change = ChangeRef<'static, 'static>;

mod change;
pub(super) mod function;
