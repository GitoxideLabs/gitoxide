use gix_diff::tree;

use crate::{bstr::BStr, Id, Tree};

/// Returned by the `for_each` function to control flow.
///
/// Use [`std::ops::ControlFlow::Continue`] to continue the traversal of changes.
/// Use [`std::ops::ControlFlow::Break`] to stop the traversal of changes and stop calling this function.
pub type Action = std::ops::ControlFlow<()>;

pub use gix_diff::tree_with_rewrites::Change as ChangeDetached;

/// Represents any possible change in order to turn one tree into another.
#[derive(Debug, Clone, Copy)]
pub enum Change<'a, 'old, 'new> {
    /// An entry was added, like the addition of a file or directory.
    Addition {
        /// The location of the file or directory, if [tracking](crate::diff::Options::track_path) was enabled.
        ///
        /// It may be empty if neither [file names](crate::diff::Options::track_filename()) nor [file paths](crate::diff::Options::track_path())
        /// are tracked.
        location: &'a BStr,
        /// The mode of the added entry.
        entry_mode: gix_object::tree::EntryMode,
        /// Identifies a relationship between this instance and another one,
        /// making it easy to reconstruct the top-level of directory changes.
        relation: Option<tree::visit::Relation>,
        /// The object id of the added entry.
        id: Id<'new>,
    },
    /// An entry was deleted, like the deletion of a file or directory.
    Deletion {
        /// The location of the file or directory, if [tracking](crate::diff::Options::track_path) was enabled.
        ///
        /// Otherwise, this value is always an empty path.
        location: &'a BStr,
        /// The mode of the deleted entry.
        entry_mode: gix_object::tree::EntryMode,
        /// Identifies a relationship between this instance and another one,
        /// making it easy to reconstruct the top-level of directory changes.
        relation: Option<tree::visit::Relation>,
        /// The object id of the deleted entry.
        id: Id<'old>,
    },
    /// An entry was modified, e.g. changing the contents of a file adjusts its object id and turning
    /// a file into a symbolic link adjusts its mode.
    Modification {
        /// The location of the file or directory, if [tracking](crate::diff::Options::track_path) was enabled.
        ///
        /// It may be empty if neither [file names](crate::diff::Options::track_filename()) nor [file paths](crate::diff::Options::track_path())
        /// are tracked.
        location: &'a BStr,
        /// The mode of the entry before the modification.
        previous_entry_mode: gix_object::tree::EntryMode,
        /// The object id of the entry before the modification.
        previous_id: Id<'old>,

        /// The mode of the entry after the modification.
        entry_mode: gix_object::tree::EntryMode,
        /// The object id after the modification.
        id: Id<'new>,
    },
    /// Entries are considered rewritten if they are not trees and they, according to some understanding of identity, were renamed
    /// or copied.
    /// In case of renames, this means they originally appeared as [`Deletion`](Change::Deletion) signalling their source as well as an
    /// [`Addition`](Change::Addition) acting as destination.
    ///
    /// In case of copies, the `copy` flag is true and typically represents a perfect copy of a source was made.
    ///
    /// This variant can only be encountered if [rewrite tracking](crate::diff::Options::track_rewrites()) is enabled.
    ///
    /// Note that mode changes may have occurred as well, i.e. changes from executable to non-executable or vice-versa.
    Rewrite {
        /// The location of the source of the rename operation.
        ///
        /// It may be empty if neither [file names](crate::diff::Options::track_filename()) nor [file paths](crate::diff::Options::track_path())
        /// are tracked.
        source_location: &'a BStr,
        /// Identifies a relationship between the source and another source,
        /// making it easy to reconstruct the top-level of directory changes.
        source_relation: Option<tree::visit::Relation>,
        /// The mode of the entry before the rename.
        source_entry_mode: gix_object::tree::EntryMode,
        /// The object id of the entry before the rename.
        ///
        /// Note that this is the same as `id` if we require the [similarity to be 100%](gix_diff::Rewrites::percentage), but may
        /// be different otherwise.
        source_id: Id<'old>,
        /// Information about the diff we performed to detect similarity and match the `source_id` with the current state at `id`.
        /// It's `None` if `source_id` is equal to `id`, as identity made an actual diff computation unnecessary.
        diff: Option<gix_diff::blob::DiffLineStats>,
        /// The mode of the entry after the rename.
        /// It could differ but still be considered a rename as we are concerned only about content.
        entry_mode: gix_object::tree::EntryMode,
        /// The location of the destination file or directory, if [tracking](crate::diff::Options::track_path) was enabled.
        ///
        /// It may be empty if neither [file names](crate::diff::Options::track_filename()) nor [file paths](crate::diff::Options::track_path())
        /// are tracked.
        location: &'a BStr,
        /// The object id after the rename.
        id: Id<'new>,
        /// Identifies a relationship between this destination and another destination,
        /// making it easy to reconstruct the top-level of directory changes.
        relation: Option<tree::visit::Relation>,
        /// If true, this rewrite is created by copy, and `source_id` is pointing to its source. Otherwise, it's a rename, and `source_id`
        /// points to a deleted object, as renames are tracked as deletions and additions of the same or similar content.
        copy: bool,
    },
}

