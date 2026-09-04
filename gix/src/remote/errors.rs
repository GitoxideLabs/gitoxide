///
pub mod find {
    /// The error returned by [`Repository::find_remote(…)`](crate::Repository::find_remote()).
    pub type Error = gix_error::Error;

    ///
    pub mod existing {
        /// The error returned by [`Repository::find_remote(…)`](crate::Repository::find_remote()).
        pub type Error = gix_error::Error;
    }

    ///
    pub mod for_fetch {
        /// The error returned by [`Repository::find_fetch_remote(…)`](crate::Repository::find_fetch_remote()).
        pub type Error = gix_error::Error;
    }
}
