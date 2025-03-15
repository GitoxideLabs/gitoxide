use super::Iter;
use crate::bstr::BString;
use crate::util::OwnedOrStaticAtomicBool;
use crate::worktree::IndexPersistedOrInMemory;
use crate::{dirwalk, PathspecDetached, Repository};
use std::path::PathBuf;

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

/// The error returned by [Repository::dirwalk_iter()].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Failed to spawn producer thread")]
    #[cfg(feature = "parallel")]
    SpawnThread(#[from] std::io::Error),
    #[error(transparent)]
    #[cfg(not(feature = "parallel"))]
    Dirwalk(#[from] dirwalk::Error),
    #[error(transparent)]
    #[cfg(not(feature = "parallel"))]
    DetachPathSpec(#[from] std::io::Error),
}

/// Lifecycle
impl Iter {
    pub(crate) fn new(
        repo: &Repository,
        index: IndexPersistedOrInMemory,
        patterns: Vec<BString>,
        should_interrupt: OwnedOrStaticAtomicBool,
        options: dirwalk::Options,
    ) -> Result<Iter, Error> {
        #[cfg(feature = "parallel")]
        {
            let repo = repo.clone().into_sync();
            let (tx, rx) = std::sync::mpsc::channel();
            let handle = std::thread::Builder::new()
                .name("gix::dirwalk::iter::producer".into())
                .spawn({
                    let should_interrupt = should_interrupt.clone();
                    move || -> Result<Outcome, dirwalk::Error> {
                        let repo: Repository = repo.into();
                        let mut collect = Collect { tx };
                        let out = repo.dirwalk(&index, patterns, &should_interrupt, options, &mut collect)?;
                        Ok(Outcome {
                            index,
                            excludes: out.excludes.detach(),
                            pathspec: out.pathspec.detach().map_err(|err| {
                                dirwalk::Error::Walk(gix_dir::walk::Error::ReadDir {
                                    path: repo.git_dir().to_owned(),
                                    source: err,
                                })
                            })?,
                            traversal_root: out.traversal_root,
                            dirwalk: out.dirwalk,
                        })
                    }
                })?;

            Ok(Iter {
                rx_and_join: Some((rx, handle)),
                should_interrupt,
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
    type Item = Result<Item, dirwalk::Error>;

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
        crate::util::parallel_iter_drop(
            self.rx_and_join
                .take()
                .map(|(rx, handle)| (rx, handle, None::<std::thread::JoinHandle<()>>)),
            &self.should_interrupt,
        );
    }
}

struct Collect {
    #[cfg(feature = "parallel")]
    tx: std::sync::mpsc::Sender<Item>,
    #[cfg(not(feature = "parallel"))]
    items: Vec<Item>,
}

impl gix_dir::walk::Delegate for Collect {
    fn emit(
        &mut self,
        entry: gix_dir::EntryRef<'_>,
        collapsed_directory_status: Option<gix_dir::entry::Status>,
    ) -> gix_dir::walk::Action {
        // NOTE: we assume that the receiver triggers interruption so the operation will stop if the receiver is down.
        let item = Item::new(entry, collapsed_directory_status);
        #[cfg(feature = "parallel")]
        self.tx.send(item).ok();
        #[cfg(not(feature = "parallel"))]
        self.items.push(item);
        gix_dir::walk::Action::Continue
    }
}
