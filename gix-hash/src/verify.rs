use crate::{ObjectId, oid};

/// The error returned by [`oid::verify()`].
#[derive(Debug)]
#[expect(missing_docs)]
pub struct Error {
    pub actual: ObjectId,
    pub expected: ObjectId,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Hash was {}, but should have been {}", self.actual, self.expected)
    }
}

impl std::error::Error for Error {}

impl oid {
    /// Verify that `self` matches the `expected` object ID.
    ///
    /// Returns an [`Error`] containing both object IDs if they differ.
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
