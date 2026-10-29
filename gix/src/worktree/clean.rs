/// Options for [`Repository::clean_worktree()`][crate::Repository::clean_worktree()].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// Only report candidates, without locking the index or deleting anything. Defaults to `true`.
    pub dry_run: bool,
    /// Also remove expendable ignored entries. Precious entries remain protected.
    pub ignored: bool,
    /// Remove untracked directories. Explicit pathspecs enable this regardless of the flag, like Git.
    pub directories: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            dry_run: true,
            ignored: false,
            directories: false,
        }
    }
}
