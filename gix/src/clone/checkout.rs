use crate::{clone::PrepareCheckout, Repository};

///
pub mod main_worktree {
    use std::{path::PathBuf, sync::atomic::AtomicBool};

    use crate::{clone::PrepareCheckout, Progress, Repository};

    /// The error returned by [`PrepareCheckout::main_worktree()`].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Repository at \"{}\" is a bare repository and cannot have a main worktree checkout", git_dir.display())]
        BareRepository { git_dir: PathBuf },
        #[error("The object pointed to by HEAD is not a treeish")]
        NoHeadTree(#[from] crate::object::peel::to_kind::Error),
        #[error("Could not create index from tree at {id}")]
        IndexFromTree {
            id: gix_hash::ObjectId,
            source: gix_index::init::from_tree::Error,
        },
        #[error("Couldn't obtain configuration for core.protect*")]
        BooleanConfig(#[from] crate::config::boolean::Error),
        #[error(transparent)]
        WriteIndex(#[from] gix_index::file::write::Error),
        #[error(transparent)]
        CheckoutOptions(#[from] crate::config::checkout_options::Error),
        #[error(transparent)]
        IndexCheckout(#[from] gix_worktree_state::checkout::Error),
        #[error(transparent)]
        Peel(#[from] crate::reference::peel::Error),
        #[error("Failed to reopen object database as Arc (only if thread-safety wasn't compiled in)")]
        OpenArcOdb(#[from] std::io::Error),
        #[error("The HEAD reference could not be located")]
        FindHead(#[from] crate::reference::find::existing::Error),
        #[error("The HEAD reference could not be located")]
        PeelHeadToId(#[from] crate::head::peel::Error),
    }

    /// The progress ids used in [`PrepareCheckout::main_worktree()`].
    ///
    /// Use this information to selectively extract the progress of interest in case the parent application has custom visualization.
    #[derive(Debug, Copy, Clone)]
    pub enum ProgressId {
        /// The amount of files checked out thus far.
        CheckoutFiles,
        /// The amount of bytes written in total, the aggregate of the size of the content of all files thus far.
        BytesWritten,
    }

    impl From<ProgressId> for gix_features::progress::Id {
        fn from(v: ProgressId) -> Self {
            match v {
                ProgressId::CheckoutFiles => *b"CLCF",
                ProgressId::BytesWritten => *b"CLCB",
            }
        }
    }

    /// Modification
    impl PrepareCheckout {
        /// Checkout the main worktree, determining how many threads to use by looking at `checkout.workers`, defaulting to using
        /// on thread per logical core.
        ///
        /// Note that this is a no-op if the remote was empty, leaving this repository empty as well. This can be validated by checking
        /// if the `head()` of the returned repository is *not* unborn.
        ///
        /// # Panics
        ///
        /// If called after it was successful. The reason here is that it auto-deletes the contained repository,
        /// and keeps track of this by means of keeping just one repository instance, which is passed to the user
        /// after success.
        pub fn main_worktree<P>(
            &mut self,
            mut progress: P,
            should_interrupt: &AtomicBool,
        ) -> Result<(Repository, gix_worktree_state::checkout::Outcome), Error>
        where
            P: gix_features::progress::NestedProgress,
            P::SubProgress: gix_features::progress::NestedProgress + 'static,
        {
            self.main_worktree_inner(&mut progress, should_interrupt)
        }

        fn main_worktree_inner(
            &mut self,
            progress: &mut dyn gix_features::progress::DynNestedProgress,
            should_interrupt: &AtomicBool,
        ) -> Result<(Repository, gix_worktree_state::checkout::Outcome), Error> {
            let _span = gix_trace::coarse!("gix::clone::PrepareCheckout::main_worktree()");
            let repo = self
                .repo
                .as_ref()
                .expect("BUG: this method may only be called until it is successful");
            let workdir = repo.workdir().ok_or_else(|| Error::BareRepository {
                git_dir: repo.git_dir().to_owned(),
            })?;

            let root_tree_id = match &self.ref_name {
                Some(reference_val) => Some(repo.find_reference(reference_val)?.peel_to_id_in_place()?),
                None => repo.head()?.try_peel_to_id_in_place()?,
            };

            let root_tree = match root_tree_id {
                Some(id) => id.object().expect("downloaded from remote").peel_to_tree()?.id,
                None => {
                    return Ok((
                        self.repo.take().expect("still present"),
                        gix_worktree_state::checkout::Outcome::default(),
                    ));
                }
            };

            let index = gix_index::State::from_tree(&root_tree, &repo.objects, repo.config.protect_options()?)
                .map_err(|err| Error::IndexFromTree {
                    id: root_tree,
                    source: err,
                })?;
            let mut index = gix_index::File::from_state(index, repo.index_path());

            let mut opts = repo.checkout_options(gix_worktree::stack::state::attributes::Source::IdMapping)?;
            opts.destination_is_initially_empty = true;

            let mut files = progress.add_child_with_id("checkout".to_string(), ProgressId::CheckoutFiles.into());
            let mut bytes = progress.add_child_with_id("writing".to_string(), ProgressId::BytesWritten.into());

            files.init(Some(index.entries().len()), crate::progress::count("files"));
            bytes.init(None, crate::progress::bytes());

            let start = std::time::Instant::now();
            let outcome = gix_worktree_state::checkout(
                &mut index,
                workdir,
                repo.objects.clone().into_arc()?,
                &files,
                &bytes,
                should_interrupt,
                opts,
            )?;
            files.show_throughput(start);
            bytes.show_throughput(start);

            index.write(Default::default())?;
            Ok((self.repo.take().expect("still present").clone(), outcome))
        }
    }
}

/// Access
impl PrepareCheckout {
    /// Get access to the repository while the checkout isn't yet completed.
    ///
    /// # Panics
    ///
    /// If the checkout is completed and the [`Repository`] was already passed on to the caller.
    pub fn repo(&self) -> &Repository {
        self.repo
            .as_ref()
            .expect("present as checkout operation isn't complete")
    }
}

/// Consumption
impl PrepareCheckout {
    /// Persist the contained repository as is even if an error may have occurred when checking out the main working tree.
    pub fn persist(mut self) -> Repository {
        self.repo.take().expect("present and consumed once")
    }
}

impl Drop for PrepareCheckout {
    fn drop(&mut self) {
        if let Some(repo) = self.repo.take() {
            std::fs::remove_dir_all(repo.workdir().unwrap_or_else(|| repo.path())).ok();
        }
    }
}

impl From<PrepareCheckout> for Repository {
    fn from(prep: PrepareCheckout) -> Self {
        prep.persist()
    }
}
