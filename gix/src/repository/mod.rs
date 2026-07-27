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

#[cfg(any(feature = "attributes", feature = "excludes"))]
pub mod attributes;
///
#[cfg(feature = "blame")]
mod blame;
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
mod worktree;

///
mod new_commit {
    /// The error returned by [`new_commit(…)`](crate::Repository::new_commit()).
    pub type Error = gix_error::Error;
}

///
mod new_commit_as {
    /// The error returned by [`new_commit_as(…)`](crate::Repository::new_commit_as()).
    pub type Error = gix_error::Error;
}

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

    /// The error returned by [Repository::blame_file()](crate::Repository::blame_file()).
    pub type Error = gix_error::Error;
}

///
#[cfg(feature = "blob-diff")]
pub mod diff_tree_to_tree {
    /// The error returned by [Repository::diff_tree_to_tree()](crate::Repository::diff_tree_to_tree()).
    pub type Error = gix_error::Error;
}

///
#[cfg(feature = "merge")]
pub mod blob_merge_options {
    /// The error returned by [Repository::blob_merge_options()](crate::Repository::blob_merge_options()).
    pub type Error = gix_error::Error;
}

///
#[cfg(feature = "merge")]
pub mod merge_resource_cache {
    /// The error returned by [Repository::merge_resource_cache()](crate::Repository::merge_resource_cache()).
    pub type Error = gix_error::Error;
}

///
#[cfg(feature = "merge")]
pub mod merge_trees {
    /// The error returned by [Repository::merge_trees()](crate::Repository::merge_trees()).
    pub type Error = gix_error::Error;
}

///
#[cfg(feature = "merge")]
pub mod merge_commits {
    /// The error returned by [Repository::merge_commits()](crate::Repository::merge_commits()).
    pub type Error = gix_error::Error;
}

///
#[cfg(feature = "merge")]
pub mod virtual_merge_base {
    /// The error returned by [Repository::virtual_merge_base()](crate::Repository::virtual_merge_base()).
    pub type Error = gix_error::Error;
}

///
#[cfg(feature = "merge")]
pub mod virtual_merge_base_with_graph {
    /// The error returned by [Repository::virtual_merge_base_with_graph()](crate::Repository::virtual_merge_base_with_graph()).
    pub type Error = gix_error::Error;
}

///
#[cfg(feature = "revision")]
pub mod merge_base_octopus_with_graph {
    /// The error returned by [Repository::merge_base_octopus_with_graph()](crate::Repository::merge_base_octopus_with_graph()).
    pub type Error = gix_error::Error;
}

///
#[cfg(feature = "revision")]
pub mod merge_base_octopus {
    /// The error returned by [Repository::merge_base_octopus()](crate::Repository::merge_base_octopus()).
    pub type Error = gix_error::Error;
}

///
#[cfg(feature = "revision")]
pub mod merge_bases_many {
    /// The error returned by [Repository::merge_bases_many()](crate::Repository::merge_bases_many()).
    pub type Error = gix_error::Error;
}

///
#[cfg(feature = "merge")]
pub mod tree_merge_options {
    /// The error returned by [Repository::tree_merge_options()](crate::Repository::tree_merge_options()).
    pub type Error = gix_error::Error;
}

