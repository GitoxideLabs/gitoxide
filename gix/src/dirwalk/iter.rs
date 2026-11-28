#[cfg(feature = "parallel")]
use gix_error::ResultExt;

use std::path::PathBuf;
#[cfg(feature = "parallel")]
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use super::Iter;
use crate::{
    PathspecDetached, Repository, Result, bstr::BString, dirwalk, util::OwnedOrStaticAtomicBool,
    worktree::IndexPersistedOrInMemory,
};

/// An entry of the directory walk as returned by the [iterator](Iter).
pub struct Item {
    /// The directory entry.
    pub entry: gix_dir::Entry,
    /// `collapsed_directory_status` is `Some(dir_status)` if this entry was part of a directory with the given
    /// `dir_status` that wasn't the same as the one of `entry` and if [gix_dir::walk::Options::emit_collapsed] was
    /// [gix_dir::walk::CollapsedEntriesEmissionMode::OnStatusMismatch]. It will also be `Some(dir_status)` if that option
    /// was [gix_dir::walk::CollapsedEntriesEmissionMode::All].
    pub collapsed_directory_status: Option<gix_dir::entry::Status>,
}

impl Item {
    fn new(entry: gix_dir::EntryRef<'_>, collapsed_directory_status: Option<gix_dir::entry::Status>) -> Self {
        Item {
            entry: entry.to_owned(),
            collapsed_directory_status,
        }
    }
}

/// The outcome of fully consumed [dirwalk iterator](Iter).
pub struct Outcome {
    /// The index originally passed in to create the iterator.
    pub index: IndexPersistedOrInMemory,
    /// The excludes stack used for the dirwalk, for access of `.gitignore` information.
    pub excludes: gix_worktree::Stack,
    /// The pathspecs used to guide the operation,
    pub pathspec: PathspecDetached,
    /// The root actually being used for the traversal, and useful to transform the paths returned for the user.
    /// It's always within the [`work-dir`](Repository::workdir).
    pub traversal_root: PathBuf,
    /// The actual result of the dirwalk.
    pub dirwalk: gix_dir::walk::Outcome,
}

/// Lifecycle
impl Iter {
    pub(crate) fn new(
        repo: &Repository,
        index: IndexPersistedOrInMemory,
        patterns: Vec<BString>,
        should_interrupt: OwnedOrStaticAtomicBool,
        options: dirwalk::Options,
    ) -> Result<Iter> {
        #[cfg(feature = "parallel")]
        {
            let repo = repo.clone().into_sync();
            let (tx, rx) = std::sync::mpsc::channel();
            let stop_worker = Arc::new(AtomicBool::new(false));
            let handle = std::thread::Builder::new()
                .name("gix::dirwalk::iter::producer".into())
                .spawn(crate::trace::in_thread({
                    let stop_worker = Arc::clone(&stop_worker);
                    move || -> Result<Outcome> {
                        let repo: Repository = repo.into();
                        let mut collect = Collect { tx, stop_worker };
                        let out = repo.dirwalk(&index, patterns, &should_interrupt, options, &mut collect)?;
                        Ok(Outcome {
                            index,
                            excludes: out.excludes.detach(),
                            pathspec: out.pathspec.detach()?,
                            traversal_root: out.traversal_root,
                            dirwalk: out.dirwalk,
                        })
                    }
                }))
                .or_error()?;

            Ok(Iter {
                rx_and_join: Some((rx, handle)),
                stop_worker,
                out: None,
            })
        }
        #[cfg(not(feature = "parallel"))]
        {
            let mut collect = Collect { items: Vec::new() };
            let out = repo.dirwalk(&index, patterns, &should_interrupt, options, &mut collect)?;
            let out = Outcome {
                index,
                excludes: out.excludes.detach(),
                pathspec: out.pathspec.detach()?,
                traversal_root: out.traversal_root,
                dirwalk: out.dirwalk,
            };

            Ok(Iter {
                items: collect.items.into_iter(),
                out: Some(out),
            })
        }
    }
}

/// Access
impl Iter {
    /// Return the outcome of the iteration, or `None` if the iterator isn't fully consumed.
    pub fn outcome_mut(&mut self) -> Option<&mut Outcome> {
        self.out.as_mut()
    }

    /// Turn the iterator into the iteration outcome, which is `None` on error or if the iteration
    /// isn't complete.
    pub fn into_outcome(mut self) -> Option<Outcome> {
        self.out.take()
    }
}

impl Iterator for Iter {
    type Item = Result<Item>;

    fn next(&mut self) -> Option<Self::Item> {
        #[cfg(feature = "parallel")]
        {
            let (rx, _join) = self.rx_and_join.as_ref()?;
            match rx.recv().ok() {
                Some(item) => Some(Ok(item)),
                None => {
                    let (_rx, handle) = self.rx_and_join.take()?;
                    match handle.join().expect("no panic") {
                        Ok(out) => {
                            self.out = Some(out);
                            None
                        }
                        Err(err) => Some(Err(err)),
                    }
                }
            }
        }
        #[cfg(not(feature = "parallel"))]
        self.items.next().map(Ok)
    }
}

#[cfg(feature = "parallel")]
impl Drop for Iter {
    fn drop(&mut self) {
        if self.rx_and_join.is_some() {
            self.stop_worker.store(true, Ordering::Relaxed);
        }
    }
}

struct Collect {
    #[cfg(feature = "parallel")]
    tx: std::sync::mpsc::Sender<Item>,
    #[cfg(feature = "parallel")]
    stop_worker: Arc<AtomicBool>,
    #[cfg(not(feature = "parallel"))]
    items: Vec<Item>,
}

impl gix_dir::walk::Delegate for Collect {
    fn should_interrupt(&self) -> bool {
        #[cfg(feature = "parallel")]
        {
            self.stop_worker.load(Ordering::Relaxed)
        }
        #[cfg(not(feature = "parallel"))]
        {
            false
        }
    }

    fn emit(
        &mut self,
        entry: gix_dir::EntryRef<'_>,
        collapsed_directory_status: Option<gix_dir::entry::Status>,
    ) -> gix_dir::walk::Action {
        let item = Item::new(entry, collapsed_directory_status);
        #[cfg(feature = "parallel")]
        if self.tx.send(item).is_err() {
            return std::ops::ControlFlow::Break(());
        }
        #[cfg(not(feature = "parallel"))]
        self.items.push(item);
        std::ops::ControlFlow::Continue(())
    }
}
