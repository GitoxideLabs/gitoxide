pub use gix_merge as plumbing;
pub use gix_merge::blob;

///
pub mod virtual_merge_base {
    use crate::Id;

    /// The outcome produced by [`Repository::virtual_merge_base()`](crate::Repository::virtual_merge_base()).
    pub struct Outcome<'repo> {
        /// The commit ids of all the virtual merge bases we have produced in the process of recursively merging the merge-bases.
        /// As they have been written to the object database, they are still available until they are garbage collected.
        /// The last one is the most recently produced and the one returned as `commit_id`.
        /// If this list is empty, this means that there was only one merge-base, which itself is already suitable the final merge-base.
        pub virtual_merge_bases: Vec<Id<'repo>>,
        /// The id of the commit that was created to hold the merged tree.
        pub commit_id: Id<'repo>,
        /// The hash of the merged tree.
        pub tree_id: Id<'repo>,
    }
}

///
pub mod commit {
    /// The outcome produced by [`Repository::merge_commits()`](crate::Repository::merge_commits()).
    #[derive(Clone)]
    pub struct Outcome<'a> {
        /// The outcome of the actual tree-merge, with the tree editor to write to obtain the actual tree id.
        pub tree_merge: crate::merge::tree::Outcome<'a>,
        /// The tree id of the base commit we used. This is either…
        /// * the single merge-base we found
        /// * the first of multiple merge-bases if [Options::with_use_first_merge_base()] was `true`.
        /// * the merged tree of all merge-bases, which then isn't linked to an actual commit.
        /// * an empty tree, if [Options::with_allow_missing_merge_base()] is enabled.
        pub merge_base_tree_id: gix_hash::ObjectId,
        /// The object ids of all the commits which were found to be merge-bases, or `None` if there was no merge-base.
        pub merge_bases: Option<gix_revision::merge_base::Bases>,
        /// A list of virtual commits that were created to merge multiple merge-bases into one, the last one being
        /// the one we used as merge-base for the merge.
        /// As they are not reachable by anything they will be garbage collected, but knowing them provides options.
        /// Would be empty if no virtual commit was needed at all as there was only a single merge-base.
        /// Otherwise, the last commit id is the one with the `merge_base_tree_id`.
        pub virtual_merge_bases: Vec<gix_hash::ObjectId>,
    }

    /// A way to configure [`Repository::merge_commits()`](crate::Repository::merge_commits()).
    #[derive(Default, Debug, Clone)]
    pub struct Options {
        allow_missing_merge_base: bool,
        tree_merge: crate::merge::tree::Options,
        use_first_merge_base: bool,
    }

    impl From<gix_merge::tree::Options> for Options {
        fn from(value: gix_merge::tree::Options) -> Self {
            Options {
                tree_merge: value.into(),
                use_first_merge_base: false,
                allow_missing_merge_base: false,
            }
        }
    }

    impl From<crate::merge::tree::Options> for Options {
        fn from(value: crate::merge::tree::Options) -> Self {
            Options {
                tree_merge: value,
                use_first_merge_base: false,
                allow_missing_merge_base: false,
            }
        }
    }

    impl From<Options> for gix_merge::commit::Options {
        fn from(
            Options {
                allow_missing_merge_base,
                tree_merge,
                use_first_merge_base,
            }: Options,
        ) -> Self {
            gix_merge::commit::Options {
                allow_missing_merge_base,
                tree_merge: tree_merge.into(),
                use_first_merge_base,
            }
        }
    }

    /// Builder
    impl Options {
        /// If `true`, merging unrelated commits is allowed, with the merge-base being assumed as empty tree.
        pub fn with_allow_missing_merge_base(mut self, allow_missing_merge_base: bool) -> Self {
            self.allow_missing_merge_base = allow_missing_merge_base;
            self
        }

        /// If `true`, do not merge multiple merge-bases into one. Instead, just use the first one.
        #[doc(alias = "no_recursive", alias = "git2")]
        pub fn with_use_first_merge_base(mut self, use_first_merge_base: bool) -> Self {
            self.use_first_merge_base = use_first_merge_base;
            self
        }
    }
}

///
pub mod tree {
    use gix_merge::blob::builtin_driver;
    pub use gix_merge::tree::{
        apply_index_entries, treat_as_unresolved, Conflict, ContentMerge, Resolution, ResolutionFailure,
        TreatAsUnresolved,
    };

