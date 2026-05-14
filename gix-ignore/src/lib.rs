//! Parse `.gitignore` files and provide utilities to match against them.
//!
//! ## Examples
//!
//! ```
//! use gix_glob::pattern::Case;
//!
//! let search = gix_ignore::Search::from_overrides(["target/", "!target/keep.me"], Default::default());
//!
//! let ignored = search
//!     .pattern_matching_relative_path("target".into(), Some(true), Case::Sensitive)
//!     .unwrap();
//! assert_eq!(ignored.sequence_number, 1);
//! assert_eq!(ignored.kind, gix_ignore::Kind::Expendable);
//! assert!(!ignored.pattern.is_negative());
//!
//! let kept = search
//!     .pattern_matching_relative_path("target/keep.me".into(), Some(false), Case::Sensitive)
//!     .unwrap();
//! assert_eq!(kept.sequence_number, 2);
//! assert!(kept.pattern.is_negative());
//! ```
//!
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

pub use gix_glob as glob;

///
pub mod search;
/// A grouping of lists of patterns while possibly keeping associated to their base path in order to find matches.
///
/// Pattern lists with base path are queryable relative to that base, otherwise they are relative to the repository root.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Default)]
pub struct Search {
    /// A list of pattern lists, each representing a patterns from a file or specified by hand, in the order they were
    /// specified in.
    ///
    /// When matching, this order is reversed.
    pub patterns: Vec<gix_glob::search::pattern::List<search::Ignore>>,
}

/// The kind of *ignored* item.
///
/// This classification is obtained when checking if a path matches an ignore pattern.
#[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum Kind {
    /// The item is ignored and will be removed to make place for tracked items that are to be checked out.
    ///
    /// This is the default for ignored items.
    /// Another way of thinking about this class is to consider these files *trashable*, or talk about them as `ignored-and-expendable`.
    #[default]
    Expendable,
    /// An ignored file was additionally marked as *precious* using the `$` prefix to indicate the file shall be kept.
    ///
    /// This means that precious files are treated like untracked files, which also must not be removed, but won't show up by default
    /// as they are also ignored.
    /// One can also talk about them as `ignored-and-precious`.
    Precious,
}

///
pub mod parse;

/// Parse git ignore patterns, line by line, from `bytes`.
///
/// If `support_precious` is `true`, we will parse `$` prefixed entries as precious.
/// This is backward-incompatible as files that actually start with `$` like `$houdini`
/// will then not be ignored anymore, instead it ignores `houdini`.
pub fn parse(bytes: &[u8], support_precious: bool) -> parse::Lines<'_> {
    parse::Lines::new(bytes, support_precious)
}
