///
pub mod edit {
    use crate::config;

    // TODO(review): kept concrete due to an E0119 collision. `clone::fetch::Error`
    //                (`gix/src/clone/fetch/mod.rs`) embeds this type via
    //                `HeadUpdate(#[from] crate::reference::edit::Error)`, and already embeds the
    //                erased `config::overrides::Error`, via `ParseConfig(#[from] ...)`. Erasing
    //                `edit::Error` would give `clone::fetch::Error` two `From<gix_error::Error>`
    //                impls. (`init::Error::EditHeadForDefaultBranch` at `gix/src/init.rs` has no
    //                erased member, so it isn't a second blocker.)
    /// The error returned by [`edit_references(…)`][crate::Repository::edit_references()], and others
    /// which ultimately create a reference.
    #[derive(Debug, thiserror::Error)]
    #[expect(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        FileTransactionPrepare(#[from] gix_ref::file::transaction::prepare::Error),
        #[error(transparent)]
        FileTransactionCommit(#[from] gix_ref::file::transaction::commit::Error),
        #[error(transparent)]
        NameValidation(#[from] gix_validate::reference::name::Error),
        #[error(
            "Could not interpret core.filesRefLockTimeout or core.packedRefsTimeout, it must be the number in milliseconds to wait for locks or negative to wait forever"
        )]
        LockTimeoutConfiguration(#[from] config::lock_timeout::Error),
        #[error(transparent)]
        ParseCommitterTime(#[from] crate::config::time::Error),
    }
}

///
pub mod peel {
    // TODO(review): kept concrete. Matched in `update()`
    //                (`gix/src/remote/connection/fetch/update_refs/mod.rs`) to detect unborn refs:
    //                `Error::ToId(gix_ref::peel::to_id::Error::
    //                FollowToObject(gix_ref::peel::to_object::Error::Follow(_)))`.
    /// The error returned by [`Reference::peel_to_id()`](crate::Reference::peel_to_id()) and
    /// [`Reference::into_fully_peeled_id()`](crate::Reference::into_fully_peeled_id()).
    #[derive(Debug, thiserror::Error)]
    #[expect(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        ToId(#[from] gix_ref::peel::to_id::Error),
        #[error(transparent)]
        PackedRefsOpen(#[from] gix_ref::packed::buffer::open::Error),
    }

    ///
    pub mod to_kind {
        /// The error returned by [`Reference::peel_to_kind(…)`](crate::Reference::peel_to_kind()).
        pub type Error = gix_error::Error;
    }
}

///
pub mod follow {
    ///
    pub mod to_object {
        /// The error returned by [`Reference::follow_to_object(…)`](crate::Reference::follow_to_object()).
        pub type Error = gix_error::Error;
    }
}

///
pub mod head_id {
    /// The error returned by [`Repository::head_id(…)`](crate::Repository::head_id()).
    pub type Error = gix_error::Error;
}

///
pub mod head_commit {
    /// The error returned by [`Repository::head_commit`(…)](crate::Repository::head_commit()).
    pub type Error = gix_error::Error;
}

///
pub mod head_tree_id {
    /// The error returned by [`Repository::head_tree_id`(…)](crate::Repository::head_tree_id()).
    pub type Error = gix_error::Error;
}

///
pub mod head_tree {
    /// The error returned by [`Repository::head_tree`(…)](crate::Repository::head_tree()).
    pub type Error = gix_error::Error;
}

///
pub mod find {
    ///
    pub mod existing {
        use gix_ref::PartialName;

        // TODO(review): kept concrete. Callers match its `NotFound` variant directly, in
        //                `Repository::head()` (`gix/src/repository/reference.rs`) and
        //                `Delegate::nth_checked_out_branch()`
        //                (`gix/src/revision/spec/parse/delegate/revision.rs`). Separately,
        //                `repository::index_or_load_from_head_or_empty::Error::ReadHead`
        //                (`gix/src/repository/mod.rs`) already has an erased slot via `PeelToTree`,
        //                so this type is doubly blocked from erasure there.
        /// The error returned by [`find_reference(…)`][crate::Repository::find_reference()], and others.
        #[derive(Debug, thiserror::Error)]
        #[expect(missing_docs)]
        pub enum Error {
            #[error(transparent)]
            Find(#[from] crate::reference::find::Error),
            #[error("The reference '{}' did not exist", name.as_ref().as_bstr())]
            NotFound { name: PartialName },
        }
    }

    /// The error returned by [`try_find_reference(…)`][crate::Repository::try_find_reference()].
    pub type Error = gix_error::Error;
}
