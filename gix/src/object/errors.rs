///
pub mod conversion {
    /// The error returned by [`crate::object::try_to_()`][crate::Object::try_to_commit_ref()].
    pub type Error = gix_error::Error;
}

///
pub mod find {
    /// Indicate that an error occurred when trying to find an object.
    pub type Error = gix_error::Error;

    ///
    pub mod existing {
        /// An object could not be found in the database, or an error occurred when trying to obtain it.
        pub type Error = gix_object::find::existing::Error;
        ///
        pub mod with_conversion {
            /// The error returned by [Repository::find_commit()](crate::Repository::find_commit).
            pub type Error = gix_error::Error;
        }
    }
}

///
pub mod write {
    /// An error to indicate writing to the loose object store failed.
    pub type Error = gix_error::Error;
}
