use std::path::PathBuf;

use crate::{
    Repository, Result,
    config::tree::{Key, gitoxide},
};

impl Repository {
    /// Return `true` if the repository is a shallow clone, i.e. contains history only up to a certain depth.
    /// Return an error if the configured shallow-file path cannot be represented natively.
    pub fn is_shallow(&self) -> Result<bool> {
        Ok(self
            .shallow_file()?
            .metadata()
            .is_ok_and(|m| m.is_file() && m.len() > 0))
    }

    /// Return a shared list of shallow commits which is updated automatically if the in-memory snapshot has become stale
    /// as the underlying file on disk has changed.
    ///
    /// The list of shallow commits represents the shallow boundary, beyond which we are lacking all (parent) commits.
    /// Note that the list is never empty, as `Ok(None)` is returned in that case indicating the repository
    /// isn't a shallow clone.
    ///
    /// The shared list is shared across all clones of this repository.
    #[gix_macros::unstable(reason = "Returns a `gix-fs` shared snapshot containing `gix-hash` object IDs.")]
    pub fn shallow_commits(&self) -> Result<Option<crate::shallow::Commits>> {
        let shallow_file = self.shallow_file()?;
        self.shallow_commits.recent_snapshot(
            || shallow_file.metadata().ok().and_then(|m| m.modified().ok()),
            || gix_shallow::read(&shallow_file),
        )
    }

    /// Return the path to the `shallow` file which contains hashes, one per line, that describe commits that don't have their
    /// parents within this repository.
    ///
    /// Note that it may not exist if the repository isn't actually shallow.
    /// Return an error if the configured path cannot be represented natively.
    pub fn shallow_file(&self) -> Result<PathBuf> {
        let shallow_name = self
            .config
            .resolved
            .string_filter(gitoxide::Core::SHALLOW_FILE, &mut self.filter_config_section())
            .unwrap_or_else(|| gitoxide::Core::SHALLOW_FILE.default_value_or_panic().into());
        Ok(self.common_dir().join(gix_path::from_bstr(shallow_name)?))
    }
}
