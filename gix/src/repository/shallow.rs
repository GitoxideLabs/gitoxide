use std::{borrow::Cow, path::PathBuf};

use crate::{Repository, config::tree::gitoxide};

impl Repository {
    /// Return `true` if the repository is a shallow clone, i.e. contains history only up to a certain depth.
    pub fn is_shallow(&self) -> bool {
        self.shallow_file().metadata().is_ok_and(|m| m.is_file() && m.len() > 0)
    }

    /// Return a shared list of shallow commits which is updated automatically if the in-memory snapshot has become stale
    /// as the underlying file on disk has changed.
    ///
    /// The list of shallow commits represents the shallow boundary, beyond which we are lacking all (parent) commits.
    /// Note that the list is never empty, as `Ok(None)` is returned in that case indicating the repository
    /// isn't a shallow clone.
    ///
    /// The shared list is shared across all clones of this repository.
    pub fn shallow_commits(&self) -> Result<Option<crate::shallow::Commits>, crate::shallow::read::Error> {
        self.shallow_commits.recent_snapshot(
            || self.shallow_file().metadata().ok().and_then(|m| m.modified().ok()),
            || gix_shallow::read(&self.shallow_file()),
        )
    }

    /// Return the path to the `shallow` file which contains hashes, one per line, that describe commits that don't have their
    /// parents within this repository.
    ///
    /// Note that it may not exist if the repository isn't actually shallow.
    pub fn shallow_file(&self) -> PathBuf {
        let shallow_name = self
            .config
            .resolved
            .string_filter(gitoxide::Core::SHALLOW_FILE, &mut self.filter_config_section())
            .unwrap_or_else(|| Cow::Borrowed("shallow".into()));
        self.common_dir().join(gix_path::from_bstr(shallow_name))
    }
}