///
pub mod change;

/// Diffing
impl<'repo> Tree<'repo> {
    /// Return a platform to see the changes needed to create other trees, for instance.
    ///
    /// # Performance
    ///
    /// It's highly recommended to [set an object cache](crate::Repository::compute_object_cache_size_for_tree_diffs)
    /// to avoid extracting the same object multiple times.
    /// By default, similar to `git diff`, rename tracking will be enabled if it is not configured.
    ///
    /// Note that if a clone with `--filter=blob=none` was created, rename tracking may fail as it might
    /// try to access blobs to compute a similarity metric. Thus, it's more compatible to turn rewrite tracking off
    /// using [`Options::track_rewrites()`](crate::diff::Options::track_rewrites()).
    #[allow(clippy::result_large_err)]
    #[doc(alias = "diff_tree_to_tree", alias = "git2")]
    pub fn changes<'a>(&'a self) -> Result<Platform<'a, 'repo>, crate::diff::options::init::Error> {
        Ok(Platform {
            state: Default::default(),
            lhs: self,
            options: crate::diff::Options::from_configuration(&self.repo.config)?,
        })
    }
}

/// The diffing platform returned by [`Tree::changes()`].
#[derive(Clone)]
pub struct Platform<'a, 'repo> {
    state: gix_diff::tree::State,
    lhs: &'a Tree<'repo>,
    options: crate::diff::Options,
}

impl Platform<'_, '_> {
    /// Adjust diff options with `change_opts`.
    pub fn options(&mut self, change_opts: impl FnOnce(&mut crate::diff::Options)) -> &mut Self {
        change_opts(&mut self.options);
        self
    }
}

/// Provide aggregated information of a diff between two trees.
#[derive(Default, Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[doc(alias = "DiffStats", alias = "git2")]
pub struct Stats {
    /// The total amount of lines added in the between blobs of the two trees.
    #[doc(alias = "insertions", alias = "git2")]
    pub lines_added: u64,
    /// The total amount of lines removed in the between blobs of the two trees.
    #[doc(alias = "deletions", alias = "git2")]
    pub lines_removed: u64,
    /// The number of files that contributed to these statistics as they were added, removed or modified.
    pub files_changed: u64,
}

///
pub mod stats {
    /// The error returned by [`stats()`](super::Platform::stats()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        CreateResourceCache(#[from] crate::repository::diff_resource_cache::Error),
        #[error(transparent)]
        ForEachChange(#[from] crate::object::tree::diff::for_each::Error),
    }
}

/// Convenience
impl Platform<'_, '_> {
    /// Calculate statistics about the lines of the diff between our current and the `other` tree.
    ///
    /// ### Performance Notes
    ///
    /// Be sure to forcefully disable [`track_rewrites(None)`](crate::diff::Options::track_rewrites) to avoid
    /// rename tracking, an operation that doesn't affect the statistics currently.
    /// As diffed resources aren't cached, if highly repetitive blobs are expected, performance
    /// may be diminished. In real-world scenarios where blobs are mostly unique, that's not an issue though.
    pub fn stats(&mut self, other: &Tree<'_>) -> Result<Stats, stats::Error> {
        // let (mut number_of_files, mut lines_added, mut lines_removed) = (0, 0, 0);
        let mut resource_cache = self.lhs.repo.diff_resource_cache_for_tree_diff()?;

        let (mut files_changed, mut lines_added, mut lines_removed) = (0, 0, 0);
        self.for_each_to_obtain_tree(other, |change| {
            if let Some(counts) = change
                .diff(&mut resource_cache)
                .ok()
                .and_then(|mut platform| platform.line_counts().ok())
                .flatten()
            {
                files_changed += 1;
                lines_added += u64::from(counts.insertions);
                lines_removed += u64::from(counts.removals);
            }

            resource_cache.clear_resource_cache_keep_allocation();
            Ok::<_, std::convert::Infallible>(std::ops::ControlFlow::Continue(()))
        })?;

        Ok(Stats {
            files_changed,
            lines_added,
            lines_removed,
        })
    }
}

///
pub mod for_each;
