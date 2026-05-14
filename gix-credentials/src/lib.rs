//! Interact with git credentials in various ways and launch helper programs.
//!
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

/// A program/executable implementing the credential helper protocol.
#[derive(Debug)]
pub struct Program {
    /// The kind of program, ready for launch.
    pub kind: program::Kind,
    /// If true, stderr is enabled, which is the default.
    pub stderr: bool,
    /// `Some(…)` if the process is running.
    child: Option<std::process::Child>,
}

///
pub mod helper;

///
pub mod program;

///
pub mod protocol;

/// Call the `git credential` helper program performing the given `action`, which reads all context from the git configuration
/// and does everything `git` typically does. The `action` should have been created with [`helper::Action::get_for_url()`] to
/// contain only the URL to kick off the process, or should be created by [`helper::NextAction`].
///
/// If more control is required, use the [`Cascade`][helper::Cascade] type.
#[allow(clippy::result_large_err)]
pub fn builtin(action: helper::Action) -> protocol::Result {
    protocol::helper_outcome_to_result(
        helper::invoke(&mut Program::from_kind(program::Kind::Builtin), &action)?,
        action,
    )
}
