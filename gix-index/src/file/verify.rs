use std::sync::atomic::AtomicBool;

use crate::File;

mod error {
    /// The error returned by [File::verify_integrity()][super::File::verify_integrity()].
    pub type Error = gix_error::Exn<gix_error::Message>;
}
pub use error::Error;

impl File {
    /// Verify the integrity of the index to assure its consistency.
    pub fn verify_integrity(&self) -> Result<(), Error> {
        use gix_error::{ResultExt, message};

        let _span = gix_features::trace::coarse!("gix_index::File::verify_integrity()");
        if let Some(checksum) = self.checksum {
            let num_bytes_to_hash = self
                .path
                .metadata()
                .or_raise(|| message("Could not read index file to generate hash"))?
                .len()
                - checksum.as_bytes().len() as u64;
            let should_interrupt = AtomicBool::new(false);
            gix_hash::bytes_of_file(
                &self.path,
                num_bytes_to_hash,
                checksum.kind(),
                &mut gix_features::progress::Discard,
                &should_interrupt,
            )
            .or_raise(|| message("Could not read index file to generate hash"))?
            .verify(&checksum)
            .or_raise(|| message("Index checksum mismatch"))?;
        }
        Ok(())
    }
}
