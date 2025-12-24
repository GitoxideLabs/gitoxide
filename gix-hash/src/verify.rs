use crate::{oid, ObjectId};

/// The error returned by [`oid::verify()`].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
#[error("Hash was {actual}, but should have been {expected}")]
pub struct Error {
    pub actual: ObjectId,
    pub expected: ObjectId,
}

impl oid {
    /// Verify that `self` matches the `expected` object id.
    ///
    /// Returns an [`Error`] containing both object ids if they differ.
    #[inline]
    pub fn verify(&self, expected: &oid) -> Result<(), Error> {
        if self == expected {
            Ok(())
        } else {
            Err(Error {
                actual: self.to_owned(),
                expected: expected.to_owned(),
            })
        }
    }
}
