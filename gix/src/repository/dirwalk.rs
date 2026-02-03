use std::sync::atomic::AtomicBool;

use crate::{
    bstr::{BStr, BString},
    config, dirwalk, is_dir_to_mode,
    util::OwnedOrStaticAtomicBool,
    worktree::IndexPersistedOrInMemory,
    Repository,
};

impl Repository {
    /// Return default options suitable for performing a directory walk on this repository.
    ///
    /// Used in conjunction with [`dirwalk()`](Self::dirwalk())
    pub fn dirwalk_options(&self) -> Result<dirwalk::Options, config::boolean::Error> {
        Ok(dirwalk::Options::from_fs_caps(self.filesystem_options()?))
    }

    /// Perform a directory walk configured with `options` under control of the `delegate`. Use `patterns` to
    /// further filter entries. `should_interrupt` is polled to see if an interrupt is requested, causing an
    /// error to be returned instead.
    ///
    /// The `index` is used to determine if entries are tracked, and for excludes and attributes
    /// lookup. Note that items will only count as tracked if they have the [`gix_index::entry::Flags::UPTODATE`]
    /// flag set.
    ///
    /// Note that dirwalks for the purpose of deletion will be initialized with the worktrees of this repository
    /// if they fall into the working directory of this repository as well to mark them as `tracked`. That way
    /// it's hard to accidentally flag them for deletion.
    /// This is intentionally not the case when deletion is not intended so they look like
    /// untracked repositories instead.
    ///
    /// See [`gix_dir::walk::delegate::Collect`] for a delegate that collects all seen entries.
    pub fn dirwalk(
        &self,
        index: &gix_index::State,
        patterns: impl IntoIterator<Item = impl AsRef<BStr>>,
        should_interrupt: &AtomicBool,
        options: dirwalk::Options,
        delegate: &mut dyn gix_dir::walk::Delegate,
    ) -> Result<dirwalk::Outcome<'_>, dirwalk::Error> {
        let _span = gix_trace::coarse!("gix::dirwalk");
        let workdir = self.workdir().ok_or(dirwalk::Error::MissingWorkDir)?;
        let mut excludes = self.excludes(
            index,
            None,
            crate::worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped,
        )?;
        let mut pathspec = self.pathspec(
            options.empty_patterns_match_prefix, /* empty patterns match prefix */
            patterns,
            true, /* inherit ignore case */
            index,
            crate::worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
        )?;

        let git_dir_realpath =
            crate::path::realpath_opts(self.git_dir(), self.current_dir(), crate::path::realpath::MAX_SYMLINKS)?;
        let fs_caps = self.filesystem_options()?;
        let accelerate_lookup = fs_caps.ignore_case.then(|| index.prepare_icase_backing());
        let mut opts = gix_dir::walk::Options::from(options);
        let worktree_relative_worktree_dirs_storage;
        if let Some(workdir) = self.workdir().filter(|_| opts.for_deletion.is_some()) {
            let linked_worktrees = self.worktrees()?;
            if !linked_worktrees.is_empty() {
                let real_workdir = gix_path::realpath_opts(
                    workdir,
                    self.options.current_dir_or_empty(),
                    gix_path::realpath::MAX_SYMLINKS,
                )?;
                worktree_relative_worktree_dirs_storage = linked_worktrees
                    .into_iter()
                    .filter_map(|proxy| proxy.base().ok())
                    .filter_map(|base| base.strip_prefix(&real_workdir).map(ToOwned::to_owned).ok())
                    .map(|rela_path| {
                        gix_path::to_unix_separators_on_windows(gix_path::into_bstr(rela_path)).into_owned()
                    })
                    .collect();
                opts.worktree_relative_worktree_dirs = Some(&worktree_relative_worktree_dirs_storage);
            }
        }
        let (outcome, traversal_root) = gix_dir::walk(
            workdir,
            gix_dir::walk::Context {
                should_interrupt: Some(should_interrupt),
                git_dir_realpath: git_dir_realpath.as_ref(),
                current_dir: self.current_dir(),
                index,
                ignore_case_index_lookup: accelerate_lookup.as_ref(),
                pathspec: &mut pathspec.search,
                pathspec_attributes: &mut |relative_path, case, is_dir, out| {
                    let stack = pathspec
                        .stack
                        .as_mut()
                        .expect("can only be called if attributes are used in patterns");
                    stack
                        .set_case(case)
                        .at_entry(relative_path, Some(is_dir_to_mode(is_dir)), &self.objects)
                        .is_ok_and(|platform| platform.matching_attributes(out))
                },
                excludes: Some(&mut excludes.inner),
                objects: &self.objects,
                explicit_traversal_root: (!options.empty_patterns_match_prefix).then_some(workdir),
            },
            opts,
            delegate,
        )?;

