use std::path::PathBuf;

use crate::{File, Marker};

mod error {
    use std::{
        fmt,
        fmt::{Debug, Display},
    };

    /// The error returned by various [`commit(…)`][super::Marker::commit()] methods
    #[derive(Debug)]
    pub struct Error<T: Debug> {
        /// The io error that prevented the attempt to succeed
        pub error: std::io::Error,
        /// The marker or file which was used in the attempt to persist it
        pub instance: T,
    }

    impl<T: Debug> Display for Error<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            Display::fmt(&self.error, f)
        }
    }

    impl<T: Debug> std::error::Error for Error<T> {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.error)
        }
    }
}
pub use error::Error;

impl Marker {
    /// Commit the changes written to the previously open file and overwrite the original file atomically, returning the resource path
    /// on success.
    ///
    /// This fails for markers which weren't created with [`File::close()`]
    pub fn commit(mut self) -> Result<PathBuf, Error<Self>> {
        if !self.created_from_file {
            return Err(Error {
                error: std::io::Error::other("refusing to commit marker that was never opened"),
                instance: self,
            });
        }
        let resource_path = self.resource_path();
        match self.inner.persist(&resource_path) {
            Ok(_) => Ok(resource_path),
            Err(err) => Err(Error {
                error: err.error,
                instance: {
                    self.inner = err.handle;
                    self
                },
            }),
        }
    }
}

impl File {
    /// Commit the changes written to this lock file and overwrite the original file atomically, returning the resource path
    /// and an open file handle on success.
    pub fn commit(mut self) -> Result<(PathBuf, Option<std::fs::File>), Error<Self>> {
        let resource_path = self.resource_path();
        match self.inner.persist(&resource_path) {
            Ok(possibly_file) => Ok((resource_path, possibly_file)),
            Err(err) => Err(Error {
                error: err.error,
                instance: {
                    self.inner = err.handle;
                    self
                },
            }),
        }
    }
}
