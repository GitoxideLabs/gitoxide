//! The implementation of creating an archive from a worktree stream, similar to `git archive`.
//!
//! ## Deviation
//!
//! This implementation is early and just does the basics. Git does more to support more context when filtering and to keep
//! more information about entries in the various archive formats.
//! `tar` is implemented in a very basic fashion only.
//!
//! ## Feature Flags
//! All features are related to which container formats are available.
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

use bstr::BString;

/// The error returned by [`write_stream()`].
pub type Error = gix_error::Exn<gix_error::Message>;

/// The supported container formats for use in [`write_stream()`].
#[derive(Default, PartialEq, Eq, Copy, Clone, Debug)]
pub enum Format {
    /// An internal format that is suitable only for intra-process communication.
    ///
    /// All transformations in the options are ignored. Calling [`write_stream`] is disallowed
    /// as it's more efficient to call [gix_worktree_stream::Stream::into_read()] right away.
    /// It is provided here as a basis available without extra dependencies, and as a debugging tool.
    #[default]
    InternalTransientNonPersistable,
    /// A standard `tar` archive.
    ///
    /// Use it as well if a custom container format is desired. The idea is to decode it on a separate thread
    /// to rewrite the data to the desired format.
    Tar,
    /// A convenience format that will `gzip` deflate the `tar` stream.
    TarGz {
        /// If `None`, use the default compression level. Otherwise use the given one which
        /// ranges from 0-9 for the deflate algorithm.
        compression_level: Option<u8>,
    },
    /// A standard `zip` archive. Note that this format silently converts illformed UTF-8 to UTF-8, which will
    /// equal a change of path.
    ///
    /// Requires the `zip` feature toggle to have an effect.
    ///
    /// ### Shortcoming
    ///
    /// Even though symlinks are stored as such, for some reason at least on MacOS those aren't restored. That works,
    /// however, when letting `git` create the archive.
    Zip {
        /// If `None`, use the default compression level. Otherwise use the given one which
        /// ranges from 0-9 for the deflate algorithm.
        compression_level: Option<u8>,
    },
}

/// Options for configuring [`write_stream()`].
#[derive(Clone, Debug)]
pub struct Options {
    /// The archive's format.
    pub format: Format,
    /// Given a `path`, originating in the git tree, to place into the archive, put `<prefix>/path` in front of it.
    ///
    /// Note that `/` should be used as separator, and that a prefix directory has to end with `/`.
    pub tree_prefix: Option<BString>,
    /// The modification time for all entries in the archive as seen since UNIX epoch.
    ///
    /// Defaults to the current time. The caller may set this to the commit time if available.
    pub modification_time: gix_date::SecondsSinceUnixEpoch,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            format: Default::default(),
            tree_prefix: None,
            modification_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|t| t.as_secs() as i64)
                .unwrap_or_default(),
        }
    }
}

mod write;
pub use write::{write_stream, write_stream_seek};
