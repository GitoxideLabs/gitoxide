use crate::oid;

impl oid {
    /// Verify that `self` matches the `expected` object ID.
    ///
    /// Returns an [`gix_error::CorruptionError`] containing both object IDs if they differ.
    #[inline]
    pub fn verify(&self, expected: &oid) -> Result<(), gix_error::CorruptionError> {
        if self == expected {
            Ok(())
        } else {
            Err(gix_error::CorruptionError::new(format!(
                "Hash was {self}, but should have been {expected}"
            )))
        }
    }
}
