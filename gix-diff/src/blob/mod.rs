//! For using text diffs, please have a look at the [`imara-diff` documentation](https://docs.rs/imara-diff),
//! maintained by [Pascal Kuthe](https://github.com/pascalkuthe).
use std::{collections::HashMap, path::PathBuf};

use bstr::BString;
pub use imara_diff::*;

///
pub mod pipeline;

///
pub mod platform;

pub mod unified_diff;
pub use unified_diff::_impl::UnifiedDiff;

pub mod git_diff;
pub use git_diff::types::GitDiff;

/// Information about the diff performed to detect similarity.
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
pub struct DiffLineStats {
    /// The amount of lines to remove from the source to get to the destination.
    pub removals: u32,
    /// The amount of lines to add to the source to get to the destination.
    pub insertions: u32,
    /// The amount of lines of the previous state, in the source.
    pub before: u32,
    /// The amount of lines of the new state, in the destination.
    pub after: u32,
    /// A range from 0 to 1.0, where 1.0 is a perfect match and 0.5 is a similarity of 50%.
    /// Similarity is the ratio between all lines in the previous blob and the current blob,
    /// calculated as `(old_lines_count - new_lines_count) as f32 / old_lines_count.max(new_lines_count) as f32`.
    pub similarity: f32,
}

/// A way to classify a resource suitable for diffing.
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum ResourceKind {
    /// The source of a rewrite, rename or copy operation, or generally the old version of a resource.
    OldOrSource,
    /// The destination of a rewrite, rename or copy operation, or generally the new version of a resource.
    NewOrDestination,
}

/// A set of values to define how to diff something that is associated with it using `git-attributes`, relevant for regular files.
///
/// Some values are related to diffing, some are related to conversions.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Driver {
    /// The name of the driver, as referred to by `[diff "name"]` in the git configuration.
    pub name: BString,
    /// The command to execute to perform the diff entirely like `<command> old-file old-hex old-mode new-file new-hex new-mode`.
    ///
    /// Please note that we don't make this call ourselves, but use it to determine that we should not run the our standard
    /// built-in algorithm but bail instead as the output of such a program isn't standardized.
    pub command: Option<BString>,
    /// The per-driver algorithm to use.
    pub algorithm: Option<Algorithm>,
    /// The external filter program to call like `<binary_to_text_command> /path/to/blob` which outputs a textual version of the provided
    /// binary file.
    /// Note that it's invoked with a shell if arguments are given.
    /// Further, if present, it will always be executed, whether `is_binary` is set or not.
    pub binary_to_text_command: Option<BString>,
    /// `Some(true)` if this driver deals with binary files, which means that a `binary_to_text_command` should be used to convert binary
    /// into a textual representation.
    /// Without such a command, anything that is considered binary is not diffed, but only the size of its data is made available.
    /// If `Some(false)`, it won't be considered binary, and the its data will not be sampled for the null-byte either.
    /// Leaving it to `None` means binary detection is automatic, and is based on the presence of the `0` byte in the first 8kB of the buffer.
    pub is_binary: Option<bool>,
}

/// A conversion pipeline to take an object or path from what's stored in `git` to what can be diffed, while
/// following the guidance of git-attributes at the respective path to learn if diffing should happen or if
/// the content is considered binary.
///
/// There are two different conversion flows, where the target of the flow is a buffer with diffable content:
// TODO: update this with information about possible directions.
///
/// * `worktree on disk` -> `text conversion`
/// * `object` -> `worktree-filters` -> `text conversion`
#[derive(Clone)]
pub struct Pipeline {
    /// A way to read data directly from the worktree.
    pub roots: pipeline::WorktreeRoots,
    /// A pipeline to convert objects from what's stored in `git` to its worktree version.
    pub worktree_filter: gix_filter::Pipeline,
    /// Options affecting the way we read files.
    pub options: pipeline::Options,
    /// Drivers to help customize the conversion behaviour depending on the location of items.
    drivers: Vec<Driver>,
    /// Pre-configured attributes to obtain additional diff-related information.
    attrs: gix_filter::attributes::search::Outcome,
    /// A buffer to manipulate paths
    path: PathBuf,
}

/// A utility for performing a diff of two blobs, including flexible conversions, conversion-caching
/// acquisition of diff information.
/// Note that this instance will not call external filters as their output can't be known programmatically,
/// but it allows to prepare their input if the caller wishes to perform this task.
///
/// Optimized for NxM lookups with built-in caching.
#[derive(Clone)]
pub struct Platform {
    /// The old version of a diff-able blob, if set.
    old: Option<platform::CacheKey>,
    /// The new version of a diff-able blob, if set.
    new: Option<platform::CacheKey>,

    /// Options to alter how diffs should be performed.
    pub options: platform::Options,
    /// A way to convert objects into a diff-able format.
    pub filter: Pipeline,
    /// A way to access .gitattributes
    pub attr_stack: gix_worktree::Stack,

    /// The way we convert resources into diffable states.
    pub filter_mode: pipeline::Mode,
    /// A continuously growing cache keeping ready-for-diff blobs by their path in the worktree,
    /// as that is what affects their final diff-able state.
    ///
    /// That way, expensive rewrite-checks with NxM matrix checks would be as fast as possible,
    /// avoiding duplicate work.
    diff_cache: HashMap<platform::CacheKey, platform::CacheValue>,
    /// A list of previously used buffers, ready for re-use.
    free_list: Vec<Vec<u8>>,
}

mod impls {
    use crate::blob::ResourceKind;

    impl std::fmt::Display for ResourceKind {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(match self {
                ResourceKind::OldOrSource => "old",
                ResourceKind::NewOrDestination => "new",
            })
        }
    }
}
