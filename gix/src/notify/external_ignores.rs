use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use gix_error::{ErrorExt, message};
use gix_notify::{Budget, Watch};

use super::{DependencyKind, Layout, Outcome, Statistics};
use crate::Error;

#[derive(Default)]
pub(super) struct Watcher {
    pub(super) native: Option<gix_notify::Watcher>,
    pub(super) retry: Option<Instant>,
    paths: Vec<PathBuf>,
}

impl Watcher {
    pub(super) fn registered_len(&self) -> usize {
        self.native.as_ref().map_or(0, |_| self.paths.len())
    }

    pub(super) fn configure(
        &mut self,
        layout: &Layout,
        now: Instant,
        retry_interval: Duration,
        statistics: &mut Statistics,
        errors: &mut Vec<Error>,
    ) {
        let paths: Vec<_> = layout
            .dependencies
            .iter()
            .filter(|dependency| dependency.kind == DependencyKind::Ignore && !layout.contains(&dependency.path))
            .map(|dependency| dependency.path.clone())
            .collect();
        if self.paths != paths {
            self.rescan(now, statistics);
            self.paths = paths;
            self.retry = None;
            self.install(now, retry_interval, statistics, errors);
        }
    }

    pub(super) fn rescan(&mut self, now: Instant, statistics: &mut Statistics) {
        statistics.removed += self.registered_len();
        self.native = None;
        self.retry = (!self.paths.is_empty()).then_some(now);
    }

    pub(super) fn poll(
        &mut self,
        now: Instant,
        retry_interval: Duration,
        out: &mut Outcome,
        backlog: &mut bool,
    ) -> bool {
        if self.retry.is_some_and(|deadline| deadline <= now) {
            self.install(now, retry_interval, &mut out.statistics, &mut out.errors);
        }
        let Some(watcher) = self.native.as_mut() else {
            return false;
        };
        let batch = watcher.drain(Budget {
            max_events: 256,
            max_bytes: 1024 * 1024,
        });
        *backlog |= batch.more;
        out.statistics.received += batch.events.len();
        out.statistics.rescans += usize::from(batch.loss.is_some());
        let invalidated = batch.loss.is_some() || batch.error.is_some() || !batch.events.is_empty();
        if let Some(error) = batch.error {
            self.rescan(now + retry_interval, &mut out.statistics);
            out.errors
                .push(error.and_raise(message("could not monitor external ignore files")));
        }
        invalidated
    }

    fn install(
        &mut self,
        now: Instant,
        retry_interval: Duration,
        statistics: &mut Statistics,
        errors: &mut Vec<Error>,
    ) {
        if self.paths.is_empty() {
            return;
        }
        let watcher = gix_notify::Watcher::new_for_files(Default::default()).and_then(|mut watcher| {
            watcher.replace(self.paths.iter().cloned().map(|path| Watch { path, recursive: false }))?;
            Ok(watcher)
        });
        match watcher {
            Ok(watcher) => {
                statistics.added += self.paths.len();
                self.native = Some(watcher);
                self.retry = None;
            }
            Err(error) => {
                self.retry = Some(now + retry_interval);
                errors.push(error.and_raise(message("could not watch external ignore files")));
            }
        }
    }
}
