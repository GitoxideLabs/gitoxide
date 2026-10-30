use std::fmt::Formatter;

use gix_hash::ObjectId;
use gix_object::bstr::BString;

use crate::{
    store_impl::{file, file::Transaction},
    transaction::RefEdit,
};

/// How to handle packed refs during a transaction
#[derive(Default)]
pub enum PackedRefs<'a> {
    /// Only propagate deletions of references. This is the default.
    /// This means deleted references are removed from disk if they are loose and from the packed-refs file if they are present.
    #[default]
    DeletionsOnly,
    /// Propagate deletions as well as updates to references which are peeled and contain an object id.
    ///
    /// This means deleted references are removed from disk if they are loose and from the packed-refs file if they are present,
    /// while updates are also written into the loose file as well as into packed-refs, potentially creating an entry.
    DeletionsAndNonSymbolicUpdates(Box<dyn gix_object::Find + 'a>),
    /// Propagate deletions as well as updates to references which are peeled and contain an object id. Furthermore delete the
    /// reference which is originally updated if it exists. If it doesn't, the new value will be written into the packed ref right away.
    /// Note that this doesn't affect symbolic references at all, which can't be placed into packed refs.
    ///
    /// Thus, this is similar to `DeletionsAndNonSymbolicUpdates`, but removes the loose reference after the update, leaving only their copy
    /// in `packed-refs`.
    DeletionsAndNonSymbolicUpdatesRemoveLooseSourceReference(Box<dyn gix_object::Find + 'a>),
}

#[derive(Debug)]
pub(in crate::store_impl::file) struct Edit {
    update: RefEdit,
    lock: Option<gix_lock::Marker>,
    /// Set if this update is coming from a symbolic reference and used to make it appear like it is the one that is handled,
    /// instead of the referent reference.
    parent_index: Option<usize>,
    /// For symbolic refs, this is the previous OID to put into the reflog instead of our own previous value. It's the
    /// peeled value of the leaf referent.
    leaf_referent_previous_oid: Option<ObjectId>,
    /// The resolved target of a symbolic update, without replacing its symbolic CAS value.
    symbolic_new_oid: Option<ObjectId>,
    /// An automatically maintained HEAD log isn't a reference edit requested by the caller.
    implicit_head: bool,
}

impl Edit {
    fn name(&self) -> BString {
        self.update.name.0.clone()
    }
}

impl std::borrow::Borrow<RefEdit> for Edit {
    fn borrow(&self) -> &RefEdit {
        &self.update
    }
}

impl std::borrow::BorrowMut<RefEdit> for Edit {
    fn borrow_mut(&mut self) -> &mut RefEdit {
        &mut self.update
    }
}

/// Edits
impl file::Store {
    /// Open a transaction with the given `edits`, and determine how to fail if a `lock` cannot be obtained.
    /// A snapshot of packed references will be obtained automatically if needed to fulfill this transaction
    /// and will be provided as result of a successful transaction. Note that upon transaction failure, packed-refs
    /// will never have been altered.
    ///
    /// The transaction inherits the parent namespace.
    pub fn transaction(&self) -> Transaction<'_, '_> {
        Transaction {
            store: self,
            packed_transaction: None,
            updates: None,
            packed_refs: PackedRefs::default(),
            reflog_message: BString::default(),
        }
    }
}

impl<'p> Transaction<'_, 'p> {
    /// Set the message for HEAD reflog entries caused by deleting its referent.
    /// Updates use their own [`crate::transaction::LogChange::message`]. The default is an empty message.
    pub fn reflog_message(mut self, message: impl Into<BString>) -> Self {
        self.reflog_message = message.into();
        self
    }

    /// Configure the way packed refs are handled during the transaction
    pub fn packed_refs(mut self, packed_refs: PackedRefs<'p>) -> Self {
        self.packed_refs = packed_refs;
        self
    }
}

impl std::fmt::Debug for Transaction<'_, '_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transaction")
            .field("store", self.store)
            .field("edits", &self.updates.as_ref().map(Vec::len))
            .finish_non_exhaustive()
    }
}

///
pub mod prepare;

///
pub mod commit;
