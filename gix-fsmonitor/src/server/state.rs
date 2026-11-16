use super::{Layout, Options, journal};
use gix_error::{Result, ResultExt, message};
use std::time::{Duration, Instant};

pub(super) struct State {
    pub(super) layout: Layout,
    pub(super) options: Options,
    pub(super) journal: journal::Journal,
    pub(super) generation: Option<u64>,
    pub(super) retry: Option<Instant>,
    pub(super) healthy: bool,
    pub(super) root_lost: bool,
}

impl State {
    pub(super) fn new(layout: Layout, options: Options) -> Result<Self> {
        let state = Self {
            layout,
            options,
            journal: journal::Journal::new(16384, 16 * 1024 * 1024, Duration::from_secs(300))
                .or_raise(|| message("generating a fresh filesystem monitor token epoch"))?,
            generation: None,
            retry: None,
            healthy: false,
            root_lost: false,
        };
        Ok(state)
    }

    pub(super) fn ingest(&mut self, batch: gix_notify::Batch, now: Instant) {
        if self.generation != Some(batch.generation) || batch.loss.is_some() {
            self.journal.reset();
            self.generation = Some(batch.generation);
        }
        if matches!(
            batch.loss,
            Some(gix_notify::Loss::BackendError | gix_notify::Loss::BackendStopped)
        ) {
            self.healthy = false;
            self.retry.get_or_insert(now + self.options.retry_interval);
        }
        if let Some(_error) = batch.error {
            gix_trace::warn!(error = %_error, "filesystem notification error");
            self.healthy = false;
            self.retry.get_or_insert(now + self.options.retry_interval);
        }
        for event in batch.events {
            if self.layout.root_lost(&event) {
                self.root_lost = true;
                self.journal.reset();
                continue;
            }
            // Creating a cookie can change its directory's metadata on Windows without losing
            // coverage. Creation, removal, rename, or an unknown change can replace that directory.
            if event.kind != gix_notify::EventKind::Modify
                && event
                    .paths
                    .iter()
                    .any(|path| self.layout.relative_to(&self.layout.cookie_dir, path).is_some())
            {
                self.journal.reset();
                self.healthy = false;
                self.retry.get_or_insert(now);
            }
            match self.layout.paths(event) {
                Some(paths) => {
                    for path in paths {
                        self.journal.record(path, now);
                    }
                }
                None => self.journal.reset(),
            }
        }
    }
}
