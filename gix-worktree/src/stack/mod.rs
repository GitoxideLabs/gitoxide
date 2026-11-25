#![allow(missing_docs)]
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use bstr::{BStr, ByteSlice};

use super::Stack;
use crate::PathIdMapping;

/// Various aggregate numbers collected from when the corresponding [`Stack`] was instantiated.
#[derive(Default, Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Statistics {
    /// The amount of platforms created to do further matching.
    pub platforms: usize,
    /// Information about the stack delegate.
    pub delegate: delegate::Statistics,
    /// Information about attributes
    #[cfg(feature = "attributes")]
    pub attributes: state::attributes::Statistics,
    /// Information about the ignore stack
    pub ignore: state::ignore::Statistics,
}

#[derive(Clone)]
pub enum State {
    /// Useful for checkout where directories need creation, but we need to access attributes as well.
    #[cfg(feature = "attributes")]
    CreateDirectoryAndAttributesStack {
        /// If there is a symlink or a file in our path, try to unlink it before creating the directory.
        unlink_on_collision: bool,
        /// Options to control how newly created path components should be validated.
        validate: gix_validate::path::component::Options,
        /// State to handle attribute information
        attributes: state::Attributes,
    },
    /// Used when adding files, requiring access to both attributes and ignore information, for example during add operations.
    #[cfg(feature = "attributes")]
    AttributesAndIgnoreStack {
        /// State to handle attribute information
        attributes: state::Attributes,
        /// State to handle exclusion information
        ignore: state::Ignore,
    },
    /// Used when only attributes are required, typically with fully virtual worktrees.
    #[cfg(feature = "attributes")]
    AttributesStack(state::Attributes),
    /// Used when providing worktree status information.
    IgnoreStack(state::Ignore),
}

#[must_use]
pub struct Platform<'a> {
    parent: &'a Stack,
    relative_path: Cow<'a, BStr>,
    is_dir: Option<bool>,
}

/// Initialization
impl Stack {
    /// Create a new instance with `worktree_root` being the base for all future paths we match.
    /// `state` defines the capabilities of the cache.
    /// The `case` configures attribute and exclusion case sensitivity at *query time*, which should match the case that
    /// `state` might be configured with.
    /// `buf` is used when reading files, and `id_mappings` should have been created with [`State::id_mappings_from_index()`].
    pub fn new(
        worktree_root: impl Into<PathBuf>,
        state: State,
        case: gix_glob::pattern::Case,
        buf: Vec<u8>,
        id_mappings: Vec<PathIdMapping>,
    ) -> Self {
        let root = worktree_root.into();
        Stack {
            stack: gix_fs::Stack::new(root),
            state,
            reject_terminal_symlinks: false,
            case,
            buf,
            id_mappings,
            statistics: Statistics::default(),
        }
    }

    /// Enable Windows-only no-follow validation of terminal non-directory entries for checkout stacks.
    ///
    /// Call this before the first [`at_path()`](Self::at_path()) or [`at_entry()`](Self::at_entry()) invocation.
    pub fn enable_terminal_symlink_check(&mut self) {
        self.reject_terminal_symlinks = true;
    }

    /// Create a new stack that takes into consideration the `ignore_case` result of a filesystem probe in `root`. It takes a configured
    /// `state` to control what it can do, while initializing attribute or ignore files that are to be queried from the ODB using
    /// `index` and `path_backing`.
    ///
    /// This is the easiest way to correctly setup a stack.
    pub fn from_state_and_ignore_case(
        root: impl Into<PathBuf>,
        ignore_case: bool,
        state: State,
        index: &gix_index::State,
        path_backing: &gix_index::PathStorageRef,
    ) -> Self {
        let case = if ignore_case {
            gix_glob::pattern::Case::Fold
        } else {
            gix_glob::pattern::Case::Sensitive
        };
        let attribute_files = state.id_mappings_from_index(index, path_backing, case);
        Stack::new(root, state, case, Vec::with_capacity(512), attribute_files)
    }
}

