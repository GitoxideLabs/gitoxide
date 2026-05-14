//! Parse [path specifications](https://git-scm.com/docs/gitglossary#Documentation/gitglossary.txt-aiddefpathspecapathspec) and
//! see if a path matches.
//!
//! ## Examples
//!
//! ```
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use std::path::Path;
//!
//! fn no_attrs(
//!     _path: &bstr::BStr,
//!     _case: gix_pathspec::attributes::glob::pattern::Case,
//!     _is_dir: bool,
//!     _out: &mut gix_pathspec::attributes::search::Outcome,
//! ) -> bool {
//!     false
//! }
//!
//! let specs = ["src/**", ":!src/generated/**"]
//!     .into_iter()
//!     .map(|spec| gix_pathspec::parse(spec.as_bytes(), Default::default()).unwrap());
//! let mut search = gix_pathspec::Search::from_specs(specs, None, Path::new(""))?;
//!
//! assert!(search.can_match_relative_path("src".into(), Some(true)));
//!
//! let matched = search
//!     .pattern_matching_relative_path("src/lib.rs".into(), Some(false), &mut no_attrs)
//!     .unwrap();
//! assert_eq!(matched.pattern.path(), "src/**");
//! assert!(!matched.pattern.is_excluded());
//!
//! let excluded = search
//!     .pattern_matching_relative_path("src/generated/lib.rs".into(), Some(false), &mut no_attrs)
//!     .unwrap();
//! assert_eq!(excluded.pattern.to_bstring(), ":(exclude)src/generated/**");
//! assert!(excluded.pattern.is_excluded());
//! # Ok(()) }
//! ```
#![deny(missing_docs)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

use bitflags::bitflags;
use bstr::BString;
/// `gix-glob` types are available through [`attributes::glob`].
pub use gix_attributes as attributes;

///
pub mod normalize {
    use std::path::PathBuf;

    /// The error returned by [Pattern::normalize()](super::Pattern::normalize()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("The path '{}' is not inside of the worktree '{}'", path.display(), worktree_path.display())]
        AbsolutePathOutsideOfWorktree { path: PathBuf, worktree_path: PathBuf },
        #[error("The path '{}' leaves the repository", path.display())]
        OutsideOfWorktree { path: PathBuf },
    }
}

mod pattern;

///
pub mod search;

///
pub mod parse;

/// Default settings for some fields of a [`Pattern`].
///
/// These can be used to represent `GIT_*_PATHSPECS` environment variables, for example.
#[derive(Debug, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Defaults {
    /// The default signature.
    pub signature: MagicSignature,
    /// The default search-mode.
    ///
    /// Note that even if it's [`SearchMode::Literal`], the pathspecs will be parsed as usual, but matched verbatim afterwards.
    ///
    /// Note that pathspecs can override this the [`SearchMode::Literal`] variant with an explicit `:(glob)` prefix.
    pub search_mode: SearchMode,
    /// If set, the pathspec will not be parsed but used verbatim. Implies [`SearchMode::Literal`] for `search_mode`.
    pub literal: bool,
}

///
pub mod defaults;

/// A lists of pathspec patterns, possibly from a file.
///
/// Pathspecs are generally relative to the root of the repository.
#[derive(Debug, Clone)]
pub struct Search {
    /// Patterns and their associated data in the order they were loaded in or specified,
    /// the line number in its source file or its sequence number (_`(pattern, value, line_number)`_).
    ///
    /// During matching, this order is reversed.
    patterns: Vec<gix_glob::search::pattern::Mapping<search::Spec>>,

    /// The path from which the patterns were read, or `None` if the patterns
    /// don't originate in a file on disk.
    pub source: Option<PathBuf>,

    /// If `true`, this means all `patterns` are exclude patterns. This means that if there is no match
    /// (which would exclude an item), we would actually match it for lack of exclusion.
    all_patterns_are_excluded: bool,
    /// The amount of bytes that are in common among all `patterns` and that aren't matched case-insensitively
    common_prefix_len: usize,
}

/// The output of a pathspec [parsing][parse()] operation. It can be used to match against a one or more paths.
#[derive(Default, PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct Pattern {
    /// The path part of a pathspec, which is typically a path possibly mixed with glob patterns.
    /// Note that it might be an empty string as well.
    ///
    /// For example, `:(top,literal,icase,attr,exclude)some/path` would yield `some/path`.
    path: BString,
    /// All magic signatures that were included in the pathspec.
    pub signature: MagicSignature,
    /// The search mode of the pathspec.
    pub search_mode: SearchMode,
    /// All attributes that were included in the `ATTR` part of the pathspec, if present.
    ///
    /// `:(attr:a=one b=):path` would yield attribute `a` and `b`.
    pub attributes: Vec<gix_attributes::Assignment>,
    /// If `true`, we are a special Nil pattern and always match.
    nil: bool,
    /// The length of bytes in `path` that belong to the prefix, which will always be matched case-sensitively
    /// on case-sensitive filesystems.
    ///
    /// That way, even though pathspecs are applied from the top, we can emulate having changed directory into
    /// a specific sub-directory in a case-sensitive file-system, even if the rest of the pathspec can be set to
    /// match case-insensitively.
    /// Is set by [Pattern::normalize()].
    prefix_len: usize,
}

bitflags! {
    /// Flags to represent 'magic signatures' which are parsed behind colons, like `:top:`.
    #[derive(Default, PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
    pub struct MagicSignature: u32 {
        /// Matches patterns from the root of the repository
        const TOP = 1 << 0;
        /// Matches patterns in case insensitive mode
        const ICASE = 1 << 1;
        /// Excludes the matching patterns from the previous results
        const EXCLUDE = 1 << 2;
        /// The pattern must match a directory, and not a file.
        /// This is equivalent to how it's handled in `gix-glob`
        const MUST_BE_DIR = 1 << 3;
    }
}

/// Parts of [magic signatures][MagicSignature] which don't stack as they all configure
/// the way path specs are matched.
#[derive(Default, PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
pub enum SearchMode {
    /// Expand special characters like `*` similar to how the shell would do it.
    ///
    /// See [`PathAwareGlob`](SearchMode::PathAwareGlob) for the alternative.
    #[default]
    ShellGlob,
    /// Special characters in the pattern, like `*` or `?`, are treated literally, effectively turning off globbing.
    Literal,
    /// A single `*` will not match a `/` in the pattern, but a `**` will
    PathAwareGlob,
}

/// Parse a git-style pathspec into a [`Pattern`],
/// setting the given `default` values in case these aren't specified in `input`.
///
/// Note that empty [paths](Pattern::path) are allowed here, and generally some processing has to be performed.
pub fn parse(input: &[u8], default: Defaults) -> Result<Pattern, parse::Error> {
    Pattern::from_bytes(input, default)
}
