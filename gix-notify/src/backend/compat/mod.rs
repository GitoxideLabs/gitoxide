use gix_error::{ErrorExt, ResultExt, message};
use notify::{EventKind as Kind, Watcher as _};

use crate::{Error, Event, EventKind, Loss, PathKind, Watch, queue};

pub(crate) struct Backend {
    watcher: notify::RecommendedWatcher,
    watches: Vec<Watch>,
}

impl Backend {
    pub(crate) fn synchronize_at(
        &mut self,
        _: &std::path::Path,
        _: std::time::Instant,
        _: u64,
    ) -> Result<crate::Fence, crate::SynchronizeError> {
        Err(crate::SynchronizeError::Unsupported)
    }

    pub(crate) fn new(watches: &[Watch], sender: queue::Sender) -> Result<Self, Error> {
        let watcher = notify::recommended_watcher(move |event| receive(&sender, event))
            .or_raise(|| message("could not create filesystem notification backend"))?;
        let mut out = Self {
            watcher,
            watches: Vec::new(),
        };
        out.replace(watches)?;
        Ok(out)
    }

    pub(crate) fn replace(&mut self, watches: &[Watch]) -> Result<(), Error> {
        // Resolve aliases so native registrations and emitted paths use the same spelling.
        let mut paths = std::collections::BTreeMap::new();
        for watch in watches {
            let path = watch
                .path
                .canonicalize()
                .or_raise(|| message!("could not resolve watched path {}", watch.path.display()))?;
            paths
                .entry(path)
                .and_modify(|recursive| *recursive |= watch.recursive)
                .or_insert(watch.recursive);
        }
        let watches: Vec<_> = paths
            .iter()
            .filter(|(path, _)| !path.ancestors().skip(1).any(|parent| paths.get(parent) == Some(&true)))
            .map(|(path, recursive)| Watch {
                path: path.clone(),
                recursive: *recursive,
            })
            .collect();
        for watch in &self.watches {
            if watches.binary_search(watch).is_err() {
                match self.watcher.unwatch(&watch.path) {
                    Err(error) if matches!(error.kind, notify::ErrorKind::WatchNotFound) => {}
                    result => result.or_raise(|| message!("could not stop watching {}", watch.path.display()))?,
                }
            }
        }
        for watch in &watches {
            if self.watches.binary_search(watch).is_ok() {
                continue;
            }
            self.watcher
                .watch(
                    &watch.path,
                    if watch.recursive {
                        notify::RecursiveMode::Recursive
                    } else {
                        notify::RecursiveMode::NonRecursive
                    },
                )
                .or_raise(|| message!("could not watch {}", watch.path.display()))?;
        }
        self.watches = watches;
        Ok(())
    }
}

fn receive(sender: &queue::Sender, event: notify::Result<notify::Event>) {
    match event {
        Ok(event) => {
            // Loss flags are meaningful even on events whose paths or access kind we ignore.
            let loss = event.need_rescan().then_some(Loss::Rescan);
            if let Some(event) = translate(event) {
                if event.paths.is_empty() {
                    sender.publish(Vec::new(), Some(loss.unwrap_or(Loss::Rescan)), None);
                } else {
                    sender.publish(vec![event], loss, None);
                }
            } else if loss.is_some() {
                sender.publish(Vec::new(), loss, None);
            }
        }
        Err(error) => sender.publish(
            Vec::new(),
            Some(Loss::BackendError),
            Some(error.and_raise(message("filesystem notification backend failed"))),
        ),
    }
}

fn translate(event: notify::Event) -> Option<Event> {
    use notify::event::{AccessKind, AccessMode, CreateKind, ModifyKind, RemoveKind};

    let (kind, path_kind) = match event.kind {
        Kind::Access(AccessKind::Close(AccessMode::Write)) => (EventKind::Modify, PathKind::Any),
        Kind::Access(_) => return None,
        Kind::Create(kind) => (
            EventKind::Create,
            match kind {
                CreateKind::File => PathKind::File,
                CreateKind::Folder => PathKind::Directory,
                _ => PathKind::Any,
            },
        ),
        Kind::Remove(kind) => (
            EventKind::Remove,
            match kind {
                RemoveKind::File => PathKind::File,
                RemoveKind::Folder => PathKind::Directory,
                _ => PathKind::Any,
            },
        ),
        Kind::Modify(ModifyKind::Name(_)) => (EventKind::Rename, PathKind::Any),
        Kind::Modify(_) => (EventKind::Modify, PathKind::Any),
        _ => (EventKind::Any, PathKind::Any),
    };
    Some(Event {
        paths: event.paths,
        kind,
        path_kind,
    })
}

#[cfg(test)]
mod tests;
