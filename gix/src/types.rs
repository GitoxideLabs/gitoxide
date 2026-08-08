use std::{cell::RefCell, path::PathBuf};

use gix_hash::ObjectId;

use crate::{head, remote};

/// A worktree checkout containing the files of the repository in consumable form.
#[derive(Debug, Clone)]
pub struct Worktree<'repo> {
    pub(crate) parent: &'repo Repository,
    /// The root path of the checkout.
    pub(crate) path: &'repo std::path::Path,
}

/// The head reference, as created from looking at `.git/HEAD`, able to represent all of its possible states.
///
/// Note that like [`Reference`], this type's data is snapshot of persisted state on disk.
#[derive(Clone)]
pub struct Head<'repo> {
    /// One of various possible states for the HEAD reference
    pub kind: head::Kind,
    /// The owning repository.
    pub repo: &'repo Repository,
}

/// An [`ObjectId`] with access to a repository.
#[derive(Clone, Copy)]
pub struct Id<'r> {
    /// The actual object id
    pub(crate) inner: ObjectId,
    /// The owning repository.
    pub repo: &'r Repository,
}

/// A decoded object with a reference to its owning repository.
#[derive(Clone)]
pub struct Object<'repo> {
    /// The id of the object
    pub id: ObjectId,
    /// The kind of the object
    pub kind: gix_object::Kind,
    /// The fully decoded object data
    pub data: Vec<u8>,
    /// The owning repository.
    pub repo: &'repo Repository,
}

impl Drop for Object<'_> {
    fn drop(&mut self) {
        self.repo.reuse_buffer(&mut self.data);
    }
}

/// A blob along with access to its owning repository.
#[derive(Clone)]
pub struct Blob<'repo> {
    /// The id of the tree
    pub id: ObjectId,
    /// The blob's data.
    pub data: Vec<u8>,
    /// The owning repository.
    pub repo: &'repo Repository,
}

impl Drop for Blob<'_> {
    fn drop(&mut self) {
        self.repo.reuse_buffer(&mut self.data);
    }
}

/// A decoded tree object with access to its owning repository.
#[derive(Clone)]
pub struct Tree<'repo> {
    /// Thek[ id of the tree
    pub id: ObjectId,
    /// The fully decoded tree data
    pub data: Vec<u8>,
    /// The owning repository.
    pub repo: &'repo Repository,
}

impl Drop for Tree<'_> {
    fn drop(&mut self) {
        self.repo.reuse_buffer(&mut self.data);
    }
}

/// A decoded tag object with access to its owning repository.
#[derive(Clone)]
pub struct Tag<'repo> {
    /// The id of the tree
    pub id: ObjectId,
    /// The fully decoded tag data
    pub data: Vec<u8>,
    /// The owning repository.
    pub repo: &'repo Repository,
}

impl Drop for Tag<'_> {
    fn drop(&mut self) {
        self.repo.reuse_buffer(&mut self.data);
    }
}

/// A decoded commit object with access to its owning repository.
#[derive(Clone)]
pub struct Commit<'repo> {
    /// The id of the commit
    pub id: ObjectId,
    /// The fully decoded commit data
    pub data: Vec<u8>,
    /// The owning repository.
    pub repo: &'repo Repository,
}

impl Drop for Commit<'_> {
    fn drop(&mut self) {
        self.repo.reuse_buffer(&mut self.data);
    }
}

/// A detached, self-contained object, without access to its source repository.
///
/// Use it if an `ObjectRef` should be sent over thread boundaries or stored in collections.
#[derive(Clone)]
pub struct ObjectDetached {
    /// The id of the object
    pub id: ObjectId,
    /// The kind of the object
    pub kind: gix_object::Kind,
    /// The fully decoded object data
    pub data: Vec<u8>,
}

/// A reference that points to an object or reference, with access to its source repository.
///
/// Note that these are snapshots and won't recognize if they are stale.
#[derive(Clone)]
pub struct Reference<'r> {
    /// The actual reference data
    pub inner: gix_ref::Reference,
    /// The owning repository.
    pub repo: &'r Repository,
}

