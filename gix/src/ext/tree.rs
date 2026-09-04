use std::borrow::BorrowMut;

use gix_object::TreeRefIter;
use gix_traverse::tree::breadthfirst;

pub trait Sealed {}

/// An extension trait for tree iterators
pub trait TreeIterExt: Sealed {
    /// Traverse this tree with `state` being provided to potentially reuse allocations, and `find` being a function to lookup trees
    /// and turn them into iterators.
    ///
    /// The `delegate` implements a way to store details about the traversal to allow paying only for what's actually used.
    /// Since it is expected to store the operation result, _unit_ is returned.
    fn traverse<StateMut, Find, V>(&self, state: StateMut, objects: Find, delegate: &mut V) -> Result<(), crate::Error>
    where
        Find: gix_object::Find,
        StateMut: BorrowMut<breadthfirst::State>,
        V: gix_traverse::tree::Visit;
}

impl Sealed for TreeRefIter<'_> {}

impl TreeIterExt for TreeRefIter<'_> {
    fn traverse<StateMut, Find, V>(&self, state: StateMut, objects: Find, delegate: &mut V) -> Result<(), crate::Error>
    where
        Find: gix_object::Find,
        StateMut: BorrowMut<breadthfirst::State>,
        V: gix_traverse::tree::Visit,
    {
        breadthfirst(*self, state, objects, delegate).map_err(gix_error::Exn::into_error)
    }
}

/// Extensions for [EntryRef](gix_object::tree::EntryRef).
pub trait TreeEntryRefExt<'a>: 'a {
    /// Attach [`repo`](crate::Repository) to the given tree entry. It can be detached later with `detach()`.
    fn attach<'repo>(self, repo: &'repo crate::Repository) -> crate::object::tree::EntryRef<'repo, 'a>;
}

impl<'a> TreeEntryRefExt<'a> for gix_object::tree::EntryRef<'a> {
    fn attach<'repo>(self, repo: &'repo crate::Repository) -> crate::object::tree::EntryRef<'repo, 'a> {
        crate::object::tree::EntryRef { inner: self, repo }
    }
}

/// Extensions for [Entry](gix_object::tree::Entry).
pub trait TreeEntryExt {
    /// Attach [`repo`](crate::Repository) to the given tree entry. It can be detached later with `detach()`.
    fn attach(self, repo: &crate::Repository) -> crate::object::tree::Entry<'_>;
}

impl TreeEntryExt for gix_object::tree::Entry {
    fn attach(self, repo: &crate::Repository) -> crate::object::tree::Entry<'_> {
        crate::object::tree::Entry { inner: self, repo }
    }
}

/// Extensions for [Change](gix_diff::tree_with_rewrites::Change).
#[cfg(feature = "blob-diff")]
pub trait TreeDiffChangeExt {
    /// Attach [`old_repo`](crate::Repository) and `new_repo` to current instance. It can be detached later with `detach()`.
    /// Note that both repositories are usually the same.
    fn attach<'old, 'new>(
        &self,
        old_repo: &'old crate::Repository,
        new_repo: &'new crate::Repository,
    ) -> crate::object::tree::diff::Change<'_, 'old, 'new>;
}
