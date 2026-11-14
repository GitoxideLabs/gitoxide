use std::{
    collections::{BTreeSet, HashMap},
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use gix::parallel::{Mutable, OwnShared, lock};
use gix::{
    Result,
    error::{OptionExt, ResultExt, message},
};
use tracing_subscriber::{
    filter::{LevelFilter, Targets},
    prelude::*,
};

const FILE_PREFIX: &str = "tix.log";
const RETENTION: Duration = Duration::from_hours(7 * 24);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Trigger {
    Index,
    Refs,
    Rescan,
    Worktree,
    GitMetadata,
}

#[derive(Clone, Copy, Debug)]
enum WatcherKind {
    References,
    Worktree,
}

struct Response {
    id: u64,
    watcher: WatcherKind,
    started: std::time::Instant,
    batches: usize,
    events: usize,
    rescans: usize,
    triggers: BTreeSet<Trigger>,
    presentations: usize,
}

impl Response {
    fn new(id: u64, watcher: WatcherKind) -> Self {
        Response {
            id,
            watcher,
            started: std::time::Instant::now(),
            batches: 0,
            events: 0,
            rescans: 0,
            triggers: BTreeSet::new(),
            presentations: 0,
        }
    }

    fn observe(&mut self, changes: &gix::notify::Changes, statistics: &gix::notify::Statistics) {
        self.events += statistics.received;
        self.rescans += statistics.rescans;
        for (changed, trigger) in [
            (changes.index, Trigger::Index),
            (changes.references, Trigger::Refs),
            (changes.worktree.is_some(), Trigger::Worktree),
            (
                changes.configuration || changes.ignores || changes.attributes || changes.operations || changes.layout,
                Trigger::GitMetadata,
            ),
            (statistics.rescans != 0, Trigger::Rescan),
        ] {
            if changed {
                self.triggers.insert(trigger);
            }
        }
    }

    fn log_trigger(&self) {
        tracing::debug!(
            response_id = self.id,
            watcher = ?self.watcher,
            batches = self.batches,
            events = self.events,
            rescans = self.rescans,
            triggers = ?self.triggers,
            "filesystem UI response triggered"
        );
    }
}

#[derive(Default)]
pub(crate) struct FilesystemResponses {
    next_id: u64,
    responses: HashMap<u64, Response>,
    pending_worktree: Option<u64>,
    pending_references: Option<u64>,
    queued_references: Vec<u64>,
    active_references: Vec<u64>,
    frame_causes: Vec<(u64, &'static str)>,
    finish_after_frame: Vec<(u64, &'static str)>,
}

impl FilesystemResponses {
    pub(crate) fn observe_monitor(&mut self, changes: &gix::notify::Changes, statistics: &gix::notify::Statistics) {
        if changes.index || changes.ignores || changes.attributes || changes.configuration || changes.worktree.is_some()
        {
            let id = self.ensure_pending(WatcherKind::Worktree);
            self.responses
                .get_mut(&id)
                .expect("a pending response is registered")
                .observe(changes, statistics);
            self.note_worktree_batch();
        }
        if changes.references || changes.configuration || changes.layout {
            let id = self.ensure_pending(WatcherKind::References);
            self.responses
                .get_mut(&id)
                .expect("a pending response is registered")
                .observe(changes, statistics);
            self.note_reference_batch();
        }
    }

    pub(crate) fn note_worktree_batch(&mut self) {
        self.note_batch(self.pending_worktree);
    }

    pub(crate) fn note_reference_batch(&mut self) {
        self.note_batch(self.pending_references);
    }

    pub(crate) fn worktree_due(&mut self, invalidated: bool) -> Vec<u64> {
        let Some(id) = self.pending_worktree.take() else {
            return Vec::new();
        };
        self.log_trigger(id);
        tracing::debug!(response_id = id, invalidated, action = "worktree-cache-invalidation");
        self.queue_frame(&[id], "worktree-cache-invalidation");
        self.finish_after_frame(&[id], "completed");
        vec![id]
    }

    pub(crate) fn references_due(&mut self) -> Vec<u64> {
        let Some(id) = self.pending_references.take() else {
            return Vec::new();
        };
        self.log_trigger(id);
        self.queued_references.push(id);
        tracing::debug!(response_id = id, action = "reference-refresh-queued");
        vec![id]
    }

    pub(crate) fn begin_reference_refresh(&mut self) -> Vec<u64> {
        self.active_references = std::mem::take(&mut self.queued_references);
        self.active_references.clone()
    }

    pub(crate) fn active_reference_ids(&self) -> &[u64] {
        &self.active_references
    }

    pub(crate) fn phase(&self, ids: &[u64], action: &'static str) {
        if !ids.is_empty() {
            tracing::debug!(response_ids = ?ids, action);
        }
    }

    pub(crate) fn queue_frame(&mut self, ids: &[u64], reason: &'static str) {
        for id in ids {
            if !self.frame_causes.contains(&(*id, reason)) {
                self.frame_causes.push((*id, reason));
            }
        }
    }

    pub(crate) fn finish_after_frame(&mut self, ids: &[u64], outcome: &'static str) {
        for id in ids {
            if !self.finish_after_frame.iter().any(|(candidate, _)| candidate == id) {
                self.finish_after_frame.push((*id, outcome));
            }
        }
    }

    pub(crate) fn has_queued_frame(&self) -> bool {
        !self.frame_causes.is_empty()
    }

    pub(crate) fn cancel_pending_worktree(&mut self, outcome: &'static str) {
        if let Some(id) = self.pending_worktree.take() {
            self.log_trigger(id);
            self.finish(&[id], outcome);
        }
    }

    pub(crate) fn frame_presented(&mut self) {
        if self.frame_causes.is_empty() {
            return;
        }
        let causes = std::mem::take(&mut self.frame_causes);
        let mut ids = causes.iter().map(|(id, _)| *id).collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        for id in &ids {
            if let Some(response) = self.responses.get_mut(id) {
                response.presentations += 1;
            }
        }
        tracing::debug!(response_ids = ?ids, ?causes, "filesystem-triggered UI frame presented");

        let finishing = std::mem::take(&mut self.finish_after_frame);
        for (id, outcome) in finishing {
            self.finish(&[id], outcome);
        }
    }

    pub(crate) fn finish(&mut self, ids: &[u64], outcome: &'static str) {
        for id in ids {
            let Some(response) = self.responses.remove(id) else {
                continue;
            };
            tracing::debug!(
                response_id = response.id,
                watcher = ?response.watcher,
                outcome,
                presentations = response.presentations,
                elapsed_ms = response.started.elapsed().as_millis(),
                "filesystem UI response finished"
            );
        }
        self.active_references.retain(|id| !ids.contains(id));
        self.queued_references.retain(|id| !ids.contains(id));
        self.frame_causes.retain(|(id, _)| !ids.contains(id));
        self.finish_after_frame.retain(|(id, _)| !ids.contains(id));
    }

    fn ensure_pending(&mut self, watcher: WatcherKind) -> u64 {
        let slot = match watcher {
            WatcherKind::References => &mut self.pending_references,
            WatcherKind::Worktree => &mut self.pending_worktree,
        };
        if let Some(id) = *slot {
            return id;
        }
        self.next_id += 1;
        let id = self.next_id;
        self.responses.insert(id, Response::new(id, watcher));
        *slot = Some(id);
        id
    }

    fn log_trigger(&self, id: u64) {
        if let Some(response) = self.responses.get(&id) {
            response.log_trigger();
        }
    }

    fn note_batch(&mut self, id: Option<u64>) {
        if let Some(response) = id.and_then(|id| self.responses.get_mut(&id)) {
            response.batches += 1;
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TraceFormat {
    Forest,
    Flat,
}

fn trace_settings(trace: u8) -> Option<(TraceFormat, LevelFilter)> {
    match trace {
        1 => Some((TraceFormat::Forest, LevelFilter::INFO)),
        2 => Some((TraceFormat::Forest, LevelFilter::DEBUG)),
        3 => Some((TraceFormat::Flat, LevelFilter::DEBUG)),
        4 => Some((TraceFormat::Flat, LevelFilter::TRACE)),
        _ => None,
    }
}

#[derive(Default)]
struct TraceBuffer(Mutable<Vec<u8>>);

impl Write for &TraceBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        lock(&self.0).extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

type SharedTraceBuffer = OwnShared<TraceBuffer>;

pub(crate) struct Guard {
    _default: Option<tracing::subscriber::DefaultGuard>,
    trace: Option<SharedTraceBuffer>,
}

impl Drop for Guard {
    fn drop(&mut self) {
        let Some(trace) = self.trace.as_ref() else {
            return;
        };
        let trace = lock(&trace.0);
        let _ = std::io::stderr().write_all(&trace);
    }
}

pub(crate) fn init(trace: u8) -> Result<Guard> {
    if trace == 0 {
        return Ok(Guard {
            _default: if cfg!(test) { None } else { try_init().ok() },
            trace: None,
        });
    }
    let output = SharedTraceBuffer::default();
    try_init_trace(trace, output.clone())?;
    tracing::info!(trace, "started tix invocation");
    Ok(Guard {
        _default: None,
        trace: Some(output),
    })
}

fn trace_subscriber(trace: u8, output: SharedTraceBuffer) -> Result<Box<dyn tracing::Subscriber + Send + Sync>> {
    let (format, level) = trace_settings(trace).ok_or_raise(|| message("trace level must be between one and four"))?;
    Ok(match format {
        TraceFormat::Forest => {
            let printer = gix_trace::forest::Printer::new().writer(output);
            Box::new(tracing_subscriber::registry().with(gix_trace::ForestLayer::from(printer).with_filter(level)))
        }
        TraceFormat::Flat => Box::new(
            tracing_subscriber::registry().with(
                tracing_subscriber::fmt::layer()
                    .with_ansi(false)
                    .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
                    .with_writer(output)
                    .with_filter(level),
            ),
        ),
    })
}

fn try_init_trace(trace: u8, output: SharedTraceBuffer) -> Result<()> {
    tracing::subscriber::set_global_default(trace_subscriber(trace, output)?).or_error()?;
    Ok(())
}

fn try_init() -> Result<tracing::subscriber::DefaultGuard> {
    let directory = log_directory().ok_or_raise(|| message("could not determine the platform log directory"))?;
    fs::create_dir_all(&directory)
        .or_raise(|| message!("could not create log directory at {}", directory.display()))?;
    let cleanup_errors = prune(&directory, SystemTime::now());
    let appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix(FILE_PREFIX)
        .build(&directory)
        .or_raise(|| message("could not open the daily diagnostic log"))?;
    let subscriber = tracing_subscriber::registry().with(
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_target(false)
            .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
            .with_writer(appender)
            .with_filter(
                Targets::new()
                    .with_default(tracing::Level::WARN)
                    .with_target("gix_tix", tracing::Level::DEBUG),
            ),
    );
    let guard = tracing::subscriber::set_default(subscriber);
    tracing::info!(path = %directory.display(), "initialized diagnostics");
    for error in cleanup_errors {
        tracing::warn!(%error, "could not prune an old diagnostic log");
    }
    Ok(guard)
}

#[cfg(target_os = "macos")]
fn log_directory() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|dirs| dirs.home_dir().join("Library/Logs/org.GitoxideLabs.tix"))
}

#[cfg(target_os = "linux")]
fn log_directory() -> Option<PathBuf> {
    directories::ProjectDirs::from("org", "GitoxideLabs", "tix").and_then(|dirs| dirs.state_dir().map(Path::to_owned))
}

#[cfg(target_os = "windows")]
fn log_directory() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|dirs| dirs.data_local_dir().join("GitoxideLabs/tix/logs"))
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn log_directory() -> Option<PathBuf> {
    directories::ProjectDirs::from("org", "GitoxideLabs", "tix").map(|dirs| dirs.data_local_dir().join("logs"))
}

fn prune(directory: &Path, now: SystemTime) -> Vec<String> {
    let mut errors = Vec::new();
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(err) => {
            errors.push(err.to_string());
            return errors;
        }
    };
    for entry in entries {
        let result = (|| -> std::io::Result<()> {
            let entry = entry?;
            let name = entry.file_name();
            if !name.to_string_lossy().starts_with(&format!("{FILE_PREFIX}.")) {
                return Ok(());
            }
            let age = now.duration_since(entry.metadata()?.modified()?).unwrap_or_default();
            if age > RETENTION {
                fs::remove_file(entry.path())?;
            }
            Ok(())
        })();
        if let Err(err) = result {
            errors.push(err.to_string());
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use std::{fs::File, time::UNIX_EPOCH};

    use super::*;

    fn statistics() -> gix::notify::Statistics {
        gix::notify::Statistics {
            received: 1,
            ..Default::default()
        }
    }

    #[test]
    fn trace_repetitions_choose_format_and_level() {
        assert_eq!(trace_settings(0), None);
        assert_eq!(trace_settings(1), Some((TraceFormat::Forest, LevelFilter::INFO)));
        assert_eq!(trace_settings(2), Some((TraceFormat::Forest, LevelFilter::DEBUG)));
        assert_eq!(trace_settings(3), Some((TraceFormat::Flat, LevelFilter::DEBUG)));
        assert_eq!(trace_settings(4), Some((TraceFormat::Flat, LevelFilter::TRACE)));
        assert_eq!(trace_settings(5), None);
        assert!(
            trace_subscriber(5, SharedTraceBuffer::default()).is_err(),
            "invalid programmatic levels are reported"
        );
    }

    #[test]
    fn forest_traces_retain_worker_spans_and_latest_recorded_fields() -> Result<()> {
        for trace in [1, 2] {
            let output = SharedTraceBuffer::default();
            let subscriber = trace_subscriber(trace, output.clone())?;
            tracing::subscriber::with_default(subscriber, || {
                let root = gix_trace::coarse!("operation", completed = gix_trace::field::Empty);
                std::thread::spawn(gix_trace::in_thread(|| {
                    let worker = gix_trace::coarse!("worker", items = gix_trace::field::Empty);
                    worker.record("items", 1_u64);
                    worker.record("items", 2_u64);
                    tracing::info!("work completed");
                    tracing::trace!("filtered worker event");
                }))
                .join()
                .expect("the tracing worker must finish without panicking");
                assert!(lock(&output.0).is_empty(), "the tree waits for its root to close");
                root.record("completed", true);
            });
            let output = lock(&output.0);
            let output = String::from_utf8_lossy(&output);
            // With `forest-ansi`, a reset separates each field name from its value.
            let output = output.replace("\u{1b}[0m", "");
            assert!(
                output.contains("operation [") && output.contains("completed: true"),
                "fields recorded after the root starts are retained: {output}"
            );
            assert!(
                output.contains("┕━ worker [") && output.contains("   ┕━"),
                "the worker and its event remain nested under the root: {output}"
            );
            assert!(
                output.contains("work completed"),
                "the worker inherits the subscriber: {output}"
            );
            assert!(
                output.contains("items: 2") && !output.contains("items: 1"),
                "the worker field retains only its latest value: {output}"
            );
            assert!(
                !output.contains("filtered worker event"),
                "worker events respect the selected level: {output}"
            );
        }
        Ok(())
    }

    #[test]
    fn flat_traces_include_closed_spans_in_the_deferred_output() -> Result<()> {
        let output = SharedTraceBuffer::default();
        let subscriber = trace_subscriber(3, output.clone())?;
        tracing::subscriber::with_default(subscriber, || {
            let span = tracing::debug_span!("operation");
            let _entered = span.enter();
            tracing::debug!("visible event");
            tracing::trace!("filtered event");
        });
        let output = lock(&output.0);
        let output = String::from_utf8_lossy(&output);
        assert!(output.contains("visible event"), "debug events are retained: {output}");
        assert!(output.contains("close"), "span completion is retained: {output}");
        assert!(
            !output.contains("filtered event"),
            "the selected level still filters: {output}"
        );
        Ok(())
    }

    #[test]
    fn coalesces_batches_and_semantic_triggers() {
        let references = gix::notify::Changes {
            references: true,
            ..Default::default()
        };
        let mut responses = FilesystemResponses::default();
        responses.observe_monitor(&references, &statistics());
        let first = responses
            .pending_references
            .expect("reference changes queue a response");
        responses.observe_monitor(
            &gix::notify::Changes {
                index: true,
                configuration: true,
                worktree: Some(gix::notify::Scope::All),
                ..Default::default()
            },
            &statistics(),
        );
        assert_eq!(
            responses.pending_references,
            Some(first),
            "events before the deadline share one response"
        );
        responses.observe_monitor(
            &references,
            &gix::notify::Statistics {
                received: 1,
                rescans: 1,
                ..Default::default()
            },
        );
        assert_eq!(
            responses.pending_references,
            Some(first),
            "rescans join the pending response"
        );
        let response = responses
            .responses
            .get(&first)
            .expect("the response is retained until acted upon");
        assert_eq!(
            response.events, 3,
            "native event counts accumulate across service calls"
        );
        assert_eq!(response.rescans, 1, "coverage loss is counted independently of events");
        assert_eq!(response.batches, 3, "each service call contributes a batch");
        assert_eq!(
            response.triggers,
            [
                Trigger::Index,
                Trigger::Refs,
                Trigger::Worktree,
                Trigger::GitMetadata,
                Trigger::Rescan
            ]
            .into_iter()
            .collect(),
            "trigger categories come from semantic invalidations and coverage loss"
        );

        assert_eq!(responses.references_due(), [first]);
        responses.observe_monitor(&references, &statistics());
        assert_ne!(
            responses.pending_references,
            Some(first),
            "activity after the deadline starts another response"
        );
    }

    #[test]
    fn attributes_overlapping_responses_to_one_presented_frame() {
        let mut responses = FilesystemResponses::default();
        assert!(!responses.has_queued_frame());
        responses.observe_monitor(
            &gix::notify::Changes {
                worktree: Some(gix::notify::Scope::All),
                ..Default::default()
            },
            &statistics(),
        );
        let worktree = responses.pending_worktree.expect("worktree changes queue a response");
        responses.worktree_due(true);
        assert!(responses.has_queued_frame());

        responses.observe_monitor(
            &gix::notify::Changes {
                references: true,
                ..Default::default()
            },
            &statistics(),
        );
        let references = responses
            .pending_references
            .expect("reference changes queue a response");
        responses.references_due();
        assert_eq!(responses.begin_reference_refresh(), [references]);
        responses.queue_frame(&[references], "lane-computation-completed");
        responses.finish_after_frame(&[references], "completed");

        assert_eq!(
            responses.frame_causes,
            [
                (worktree, "worktree-cache-invalidation"),
                (references, "lane-computation-completed")
            ],
            "one frame retains both filesystem causes"
        );
        responses.frame_presented();
        assert!(!responses.has_queued_frame());
        assert!(
            responses.responses.is_empty(),
            "both responses finish after the shared frame"
        );
    }

    #[test]
    fn prunes_only_expired_daily_logs() -> gix_testtools::Result {
        let directory = std::env::temp_dir().join(format!(
            "gix-tix-log-prune-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        ));
        fs::create_dir(&directory)?;
        let old = directory.join("tix.log.older");
        let recent = directory.join("tix.log.recent");
        let unrelated = directory.join("other.log.old");
        File::create(&old)?.set_modified(UNIX_EPOCH)?;
        File::create(&recent)?;
        File::create(&unrelated)?.set_modified(UNIX_EPOCH)?;

        assert!(prune(&directory, SystemTime::now()).is_empty());
        assert!(!old.exists(), "expired tix logs are removed");
        assert!(recent.exists(), "recent tix logs are retained");
        assert!(unrelated.exists(), "unrelated files are retained");
        fs::remove_dir_all(directory)?;
        Ok(())
    }
}
