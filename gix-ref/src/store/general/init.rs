use std::path::PathBuf;

mod error {
    /// The error returned by [`crate::Store::at()`].
    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error("There was an error accessing the store's directory")]
        Io(#[from] std::io::Error),
    }
}

pub use error::Error;

use crate::file;

#[expect(
    dead_code,
    reason = "callers still initialize file::Store directly while the general store awaits ref-table support"
)]
impl crate::Store {
    /// Create a new store at the given location, typically the `.git/` directory.
    /// Use [`at_opts()`](Self::at_opts) to adjust settings.
    ///
    /// Note that if [`precompose_unicode`](crate::store::init::Options::precompose_unicode) is set in the options,
    /// the `git_dir` is also expected to use precomposed unicode, or else some operations that strip prefixes will fail.
    pub fn at(git_dir: PathBuf, object_hash: gix_hash::Kind) -> Result<Self, Error> {
        Self::at_opts(git_dir, object_hash, Default::default())
    }

    /// Create a new store at the given location, typically the `.git/` directory.
    /// Use [`opts`](crate::store::init::Options) to adjust settings.
    ///
    /// Note that if [`precompose_unicode`](crate::store::init::Options::precompose_unicode) is set in the options,
    /// the `git_dir` is also expected to use precomposed unicode, or else some operations that strip prefixes will fail.
    pub fn at_opts(
        git_dir: PathBuf,
        object_hash: gix_hash::Kind,
        opts: crate::store::init::Options,
    ) -> Result<Self, Error> {
        // for now, just try to read the directory - later we will do that naturally as we have to figure out if it's a ref-table or not.
        std::fs::read_dir(&git_dir)?;
        Ok(crate::Store {
            inner: crate::store::State::Loose {
                store: file::Store::at_opts(git_dir, object_hash, opts),
            },
        })
    }
}
