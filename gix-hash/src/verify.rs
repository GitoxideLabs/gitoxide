use crate::oid;

/// The error returned by [`oid::verify()`].
pub type Error = gix_error::CorruptionError;

impl oid {
    /// Verify that `self` matches the `expected` object ID.
    ///
    /// Returns an [`Error`] containing both object IDs if they differ.
    #[inline]
    pub fn verify(&self, expected: &oid) -> Result<(), Error> {
        if self == expected {
            Ok(())
        } else {
            Err(Error::new(format!("Hash was {self}, but should have been {expected}")))
        }
    }
}
