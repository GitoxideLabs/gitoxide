use gix_error::Result;

use super::{Driver, hunk_header};

impl Driver {
    /// Compile this driver's hunk-header definition, falling back to its built-in language
    /// or Git's default identifier matcher when no custom expression is configured.
    pub fn hunk_header(&self) -> Result<hunk_header::Matcher> {
        self.hunk_header
            .clone()
            .or_else(|| hunk_header::Definition::builtin(&self.name))
            .map(|definition| definition.to_matcher())
            .transpose()
            .map(Option::unwrap_or_default)
    }
}
