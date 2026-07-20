use std::path::PathBuf;

mod error {
    /// The error returned by [`crate::Store::at()`].
    #[derive(Debug)]
    pub enum Error {
        Io(std::io::Error),
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Io(_) => f.write_str("There was an error accessing the store's directory"),
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::Io(err) => Some(err),
            }
        }
    }

    impl From<std::io::Error> for Error {
        fn from(err: std::io::Error) -> Self {
            Error::Io(err)
        }
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
    /// Use [`opts`](crate::store::init::Options) to adjust settings.
    ///
    /// Note that if [`precompose_unicode`](crate::store::init::Options::precompose_unicode) is set in the options,
    /// the `git_dir` is also expected to use precomposed unicode, or else some operations that strip prefixes will fail.
    pub fn at(git_dir: PathBuf, opts: crate::store::init::Options) -> Result<Self, Error> {
        // for now, just try to read the directory - later we will do that naturally as we have to figure out if it's a ref-table or not.
        std::fs::read_dir(&git_dir)?;
        Ok(crate::Store {
            inner: crate::store::State::Loose {
                store: file::Store::at(git_dir, opts),
            },
        })
    }
}
