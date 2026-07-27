///
pub mod find {
    /// The error returned by [`Repository::find_remote(…)`](crate::Repository::find_remote()).
    pub type Error = gix_error::Error;

    ///
    pub mod existing {
        use crate::bstr::BString;

        // TODO(review): kept concrete. Matched at `gix/tests/gix/repository/remote.rs:229`:
        //                `gix::remote::find::existing::Error::NotFound { .. }`. Separately,
        //                `env::collate::fetch::Error::FindExistingRemote` (`gix/src/env.rs`)
        //                already has an erased slot via `CredentialHelperConfig` (feature
        //                `credentials`, on by default), so this type is now doubly blocked from
        //                erasure there. Its other `#[from]` parent, `remote::find::for_fetch::
        //                Error::FindExisting` (below), still has no other erased member.
        /// The error returned by [`Repository::find_remote(…)`](crate::Repository::find_remote()).
        #[derive(Debug, thiserror::Error)]
        #[expect(missing_docs)]
        pub enum Error {
            #[error(transparent)]
            Find(#[from] super::Error),
            #[error("remote name could not be parsed as URL")]
            UrlParse(#[from] gix_url::parse::Error),
            #[error("The remote named {name:?} did not exist")]
            NotFound { name: BString },
        }
    }

    ///
    pub mod for_fetch {
        // TODO(review): kept concrete. Matched at `gix/tests/gix/reference/remote.rs:84`:
        //                `Err(gix::remote::find::for_fetch::Error::ExactlyOneRemoteNotAvailable)`.
        //                No `#[from]` parents embed this type.
        /// The error returned by [`Repository::find_fetch_remote(…)`](crate::Repository::find_fetch_remote()).
        #[derive(Debug, thiserror::Error)]
        #[expect(missing_docs)]
        pub enum Error {
            #[error(transparent)]
            FindExisting(#[from] super::existing::Error),
            #[error(transparent)]
            FindExistingReferences(#[from] crate::reference::find::existing::Error),
            #[error("Could not initialize a URL remote")]
            Init(#[from] crate::remote::init::Error),
            #[error("remote name could not be parsed as URL")]
            UrlParse(#[from] gix_url::parse::Error),
            #[error("No configured remote could be found, or too many were available")]
            ExactlyOneRemoteNotAvailable,
        }
    }
}
