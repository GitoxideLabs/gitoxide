use crate::note;

impl crate::Repository {
    /// Return a platform for repeated Git notes queries and mutations.
    ///
    /// Selected notes references do not have to exist. Missing references are treated
    /// as containing no notes rather than causing queries to fail.
    pub fn notes(&self) -> Result<note::Platform<'_>, crate::Error> {
        note::Platform::new(self)
    }
}