        Ok(dirwalk::Outcome {
            dirwalk: outcome,
            traversal_root,
            excludes,
            pathspec,
        })
    }

    /// Create an iterator over a running traversal, which stops if the iterator is dropped. All arguments
    /// are the same as in [`dirwalk()`](Self::dirwalk).
    ///
    /// `should_interrupt` should be set to `Default::default()` if it is supposed to be unused.
    /// Otherwise, it can be created by passing a `&'static AtomicBool`, `&Arc<AtomicBool>` or `Arc<AtomicBool>`.
    pub fn dirwalk_iter(
        &self,
        index: impl Into<IndexPersistedOrInMemory>,
        patterns: impl IntoIterator<Item = impl Into<BString>>,
        should_interrupt: OwnedOrStaticAtomicBool,
        options: dirwalk::Options,
    ) -> Result<dirwalk::Iter, dirwalk::iter::Error> {
        dirwalk::Iter::new(
            self,
            index.into(),
            patterns.into_iter().map(Into::into).collect(),
            should_interrupt,
            options,
        )
    }

    /// Perform a directory walk configured with `options`, calling `for_each` for each entry. Use `patterns` to
    /// further filter entries. `should_interrupt` is polled to see if an interrupt is requested, causing an
    /// error to be returned instead.
    ///
    /// This is a convenience method that wraps [`dirwalk()`](Self::dirwalk) and provides a simpler interface
    /// for processing entries via a closure rather than implementing the [`gix_dir::walk::Delegate`] trait.
    ///
    /// The `index` is used to determine if entries are tracked, and for excludes and attributes
    /// lookup. Note that items will only count as tracked if they have the [`gix_index::entry::Flags::UPTODATE`]
    /// flag set.
    ///
    /// The closure receives a [`dirwalk::for_each::Entry`] for each entry in the walk and should return
    /// a [`gix_dir::walk::Action`] to control traversal flow:
    /// - `std::ops::ControlFlow::Continue(())` to continue the traversal
    /// - `std::ops::ControlFlow::Break(())` to stop the traversal
    ///
    /// # Example
    ///
    /// ```no_run
    /// use gix::dirwalk::for_each::Entry;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let repo = gix::discover(".")?;
    /// let index = repo.index()?;
    /// let options = repo.dirwalk_options()?;
    ///
    /// repo.dirwalk_for_each(
    ///     &index,
    ///     std::iter::empty::<&str>(),
    ///     &Default::default(),
    ///     options,
    ///     |entry: Entry<'_>| -> Result<gix::dir::walk::Action, std::convert::Infallible> {
    ///         println!("{}", entry.entry.rela_path);
    ///         Ok(std::ops::ControlFlow::Continue(()))
    ///     },
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn dirwalk_for_each<E>(
        &self,
        index: &gix_index::State,
        patterns: impl IntoIterator<Item = impl AsRef<BStr>>,
        should_interrupt: &AtomicBool,
        options: dirwalk::Options,
        mut for_each: impl FnMut(dirwalk::for_each::Entry<'_>) -> Result<gix_dir::walk::Action, E>,
    ) -> Result<dirwalk::Outcome<'_>, dirwalk::for_each::Error>
    where
        E: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    {
        struct ForEachDelegate<F, E> {
            for_each: F,
            error: Option<E>,
        }

        impl<F, E> gix_dir::walk::Delegate for ForEachDelegate<F, E>
        where
            F: FnMut(dirwalk::for_each::Entry<'_>) -> Result<gix_dir::walk::Action, E>,
        {
            fn emit(
                &mut self,
                entry: gix_dir::EntryRef<'_>,
                collapsed_directory_status: Option<gix_dir::entry::Status>,
            ) -> gix_dir::walk::Action {
                match (self.for_each)(dirwalk::for_each::Entry::new(entry, collapsed_directory_status)) {
                    Ok(action) => action,
                    Err(err) => {
                        self.error = Some(err);
                        std::ops::ControlFlow::Break(())
                    }
                }
            }
        }

        let mut delegate = ForEachDelegate {
            for_each: &mut for_each,
            error: None,
        };

        match self.dirwalk(index, patterns, should_interrupt, options, &mut delegate) {
            Ok(outcome) => {
                if let Some(err) = delegate.error {
                    Err(dirwalk::for_each::Error::ForEach(err.into()))
                } else {
                    Ok(outcome)
                }
            }
            Err(err) => Err(dirwalk::for_each::Error::Dirwalk(err)),
        }
    }
}
