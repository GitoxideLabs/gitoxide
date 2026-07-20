use std::sync::atomic::AtomicBool;

use crate::File;

mod error {
    /// The error returned by [File::verify_integrity()][super::File::verify_integrity()].
    #[derive(Debug)]
    #[allow(missing_docs)]
    pub enum Error {
        Io(gix_hash::io::Error),
        Verify(gix_hash::verify::Error),
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Io(_) => f.write_str("Could not read index file to generate hash"),
                Error::Verify(_) => f.write_str("Index checksum mismatch"),
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::Io(err) => Some(err),
                Error::Verify(err) => Some(err),
            }
        }
    }

    impl From<gix_hash::io::Error> for Error {
        fn from(err: gix_hash::io::Error) -> Self {
            Error::Io(err)
        }
    }

    impl From<gix_hash::verify::Error> for Error {
        fn from(err: gix_hash::verify::Error) -> Self {
            Error::Verify(err)
        }
    }
}
pub use error::Error;

impl File {
    /// Verify the integrity of the index to assure its consistency.
    pub fn verify_integrity(&self) -> Result<(), Error> {
        let _span = gix_features::trace::coarse!("gix_index::File::verify_integrity()");
        if let Some(checksum) = self.checksum {
            let num_bytes_to_hash =
                self.path.metadata().map_err(gix_hash::io::Error::from)?.len() - checksum.as_bytes().len() as u64;
            let should_interrupt = AtomicBool::new(false);
            gix_hash::bytes_of_file(
                &self.path,
                num_bytes_to_hash,
                checksum.kind(),
                &mut gix_features::progress::Discard,
                &should_interrupt,
            )?
            .verify(&checksum)?;
        }
        Ok(())
    }
}