    /// The outcome produced by [`Repository::merge_trees()`](crate::Repository::merge_trees()).
    #[derive(Clone)]
    pub struct Outcome<'repo> {
        /// The ready-made (but unwritten) *base* tree, including all non-conflicting changes, and the changes that had
        /// conflicts which could be resolved automatically.
        ///
        /// This means, if all of their changes were conflicting, this will be equivalent to the *base* tree.
        pub tree: crate::object::tree::Editor<'repo>,
        /// The set of conflicts we encountered. Can be empty to indicate there was no conflict.
        /// Note that conflicts might have been auto-resolved, but they are listed here for completeness.
        /// Use [`has_unresolved_conflicts()`](Outcome::has_unresolved_conflicts()) to see if any action is needed
        /// before using [`tree`](Outcome::tree).
        pub conflicts: Vec<Conflict>,
        /// `true` if `conflicts` contains only a single *unresolved* conflict in the last slot, but possibly more resolved ones.
        /// This also makes this outcome a very partial merge that cannot be completed.
        pub failed_on_first_unresolved_conflict: bool,
    }

    impl Outcome<'_> {
        /// Return `true` if there is any conflict that would still need to be resolved as they would yield undesirable trees.
        /// This is based on `how` to determine what should be considered unresolved.
        pub fn has_unresolved_conflicts(&self, how: TreatAsUnresolved) -> bool {
            self.conflicts.iter().any(|c| c.is_unresolved(how))
        }

        /// Returns `true` if `index` changed as we applied conflicting stages to it, using `how` to determine if a
        /// conflict should be considered unresolved.
        ///
        /// `removal_mode` decides how unconflicted entries should be removed if they are superseded by
        /// their conflicted counterparts.
        ///
        /// It's important that `index` is at the state of [`Self::tree`].
        /// Note that in practice, whenever there is a single [conflict](Conflict), this function will return `true`.
        pub fn index_changed_after_applying_conflicts(
            &self,
            index: &mut gix_index::State,
            how: TreatAsUnresolved,
            removal_mode: apply_index_entries::RemovalMode,
        ) -> bool {
            apply_index_entries(&self.conflicts, how, index, removal_mode)
        }
    }

    /// A way to configure [`Repository::merge_trees()`](crate::Repository::merge_trees()).
    #[derive(Default, Debug, Clone)]
    pub struct Options {
        inner: gix_merge::tree::Options,
        file_favor: Option<FileFavor>,
        tree_favor: Option<TreeFavor>,
    }

    impl From<gix_merge::tree::Options> for Options {
        fn from(opts: gix_merge::tree::Options) -> Self {
            Options {
                inner: opts,
                file_favor: None,
                tree_favor: None,
            }
        }
    }

    impl From<Options> for gix_merge::tree::Options {
        fn from(value: Options) -> Self {
            let mut opts = value.inner;
            if let Some(file_favor) = value.file_favor {
                let (resolve_binary, resolve_text) = match file_favor {
                    FileFavor::Ours => (
                        builtin_driver::binary::ResolveWith::Ours,
                        builtin_driver::text::Conflict::ResolveWithOurs,
                    ),
                    FileFavor::Theirs => (
                        builtin_driver::binary::ResolveWith::Theirs,
                        builtin_driver::text::Conflict::ResolveWithTheirs,
                    ),
                };

                opts.symlink_conflicts = Some(resolve_binary);
                opts.blob_merge.resolve_binary_with = Some(resolve_binary);
                opts.blob_merge.text.conflict = resolve_text;
            }
            opts.tree_conflicts = value.tree_favor.map(Into::into);
            opts
        }
    }

    /// Identify how files should be resolved in case of conflicts.
    ///
    /// This works for…
    ///
    /// * content merges
    /// * binary files
    /// * symlinks (a form of file after all)
    ///
    /// Note that *union* merges aren't available as they aren't available for binaries or symlinks.
    #[derive(Debug, Copy, Clone)]
    pub enum FileFavor {
        /// Choose *our* side in case of a conflict.
        /// Note that this choice is precise, so *ours* hunk will only be chosen if they conflict with *theirs*,
        /// so *their* hunks may still show up in the merged result.
        Ours,
        /// Choose *their* side in case of a conflict.
        /// Note that this choice is precise, so *ours* hunk will only be chosen if they conflict with *theirs*,
        /// so *their* hunks may still show up in the merged result.
        Theirs,
    }

    /// Control how irreconcilable changes to trees should be resolved.
    ///
    /// Examples for such issues are:
    ///
    /// * *we*: delete, *they*: modify
    /// * *we*: rename, *they*: rename to something else
    /// * *we*: delete, *they*: rename
    ///
    /// Use this to control which entries are visible to in the resulting tree.
    /// Also note that this does not apply to the many tree-related changes are reconcilable.
    #[derive(Debug, Copy, Clone)]
    pub enum TreeFavor {
        /// Choose *our* side in case of a conflict.
        /// Note that content-merges are *still* performed according to the [FileFavor].
        Ours,
        /// Choose the state of the shared common ancestor, dropping both *ours* and *their* changes.
        /// Content merges are not performed here.
        Ancestor,
    }

    impl From<TreeFavor> for gix_merge::tree::ResolveWith {
        fn from(value: TreeFavor) -> Self {
            match value {
                TreeFavor::Ours => gix_merge::tree::ResolveWith::Ours,
                TreeFavor::Ancestor => gix_merge::tree::ResolveWith::Ancestor,
            }
        }
    }

    /// Builder
    impl Options {
        /// If *not* `None`, rename tracking will be performed when determining the changes of each side of the merge.
        pub fn with_rewrites(mut self, rewrites: Option<gix_diff::Rewrites>) -> Self {
            self.inner.rewrites = rewrites;
            self
        }

        /// If `Some(what-is-unresolved)`, the first unresolved conflict will cause the entire merge to stop.
        /// This is useful to see if there is any conflict, without performing the whole operation, something
        /// that can be very relevant during merges that would cause a lot of blob-diffs.
        pub fn with_fail_on_conflict(mut self, fail_on_conflict: Option<TreatAsUnresolved>) -> Self {
            self.inner.fail_on_conflict = fail_on_conflict;
            self
        }

        /// When `None`, the default, both sides will be treated equally, and in case of conflict an unbiased representation
        /// is chosen both for content and for trees, causing a conflict.
        ///
        /// With `Some(favor)` one can choose a side to prefer in order to forcefully resolve an otherwise irreconcilable conflict,
        /// loosing information in the process.
        pub fn with_file_favor(mut self, file_favor: Option<FileFavor>) -> Self {
            self.file_favor = file_favor;
            self
        }

        /// When `None`, the default, both sides will be treated equally, trying to keep both conflicting changes in the tree, possibly
        /// by renaming one side to move it out of the way.
        ///
        /// With `Some(favor)` one can choose a side to prefer in order to forcefully resolve an otherwise irreconcilable conflict,
        /// loosing information in the process.
        pub fn with_tree_favor(mut self, tree_favor: Option<TreeFavor>) -> Self {
            self.tree_favor = tree_favor;
            self
        }
    }
}
