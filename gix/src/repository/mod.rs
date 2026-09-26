//!
#![allow(clippy::empty_docs)]

/// The kind of Git repository, focussing on the repository data itself, i.e. what's in `.git`.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Kind {
    /// An ordinary Git repository.
    Common,
    /// A submodule worktree, whose `git` repository lives in `.git/modules/**/<name>` of the parent repository.
    ///
    /// Note that 'old-form' submodules (with a nested `.git` directory) are represented as [`Kind::Common`].
    Submodule,
    /// A worktree, whose `git` repository lives in `.git/worktrees/**/<name>` of the parent repository.
    LinkedWorkTree,
}

/// A supported repository format version, as selected by `core.repositoryFormatVersion`.
///
/// See [Git's repository format specification](https://git-scm.com/docs/repository-version).
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum FormatVersion {
    /// The original SHA-1 repository format, used when no version is configured.
    ///
    /// Objects use SHA-1 identifiers, and references use loose files and `packed-refs`.
    /// Git honors these historical `extensions.*` keys even in version 0:
    ///
    /// - `noop`: changes no behavior; used for compatibility testing.
    /// - `preciousObjects`: forbids deleting objects, including during pruning and repacking.
    /// - `partialClone`: names the promisor remote that can supply intentionally omitted objects.
    /// - `worktreeConfig`: enables per-worktree `config.worktree` files in addition to the shared configuration.
    ///
    /// Unknown extension keys are ignored, but known version-1-only extensions such as `objectFormat` and
    /// `relativeWorktrees` are rejected. Enabling those requires [`V1`][Self::V1].
    #[default]
    V0,
    /// Version 0's base format with explicit extension negotiation.
    ///
    /// The version number alone does not change storage or enable features. Git requires readers to understand
    /// every configured `extensions.*` key and its value before operating on a version-1 repository.
    /// Examples of version-1-only extensions include:
    ///
    /// - `objectFormat`: selects SHA-1 or SHA-256 objects; without it, SHA-1 remains the default.
    ///   `gix` needs the corresponding `sha1` or `sha256` Cargo feature.
    /// - `relativeWorktrees`: records that worktrees may use relative links to their administrative Git directories.
    ///   Creating relative links is controlled separately by `worktree.useRelativePaths`.
    /// - `refStorage`: selects Git's reference storage backend, including reftable.
    ///
    /// The historical version-0 extensions remain valid. Recognizing version 1 does not imply that `gix` supports
    /// every extension. With no extensions configured, prefer [`V0`][Self::V0] for compatibility with older readers.
    V1,
}

impl FormatVersion {
    /// Validate that upgrading this repository format to version 1 will not activate unsupported extensions.
    ///
    /// For version 0, permit only Git's grandfathered `noop`, `preciousObjects`, `partialClone`, and `worktreeConfig`
    /// extensions, without subsections. Unknown extensions that version 0 ignores become significant in version 1.
    /// Version 1 needs no upgrade and is accepted without checking its extensions.
    ///
    /// This only checks extension names; it neither validates values nor modifies `config`.
    pub fn validate_upgrade_to_v1(self, config: &gix_config::File) -> crate::Result<()> {
        if self == Self::V1 {
            return Ok(());
        }
        for section in config.sections_by_name("extensions").into_iter().flatten() {
            for name in section.value_names() {
                if section.header().subsection_name().is_some()
                    || !["noop", "preciousobjects", "partialclone", "worktreeconfig"]
                        .iter()
                        .any(|known| name.eq_ignore_ascii_case(known))
                {
                    let mut error =
                        gix_error::validation("Cannot upgrade repository format with unsupported extension")
                            .with("extension", name);
                    if let Some(subsection) = section.header().subsection_name() {
                        error = error.with("subsection", subsection);
                    }
                    return Err(crate::Error::from_error(error));
                }
            }
        }
        Ok(())
    }
}

#[cfg(any(feature = "attributes", feature = "excludes"))]
pub mod attributes;
///
#[cfg(feature = "blame")]
mod blame;
/// Local branch operations.
pub mod branch;
mod cache;
#[cfg(feature = "worktree-mutation")]
mod checkout;
mod config;

///
#[cfg(feature = "blob-diff")]
mod diff;
///
#[cfg(feature = "dirwalk")]
mod dirwalk;
///
#[cfg(feature = "attributes")]
pub mod filter;
///
pub mod freelist;
mod graph;
pub(crate) mod identity;
mod impls;
#[cfg(feature = "index")]
mod index;
pub(crate) mod init;
mod location;
#[cfg(feature = "mailmap")]
mod mailmap;
///
#[cfg(feature = "merge")]
mod merge;
#[cfg(feature = "notes")]
mod note;
mod object;
#[cfg(feature = "attributes")]
mod pathspec;
mod reference;
mod remote;
mod revision;
mod shallow;
mod state;
#[cfg(feature = "attributes")]
mod submodule;
mod thread_safe;
pub(crate) mod worktree;

///
#[cfg(feature = "blame")]
pub mod blame_file {
    /// Options to be passed to [Repository::blame_file()](crate::Repository::blame_file()).
    #[derive(Default, Debug, Clone)]
    pub struct Options {
        /// The algorithm to use for diffing. If `None`, `diff.algorithm` will be used.
        pub diff_algorithm: Option<gix_diff::blob::Algorithm>,
        /// The ranges to blame in the file.
        pub ranges: gix_blame::BlameRanges,
        /// Don't consider commits before the given date.
        pub since: Option<gix_date::Time>,
        /// Determine if rename tracking should be performed, and how.
        pub rewrites: Option<gix_diff::Rewrites>,
    }
}