/// A thread-local handle to interact with a repository from a single thread.
///
/// It is `Send`, but **not** `Sync` - for the latter you can convert it using
/// [`Repository::into_sync()`].
///
/// Note that it clones itself so that it is empty, requiring the user to configure each clone separately, specifically
/// and explicitly. This is to have the fastest-possible default configuration available by default, but allow
/// those who experiment with workloads to get speed boosts of 2x or more.
///
/// ### `Send` only with `parallel` feature
///
/// When built with `default-features = false`, this type is **not** `Send`.
/// The minimal feature set to activate `Send` is `features = ["parallel"]`.
pub struct Repository {
    /// A ref store with shared ownership (or the equivalent of it).
    pub refs: crate::RefStore,
    /// A way to access objects.
    pub objects: crate::OdbHandle,

    /// The worktree selected when the repository was opened, or `None` for a bare repository.
    pub(crate) work_tree: Option<PathBuf>,
    /// The index selected when the repository was opened, including any `gitoxide.core.indexFile` override.
    pub(crate) index_path: PathBuf,
    /// The path to the resolved common directory if this is a linked worktree repository or it is otherwise set.
    pub(crate) common_dir: Option<PathBuf>,
    /// A free-list of reusable object backing buffers
    pub(crate) bufs: Option<RefCell<Vec<Vec<u8>>>>,
    /// A pre-assembled selection of often-accessed configuration values for quick access.
    pub(crate) config: crate::config::Cache,
    /// the options obtained when instantiating this repository.
    ///
    /// Particularly useful when following linked worktrees and instantiating new equally configured worktree repositories.
    pub(crate) options: crate::open::Options,
    #[cfg(feature = "index")]
    pub(crate) index: crate::worktree::IndexStorage,
    #[cfg(feature = "attributes")]
    pub(crate) modules: crate::submodule::ModulesFileStorage,
    pub(crate) shallow_commits: crate::shallow::CommitsStorage,
}

/// An instance with access to everything a git repository entails, best imagined as container implementing `Sync + Send` for _most_
/// for system resources required to interact with a `git` repository which are loaded in once the instance is created.
///
/// Use this type to reference it in a threaded context for creation the creation of a thread-local [`Repositories`][Repository].
///
/// Note that this type purposefully isn't very useful until it is converted into a thread-local repository with `to_thread_local()`,
/// it's merely meant to be able to exist in a `Sync` context.
///
/// Note that it can also cheaply be cloned, and it will retain references to all contained resources.
///
/// ### `Send` only with `parallel` feature
///
/// When built with `default-features = false`, this type is **not** `Send`.
/// The minimal feature set to activate `Send` is `features = ["parallel"]`.
#[derive(Clone)]
pub struct ThreadSafeRepository {
    /// A store for references to point at objects
    pub refs: crate::RefStore,
    /// A store for objects that contain data
    pub objects: gix_features::threading::OwnShared<gix_odb::Store>,
    /// The path to the worktree at which to find checked out files
    pub work_tree: Option<PathBuf>,
    /// The path to the index selected when the repository was opened.
    pub(crate) index_path: PathBuf,
    /// The path to the common directory if this is a linked worktree repository or it is otherwise set.
    pub common_dir: Option<PathBuf>,
    pub(crate) config: crate::config::Cache,
    /// options obtained when instantiating this repository for use when following linked worktrees.
    pub(crate) linked_worktree_options: crate::open::Options,
    /// The index of this instances worktree.
    #[cfg(feature = "index")]
    pub(crate) index: crate::worktree::IndexStorage,
    #[cfg(feature = "attributes")]
    pub(crate) modules: crate::submodule::ModulesFileStorage,
    pub(crate) shallow_commits: crate::shallow::CommitsStorage,
}

