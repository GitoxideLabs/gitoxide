use std::sync::atomic::AtomicBool;

use gix_features::progress::Progress;

use crate::data::File;

///
pub mod checksum {
    /// Returned by [`data::File::verify_checksum()`][crate::data::File::verify_checksum()].
    pub type Error = crate::verify::checksum::Error;
}

/// Checksums and verify checksums
impl<T> File<T>
where
    T: crate::FileData,
{
    /// The checksum in the trailer of this pack data file
    pub fn checksum(&self) -> gix_hash::ObjectId {
        gix_hash::ObjectId::from_bytes_or_panic(&self.data[self.data.len() - self.hash_len..])
    }

    /// Verifies that the checksum of the packfile over all bytes preceding it indeed matches the actual checksum,
    /// returning the actual checksum equivalent to the return value of [`checksum()`][File::checksum()] if there
    /// is no mismatch.
    ///
    /// Note that if no `progress` is desired, one can pass [`gix_features::progress::Discard`].
    ///
    /// Have a look at [`index::File::verify_integrity(…)`][crate::index::File::verify_integrity()] for an
    /// even more thorough integrity check.
    pub fn verify_checksum(
        &self,
        progress: &mut dyn Progress,
        should_interrupt: &AtomicBool,
    ) -> Result<gix_hash::ObjectId, checksum::Error> {
        crate::verify::checksum_on_disk_or_mmap(
            self.path(),
            &self.data,
            self.checksum(),
            self.object_hash,
            progress,
            should_interrupt,
        )
    }
}
