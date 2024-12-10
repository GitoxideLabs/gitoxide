use crate::dirwalk::Options;
use gix_dir::walk::{CollapsedEntriesEmissionMode, EmissionMode, ForDeletionMode};

/// Construction
impl Options {
    pub(crate) fn from_fs_caps(caps: gix_fs::Capabilities) -> Self {
        Self {
            precompose_unicode: caps.precompose_unicode,
            ignore_case: caps.ignore_case,
            recurse_repositories: false,
            emit_pruned: false,
            emit_ignored: None,
            for_deletion: None,
            emit_tracked: false,
            emit_untracked: Default::default(),
            emit_empty_directories: false,
            classify_untracked_bare_repositories: false,
            emit_collapsed: None,
            empty_patterns_match_prefix: false,
            symlinks_to_directories_are_ignored_like_directories: false,
        }
    }
}

impl From<Options> for gix_dir::walk::Options<'static> {
    fn from(v: Options) -> Self {
        gix_dir::walk::Options {
            precompose_unicode: v.precompose_unicode,
            ignore_case: v.ignore_case,
            recurse_repositories: v.recurse_repositories,
            emit_pruned: v.emit_pruned,
            emit_ignored: v.emit_ignored,
            for_deletion: v.for_deletion,
            emit_tracked: v.emit_tracked,
            emit_untracked: v.emit_untracked,
            emit_empty_directories: v.emit_empty_directories,
            classify_untracked_bare_repositories: v.classify_untracked_bare_repositories,
            emit_collapsed: v.emit_collapsed,
            symlinks_to_directories_are_ignored_like_directories: v
                .symlinks_to_directories_are_ignored_like_directories,
            worktree_relative_worktree_dirs: None,
        }
    }
}

impl Options {
    /// If `true`, default `false`, pathspecs and the directory walk itself will be setup to use the [prefix](crate::Repository::prefix)
    /// if patterns are empty.
    ///
    /// This means that the directory walk will be limited to only what's inside the [repository prefix](crate::Repository::prefix).
    /// By default, the directory walk will see everything.
    pub fn empty_patterns_match_prefix(mut self, toggle: bool) -> Self {
        self.empty_patterns_match_prefix = toggle;
        self
    }
    /// Like [`empty_patterns_match_prefix()`](Self::empty_patterns_match_prefix), but only requires a mutably borrowed instance.
    pub fn set_empty_patterns_match_prefix(&mut self, toggle: bool) -> &mut Self {
        self.empty_patterns_match_prefix = toggle;
        self
    }
    /// If `toggle` is `true`, we will stop figuring out if any directory that is a candidate for recursion is also a nested repository,
    /// which saves time but leads to recurse into it. If `false`, nested repositories will not be traversed.
    pub fn recurse_repositories(mut self, toggle: bool) -> Self {
        self.recurse_repositories = toggle;
        self
    }
    /// Like [`recurse_repositories()`](Self::recurse_repositories), but only requires a mutably borrowed instance.
    pub fn set_recurse_repositories(&mut self, toggle: bool) -> &mut Self {
        self.recurse_repositories = toggle;
        self
    }
    /// If `toggle` is `true`, entries that are pruned and whose [Kind](gix_dir::entry::Kind) is known will be emitted.
    pub fn emit_pruned(mut self, toggle: bool) -> Self {
        self.emit_pruned = toggle;
        self
    }
    /// Like [`emit_pruned()`](Self::emit_pruned), but only requires a mutably borrowed instance.
    pub fn set_emit_pruned(&mut self, toggle: bool) -> &mut Self {
        self.emit_pruned = toggle;
        self
    }
    /// If `value` is `Some(mode)`, entries that are ignored will be emitted according to the given `mode`.
    /// If `None`, ignored entries will not be emitted at all.
    pub fn emit_ignored(mut self, value: Option<EmissionMode>) -> Self {
        self.emit_ignored = value;
        self
    }
    /// Like [`emit_ignored()`](Self::emit_ignored), but only requires a mutably borrowed instance.
    pub fn set_emit_ignored(&mut self, value: Option<EmissionMode>) -> &mut Self {
        self.emit_ignored = value;
        self
    }
    /// When the walk is for deletion, `value` must be `Some(_)` to assure we don't collapse directories that have precious files in
    /// them, and otherwise assure that no entries are observable that shouldn't be deleted.
    /// If `None`, precious files are treated like expendable files, which is usually what you want when displaying them
    /// for addition to the repository, and the collapse of folders can be more generous in relation to ignored files.
    pub fn for_deletion(mut self, value: Option<ForDeletionMode>) -> Self {
        self.for_deletion = value;
        self
    }
    /// Like [`for_deletion()`](Self::for_deletion), but only requires a mutably borrowed instance.
    pub fn set_for_deletion(&mut self, value: Option<ForDeletionMode>) -> &mut Self {
        self.for_deletion = value;
        self
    }
    /// If `toggle` is `true`, we will also emit entries for tracked items. Otherwise these will remain 'hidden',
    /// even if a pathspec directly refers to it.
    pub fn emit_tracked(mut self, toggle: bool) -> Self {
        self.emit_tracked = toggle;
        self
    }
    /// Like [`emit_tracked()`](Self::emit_tracked), but only requires a mutably borrowed instance.
    pub fn set_emit_tracked(&mut self, toggle: bool) -> &mut Self {
        self.emit_tracked = toggle;
        self
    }
    /// Controls the way untracked files are emitted. By default, this is happening immediately and without any simplification.
    pub fn emit_untracked(mut self, mode: EmissionMode) -> Self {
        self.emit_untracked = mode;
        self
    }
    /// Like [`emit_untracked()`](Self::emit_untracked), but only requires a mutably borrowed instance.
    pub fn set_emit_untracked(&mut self, mode: EmissionMode) -> &mut Self {
        self.emit_untracked = mode;
        self
    }
    /// If `toggle` is `true`, emit empty directories as well. Note that a directory also counts as empty if it has any
    /// amount or depth of nested subdirectories, as long as none of them includes a file.
    /// Thus, this makes leaf-level empty directories visible, as those don't have any content.
    pub fn emit_empty_directories(mut self, toggle: bool) -> Self {
        self.emit_empty_directories = toggle;
        self
    }

