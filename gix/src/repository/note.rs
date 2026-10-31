use crate::{Result, note};

impl crate::Repository {
    /// Return a platform for repeated Git notes queries and mutations.
    ///
    /// Selected notes references do not have to exist. Missing references are treated
    /// as containing no notes rather than causing queries to fail.
    #[gix_macros::unstable(since = "2026-07-30")]
    pub fn notes(&self) -> Result<note::Platform<'_>> {
        note::Platform::new(self)
    }
}
