//!
#![allow(clippy::empty_docs)]

/// The kind of repository.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Kind {
    /// A submodule worktree, whose `git` repository lives in `.git/modules/**/<name>` of the parent repository.
    ///
    /// Note that 'old-form' submodule will register as `Worktree {is_linked: false}`.
    Submodule,
    /// A bare repository does not have a work tree, that is files on disk beyond the `git` repository itself.
    Bare,
    /// A `git` repository along with a checked out files in a work tree.
    WorkTree {
        /// If true, this is the git dir associated with this _linked_ worktree, otherwise it is a repository with _main_ worktree.
        is_linked: bool,
    },
}

#[cfg(any(feature = "attributes", feature = "excludes"))]
pub mod attributes;
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
mod kind;
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
#[cfg(feature = "blame")]
pub mod blame_file {
    /// The error returned by [Repository::blame_file()](crate::Repository::blame_file()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        CommitGraphIfEnabled(#[from] super::commit_graph_if_enabled::Error),
        #[error(transparent)]
        DiffResourceCache(#[from] super::diff_resource_cache::Error),
        #[error(transparent)]
        Blame(#[from] gix_blame::Error),
    }
}

///
#[cfg(feature = "blob-diff")]
pub mod diff_tree_to_tree {
    /// The error returned by [Repository::diff_tree_to_tree()](crate::Repository::diff_tree_to_tree()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        DiffOptions(#[from] crate::diff::options::init::Error),
        #[error(transparent)]
        CreateResourceCache(#[from] super::diff_resource_cache::Error),
        #[error(transparent)]
        TreeDiff(#[from] gix_diff::tree_with_rewrites::Error),
    }
}

///
#[cfg(feature = "merge")]
pub mod blob_merge_options {
    /// The error returned by [Repository::blob_merge_options()](crate::Repository::blob_merge_options()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        DiffAlgorithm(#[from] crate::config::diff::algorithm::Error),
        #[error(transparent)]
        ConflictStyle(#[from] crate::config::key::GenericErrorWithValue),
    }
}

///
#[cfg(feature = "merge")]
pub mod merge_resource_cache {
    /// The error returned by [Repository::merge_resource_cache()](crate::Repository::merge_resource_cache()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        RenormalizeConfig(#[from] crate::config::boolean::Error),
        #[error(transparent)]
        PipelineOptions(#[from] crate::config::merge::pipeline_options::Error),
        #[error(transparent)]
        Index(#[from] crate::repository::index_or_load_from_head_or_empty::Error),
        #[error(transparent)]
        AttributeStack(#[from] crate::config::attribute_stack::Error),
        #[error(transparent)]
        CommandContext(#[from] crate::config::command_context::Error),
        #[error(transparent)]
        FilterPipeline(#[from] crate::filter::pipeline::options::Error),
        #[error(transparent)]
        DriversConfig(#[from] crate::config::merge::drivers::Error),
    }
}

///
#[cfg(feature = "merge")]
pub mod merge_trees {
    /// The error returned by [Repository::merge_trees()](crate::Repository::merge_trees()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        MergeResourceCache(#[from] super::merge_resource_cache::Error),
        #[error(transparent)]
        DiffResourceCache(#[from] super::diff_resource_cache::Error),
        #[error(transparent)]
        TreeMerge(#[from] gix_merge::tree::Error),
        #[error(transparent)]
        ValidationOptions(#[from] crate::config::boolean::Error),
    }
}

///
#[cfg(feature = "merge")]
pub mod merge_commits {
    /// The error returned by [Repository::merge_commits()](crate::Repository::merge_commits()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        OpenCommitGraph(#[from] super::commit_graph_if_enabled::Error),
        #[error(transparent)]
        MergeResourceCache(#[from] super::merge_resource_cache::Error),
        #[error(transparent)]
        DiffResourceCache(#[from] super::diff_resource_cache::Error),
        #[error(transparent)]
        CommitMerge(#[from] gix_merge::commit::Error),
        #[error(transparent)]
        ValidationOptions(#[from] crate::config::boolean::Error),
    }
}

///
#[cfg(feature = "merge")]
pub mod virtual_merge_base {
    /// The error returned by [Repository::virtual_merge_base()](crate::Repository::virtual_merge_base()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        OpenCommitGraph(#[from] super::commit_graph_if_enabled::Error),
        #[error(transparent)]
        VirtualMergeBase(#[from] super::virtual_merge_base_with_graph::Error),
    }
}

///
#[cfg(feature = "merge")]
pub mod virtual_merge_base_with_graph {
    /// The error returned by [Repository::virtual_merge_base_with_graph()](crate::Repository::virtual_merge_base_with_graph()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("No commit was provided as merge-base")]
        MissingCommit,
        #[error(transparent)]
        MergeResourceCache(#[from] super::merge_resource_cache::Error),
        #[error(transparent)]
        DiffResourceCache(#[from] super::diff_resource_cache::Error),
        #[error(transparent)]
        CommitMerge(#[from] gix_merge::commit::Error),
        #[error(transparent)]
        FindCommit(#[from] crate::object::find::existing::with_conversion::Error),
        #[error(transparent)]
        DecodeCommit(#[from] gix_object::decode::Error),
    }
}

///
#[cfg(feature = "revision")]
pub mod merge_base_octopus_with_graph {
    /// The error returned by [Repository::merge_base_octopus_with_graph()](crate::Repository::merge_base_octopus_with_graph()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("No commit was provided")]
        MissingCommit,
        #[error("No merge base was found between the given commits")]
        NoMergeBase,
        #[error(transparent)]
        MergeBase(#[from] gix_revision::merge_base::Error),
    }
}

///
#[cfg(feature = "revision")]
pub mod merge_base_octopus {
    /// The error returned by [Repository::merge_base_octopus()](crate::Repository::merge_base_octopus()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        OpenCache(#[from] crate::repository::commit_graph_if_enabled::Error),
        #[error(transparent)]
        MergeBaseOctopus(#[from] super::merge_base_octopus_with_graph::Error),
    }
}

///
#[cfg(feature = "revision")]
pub mod merge_bases_many {
    /// The error returned by [Repository::merge_bases_many()](crate::Repository::merge_bases_many()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        OpenCache(#[from] crate::repository::commit_graph_if_enabled::Error),
        #[error(transparent)]
        MergeBase(#[from] gix_revision::merge_base::Error),
    }
}

///
#[cfg(feature = "merge")]
pub mod tree_merge_options {
    /// The error returned by [Repository::tree_merge_options()](crate::Repository::tree_merge_options()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        BlobMergeOptions(#[from] super::blob_merge_options::Error),
        #[error(transparent)]
        RewritesConfig(#[from] crate::diff::new_rewrites::Error),
        #[error(transparent)]
        CommandContext(#[from] crate::config::command_context::Error),
    }
}

///
#[cfg(feature = "blob-diff")]
pub mod diff_resource_cache {
    /// The error returned by [Repository::diff_resource_cache()](crate::Repository::diff_resource_cache()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
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
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        FindTree(#[from] crate::object::find::existing::with_conversion::Error),
        #[error(transparent)]
        InitEditor(#[from] crate::object::tree::editor::init::Error),
    }
}

///
#[cfg(feature = "revision")]
pub mod merge_base {
    /// The error returned by [Repository::merge_base()](crate::Repository::merge_base()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        OpenCache(#[from] crate::repository::commit_graph_if_enabled::Error),
        #[error(transparent)]
        FindMergeBase(#[from] gix_revision::merge_base::Error),
        #[error("Could not find a merge-base between commits {first} and {second}")]
        NotFound {
            first: gix_hash::ObjectId,
            second: gix_hash::ObjectId,
        },
    }
}

///
#[cfg(feature = "revision")]
pub mod merge_base_with_graph {
    /// The error returned by [Repository::merge_base_with_cache()](crate::Repository::merge_base_with_graph()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        FindMergeBase(#[from] gix_revision::merge_base::Error),
        #[error("Could not find a merge-base between commits {first} and {second}")]
        NotFound {
            first: gix_hash::ObjectId,
            second: gix_hash::ObjectId,
        },
    }
}

///
pub mod commit_graph_if_enabled {
    /// The error returned by [Repository::commit_graph_if_enabled()](crate::Repository::commit_graph_if_enabled()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        ConfigBoolean(#[from] crate::config::boolean::Error),
        #[error(transparent)]
        OpenCommitGraph(#[from] gix_commitgraph::init::Error),
    }
}

///
#[cfg(feature = "index")]
pub mod index_from_tree {
    /// The error returned by [Repository::index_from_tree()](crate::Repository::index_from_tree).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
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
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("The configured name of the remote ref to merge wasn't valid")]
        ValidateFetchRemoteRefName(#[from] gix_validate::reference::name::Error),
        #[error(transparent)]
        PushDefault(#[from] crate::config::key::GenericErrorWithValue),
        #[error(transparent)]
        FindPushRemote(#[from] crate::remote::find::existing::Error),
    }
}

///
pub mod branch_remote_tracking_ref_name {
    /// The error returned by [Repository::branch_remote_tracking_ref_name()](crate::Repository::branch_remote_tracking_ref_name()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("The name of the tracking reference was invalid")]
        ValidateTrackingRef(#[from] gix_validate::reference::name::Error),
        #[error("Could not get the remote reference to translate into the local tracking branch")]
        RemoteRef(#[from] super::branch_remote_ref_name::Error),
        #[error("Couldn't find remote to obtain fetch-specs for mapping to the tracking reference")]
        FindRemote(#[from] crate::remote::find::existing::Error),
    }
}

///
pub mod upstream_branch_and_remote_name_for_tracking_branch {
    /// The error returned by [Repository::upstream_branch_and_remote_name_for_tracking_branch()](crate::Repository::upstream_branch_and_remote_for_tracking_branch()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("The input branch '{}' needs to be a remote tracking branch", full_name.as_bstr())]
        BranchCategory { full_name: gix_ref::FullName },
        #[error(transparent)]
        FindRemote(#[from] crate::remote::find::existing::Error),
        #[error("Found ambiguous remotes without 1:1 mapping or more than one match: {}", remotes.iter()
                                                                            .map(|r| r.as_bstr().to_string())
                                                                            .collect::<Vec<_>>().join(", "))]
        AmbiguousRemotes { remotes: Vec<crate::remote::Name<'static>> },
        #[error(transparent)]
        ValidateUpstreamBranch(#[from] gix_ref::name::Error),
    }
}

///
#[cfg(feature = "attributes")]
pub mod pathspec_defaults_ignore_case {
    /// The error returned by [Repository::pathspec_defaults_ignore_case()](crate::Repository::pathspec_defaults_inherit_ignore_case()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Filesystem configuration could not be obtained to learn about case sensitivity")]
        FilesystemConfig(#[from] crate::config::boolean::Error),
        #[error(transparent)]
        Defaults(#[from] gix_pathspec::defaults::from_environment::Error),
    }
}

///
#[cfg(feature = "index")]
pub mod index_or_load_from_head {
    /// The error returned by [`Repository::index_or_load_from_head()`](crate::Repository::index_or_load_from_head()).
    #[derive(thiserror::Error, Debug)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        HeadCommit(#[from] crate::reference::head_commit::Error),
        #[error(transparent)]
        TreeId(#[from] gix_object::decode::Error),
        #[error(transparent)]
        TraverseTree(#[from] crate::repository::index_from_tree::Error),
        #[error(transparent)]
        OpenIndex(#[from] crate::worktree::open_index::Error),
    }
}

///
#[cfg(feature = "index")]
pub mod index_or_load_from_head_or_empty {
    /// The error returned by [`Repository::index_or_load_from_head_or_empty()`](crate::Repository::index_or_load_from_head_or_empty()).
    #[derive(thiserror::Error, Debug)]
    #[allow(missing_docs)]
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
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        FindTree(#[from] crate::object::find::existing::Error),
        #[error(transparent)]
        OpenTree(#[from] crate::repository::index_from_tree::Error),
        #[error(transparent)]
        AttributesCache(#[from] crate::config::attribute_stack::Error),
        #[error(transparent)]
        FilterPipeline(#[from] crate::filter::pipeline::options::Error),
        #[error(transparent)]
        CommandContext(#[from] crate::config::command_context::Error),
        #[error("Needed {id} to be a tree to turn into a workspace stream, got {actual}")]
        NotATree {
            id: gix_hash::ObjectId,
            actual: gix_object::Kind,
        },
    }
}

///
#[cfg(feature = "worktree-archive")]
pub mod worktree_archive {
    /// The error returned by [`Repository::worktree_archive()`](crate::Repository::worktree_archive()).
    pub type Error = gix_archive::Error;
}
