//!
#![allow(clippy::empty_docs)]
use gix_object::commit::MessageRef;
use gix_ref::store::ReferenceExt;

use crate::{
    Reference,
    bstr::{BStr, BString, ByteVec},
};

impl Reference<'_> {
    /// Return a platform for obtaining iterators over reference logs.
    pub fn log_iter(&self) -> gix_ref::store::log::Platform<'_> {
        self.inner.log_iter(&self.repo.refs)
    }

    /// Return true if a reflog is present for this reference.
    ///
    /// This compatibility convenience returns `false` if the reference backend
    /// cannot be accessed. Use [`Reference::try_log_exists()`] when the caller
    /// must distinguish absence from an access or corruption error.
    pub fn log_exists(&self) -> bool {
        self.try_log_exists().unwrap_or(false)
    }

    /// Return whether a reflog is present, preserving adapter access errors.
    pub fn try_log_exists(&self) -> Result<bool, gix_ref::store::log::Error> {
        self.inner.log_exists(&self.repo.refs)
    }
}

/// Generate a message typical for git commit logs based on the given `operation`, commit `message` and `num_parents` of the commit.
pub fn message(operation: &str, message: &BStr, num_parents: usize) -> BString {
    let mut out = BString::from(operation);
    if let Some(commit_type) = commit_type_by_parents(num_parents) {
        out.push_str(b" (");
        out.extend_from_slice(commit_type.as_bytes());
        out.push_byte(b')');
    }
    out.push_str(b": ");
    out.extend_from_slice(&MessageRef::from_bytes(message).summary());
    out
}

pub(crate) fn commit_type_by_parents(count: usize) -> Option<&'static str> {
    Some(match count {
        0 => "initial",
        1 => return None,
        _two_or_more => "merge",
    })
}
