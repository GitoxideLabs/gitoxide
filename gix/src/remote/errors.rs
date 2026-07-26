///
pub mod find {
    /// The error returned by [`Repository::find_remote(…)`](crate::Repository::find_remote()).
    pub type Error = gix_error::Error;

    ///
    pub mod existing {
        use crate::bstr::BString;

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
