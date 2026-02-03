use gix_dir::EntryRef;

use crate::dirwalk;

/// The error returned by [`crate::Repository::dirwalk_for_each()`].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    Dirwalk(#[from] dirwalk::Error),
    #[error("The user-provided callback failed")]
    ForEach(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
}

/// An entry provided to the `for_each` callback during a directory walk.
#[derive(Debug, Clone)]
pub struct Entry<'a> {
    /// The directory entry itself.
    pub entry: EntryRef<'a>,
    /// `collapsed_directory_status` is `Some(dir_status)` if this entry was part of a directory with the given
    /// `dir_status` that wasn't the same as the one of `entry` and if [gix_dir::walk::Options::emit_collapsed] was
    /// [gix_dir::walk::CollapsedEntriesEmissionMode::OnStatusMismatch]. It will also be `Some(dir_status)` if that option
    /// was [gix_dir::walk::CollapsedEntriesEmissionMode::All].
    pub collapsed_directory_status: Option<gix_dir::entry::Status>,
}

impl<'a> Entry<'a> {
    /// Create a new entry from the given components.
    pub fn new(entry: EntryRef<'a>, collapsed_directory_status: Option<gix_dir::entry::Status>) -> Self {
        Self {
            entry,
            collapsed_directory_status,
        }
    }
}
