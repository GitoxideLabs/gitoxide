///
pub mod edit {
    /// The error returned by [`edit_references(…)`][crate::Repository::edit_references()], and others
    /// which ultimately create a reference.
    pub type Error = gix_error::Error;
}

///
pub mod peel {
    /// The error returned by [`Reference::peel_to_id()`](crate::Reference::peel_to_id()) and
    /// [`Reference::into_fully_peeled_id()`](crate::Reference::into_fully_peeled_id()).
    pub type Error = gix_error::Error;

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
        /// The error returned by [`find_reference(…)`][crate::Repository::find_reference()], and others.
        pub type Error = gix_error::Error;
    }

    /// The error returned by [`try_find_reference(…)`][crate::Repository::try_find_reference()].
    pub type Error = gix_error::Error;
}
