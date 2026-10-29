//! Validate raw object structure using Git-compatible diagnostic identifiers and policies.
//!
//! These checks do not establish graph connectivity or verify object checksums. Callers
//! must separately ensure that referenced objects exist with the expected kinds.

use gix_error::{Result, bail, message};
use gix_hash::ObjectId;

mod commit;
mod headers;
mod options;
mod tag;
mod tree;
pub use options::{Diagnostic, MessageId, Options, Severity};

/// Validate the standalone structure of an object. Diagnostics are bounded by the
/// number of message identifiers, rather than the object's number of malformed fields.
pub fn validate(object_id: ObjectId, object: gix_object::Data<'_>, options: &Options) -> Result<Vec<Diagnostic>> {
    if object_id.kind() != object.object_hash {
        bail!(message("Object data and identifier use different hash formats").validation());
    }
    if options.is_skipped(&object_id) {
        return Ok(Vec::new());
    }
    let mut out = Reporter::new(object_id, options);
    match object.kind {
        gix_object::Kind::Commit => commit::validate(object.data, object.object_hash, &mut out),
        gix_object::Kind::Tag => tag::validate(object.data, object.object_hash, &mut out),
        gix_object::Kind::Tree => tree::validate(object.data, object.object_hash, options, &mut out),
        gix_object::Kind::Blob => {}
    }
    Ok(out.finish())
}

pub(super) struct Reporter<'a> {
    object_id: ObjectId,
    options: &'a Options,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Reporter<'a> {
    pub fn new(object_id: ObjectId, options: &'a Options) -> Self {
        Self {
            object_id,
            options,
            diagnostics: Vec::new(),
        }
    }

    pub fn report(&mut self, id: MessageId) -> bool {
        let Some(diagnostic) = Diagnostic::new(self.object_id, id, self.options) else {
            return false;
        };
        let reject = diagnostic.severity == Severity::Error;
        if !self.diagnostics.iter().any(|previous| previous.id == id) {
            self.diagnostics.push(diagnostic);
        }
        reject
    }

    pub fn finish(self) -> Vec<Diagnostic> {
        self.diagnostics
    }
}
