//! Algorithms for diffing various git object types and for generating patches, highly optimized for performance.
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

/// Re-export for use in public API.
#[cfg(feature = "blob")]
pub use gix_command as command;
/// Re-export for use in public API.
#[cfg(feature = "blob")]
pub use gix_object as object;

/// A structure to capture how to perform rename and copy tracking, used by the [rewrites::Tracker].
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg(feature = "blob")]
pub struct Rewrites {
    /// If `Some(…)`, also find copies. `None` is the default which does not try to detect copies at all.
    ///
    /// Note that this is an even more expensive operation than detecting renames stemming from additions and deletions
    /// as the resulting set to search through is usually larger.
    pub copies: Option<rewrites::Copies>,
    /// The percentage of similarity needed for files to be considered renamed, defaulting to `Some(0.5)`.
    /// This field is similar to `git diff -M50%`.
    ///
    /// If `None`, files are only considered equal if their content matches 100%.
    /// Note that values greater than 1.0 have no different effect than 1.0.
    /// Similarity follows Git's byte-span scoring, quantized to 60000 steps, and is independent of the line-diff algorithm.
    pub percentage: Option<f32>,
    /// The amount of files to consider for fuzzy rename or copy tracking. Defaults to 1000, meaning that only 1000*1000
    /// combinations can be tested for fuzzy matches, i.e. the ones that try to find matches by comparing similarity.
    /// If 0, there is no limit.
    ///
    /// Exact matches and the stricter unique-basename pass run before this limit is applied.
    /// If the remaining combinations exceed the limit, exhaustive similarity matching is skipped.
    /// Copy detection over all sources may fall back to modified and deleted sources to stay within the limit.
    pub limit: usize,

    /// If `true`, empty blobs will be tracked. If `false`, they do not participate in rename tracking.
    ///
    /// Leaving this off usually leads to better results as empty files don't have a unique-enough identity.
    pub track_empty: bool,
}

/// Contains a [Tracker](rewrites::Tracker) to detect rewrites.
#[cfg(feature = "blob")]
pub mod rewrites;

///
pub mod tree;
pub use tree::function::diff as tree;

///
#[cfg(feature = "blob")]
pub mod tree_with_rewrites;
#[cfg(feature = "blob")]
pub use tree_with_rewrites::function::diff as tree_with_rewrites;

///
#[cfg(feature = "index")]
pub mod index;
#[cfg(feature = "index")]
pub use index::function::diff as index;

///
#[cfg(feature = "blob")]
pub mod blob;