///
#[cfg(feature = "blob-diff")]
pub mod diff_resource_cache {
    /// The error returned by [Repository::diff_resource_cache()](crate::Repository::diff_resource_cache()).
    // TODO(review): kept concrete because `status::tree_index::Error` already embeds the erased
    //                `diff::new_rewrites::Error` (via `RewritesConfiguration`); erasing this one would give
    //                it a second `From<gix::Error>` via `DiffResourceCache`. `status::tree_index::Error` in
    //                turn has to stay concrete because `status::iter::Error` already embeds the erased
    //                `status::index_worktree::Error`, and erasing `tree_index::Error` would give it a second
    //                `From<gix::Error>` too - and `status::iter::Error` must stay concrete since callers match
    //                its variants. `index_or_load_from_head_or_empty` also has to stay concrete, because its
    //                erasure would give this enum's own `ResourceCache` variant (already erased) a colliding
    //                sibling in `Index`.
    #[derive(Debug, thiserror::Error)]
    #[expect(missing_docs)]
    pub enum Error {
        #[error("Could not obtain resource cache for diffing")]
        ResourceCache(#[from] crate::diff::resource_cache::Error),
        #[error(transparent)]
        Index(#[from] crate::repository::index_or_load_from_head_or_empty::Error),
        #[error(transparent)]
        AttributeStack(#[from] crate::config::attribute_stack::Error),
    }
}

///
#[cfg(feature = "tree-editor")]
pub mod edit_tree {
    /// The error returned by [Repository::edit_tree()](crate::Repository::edit_tree).
    pub type Error = gix_error::Error;
}

///
#[cfg(feature = "revision")]
pub mod merge_base {
    /// The error returned by [Repository::merge_base()](crate::Repository::merge_base()).
    pub type Error = gix_error::Error;
}

///
#[cfg(feature = "revision")]
pub mod merge_base_with_graph {
    /// The error returned by [Repository::merge_base_with_cache()](crate::Repository::merge_base_with_graph()).
    pub type Error = gix_error::Error;
}

///
pub mod commit_graph_if_enabled {
    /// The error returned by [Repository::commit_graph_if_enabled()](crate::Repository::commit_graph_if_enabled()).
    pub type Error = gix_error::Error;
}

///
#[cfg(feature = "index")]
pub mod index_from_tree {
    /// The error returned by [Repository::index_from_tree()](crate::Repository::index_from_tree).
    // TODO(review): kept concrete due to two separate E0119 collisions on external parents that embed
    //                this type via `#[from]`: `repository::index_or_load_from_head_or_empty::Error`
    //                (`gix/src/repository/mod.rs`, via `TraverseTree`) already embeds the erased
    //                `object::peel::to_kind::Error` via `PeelToTree`; `status::tree_index::Error`
    //                (`gix/src/status/tree_index.rs`, via `IndexFromMTree`) already embeds the erased
    //                `diff::new_rewrites::Error` via `RewritesConfiguration`. Erasing `index_from_tree::Error`
    //                would give either enum a second `From<gix_error::Error>` impl.
    #[derive(Debug, thiserror::Error)]
    #[expect(missing_docs)]
    pub enum Error {
        #[error("Could not create index from tree at {id}")]
        IndexFromTree {
            id: gix_hash::ObjectId,
            source: gix_index::init::from_tree::Error,
        },
        #[error("Couldn't obtain configuration for core.protect*")]
        BooleanConfig(#[from] crate::config::boolean::Error),
    }
}

///
pub mod branch_remote_ref_name {
    /// The error returned by [Repository::branch_remote_ref_name()](crate::Repository::branch_remote_ref_name()).
    pub type Error = gix_error::Error;
}

///
pub mod branch_remote_tracking_ref_name {
    /// The error returned by [Repository::branch_remote_tracking_ref_name()](crate::Repository::branch_remote_tracking_ref_name()).
    pub type Error = gix_error::Error;
}

///
pub mod upstream_branch_and_remote_name_for_tracking_branch {
    /// The error returned by [Repository::upstream_branch_and_remote_name_for_tracking_branch()](crate::Repository::upstream_branch_and_remote_for_tracking_branch()).
    pub type Error = gix_error::Error;
}

///
pub mod normalize_path {
    // TODO(review): kept concrete. Callers match its variants directly:
    //                `gix/tests/repository.rs:74` matches `Error::OutsideOfRepository { .. }`, and
    //                `:99` matches `Error::AbsolutePathOutsideOfRepository { .. }`. No `#[from]`
    //                parents embed this type.
    /// The error returned by [Repository::normalize_path()](crate::Repository::normalize_path()).
    #[derive(Debug, thiserror::Error)]
    #[expect(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        Realpath(#[from] gix_path::realpath::Error),
        #[error("The path '{}' leaves the repository", path.display())]
        OutsideOfRepository { path: std::path::PathBuf },
        #[error(
            "The absolute path '{}' is not inside the repository at '{}'",
            path.display(),
            root.display()
        )]
        AbsolutePathOutsideOfRepository {
            path: std::path::PathBuf,
            root: std::path::PathBuf,
        },
    }
}

///
#[cfg(feature = "attributes")]
pub mod pathspec_defaults_ignore_case {
    /// The error returned by [Repository::pathspec_defaults_ignore_case()](crate::Repository::pathspec_defaults_inherit_ignore_case()).
    pub type Error = gix_error::Error;
}

///
#[cfg(feature = "index")]
pub mod index_or_load_from_head {
    /// The error returned by [`Repository::index_or_load_from_head()`](crate::Repository::index_or_load_from_head()).
    pub type Error = gix_error::Error;
}

///
#[cfg(feature = "index")]
pub mod index_or_load_from_head_or_empty {
    /// The error returned by [`Repository::index_or_load_from_head_or_empty()`](crate::Repository::index_or_load_from_head_or_empty()).
    // TODO(review): kept concrete because `diff_resource_cache::Error` already embeds the erased
    //                `diff::resource_cache::Error` (via `ResourceCache`); erasing this one would give it a
    //                second `From<gix::Error>` via `Index`. `diff_resource_cache::Error` in turn has to stay
    //                concrete for the reasons noted on its own `TODO(review)`.
    #[derive(thiserror::Error, Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        ReadHead(#[from] crate::reference::find::existing::Error),
        #[error(transparent)]
        FindCommit(#[from] crate::object::find::existing::Error),
        #[error(transparent)]
        PeelToTree(#[from] crate::object::peel::to_kind::Error),
        #[error(transparent)]
        TreeId(#[from] gix_object::decode::Error),
        #[error(transparent)]
        TraverseTree(#[from] crate::repository::index_from_tree::Error),
        #[error(transparent)]
        OpenIndex(#[from] crate::worktree::open_index::Error),
    }
}

///
#[cfg(feature = "worktree-stream")]
pub mod worktree_stream {
    /// The error returned by [`Repository::worktree_stream()`](crate::Repository::worktree_stream()).
    pub type Error = gix_error::Error;
}

///
#[cfg(feature = "worktree-archive")]
pub mod worktree_archive {
    /// The error returned by [`Repository::worktree_archive()`](crate::Repository::worktree_archive()).
    pub type Error = gix_error::Error;
}
