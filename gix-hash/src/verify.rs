use crate::oid;
use gix_error::ExnMessageResult;

impl oid {
    /// Verify that `self` matches the `expected` object ID.
    ///
    /// Returns an error classified as [`gix_error::Class::Corruption`] containing both object IDs if they differ.
    #[inline]
    pub fn verify(&self, expected: &oid) -> ExnMessageResult {
        if self == expected {
            Ok(())
        } else {
            Err(gix_error::corruption(format!("Hash was {self}, but should have been {expected}")).into())
        }
    }
}
