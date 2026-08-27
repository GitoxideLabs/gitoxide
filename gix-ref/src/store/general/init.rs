use std::path::PathBuf;

use crate::file;

impl crate::Store {
    /// Create a new store at the given location, typically the `.git/` directory.
    /// Use [`at_opts()`](Self::at_opts) to adjust options.
    ///
    /// Note that if [`precompose_unicode`](crate::store::init::Options::precompose_unicode) is set in the options,
    /// the `git_dir` is also expected to use precomposed unicode, or else some operations that strip prefixes will fail.
    pub fn at(git_dir: PathBuf, object_hash: gix_hash::Kind) -> Self {
        Self::at_opts(git_dir, object_hash, Default::default())
    }

    /// Create a new store at the given location, typically the `.git/` directory.
    /// Use [`opts`](crate::store::init::Options) to adjust settings.
    ///
    /// Note that if [`precompose_unicode`](crate::store::init::Options::precompose_unicode) is set in the options,
    /// the `git_dir` is also expected to use precomposed unicode, or else some operations that strip prefixes will fail.
    pub fn at_opts(git_dir: PathBuf, object_hash: gix_hash::Kind, opts: crate::store::init::Options) -> Self {
        crate::Store {
            inner: crate::store::State::Files {
                store: file::Store::at_opts(git_dir, object_hash, opts),
            },
        }
    }

    /// Create a files-backed store for a linked worktree.
    pub fn for_linked_worktree(git_dir: PathBuf, common_dir: PathBuf, object_hash: gix_hash::Kind) -> Self {
        Self::for_linked_worktree_opts(git_dir, common_dir, object_hash, Default::default())
    }

    /// Create a files-backed store for a linked worktree with `opts`.
    pub fn for_linked_worktree_opts(
        git_dir: PathBuf,
        common_dir: PathBuf,
        object_hash: gix_hash::Kind,
        opts: crate::store::init::Options,
    ) -> Self {
        crate::Store {
            inner: crate::store::State::Files {
                store: file::Store::for_linked_worktree_opts(git_dir, common_dir, object_hash, opts),
            },
        }
    }
}