/// A remote which represents a way to interact with hosts for remote clones of the parent repository.
#[derive(Debug, Clone, PartialEq)]
pub struct Remote<'repo> {
    /// The remotes symbolic name, only present if persisted in git configuration files.
    pub(crate) name: Option<remote::Name<'static>>,
    /// The urls of the hosts to talk to for fetching, in configuration order.
    pub(crate) urls: Vec<gix_url::Url>,
    /// Rewrite aliases corresponding one-for-one to `urls` when fetching, produced by `insteadOf` rules.
    pub(crate) url_aliases: Vec<Option<gix_url::Url>>,
    /// Rewrite aliases corresponding one-for-one to `urls` when they are push fallbacks because there are no explicit
    /// `push_urls`. Matching `pushInsteadOf` rules take precedence, with `insteadOf` used when none match.
    pub(crate) url_push_aliases: Vec<Option<gix_url::Url>>,
    /// The explicit urls to use specifically for pushing, in configuration order.
    pub(crate) push_urls: Vec<gix_url::Url>,
    /// Rewrite aliases corresponding one-for-one to explicit `push_urls`, produced only by normal `insteadOf` rules;
    /// `pushInsteadOf` deliberately does not apply to explicit push urls.
    pub(crate) push_url_aliases: Vec<Option<gix_url::Url>>,
    /// Refspecs for use when fetching.
    pub(crate) fetch_specs: Vec<gix_refspec::RefSpec>,
    /// Refspecs for use when pushing.
    pub(crate) push_specs: Vec<gix_refspec::RefSpec>,
    /// Tell us what to do with tags when fetched.
    pub(crate) fetch_tags: remote::fetch::Tags,
    // /// Delete local tracking branches that don't exist on the remote anymore.
    // pub(crate) prune: bool,
    // /// Delete tags that don't exist on the remote anymore, equivalent to pruning the refspec `refs/tags/*:refs/tags/*`.
    // pub(crate) prune_tags: bool,
    /// The owning repository.
    pub repo: &'repo Repository,
}

/// A remote without access to its source repository.
///
/// It's needed to allow flipping the underlying repository object hash.
/// Git solves this by carefully partially initialising the repository,
/// but doing this (including the transformation) seems easier.
#[derive(Debug, Clone, PartialEq)]
#[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
pub(crate) struct RemoteDetached {
    /// The remotes symbolic name, only present if persisted in git configuration files.
    pub(crate) name: Option<remote::Name<'static>>,
    /// The urls of the hosts to talk to for fetching, in configuration order.
    pub(crate) urls: Vec<gix_url::Url>,
    /// The rewritten `urls` when used for fetching.
    pub(crate) url_aliases: Vec<Option<gix_url::Url>>,
    /// Refspecs for use when fetching.
    pub(crate) fetch_specs: Vec<gix_refspec::RefSpec>,
    /// Tell us what to do with tags when fetched.
    pub(crate) fetch_tags: remote::fetch::Tags,
}

/// A utility to make matching against pathspecs simple.
///
/// Note that to perform pathspec matching, attribute access might need to be provided. For that, we use our own
/// and argue that the implementation is only going to incur costs for it when a pathspec matches *and* has attributes.
/// Should this potential duplication of effort to maintain attribute state be unacceptable, the user may fall back
/// to the underlying plumbing.
#[derive(Clone)]
#[cfg(feature = "attributes")]
pub struct Pathspec<'repo> {
    /// The owning repository.
    pub repo: &'repo Repository,
    /// The cache to power attribute access. It's only initialized if we have a pattern with attributes.
    pub(crate) stack: Option<gix_worktree::Stack>,
    /// The prepared search to use for checking matches.
    pub(crate) search: gix_pathspec::Search,
}

/// Like [`Pathspec`], but without a Repository reference and with minimal API.
#[derive(Clone)]
#[cfg(feature = "attributes")]
pub struct PathspecDetached {
    /// The cache to power attribute access. It's only initialized if we have a pattern with attributes.
    pub stack: Option<gix_worktree::Stack>,
    /// The prepared search to use for checking matches.
    pub search: gix_pathspec::Search,
    /// A thread-safe version of an ODB.
    pub odb: crate::OdbHandleArc,
}

/// A stand-in for the submodule of a particular name.
#[derive(Clone)]
#[cfg(feature = "attributes")]
pub struct Submodule<'repo> {
    pub(crate) state: std::rc::Rc<crate::submodule::SharedState<'repo>>,
    pub(crate) name: crate::bstr::BString,
}

/// A utility to access `.gitattributes` and `.gitignore` information efficiently.
#[cfg(any(feature = "attributes", feature = "excludes"))]
pub struct AttributeStack<'repo> {
    /// The owning repository.
    pub repo: &'repo Repository,
    pub(crate) inner: gix_worktree::Stack,
}
