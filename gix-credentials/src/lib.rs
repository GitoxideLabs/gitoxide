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

use gix_error::Result;
/// A native or external implementation of the credential helper protocol.
#[derive(Debug)]
pub struct Program {
    /// The kind of program, ready for launch.
    pub kind: program::Kind,
    /// Working directory for helper processes and relative native helper paths.
    /// `None` uses the current process directory.
    pub current_dir: Option<std::path::PathBuf>,
    /// If true, stderr is enabled, which is the default.
    pub stderr: bool,
    /// How long the native `store` helper waits for its file lock. Defaults to one second.
    /// Corresponds to Git's `credentialStore.lockTimeoutMS`.
    pub store_lock_timeout: gix_lock::acquire::Fail,
    /// The executable and arguments used to start a native cache daemon when needed.
    /// Without a launcher, existing daemons can be queried, but an absent daemon cannot store credentials.
    #[cfg(unix)]
    pub cache_daemon: Option<program::cache::DaemonCommand>,
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
/// If more control is required, use the [`Cascade`][helper::Cascade] type.
pub fn builtin(action: helper::Action) -> Result<Option<protocol::Outcome>> {
    protocol::helper_outcome_to_result(
        helper::invoke(&mut Program::from_kind(program::Kind::Builtin), &action)?,
        action,
    )
}
