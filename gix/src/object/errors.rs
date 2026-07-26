///
pub mod conversion {
    /// The error returned by [`crate::object::try_to_()`][crate::Object::try_to_commit_ref()].
    pub type Error = gix_error::Error;
}

///
pub mod find {
    // TODO(review): kept concrete. Erasing this would give it a second `From<gix_error::Error>` impl
    //                where it's embedded via `FindObject(#[from] crate::object::find::Error)` in
    //                `update::Error` (`gix/src/remote/connection/fetch/update_refs/update.rs`), which
    //                already has one via `FindReference(#[from] crate::reference::find::Error)`
    //                (`reference::find::Error` is already erased) — E0119.
    /// Indicate that an error occurred when trying to find an object.
    #[derive(Debug, thiserror::Error)]
    #[error(transparent)]
    pub struct Error(#[from] pub gix_object::find::Error);

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
