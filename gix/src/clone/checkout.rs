use crate::{Repository, clone::PrepareCheckout};

///
pub mod main_worktree {
    use std::sync::atomic::AtomicBool;

    use gix_error::{ErrorExt, ResultExt};

    use crate::{Progress, Repository, clone::PrepareCheckout};

    /// The error returned by [`PrepareCheckout::main_worktree()`].
    pub type Error = gix_error::Error;

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
            let workdir = repo.workdir().ok_or_else(|| {
                gix_error::Error::from_error(gix_error::message!(
                    "Repository at \"{}\" is a bare repository and cannot have a main worktree checkout",
                    repo.git_dir().display()
                ))
            })?;

            let root_tree_id = match &self.ref_name {
                Some(reference_val) => Some(
                    repo.find_reference(reference_val)
                        .or_raise(|| gix_error::message("The HEAD reference could not be located"))?
                        .peel_to_id()
                        .map_err(gix_error::Error::from_error)?,
                ),
                None => repo
                    .head()
                    .or_raise(|| gix_error::message("The HEAD reference could not be located"))?
                    .try_peel_to_id()
                    .or_raise(|| gix_error::message("The HEAD reference could not be located"))?,
            };

            let root_tree = match root_tree_id {
                Some(id) => {
                    id.object()
                        .expect("downloaded from remote")
                        .peel_to_tree()
                        .or_raise(|| gix_error::message("The object pointed to by HEAD is not a treeish"))?
                        .id
                }
                None => {
                    return Ok((
                        self.repo.take().expect("still present"),
                        gix_worktree_state::checkout::Outcome::default(),
                    ));
                }
            };

            let protect_options = repo
                .config
                .protect_options()
                .or_raise(|| gix_error::message("Couldn't obtain configuration for core.protect*"))?;
            let index = gix_index::State::from_tree(&root_tree, &repo.objects, protect_options).map_err(|err| {
                gix_error::Error::from(
                    err.and_raise(gix_error::message!("Could not create index from tree at {root_tree}")),
                )
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
                repo.objects.clone().into_arc().or_raise(|| {
                    gix_error::message(
                        "Failed to reopen object database as Arc (only if thread-safety wasn't compiled in)",
                    )
                })?,
                &files,
                &bytes,
                should_interrupt,
                opts,
            )
            .map_err(gix_error::Error::from_error)?;
            files.show_throughput(start);
            bytes.show_throughput(start);

            index.write(Default::default()).map_err(gix_error::Error::from_error)?;
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
            super::cleanup_clone_destination_on_drop(&repo, self.remove_worktree_on_drop);
        }
    }
}

impl From<PrepareCheckout> for Repository {
    fn from(prep: PrepareCheckout) -> Self {
        prep.persist()
    }
}
