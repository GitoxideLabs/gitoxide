///
pub mod conversion {

    /// The error returned by [`crate::object::try_to_()`][crate::Object::try_to_commit_ref()].
    #[derive(Debug, thiserror::Error)]
    #[expect(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        Decode(#[from] gix_object::decode::Error),
        #[error("Expected object type {}, but got {}", .expected, .actual)]
        UnexpectedType {
            expected: gix_object::Kind,
            actual: gix_object::Kind,
        },
    }
}

///
pub mod find {
    /// Indicate that an error occurred when trying to find an object.
    #[derive(Debug, thiserror::Error)]
    #[error(transparent)]
    pub struct Error(#[from] pub gix_object::find::Error);

    ///
    pub mod existing {
        /// An object could not be found in the database, or an error occurred when trying to obtain it.
        pub type Error = gix_error::Error;
        ///
        pub mod with_conversion {
            /// The error returned by [Repository::find_commit()](crate::Repository::find_commit).
            #[derive(Debug, thiserror::Error)]
            #[expect(missing_docs)]
            pub enum Error {
                #[error(transparent)]
                Find(#[from] crate::object::find::existing::Error),
                #[error(transparent)]
                Convert(#[from] crate::object::try_into::Error),
            }
        }
    }
}

///
pub mod write {
    /// An error to indicate writing to the loose object store failed.
    #[derive(Debug, thiserror::Error)]
    #[error(transparent)]
    pub struct Error(#[from] pub gix_object::write::Error);
}
