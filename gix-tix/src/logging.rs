use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use anyhow::{Context, Result};
use tracing_subscriber::{filter::Targets, prelude::*};

const FILE_PREFIX: &str = "tix.log";
const RETENTION: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const MAX_TRIGGER_PATHS: usize = 16;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Trigger {
    Head,
    Index,
    PackedRefs,
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
    kinds: BTreeMap<&'static str, usize>,
    triggers: BTreeSet<Trigger>,
    seen_paths: HashSet<PathBuf>,
    paths: Vec<PathBuf>,
    omitted_paths: usize,
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
            kinds: BTreeMap::new(),
            triggers: BTreeSet::new(),
            seen_paths: HashSet::new(),
            paths: Vec::new(),
            omitted_paths: 0,
            presentations: 0,
        }
    }

    fn observe(&mut self, event: &notify::Event, classify: impl Fn(&Path) -> Trigger) {
        self.events += 1;
        self.rescans += usize::from(event.need_rescan());
        *self.kinds.entry(event_kind(&event.kind)).or_default() += 1;
        if event.need_rescan() {
            self.triggers.insert(Trigger::Rescan);
        }
        for path in &event.paths {
            self.triggers.insert(classify(path));
            if !self.seen_paths.insert(path.clone()) {
                continue;
            }
            if self.paths.len() < MAX_TRIGGER_PATHS {
                self.paths.push(path.clone());
            } else {
                self.omitted_paths += 1;
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
            event_kinds = ?self.kinds,
            triggers = ?self.triggers,
            paths = ?self.paths,
            omitted_paths = self.omitted_paths,
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
    pub(crate) fn observe_worktree(&mut self, event: &notify::Event, workdir: &Path, index: &Path) -> u64 {
        let id = self.ensure_pending(WatcherKind::Worktree);
        self.responses
            .get_mut(&id)
            .expect("a pending response is registered")
            .observe(event, |path| {
                if path == index {
                    Trigger::Index
                } else if path.starts_with(workdir) {
                    Trigger::Worktree
                } else {
                    Trigger::GitMetadata
                }
            });
        id
    }

    pub(crate) fn observe_references(&mut self, event: &notify::Event, git_dir: &Path, common_dir: &Path) -> u64 {
        let id = self.ensure_pending(WatcherKind::References);
        self.responses
            .get_mut(&id)
            .expect("a pending response is registered")
            .observe(event, |path| classify_reference_path(path, git_dir, common_dir));
        id
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

    pub(crate) fn fail_pending_worktree(&mut self) {
        self.cancel_pending_worktree("watcher-failure");
    }

    pub(crate) fn cancel_pending_worktree(&mut self, outcome: &'static str) {
        if let Some(id) = self.pending_worktree.take() {
            self.log_trigger(id);
            self.finish(&[id], outcome);
        }
    }

    pub(crate) fn fail_pending_references(&mut self) {
        if let Some(id) = self.pending_references.take() {
            self.log_trigger(id);
            self.finish(&[id], "watcher-failure");
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

fn event_kind(kind: &notify::EventKind) -> &'static str {
    match kind {
        notify::EventKind::Access(_) => "access",
        notify::EventKind::Create(_) => "create",
        notify::EventKind::Modify(_) => "modify",
        notify::EventKind::Remove(_) => "remove",
        notify::EventKind::Other => "other",
        notify::EventKind::Any => "any",
    }
}

fn classify_reference_path(path: &Path, git_dir: &Path, common_dir: &Path) -> Trigger {
    let head = git_dir.join("HEAD");
    let common_head = common_dir.join("HEAD");
    let linked_head = path
        .strip_prefix(common_dir.join("worktrees"))
        .ok()
        .is_some_and(|relative| {
            let mut components = relative.components();
            let name = components
                .nth(1)
                .map(|component| component.as_os_str().as_encoded_bytes());
            matches!(name, Some(b"HEAD" | b"HEAD.lock")) && components.next().is_none()
        });
    let index = git_dir.join("index");
    let packed_refs = common_dir.join("packed-refs");
    if path == head
        || path == head.with_extension("lock")
        || path == common_head
        || path == common_head.with_extension("lock")
        || linked_head
    {
        Trigger::Head
    } else if path == index || path == index.with_extension("lock") {
        Trigger::Index
    } else if path == packed_refs || path == packed_refs.with_extension("lock") {
        Trigger::PackedRefs
    } else if path.starts_with(git_dir.join("refs")) || path.starts_with(common_dir.join("refs")) {
        Trigger::Refs
    } else {
        Trigger::GitMetadata
    }
}

pub(crate) fn init() -> Option<tracing::subscriber::DefaultGuard> {
    try_init().ok()
}

fn try_init() -> Result<tracing::subscriber::DefaultGuard> {
    let directory = log_directory().context("could not determine the platform log directory")?;
    fs::create_dir_all(&directory)
        .with_context(|| format!("could not create log directory at {}", directory.display()))?;
    let cleanup_errors = prune(&directory, SystemTime::now());
    let appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix(FILE_PREFIX)
        .build(&directory)
        .context("could not open the daily diagnostic log")?;
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

    use notify::event::{Flag, ModifyKind};

    use super::*;

    fn modified(path: impl Into<PathBuf>) -> notify::Event {
        notify::Event::new(notify::EventKind::Modify(ModifyKind::Any)).add_path(path.into())
    }

    #[test]
    fn classifies_reference_triggers() {
        let common = Path::new("/repo/.git");
        let linked = common.join("worktrees/topic");
        assert_eq!(
            classify_reference_path(&linked.join("HEAD"), &linked, common),
            Trigger::Head
        );
        assert_eq!(
            classify_reference_path(&linked.join("HEAD.lock"), &linked, common),
            Trigger::Head,
            "transaction lock files retain their semantic trigger"
        );
        assert_eq!(
            classify_reference_path(&common.join("worktrees/other/HEAD"), &linked, common),
            Trigger::Head,
            "other linked worktree HEADs retain their semantic trigger"
        );
        assert_eq!(
            classify_reference_path(&linked.join("index"), &linked, common),
            Trigger::Index
        );
        assert_eq!(
            classify_reference_path(&common.join("packed-refs"), &linked, common),
            Trigger::PackedRefs
        );
        assert_eq!(
            classify_reference_path(&common.join("refs/heads/main"), &linked, common),
            Trigger::Refs
        );
        assert_eq!(
            classify_reference_path(&common.join("config"), &linked, common),
            Trigger::GitMetadata
        );
    }

    #[test]
    fn coalesces_batches_and_bounds_deduplicated_trigger_paths() {
        let common = Path::new("/repo/.git");
        let mut responses = FilesystemResponses::default();
        let first = responses.observe_references(&modified(common.join("HEAD")), common, common);
        responses.note_reference_batch();
        let second = responses.observe_references(&modified(common.join("refs/heads/main")), common, common);
        let rescan = notify::Event::new(notify::EventKind::Other).set_flag(Flag::Rescan);
        let third = responses.observe_references(&rescan, common, common);
        responses.note_reference_batch();
        assert_eq!(first, second, "events before the deadline share one response");
        assert_eq!(first, third, "rescans join the pending response");
        let response = responses
            .responses
            .get(&first)
            .expect("the response is retained until acted upon");
        assert_eq!(response.events, 3);
        assert_eq!(response.batches, 2);
        assert_eq!(response.kinds, [("modify", 2), ("other", 1)].into_iter().collect());
        assert_eq!(
            response.triggers,
            [Trigger::Head, Trigger::Refs, Trigger::Rescan].into_iter().collect()
        );

        assert_eq!(responses.references_due(), [first]);
        let next = responses.observe_references(&modified(common.join("HEAD")), common, common);
        assert_ne!(next, first, "activity after the deadline starts another response");

        let mut many = notify::Event::new(notify::EventKind::Modify(ModifyKind::Any));
        for index in 0..MAX_TRIGGER_PATHS + 4 {
            many = many.add_path(common.join(format!("refs/heads/{index}")));
        }
        many = many.add_path(common.join(format!("refs/heads/{}", MAX_TRIGGER_PATHS + 3)));
        responses.observe_references(&many, common, common);
        let response = responses.responses.get(&next).expect("the new response is pending");
        assert_eq!(response.paths.len(), MAX_TRIGGER_PATHS);
        assert_eq!(
            response.omitted_paths, 5,
            "only unique paths beyond the cap are counted"
        );
    }

    #[test]
    fn attributes_overlapping_responses_to_one_presented_frame() {
        let common = Path::new("/repo/.git");
        let workdir = Path::new("/repo");
        let mut responses = FilesystemResponses::default();
        assert!(!responses.has_queued_frame());
        let worktree = responses.observe_worktree(&modified(workdir.join("file")), workdir, &common.join("index"));
        responses.worktree_due(true);
        assert!(responses.has_queued_frame());

        let references = responses.observe_references(&modified(common.join("HEAD")), common, common);
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
