use std::sync::atomic::AtomicBool;

use crate::status::{OwnedOrStaticAtomicBool, Platform, Submodule, UntrackedFiles, index_worktree, tree_index};

/// Builder
impl<Progress> Platform<'_, Progress>
where
    Progress: gix_features::progress::Progress,
{
    /// Call `cb` on dirwalk options if these are set (which is the default when created through [`Repository::status()`](crate::Repository::status())).
    /// The directory walk is used to find untracked files or ignored files.
    ///
    /// `cb` will be able to run builder-methods on the passed dirwalk options.
    pub fn dirwalk_options(mut self, cb: impl FnOnce(crate::dirwalk::Options) -> crate::dirwalk::Options) -> Self {
        if let Some(opts) = self.index_worktree_options.dirwalk_options.take() {
            self.index_worktree_options.dirwalk_options = Some(cb(opts));
        }
        self
    }
    /// Like [dirwalk_options()](Self::dirwalk_options), but taking a mutable instance instead.
    pub fn dirwalk_options_mut(&mut self, cb: impl FnOnce(&mut crate::dirwalk::Options)) -> &mut Self {
        if let Some(opts) = self.index_worktree_options.dirwalk_options.as_mut() {
            cb(opts);
        }
        self
    }
    /// A simple way to explicitly set the desired way of listing `untracked_files`, overriding any value
    /// set by the git configuration.
    ///
    /// Note that if [`None`](UntrackedFiles::None) is used, the directory walk will be disabled entirely
    /// after this call. Further, if no dirwalk options are present anymore, this call has no effect.
    pub fn untracked_files(mut self, untracked_files: UntrackedFiles) -> Self {
        let mode = match untracked_files {
            UntrackedFiles::None => {
                self.index_worktree_options.dirwalk_options.take();
                return self;
            }
            UntrackedFiles::Collapsed => gix_dir::walk::EmissionMode::CollapseDirectory,
            UntrackedFiles::Files => gix_dir::walk::EmissionMode::Matching,
        };
        self.dirwalk_options(|cb| cb.emit_untracked(mode))
    }

    /// Set the interrupt flag to `should_interrupt`, which typically is an application-wide flag
    /// that is ultimately controlled by user interrupts.
    ///
    /// If it is `true`, the iteration will stop immediately.
    pub fn should_interrupt_shared(mut self, should_interrupt: &'static AtomicBool) -> Self {
        self.should_interrupt = Some(OwnedOrStaticAtomicBool::Shared(should_interrupt));
        self
    }

    /// Set the interrupt flag to `should_interrupt`, as controlled by the caller.
    ///
    /// If it is `true`, the iteration will stop immediately.
    pub fn should_interrupt_owned(mut self, should_interrupt: std::sync::Arc<AtomicBool>) -> Self {
        self.should_interrupt = Some(OwnedOrStaticAtomicBool::Owned {
            flag: should_interrupt,
            private: false,
        });
        self
    }

    /// Configure how the `submodule_status` is obtained when looking at submodules that are still mentioned in the index.
    // If `None` is given, no submodule status check is performed.
    pub fn index_worktree_submodules(mut self, submodules: impl Into<Option<Submodule>>) -> Self {
        let submodules = submodules.into();
        self.submodules = match submodules {
            None => Submodule::Given {
                ignore: crate::submodule::config::Ignore::All,
                check_dirty: false,
            },
            Some(status) => status,
        };
        self
    }

    /// Set the `index` to use when making comparisons to the worktree and the head revision.
    ///
    /// Defaults to the current index, or an empty one if it doesn't exist (yet).
    pub fn index(mut self, index: crate::worktree::IndexPersistedOrInMemory) -> Self {
        self.index = Some(index);
        self
    }

    /// Configure the index-to-worktree rename tracking with `rewrites`, which is `None` by default.
    ///
    /// Note that Git does not have configuration related to rename tracking of changes between the index
    /// and the worktree. The closest there is can be obtained using [`crate::diff::new_rewrites()`], which refers
    /// to rename tracking between trees.
    ///
    /// Also note that if `rewrites` are `Some()`, [`sorting`](index_worktree::Options::sorting) will automatically be
    /// configured to assure deterministic outcomes for rewrite solutions.
    pub fn index_worktree_rewrites(mut self, rewrites: impl Into<Option<gix_diff::Rewrites>>) -> Self {
        let rewrites = rewrites.into();
        self.index_worktree_options.rewrites = rewrites;
        if rewrites.is_some() && self.index_worktree_options.sorting.is_none() {
            self.index_worktree_options.sorting =
                Some(gix_status::index_as_worktree_with_renames::Sorting::ByPathCaseSensitive);
        }
        self
    }

    /// Adjust all options related to the index-worktree status.
    /// This is a catch-all in case there are no more specific methods that could be used instead to change
    /// the respective option.
    pub fn index_worktree_options_mut(mut self, cb: impl FnOnce(&mut index_worktree::Options)) -> Self {
        cb(&mut self.index_worktree_options);
        self
    }

    /// Set the tree at which the `HEAD` ref ought to reside.
    /// Setting this explicitly allows to compare the index to a tree that it possibly didn't originate from.
    ///
    /// If not set explicitly, it will be read via `HEAD^{tree}`.
    pub fn head_tree(mut self, tree: impl Into<gix_hash::ObjectId>) -> Self {
        self.head_tree = Some(Some(tree.into()));
        self
    }

    /// Configure how rename tracking should be performed when looking at changes between the [head tree](Self::head_tree())
    /// and the index.
    pub fn tree_index_track_renames(mut self, renames: tree_index::TrackRenames) -> Self {
        self.tree_index_renames = renames;
        self
    }
}