/// Entry points for attribute query
impl Stack {
    /// Append the `relative` path to the root directory of the cache and efficiently create leading directories, while assuring that no
    /// symlinks are in that path.
    /// Unless `mode` is known with `Some(gix_index::entry::Mode::DIR|COMMIT)`,
    /// then `relative` points to a directory itself in which case the entire resulting path is created as directory.
    /// If it's not known it is assumed to be a file.
    /// `objects` maybe used to lookup objects from an [id mapping][crate::stack::State::id_mappings_from_index()], with mappnigs
    ///
    /// Provide access to cached information for that `relative` path via the returned platform.
    pub fn at_path(
        &mut self,
        relative: impl ToNormalPathComponents,
        mode: Option<gix_index::entry::Mode>,
        objects: &dyn gix_object::Find,
    ) -> std::io::Result<Platform<'_>> {
        self.statistics.platforms += 1;
        let mut delegate = StackDelegate {
            state: &mut self.state,
            buf: &mut self.buf,
            mode,
            id_mappings: &self.id_mappings,
            objects,
            case: self.case,
            reject_temrinal_symlinks: self.reject_terminal_symlinks,
            statistics: &mut self.statistics,
        };
        self.stack.make_relative_path_current(relative, &mut delegate)?;
        let relative_path = gix_path::to_unix_separators_on_windows(
            gix_path::into_bstr(self.stack.current_relative()).map_err(std::io::Error::other)?,
        );
        Ok(Platform {
            parent: self,
            relative_path,
            is_dir: mode_is_dir(mode),
        })
    }

    /// Obtain a platform for lookups from a repo-`relative` path, typically obtained from an index entry. `mode` should reflect
    /// the kind of item set here, or left at `None` if unknown.
    /// `objects` maybe used to lookup objects from an [id mapping][crate::stack::State::id_mappings_from_index()].
    /// All effects are similar to [`at_path()`][Self::at_path()].
    ///
    /// If `relative` ends with `/` and `mode` is `None`, it is automatically assumed set to be a directory.
    pub fn at_entry<'r>(
        &mut self,
        relative: impl Into<&'r BStr>,
        mode: Option<gix_index::entry::Mode>,
        objects: &dyn gix_object::Find,
    ) -> std::io::Result<Platform<'_>> {
        let relative = relative.into();
        self.at_path(
            relative,
            mode.or_else(|| relative.ends_with_str("/").then_some(gix_index::entry::Mode::DIR)),
            objects,
        )
    }
}

fn mode_is_dir(mode: Option<gix_index::entry::Mode>) -> Option<bool> {
    mode.map(|m|
        // This applies to directories and commits (submodules are directories on disk)
        m.is_sparse() || m.is_submodule())
}

/// Mutation
impl Stack {
    /// Reset the statistics after returning them.
    pub fn take_statistics(&mut self) -> Statistics {
        std::mem::take(&mut self.statistics)
    }

    /// Return our state for applying changes.
    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    /// Change the `case` of the next match to the given one.
    pub fn set_case(&mut self, case: gix_glob::pattern::Case) -> &mut Self {
        self.case = case;
        self
    }
}

/// Access
impl Stack {
    /// Return the statistics we gathered thus far.
    pub fn statistics(&self) -> &Statistics {
        &self.statistics
    }
    /// Return the state for introspection.
    pub fn state(&self) -> &State {
        &self.state
    }

    /// Paths whose rules can be read from the object database and their indexed blob IDs.
    pub fn id_mappings(&self) -> &[(bstr::BString, gix_hash::ObjectId)] {
        &self.id_mappings
    }

    /// Return the base path against which all entries or paths should be relative to when querying.
    ///
    /// Note that this path _may_ not be canonicalized.
    pub fn base(&self) -> &Path {
        self.stack.root()
    }
}

///
pub mod delegate;
use delegate::StackDelegate;
use gix_fs::stack::ToNormalPathComponents;

mod platform;
///
pub mod state;
