use crate::note;

impl crate::Repository {
    /// Return a platform for repeated Git notes queries and mutations.
    pub fn notes(&self) -> Result<note::Platform, note::Error> {
        note::Platform::new(self.clone())
    }
}