    /// Like [`emit_empty_directories()`](Self::emit_empty_directories), but only requires a mutably borrowed instance.
    pub fn set_emit_empty_directories(&mut self, toggle: bool) -> &mut Self {
        self.emit_empty_directories = toggle;
        self
    }

    /// If `toggle` is `true`, we will not only find non-bare repositories in untracked directories, but also bare ones.
    ///
    /// Note that this is very costly, but without it, bare repositories will appear like untracked directories when collapsed,
    /// and they will be recursed into.
    pub fn classify_untracked_bare_repositories(mut self, toggle: bool) -> Self {
        self.classify_untracked_bare_repositories = toggle;
        self
    }

    /// Like [`classify_untracked_bare_repositories()`](Self::classify_untracked_bare_repositories), but only requires a mutably borrowed instance.
    pub fn set_classify_untracked_bare_repositories(&mut self, toggle: bool) -> &mut Self {
        self.classify_untracked_bare_repositories = toggle;
        self
    }

    /// Control whether entries that are in an about-to-be collapsed directory will be emitted. The default is `None`,
    /// so entries in a collapsed directory are not observable.
    pub fn emit_collapsed(mut self, value: Option<CollapsedEntriesEmissionMode>) -> Self {
        self.emit_collapsed = value;
        self
    }

    /// Like [`emit_collapsed()`](Self::emit_collapsed), but only requires a mutably borrowed instance.
    pub fn set_emit_collapsed(&mut self, value: Option<CollapsedEntriesEmissionMode>) -> &mut Self {
        self.emit_collapsed = value;
        self
    }

    /// This is a `libgit2` compatibility flag, and if enabled, symlinks that point to directories will be considered a directory
    /// when checking for exclusion.
    ///
    /// This is relevant if `src2` points to `src`, and is excluded with `src2/`. If `false`, `src2` will not be excluded,
    /// if `true` it will be excluded as the symlink is considered a directory.
    ///
    /// In other words, for Git compatibility this flag should be `false`, the default, for `git2` compatibility it should be `true`.
    pub fn symlinks_to_directories_are_ignored_like_directories(mut self, toggle: bool) -> Self {
        self.symlinks_to_directories_are_ignored_like_directories = toggle;
        self
    }

    /// Like [`symlinks_to_directories_are_ignored_like_directories()`](Self::symlinks_to_directories_are_ignored_like_directories),
    /// but only requires a mutably borrowed instance.
    pub fn set_symlinks_to_directories_are_ignored_like_directories(&mut self, toggle: bool) -> &mut Self {
        self.symlinks_to_directories_are_ignored_like_directories = toggle;
        self
    }
}
