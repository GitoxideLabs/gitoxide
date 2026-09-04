use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use gix_features::progress::{NestedProgress, Progress};

/// The kind of `HEAD` to install in a newly-created worktree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Head {
    /// Check out an existing local branch.
    Attached(gix_ref::FullName),
    /// Check out a commit with a detached `HEAD`.
    Detached(gix_hash::ObjectId),
}

/// The error returned by [`Repository::create_worktree()`][crate::Repository::create_worktree()].
#[derive(Debug, thiserror::Error)]
#[expect(missing_docs)]
pub enum Error {
    #[error("{name:?} is not a local branch")]
    NotLocalBranch { name: gix_ref::FullName },
    #[error("The local branch {name:?} is checked out in {worktree_dirs:?}")]
    CheckedOut {
        name: gix_ref::FullName,
        worktree_dirs: Vec<std::path::PathBuf>,
    },
    #[error("The worktree destination {destination:?} is already registered")]
    DestinationRegistered { destination: std::path::PathBuf },
    #[error("Failed to read or iterate worktree directories")]
    WorktreeListing(#[source] std::io::Error),
    #[error("Could not open a worktree repository")]
    OpenWorktreeRepo(#[source] crate::open::Error),
    #[error("Failed to follow a symbolic reference while inspecting worktrees")]
    FollowSymref(#[source] gix_ref::file::find::existing::Error),
    #[error("The local branch could not be found")]
    FindBranch(#[from] crate::reference::find::existing::Error),
    #[error("The local branch could not be peeled to a commit")]
    PeelBranch(#[from] crate::reference::peel::to_kind::Error),
    #[error("The detached target is not an existing commit")]
    FindDetachedCommit(#[from] crate::object::find::existing::with_conversion::Error),
    #[error("Could not decode the commit")]
    DecodeCommit(#[from] gix_object::decode::Error),
    #[error("Could not prepare the linked worktree")]
    Prepare(#[source] std::io::Error),
    #[error("Could not write the linked worktree HEAD")]
    WriteHead(#[source] std::io::Error),
    #[error("Could not create an index from the target tree")]
    IndexFromTree(#[from] crate::repository::index_from_tree::Error),
    #[error(transparent)]
    CheckoutOptions(#[from] crate::config::checkout_options::Error),
    #[error("Failed to reopen the object database for checkout")]
    OpenArcOdb(#[source] std::io::Error),
    #[error(transparent)]
    Checkout(#[from] gix_worktree_state::checkout::Error),
    #[error("Worktree creation was interrupted")]
    Interrupted,
    #[error("Could not write the linked worktree index")]
    WriteIndex(#[from] gix_index::file::write::Error),
    #[error("Could not finish creating the linked worktree")]
    Persist(#[source] std::io::Error),
}

impl crate::Repository {
    /// Create and check out a linked worktree at `destination` with the given `head`.
    ///
    /// Attached heads must name an existing local branch which isn't checked out in another worktree.
    /// The destination must either not exist or be an empty, unregistered directory. Any files created by this
    /// method are removed if creation fails or is interrupted.
    pub fn create_worktree<P>(
        &self,
        destination: impl AsRef<Path>,
        head: Head,
        mut progress: P,
        should_interrupt: &AtomicBool,
    ) -> Result<(crate::Repository, gix_worktree_state::checkout::Outcome), Error>
    where
        P: NestedProgress,
        P::SubProgress: NestedProgress + 'static,
    {
        let destination = destination.as_ref();
        let (head_target, root_tree_id) = match head {
            Head::Attached(name) => {
                if name.category() != Some(gix_ref::Category::LocalBranch) {
                    return Err(Error::NotLocalBranch { name });
                }
                let checked_out = self.checked_out_branches().map_err(|err| match err {
                    crate::repository::worktree::CheckedOutBranchesError::WorktreeListing(err) => {
                        Error::WorktreeListing(err)
                    }
                    crate::repository::worktree::CheckedOutBranchesError::OpenWorktreeRepo(err) => {
                        Error::OpenWorktreeRepo(err)
                    }
                    crate::repository::worktree::CheckedOutBranchesError::FollowSymref(err) => Error::FollowSymref(err),
                })?;
                if let Some(worktree_dirs) = checked_out.get(&name) {
                    return Err(Error::CheckedOut {
                        name,
                        worktree_dirs: worktree_dirs.clone(),
                    });
                }
                let mut reference = self.find_reference(name.as_ref())?;
                let root_tree_id = reference.peel_to_commit()?.tree_id()?.detach();
                (gix_ref::Target::Symbolic(name), root_tree_id)
            }
            Head::Detached(commit_id) => {
                let root_tree_id = self.find_commit(commit_id)?.tree_id()?.detach();
                (gix_ref::Target::Object(commit_id), root_tree_id)
            }
        };
        if should_interrupt.load(Ordering::Relaxed) {
            return Err(Error::Interrupted);
        }

        let main_repo = self.main_repo().map_err(Error::OpenWorktreeRepo)?;
        let mut registered_destinations = main_repo.workdir().map(Path::to_owned).into_iter().collect::<Vec<_>>();
        for worktree in self.worktrees().map_err(Error::WorktreeListing)? {
            registered_destinations.push(worktree.base().map_err(Error::WorktreeListing)?);
        }
        let prepared = gix_worktree::create::prepare(self.common_dir(), destination).map_err(Error::Prepare)?;
        let canonical_destination = std::fs::canonicalize(prepared.work_dir()).map_err(Error::Prepare)?;
        for registered_destination in registered_destinations {
            let registered_destination = gix_path::realpath(registered_destination)
                .map_err(|err| Error::WorktreeListing(std::io::Error::other(err)))?;
            let same_destination = if registered_destination == prepared.work_dir() {
                true
            } else {
                match std::fs::canonicalize(&registered_destination) {
                    Ok(path) => path == canonical_destination,
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => false,
                    Err(err) => return Err(Error::WorktreeListing(err)),
                }
            };
            if same_destination {
                return Err(Error::DestinationRegistered {
                    destination: destination.to_owned(),
                });
            }
        }
        let mut head_contents = Vec::with_capacity(6 + self.object_hash().len_in_hex());
        match &head_target {
            gix_ref::Target::Object(id) => id.write_hex_to(&mut head_contents).map_err(Error::WriteHead)?,
            gix_ref::Target::Symbolic(name) => {
                head_contents.extend_from_slice(b"ref: ");
                head_contents.extend_from_slice(name.as_bstr());
            }
        }
        head_contents.push(b'\n');
        std::fs::write(prepared.git_dir().join("HEAD"), head_contents).map_err(Error::WriteHead)?;

        let options = self
            .options
            .clone()
            .without_repository_environment_overrides()
            .open_path_as_is(true);
        let repo = crate::ThreadSafeRepository::open_opts(prepared.git_dir(), options)
            .map_err(Error::OpenWorktreeRepo)?
            .to_thread_local();
        let mut index = repo.index_from_tree(&root_tree_id)?;
        let mut checkout_options = repo.checkout_options(gix_worktree::stack::state::attributes::Source::IdMapping)?;
        checkout_options.destination_is_initially_empty = true;

        let mut files = progress.add_child("checkout");
        let mut bytes = progress.add_child("writing");
        files.init(Some(index.entries().len()), crate::progress::count("files"));
        bytes.init(None, crate::progress::bytes());
        let started = std::time::Instant::now();
        let outcome = gix_worktree_state::checkout(
            &mut index,
            prepared.work_dir(),
            repo.objects.clone().into_arc().map_err(Error::OpenArcOdb)?,
            &files,
            &bytes,
            should_interrupt,
            checkout_options,
        )?;
        files.show_throughput(started);
        bytes.show_throughput(started);
        if should_interrupt.load(Ordering::Relaxed) {
            return Err(Error::Interrupted);
        }
        index.write(Default::default())?;
        prepared.persist().map_err(Error::Persist)?;
        Ok((repo, outcome))
    }
}
