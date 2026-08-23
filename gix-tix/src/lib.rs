//! A fast, interactive commit graph for terminals.

#![forbid(unsafe_code)]

mod app;
mod change_id;
pub mod command;
mod command_menu;
mod edit;
mod enrich;
mod history;
mod logging;
mod menu;
mod ref_tree;
#[cfg(test)]
mod test_repository;
mod ui;

use std::{
    collections::{HashMap, HashSet, VecDeque},
    ffi::OsString,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use app::{
    Action, App, ChangeGroup, ChangeKind, ChangePane, Changes, ChangesMode, ComparedParent, Effect, PathChange,
    SelectionRelation, SharedCommitRow, State,
};
use command_menu::{Command as MenuCommand, CommandId};
use crossterm::{
    clipboard::CopyToClipboard,
    cursor,
    event::{
        self, DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste, EnableFocusChange,
        EnableMouseCapture, Event as TerminalEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
        KeyboardEnhancementFlags, MouseEventKind, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    style::ResetColor,
    terminal::{self, Clear, ClearType},
};
use gix::{
    bstr::{BString, ByteSlice},
    prelude::TreeDiffChangeExt,
};
use history::{Authors, Decorations, Event, HistoryGraph, SelectionRef, SharedAuthors};
use menu::{Item as MenuItem, Menu};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use ratatui::{TerminalOptions, Viewport, backend::CrosstermBackend, layout::Position, text::Line};

const EVENT_BATCH_SIZE: usize = 256;
const OBJECT_CACHE_SIZE: usize = 4 * 1024 * 1024;
const FRAME_INTERVAL: Duration = Duration::from_nanos(16_666_667);
const TODO_PROGRESS_DELAY: Duration = Duration::from_millis(300);
const HISTORY_STATUS_DELAY: Duration = Duration::from_millis(500);
const REPEAT_IDLE: Duration = Duration::from_millis(75);
const REF_EVENT_IDLE: Duration = Duration::from_millis(100);
const IMMEDIATE_PAGER_EXIT: Duration = Duration::from_millis(250);
const REF_EVENT_INTERVAL: Duration = Duration::from_millis(250);
const WATCH_RETRY_INTERVAL: Duration = Duration::from_secs(5);
const LINE_DIFF_POOL_IDLE: Duration = Duration::from_secs(10);
const THEME_QUERY_TIMEOUT: Duration = Duration::from_millis(100);
const WORKTREE_STATUS_CURRENT: usize = 0;
const WORKTREE_STATUS_PARTIAL: usize = usize::MAX - 1;
const WORKTREE_STATUS_FULL: usize = usize::MAX;

struct FillRepository {
    path: PathBuf,
    bare: bool,
    retained: Option<gix::Repository>,
    retain: bool,
}

struct PendingConflictResolution {
    commit: gix::ObjectId,
    head: Option<ConflictHead>,
    ref_changes: Vec<edit::undo::RefChange>,
    record_undo: bool,
}

struct ConflictHead {
    reference: Option<gix::refs::FullName>,
    parents: Vec<gix::ObjectId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WorktreeStatusHead {
    reference: Option<gix::refs::FullName>,
    target: Option<gix::ObjectId>,
}

#[derive(Default)]
struct WorktreeStatusParts {
    staged: bool,
    scopes: HashSet<BString>,
}

enum ExternalConflictResolution {
    Current,
    Changed,
    Complete(gix::ObjectId, Vec<edit::undo::RefChange>, bool),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConflictReconcileStatus {
    Inactive,
    Amend,
    Blocked,
    Complete,
}

struct WorktreeWatcher {
    watcher: RecommendedWatcher,
    events: mpsc::Receiver<notify::Result<notify::Event>>,
    directories: HashSet<PathBuf>,
    index_projection: Vec<IndexWatchEntry>,
    workdir: PathBuf,
    dot_git: PathBuf,
    git_dir: PathBuf,
    index: PathBuf,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct IndexWatchEntry {
    path: BString,
    mode: u32,
    flags: u32,
}

#[derive(Default)]
struct WorktreeWatchRefresh {
    full: bool,
    index: bool,
    scopes: HashSet<PathBuf>,
}

impl WorktreeWatchRefresh {
    fn add_scope(&mut self, scope: &Path, workdir: &Path) {
        if scope == workdir {
            self.full = true;
            self.scopes.clear();
        } else if !self.full && scope.starts_with(workdir) {
            self.scopes.insert(scope.to_owned());
        }
    }

    fn observe(&mut self, event: &notify::Event, workdir: &Path, index: &Path, directories: &HashSet<PathBuf>) {
        if self.full {
            return;
        }
        if event.need_rescan() || event.paths.is_empty() || matches!(event.kind, notify::EventKind::Any) {
            self.full = true;
            self.scopes.clear();
            return;
        }
        for path in &event.paths {
            if path == index {
                self.index = true;
            } else if path.file_name().is_some_and(|name| name == ".gitignore") {
                if let Some(parent) = path.parent() {
                    self.add_scope(parent, workdir);
                } else {
                    self.full = true;
                    self.scopes.clear();
                    return;
                }
            }
        }
        let is_directory = |path: &Path| {
            directories.contains(path)
                || std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_dir())
        };
        match event.kind {
            notify::EventKind::Create(notify::event::CreateKind::Folder)
            | notify::EventKind::Remove(notify::event::RemoveKind::Folder) => {
                for path in &event.paths {
                    self.add_scope(path, workdir);
                }
            }
            notify::EventKind::Modify(notify::event::ModifyKind::Name(_))
                if event.paths.iter().any(|path| is_directory(path)) =>
            {
                for path in &event.paths {
                    if let Some(parent) = path.parent() {
                        self.add_scope(parent, workdir);
                    }
                }
            }
            notify::EventKind::Create(notify::event::CreateKind::Any | notify::event::CreateKind::Other)
            | notify::EventKind::Remove(notify::event::RemoveKind::Any | notify::event::RemoveKind::Other)
            | notify::EventKind::Modify(notify::event::ModifyKind::Any) => {
                for path in event.paths.iter().filter(|path| is_directory(path)) {
                    self.add_scope(path, workdir);
                }
            }
            notify::EventKind::Other => {
                self.full = true;
                self.scopes.clear();
            }
            _ => {}
        }
    }

    fn is_empty(&self) -> bool {
        !self.full && !self.index && self.scopes.is_empty()
    }
}

struct RefWatcher {
    _watcher: RecommendedWatcher,
    events: mpsc::Receiver<notify::Result<notify::Event>>,
    git_dir: PathBuf,
    worktrees_dir: PathBuf,
}

impl WorktreeWatcher {
    fn event_is_relevant(&self, event: &notify::Event) -> bool {
        worktree_event_is_relevant(event, &self.workdir, &self.dot_git, &self.git_dir, &self.index)
    }
}

impl RefWatcher {
    fn event_is_relevant(&self, event: &notify::Event) -> bool {
        reference_event_is_relevant(event, &self.git_dir, &self.worktrees_dir)
    }

    fn watch_set_may_change(&self, event: &notify::Event) -> bool {
        reference_watch_set_may_change(event, &self.worktrees_dir)
    }
}

#[derive(Default)]
struct WorktreeDirectories {
    root: PathBuf,
    paths: HashSet<PathBuf>,
}

impl gix::dir::walk::Delegate for WorktreeDirectories {
    fn emit(
        &mut self,
        _entry: gix::dir::EntryRef<'_>,
        _collapsed_directory_status: Option<gix::dir::entry::Status>,
    ) -> gix::dir::walk::Action {
        std::ops::ControlFlow::Continue(())
    }

    fn can_recurse(
        &mut self,
        entry: gix::dir::EntryRef<'_>,
        for_deletion: Option<gix::dir::walk::ForDeletionMode>,
        worktree_root_is_repository: bool,
    ) -> bool {
        let recurse = entry.status.can_recurse(
            entry.disk_kind,
            entry.pathspec_match,
            for_deletion,
            worktree_root_is_repository,
        );
        if recurse {
            self.paths
                .insert(self.root.join(gix::path::from_bstr(entry.rela_path.as_ref())));
        }
        recurse
    }
}

fn worktree_event_is_relevant(
    event: &notify::Event,
    workdir: &Path,
    dot_git: &Path,
    git_dir: &Path,
    index: &Path,
) -> bool {
    event.need_rescan()
        || (!matches!(event.kind, notify::EventKind::Access(_))
            && (event.paths.is_empty()
                || event.paths.iter().any(|path| {
                    path == index
                        || (path.starts_with(workdir) && !path.starts_with(dot_git) && !path.starts_with(git_dir))
                })))
}

fn worktree_status_event_scopes(
    event: &notify::Event,
    workdir: &Path,
    dot_git: &Path,
    git_dir: &Path,
    index: &Path,
) -> Option<Vec<BString>> {
    if event.need_rescan() || event.paths.is_empty() || event.paths.iter().any(|path| path == index) {
        return None;
    }
    let mut out = Vec::new();
    for path in &event.paths {
        if !path.starts_with(workdir) || path.starts_with(dot_git) || path.starts_with(git_dir) {
            continue;
        }
        if path
            .file_name()
            .is_some_and(|name| name == ".gitattributes" || name == ".gitmodules")
        {
            return None;
        }
        let scope = if path.file_name().is_some_and(|name| name == ".gitignore") {
            path.parent()?
        } else {
            path
        };
        let relative = scope.strip_prefix(workdir).ok()?;
        if relative.as_os_str().is_empty()
            || !relative
                .components()
                .all(|component| matches!(component, std::path::Component::Normal(_)))
        {
            return None;
        }
        let relative = gix::path::try_into_bstr(relative).ok()?;
        out.push(gix::path::to_unix_separators_on_windows(relative).into_owned());
    }
    (!out.is_empty()).then_some(out)
}

fn notification_is_actionable(event: &notify::Event) -> bool {
    event.need_rescan()
        || (!matches!(event.kind, notify::EventKind::Access(_))
            && (event.paths.is_empty()
                || matches!(
                    event.kind,
                    notify::EventKind::Modify(notify::event::ModifyKind::Name(_))
                )
                || event.paths.iter().any(|path| {
                    !path
                        .file_name()
                        .is_some_and(|name| name.as_encoded_bytes().ends_with(b".lock"))
                })))
}

fn reference_event_is_relevant(event: &notify::Event, git_dir: &Path, worktrees_dir: &Path) -> bool {
    let common_dir = worktrees_dir.parent();
    notification_is_actionable(event)
        && (event.need_rescan()
            || event.paths.is_empty()
            || event.paths.iter().any(|path| {
                let is_index = [Some(git_dir), common_dir]
                    .into_iter()
                    .flatten()
                    .any(|git_dir| path == &git_dir.join("index") || path == &git_dir.join("index.lock"));
                if is_index {
                    return false;
                }
                if let Ok(relative) = path.strip_prefix(git_dir)
                    && (relative.components().count() <= 1 || relative.starts_with("refs"))
                {
                    return true;
                }
                let Ok(relative) = path.strip_prefix(worktrees_dir) else {
                    return true;
                };
                let mut components = relative.components();
                let Some(_) = components.next() else { return true };
                match components.next() {
                    None => true,
                    Some(name) => matches!(name.as_os_str().as_encoded_bytes(), b"HEAD" | b"gitdir"),
                }
            }))
}

fn reference_event_changes_status_configuration(event: &notify::Event, git_dir: &Path, worktrees_dir: &Path) -> bool {
    event.need_rescan()
        || event.paths.is_empty()
        || event.paths.iter().any(|path| {
            [Some(git_dir), worktrees_dir.parent()]
                .into_iter()
                .flatten()
                .any(|dir| path == &dir.join("config") || path == &dir.join("config.worktree"))
        })
}

fn reference_watch_set_may_change(event: &notify::Event, worktrees_dir: &Path) -> bool {
    event.need_rescan()
        || event.paths.iter().any(|path| {
            path.strip_prefix(worktrees_dir)
                .is_ok_and(|relative| relative.components().count() <= 1)
        })
}

fn unseen_filesystem_redraw(current: bool, focused: bool, filesystem_frame: bool) -> bool {
    !focused && (current || filesystem_frame)
}

fn worktree_watcher_needed(repository_is_bare: bool, mode: Option<ChangesMode>) -> bool {
    !repository_is_bare && mode == Some(ChangesMode::Both)
}

fn schedule_once(deadline: &mut Option<Instant>, now: Instant, delay: Duration) -> bool {
    if deadline.is_some() {
        false
    } else {
        *deadline = Some(now + delay);
        true
    }
}

fn take_due(deadline: &mut Option<Instant>, now: Instant) -> bool {
    if deadline.is_some_and(|deadline| now >= deadline) {
        *deadline = None;
        true
    } else {
        false
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SelectionRelationCache {
    id: gix::ObjectId,
    refs: Vec<SelectionRef>,
    relation: Option<SelectionRelation>,
}

const TREE_CHANGES_CACHE_SIZE: usize = 8;
type TreeChangesEntry = (app::TreeDiffTarget, Changes);

#[derive(Default)]
struct TreeChangesCache(VecDeque<TreeChangesEntry>);

impl TreeChangesCache {
    fn as_ref(&self) -> Option<&TreeChangesEntry> {
        self.0.front()
    }

    fn activate(&mut self, target: app::TreeDiffTarget) -> bool {
        let Some(position) = self.0.iter().position(|(cached, _)| *cached == target) else {
            return false;
        };
        if position != 0 {
            let entry = self.0.remove(position).expect("the cached position exists");
            self.0.push_front(entry);
        }
        true
    }

    fn insert(&mut self, entry: TreeChangesEntry) {
        self.0.push_front(entry);
        self.0.truncate(TREE_CHANGES_CACHE_SIZE);
    }

    fn clear(&mut self) {
        self.0.clear();
    }
}

type LineCounts = Option<(u32, u32)>;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DiffResource {
    id: gix::ObjectId,
    mode: gix::objs::tree::EntryMode,
    path: BString,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum FileChange {
    Tree(gix::object::tree::diff::ChangeDetached),
    Worktree {
        old: Option<DiffResource>,
        new: Option<DiffResource>,
    },
    Unavailable(&'static str),
}

struct LineDiffJob {
    index: usize,
    change: FileChange,
}

enum LineDiffMessage {
    Job(LineDiffJob),
    FinishBatch,
}

enum LineDiffResult {
    Change(usize, FileChange, Result<LineCounts>),
    BatchFinished,
}

struct LineDiffPool {
    repository_path: PathBuf,
    bare: bool,
    parallelism: usize,
    active: Option<LineDiffWorkers>,
    last_used: Option<Instant>,
}

struct LineDiffWorkers {
    jobs: Vec<mpsc::Sender<LineDiffMessage>>,
    results: mpsc::Receiver<LineDiffResult>,
    workers: Vec<std::thread::JoinHandle<()>>,
}

type LineDiffState = (gix::diff::blob::Platform, Option<gix::diff::blob::Platform>);

fn worktree_diff_cache(
    repository: &gix::Repository,
    mode: gix::diff::blob::pipeline::Mode,
) -> Result<Option<gix::diff::blob::Platform>> {
    let Some(workdir) = repository.workdir() else {
        return Ok(None);
    };
    repository
        .diff_resource_cache(
            mode,
            gix::diff::blob::pipeline::WorktreeRoots {
                old_root: None,
                new_root: Some(workdir.to_owned()),
            },
        )
        .map(Some)
        .context("could not initialize worktree diff resources")
}

fn set_worktree_resources(
    repository: &gix::Repository,
    cache: &mut gix::diff::blob::Platform,
    old: Option<&DiffResource>,
    new: Option<&DiffResource>,
) -> Result<()> {
    let fallback = old.or(new).context("a file diff needs at least one resource")?;
    let old_resource = old.unwrap_or(fallback);
    cache
        .set_resource(
            old.map_or_else(|| repository.object_hash().null(), |resource| resource.id),
            old_resource.mode.kind(),
            old_resource.path.as_bstr(),
            gix::diff::blob::ResourceKind::OldOrSource,
            repository,
        )
        .context("could not prepare old worktree diff resource")?;
    let new_resource = new.unwrap_or(fallback);
    cache
        .set_resource(
            new.map_or_else(|| repository.object_hash().null(), |resource| resource.id),
            new_resource.mode.kind(),
            new_resource.path.as_bstr(),
            gix::diff::blob::ResourceKind::NewOrDestination,
            repository,
        )
        .context("could not prepare new worktree diff resource")?;
    Ok(())
}

fn line_counts_for_change(
    repository: &gix::Repository,
    change: &FileChange,
    tree_cache: &mut gix::diff::blob::Platform,
    worktree_cache: Option<&mut gix::diff::blob::Platform>,
) -> Result<LineCounts> {
    let counts = match change {
        FileChange::Tree(change) => change
            .attach(repository, repository)
            .diff(tree_cache)
            .context("could not prepare line diff")?
            .line_counts()
            .context("could not count changed lines")?,
        FileChange::Worktree { old, new } => {
            let cache = worktree_cache.context("a working tree is required to count changed lines")?;
            set_worktree_resources(repository, cache, old.as_ref(), new.as_ref())?;
            gix::object::blob::diff::Platform { resource_cache: cache }
                .line_counts()
                .context("could not count worktree changed lines")?
        }
        FileChange::Unavailable(_) => None,
    };
    Ok(counts.map(|counts| (counts.insertions, counts.removals)))
}

fn line_diff_state(repository: &gix::Repository) -> Result<LineDiffState> {
    let tree_cache = repository
        .diff_resource_cache_for_tree_diff()
        .context("could not initialize parallel line diffs")?;
    let worktree_cache = worktree_diff_cache(repository, gix::diff::blob::pipeline::Mode::ToGit)?;
    Ok((tree_cache, worktree_cache))
}

impl LineDiffPool {
    fn new(repository_path: &Path, bare: bool, parallelism: usize) -> Self {
        LineDiffPool {
            repository_path: repository_path.to_owned(),
            bare,
            parallelism: parallelism.max(1),
            active: None,
            last_used: None,
        }
    }

    fn line_counts(&mut self, changes: Vec<FileChange>) -> Result<Vec<(FileChange, LineCounts)>> {
        if self.active.is_none() {
            self.active = Some(LineDiffWorkers::new(
                &self.repository_path,
                self.bare,
                self.parallelism,
            )?);
        }
        let result = self
            .active
            .as_mut()
            .expect("line diff workers were just initialized")
            .line_counts(changes);
        self.last_used = Some(Instant::now());
        result
    }

    fn expire(&mut self, now: Instant) -> bool {
        let expired = self
            .last_used
            .is_some_and(|last_used| now.saturating_duration_since(last_used) >= LINE_DIFF_POOL_IDLE);
        if expired {
            self.active = None;
            self.last_used = None;
        }
        expired
    }

    fn idle_timeout(&self, now: Instant) -> Option<Duration> {
        self.last_used
            .map(|last_used| LINE_DIFF_POOL_IDLE.saturating_sub(now.saturating_duration_since(last_used)))
    }
}

impl LineDiffWorkers {
    fn new(repository_path: &Path, bare: bool, parallelism: usize) -> Result<Self> {
        let repository = open_repository(repository_path, bare, false)
            .context("could not open repository for parallel line diffs")?;
        drop(line_diff_state(&repository)?);
        let repository = repository.into_sync();
        let (result_sender, results) = mpsc::channel();
        let mut jobs = Vec::with_capacity(parallelism);
        let workers = (0..parallelism)
            .map(|_| {
                let (job_sender, job_receiver) = mpsc::channel();
                jobs.push(job_sender);
                let result_sender = result_sender.clone();
                let repository = repository.clone();
                std::thread::spawn(move || {
                    let mut repository = repository.to_thread_local();
                    repository.object_cache_size(OBJECT_CACHE_SIZE);
                    let mut state: Option<LineDiffState> = None;
                    while let Ok(message) = job_receiver.recv() {
                        match message {
                            LineDiffMessage::Job(job) => {
                                let result = (|| {
                                    if state.is_none() {
                                        state = Some(line_diff_state(&repository)?);
                                    }
                                    let (tree_cache, worktree_cache) =
                                        state.as_mut().expect("line diff state was just initialized");
                                    let result = line_counts_for_change(
                                        &repository,
                                        &job.change,
                                        tree_cache,
                                        worktree_cache.as_mut(),
                                    );
                                    tree_cache.clear_resource_cache_keep_allocation();
                                    if let Some(cache) = worktree_cache.as_mut() {
                                        cache.clear_resource_cache_keep_allocation();
                                    }
                                    result
                                })();
                                if result_sender
                                    .send(LineDiffResult::Change(job.index, job.change, result))
                                    .is_err()
                                {
                                    break;
                                }
                            }
                            LineDiffMessage::FinishBatch => {
                                state = None;
                                if result_sender.send(LineDiffResult::BatchFinished).is_err() {
                                    break;
                                }
                            }
                        }
                    }
                })
            })
            .collect();
        Ok(LineDiffWorkers { jobs, results, workers })
    }

    fn line_counts(&mut self, changes: Vec<FileChange>) -> Result<Vec<(FileChange, LineCounts)>> {
        let len = changes.len();
        let worker_count = self.jobs.len();
        for (index, change) in changes.into_iter().enumerate() {
            self.jobs[index % worker_count]
                .send(LineDiffMessage::Job(LineDiffJob { index, change }))
                .context("line diff workers stopped unexpectedly")?;
        }
        for jobs in &self.jobs {
            jobs.send(LineDiffMessage::FinishBatch)
                .context("line diff workers stopped unexpectedly")?;
        }

        let mut out: Vec<_> = std::iter::repeat_with(|| None).take(len).collect();
        let mut first_error = None;
        let mut completed = 0;
        let mut finished = 0;
        while completed < len || finished < worker_count {
            match self.results.recv().context("line diff workers stopped unexpectedly")? {
                LineDiffResult::Change(index, change, Ok(lines)) => {
                    *out.get_mut(index).expect("jobs preserve their original result index") = Some((change, lines));
                    completed += 1;
                }
                LineDiffResult::Change(_, _, Err(err)) => {
                    if first_error.is_none() {
                        first_error = Some(err);
                    }
                    completed += 1;
                }
                LineDiffResult::BatchFinished => finished += 1,
            }
        }
        if let Some(err) = first_error {
            return Err(err);
        }
        out.into_iter()
            .map(|entry| entry.context("line diff worker omitted a result"))
            .collect()
    }
}

impl Drop for LineDiffWorkers {
    fn drop(&mut self) {
        self.jobs.clear();
        for worker in self.workers.drain(..) {
            drop(worker.join());
        }
    }
}

fn sync_line_diff_pool(
    pool: &mut Option<LineDiffPool>,
    visible: bool,
    repository_path: &Path,
    bare: bool,
    parallelism: usize,
) {
    if visible && pool.is_none() {
        *pool = Some(LineDiffPool::new(repository_path, bare, parallelism));
    } else if !visible {
        *pool = None;
    }
}

enum FileDiff {
    External(gix::diff::blob::platform::prepare_diff_command::Command),
    Pager { command: Command, diff: BuiltInDiff },
    BuiltIn(BuiltInDiff),
}

enum PreparedFileDiff {
    External(gix::diff::blob::platform::prepare_diff_command::Command, LineCounts),
    BuiltIn(BuiltInDiff, LineCounts),
}

struct CommitDiff {
    external: Vec<gix::diff::blob::platform::prepare_diff_command::Command>,
    internal: FileDiff,
}

pub(crate) struct BuiltInDiff {
    title: BString,
    summary: Option<Vec<Line<'static>>>,
    lines: Vec<BString>,
    max_width: usize,
}

impl BuiltInDiff {
    fn new(title: BString, lines: Vec<BString>) -> Self {
        let max_width = lines
            .iter()
            .map(|line| Line::from(line.to_str_lossy()).width())
            .max()
            .unwrap_or_default();
        BuiltInDiff {
            title,
            summary: None,
            lines,
            max_width,
        }
    }

    fn with_summary(mut self, summary: Vec<Line<'static>>) -> Self {
        self.max_width = self
            .max_width
            .max(summary.iter().map(Line::width).max().unwrap_or_default());
        self.summary = Some(summary);
        self
    }

    fn display_line_count(&self) -> usize {
        self.lines.len() + self.summary.as_ref().map_or(0, |summary| summary.len() + 1)
    }

    fn write_to(&self, mut out: impl Write) -> io::Result<()> {
        if let Some(summary) = &self.summary {
            out.write_all(&self.title)?;
            out.write_all(b"\n")?;
            for line in summary {
                for span in &line.spans {
                    out.write_all(span.content.as_bytes())?;
                }
                out.write_all(b"\n")?;
            }
            out.write_all(b"\n")?;
        }
        for line in &self.lines {
            out.write_all(line)?;
            out.write_all(b"\n")?;
        }
        Ok(())
    }
}

/// Options for [`run()`].
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// Draw interactively on the normal screen instead of the alternate screen.
    pub no_alt_screen: bool,
    /// Exit after the final frame, optionally replaying read-only keyboard input first.
    pub quit_on_finish: Option<String>,
    /// Revisions whose reachable commits should initially be hidden.
    pub hide: Vec<OsString>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RefreshKind {
    History,
    RefTree { enter: bool },
}

fn detect_commit_pane_background() -> Option<(u8, u8, u8)> {
    let mut options = terminal_colorsaurus::QueryOptions::default();
    options.timeout = THEME_QUERY_TIMEOUT;
    match terminal_colorsaurus::background_color(options) {
        Ok(background) => {
            let color = background.scale_to_8bit();
            let shaded = shade_terminal_background(color, background.perceived_lightness() <= 0.5);
            tracing::debug!(?color, ?shaded, "detected terminal background");
            Some(shaded)
        }
        Err(err) => {
            tracing::debug!(error = %err, "terminal background detection unavailable");
            None
        }
    }
}

fn shade_terminal_background((red, green, blue): (u8, u8, u8), dark: bool) -> (u8, u8, u8) {
    let shade = |channel: u8| {
        if dark {
            channel + (u8::MAX - channel) / 16
        } else {
            channel - channel / 16
        }
    };
    (shade(red), shade(green), shade(blue))
}

/// Run the interactive commit graph for `repository`.
pub fn run(repository: gix::ThreadSafeRepository, revisions: Vec<OsString>, mut options: Options) -> Result<()> {
    let _log_guard = logging::init();
    let mut repository_path = repository.git_dir().to_owned();
    let common_dir = normalize_common_dir(repository.common_dir.clone().unwrap_or_else(|| repository_path.clone()))?;
    let (hide, unavailable) = validate_hidden_revisions(&mut repository_path, &common_dir, &options.hide)?;
    options.hide = hide;
    for (revision, err) in unavailable {
        eprintln!(
            "warning: ignoring unavailable hidden revision {}: {err}",
            revision.to_string_lossy()
        );
    }
    tracing::info!(
        revision_count = revisions.len(),
        hidden_revision_count = options.hide.len(),
        "starting tix"
    );
    let commit_pane_background = detect_commit_pane_background();
    let quit_on_finish = options.quit_on_finish.is_some();
    let inline = quit_on_finish || options.no_alt_screen;
    let terminal_result = if inline {
        let (_, height) = terminal::size().context("could not determine terminal size")?;
        ratatui::try_init_with_options(TerminalOptions {
            viewport: Viewport::Inline(height),
        })
    } else {
        ratatui::try_init()
    };
    let mut terminal = terminal_result
        .inspect_err(|_| {
            if inline {
                let _ = terminal::disable_raw_mode();
            } else {
                ratatui::restore();
            }
        })
        .context("could not initialize terminal")?;
    let enhanced_keyboard = !quit_on_finish && terminal::supports_keyboard_enhancement().unwrap_or(false);
    let keyboard_setup = if quit_on_finish {
        Ok(())
    } else {
        enable_input(terminal.backend_mut(), enhanced_keyboard)
    };
    let result = keyboard_setup
        .context("could not enable enhanced keyboard events")
        .and_then(|()| {
            if !quit_on_finish {
                let hook = std::panic::take_hook();
                std::panic::set_hook(Box::new(move |info| {
                    let mut backend = CrosstermBackend::new(std::io::stdout());
                    let _ = disable_input(&mut backend, enhanced_keyboard);
                    let _ = execute!(backend, cursor::Show);
                    hook(info);
                }));
            }
            event_loop(
                &mut terminal,
                repository,
                revisions,
                options,
                enhanced_keyboard,
                commit_pane_background,
            )
        });
    let keyboard_restore = if quit_on_finish {
        Ok(())
    } else {
        disable_input(terminal.backend_mut(), enhanced_keyboard)
    };
    let cursor_restore = if inline {
        let area = terminal.get_frame().area();
        terminal
            .set_cursor_position(Position::new(area.x, area.bottom().saturating_sub(1)))
            .and_then(|()| terminal.show_cursor())
    } else {
        Ok(())
    };
    drop(terminal);
    let restore = if inline {
        terminal::disable_raw_mode()
    } else {
        ratatui::try_restore()
    }
    .context("could not restore terminal");
    if inline {
        eprintln!();
    }
    let lane_time = result?;
    keyboard_restore.context("could not restore keyboard events")?;
    cursor_restore.context("could not restore terminal cursor")?;
    restore?;
    if let Some(lane_time) = lane_time {
        eprintln!("lane computation: {:.3}s", lane_time.as_secs_f64());
    }
    Ok(())
}

#[expect(clippy::type_complexity, reason = "forward the hidden-revision result unchanged")]
fn validate_hidden_revisions(
    repository_path: &mut PathBuf,
    common_dir: &Path,
    hide: &[OsString],
) -> Result<(Vec<OsString>, Vec<(OsString, String)>)> {
    let (repository, _) = open_history_repository(repository_path, common_dir)?;
    history::available_hidden_revisions(&repository, hide, false)
}

fn enable_input(backend: &mut CrosstermBackend<std::io::Stdout>, enhanced_keyboard: bool) -> std::io::Result<()> {
    execute!(backend, EnableFocusChange, EnableMouseCapture, EnableBracketedPaste)?;
    if enhanced_keyboard {
        execute!(backend, PushKeyboardEnhancementFlags(keyboard_enhancement_flags()))?;
    }
    Ok(())
}

fn keyboard_enhancement_flags() -> KeyboardEnhancementFlags {
    KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
        | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
}

fn disable_input(backend: &mut CrosstermBackend<std::io::Stdout>, enhanced_keyboard: bool) -> std::io::Result<()> {
    if enhanced_keyboard {
        execute!(backend, PopKeyboardEnhancementFlags)?;
    }
    execute!(backend, DisableBracketedPaste, DisableMouseCapture, DisableFocusChange)
}

fn is_key_press(event: &TerminalEvent) -> bool {
    matches!(event, TerminalEvent::Key(key) if key.kind != KeyEventKind::Release)
}

fn event_loop(
    terminal: &mut ratatui::DefaultTerminal,
    mut repository: gix::ThreadSafeRepository,
    revisions: Vec<OsString>,
    options: Options,
    enhanced_keyboard: bool,
    commit_pane_background: Option<(u8, u8, u8)>,
) -> Result<Option<Duration>> {
    let Options {
        quit_on_finish, hide, ..
    } = options;
    let mut quit_inputs: VecDeque<_> = quit_on_finish
        .as_deref()
        .unwrap_or_default()
        .chars()
        .map(diagnostic_key)
        .collect();
    let quit_on_finish = quit_on_finish.is_some();
    let mut repository_path = repository.git_dir().to_owned();
    let common_dir = normalize_common_dir(repository.common_dir.clone().unwrap_or_else(|| repository_path.clone()))?;
    let (mut view_repository, recovered_at_startup) = open_history_repository(&mut repository_path, &common_dir)?;
    view_repository.object_cache_size(None);
    let (mut repository_is_bare, mut mailmap, mut ref_snapshot, mut worktree_head_unborn) = {
        let bare = view_repository.workdir().is_none();
        let mailmap = view_repository.open_mailmap();
        let refs = history::snapshot(&view_repository, &revisions, &hide, false)?;
        let unborn = !bare && view_repository.head()?.is_unborn();
        (bare, mailmap, refs, unborn)
    };
    if recovered_at_startup {
        repository = view_repository.into_sync();
        repository_is_bare = true;
    } else {
        drop(view_repository);
    }
    let authors = gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(Authors::default()));
    let mut watcher_retry_deadline = None;
    let mut ref_watcher = match start_ref_watcher(&repository_path, &common_dir) {
        Ok(watcher) => Some(watcher),
        Err(err) => {
            tracing::warn!(error = %err, "reference watcher startup failed");
            schedule_once(&mut watcher_retry_deadline, Instant::now(), WATCH_RETRY_INTERVAL);
            None
        }
    };
    let mut ref_watch_set_changed = false;
    let mut ref_status_config_changed = false;
    let (cancelled, receiver) = start_history(
        repository,
        &revisions,
        &hide,
        false,
        gix::features::threading::OwnShared::clone(&authors),
    );

    let mut app = App::new(1);
    app.set_view_tips(&ref_snapshot.view_tips);
    app.set_worktree_head_unborn(worktree_head_unborn);
    app.set_worktree_branch(current_worktree_branch(&ref_snapshot));
    app.commit_pane_background = commit_pane_background;
    if recovered_at_startup {
        app.leave_attention("worktree removed; using the common repository without worktree changes");
    }
    let mut lane_receiver: Option<mpsc::Receiver<(Vec<SharedCommitRow>, app::Graph, Duration)>> = None;
    let mut refresh_receiver: Option<mpsc::Receiver<(RefreshKind, HistoryGraph, Result<history::Refresh>)>> = None;
    let mut refresh_pending = false;
    let mut ref_tree_refresh_pending = false;
    let mut return_to_history_after_refresh = None;
    let mut ref_refresh_deadline: Option<Instant> = None;
    let mut refresh_expand_hidden = false;
    let mut verification_receiver = None;
    let mut commit_message = None;
    let mut tree_changes = TreeChangesCache::default();
    let mut worktree_changes = None;
    let mut cached_status_head = None;
    let mut worktree_status_parts = WorktreeStatusParts::default();
    let mut worktree_watcher: Option<WorktreeWatcher> = None;
    let mut worktree_refresh_deadline: Option<Instant> = None;
    let mut worktree_watch_refresh = WorktreeWatchRefresh::default();
    let mut queued_worktree_status_full = false;
    let mut queued_worktree_status_scopes = HashSet::new();
    let mut selection_relation = None;
    let mut history_graph = None;
    let line_diff_parallelism = std::thread::available_parallelism().map_or(1, Into::into);
    let mut line_diff_pool = None;
    let mut fill_repository = FillRepository {
        path: repository_path.clone(),
        bare: repository_is_bare,
        retained: None,
        retain: false,
    };
    app.set_worktree_changes_available(!repository_is_bare);
    app.configure_hidden_filter(!hide.is_empty());
    sync_line_diff_pool(
        &mut line_diff_pool,
        app.changes_mode.is_some(),
        &repository_path,
        repository_is_bare,
        line_diff_parallelism,
    );
    if worktree_watcher_needed(repository_is_bare, app.changes_mode) {
        match start_worktree_watcher(&repository_path, repository_is_bare) {
            Ok(watcher) => worktree_watcher = Some(watcher),
            Err(err) => {
                tracing::warn!(error = %err, "worktree watcher startup failed");
                app.worktree_changes.error = Some(format!("worktree watch: {err}"));
                schedule_once(&mut watcher_retry_deadline, Instant::now(), WATCH_RETRY_INTERVAL);
            }
        }
    }
    let mut decorations = Decorations::new();
    let mut ref_tree = ref_tree::Tree::default();
    let mut command_picker = Menu::default();
    let mut command_picker_key = None;
    let mut filesystem_responses = logging::FilesystemResponses::default();
    let mut focused = true;
    draw(
        terminal,
        &mut app,
        &mut command_picker,
        &decorations,
        &mailmap,
        &authors,
        &mut fill_repository,
        &mut commit_message,
        &mut tree_changes,
        &mut worktree_changes,
        &mut cached_status_head,
        &mut worktree_status_parts,
        &mut history_graph,
        &mut selection_relation,
        &mut line_diff_pool,
        focused,
        &mut ref_tree,
        &mut filesystem_responses,
    )?;
    let mut last_draw = Instant::now();
    let mut dirty = false;
    let mut urgent = false;
    let mut history_finished = false;
    let mut repeat_deadline: Option<Instant> = None;
    let mut history_status_deadline: Option<Instant> = None;
    let mut pending_terminal_event = None;
    let mut pending_rebase_conflict: Option<edit::time_travel::Conflict> = None;
    let mut pending_conflict_clear_undo_on_accept = false;
    let mut pending_todo_rebase_conflict: Option<edit::rebase::PlanConflict> = None;
    let mut pending_todo_rebase_plan: Option<edit::rebase::Plan> = None;
    let mut pending_todo_ref_changes = Vec::new();
    let mut pending_conflict_resolution: Option<PendingConflictResolution> = None;
    let result: Result<Option<Duration>> = (|| loop {
        if let Some(pool) = line_diff_pool.as_mut() {
            pool.expire(Instant::now());
        }
        if let Some(mut recovered) =
            recover_event_loop_repository(&mut repository_path, &common_dir, &mut repository_is_bare)?
        {
            recovered.object_cache_size(None);
            mailmap = recovered.open_mailmap();
            fill_repository.path.clone_from(&repository_path);
            fill_repository.bare = true;
            fill_repository.retain = false;
            fill_repository.retained = None;
            app.set_worktree_changes_available(false);
            app.set_worktree_branch(None);
            worktree_watcher = None;
            worktree_refresh_deadline = None;
            worktree_watch_refresh = WorktreeWatchRefresh::default();
            queued_worktree_status_full = false;
            queued_worktree_status_scopes.clear();
            filesystem_responses.cancel_pending_worktree("worktree-unavailable");
            worktree_changes = None;
            cached_status_head = None;
            worktree_status_parts = WorktreeStatusParts::default();
            line_diff_pool = None;
            sync_line_diff_pool(
                &mut line_diff_pool,
                app.changes_mode.is_some(),
                &repository_path,
                true,
                line_diff_parallelism,
            );
            tracing::warn!(common_dir = %repository_path.display(), "worktree disappeared; recovered with common repository");
            ref_watcher = match start_ref_watcher(&repository_path, &repository_path) {
                Ok(watcher) => Some(watcher),
                Err(err) => {
                    tracing::warn!(error = %err, "reference watcher recovery failed");
                    schedule_once(&mut watcher_retry_deadline, Instant::now(), WATCH_RETRY_INTERVAL);
                    None
                }
            };
            ref_watch_set_changed = false;
            ref_status_config_changed = false;
            app.leave_attention("worktree removed; using the common repository without worktree changes");
            if history_graph.is_some() {
                refresh_pending = true;
            }
            dirty = true;
            urgent = true;
        }
        let mut conflict_refresh_due = false;
        let mut worktree_watch_error = None;
        let mut worktree_events_drained = true;
        if let Some(watcher) = worktree_watcher.as_mut() {
            let mut received = 0;
            let mut relevant = 0;
            let mut rescans = 0;
            while received < EVENT_BATCH_SIZE {
                match watcher.events.try_recv() {
                    Ok(Ok(event)) => {
                        received += 1;
                        rescans += usize::from(event.need_rescan());
                        if watcher.event_is_relevant(&event) {
                            relevant += 1;
                            worktree_watch_refresh.observe(
                                &event,
                                &watcher.workdir,
                                &watcher.index,
                                &watcher.directories,
                            );
                            filesystem_responses.observe_worktree(&event, &watcher.workdir, &watcher.index);
                            if !queued_worktree_status_full {
                                match worktree_status_event_scopes(
                                    &event,
                                    &watcher.workdir,
                                    &watcher.dot_git,
                                    &watcher.git_dir,
                                    &watcher.index,
                                ) {
                                    Some(scopes) => queued_worktree_status_scopes.extend(scopes),
                                    None => {
                                        queued_worktree_status_full = true;
                                        queued_worktree_status_scopes.clear();
                                    }
                                }
                            }
                            schedule_once(&mut worktree_refresh_deadline, Instant::now(), Duration::ZERO);
                        }
                    }
                    Ok(Err(err)) => {
                        worktree_watch_error = Some(err);
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty | mpsc::TryRecvError::Disconnected) => break,
                }
            }
            if received > 0 {
                if relevant > 0 {
                    filesystem_responses.note_worktree_batch();
                }
                tracing::debug!(received, relevant, rescans, "processed worktree event batch");
            }
            worktree_events_drained = received < EVENT_BATCH_SIZE;
        }
        if let Some(err) = worktree_watch_error {
            tracing::warn!(error = %err, "worktree watcher failed");
            filesystem_responses.fail_pending_worktree();
            app.worktree_changes.error = Some(format!("worktree watch: {err}"));
            worktree_watcher = None;
            worktree_refresh_deadline = None;
            worktree_watch_refresh = WorktreeWatchRefresh::default();
            queued_worktree_status_full = false;
            queued_worktree_status_scopes.clear();
            schedule_once(&mut watcher_retry_deadline, Instant::now(), WATCH_RETRY_INTERVAL);
            dirty = true;
            urgent = true;
        }
        if worktree_events_drained && take_due(&mut worktree_refresh_deadline, Instant::now()) {
            conflict_refresh_due = true;
            let watch_refresh = std::mem::take(&mut worktree_watch_refresh);
            let watch_result = worktree_watcher
                .as_mut()
                .filter(|_| !watch_refresh.is_empty())
                .map(|watcher| reconcile_worktree_watcher(watcher, &repository_path, repository_is_bare, watch_refresh))
                .transpose();
            if let Err(err) = watch_result {
                tracing::warn!(error = %err, "worktree watcher update failed");
                queued_worktree_status_full = true;
                queued_worktree_status_scopes.clear();
                match start_worktree_watcher(&repository_path, repository_is_bare) {
                    Ok(watcher) => worktree_watcher = Some(watcher),
                    Err(err) => {
                        tracing::warn!(error = %err, "worktree watcher rebuild failed");
                        app.worktree_changes.error = Some(format!("worktree watch: {err}"));
                        worktree_watcher = None;
                        schedule_once(&mut watcher_retry_deadline, Instant::now(), WATCH_RETRY_INTERVAL);
                    }
                }
            }
            let invalidated = if std::mem::take(&mut queued_worktree_status_full) {
                queued_worktree_status_scopes.clear();
                worktree_status_parts = WorktreeStatusParts::default();
                invalidate_worktree_changes(&mut worktree_changes)
            } else {
                invalidate_worktree_status_parts(
                    &mut worktree_changes,
                    &mut worktree_status_parts,
                    false,
                    queued_worktree_status_scopes.drain(),
                )
            };
            filesystem_responses.worktree_due(invalidated);
            tracing::debug!(invalidated, "worktree event deadline elapsed");
            dirty = true;
            urgent = true;
        }
        let mut ref_watch_error = None;
        if let Some(watcher) = ref_watcher.as_mut() {
            let mut received = 0;
            let mut actionable = 0;
            let mut rescans = 0;
            while received < EVENT_BATCH_SIZE {
                match watcher.events.try_recv() {
                    Ok(Ok(event)) => {
                        received += 1;
                        rescans += usize::from(event.need_rescan());
                        if watcher.event_is_relevant(&event) {
                            actionable += 1;
                            ref_watch_set_changed |= watcher.watch_set_may_change(&event);
                            ref_status_config_changed |= reference_event_changes_status_configuration(
                                &event,
                                &watcher.git_dir,
                                &watcher.worktrees_dir,
                            );
                            filesystem_responses.observe_references(&event, &repository_path, &common_dir);
                            ref_refresh_deadline = Some(Instant::now() + REF_EVENT_IDLE);
                        }
                    }
                    Ok(Err(err)) => {
                        ref_watch_error = Some(err);
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty | mpsc::TryRecvError::Disconnected) => break,
                }
            }
            if received > 0 {
                if actionable > 0 {
                    filesystem_responses.note_reference_batch();
                }
                tracing::debug!(received, actionable, rescans, "processed reference event batch");
            }
        }
        if let Some(err) = ref_watch_error {
            tracing::warn!(error = %err, "reference watcher failed");
            filesystem_responses.fail_pending_references();
            ref_watcher = None;
            ref_refresh_deadline = None;
            ref_watch_set_changed = false;
            ref_status_config_changed = false;
            schedule_once(&mut watcher_retry_deadline, Instant::now(), WATCH_RETRY_INTERVAL);
        }
        if take_due(&mut ref_refresh_deadline, Instant::now()) {
            conflict_refresh_due = true;
            app.clear_enrichments();
            if std::mem::take(&mut ref_watch_set_changed) {
                match start_ref_watcher(&repository_path, &common_dir) {
                    Ok(watcher) => {
                        ref_watcher = Some(watcher);
                    }
                    Err(err) => {
                        tracing::warn!(error = %err, "reference watcher rebuild failed");
                        ref_watcher = None;
                        schedule_once(&mut watcher_retry_deadline, Instant::now(), WATCH_RETRY_INTERVAL);
                    }
                }
            }
            let response_ids = filesystem_responses.references_due();
            refresh_pending = true;
            let status_config_changed = std::mem::take(&mut ref_status_config_changed);
            if status_config_changed {
                fill_repository.retain = false;
                fill_repository.retained = None;
            }
            let head_changed = status_config_changed
                || worktree_changes
                    .as_ref()
                    .is_some_and(|(marker, _)| *marker != WORKTREE_STATUS_FULL)
                    && match cached_status_head.as_ref() {
                        Some(previous) => open_repository(&repository_path, repository_is_bare, false)
                            .context("could not reopen repository to compare HEAD after a reference change")
                            .and_then(|repository| worktree_status_head(&repository))
                            .map_or_else(
                                |err| {
                                    tracing::warn!(error = %err, "could not compare HEAD after a reference change");
                                    true
                                },
                                |current| current != *previous,
                            ),
                        None => true,
                    };
            let invalidated = if status_config_changed {
                worktree_status_parts = WorktreeStatusParts::default();
                invalidate_worktree_changes(&mut worktree_changes)
            } else {
                head_changed
                    && invalidate_worktree_status_parts(
                        &mut worktree_changes,
                        &mut worktree_status_parts,
                        true,
                        std::iter::empty(),
                    )
            };
            filesystem_responses.phase(&response_ids, "reference-worktree-cache-invalidation");
            if invalidated {
                filesystem_responses.queue_frame(&response_ids, "reference-worktree-cache-invalidation");
                dirty = true;
                urgent = true;
            }
        }
        if conflict_refresh_due
            && reconcile_external_conflict_reporting(
                &mut app,
                &repository_path,
                repository_is_bare,
                &mut pending_conflict_resolution,
            ) == ConflictReconcileStatus::Complete
        {
            invalidate_worktree_changes(&mut worktree_changes);
            refresh_pending = true;
            dirty = true;
            urgent = true;
        }
        if take_due(&mut watcher_retry_deadline, Instant::now()) {
            let mut retry = false;
            if ref_watcher.is_none() {
                match start_ref_watcher(&repository_path, &common_dir) {
                    Ok(watcher) => {
                        tracing::info!("reference watcher recovered");
                        ref_watcher = Some(watcher);
                    }
                    Err(err) => {
                        tracing::warn!(error = %err, "reference watcher retry failed");
                        retry = true;
                    }
                }
            }
            if worktree_watcher_needed(repository_is_bare, app.changes_mode) && worktree_watcher.is_none() {
                match start_worktree_watcher(&repository_path, repository_is_bare) {
                    Ok(watcher) => {
                        tracing::info!("worktree watcher recovered");
                        worktree_watcher = Some(watcher);
                        if app
                            .worktree_changes
                            .error
                            .as_deref()
                            .is_some_and(|message| message.starts_with("worktree watch:"))
                        {
                            app.worktree_changes.error = None;
                        }
                        invalidate_worktree_changes(&mut worktree_changes);
                        dirty = true;
                        urgent = true;
                    }
                    Err(err) => {
                        tracing::warn!(error = %err, "worktree watcher retry failed");
                        app.worktree_changes.error = Some(format!("worktree watch: {err}"));
                        retry = true;
                    }
                }
            }
            if retry {
                schedule_once(&mut watcher_retry_deadline, Instant::now(), WATCH_RETRY_INTERVAL);
            }
        }
        if take_due(&mut history_status_deadline, Instant::now()) {
            app.deferred_history_state = None;
            let response_ids = filesystem_responses.active_reference_ids().to_vec();
            filesystem_responses.queue_frame(&response_ids, "delayed-history-status");
            dirty = true;
            urgent = true;
        }
        if repeat_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            repeat_deadline = None;
            if app.changes_suppressed {
                app.changes_suppressed = false;
                dirty = true;
                urgent = true;
            } else {
                fill_repository.retain = false;
                fill_repository.retained = None;
            }
        }
        if let Some(result) = verification_receiver.as_ref().map(mpsc::Receiver::try_recv) {
            match result {
                Ok(results) => {
                    app.finish_signature_verification(results);
                    verification_receiver = None;
                    dirty = true;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    anyhow::bail!("signature verification worker stopped unexpectedly")
                }
            }
        }
        if let Some(result) = lane_receiver.as_ref().map(mpsc::Receiver::try_recv) {
            match result {
                Ok((rows, graph, lane_time)) => {
                    let scan =
                        scan_change_ids(&repository_path, repository_is_bare, change_id_scan_needed(&app), &rows)
                            .unwrap_or_else(|err| {
                                tracing::warn!(error = %err, "change ID scan failed");
                                change_id::Scan::default()
                            });
                    app.finish_lane_computation(rows, graph, lane_time);
                    app.set_change_ids(scan.overrides, scan.duplicates);
                    ref_tree.set_history_commits(app.rows.iter().map(|row| row.id));
                    if return_to_history_after_refresh.take().is_some() {
                        ref_tree.leave();
                    }
                    update_hidden_branch_updates(&mut app, history_graph.as_ref(), &ref_snapshot);
                    let response_ids = filesystem_responses.active_reference_ids().to_vec();
                    filesystem_responses.phase(&response_ids, "lane-computation-completed");
                    filesystem_responses.queue_frame(&response_ids, "lane-computation-completed");
                    filesystem_responses.finish_after_frame(&response_ids, "completed");
                    history_status_deadline = None;
                    app.deferred_history_state = None;
                    selection_relation = None;
                    app.selection_relation = None;
                    lane_receiver = None;
                    dirty = true;
                    if quit_on_finish {
                        urgent = true;
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    anyhow::bail!("lane worker stopped unexpectedly")
                }
            }
        }
        if let Some(result) = refresh_receiver.as_ref().map(mpsc::Receiver::try_recv) {
            match result {
                Ok((kind, mut graph, result)) => {
                    let result = result?;
                    if matches!(kind, RefreshKind::RefTree { .. }) {
                        graph.set_current_view(&ref_snapshot.view_tips);
                    }
                    app.set_known_descendants(graph.commits_with_descendants());
                    app.set_known_merge_descendants(graph.commits_with_merge_descendants());
                    app.set_worktree_branch(
                        (!repository_is_bare)
                            .then(|| current_worktree_branch(&result.refs))
                            .flatten(),
                    );
                    ref_tree.rebuild(&graph, &result.refs, &result.decorations);
                    history_graph = Some(graph);
                    tracing::info!(commit_count = result.commits.rows.len(), "history refresh completed");
                    if let RefreshKind::RefTree { enter } = kind {
                        let hidden_tips = if app.show_hidden {
                            &[][..]
                        } else {
                            ref_snapshot.hidden_tips.as_slice()
                        };
                        if let Some(rows) =
                            app.start_refresh(result.commits, &ref_snapshot.view_tips, hidden_tips, false)
                        {
                            lane_receiver = Some(start_lane_worker(rows));
                        }
                        if enter {
                            command_picker.close();
                            ref_tree.toggle();
                            app.history_display_expanded = false;
                        }
                        refresh_receiver = None;
                        dirty = true;
                        continue;
                    }
                    let response_ids = filesystem_responses.active_reference_ids().to_vec();
                    filesystem_responses.phase(&response_ids, "history-refresh-completed");
                    filesystem_responses.queue_frame(&response_ids, "history-refresh-completed");
                    let decorated_successor = app
                        .selected
                        .and_then(|index| app.rows.get(index))
                        .and_then(|row| decoration_successor(row.id, &decorations, &result.decorations));
                    app.set_worktree_head(
                        (!repository_is_bare)
                            .then(|| decoration_head(&result.decorations))
                            .flatten(),
                        false,
                    );
                    if let Some(successor) = decorated_successor {
                        app.select_commit_after_refresh(successor);
                    }
                    if let Some(id) = return_to_history_after_refresh {
                        app.select_commit_after_refresh(id);
                    }
                    worktree_head_unborn = !repository_is_bare
                        && open_repository(&repository_path, false, false)
                            .and_then(|repo| Ok(repo.head()?.is_unborn()))
                            .unwrap_or(false);
                    app.set_worktree_head_unborn(worktree_head_unborn);
                    decorations = result.decorations;
                    selection_relation = None;
                    app.selection_relation = None;
                    let hidden_tips = if app.show_hidden {
                        &[][..]
                    } else {
                        result.refs.hidden_tips.as_slice()
                    };
                    if let Some(rows) = app.start_refresh(result.commits, &result.refs.view_tips, hidden_tips, false) {
                        lane_receiver = Some(start_lane_worker(rows));
                    }
                    refresh_receiver = None;
                    dirty = true;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => anyhow::bail!("history refresh worker stopped unexpectedly"),
            }
        }
        if ref_tree_refresh_pending
            && refresh_receiver.is_none()
            && lane_receiver.is_none()
            && history_graph.is_some()
            && matches!(app.state, State::Complete | State::Cancelled)
        {
            ref_tree_refresh_pending = false;
            refresh_receiver = Some(start_history_refresh(
                repository_path.clone(),
                repository_is_bare,
                revisions.clone(),
                if app.show_hidden { Vec::new() } else { hide.clone() },
                true,
                Default::default(),
                gix::features::threading::OwnShared::clone(&authors),
                history_graph
                    .take()
                    .expect("ref-tree refresh starts only with a cached history graph"),
                RefreshKind::RefTree { enter: true },
            ));
            app.deferred_history_state = Some(app.state);
            app.state = State::Loading;
        }
        if refresh_pending
            && refresh_receiver.is_none()
            && lane_receiver.is_none()
            && history_graph.is_some()
            && matches!(app.state, State::Complete | State::Cancelled)
        {
            let refresh_started = Instant::now();
            let response_ids = filesystem_responses.begin_reference_refresh();
            let repository = match open_repository(&repository_path, repository_is_bare, true) {
                Ok(repository) => repository,
                Err(_err) if worktree_repository_is_gone(&repository_path) => continue,
                Err(err) => return Err(err).context("could not inspect changed references"),
            };
            let next = history::snapshot(&repository, &revisions, &hide, false)?;
            let hidden_changed = next.hidden != ref_snapshot.hidden;
            let worktree_tips_changed = ref_tree.is_active() && next.worktrees != ref_snapshot.worktrees;
            let tips_changed = next.view != ref_snapshot.view || hidden_changed || worktree_tips_changed;
            tracing::debug!(
                ?response_ids,
                tips_changed,
                hidden_changed,
                "compared reference snapshot"
            );
            ref_snapshot = next;
            app.set_worktree_branch(current_worktree_branch(&ref_snapshot));
            refresh_pending = false;
            let hidden = if app.show_hidden { Vec::new() } else { hide.clone() };
            let expand = if refresh_expand_hidden || hidden_changed {
                app.hidden_ids()
            } else {
                Default::default()
            };
            refresh_receiver = Some(start_history_refresh(
                repository_path.clone(),
                repository_is_bare,
                revisions.clone(),
                hidden,
                false,
                expand,
                gix::features::threading::OwnShared::clone(&authors),
                history_graph
                    .take()
                    .expect("refresh starts only with a cached history graph"),
                RefreshKind::History,
            ));
            refresh_expand_hidden = false;
            app.deferred_history_state = Some(app.state);
            history_status_deadline = Some(refresh_started + HISTORY_STATUS_DELAY);
            app.state = State::Loading;
            filesystem_responses.phase(&response_ids, "history-refresh-started");
            tracing::info!(?response_ids, "started history refresh");
        }
        if urgent {
            draw(
                terminal,
                &mut app,
                &mut command_picker,
                &decorations,
                &mailmap,
                &authors,
                &mut fill_repository,
                &mut commit_message,
                &mut tree_changes,
                &mut worktree_changes,
                &mut cached_status_head,
                &mut worktree_status_parts,
                &mut history_graph,
                &mut selection_relation,
                &mut line_diff_pool,
                focused,
                &mut ref_tree,
                &mut filesystem_responses,
            )?;
            last_draw = Instant::now();
            dirty = false;
            urgent = false;
            if repeat_deadline.is_none() {
                fill_repository.retain = false;
                fill_repository.retained = None;
            }
            if quit_on_finish
                && quit_inputs.is_empty()
                && matches!(app.state, State::Complete)
                && lane_receiver.is_none()
            {
                return Ok(app.lane_time);
            }
            continue;
        }
        let mut events = 0;
        while !history_finished && events < EVENT_BATCH_SIZE {
            let message = match receiver.try_recv() {
                Ok(message) => message,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    anyhow::bail!("history worker stopped unexpectedly")
                }
            };
            events += 1;
            dirty = true;
            match message? {
                Event::Decorations(value) => {
                    app.set_worktree_head((!repository_is_bare).then(|| decoration_head(&value)).flatten(), true);
                    decorations = value;
                }
                Event::Commits(rows) => app.extend_commits(rows),
                Event::HiddenCommits(rows) => app.extend_hidden_commits(rows),
                Event::VisibleComplete => {
                    if let Some(rows) = app.start_lane_computation() {
                        lane_receiver = Some(start_lane_worker(rows));
                    }
                }
                Event::Complete(graph) => {
                    history_finished = true;
                    app.set_known_descendants(graph.commits_with_descendants());
                    app.set_known_merge_descendants(graph.commits_with_merge_descendants());
                    ref_tree.rebuild(&graph, &ref_snapshot, &decorations);
                    history_graph = Some(graph);
                    update_hidden_branch_updates(&mut app, history_graph.as_ref(), &ref_snapshot);
                    selection_relation = None;
                    app.selection_relation = None;
                }
                Event::Cancelled => {
                    history_finished = true;
                    drop(app.update(Action::Cancelled));
                }
            }
        }
        let streaming = matches!(app.state, State::Loading | State::Cancelling | State::Computing)
            || verification_receiver.is_some()
            || repeat_deadline.is_some();
        if should_draw(dirty, streaming, last_draw.elapsed()) {
            draw(
                terminal,
                &mut app,
                &mut command_picker,
                &decorations,
                &mailmap,
                &authors,
                &mut fill_repository,
                &mut commit_message,
                &mut tree_changes,
                &mut worktree_changes,
                &mut cached_status_head,
                &mut worktree_status_parts,
                &mut history_graph,
                &mut selection_relation,
                &mut line_diff_pool,
                focused,
                &mut ref_tree,
                &mut filesystem_responses,
            )?;
            last_draw = Instant::now();
            dirty = false;
        }
        let repeat_timeout = repeat_deadline.map(|deadline| deadline.saturating_duration_since(Instant::now()));
        let watcher_timeout = ref_watcher.as_ref().map(|_| REF_EVENT_INTERVAL);
        let ref_refresh_timeout =
            ref_refresh_deadline.map(|deadline| deadline.saturating_duration_since(Instant::now()));
        let worktree_timeout = worktree_refresh_deadline
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            .or_else(|| worktree_watcher.as_ref().map(|_| REF_EVENT_INTERVAL));
        let retry_timeout = watcher_retry_deadline.map(|deadline| deadline.saturating_duration_since(Instant::now()));
        let history_status_timeout =
            history_status_deadline.map(|deadline| deadline.saturating_duration_since(Instant::now()));
        let line_diff_timeout = line_diff_pool
            .as_ref()
            .and_then(|pool| pool.idle_timeout(Instant::now()));
        let wake_after = [
            repeat_timeout,
            watcher_timeout,
            ref_refresh_timeout,
            worktree_timeout,
            retry_timeout,
            history_status_timeout,
            line_diff_timeout,
        ]
        .into_iter()
        .flatten()
        .min();
        let (terminal_event, diagnostic_input) = match pending_terminal_event.take() {
            Some(event) => (Some(event), false),
            None => match next_diagnostic_input(&mut quit_inputs, app.state, lane_receiver.is_some()) {
                Some(key) => (Some(TerminalEvent::Key(key)), true),
                None => (
                    match poll_timeout(streaming, events, dirty, last_draw.elapsed(), wake_after) {
                        Some(timeout) if event::poll(timeout)? => Some(event::read()?),
                        Some(_) => None,
                        None => Some(event::read()?),
                    },
                    false,
                ),
            },
        };
        let Some(terminal_event) = terminal_event else {
            continue;
        };
        if swallow_command_menu_key_event(&terminal_event, &mut command_picker_key) {
            continue;
        }
        if ref_tree.is_active() {
            if matches!(
                &terminal_event,
                TerminalEvent::Key(KeyEvent {
                    kind: KeyEventKind::Press | KeyEventKind::Repeat,
                    ..
                }) | TerminalEvent::Mouse(_)
            ) {
                app.dismiss_undo_position();
            }
            match &terminal_event {
                TerminalEvent::Key(key) if key.kind != KeyEventKind::Release => match ref_tree.handle_key(*key) {
                    ref_tree::Input::Handled => {
                        dirty = true;
                        urgent = true;
                        continue;
                    }
                    ref_tree::Input::PinReferences { id, kinds } => {
                        let result = open_repository(&repository_path, repository_is_bare, false)
                            .context("could not open repository to pin ref-tree references")
                            .and_then(|repository| ref_tree::pin_references_reporting(&repository, id, &kinds));
                        match result {
                            Ok((pins, changes)) if !pins.is_empty() => {
                                app.select_commit_after_refresh(pins[0].id);
                                return_to_history_after_refresh = Some(pins[0].id);
                                leave_recorded_success(
                                    &mut app,
                                    &repository_path,
                                    repository_is_bare,
                                    "pin references",
                                    &changes,
                                    if pins.len() == 1 {
                                        "pinned selected reference".into()
                                    } else {
                                        format!("pinned {} selected references", pins.len())
                                    },
                                );
                                refresh_pending = true;
                                refresh_expand_hidden = true;
                            }
                            Ok(_) => {
                                ref_tree.leave_attention("selected references changed before they could be pinned");
                            }
                            Err(err) => ref_tree.leave_error(format!("pin selected references: {err:#}")),
                        }
                        dirty = true;
                        urgent = true;
                        continue;
                    }
                    ref_tree::Input::ResolveRemoteReferences(names) => {
                        match open_repository(&repository_path, repository_is_bare, false) {
                            Ok(repository) => {
                                ref_tree.set_remote_deletions(ref_tree::resolve_remote_deletions(&repository, names));
                            }
                            Err(err) => ref_tree.leave_error(format!("remote edit: {err:#}")),
                        }
                        dirty = true;
                        urgent = true;
                        continue;
                    }
                    ref_tree::Input::DeleteLocalBranches { names, fallback } => {
                        let label = {
                            let names = names
                                .iter()
                                .map(|name| name.shorten().to_str_lossy().into_owned())
                                .collect::<Vec<_>>();
                            if names.len() == 1 {
                                format!("branch {}", names[0])
                            } else {
                                format!("branches {}", names.join(", "))
                            }
                        };
                        let mut deleted = false;
                        match open_repository(&repository_path, repository_is_bare, false) {
                            Ok(mut repository) => {
                                let before = names
                                    .iter()
                                    .map(|name| {
                                        edit::undo::state(&repository, name.as_ref()).map(|state| (name.clone(), state))
                                    })
                                    .collect::<Result<Vec<_>>>();
                                let result = repository.delete_local_branches(names);
                                let changed = result.is_ok()
                                    || result.as_ref().is_err_and(|err| {
                                        matches!(err, gix::repository::branch::delete::Error::Cleanup { .. })
                                    });
                                if changed {
                                    let recorded = before
                                        .and_then(|before| {
                                            before
                                                .into_iter()
                                                .map(|(name, before)| {
                                                    edit::undo::state(&repository, name.as_ref())
                                                        .map(|after| edit::undo::RefChange { name, before, after })
                                                })
                                                .collect::<Result<Vec<_>>>()
                                        })
                                        .map(|changes| {
                                            changes
                                                .into_iter()
                                                .filter(|change| change.before != change.after)
                                                .collect::<Vec<_>>()
                                        })
                                        .and_then(|changes| {
                                            edit::undo::record(&repository, "delete local branches", &changes)
                                                .map(|_| ())
                                        });
                                    let message = result.as_ref().map_or_else(
                                        |err| format!("delete {label}: {err}"),
                                        |()| format!("deleted {label}"),
                                    );
                                    match (result, recorded) {
                                        (Ok(()), Ok(())) => ref_tree.leave_success(message),
                                        (_, Ok(())) => ref_tree.leave_attention(message),
                                        (_, Err(err)) => {
                                            ref_tree.leave_attention(format!("{message}; undo history: {err:#}"));
                                        }
                                    }
                                    deleted = true;
                                    refresh_pending = true;
                                } else if let Err(err) = result {
                                    ref_tree.leave_error(format!("delete {label}: {err}"));
                                }
                            }
                            Err(err) => ref_tree.leave_error(format!("delete {label}: {err:#}")),
                        }
                        if deleted {
                            ref_tree.select_after_reference_deletion(fallback);
                        }
                        dirty = true;
                        urgent = true;
                        continue;
                    }
                    ref_tree::Input::DeleteRemoteReferences { groups, fallback } => {
                        match with_suspended_terminal(terminal, enhanced_keyboard, || {
                            Ok(push_remote_deletions(&repository_path, &groups))
                        }) {
                            Ok(outcome) => {
                                if outcome.deleted != 0 {
                                    ref_tree.select_after_reference_deletion(fallback);
                                    refresh_pending = true;
                                }
                                let deleted = format!(
                                    "deleted {} remote reference{}",
                                    outcome.deleted,
                                    if outcome.deleted == 1 { "" } else { "s" }
                                );
                                if outcome.failures.is_empty() {
                                    ref_tree.leave_success(deleted);
                                } else if outcome.deleted == 0 {
                                    ref_tree.leave_error(format!("delete on remote: {}", outcome.failures.join("; ")));
                                } else {
                                    ref_tree
                                        .leave_attention(format!("{deleted}; failed: {}", outcome.failures.join("; ")));
                                }
                            }
                            Err(err) => ref_tree.leave_error(format!("delete on remote: {err:#}")),
                        }
                        dirty = true;
                        urgent = true;
                        continue;
                    }
                    ref_tree::Input::Quit => return Ok(None),
                },
                TerminalEvent::Mouse(mouse) if ref_tree.handle_mouse(mouse.kind, mouse.modifiers, 1) => {
                    dirty = true;
                    urgent = true;
                    continue;
                }
                TerminalEvent::Paste(_) => {
                    ref_tree.leave_attention("commit paste is available only in history");
                    dirty = true;
                    urgent = true;
                    continue;
                }
                _ => {}
            }
        }
        if focused
            && !diagnostic_input
            && opens_command_menu(&terminal_event, app.actions_expanded, command_picker.is_open())
        {
            let commands = command_menu::commands(&app, &decorations, app.has_verifiable_signatures());
            command_picker.open(&command_picker_items(&commands));
            app.close_shortcut_groups();
            dirty = true;
            urgent = true;
            continue;
        }
        let command_action = if focused && command_picker.is_open() && !diagnostic_input {
            let commands = command_menu::commands(&app, &decorations, app.has_verifiable_signatures());
            let input = command_menu_input(&terminal_event, &mut command_picker, &commands);
            if !command_picker.is_open()
                && let TerminalEvent::Key(key) = &terminal_event
            {
                command_picker_key = Some(key.code);
            }
            match input {
                CommandMenuInput::Pass => None,
                CommandMenuInput::Handled => {
                    dirty = true;
                    urgent = true;
                    continue;
                }
                CommandMenuInput::Submit(action) => Some(action),
            }
        } else {
            None
        };
        let key_pressed = is_key_press(&terminal_event);
        let (mut action, repeats_history, is_repeat, throttles_draw) = if let Some(action) = command_action {
            (Some(action), false, false, false)
        } else {
            match terminal_event {
                TerminalEvent::Key(key) => {
                    let action = if diagnostic_input {
                        diagnostic_action(key, &app)
                    } else {
                        app_action(key, &app)
                    };
                    let repeats_history =
                        retains_fill_repository(key.kind, action.as_ref(), app.changes_focus.is_some());
                    (action, repeats_history, key.kind == KeyEventKind::Repeat, false)
                }
                TerminalEvent::Mouse(mouse) => {
                    let kind = mouse.kind;
                    let modifiers = mouse.modifiers;
                    let mut distance = 1;
                    if matches!(kind, MouseEventKind::ScrollUp | MouseEventKind::ScrollDown) {
                        while distance < EVENT_BATCH_SIZE && event::poll(Duration::ZERO)? {
                            let next = event::read()?;
                            match next {
                                TerminalEvent::Mouse(next) if next.kind == kind && next.modifiers == modifiers => {
                                    distance += 1;
                                }
                                next => {
                                    pending_terminal_event = Some(next);
                                    break;
                                }
                            }
                        }
                    }
                    let Some(action) = mouse_scroll_action(kind, modifiers, distance, app.changes_focus.is_some())
                    else {
                        continue;
                    };
                    let repeats_history = app.changes_focus.is_none() && repeats_viewport(&action);
                    (Some(action), repeats_history, true, true)
                }
                TerminalEvent::Paste(pasted) => {
                    let action = (|| {
                        anyhow::ensure!(!repository_is_bare, "copy-insert requires a worktree");
                        let target = app
                            .paste_insert_target()
                            .context("copy-insert paste requires an editable history selection")?;
                        let repository = open_repository(&repository_path, repository_is_bare, false)
                            .context("could not open repository for pasted commit")?;
                        let source = resolve_pasted_commit(&repository, &pasted)?;
                        Ok::<_, anyhow::Error>(Action::PasteInsert { source, target })
                    })();
                    match action {
                        Ok(action) => (Some(action), false, false, false),
                        Err(err) => {
                            app.leave_attention(format!("paste: {err:#}"));
                            dirty = true;
                            urgent = true;
                            continue;
                        }
                    }
                }
                TerminalEvent::FocusLost => {
                    focused = false;
                    app.changes_suppressed = false;
                    repeat_deadline = None;
                    dirty = true;
                    urgent = true;
                    continue;
                }
                TerminalEvent::FocusGained => {
                    focused = true;
                    if app.unseen_filesystem_redraw {
                        dirty = true;
                        urgent = true;
                    }
                    continue;
                }
                TerminalEvent::Resize(_, _) => {
                    dirty = true;
                    urgent = true;
                    continue;
                }
            }
        };
        if !focused {
            continue;
        }
        if diagnostic_input && action.is_none() {
            dirty = true;
            urgent = true;
            continue;
        }
        if repeats_history || throttles_draw {
            repeat_deadline = Some(Instant::now() + REPEAT_IDLE);
        }
        if repeats_history {
            fill_repository.retain = true;
        } else if !is_repeat {
            fill_repository.retain = false;
            fill_repository.retained = None;
        }
        if repeats_history && app.changes_mode.is_some() {
            app.changes_suppressed = true;
        } else if !is_repeat && app.changes_suppressed {
            app.changes_suppressed = false;
            repeat_deadline = None;
            dirty = true;
            urgent = true;
        }
        let conflict_reconcile = if action == Some(Action::ForceQuit) {
            ConflictReconcileStatus::Inactive
        } else {
            reconcile_external_conflict_reporting(
                &mut app,
                &repository_path,
                repository_is_bare,
                &mut pending_conflict_resolution,
            )
        };
        if conflict_reconcile == ConflictReconcileStatus::Complete {
            invalidate_worktree_changes(&mut worktree_changes);
            refresh_pending = true;
            dirty = true;
            urgent = true;
        }
        if key_pressed
            && action == Some(Action::OpenDiff)
            && app.changes_focus.is_none()
            && pending_conflict_resolution.is_some()
            && conflict_reconcile == ConflictReconcileStatus::Amend
        {
            action = Some(Action::Amend);
        }
        if key_pressed
            && action == Some(Action::Cancel)
            && pending_rebase_conflict.is_none()
            && pending_todo_rebase_conflict.is_none()
            && app.has_rebase_conflict()
        {
            let recorded = pending_conflict_resolution.as_mut().and_then(|pending| {
                pending.record_undo.then(|| {
                    record_and_clear_pending_undo(
                        &repository_path,
                        repository_is_bare,
                        "materialize time-travel conflict",
                        &mut pending.ref_changes,
                    )
                })
            });
            pending_conflict_resolution = None;
            app.clear_rebase_conflict();
            if let Some(Err(err)) = recorded {
                app.leave_attention(format!("cancelled conflict; undo history: {err:#}"));
            }
            dirty = true;
            urgent = true;
            continue;
        }
        if key_pressed && pending_rebase_conflict.is_some() {
            if action == Some(Action::OpenDiff) && app.changes_focus.is_none() {
                let clear_undo_on_accept = std::mem::take(&mut pending_conflict_clear_undo_on_accept);
                let record_undo = !clear_undo_on_accept;
                let conflict = pending_rebase_conflict
                    .take()
                    .expect("a pending conflict was checked before accepting it");
                let original = conflict.original();
                match conflict.accept() {
                    Ok((mut notice, id, _, ref_changes)) => {
                        if clear_undo_on_accept
                            && let Err(err) = clear_undo_history(&repository_path, repository_is_bare)
                        {
                            notice = format!("{notice}; undo history: {err:#}");
                        }
                        let head = match conflict_head(&repository_path, repository_is_bare, id) {
                            Ok(head) => Some(head),
                            Err(err) => {
                                notice = format!("{notice}; external-amend detection is unavailable: {err:#}");
                                None
                            }
                        };
                        pending_conflict_resolution = Some(PendingConflictResolution {
                            commit: id,
                            head,
                            ref_changes,
                            record_undo,
                        });
                        tracing::info!(commit_id = %original, rewritten_id = %id, "accepted suspended rebase conflict");
                        app.begin_conflict_resolution();
                        app.leave_attention(notice);
                        app.select_commit_after_refresh(id);
                    }
                    Err(err) => {
                        pending_conflict_resolution = None;
                        tracing::warn!(commit_id = %original, error = %err, "suspended rebase conflict checkout failed");
                        app.clear_rebase_conflict();
                        app.leave_error(format!("conflict checkout: {err:#}"));
                    }
                }
                sync_line_diff_pool(
                    &mut line_diff_pool,
                    app.changes_mode.is_some(),
                    &repository_path,
                    repository_is_bare,
                    line_diff_parallelism,
                );
                if worktree_watcher.is_none() {
                    match start_worktree_watcher(&repository_path, repository_is_bare) {
                        Ok(watcher) => worktree_watcher = Some(watcher),
                        Err(err) => {
                            tracing::warn!(error = %err, "worktree watcher startup after conflict failed");
                            app.worktree_changes.error = Some(format!("worktree watch: {err}"));
                            schedule_once(&mut watcher_retry_deadline, Instant::now(), WATCH_RETRY_INTERVAL);
                        }
                    }
                }
                invalidate_worktree_changes(&mut worktree_changes);
                refresh_pending = true;
                dirty = true;
                urgent = true;
                continue;
            }
            if action == Some(Action::Cancel) && app.changes_focus.is_none() {
                let record_undo = !std::mem::take(&mut pending_conflict_clear_undo_on_accept);
                let conflict = pending_rebase_conflict
                    .take()
                    .expect("a pending conflict was checked before discarding it");
                tracing::info!(commit_id = %conflict.original(), "discarded suspended rebase conflict");
                let mut changes = conflict.into_ref_changes();
                let recorded = record_undo.then(|| {
                    record_and_clear_pending_undo(
                        &repository_path,
                        repository_is_bare,
                        "time travel before conflict",
                        &mut changes,
                    )
                });
                app.clear_rebase_conflict();
                if let Some(Err(err)) = recorded {
                    app.leave_attention(format!("cancelled conflict; undo history: {err:#}"));
                }
                dirty = true;
                urgent = true;
                continue;
            }
        }
        if key_pressed && pending_todo_rebase_conflict.is_some() {
            if action == Some(Action::OpenDiff) && app.changes_focus.is_none() {
                let conflict = pending_todo_rebase_conflict
                    .take()
                    .expect("a pending todo conflict was checked before accepting it");
                let plan = conflict.continuation_plan();
                match edit::time_travel::materialize_plan_conflict_reporting(
                    conflict,
                    &repository_path,
                    repository_is_bare,
                    &revisions,
                    false,
                ) {
                    Ok((notice, id, _, mut ref_changes)) => {
                        pending_todo_ref_changes.append(&mut ref_changes);
                        pending_todo_rebase_plan = Some(plan);
                        app.begin_conflict_resolution();
                        app.arm_rebase_continuation();
                        app.leave_attention(format!("{notice}; resolve the index, then press <enter>"));
                        app.select_commit_after_refresh(id);
                    }
                    Err(err) => {
                        pending_todo_ref_changes.clear();
                        app.clear_rebase_conflict();
                        app.leave_error(format!("conflict checkout: {err:#}"));
                    }
                }
                invalidate_worktree_changes(&mut worktree_changes);
                refresh_pending = true;
                dirty = true;
                urgent = true;
                continue;
            }
            if action == Some(Action::Cancel) && app.changes_focus.is_none() {
                let conflict = pending_todo_rebase_conflict
                    .take()
                    .expect("a pending todo conflict was checked before discarding it");
                tracing::info!(commit_id = %conflict.original(), "discarded suspended todo rebase conflict");
                let recorded = record_and_clear_pending_undo(
                    &repository_path,
                    repository_is_bare,
                    "materialize rebase conflict",
                    &mut pending_todo_ref_changes,
                );
                app.clear_rebase_conflict();
                if let Err(err) = recorded {
                    app.leave_attention(format!("cancelled rebase conflict; undo history: {err:#}"));
                }
                refresh_pending = true;
                dirty = true;
                urgent = true;
                continue;
            }
        }
        if (pending_rebase_conflict.is_some() || pending_todo_rebase_conflict.is_some())
            && !action_allowed_during_rebase_continuation(action.as_ref(), app.changes_focus.is_some())
        {
            dirty = true;
            urgent = true;
            continue;
        }
        if key_pressed
            && action == Some(Action::Cancel)
            && app.changes_focus.is_none()
            && pending_todo_rebase_plan.is_some()
        {
            drop(pending_todo_rebase_plan.take());
            let recorded = record_and_clear_pending_undo(
                &repository_path,
                repository_is_bare,
                "materialize rebase conflict",
                &mut pending_todo_ref_changes,
            );
            app.clear_rebase_continuation();
            let message = "stopped rebase continuation; the partially applied repository remains unchanged";
            app.leave_attention(match recorded {
                Ok(()) => message.into(),
                Err(err) => format!("{message}; undo history: {err:#}"),
            });
            tracing::info!("stopped materialized rebase continuation without rolling back repository state");
            dirty = true;
            urgent = true;
            continue;
        }
        if key_pressed
            && action == Some(Action::OpenDiff)
            && app.changes_focus.is_none()
            && pending_todo_rebase_plan.is_some()
        {
            let plan = pending_todo_rebase_plan
                .take()
                .expect("a pending continuation plan was checked before resuming it");
            let result = (|| {
                let mut repository = open_repository(&repository_path, repository_is_bare, false)
                    .context("could not reopen repository to continue the rebase")?;
                repository.object_cache_size(None);
                stage_resolved_conflict_paths(&repository)?;
                let graph = HistoryGraph::for_commits(&repository, &plan.scope)?;
                run_rebase_plan(terminal, repository.into_sync(), &graph, plan.clone())
            })();
            match result {
                Ok(edit::rebase::PlanPerform::Complete(outcome)) => {
                    let checkout = if outcome.selected.is_some() {
                        edit::time_travel::checkout_plan_reporting(
                            &repository_path,
                            repository_is_bare,
                            &outcome,
                            &revisions,
                            false,
                        )
                    } else {
                        Ok((None, outcome.ref_changes.clone()))
                    };
                    app.clear_rebase_conflict();
                    app.clear_rebase_continuation();
                    app.set_worktree_conflicted(false);
                    let mut changes = std::mem::take(&mut pending_todo_ref_changes);
                    let message = match checkout {
                        Ok((notice, mut outcome_changes)) => {
                            changes.append(&mut outcome_changes);
                            notice.unwrap_or_else(|| "rebased history".into())
                        }
                        Err(err) => {
                            changes.extend(outcome.ref_changes.iter().cloned());
                            format!("rebase applied, checkout failed: {err:#}")
                        }
                    };
                    leave_recorded_success(
                        &mut app,
                        &repository_path,
                        repository_is_bare,
                        "rebase history",
                        &changes,
                        message,
                    );
                    refresh_pending = true;
                }
                Ok(edit::rebase::PlanPerform::Conflict(conflict)) => {
                    let id = conflict.commit();
                    preview_todo_rebase_conflict(
                        &mut app,
                        &conflict,
                        &authors,
                        &ref_snapshot.view_tips,
                        &ref_snapshot.hidden_tips,
                    )?;
                    app.arm_rebase_conflict(id);
                    app.select_commit(id);
                    pending_todo_rebase_conflict = Some(conflict);
                }
                Err(err) => {
                    pending_todo_rebase_plan = Some(plan);
                    app.leave_error(format!("continue rebase: {err:#}"));
                }
            }
            invalidate_worktree_changes(&mut worktree_changes);
            dirty = true;
            urgent = true;
            continue;
        }
        if pending_todo_rebase_plan.is_some()
            && !action_allowed_during_rebase_continuation(action.as_ref(), app.changes_focus.is_some())
        {
            dirty = true;
            urgent = true;
            continue;
        }
        if pending_conflict_resolution.is_some()
            && !(action == Some(Action::Amend) && conflict_reconcile == ConflictReconcileStatus::Amend)
            && !action_allowed_during_rebase_continuation(action.as_ref(), app.changes_focus.is_some())
        {
            if conflict_reconcile == ConflictReconcileStatus::Amend {
                app.leave_attention("resolve the checked-out conflict, then press <enter> to amend");
            }
            dirty = true;
            urgent = true;
            continue;
        }
        let Some(action) = action else {
            continue;
        };
        if action == Action::ToggleRefTree {
            app.dismiss_undo_position();
            if ref_tree.is_active() {
                ref_tree.leave();
            } else {
                ref_tree_refresh_pending = !ref_tree_refresh_pending;
            }
            dirty = true;
            urgent = true;
            continue;
        }
        let action = copy_selected_path_action(
            action,
            &app,
            tree_changes.as_ref().map(|(_, changes)| changes),
            worktree_changes.as_ref().map(|(_, changes)| changes),
        );
        let force_quit = action == Action::ForceQuit;
        dirty = true;
        urgent |= !throttles_draw;
        let previous_changes_mode = app.changes_mode;
        let toggles_changes = action == Action::ToggleChanges;
        let refreshes_worktree = action == Action::Refresh && app.changes_mode == Some(ChangesMode::Both);
        let effects = app.update(action);
        if refreshes_worktree {
            invalidate_worktree_changes(&mut worktree_changes);
            worktree_watch_refresh = WorktreeWatchRefresh::default();
            queued_worktree_status_full = false;
            queued_worktree_status_scopes.clear();
            worktree_refresh_deadline = None;
            match start_worktree_watcher(&repository_path, repository_is_bare) {
                Ok(watcher) => worktree_watcher = Some(watcher),
                Err(err) => {
                    tracing::warn!(error = %err, "worktree watcher refresh failed");
                    app.worktree_changes.error = Some(format!("worktree watch: {err}"));
                    worktree_watcher = None;
                    schedule_once(&mut watcher_retry_deadline, Instant::now(), WATCH_RETRY_INTERVAL);
                }
            }
        }
        if toggles_changes {
            sync_line_diff_pool(
                &mut line_diff_pool,
                app.changes_mode.is_some(),
                &repository_path,
                repository_is_bare,
                line_diff_parallelism,
            );
            if app.changes_mode == Some(ChangesMode::Both) {
                invalidate_worktree_changes(&mut worktree_changes);
                worktree_watch_refresh = WorktreeWatchRefresh::default();
                queued_worktree_status_full = false;
                queued_worktree_status_scopes.clear();
                worktree_refresh_deadline = None;
                match start_worktree_watcher(&repository_path, repository_is_bare) {
                    Ok(watcher) => {
                        worktree_watcher = Some(watcher);
                        if app
                            .worktree_changes
                            .error
                            .as_deref()
                            .is_some_and(|message| message.starts_with("worktree watch:"))
                        {
                            app.worktree_changes.error = None;
                        }
                    }
                    Err(err) => {
                        tracing::warn!(error = %err, "worktree watcher startup failed");
                        app.worktree_changes.error = Some(format!("worktree watch: {err}"));
                        schedule_once(&mut watcher_retry_deadline, Instant::now(), WATCH_RETRY_INTERVAL);
                    }
                }
            } else if previous_changes_mode == Some(ChangesMode::Both) {
                worktree_watcher = None;
                worktree_refresh_deadline = None;
                worktree_watch_refresh = WorktreeWatchRefresh::default();
                queued_worktree_status_full = false;
                queued_worktree_status_scopes.clear();
                filesystem_responses.cancel_pending_worktree("watcher-disabled");
            }
        }
        for effect in effects {
            match effect {
                Effect::Cancel => cancelled.store(true, Ordering::Relaxed),
                direction @ (Effect::Undo | Effect::Redo) => {
                    let undoing = matches!(direction, Effect::Undo);
                    fill_repository.retain = false;
                    fill_repository.retained = None;
                    let repository = open_repository(&repository_path, repository_is_bare, false)
                        .context("could not open repository for undo")?;
                    match edit::undo::review_blocks_undo(&repository) {
                        Ok(true) => {
                            app.dismiss_undo_position();
                            app.leave_attention("undo and redo are unavailable during a review");
                            continue;
                        }
                        Ok(false) => {}
                        Err(err) => {
                            app.dismiss_undo_position();
                            app.leave_error(format!("undo: {err:#}"));
                            continue;
                        }
                    }
                    let current = edit::undo::position(&repository);
                    let planned = if undoing {
                        edit::undo::plan_undo(&repository)
                    } else {
                        edit::undo::plan_redo(&repository)
                    };
                    match planned {
                        Ok(Some(plan)) => {
                            let crossed = plan.title.clone();
                            let position = plan.position.clone();
                            match plan.apply(&repository) {
                                Ok(()) => {
                                    pending_conflict_resolution = None;
                                    app.show_undo_position(
                                        position.undo,
                                        position.undo + position.redo,
                                        position.title,
                                    );
                                    if let Ok(id) = repository.head_id() {
                                        app.select_commit_after_refresh(id.detach());
                                    }
                                    invalidate_worktree_changes(&mut worktree_changes);
                                    refresh_pending = true;
                                }
                                Err(err) => {
                                    if let Ok(position) = current {
                                        app.show_undo_position(
                                            position.undo,
                                            position.undo + position.redo,
                                            position.title,
                                        );
                                    }
                                    app.leave_error(format!(
                                        "{} {crossed}: {err:#}",
                                        if undoing { "undo" } else { "redo" }
                                    ));
                                }
                            }
                        }
                        Ok(None) => {
                            if let Ok(position) = current {
                                app.show_undo_position(position.undo, position.undo + position.redo, position.title);
                            }
                            app.leave_attention(if undoing { "nothing to undo" } else { "nothing to redo" });
                        }
                        Err(err) => app.leave_error(format!("{}: {err:#}", if undoing { "undo" } else { "redo" })),
                    }
                }
                Effect::CopyId(id) => execute!(
                    terminal.backend_mut(),
                    CopyToClipboard::to_clipboard_from(id.to_hex().to_string())
                )?,
                Effect::CopyPath(path) => execute!(terminal.backend_mut(), CopyToClipboard::to_clipboard_from(path))?,
                Effect::CopyAuthor(author) => {
                    let actor = actor_bytes(author);
                    execute!(terminal.backend_mut(), CopyToClipboard::to_clipboard_from(actor))?;
                }
                Effect::Reload(show_hidden) => {
                    app.show_hidden = show_hidden;
                    refresh_pending = true;
                    refresh_expand_hidden = true;
                }
                Effect::OpenDiff(pane, index) => {
                    let changes = match pane {
                        ChangePane::Tree => tree_changes.as_ref().map(|(_, changes)| changes),
                        ChangePane::Worktree => worktree_changes.as_ref().map(|(_, changes)| changes),
                    };
                    let result = changes
                        .and_then(|changes| changes.diffs.get(index).zip(changes.paths.get(index)))
                        .context("selected path no longer has diff resources")
                        .and_then(|(change, path)| {
                            prepare_file_diff(&repository_path, repository_is_bare, change, path)
                        })
                        .and_then(|diff| show_file_diff(terminal, diff, enhanced_keyboard));
                    match result {
                        Ok(true) => app.focus_history(),
                        Err(err) => app.changes_mut(pane).error = Some(format!("{err:#}")),
                        Ok(false) => {}
                    }
                }
                Effect::OpenCommitDiff(target) => {
                    let title = match target {
                        app::TreeDiffTarget::Commit { id, .. } => app
                            .rows
                            .iter()
                            .find(|row| row.id == id)
                            .map(|row| {
                                ui::commit_diff_title(row, app.title(row), &mailmap, app.use_mailmap, app.show_emails)
                            })
                            .context("selected commit is no longer available")?,
                        app::TreeDiffTarget::Branch { base, tip } => {
                            format!("{}..{}", base.to_hex_with_len(7), tip.to_hex_with_len(7)).into()
                        }
                    };
                    let cached = tree_changes
                        .as_ref()
                        .filter(|(cached_target, _)| *cached_target == target)
                        .map(|(_, changes)| changes);
                    let result = prepare_commit_diff(&repository_path, repository_is_bare, target, cached, title)
                        .and_then(|diff| show_commit_diff(terminal, diff, enhanced_keyboard));
                    match result {
                        Ok(true) => app.focus_history(),
                        Err(err) => app.leave_error(format!("diff: {err:#}")),
                        Ok(false) => {}
                    }
                }
                Effect::Reword(id) => {
                    let hidden = if app.show_hidden { &[][..] } else { hide.as_slice() };
                    let result = reword_commit(
                        terminal,
                        &repository_path,
                        repository_is_bare,
                        &revisions,
                        hidden,
                        id,
                        enhanced_keyboard,
                    );
                    match result {
                        Ok(Some(edit::reword::Perform::Complete(edit::reword::Outcome {
                            target,
                            commit: Some(new_id),
                            ref_changes,
                            ..
                        }))) => {
                            leave_recorded_success(
                                &mut app,
                                &repository_path,
                                repository_is_bare,
                                "reword commit",
                                &ref_changes,
                                format!(
                                    "reworded {} as {}",
                                    target.to_hex_with_len(7),
                                    new_id.to_hex_with_len(7)
                                ),
                            );
                            app.select_commit_after_refresh(new_id);
                            refresh_pending = true;
                        }
                        Ok(Some(edit::reword::Perform::Complete(edit::reword::Outcome {
                            target,
                            enrichment: Some(enrichment),
                            ref_changes,
                            ..
                        }))) => {
                            app.clear_enrichments();
                            app.set_enrichment(target, enrichment);
                            leave_recorded_success(
                                &mut app,
                                &repository_path,
                                repository_is_bare,
                                "edit commit enrichment",
                                &ref_changes,
                                "updated enrichment",
                            );
                            if target != id {
                                app.select_commit_after_refresh(target);
                                refresh_pending = true;
                            }
                        }
                        Ok(Some(edit::reword::Perform::Conflict(rebase))) => {
                            let conflict = edit::time_travel::Conflict::from_rebase(
                                rebase,
                                &repository_path,
                                repository_is_bare,
                                &revisions,
                                false,
                            );
                            let original = conflict.original();
                            app.arm_rebase_conflict(original);
                            app.select_commit(original);
                            pending_conflict_clear_undo_on_accept = false;
                            pending_rebase_conflict = Some(conflict);
                        }
                        Ok(None) => {}
                        Ok(Some(edit::reword::Perform::Complete(outcome))) => {
                            if outcome.target != id {
                                app.select_commit_after_refresh(outcome.target);
                                refresh_pending = true;
                            }
                        }
                        Err(err) => app.leave_error(format!("reword: {err:#}")),
                    }
                }
                Effect::NewCommit { parent, empty } => {
                    let result = history_graph
                        .as_ref()
                        .context("creating a commit requires a completed history graph")
                        .and_then(|graph| {
                            create_commit(
                                terminal,
                                &repository_path,
                                repository_is_bare,
                                graph,
                                parent,
                                if empty {
                                    CreateMode::InsertEmpty
                                } else {
                                    CreateMode::Insert
                                },
                                enhanced_keyboard,
                            )
                        });
                    match result {
                        Ok(Some(edit::rebase::Perform::Complete(outcome))) => {
                            let new_id = outcome.selected.context("creating a commit did not select it")?;
                            leave_recorded_success(
                                &mut app,
                                &repository_path,
                                repository_is_bare,
                                if empty { "create empty commit" } else { "create commit" },
                                &outcome.ref_changes,
                                format!("created {}", new_id.to_hex_with_len(7)),
                            );
                            app.select_commit_after_refresh(new_id);
                            refresh_pending = true;
                        }
                        Ok(Some(edit::rebase::Perform::Conflict(rebase))) => {
                            let conflict = edit::time_travel::Conflict::from_rebase(
                                rebase,
                                &repository_path,
                                repository_is_bare,
                                &revisions,
                                false,
                            );
                            let original = conflict.original();
                            app.arm_rebase_conflict(original);
                            app.select_commit(original);
                            pending_conflict_clear_undo_on_accept = false;
                            pending_rebase_conflict = Some(conflict);
                        }
                        Ok(None) => app.leave_attention("no commit created: no input was provided"),
                        Err(err) => app.leave_error(format!("new commit: {err:#}")),
                    }
                }
                Effect::ForkCommit(parent) => {
                    fill_repository.retain = false;
                    fill_repository.retained = None;
                    let created = history_graph
                        .as_ref()
                        .context("creating a fork requires a completed history graph")
                        .and_then(|graph| {
                            create_commit(
                                terminal,
                                &repository_path,
                                repository_is_bare,
                                graph,
                                Some(parent),
                                CreateMode::Fork,
                                enhanced_keyboard,
                            )
                        });
                    match created {
                        Ok(Some(edit::rebase::Perform::Complete(outcome))) => {
                            let new_id = outcome.selected.context("creating a fork did not select it")?;
                            let ref_changes = outcome.ref_changes;
                            let review_roots: Vec<_> =
                                app.rows.iter().filter(|row| row.is_review).map(|row| row.id).collect();
                            let travel = open_repository(&repository_path, repository_is_bare, false)
                                .context("could not reopen repository before travelling to fork")
                                .and_then(|repository| edit::loaded_graph(&repository))
                                .and_then(|graph| {
                                    edit::time_travel::perform(
                                        &repository_path,
                                        repository_is_bare,
                                        new_id,
                                        &graph,
                                        &review_roots,
                                        &revisions,
                                        false,
                                    )
                                });
                            match travel {
                                Ok(edit::time_travel::Perform::Complete {
                                    notice,
                                    selected,
                                    ref_changes: mut travel_changes,
                                    ..
                                }) => {
                                    let mut changes = ref_changes;
                                    changes.append(&mut travel_changes);
                                    leave_recorded_success(
                                        &mut app,
                                        &repository_path,
                                        repository_is_bare,
                                        "fork commit",
                                        &changes,
                                        notice.map_or_else(
                                            || format!("created fork {}", new_id.to_hex_with_len(7)),
                                            |notice| format!("created fork {}; {notice}", new_id.to_hex_with_len(7)),
                                        ),
                                    );
                                    app.select_commit_after_refresh(selected);
                                    invalidate_worktree_changes(&mut worktree_changes);
                                    refresh_pending = true;
                                }
                                Ok(edit::time_travel::Perform::Conflict(mut conflict)) => {
                                    conflict.prepend_ref_changes(ref_changes);
                                    let original = conflict.original();
                                    app.arm_rebase_conflict(original);
                                    app.select_commit(original);
                                    pending_conflict_clear_undo_on_accept = false;
                                    pending_rebase_conflict = Some(conflict);
                                }
                                Err(err) => {
                                    leave_recorded_success(
                                        &mut app,
                                        &repository_path,
                                        repository_is_bare,
                                        "fork commit",
                                        &ref_changes,
                                        format!(
                                            "created fork {}, but checkout failed: {err:#}",
                                            new_id.to_hex_with_len(7)
                                        ),
                                    );
                                    invalidate_worktree_changes(&mut worktree_changes);
                                    refresh_pending = true;
                                }
                            }
                        }
                        Ok(Some(edit::rebase::Perform::Conflict(rebase))) => {
                            let conflict = edit::time_travel::Conflict::from_rebase(
                                rebase,
                                &repository_path,
                                repository_is_bare,
                                &revisions,
                                false,
                            );
                            let original = conflict.original();
                            app.arm_rebase_conflict(original);
                            app.select_commit(original);
                            pending_conflict_clear_undo_on_accept = false;
                            pending_rebase_conflict = Some(conflict);
                        }
                        Ok(None) => app.leave_attention("no fork created: no input was provided"),
                        Err(err) => app.leave_error(format!("fork: {err:#}")),
                    }
                }
                Effect::Split(id) => {
                    fill_repository.retain = false;
                    fill_repository.retained = None;
                    let result = history_graph
                        .as_ref()
                        .context("splitting HEAD requires a completed history graph")
                        .and_then(|graph| {
                            split_commit(terminal, &repository_path, repository_is_bare, graph, enhanced_keyboard)
                        });
                    match result {
                        Ok(Some(outcome)) => {
                            let new_id = outcome.selected.context("splitting HEAD did not select its result")?;
                            leave_recorded_success(
                                &mut app,
                                &repository_path,
                                repository_is_bare,
                                "split commit",
                                &outcome.ref_changes,
                                format!("split {} as {}", id.to_hex_with_len(7), new_id.to_hex_with_len(7)),
                            );
                            invalidate_worktree_changes(&mut worktree_changes);
                            app.select_commit_after_refresh(new_id);
                            refresh_pending = true;
                        }
                        Ok(None) => app.leave_attention("no split performed: no input was provided"),
                        Err(err) => app.leave_error(format!("split: {err:#}")),
                    }
                }
                edit @ (Effect::Amend(id) | Effect::Spill(id)) => {
                    fill_repository.retain = false;
                    fill_repository.retained = None;
                    let kind = if matches!(edit, Effect::Amend(_)) {
                        edit::head::Kind::Amend
                    } else {
                        edit::head::Kind::Spill
                    };
                    let verb = if kind == edit::head::Kind::Amend {
                        "amend"
                    } else {
                        "spill"
                    };
                    let path = match (kind, app.changes_focus) {
                        (edit::head::Kind::Spill, Some(ChangePane::Tree)) => Some(
                            tree_changes
                                .as_ref()
                                .filter(|(target, _)| target.selected() == id)
                                .and_then(|(_, changes)| {
                                    changes
                                        .paths
                                        .get(app.tree_changes.selected)
                                        .cloned()
                                        .map(|path| (path, changes.parent.map(|parent| parent.id)))
                                })
                                .context("selected tree path is no longer available"),
                        ),
                        (edit::head::Kind::Amend, Some(ChangePane::Worktree)) => Some(
                            worktree_changes
                                .as_ref()
                                .and_then(|(_, changes)| changes.paths.get(app.worktree_changes.selected))
                                .cloned()
                                .map(|path| (path, None))
                                .context("selected worktree path is no longer available"),
                        ),
                        _ => None,
                    }
                    .transpose();
                    let resolving_conflict = pending_conflict_resolution.is_some();
                    let result = history_graph
                        .as_ref()
                        .context("editing HEAD requires a completed history graph")
                        .and_then(|graph| {
                            path.and_then(|path| {
                                run_with_todo_progress(terminal, |report| {
                                    let repository = open_repository(&repository_path, repository_is_bare, false)
                                        .context("could not open repository for HEAD edit")?;
                                    if kind == edit::head::Kind::Amend && resolving_conflict {
                                        stage_resolved_conflict_paths(&repository)?;
                                    }
                                    edit::head::perform_with_changes(
                                        repository,
                                        graph,
                                        kind,
                                        path.as_ref().map(|(path, parent)| (path, *parent)),
                                        resolving_conflict,
                                        report,
                                    )
                                })
                            })
                        });
                    match result {
                        Ok(Some(outcome)) => {
                            let new_id = outcome.selected.context("editing HEAD did not select its result")?;
                            let pending = if kind == edit::head::Kind::Amend {
                                pending_conflict_resolution.take()
                            } else {
                                None
                            };
                            let resolved_conflict = pending.is_some();
                            let record_undo = pending.as_ref().is_none_or(|pending| pending.record_undo);
                            let mut changes = pending.map(|pending| pending.ref_changes).unwrap_or_default();
                            changes.extend(outcome.ref_changes.iter().cloned());
                            let message =
                                format!("{verb}ed {} as {}", id.to_hex_with_len(7), new_id.to_hex_with_len(7));
                            if record_undo {
                                leave_recorded_success(
                                    &mut app,
                                    &repository_path,
                                    repository_is_bare,
                                    if resolved_conflict {
                                        "resolve rebase conflict"
                                    } else {
                                        verb
                                    },
                                    &changes,
                                    message,
                                );
                            } else {
                                app.leave_success(message);
                            }
                            invalidate_worktree_changes(&mut worktree_changes);
                            app.select_commit_after_refresh(new_id);
                            refresh_pending = true;
                        }
                        Ok(None) => app.leave_attention(format!("nothing to {verb}")),
                        Err(err) => app.leave_error(format!("{verb}: {err:#}")),
                    }
                }
                Effect::Stash(id) => {
                    fill_repository.retain = false;
                    fill_repository.retained = None;
                    match edit::stash::save_manual(&repository_path, repository_is_bare, id) {
                        Ok(notice) => {
                            app.leave_success(notice);
                            app.select_commit_after_refresh(id);
                            invalidate_worktree_changes(&mut worktree_changes);
                            refresh_pending = true;
                        }
                        Err(err) => app.leave_error(format!("stash: {err:#}")),
                    }
                }
                Effect::Unstash(id) => {
                    fill_repository.retain = false;
                    fill_repository.retained = None;
                    match edit::stash::restore_manual(&repository_path, repository_is_bare, id) {
                        Ok(notice) => {
                            app.leave_success(notice);
                            app.select_commit_after_refresh(id);
                            invalidate_worktree_changes(&mut worktree_changes);
                            refresh_pending = true;
                        }
                        Err(err) => app.leave_error(format!("unstash: {err:#}")),
                    }
                }
                Effect::Forget(id) => {
                    fill_repository.retain = false;
                    fill_repository.retained = None;
                    let cancels_review = app.rows.iter().any(|row| row.id == id && row.is_review);
                    if cancels_review {
                        app.dismiss_undo_position();
                    }
                    let result = history_graph
                        .as_ref()
                        .context("forget requires a completed history graph")
                        .and_then(|graph| {
                            if cancels_review {
                                clear_undo_history(&repository_path, repository_is_bare)
                                    .context("could not clear undo history before cancelling review")?;
                            }
                            forget_commit(terminal, &repository_path, repository_is_bare, graph, id)
                        });
                    match result {
                        Ok(edit::forget::Perform::Complete(outcome)) => {
                            let ref_changes = outcome.ref_changes.clone();
                            let returned = outcome.review_return.as_ref().map(|name| {
                                edit::time_travel::checkout_review_return_reporting(
                                    &repository_path,
                                    repository_is_bare,
                                    name,
                                    &revisions,
                                    false,
                                )
                            });
                            match returned.transpose() {
                                Ok(Some((selected, _, mut checkout_changes))) => {
                                    let mut changes = ref_changes;
                                    changes.append(&mut checkout_changes);
                                    if cancels_review {
                                        app.leave_success("cancelled review");
                                    } else {
                                        leave_recorded_success(
                                            &mut app,
                                            &repository_path,
                                            repository_is_bare,
                                            "cancel review",
                                            &changes,
                                            "cancelled review",
                                        );
                                    }
                                    app.select_commit_after_refresh(selected);
                                }
                                Err(err) => {
                                    let message = format!("review cancelled, return checkout failed: {err:#}");
                                    if cancels_review {
                                        app.leave_attention(message);
                                    } else {
                                        leave_recorded_success(
                                            &mut app,
                                            &repository_path,
                                            repository_is_bare,
                                            "forget commit",
                                            &ref_changes,
                                            message,
                                        );
                                    }
                                }
                                Ok(None) => {
                                    let message = format!("forgot {}", id.to_hex_with_len(7));
                                    if cancels_review {
                                        app.leave_success(message);
                                    } else {
                                        leave_recorded_success(
                                            &mut app,
                                            &repository_path,
                                            repository_is_bare,
                                            "forget commit",
                                            &ref_changes,
                                            message,
                                        );
                                    }
                                    if let Some(selected) = outcome.selected {
                                        app.select_commit(selected);
                                    }
                                }
                            }
                            invalidate_worktree_changes(&mut worktree_changes);
                            refresh_pending = true;
                        }
                        Ok(edit::forget::Perform::Conflict(conflict)) => {
                            let conflict = edit::time_travel::Conflict::from_rebase(
                                conflict.into_rebase(),
                                &repository_path,
                                repository_is_bare,
                                &revisions,
                                false,
                            );
                            let original = conflict.original();
                            app.arm_rebase_conflict(original);
                            app.select_commit(original);
                            pending_conflict_clear_undo_on_accept = false;
                            pending_rebase_conflict = Some(conflict);
                        }
                        Err(err) => app.leave_error(format!("forget: {err:#}")),
                    }
                }
                Effect::Rebase { base, onto, commits } => {
                    fill_repository.retain = false;
                    fill_repository.retained = None;
                    let todo_commits = (|| {
                        let mut repository = open_repository(&repository_path, repository_is_bare, false)
                            .context("could not open repository before formatting the rebase todo")?;
                        repository.object_cache_size(None);
                        load_rebase_todo_commits(&repository, &mut app, &authors, &commits)
                    })();
                    let result = history_graph
                        .as_ref()
                        .context("rebasing requires a completed history graph")
                        .and_then(|graph| {
                            rebase_history(
                                terminal,
                                &repository_path,
                                repository_is_bare,
                                graph,
                                base,
                                onto,
                                todo_commits?,
                                enhanced_keyboard,
                            )
                        });
                    match result {
                        Ok(Some(edit::rebase::PlanPerform::Complete(outcome))) => {
                            let notice = if outcome.selected.is_some() {
                                edit::time_travel::checkout_plan_reporting(
                                    &repository_path,
                                    repository_is_bare,
                                    &outcome,
                                    &revisions,
                                    false,
                                )
                            } else {
                                Ok((None, outcome.ref_changes.clone()))
                            };
                            match notice {
                                Ok((notice, changes)) => {
                                    leave_recorded_success(
                                        &mut app,
                                        &repository_path,
                                        repository_is_bare,
                                        "rebase history",
                                        &changes,
                                        notice.unwrap_or_else(|| "rebased history".to_owned()),
                                    );
                                    app.select_commit_after_refresh(base);
                                    invalidate_worktree_changes(&mut worktree_changes);
                                    refresh_pending = true;
                                }
                                Err(err) => {
                                    leave_recorded_success(
                                        &mut app,
                                        &repository_path,
                                        repository_is_bare,
                                        "rebase history",
                                        &outcome.ref_changes,
                                        format!("rebase applied, checkout failed: {err:#}"),
                                    );
                                    invalidate_worktree_changes(&mut worktree_changes);
                                    refresh_pending = true;
                                }
                            }
                        }
                        Ok(Some(edit::rebase::PlanPerform::Conflict(conflict))) => {
                            let id = conflict.commit();
                            preview_todo_rebase_conflict(
                                &mut app,
                                &conflict,
                                &authors,
                                &ref_snapshot.view_tips,
                                &ref_snapshot.hidden_tips,
                            )?;
                            app.arm_rebase_conflict(id);
                            app.select_commit(id);
                            pending_todo_rebase_conflict = Some(conflict);
                        }
                        Ok(None) => app.leave_attention("no rebase performed: the todo was unchanged"),
                        Err(err) => app.leave_error(format!("rebase: {err:#}")),
                    }
                }
                Effect::Squash { source, target } => {
                    fill_repository.retain = false;
                    fill_repository.retained = None;
                    let result = (|| {
                        let graph = history_graph
                            .as_ref()
                            .context("squashing requires a completed history graph")?;
                        let mut repository = open_repository(&repository_path, repository_is_bare, false)
                            .context("could not open repository to squash commits")?;
                        repository.object_cache_size(None);
                        let plan = edit::rebase::squash_plan(&repository, graph, source, target)?;
                        run_rebase_plan(terminal, repository.into_sync(), graph, plan)
                    })();
                    match result {
                        Ok(edit::rebase::PlanPerform::Complete(outcome)) => {
                            let combined = outcome.map(target).unwrap_or(target);
                            let notice = if outcome.selected.is_some() {
                                edit::time_travel::checkout_plan_reporting(
                                    &repository_path,
                                    repository_is_bare,
                                    &outcome,
                                    &revisions,
                                    false,
                                )
                            } else {
                                Ok((None, outcome.ref_changes.clone()))
                            };
                            let (message, changes) = notice.map_or_else(
                                |err| {
                                    (
                                        format!("squash applied, checkout failed: {err:#}"),
                                        outcome.ref_changes.clone(),
                                    )
                                },
                                |(notice, changes)| {
                                    (
                                        notice.unwrap_or_else(|| {
                                            format!(
                                                "squashed {} into {}",
                                                source.to_hex_with_len(7),
                                                combined.to_hex_with_len(7)
                                            )
                                        }),
                                        changes,
                                    )
                                },
                            );
                            leave_recorded_success(
                                &mut app,
                                &repository_path,
                                repository_is_bare,
                                "squash commits",
                                &changes,
                                message,
                            );
                            app.select_commit_after_refresh(combined);
                            invalidate_worktree_changes(&mut worktree_changes);
                            refresh_pending = true;
                        }
                        Ok(edit::rebase::PlanPerform::Conflict(conflict)) => {
                            let id = conflict.commit();
                            preview_todo_rebase_conflict(
                                &mut app,
                                &conflict,
                                &authors,
                                &ref_snapshot.view_tips,
                                &ref_snapshot.hidden_tips,
                            )?;
                            app.arm_rebase_conflict(id);
                            app.select_commit(id);
                            pending_todo_rebase_conflict = Some(conflict);
                        }
                        Err(err) => app.leave_error(format!("squash: {err:#}")),
                    }
                }
                effect @ (Effect::Insert { .. } | Effect::PasteInsert { .. }) => {
                    let (source, base, target, copy, pasted) = match effect {
                        Effect::Insert {
                            source,
                            base,
                            target,
                            copy,
                        } => (source, base, target, copy, false),
                        Effect::PasteInsert { source, target } => (source, source, target, true, true),
                        _ => unreachable!("the match arm accepts only insertion effects"),
                    };
                    fill_repository.retain = false;
                    fill_repository.retained = None;
                    let result = (|| {
                        let mut repository = open_repository(&repository_path, repository_is_bare, false)
                            .context("could not open repository to insert commits")?;
                        repository.object_cache_size(None);
                        let loaded_graph;
                        let graph = if pasted {
                            let graph_revisions = [
                                OsString::from("HEAD"),
                                OsString::from(source.to_string()),
                                OsString::from(target.to_string()),
                            ];
                            loaded_graph = edit::loaded_view_graph_with(&repository, &graph_revisions)?;
                            &loaded_graph
                        } else {
                            history_graph
                                .as_ref()
                                .context("inserting commits requires a completed history graph")?
                        };
                        let plan = if copy {
                            edit::rebase::copy_insert_plan(&repository, graph, source, target)?
                        } else if base == source {
                            edit::rebase::move_insert_plan(&repository, graph, source, target)?
                        } else {
                            edit::rebase::stack_insert_plan(&repository, graph, base, source, target)?
                        };
                        run_rebase_plan(terminal, repository.into_sync(), graph, plan)
                    })();
                    match result {
                        Ok(edit::rebase::PlanPerform::Complete(outcome)) => {
                            let inserted = if copy {
                                outcome.selected.expect("copy-insert selects the copied commit")
                            } else {
                                outcome.map(source).unwrap_or(source)
                            };
                            let notice = edit::time_travel::checkout_plan_reporting(
                                &repository_path,
                                repository_is_bare,
                                &outcome,
                                &revisions,
                                false,
                            );
                            let (message, changes) = notice.map_or_else(
                                |err| {
                                    (
                                        format!("insert applied, checkout failed: {err:#}"),
                                        outcome.ref_changes.clone(),
                                    )
                                },
                                |(notice, changes)| {
                                    (
                                        notice.unwrap_or_else(|| {
                                            if copy {
                                                format!(
                                                    "copied {} as {} above {}",
                                                    source.to_hex_with_len(7),
                                                    inserted.to_hex_with_len(7),
                                                    target.to_hex_with_len(7)
                                                )
                                            } else {
                                                format!(
                                                    "inserted {} above {}",
                                                    inserted.to_hex_with_len(7),
                                                    target.to_hex_with_len(7)
                                                )
                                            }
                                        }),
                                        changes,
                                    )
                                },
                            );
                            leave_recorded_success(
                                &mut app,
                                &repository_path,
                                repository_is_bare,
                                if copy {
                                    "copy-insert commit"
                                } else if base == source {
                                    "move-insert commit"
                                } else {
                                    "stack-insert commits"
                                },
                                &changes,
                                message,
                            );
                            app.select_commit_after_refresh(inserted);
                            invalidate_worktree_changes(&mut worktree_changes);
                            refresh_pending = true;
                        }
                        Ok(edit::rebase::PlanPerform::Conflict(conflict)) => {
                            let id = conflict.commit();
                            preview_todo_rebase_conflict(
                                &mut app,
                                &conflict,
                                &authors,
                                &ref_snapshot.view_tips,
                                &ref_snapshot.hidden_tips,
                            )?;
                            app.arm_rebase_conflict(id);
                            app.select_commit(id);
                            pending_todo_rebase_conflict = Some(conflict);
                        }
                        Err(err) => app.leave_error(format!("insert: {err:#}")),
                    }
                }
                Effect::StartReview { tip, base } => {
                    fill_repository.retain = false;
                    fill_repository.retained = None;
                    let result = history_graph
                        .as_ref()
                        .context("review requires a completed history graph")
                        .and_then(|graph| edit::review::start(&repository_path, repository_is_bare, graph, tip, base));
                    match result {
                        Ok(started) => {
                            app.dismiss_undo_position();
                            let message = format!(
                                "started review {} at {}",
                                started.reference.shorten(),
                                started.commit.to_hex_with_len(7)
                            );
                            match clear_undo_history(&repository_path, repository_is_bare) {
                                Ok(()) => app.leave_success(message),
                                Err(err) => app.leave_attention(format!("{message}; undo history: {err:#}")),
                            }
                            app.select_commit_after_refresh(started.commit);
                            invalidate_worktree_changes(&mut worktree_changes);
                            refresh_pending = true;
                        }
                        Err(err) => app.leave_error(format!("review: {err:#}")),
                    }
                }
                Effect::FinishReview { review: id, return_to } => {
                    fill_repository.retain = false;
                    fill_repository.retained = None;
                    let result = history_graph
                        .as_ref()
                        .context("finishing review requires a completed history graph")
                        .and_then(|graph| {
                            run_with_todo_progress(terminal, |report| {
                                let mut repo = open_repository(&repository_path, repository_is_bare, false)
                                    .context("could not open repository to finish review")?;
                                repo.object_cache_size(None);
                                edit::review::finish_with_progress(repo, graph, id, return_to, report)
                            })
                        });
                    match result {
                        Ok(edit::review::Finish::Complete(finished)) => {
                            let undo_cleared = clear_undo_history(&repository_path, repository_is_bare);
                            let checkout = edit::time_travel::checkout_plan_reporting(
                                &repository_path,
                                repository_is_bare,
                                &finished.outcome,
                                &revisions,
                                false,
                            );
                            let mut message = checkout.map_or_else(
                                |err| format!("review applied, return checkout failed: {err:#}"),
                                |(_, _changes)| format!("finished review as {}", finished.commit.to_hex_with_len(7)),
                            );
                            if let Err(err) = undo_cleared {
                                message = format!("{message}; undo history: {err:#}");
                            }
                            app.dismiss_undo_position();
                            app.leave_success(message);
                            app.select_commit_after_refresh(finished.outcome.selected.unwrap_or(finished.commit));
                            invalidate_worktree_changes(&mut worktree_changes);
                            refresh_pending = true;
                        }
                        Ok(edit::review::Finish::SelectReturn { tip }) => {
                            if !app.select_review_return(id, tip) {
                                app.leave_error(
                                    "finish review: no visible return commit descends from the reviewed tip",
                                );
                            }
                        }
                        Ok(edit::review::Finish::Conflict(rebase)) => {
                            let conflict = edit::time_travel::Conflict::from_rebase(
                                rebase,
                                &repository_path,
                                repository_is_bare,
                                &revisions,
                                false,
                            );
                            let original = conflict.original();
                            app.arm_rebase_conflict(original);
                            app.select_commit(original);
                            pending_conflict_clear_undo_on_accept = true;
                            pending_rebase_conflict = Some(conflict);
                        }
                        Err(err) => app.leave_error(format!("finish review: {err:#}")),
                    }
                }
                Effect::Attach => {
                    fill_repository.retain = false;
                    fill_repository.retained = None;
                    match edit::time_travel::attach_reporting(&repository_path, repository_is_bare, &revisions, false) {
                        Ok((notice, changes)) => {
                            leave_recorded_success(
                                &mut app,
                                &repository_path,
                                repository_is_bare,
                                "attach HEAD",
                                &changes,
                                notice,
                            );
                            refresh_pending = true;
                        }
                        Err(err) => app.leave_error(format!("attach: {err:#}")),
                    }
                }
                Effect::TimeTravel(id) => {
                    fill_repository.retain = false;
                    fill_repository.retained = None;
                    let review_roots: Vec<_> = app.rows.iter().filter(|row| row.is_review).map(|row| row.id).collect();
                    app.begin_time_travel_animation();
                    let result = history_graph
                        .as_ref()
                        .context("time-travel requires a completed history graph")
                        .and_then(|graph| {
                            let repository = open_fill_repository(&repository_path, repository_is_bare)
                                .context("could not open repository for time-travel animation")?;
                            run_with_rebase_selection(
                                |report| {
                                    edit::time_travel::perform_reporting_rebased(
                                        &repository_path,
                                        repository_is_bare,
                                        id,
                                        graph,
                                        &review_roots,
                                        &revisions,
                                        false,
                                        report,
                                    )
                                },
                                |id| {
                                    app.select_commit_for_time_travel(id);
                                    let render_rows = terminal.get_frame().area().height.saturating_sub(1) as usize;
                                    load_visible_history_metadata(&repository, &mut app, &authors, render_rows)?;
                                    let message = commit_message.as_ref().map(|(_, message)| message.as_bstr());
                                    let tree = tree_changes.as_ref().map(|(_, changes)| changes);
                                    let worktree = worktree_changes
                                        .as_ref()
                                        .filter(|(marker, _)| *marker == WORKTREE_STATUS_CURRENT)
                                        .map(|(_, changes)| changes);
                                    terminal
                                        .draw(|frame| {
                                            ui::draw_with_worktree(
                                                frame,
                                                &mut app,
                                                &decorations,
                                                &mailmap,
                                                message,
                                                tree,
                                                worktree,
                                            );
                                        })
                                        .context("could not draw time-travel animation")?;
                                    Ok(())
                                },
                            )
                        });
                    app.finish_time_travel_animation();
                    match result {
                        Ok(edit::time_travel::Perform::Complete {
                            notice: Some(notice),
                            selected,
                            ref_changes,
                            ..
                        }) => {
                            tracing::info!(selected = %id, %notice, "completed time-travel action");
                            leave_recorded_success(
                                &mut app,
                                &repository_path,
                                repository_is_bare,
                                "time travel",
                                &ref_changes,
                                notice,
                            );
                            app.select_commit_after_refresh(selected);
                            invalidate_worktree_changes(&mut worktree_changes);
                            refresh_pending = true;
                        }
                        Ok(edit::time_travel::Perform::Complete {
                            notice: None,
                            selected,
                            ref_changes,
                            ..
                        }) => {
                            if !ref_changes.is_empty() {
                                app.select_commit_after_refresh(selected);
                                leave_recorded_success(
                                    &mut app,
                                    &repository_path,
                                    repository_is_bare,
                                    "time travel",
                                    &ref_changes,
                                    "time-travelled",
                                );
                                invalidate_worktree_changes(&mut worktree_changes);
                                refresh_pending = true;
                            }
                        }
                        Ok(edit::time_travel::Perform::Conflict(conflict)) => {
                            let original = conflict.original();
                            app.arm_rebase_conflict(original);
                            app.select_commit(original);
                            pending_conflict_clear_undo_on_accept = false;
                            pending_rebase_conflict = Some(conflict);
                        }
                        Err(err) => {
                            app.leave_error(format!("time-travel: {err:#}"));
                            invalidate_worktree_changes(&mut worktree_changes);
                            refresh_pending = true;
                        }
                    }
                }
                Effect::TogglePin(id) => {
                    fill_repository.retain = false;
                    fill_repository.retained = None;
                    match edit::time_travel::toggle_pin_reporting(&repository_path, repository_is_bare, id) {
                        Ok((edit::time_travel::PinToggle::Removed(count), changes)) => {
                            leave_recorded_success(
                                &mut app,
                                &repository_path,
                                repository_is_bare,
                                "unpin commit",
                                &changes,
                                format!("removed {count} pin{}", if count == 1 { "" } else { "s" }),
                            );
                            app.select_commit_after_refresh(id);
                            refresh_pending = true;
                        }
                        Ok((edit::time_travel::PinToggle::Created, changes)) => {
                            leave_recorded_success(
                                &mut app,
                                &repository_path,
                                repository_is_bare,
                                "pin commit",
                                &changes,
                                "pinned selected commit",
                            );
                            app.select_commit_after_refresh(id);
                            refresh_pending = true;
                        }
                        Err(err) => app.leave_error(format!("toggle pin: {err:#}")),
                    }
                }
                Effect::ToggleTodo(id) => {
                    fill_repository.retain = false;
                    fill_repository.retained = None;
                    let result = open_repository(&repository_path, repository_is_bare, false)
                        .context("could not open repository to update the enrichment")
                        .and_then(|repo| tracked_ref_update(&repo, enrich::REF_NAME, |repo| enrich::toggle(repo, id)));
                    match result {
                        Ok((enrichment, changes)) => {
                            let enabled = enrichment.todo;
                            app.clear_enrichments();
                            app.set_enrichment(id, enrichment);
                            leave_tracked_success(
                                &mut app,
                                &repository_path,
                                repository_is_bare,
                                "toggle todo",
                                changes,
                                if enabled {
                                    "marked commit todo"
                                } else {
                                    "cleared commit todo"
                                },
                            );
                        }
                        Err(err) => app.leave_error(format!("todo: {err:#}")),
                    }
                }
                Effect::ToggleChecksPass(id) => {
                    fill_repository.retain = false;
                    fill_repository.retained = None;
                    let result = open_repository(&repository_path, repository_is_bare, false)
                        .context("could not open repository to update the tree enrichment")
                        .and_then(|repo| {
                            tracked_ref_update(&repo, enrich::TREE_REF_NAME, |repo| {
                                enrich::toggle_checks_pass(repo, id)
                            })
                        });
                    match result {
                        Ok((enrichment, changes)) => {
                            let enabled = enrichment.checks_pass;
                            app.clear_enrichments();
                            app.set_tree_enrichment(id, enrichment);
                            leave_tracked_success(
                                &mut app,
                                &repository_path,
                                repository_is_bare,
                                "toggle checks-pass",
                                changes,
                                if enabled {
                                    "marked tree checks-pass"
                                } else {
                                    "cleared tree checks-pass"
                                },
                            );
                        }
                        Err(err) => app.leave_error(format!("checks-pass: {err:#}")),
                    }
                }
                Effect::EditNote(id) => {
                    fill_repository.retain = false;
                    fill_repository.retained = None;
                    match edit_note(terminal, &repository_path, repository_is_bare, id, enhanced_keyboard) {
                        Ok(Some((enrichment, changes))) => {
                            let has_note = enrichment.note.is_some();
                            app.clear_enrichments();
                            app.set_enrichment(id, enrichment);
                            leave_tracked_success(
                                &mut app,
                                &repository_path,
                                repository_is_bare,
                                "edit commit note",
                                changes,
                                if has_note { "saved note" } else { "cleared note" },
                            );
                        }
                        Ok(None) => {}
                        Err(err) => app.leave_error(format!("note: {err:#}")),
                    }
                }
                Effect::EditGitNote(id) => {
                    fill_repository.retain = false;
                    fill_repository.retained = None;
                    match edit_git_note(terminal, &repository_path, repository_is_bare, id, enhanced_keyboard) {
                        Ok(Some((saved, changes))) => {
                            app.clear_notes(id);
                            leave_tracked_success(
                                &mut app,
                                &repository_path,
                                repository_is_bare,
                                "edit Git note",
                                changes,
                                if saved { "saved Git note" } else { "cleared Git note" },
                            );
                        }
                        Ok(None) => {}
                        Err(err) => app.leave_error(format!("Git note: {err:#}")),
                    }
                }
                Effect::VerifySignatures(ids) => {
                    verification_receiver = Some(start_signature_verification(
                        repository_path.clone(),
                        repository_is_bare,
                        ids,
                    ));
                }
                Effect::Quit if force_quit => return Ok(None),
                Effect::Quit => {
                    if let Some(conflict) = pending_rebase_conflict.take() {
                        let mut changes = conflict.into_ref_changes();
                        if !std::mem::take(&mut pending_conflict_clear_undo_on_accept)
                            && let Err(err) = record_and_clear_pending_undo(
                                &repository_path,
                                repository_is_bare,
                                "time travel before conflict",
                                &mut changes,
                            )
                        {
                            tracing::warn!(error = %err, "could not record suspended conflict before exit");
                        }
                    }
                    if let Some(mut pending) = pending_conflict_resolution.take()
                        && pending.record_undo
                        && let Err(err) = record_and_clear_pending_undo(
                            &repository_path,
                            repository_is_bare,
                            "materialize time-travel conflict",
                            &mut pending.ref_changes,
                        )
                    {
                        tracing::warn!(error = %err, "could not record materialized conflict before exit");
                    }
                    let todo_pending =
                        pending_todo_rebase_conflict.take().is_some() || pending_todo_rebase_plan.take().is_some();
                    if todo_pending
                        && let Err(err) = record_and_clear_pending_undo(
                            &repository_path,
                            repository_is_bare,
                            "materialize rebase conflict",
                            &mut pending_todo_ref_changes,
                        )
                    {
                        tracing::warn!(error = %err, "could not record materialized rebase before exit");
                    }
                    return Ok(None);
                }
            }
        }
    })();
    result
}

fn start_lane_worker(rows: Vec<SharedCommitRow>) -> mpsc::Receiver<(Vec<SharedCommitRow>, app::Graph, Duration)> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(app::compute_lanes(rows));
    });
    receiver
}

fn scan_change_ids(
    repository_path: &Path,
    bare: bool,
    enabled: bool,
    rows: &[SharedCommitRow],
) -> Result<change_id::Scan> {
    if !enabled {
        return Ok(change_id::Scan::default());
    }
    let mut repository = open_repository(repository_path, bare, false)?;
    repository.object_cache_size_if_unset(OBJECT_CACHE_SIZE);
    change_id::scan(&repository, &rows.iter().map(|row| row.id).collect::<Vec<_>>())
}

fn change_id_scan_needed(app: &App) -> bool {
    app.has_hidden_filter && !app.show_hidden
}

type SignatureVerification = (gix::ObjectId, bool);

fn start_signature_verification(
    repository_path: PathBuf,
    bare: bool,
    ids: Vec<gix::ObjectId>,
) -> mpsc::Receiver<Vec<SignatureVerification>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let results = match open_repository(&repository_path, bare, false) {
            Ok(mut repository) => {
                repository.object_cache_size(None);
                ids.into_iter()
                    .map(|id| {
                        let result = repository
                            .find_commit(id)
                            .context("could not read signed commit")
                            .and_then(|commit| {
                                commit
                                    .verify_signature()
                                    .context("could not verify commit signature")
                                    .and_then(|outcome| outcome.context("commit no longer has a signature"))
                            });
                        match result {
                            Ok(outcome) if outcome.is_valid() => (id, true),
                            Ok(_) | Err(_) => (id, false),
                        }
                    })
                    .collect()
            }
            Err(_) => ids.into_iter().map(|id| (id, false)).collect(),
        };
        let _ = sender.send(results);
    });
    receiver
}

fn start_history(
    repository: gix::ThreadSafeRepository,
    revisions: &[OsString],
    hidden_revisions: &[OsString],
    include_worktrees: bool,
    authors: SharedAuthors,
) -> (Arc<AtomicBool>, mpsc::Receiver<Result<Event>>) {
    let cancelled = Arc::new(AtomicBool::new(false));
    let worker_cancelled = Arc::clone(&cancelled);
    let (sender, receiver) = mpsc::channel();
    let revisions = revisions.to_vec();
    let hidden_revisions = hidden_revisions.to_vec();
    std::thread::spawn(move || {
        let mut repository = repository.to_thread_local();
        repository.object_cache_size_if_unset(OBJECT_CACHE_SIZE);
        let result = history::load(
            &repository,
            &revisions,
            &hidden_revisions,
            include_worktrees,
            &authors,
            &worker_cancelled,
            |event| sender.send(Ok(event)).is_ok(),
        );
        if let Err(err) = result {
            let _ = sender.send(Err(err));
        }
    });
    (cancelled, receiver)
}

#[expect(
    clippy::too_many_arguments,
    reason = "the worker owns each independent refresh input"
)]
fn start_history_refresh(
    repository_path: PathBuf,
    bare: bool,
    revisions: Vec<OsString>,
    hidden_revisions: Vec<OsString>,
    include_worktrees: bool,
    expand: std::collections::HashSet<gix::ObjectId>,
    authors: SharedAuthors,
    mut graph: HistoryGraph,
    kind: RefreshKind,
) -> mpsc::Receiver<(RefreshKind, HistoryGraph, Result<history::Refresh>)> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = open_repository(&repository_path, bare, true)
            .context("could not reopen repository for history refresh")
            .and_then(|mut repository| {
                repository.object_cache_size_if_unset(OBJECT_CACHE_SIZE);
                graph.refresh(
                    &repository,
                    &revisions,
                    &hidden_revisions,
                    include_worktrees,
                    &expand,
                    &authors,
                )
            });
        let _ = sender.send((kind, graph, result));
    });
    receiver
}

fn start_ref_watcher(git_dir: &Path, common_dir: &Path) -> Result<RefWatcher> {
    let (sender, events) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = sender.send(event);
    })
    .context("could not initialize reference watcher")?;
    let worktrees_dir = common_dir.join("worktrees");
    let linked_git_dir_is_covered = worktrees_dir.is_dir() && git_dir.starts_with(&worktrees_dir);
    let mut roots = vec![(common_dir.to_owned(), RecursiveMode::NonRecursive)];
    if git_dir != common_dir && !linked_git_dir_is_covered {
        roots.push((git_dir.to_owned(), RecursiveMode::NonRecursive));
    }
    for root in [common_dir.join("refs"), git_dir.join("refs")] {
        if root.is_dir()
            && !(linked_git_dir_is_covered && root.starts_with(&worktrees_dir))
            && !roots.iter().any(|(path, _)| path == &root)
        {
            roots.push((root, RecursiveMode::Recursive));
        }
    }
    if worktrees_dir.is_dir() {
        roots.push((worktrees_dir.clone(), RecursiveMode::Recursive));
    }
    for (path, mode) in &roots {
        watcher
            .watch(path, *mode)
            .with_context(|| format!("could not watch references at {}", path.display()))?;
    }
    tracing::info!(?roots, "watching references");
    Ok(RefWatcher {
        _watcher: watcher,
        events,
        git_dir: git_dir.to_owned(),
        worktrees_dir,
    })
}

fn start_worktree_watcher(repository_path: &Path, bare: bool) -> Result<WorktreeWatcher> {
    let started = Instant::now();
    let repository = open_repository(repository_path, bare, false)
        .context("could not open repository for worktree watcher setup")?;
    let workdir = repository
        .workdir()
        .context("cannot watch a bare repository")?
        .to_owned();
    let index_path = repository.index_path();
    let git_dir = repository.git_dir().to_owned();
    let dot_git = workdir.join(gix::discover::DOT_GIT_DIR);
    let dirwalk_started = Instant::now();
    let index = repository
        .index_or_empty()
        .context("could not open index for worktree watcher")?;
    let mut directories = worktree_watch_directories_with_index(&repository, &index)?;
    let index_projection = index_watch_projection(&index);
    let dirwalk_ms = dirwalk_started.elapsed().as_millis();
    let registration_started = Instant::now();
    let (sender, events) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = sender.send(event);
    })
    .context("could not initialize worktree watcher")?;
    let index_parent = index_path.parent().context("index path has no parent")?;
    directories.insert(index_parent.to_owned());
    {
        let mut paths = watcher.paths_mut();
        for directory in &directories {
            paths
                .add(directory, RecursiveMode::NonRecursive)
                .with_context(|| format!("could not watch worktree directory at {}", directory.display()))?;
        }
        paths.commit().context("could not apply worktree watches")?;
    }
    tracing::info!(
        workdir = %workdir.display(),
        index = %index_path.display(),
        directories = directories.len(),
        dirwalk_ms,
        registration_ms = registration_started.elapsed().as_millis(),
        setup_ms = started.elapsed().as_millis(),
        "watching worktree changes"
    );
    Ok(WorktreeWatcher {
        watcher,
        events,
        directories,
        index_projection,
        workdir,
        dot_git,
        git_dir,
        index: index_path,
    })
}

fn worktree_status_head(repository: &gix::Repository) -> Result<WorktreeStatusHead> {
    let mut head = repository.head().context("could not read HEAD")?;
    let reference = head.referent_name().map(ToOwned::to_owned);
    let target = head
        .try_peel_to_id()
        .context("could not peel HEAD")?
        .map(gix::Id::detach);
    Ok(WorktreeStatusHead { reference, target })
}

fn remember_worktree_status_head(
    cached: &mut Option<WorktreeStatusHead>,
    refreshes_staged: bool,
    scanned: Result<WorktreeStatusHead>,
) {
    if refreshes_staged {
        *cached = match scanned {
            Ok(head) => Some(head),
            Err(err) => {
                tracing::warn!(error = %err, "could not remember HEAD for worktree status");
                None
            }
        };
    }
}

#[cfg(test)]
fn worktree_watch_directories(repository: &gix::Repository) -> Result<HashSet<PathBuf>> {
    let index = repository
        .index_or_empty()
        .context("could not open index for worktree watcher")?;
    worktree_watch_directories_with_index(repository, &index)
}

fn worktree_watch_directories_with_index(
    repository: &gix::Repository,
    index: &gix::index::State,
) -> Result<HashSet<PathBuf>> {
    let root = repository
        .workdir()
        .context("cannot walk a bare repository")?
        .to_owned();
    let options = repository
        .dirwalk_options()
        .context("could not configure worktree directory walk")?;
    let mut directories = WorktreeDirectories {
        root: root.clone(),
        paths: HashSet::from([root]),
    };
    repository
        .dirwalk(index, None::<&str>, &AtomicBool::default(), options, &mut directories)
        .context("could not enumerate worktree directories")?;
    Ok(directories.paths)
}

fn index_watch_projection(index: &gix::index::State) -> Vec<IndexWatchEntry> {
    let mut out: Vec<_> = index
        .entries()
        .iter()
        .map(|entry| IndexWatchEntry {
            path: entry.path(index).to_owned(),
            mode: entry.mode.bits(),
            flags: entry.flags.bits(),
        })
        .collect();
    out.sort_unstable();
    out
}

fn changed_index_watch_scopes(
    before: &[IndexWatchEntry],
    after: &[IndexWatchEntry],
    workdir: &Path,
) -> HashSet<PathBuf> {
    fn add_scope(entry: &IndexWatchEntry, workdir: &Path, out: &mut HashSet<PathBuf>) {
        let path = entry.path.as_bstr();
        let scope = path
            .find_byte(b'/')
            .map(|pos| &path[..pos])
            .or_else(|| (entry.mode == gix::index::entry::Mode::DIR.bits()).then_some(path.as_ref()));
        if let Some(scope) = scope {
            out.insert(workdir.join(gix::path::from_bstr(scope)));
        }
    }

    let mut out = HashSet::new();
    let (mut left, mut right) = (0, 0);
    while left < before.len() || right < after.len() {
        match (before.get(left), after.get(right)) {
            (Some(a), Some(b)) => match a.cmp(b) {
                std::cmp::Ordering::Less => {
                    add_scope(a, workdir, &mut out);
                    left += 1;
                }
                std::cmp::Ordering::Greater => {
                    add_scope(b, workdir, &mut out);
                    right += 1;
                }
                std::cmp::Ordering::Equal => {
                    left += 1;
                    right += 1;
                }
            },
            (Some(a), None) => {
                add_scope(a, workdir, &mut out);
                left += 1;
            }
            (None, Some(b)) => {
                add_scope(b, workdir, &mut out);
                right += 1;
            }
            (None, None) => break,
        }
    }
    out
}

fn minimize_worktree_scopes(scopes: HashSet<PathBuf>) -> Vec<PathBuf> {
    let mut scopes: Vec<_> = scopes.into_iter().collect();
    scopes.sort_by(|a, b| {
        a.components()
            .count()
            .cmp(&b.components().count())
            .then_with(|| a.cmp(b))
    });
    let mut out: Vec<PathBuf> = Vec::new();
    for scope in scopes {
        if !out.iter().any(|parent| scope.starts_with(parent)) {
            out.push(scope);
        }
    }
    out
}

fn reconcile_worktree_watcher(
    watcher: &mut WorktreeWatcher,
    repository_path: &Path,
    bare: bool,
    mut refresh: WorktreeWatchRefresh,
) -> Result<(usize, usize)> {
    let repository = open_repository(repository_path, bare, false)
        .context("could not reopen repository to update worktree watches")?;
    let index = repository
        .index_or_empty()
        .context("could not open index to update worktree watches")?;
    let next_projection = index_watch_projection(&index);
    let update_projection = refresh.index || refresh.full;
    if refresh.index {
        refresh.scopes.extend(changed_index_watch_scopes(
            &watcher.index_projection,
            &next_projection,
            &watcher.workdir,
        ));
    }
    if !refresh.full && refresh.scopes.is_empty() {
        if update_projection {
            watcher.index_projection = next_projection;
        }
        return Ok((0, 0));
    }

    let all_desired = worktree_watch_directories_with_index(&repository, &index)?;
    let scopes = minimize_worktree_scopes(refresh.scopes);
    let mut desired = if refresh.full {
        all_desired
    } else {
        watcher
            .directories
            .iter()
            .filter(|path| !scopes.iter().any(|scope| path.starts_with(scope)))
            .cloned()
            .chain(
                all_desired
                    .into_iter()
                    .filter(|path| scopes.iter().any(|scope| path.starts_with(scope))),
            )
            .collect()
    };
    desired.insert(watcher.index.parent().context("index path has no parent")?.to_owned());
    let changed = update_worktree_watch_paths(watcher, desired)?;
    if update_projection {
        watcher.index_projection = next_projection;
    }
    Ok(changed)
}

fn update_worktree_watch_paths(watcher: &mut WorktreeWatcher, desired: HashSet<PathBuf>) -> Result<(usize, usize)> {
    let mut remove: Vec<_> = watcher.directories.difference(&desired).cloned().collect();
    let mut add: Vec<_> = desired.difference(&watcher.directories).cloned().collect();
    if remove.is_empty() && add.is_empty() {
        return Ok((0, 0));
    }
    remove.sort_by(|a, b| {
        b.components()
            .count()
            .cmp(&a.components().count())
            .then_with(|| a.cmp(b))
    });
    add.sort_by(|a, b| {
        a.components()
            .count()
            .cmp(&b.components().count())
            .then_with(|| a.cmp(b))
    });
    let removed = remove.len();
    let added = add.len();
    let mut first_error = None;
    let mut paths = watcher.watcher.paths_mut();
    for path in remove {
        if let Err(err) = paths.remove(&path)
            && !matches!(&err.kind, notify::ErrorKind::WatchNotFound)
            && first_error.is_none()
        {
            first_error = Some(err);
        }
    }
    for path in add {
        if let Err(err) = paths.add(&path, RecursiveMode::NonRecursive)
            && first_error.is_none()
        {
            first_error = Some(err);
        }
    }
    if let Err(err) = paths.commit()
        && first_error.is_none()
    {
        first_error = Some(err);
    }
    if let Some(err) = first_error {
        return Err(err).context("could not update worktree watches");
    }
    tracing::debug!(removed, added, "updated worktree watches");
    watcher.directories = desired;
    Ok((removed, added))
}

fn invalidate_worktree_changes(changes: &mut Option<(usize, Changes)>) -> bool {
    if let Some((marker, _)) = changes {
        if *marker == WORKTREE_STATUS_FULL {
            return false;
        }
        *marker = WORKTREE_STATUS_FULL;
        return true;
    }
    false
}

fn invalidate_worktree_status_parts(
    changes: &mut Option<(usize, Changes)>,
    parts: &mut WorktreeStatusParts,
    staged: bool,
    scopes: impl IntoIterator<Item = BString>,
) -> bool {
    let Some((marker, _)) = changes else {
        return false;
    };
    if *marker == WORKTREE_STATUS_FULL {
        return false;
    }
    parts.staged |= staged;
    parts.scopes.extend(scopes);
    *marker = WORKTREE_STATUS_PARTIAL;
    true
}

fn leave_recorded_success(
    app: &mut App,
    repository_path: &Path,
    bare: bool,
    title: &str,
    changes: &[edit::undo::RefChange],
    message: impl Into<String>,
) {
    let message = message.into();
    match record_undo(repository_path, bare, title, changes) {
        Ok(()) => app.leave_success(message),
        Err(err) => app.leave_attention(format!("{message}; undo history: {err:#}")),
    }
}

fn record_undo(repository_path: &Path, bare: bool, title: &str, changes: &[edit::undo::RefChange]) -> Result<()> {
    open_repository(repository_path, bare, false)
        .context("could not reopen repository for undo history")
        .and_then(|repo| edit::undo::record(&repo, title, changes).map(|_| ()))
}

fn clear_undo_history(repository_path: &Path, bare: bool) -> Result<()> {
    open_repository(repository_path, bare, false)
        .context("could not reopen repository to clear undo history")
        .and_then(|repo| edit::undo::clear(&repo))
}

fn record_and_clear_pending_undo(
    repository_path: &Path,
    bare: bool,
    title: &str,
    changes: &mut Vec<edit::undo::RefChange>,
) -> Result<()> {
    let result = record_undo(repository_path, bare, title, changes);
    changes.clear();
    result
}

fn conflict_head(repository_path: &Path, bare: bool, commit: gix::ObjectId) -> Result<ConflictHead> {
    let repository = open_repository(repository_path, bare, false)
        .context("could not reopen the repository after checking out a conflict")?;
    let head = repository.head().context("could not inspect the conflicted HEAD")?;
    let id = head
        .id()
        .map(gix::Id::detach)
        .context("the conflicted HEAD is unborn")?;
    anyhow::ensure!(id == commit, "the conflict checkout did not leave HEAD at {commit}");
    let reference = head.referent_name().map(ToOwned::to_owned);
    drop(head);
    let name = reference
        .clone()
        .unwrap_or_else(|| "HEAD".try_into().expect("valid reference name"));
    anyhow::ensure!(
        edit::undo::state(&repository, name.as_ref())? == edit::undo::State::Object(commit),
        "the conflicted HEAD attachment does not directly reference {commit}"
    );
    let parents = repository
        .find_commit(commit)
        .context("could not find the checked-out conflict commit")?
        .parent_ids()
        .map(gix::Id::detach)
        .collect();
    Ok(ConflictHead { reference, parents })
}

fn reconcile_external_conflict(
    repository_path: &Path,
    bare: bool,
    pending: &mut Option<PendingConflictResolution>,
) -> Result<ExternalConflictResolution> {
    let state = pending
        .as_ref()
        .context("external conflict reconciliation requires pending state")?;
    let Some(expected) = state.head.as_ref() else {
        return Ok(ExternalConflictResolution::Current);
    };
    let repository =
        open_repository(repository_path, bare, false).context("could not inspect external conflict resolution")?;
    let head = repository.head().context("could not inspect HEAD after the conflict")?;
    let reference = head.referent_name().map(ToOwned::to_owned);
    anyhow::ensure!(
        reference == expected.reference,
        "HEAD attachment changed while resolving the conflict; return to the conflict checkout or exit"
    );
    let replacement = head
        .id()
        .map(gix::Id::detach)
        .context("HEAD became unborn while resolving the conflict")?;
    drop(head);
    if replacement == state.commit {
        return Ok(ExternalConflictResolution::Current);
    }

    let replacement_commit = repository
        .find_commit(replacement)
        .context("the replacement HEAD is not a commit")?
        .decode()
        .context("could not decode the replacement HEAD commit")?
        .into_owned()
        .context("could not own the replacement HEAD commit")?;
    anyhow::ensure!(
        replacement_commit.parents.as_slice() == expected.parents,
        "HEAD moved to an unrelated commit while resolving the conflict; return to the conflict checkout or exit"
    );
    let index = repository
        .open_index()
        .context("could not inspect the conflict index")?;
    if index
        .entries()
        .iter()
        .any(|entry| entry.stage() != gix::index::entry::Stage::Unconflicted)
    {
        return Ok(ExternalConflictResolution::Changed);
    }
    if edit::create::index_tree(&repository, &index)? != replacement_commit.tree {
        return Ok(ExternalConflictResolution::Changed);
    }

    let name = expected
        .reference
        .clone()
        .unwrap_or_else(|| "HEAD".try_into().expect("valid reference name"));
    anyhow::ensure!(
        edit::undo::state(&repository, name.as_ref())? == edit::undo::State::Object(replacement),
        "HEAD no longer directly references its replacement commit"
    );
    let finalized = if edit::rebase::is_pending(&replacement_commit) {
        drop(index);
        let graph = edit::loaded_graph(&repository).context("could not load history to finalize the external amend")?;
        let outcome = edit::head::amend_index_reporting(repository, &graph)
            .context("could not finalize the externally amended pending commit")?
            .context("the externally amended pending commit was not finalized")?;
        let selected = outcome
            .selected
            .context("finalizing the externally amended conflict did not select its result")?;
        Some((selected, outcome.ref_changes))
    } else {
        None
    };
    let accepted = state.commit;
    let mut state = pending
        .take()
        .expect("the pending conflict was inspected immediately before completion");
    state.ref_changes.push(edit::undo::RefChange {
        name,
        before: edit::undo::State::Object(accepted),
        after: edit::undo::State::Object(replacement),
    });
    let replacement = match finalized {
        Some((selected, changes)) => {
            state.ref_changes.extend(changes);
            selected
        }
        None => replacement,
    };
    Ok(ExternalConflictResolution::Complete(
        replacement,
        state.ref_changes,
        state.record_undo,
    ))
}

fn reconcile_external_conflict_reporting(
    app: &mut App,
    repository_path: &Path,
    bare: bool,
    pending: &mut Option<PendingConflictResolution>,
) -> ConflictReconcileStatus {
    if pending.is_none() {
        return ConflictReconcileStatus::Inactive;
    }
    match reconcile_external_conflict(repository_path, bare, pending) {
        Ok(ExternalConflictResolution::Complete(replacement, changes, record_undo)) => {
            let message = format!("resolved rebase conflict as {}", replacement.to_hex_with_len(7));
            if record_undo {
                leave_recorded_success(app, repository_path, bare, "resolve rebase conflict", &changes, message);
            } else {
                app.leave_success(message);
            }
            app.clear_rebase_conflict();
            app.set_worktree_conflicted(false);
            app.select_commit_after_refresh(replacement);
            tracing::info!(commit_id = %replacement, "recognized externally amended conflict resolution");
            ConflictReconcileStatus::Complete
        }
        Ok(ExternalConflictResolution::Current) => ConflictReconcileStatus::Amend,
        Ok(ExternalConflictResolution::Changed) => {
            app.leave_attention(
                "external conflict resolution is incomplete; finish it, return to the conflict checkout, or press q",
            );
            ConflictReconcileStatus::Blocked
        }
        Err(err) => {
            app.leave_attention(format!("conflict resolution remains pending: {err:#}"));
            ConflictReconcileStatus::Blocked
        }
    }
}

fn tracked_ref_update<T>(
    repo: &gix::Repository,
    name: &str,
    update: impl FnOnce(&gix::Repository) -> Result<T>,
) -> Result<(T, Result<Vec<edit::undo::RefChange>>)> {
    let name: gix::refs::FullName = name.try_into().context("tracked reference name is invalid")?;
    let before = edit::undo::state(repo, name.as_ref());
    let value = update(repo)?;
    let changes = before.and_then(|before| {
        edit::undo::state(repo, name.as_ref()).map(|after| {
            (before != after)
                .then_some(edit::undo::RefChange { name, before, after })
                .into_iter()
                .collect()
        })
    });
    Ok((value, changes))
}

fn leave_tracked_success(
    app: &mut App,
    repository_path: &Path,
    bare: bool,
    title: &str,
    changes: Result<Vec<edit::undo::RefChange>>,
    message: impl Into<String>,
) {
    let message = message.into();
    match changes {
        Ok(changes) => leave_recorded_success(app, repository_path, bare, title, &changes, message),
        Err(err) => app.leave_attention(format!("{message}; undo history: {err:#}")),
    }
}

fn remembered_change_selection(view: &app::ChangesView, changes: Option<&Changes>) -> Option<(BString, usize)> {
    changes.and_then(|changes| {
        changes
            .paths
            .get(view.selected)
            .map(|change| (change.path.clone(), view.selected.saturating_sub(view.offset)))
    })
}

fn decoration_head(decorations: &Decorations) -> Option<gix::ObjectId> {
    decorations.iter().find_map(|(id, decorations)| {
        decorations
            .iter()
            .any(|decoration| decoration.kind == history::DecorationKind::Head)
            .then_some(*id)
    })
}

fn current_worktree_branch(refs: &history::RefSnapshot) -> Option<(gix::ObjectId, bool)> {
    refs.worktrees
        .iter()
        .find(|worktree| {
            worktree.is_current
                && worktree
                    .reference
                    .as_ref()
                    .is_some_and(|reference| reference.as_bstr().starts_with(b"refs/heads/"))
        })
        .map(|worktree| (worktree.label_id, worktree.is_detached))
}

fn decoration_successor(selected: gix::ObjectId, current: &Decorations, next: &Decorations) -> Option<gix::ObjectId> {
    let selected = current.get(&selected)?;
    let mut matches = next.iter().filter_map(|(id, decorations)| {
        decorations
            .iter()
            .any(|decoration| selected.contains(decoration))
            .then_some(*id)
    });
    let successor = matches.next()?;
    matches.all(|candidate| candidate == successor).then_some(successor)
}

fn update_hidden_branch_updates(app: &mut App, graph: Option<&HistoryGraph>, refs: &history::RefSnapshot) {
    let updates = graph.map_or_else(HashMap::new, |graph| {
        graph.hidden_branch_updates(
            &refs.view_tips,
            refs.hidden
                .iter()
                .filter(|(name, _)| name.starts_with(b"refs/heads/"))
                .filter_map(|(_, target)| target.try_id().map(ToOwned::to_owned)),
        )
    });
    app.set_hidden_branch_updates(updates);
}

fn restore_change_selection(view: &mut app::ChangesView, changes: &Changes, remembered: Option<(BString, usize)>) {
    let Some((path, viewport_row)) = remembered else {
        return;
    };
    if let Some(selected) = changes.paths.iter().position(|change| change.path == path) {
        view.selected = selected;
        view.offset = selected.saturating_sub(viewport_row);
    }
}

fn load_visible_history_metadata(
    repository: &gix::Repository,
    app: &mut App,
    authors: &SharedAuthors,
    render_rows: usize,
) -> Result<()> {
    let start = app.offset.min(app.history_len());
    let end = start.saturating_add(render_rows).min(app.history_len());
    for index in app.visible_history_indices(start..end) {
        if app.rows[index].metadata_loaded {
            continue;
        }
        let (metadata, attributions) =
            history::load_metadata(repository, app.rows[index].id, authors).context("could not load visible commit")?;
        app.set_metadata(index, metadata, attributions);
    }
    Ok(())
}

#[expect(clippy::too_many_arguments, reason = "drawing needs the complete view state")]
fn draw(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    command_picker: &mut Menu<CommandId>,
    decorations: &Decorations,
    mailmap: &gix::mailmap::Snapshot,
    authors: &SharedAuthors,
    fill_repository: &mut FillRepository,
    commit_message: &mut Option<(gix::ObjectId, BString)>,
    tree_changes: &mut TreeChangesCache,
    worktree_changes: &mut Option<(usize, Changes)>,
    status_head: &mut Option<WorktreeStatusHead>,
    status_parts: &mut WorktreeStatusParts,
    history_graph: &mut Option<HistoryGraph>,
    selection_cache: &mut Option<SelectionRelationCache>,
    line_diff_pool: &mut Option<LineDiffPool>,
    focused: bool,
    ref_tree: &mut ref_tree::Tree,
    filesystem_responses: &mut logging::FilesystemResponses,
) -> Result<()> {
    let render_rows = terminal.get_frame().area().height.saturating_sub(1) as usize;
    if !history_is_ready_to_draw(app.state, app.rows.len()) {
        return Ok(());
    }
    if ref_tree.is_active() {
        terminal
            .autoresize()
            .context("could not resize the terminal before drawing")?;
        {
            let mut frame = terminal.get_frame();
            ref_tree.draw(&mut frame, history_graph.as_ref());
        }
        terminal
            .apply_buffer_with_cursor(None)
            .context("could not draw ref-tree overview")?;
        filesystem_responses.frame_presented();
        return Ok(());
    }
    app.unseen_filesystem_redraw = unseen_filesystem_redraw(
        app.unseen_filesystem_redraw,
        focused,
        filesystem_responses.has_queued_frame(),
    );
    if let Some((_, changes)) = worktree_changes.as_ref() {
        app.set_worktree_conflicted(changes.paths.iter().any(|change| change.kind == ChangeKind::Unmerged));
    }
    app.viewport_rows = app.viewport_rows.min(render_rows.max(1));
    app.prepare_history_viewport();
    let start = app.offset.min(app.history_len());
    let end = start.saturating_add(render_rows).min(app.history_len());
    let visible_indices = app.visible_history_indices(start..end);
    let repository_fill_allowed = !app.has_rebase_conflict();
    let notes_to_load: Vec<_> = visible_indices
        .iter()
        .map(|index| app.rows[*index].id)
        .filter(|id| repository_fill_allowed && !app.notes_loaded(*id))
        .collect();
    let enrichments_to_load: Vec<_> = visible_indices
        .iter()
        .map(|index| app.rows[*index].id)
        .filter(|id| repository_fill_allowed && !app.enrichment_loaded(*id))
        .collect();
    let tree_enrichments_to_load: Vec<_> = visible_indices
        .iter()
        .map(|index| app.rows[*index].id)
        .filter(|id| repository_fill_allowed && !app.tree_enrichment_loaded(*id))
        .collect();
    let changes_visible = app.changes_visible();
    let selected_id = app.selected.and_then(|index| app.rows.get(index)).map(|row| row.id);
    app.selection_relation = selection_cache
        .as_ref()
        .filter(|cached| Some(cached.id) == selected_id)
        .and_then(|cached| cached.relation);
    let relation_to_load = matches!(app.state, State::Complete | State::Cancelled)
        .then_some(selected_id)
        .flatten()
        .filter(|id| selection_cache.as_ref().is_none_or(|cached| cached.id != *id));
    let selected = (app.show_commit || app.changes_mode.is_some())
        .then_some(selected_id)
        .flatten();
    let message_to_load = app
        .show_commit
        .then_some(selected)
        .flatten()
        .filter(|_| repository_fill_allowed)
        .filter(|id| commit_message.as_ref().map(|(cached, _)| cached) != Some(id));
    if message_to_load.is_some() {
        app.reset_commit_view();
    }
    if changes_visible && selected.is_some() && tree_changes.as_ref().map(|(target, _)| target.selected()) != selected {
        app.changes_parent = 0;
    }
    let desired_tree_changes = (changes_visible && app.changes_mode.is_some())
        .then(|| app.selected_tree_diff_target())
        .flatten();
    let tree_changes_changed =
        desired_tree_changes.is_some_and(|target| tree_changes.as_ref().is_none_or(|(cached, _)| *cached != target));
    let tree_selection = tree_changes_changed
        .then(|| remembered_change_selection(&app.tree_changes, tree_changes.as_ref().map(|(_, changes)| changes)))
        .flatten();
    let tree_changes_to_load = desired_tree_changes.filter(|target| !tree_changes.activate(*target));
    if tree_changes_changed
        && tree_changes_to_load.is_none()
        && let Some(changes) = tree_changes.as_ref().map(|(_, changes)| changes)
    {
        restore_change_selection(&mut app.tree_changes, changes, tree_selection.clone());
    }
    let worktree_changes_to_load = changes_visible
        && app.changes_mode == Some(ChangesMode::Both)
        && worktree_changes
            .as_ref()
            .is_none_or(|(marker, _)| *marker != WORKTREE_STATUS_CURRENT);
    let worktree_selection = worktree_changes_to_load
        .then(|| {
            remembered_change_selection(
                &app.worktree_changes,
                worktree_changes.as_ref().map(|(_, changes)| changes),
            )
        })
        .flatten();
    if !app.show_commit
        || selected.is_none()
        || (!repository_fill_allowed && commit_message.as_ref().map(|(id, _)| *id) != selected)
    {
        *commit_message = None;
    }
    if app.changes_mode.is_none() {
        tree_changes.clear();
        *worktree_changes = None;
        *status_head = None;
        *status_parts = WorktreeStatusParts::default();
    }
    if let Some(id) = relation_to_load
        && let Some(graph) = history_graph
    {
        let refs = graph.selection_refs(id, decorations);
        let hidden: Vec<_> = app.hidden_ids().into_iter().collect();
        let relation = graph.selection_relation(id, &refs, &hidden);
        *selection_cache = Some(SelectionRelationCache { id, refs, relation });
        app.selection_relation = relation;
    }
    if !notes_to_load.is_empty()
        || !enrichments_to_load.is_empty()
        || !tree_enrichments_to_load.is_empty()
        || visible_indices.iter().any(|index| !app.rows[*index].metadata_loaded)
        || message_to_load.is_some()
        || tree_changes_to_load.is_some()
        || worktree_changes_to_load
    {
        let mut one_shot_repository = None;
        let repository = if fill_repository.retain {
            match &mut fill_repository.retained {
                Some(repository) => repository,
                slot @ None => slot.insert(open_fill_repository(&fill_repository.path, fill_repository.bare)?),
            }
        } else {
            one_shot_repository.insert(open_fill_repository(&fill_repository.path, fill_repository.bare)?)
        };
        if !notes_to_load.is_empty() {
            let mut notes = repository.notes().context("could not open Git notes")?;
            for id in notes_to_load {
                let loaded = notes
                    .get(id)
                    .context("could not load visible commit notes")?
                    .into_iter()
                    .map(|note| {
                        let mut blob = note.blob;
                        BString::from(blob.take_data())
                    })
                    .collect();
                app.set_notes(id, loaded);
            }
        }
        if !enrichments_to_load.is_empty() {
            let mut notes = enrich::open(repository)?;
            for id in enrichments_to_load {
                let loaded = crate::change_id::for_commit(repository, id)
                    .and_then(|change_id| enrich::load(&mut notes, change_id));
                match loaded {
                    Ok(enrichment) => app.set_enrichment(id, enrichment),
                    Err(err) => {
                        tracing::warn!(commit_id = %id, error = %err, "ignored malformed tix enrichment");
                        app.set_enrichment(id, enrich::Enrichment::default());
                    }
                }
            }
        }
        if !tree_enrichments_to_load.is_empty() {
            let mut notes = enrich::open_tree(repository)?;
            for id in tree_enrichments_to_load {
                let loaded = enrich::tree_id(repository, id).and_then(|tree_id| enrich::load_tree(&mut notes, tree_id));
                match loaded {
                    Ok(enrichment) => app.set_tree_enrichment(id, enrichment),
                    Err(err) => {
                        tracing::warn!(commit_id = %id, error = %err, "ignored malformed tix tree enrichment");
                        app.set_tree_enrichment(id, enrich::TreeEnrichment::default());
                    }
                }
            }
        }
        if let Some(id) = message_to_load {
            *commit_message = Some((id, load_commit_message(repository, id)?));
        }
        if let Some(target) = tree_changes_to_load {
            repository.object_cache_size(OBJECT_CACHE_SIZE);
            let loaded = load_changes(
                repository,
                target,
                line_diff_pool
                    .as_mut()
                    .context("line diff pool is missing while the changes pane is visible")?,
            );
            repository.object_cache_size(None);
            let loaded = loaded?;
            app.changes_parent = loaded.parent.map_or(0, |parent| parent.index);
            restore_change_selection(&mut app.tree_changes, &loaded, tree_selection);
            tree_changes.insert((target, loaded));
        }
        if worktree_changes_to_load {
            let started = Instant::now();
            repository.object_cache_size(OBJECT_CACHE_SIZE);
            let line_diff_pool = line_diff_pool
                .as_mut()
                .context("line diff pool is missing while the changes pane is visible")?;
            let partial = worktree_changes
                .as_ref()
                .is_some_and(|(marker, _)| *marker == WORKTREE_STATUS_PARTIAL);
            let refreshes_staged = !partial || status_parts.staged;
            let status_head_before = worktree_status_head(repository);
            let loaded = if partial {
                let mut updated = worktree_changes
                    .as_ref()
                    .map(|(_, changes)| changes.clone())
                    .expect("partial status requires cached changes");
                update_worktree_changes(repository, &mut updated, status_parts, line_diff_pool)
                    .map(|full| (updated, refreshes_staged || full))
            } else {
                load_worktree_changes(repository, line_diff_pool).map(|loaded| (loaded, true))
            };
            repository.object_cache_size(None);
            *status_parts = WorktreeStatusParts::default();
            match loaded {
                Ok((loaded, refreshes_staged)) => {
                    tracing::debug!(
                        partial,
                        path_count = loaded.paths.len(),
                        elapsed_ms = started.elapsed().as_millis(),
                        "loaded worktree changes"
                    );
                    if !app
                        .worktree_changes
                        .error
                        .as_deref()
                        .is_some_and(|message| message.starts_with("worktree watch:"))
                    {
                        app.worktree_changes.error = None;
                    }
                    app.set_worktree_conflicted(loaded.paths.iter().any(|change| change.kind == ChangeKind::Unmerged));
                    restore_change_selection(&mut app.worktree_changes, &loaded, worktree_selection);
                    *worktree_changes = Some((WORKTREE_STATUS_CURRENT, loaded));
                    remember_worktree_status_head(status_head, refreshes_staged, status_head_before);
                }
                Err(err) => {
                    tracing::warn!(error = %err, "could not load worktree changes");
                    app.worktree_changes.error = Some(format!("status: {err:#}"));
                    if let Some((marker, _)) = worktree_changes.as_mut() {
                        *marker = WORKTREE_STATUS_CURRENT;
                    } else {
                        *worktree_changes = Some((WORKTREE_STATUS_CURRENT, Changes::default()));
                    }
                }
            }
        }
        load_visible_history_metadata(repository, app, authors, render_rows)?;
    }
    let message = commit_message.as_ref().map(|(_, message)| message.as_bstr());
    let tree_changes = tree_changes.as_ref().map(|(_, changes)| changes);
    let worktree_changes = worktree_changes
        .as_ref()
        .filter(|(marker, _)| *marker == WORKTREE_STATUS_CURRENT)
        .map(|(_, changes)| changes);
    terminal
        .autoresize()
        .context("could not resize the terminal before drawing")?;
    let cursor = {
        let mut frame = terminal.get_frame();
        ui::draw_with_worktree(
            &mut frame,
            app,
            decorations,
            mailmap,
            message,
            tree_changes,
            worktree_changes,
        );
        if command_picker.is_open() {
            let commands = command_menu::commands(app, decorations, app.has_verifiable_signatures());
            let items = command_picker_items(&commands);
            command_picker.sync(&items);
            ui::draw_command_menu(&mut frame, command_picker, &commands)
        } else {
            None
        }
    };
    if matches!(app.state, State::Complete | State::Cancelled) {
        let response_ids = filesystem_responses.active_reference_ids().to_vec();
        filesystem_responses.finish_after_frame(&response_ids, "completed");
    }
    terminal
        .apply_buffer_with_cursor(cursor)
        .context("could not draw terminal frame")?;
    filesystem_responses.frame_presented();
    Ok(())
}

fn open_repository(repository_path: &Path, bare: bool, isolated: bool) -> Result<gix::Repository> {
    let options = if isolated {
        gix::open::Options::isolated()
    } else {
        gix::open::Options::default()
    }
    .open_path_as_is(bare);
    let options = if bare {
        options.cli_overrides(["core.bare=true"])
    } else {
        options
    };
    Ok(gix::open_opts(repository_path, options)?)
}

fn open_history_repository(repository_path: &mut PathBuf, common_dir: &Path) -> Result<(gix::Repository, bool)> {
    match gix::open(&*repository_path) {
        Ok(repository) => Ok((repository, false)),
        Err(_err) if worktree_repository_is_gone(repository_path) => {
            let repository = recover_common_repository(common_dir)
                .context("could not recover before history traversal after the worktree repository disappeared")?;
            common_dir.clone_into(repository_path);
            Ok((repository, true))
        }
        Err(err) => Err(err).context("could not open repository for history view"),
    }
}

fn recover_common_repository(common_dir: &Path) -> Result<gix::Repository> {
    std::env::set_current_dir(common_dir).with_context(|| {
        format!(
            "could not change directory to common repository at {}",
            common_dir.display()
        )
    })?;
    open_repository(common_dir, true, false)
        .with_context(|| format!("could not open common repository at {} as bare", common_dir.display()))
}

fn recover_event_loop_repository(
    repository_path: &mut PathBuf,
    common_dir: &Path,
    bare: &mut bool,
) -> Result<Option<gix::Repository>> {
    if *bare || !worktree_repository_is_gone(repository_path) {
        return Ok(None);
    }
    let repository =
        recover_common_repository(common_dir).context("could not recover after the worktree repository disappeared")?;
    common_dir.clone_into(repository_path);
    *bare = true;
    Ok(Some(repository))
}

fn normalize_common_dir(common_dir: PathBuf) -> Result<PathBuf> {
    let current_dir = std::env::current_dir().context("could not obtain current directory")?;
    gix::path::normalize(common_dir.into(), &current_dir)
        .map(Into::into)
        .context("common repository path could not be normalized")
}

fn worktree_repository_is_gone(repository_path: &Path) -> bool {
    !repository_path.is_dir() || std::env::current_dir().is_err()
}

fn open_fill_repository(repository_path: &Path, bare: bool) -> Result<gix::Repository> {
    let mut repository =
        open_repository(repository_path, bare, false).context("could not open repository for history view")?;
    repository.object_cache_size(None);
    Ok(repository)
}

fn prepare_file_diff(repository_path: &Path, bare: bool, change: &FileChange, path: &PathChange) -> Result<FileDiff> {
    let mut repository =
        open_repository(repository_path, bare, false).context("could not open repository for file diff")?;
    repository.object_cache_size(OBJECT_CACHE_SIZE);
    prepare_file_diff_with_repository(&repository, change, path)
}

fn prepare_commit_diff(
    repository_path: &Path,
    bare: bool,
    target: app::TreeDiffTarget,
    cached: Option<&Changes>,
    title: BString,
) -> Result<CommitDiff> {
    let mut repository =
        open_repository(repository_path, bare, false).context("could not open repository for commit diff")?;
    repository.object_cache_size(OBJECT_CACHE_SIZE);
    prepare_commit_diff_with_repository(&repository, target, cached, title)
}

fn prepare_commit_diff_with_repository(
    repository: &gix::Repository,
    target: app::TreeDiffTarget,
    cached: Option<&Changes>,
    title: BString,
) -> Result<CommitDiff> {
    let loaded = cached
        .is_none()
        .then(|| load_changes_without_lines(repository, target))
        .transpose()?;
    let changes = cached
        .or(loaded.as_ref())
        .context("commit diff changes were neither cached nor loaded")?;
    let mut external = Vec::new();
    let mut lines = Vec::new();
    let mut lines_added = 0u64;
    let mut lines_removed = 0u64;
    let mut line_counts = Vec::with_capacity(changes.paths.len());
    for (change, path) in changes.diffs.iter().zip(&changes.paths) {
        let counts = match prepare_file_diff_content(repository, change, path, true)? {
            PreparedFileDiff::External(command, counts) => {
                external.push(command);
                counts
            }
            PreparedFileDiff::BuiltIn(diff, counts) => {
                lines.extend(diff.lines);
                counts
            }
        };
        if let Some((added, removed)) = counts {
            lines_added += u64::from(added);
            lines_removed += u64::from(removed);
        }
        line_counts.push(counts);
    }
    let summary = ui::commit_diff_summary(changes, &line_counts, lines_added, lines_removed);
    let internal = prepare_pager(repository, BuiltInDiff::new(title, lines).with_summary(summary))?;
    Ok(CommitDiff { external, internal })
}

fn prepare_file_diff_with_repository(
    repository: &gix::Repository,
    change: &FileChange,
    path: &PathChange,
) -> Result<FileDiff> {
    match prepare_file_diff_content(repository, change, path, false)? {
        PreparedFileDiff::External(command, _) => Ok(FileDiff::External(command)),
        PreparedFileDiff::BuiltIn(diff, _) => prepare_pager(repository, diff),
    }
}

fn prepare_file_diff_content(
    repository: &gix::Repository,
    change: &FileChange,
    path: &PathChange,
    count_lines: bool,
) -> Result<PreparedFileDiff> {
    if let FileChange::Unavailable(message) = change {
        anyhow::bail!("{message}");
    }
    let global_command = repository
        .config_snapshot()
        .trusted_program(gix::config::tree::Diff::EXTERNAL)
        .map(gix::path::os_string_into_bstring)
        .transpose()
        .context("external diff command is not representable on this platform")?;
    let mut resources = match change {
        FileChange::Tree(_) => repository
            .diff_resource_cache(
                gix::diff::blob::pipeline::Mode::ToGitUnlessBinaryToTextIsPresent,
                Default::default(),
            )
            .context("could not initialize file diff")?,
        FileChange::Worktree { .. } => worktree_diff_cache(
            repository,
            gix::diff::blob::pipeline::Mode::ToGitUnlessBinaryToTextIsPresent,
        )?
        .context("a working tree is required to show this diff")?,
        FileChange::Unavailable(_) => unreachable!("handled above"),
    };
    resources.options.skip_internal_diff_if_external_is_configured = true;
    match change {
        FileChange::Tree(change) => {
            change
                .attach(repository, repository)
                .diff(&mut resources)
                .context("could not prepare selected file")?;
        }
        FileChange::Worktree { old, new } => {
            set_worktree_resources(repository, &mut resources, old.as_ref(), new.as_ref())?;
        }
        FileChange::Unavailable(_) => unreachable!("handled above"),
    }
    let prepared = resources.prepare_diff().context("could not prepare selected diff")?;
    match prepared.operation {
        gix::diff::blob::platform::prepare_diff::Operation::ExternalCommand { command } => {
            let counts = count_lines
                .then(|| {
                    let input = prepared.interned_input();
                    let diff = gix::diff::blob::diff_with_slider_heuristics(
                        repository.diff_algorithm().context("could not obtain diff algorithm")?,
                        &input,
                    );
                    Ok::<_, anyhow::Error>((diff.count_additions(), diff.count_removals()))
                })
                .transpose()?;
            let command = command.to_owned();
            prepare_external_diff(repository, &resources, command)
                .map(|command| PreparedFileDiff::External(command, counts))
        }
        gix::diff::blob::platform::prepare_diff::Operation::InternalDiff { algorithm } => {
            if let Some(command) = global_command {
                let counts = count_lines.then(|| {
                    let input = prepared.interned_input();
                    let diff = gix::diff::blob::diff_with_slider_heuristics(algorithm, &input);
                    (diff.count_additions(), diff.count_removals())
                });
                return prepare_external_diff(repository, &resources, command)
                    .map(|command| PreparedFileDiff::External(command, counts));
            }
            let input = prepared.interned_input();
            let diff = gix::diff::blob::diff_with_slider_heuristics(algorithm, &input);
            let counts = Some((diff.count_additions(), diff.count_removals()));
            let rendered = gix::diff::blob::UnifiedDiff::new(
                &diff,
                &input,
                gix::diff::blob::unified_diff::ConsumeBinaryHunk::new(BString::default(), "\n"),
                gix::diff::blob::unified_diff::ContextSize::symmetrical(3),
            )
            .consume()
            .context("could not render selected diff")?;
            Ok(PreparedFileDiff::BuiltIn(
                built_in_diff(path, change, Some(rendered), false),
                counts,
            ))
        }
        gix::diff::blob::platform::prepare_diff::Operation::SourceOrDestinationIsBinary => {
            Ok(PreparedFileDiff::BuiltIn(built_in_diff(path, change, None, true), None))
        }
    }
}

fn prepare_pager(repository: &gix::Repository, diff: BuiltInDiff) -> Result<FileDiff> {
    let Some(program) = repository.config_snapshot().trusted_program("core.pager") else {
        return Ok(FileDiff::BuiltIn(diff));
    };
    if program.is_empty() || program == "cat" {
        return Ok(FileDiff::BuiltIn(diff));
    }
    let command = gix::command::prepare(program)
        .command_may_be_shell_script_disallow_manual_argument_splitting()
        .with_context(
            repository
                .command_context()
                .context("could not prepare pager environment")?,
        )
        .env("GIT_PAGER_IN_USE", "true")
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .into();
    Ok(FileDiff::Pager { command, diff })
}

fn prepare_external_diff(
    repository: &gix::Repository,
    resources: &gix::diff::blob::Platform,
    command: BString,
) -> Result<gix::diff::blob::platform::prepare_diff_command::Command> {
    resources
        .prepare_diff_command(
            command,
            repository
                .command_context()
                .context("could not prepare external diff environment")?,
            0,
            1,
        )
        .context("could not prepare external diff command")
}

fn built_in_diff(path: &PathChange, change: &FileChange, rendered: Option<BString>, binary: bool) -> BuiltInDiff {
    let (old_path, new_path, old_mode, new_mode) = match change {
        FileChange::Tree(gix::object::tree::diff::ChangeDetached::Addition { entry_mode, .. }) => {
            (None, Some(path.path.as_bstr()), None, Some(*entry_mode))
        }
        FileChange::Tree(gix::object::tree::diff::ChangeDetached::Deletion { entry_mode, .. }) => {
            (Some(path.path.as_bstr()), None, Some(*entry_mode), None)
        }
        FileChange::Tree(gix::object::tree::diff::ChangeDetached::Modification {
            previous_entry_mode,
            entry_mode,
            ..
        }) => (
            Some(path.path.as_bstr()),
            Some(path.path.as_bstr()),
            Some(*previous_entry_mode),
            Some(*entry_mode),
        ),
        FileChange::Tree(gix::object::tree::diff::ChangeDetached::Rewrite {
            source_entry_mode,
            entry_mode,
            ..
        }) => (
            path.source.as_ref().map(|path| path.as_bstr()),
            Some(path.path.as_bstr()),
            Some(*source_entry_mode),
            Some(*entry_mode),
        ),
        FileChange::Worktree { old, new } => (
            old.as_ref().map(|resource| resource.path.as_bstr()),
            new.as_ref().map(|resource| resource.path.as_bstr()),
            old.as_ref().map(|resource| resource.mode),
            new.as_ref().map(|resource| resource.mode),
        ),
        FileChange::Unavailable(_) => unreachable!("unavailable diffs aren't rendered"),
    };
    let display_path = |path: Option<&gix::bstr::BStr>, prefix: &str| -> BString {
        path.map_or_else(
            || "/dev/null".into(),
            |path| format!("{prefix}{}", path.to_str_lossy()).into(),
        )
    };
    let mut lines = vec![
        format!("--- {}", display_path(old_path, "a/").to_str_lossy()).into(),
        format!("+++ {}", display_path(new_path, "b/").to_str_lossy()).into(),
    ];
    if old_mode != new_mode {
        if let Some(mode) = old_mode {
            lines.push(format!("old mode {}", mode.kind().as_octal_str()).into());
        }
        if let Some(mode) = new_mode {
            lines.push(format!("new mode {}", mode.kind().as_octal_str()).into());
        }
    }
    if binary {
        lines.push("Binary files differ".into());
    } else if let Some(rendered) = rendered {
        lines.extend(rendered.lines().map(BString::from));
    }
    BuiltInDiff::new(
        format!("{} {}", path.kind.letter(), path.path.to_str_lossy()).into(),
        lines,
    )
}

fn show_file_diff(terminal: &mut ratatui::DefaultTerminal, diff: FileDiff, enhanced_keyboard: bool) -> Result<bool> {
    match diff {
        FileDiff::External(command) => run_external_diff(terminal, command, enhanced_keyboard).map(|()| false),
        FileDiff::Pager { command, diff } => run_pager(terminal, command, &diff, enhanced_keyboard).map(|()| false),
        FileDiff::BuiltIn(diff) => show_builtin_diff(terminal, &diff),
    }
}

fn show_commit_diff(
    terminal: &mut ratatui::DefaultTerminal,
    diff: CommitDiff,
    enhanced_keyboard: bool,
) -> Result<bool> {
    if show_file_diff(terminal, diff.internal, enhanced_keyboard)? {
        return Ok(true);
    }
    for command in diff.external {
        run_external_diff(terminal, command, enhanced_keyboard)?;
    }
    Ok(false)
}

#[tracing::instrument(skip_all, fields(commit_id = %id))]
fn edit_note(
    terminal: &mut ratatui::DefaultTerminal,
    repository_path: &Path,
    bare: bool,
    id: gix::ObjectId,
    enhanced_keyboard: bool,
) -> Result<Option<(enrich::Enrichment, Result<Vec<edit::undo::RefChange>>)>> {
    let (editor, enrichment, document) = {
        let repository =
            open_repository(repository_path, bare, false).context("could not open repository before editing note")?;
        let change_id = change_id::for_commit(&repository, id)?;
        let enrichment = enrich::load(&mut enrich::open(&repository)?, change_id)?;
        let document = enrichment.note.clone().unwrap_or_default();
        let editor = repository.editor().context("no Git editor is available")?;
        (editor, enrichment, document)
    };
    let edited = edit::edit_document(
        terminal,
        &editor,
        &document,
        &format!("tix-note-{}.md", std::process::id()),
        enhanced_keyboard,
    )?;
    let cleaned = edit::reword::cleanup_message(edited.as_deref().unwrap_or(&document), None);
    let desired_note = (!cleaned.is_empty()).then_some(cleaned.as_bstr());
    if enrichment.note.as_ref().map(|note| note.as_bstr()) == desired_note {
        return Ok(None);
    }

    let repository =
        open_repository(repository_path, bare, false).context("could not reopen repository after editing note")?;
    let name: gix::refs::FullName = enrich::REF_NAME.try_into().expect("valid enrich ref");
    let before = edit::undo::state(&repository, name.as_ref());
    let enrichment = enrich::set_note(&repository, id, desired_note.map(AsRef::as_ref))?;
    let changes = before.and_then(|before| {
        edit::undo::state(&repository, name.as_ref()).map(|after| {
            (before != after)
                .then_some(edit::undo::RefChange { name, before, after })
                .into_iter()
                .collect()
        })
    });
    Ok(Some((enrichment, changes)))
}

#[tracing::instrument(skip_all, fields(commit_id = %id))]
fn edit_git_note(
    terminal: &mut ratatui::DefaultTerminal,
    repository_path: &Path,
    bare: bool,
    id: gix::ObjectId,
    enhanced_keyboard: bool,
) -> Result<Option<(bool, Result<Vec<edit::undo::RefChange>>)>> {
    let (editor, reference, document) = {
        let repository = open_repository(repository_path, bare, false)
            .context("could not open repository before editing Git note")?;
        let editor = repository.editor().context("no Git editor is available")?;
        let notes = repository.notes()?;
        let reference = notes
            .default_ref()
            .context("no default Git notes reference is configured")?
            .to_owned();
        let mut notes = notes.with_refs([reference.as_bstr()])?;
        let document = notes
            .get(id)?
            .first()
            .map(|note| note.blob.data.clone())
            .unwrap_or_default();
        (editor, reference, document)
    };
    let edited = edit::edit_document(
        terminal,
        &editor,
        &document,
        &format!("tix-git-note-{}.md", std::process::id()),
        enhanced_keyboard,
    )?;
    let cleaned = edit::reword::cleanup_message(edited.as_deref().unwrap_or(&document), None);
    if cleaned == document {
        return Ok(None);
    }

    let repository =
        open_repository(repository_path, bare, false).context("could not reopen repository after editing Git note")?;
    let changes = set_git_note_reporting(
        &repository,
        reference.as_ref(),
        id,
        (!cleaned.is_empty()).then_some(cleaned.as_ref()),
    )?;
    Ok(Some((!cleaned.is_empty(), changes)))
}

fn set_git_note_reporting(
    repository: &gix::Repository,
    reference: &gix::refs::FullNameRef,
    id: gix::ObjectId,
    data: Option<&[u8]>,
) -> Result<Result<Vec<edit::undo::RefChange>>> {
    let name = reference.to_owned();
    let before = edit::undo::state(repository, reference);
    let mut notes = repository.notes()?;
    match data {
        Some(data) => {
            notes
                .add_to_ref(reference, id, data)
                .context("could not save Git note")?;
        }
        None => {
            notes
                .remove(reference.as_partial_name().to_owned(), id)
                .context("could not remove Git note")?;
        }
    }
    Ok(before.and_then(|before| {
        edit::undo::state(repository, reference).map(|after| {
            (before != after)
                .then_some(edit::undo::RefChange { name, before, after })
                .into_iter()
                .collect()
        })
    }))
}

#[cfg(test)]
pub(crate) fn set_git_note(
    repository: &gix::Repository,
    reference: &gix::refs::FullNameRef,
    id: gix::ObjectId,
    data: Option<&[u8]>,
) -> Result<Vec<edit::undo::RefChange>> {
    set_git_note_reporting(repository, reference, id, data)?
}

#[tracing::instrument(skip_all, fields(commit_id = %id))]
fn reword_commit(
    terminal: &mut ratatui::DefaultTerminal,
    repository_path: &Path,
    bare: bool,
    revisions: &[OsString],
    hidden_revisions: &[OsString],
    id: gix::ObjectId,
    enhanced_keyboard: bool,
) -> Result<Option<edit::reword::Perform>> {
    let (editor, document, change_id) = {
        let mut repository =
            open_repository(repository_path, bare, false).context("could not open repository before editing commit")?;
        repository.object_cache_size(None);
        let change_id = change_id::for_commit(&repository, id)?;
        let (editor, document) = edit::reword::document(&repository, id)?;
        (editor, document, change_id)
    };
    let Some(edited) = edit::edit_document(
        terminal,
        &editor,
        &document,
        &format!("tix-reword-{}.md", std::process::id()),
        enhanced_keyboard,
    )?
    else {
        return Ok(None);
    };

    run_with_todo_progress(terminal, move |report| {
        let mut repository = open_repository(repository_path, bare, false)
            .context("could not reopen repository after editing commit")?;
        repository.object_cache_size(None);
        let (graph, id) = edit::reword::relocate_after_editor(&repository, revisions, hidden_revisions, change_id)?;
        edit::reword::apply_conflict_reporting(repository, &graph, id, &edited, report)
    })
    .map(Some)
}

pub(crate) fn load_rebase_todo_commits(
    repository: &gix::Repository,
    app: &mut App,
    authors: &SharedAuthors,
    scope: &[gix::ObjectId],
) -> Result<Vec<edit::todo::Commit>> {
    let row_indices: HashMap<_, _> = app
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| (row.id, index))
        .collect();
    let mut notes = repository.notes().context("could not open Git notes")?;
    for id in scope {
        let index = row_indices
            .get(id)
            .copied()
            .context("an editable commit disappeared from the history view")?;
        if !app.rows[index].metadata_loaded {
            let (metadata, attributions) =
                history::load_metadata(repository, *id, authors).context("could not load editable commit metadata")?;
            app.set_metadata(index, metadata, attributions);
        }
        let loaded = notes
            .get(*id)
            .context("could not load commit notes")?
            .into_iter()
            .map(|note| {
                let mut blob = note.blob;
                BString::from(blob.take_data())
            })
            .collect();
        app.set_notes(*id, loaded);
    }
    let mailmap = repository.open_mailmap();
    scope
        .iter()
        .map(|id| {
            let row = row_indices
                .get(id)
                .and_then(|index| app.rows.get(*index))
                .context("an editable commit disappeared while formatting the todo")?;
            Ok(edit::todo::Commit {
                id: *id,
                parents: row.parent_ids.to_vec(),
                info: ui::todo_metadata(app, row, &mailmap),
            })
        })
        .collect()
}

#[tracing::instrument(skip_all, fields(base = %base, commits = commits.len()))]
#[expect(
    clippy::too_many_arguments,
    reason = "the editor bridges terminal, repository, and selected view state"
)]
fn rebase_history(
    terminal: &mut ratatui::DefaultTerminal,
    repository_path: &Path,
    bare: bool,
    graph: &HistoryGraph,
    base: gix::ObjectId,
    onto: gix::ObjectId,
    commits: Vec<edit::todo::Commit>,
    enhanced_keyboard: bool,
) -> Result<Option<edit::rebase::PlanPerform>> {
    let (prepared, editor) = {
        let mut repository =
            open_repository(repository_path, bare, false).context("could not open repository before rebasing")?;
        repository.object_cache_size(None);
        let editor = repository.editor().context("no Git editor is available")?;
        let prepared = edit::todo::prepare(
            &repository,
            base,
            onto,
            &commits,
            &[],
            edit::todo::OntoKind::UpdatedBase,
            false,
        )?;
        (prepared, editor)
    };
    let edited = edit::edit_document(
        terminal,
        &editor,
        &prepared.document,
        &format!("tix-rebase-{}.md", std::process::id()),
        enhanced_keyboard,
    )?;
    let edited = match edited {
        Some(edited) => edited,
        None if prepared.apply_unchanged => prepared.document.clone(),
        None => return Ok(None),
    };
    let mut repository =
        open_repository(repository_path, bare, false).context("could not reopen repository after editing rebase")?;
    repository.object_cache_size(None);
    let Some(parsed) = edit::todo::parse(&repository, &edited)? else {
        return Ok(None);
    };
    run_rebase_plan(terminal, repository.into_sync(), graph, parsed.plan).map(Some)
}

fn stage_resolved_conflict_paths(repository: &gix::Repository) -> Result<()> {
    let index = repository
        .open_index()
        .context("could not inspect the conflict index")?;
    let mut paths: Vec<BString> = index
        .entries()
        .iter()
        .filter(|entry| entry.stage() != gix::index::entry::Stage::Unconflicted)
        .map(|entry| entry.path(&index).to_owned())
        .collect();
    paths.sort();
    paths.dedup();
    if paths.is_empty() {
        return Ok(());
    }

    let workdir = repository
        .workdir()
        .context("cannot resolve a conflict without a worktree")?;
    let mut command = Command::new("git");
    command
        .arg("--literal-pathspecs")
        .arg("-C")
        .arg(workdir)
        .args(["add", "-A", "--"]);
    for path in &paths {
        command.arg(gix::path::from_bstr(path.as_bstr()).as_ref());
    }
    let output = command
        .output()
        .context("could not launch git add for resolved paths")?;
    if !output.status.success() {
        let stderr = output.stderr.trim().to_str_lossy();
        if stderr.is_empty() {
            anyhow::bail!("git add for resolved paths failed with {}", output.status);
        }
        anyhow::bail!("git add for resolved paths failed with {}: {stderr}", output.status);
    }

    let index = repository
        .open_index()
        .context("could not verify the resolved conflict index")?;
    if index
        .entries()
        .iter()
        .any(|entry| entry.stage() != gix::index::entry::Stage::Unconflicted)
    {
        anyhow::bail!("the conflict index still has unresolved entries");
    }
    Ok(())
}

fn preview_todo_rebase_conflict(
    app: &mut App,
    conflict: &edit::rebase::PlanConflict,
    authors: &SharedAuthors,
    view_tips: &[gix::ObjectId],
    hidden_tips: &[gix::ObjectId],
) -> Result<()> {
    let repo = conflict.repository();
    let mut rows = Vec::with_capacity(conflict.produced().len());
    let mut attributions = Vec::new();
    for id in conflict.produced().iter().rev().copied() {
        let commit = repo
            .find_commit(id)
            .context("could not load an in-memory rebase result")?;
        let parent_ids = commit.parent_ids().map(gix::Id::detach).collect();
        let (metadata, mut row_attributions) = history::load_metadata(repo, id, authors)?;
        let attribution_start = attributions.len();
        let attribution_len = metadata.attributions.len();
        attributions.append(&mut row_attributions);
        rows.push(app::Commit {
            id,
            parent_ids,
            committer_time: metadata.committer_time,
            author_time: metadata.author_time,
            author: metadata.author,
            attributions: attribution_start..attribution_start + attribution_len,
            title: metadata.title,
            metadata_loaded: true,
            has_agent_marker: metadata.has_agent_marker,
            is_review: metadata.is_review,
            signature: metadata.signature,
        });
    }
    let view_tips = view_tips.iter().filter_map(|id| conflict.map(*id)).collect::<Vec<_>>();
    let hidden_tips = hidden_tips
        .iter()
        .filter_map(|id| conflict.map(*id))
        .collect::<Vec<_>>();
    if let Some(rows) = app.start_refresh(
        app::LoadedCommits { rows, attributions },
        &view_tips,
        &hidden_tips,
        false,
    ) {
        let (rows, graph, elapsed) = app::compute_lanes(rows);
        app.finish_lane_computation(rows, graph, elapsed);
    }
    Ok(())
}

enum RebaseWorkerEvent<T> {
    Progress(edit::rebase::Progress),
    Complete(Result<T>),
}

enum TravelWorkerEvent<T> {
    Rebased(gix::ObjectId),
    Complete(Result<T>),
}

fn run_with_rebase_selection<T: Send>(
    operation: impl FnOnce(&mut dyn FnMut(gix::ObjectId)) -> Result<T> + Send,
    mut render: impl FnMut(gix::ObjectId) -> Result<()>,
) -> Result<T> {
    std::thread::scope(|scope| {
        let (sender, receiver) = mpsc::channel();
        let worker = scope.spawn(move || {
            let mut report = |id| {
                let _ = sender.send(TravelWorkerEvent::Rebased(id));
            };
            let result = operation(&mut report);
            let _ = sender.send(TravelWorkerEvent::Complete(result));
        });
        let mut last_draw: Option<Instant> = None;
        let mut latest = None;
        let mut rendered = None;
        let mut complete = None;
        let mut render_error = None;
        let result = loop {
            let event = if complete.is_some() {
                None
            } else if render_error.is_none()
                && latest != rendered
                && let Some(last_draw) = last_draw
            {
                match receiver.recv_timeout(FRAME_INTERVAL.saturating_sub(last_draw.elapsed())) {
                    Ok(event) => Some(event),
                    Err(mpsc::RecvTimeoutError::Timeout) => None,
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        break Err(anyhow::anyhow!("time-travel worker stopped unexpectedly"));
                    }
                }
            } else {
                match receiver.recv() {
                    Ok(event) => Some(event),
                    Err(_) => break Err(anyhow::anyhow!("time-travel worker stopped unexpectedly")),
                }
            };
            match event {
                Some(TravelWorkerEvent::Rebased(id)) => latest = Some(id),
                Some(TravelWorkerEvent::Complete(result)) => complete = Some(result),
                None => {}
            }
            if render_error.is_none()
                && latest != rendered
                && (last_draw.is_none()
                    || last_draw.is_some_and(|last_draw| last_draw.elapsed() >= FRAME_INTERVAL)
                    || complete.is_some())
            {
                if complete.is_some()
                    && let Some(last_draw) = last_draw
                {
                    std::thread::sleep(FRAME_INTERVAL.saturating_sub(last_draw.elapsed()));
                }
                let id = latest.expect("a changed rebased commit is available");
                if let Err(err) = render(id) {
                    render_error = Some(err);
                    continue;
                }
                rendered = latest;
                last_draw = Some(Instant::now());
            }
            if let Some(result) = complete.take() {
                break result;
            }
        };
        if let Some(err) = render_error {
            tracing::warn!(error = %err, "time-travel animation stopped");
        }
        if worker.join().is_err() {
            return Err(anyhow::anyhow!("time-travel worker panicked"));
        }
        result
    })
}

fn run_rebase_plan(
    terminal: &mut ratatui::DefaultTerminal,
    repository: gix::ThreadSafeRepository,
    graph: &HistoryGraph,
    plan: edit::rebase::Plan,
) -> Result<edit::rebase::PlanPerform> {
    run_with_todo_progress(terminal, move |report| {
        let mut repository = repository.to_thread_local();
        repository.object_cache_size(None);
        edit::rebase::perform_plan_with_progress(&repository, graph, plan, report)
    })
}

fn run_with_todo_progress<T: Send>(
    terminal: &mut ratatui::DefaultTerminal,
    operation: impl FnOnce(&mut dyn FnMut(edit::rebase::Progress)) -> Result<T> + Send,
) -> Result<T> {
    std::thread::scope(|scope| {
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = scope.spawn(move || {
            let mut report = |progress| {
                let _ = sender.try_send(RebaseWorkerEvent::Progress(progress));
            };
            let result = operation(&mut report);
            let _ = sender.send(RebaseWorkerEvent::Complete(result));
        });
        let started = Instant::now();
        let mut last_draw = started;
        let mut latest = None;
        let mut rendered = None;
        let result = loop {
            let now = Instant::now();
            let timeout = if now.duration_since(started) < TODO_PROGRESS_DELAY {
                Some(TODO_PROGRESS_DELAY.saturating_sub(now.duration_since(started)))
            } else if latest != rendered {
                Some(FRAME_INTERVAL.saturating_sub(now.duration_since(last_draw)))
            } else {
                None
            };
            let event = match timeout {
                Some(timeout) => match receiver.recv_timeout(timeout) {
                    Ok(event) => Some(event),
                    Err(mpsc::RecvTimeoutError::Timeout) => None,
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        break Err(anyhow::anyhow!("rebase worker stopped unexpectedly"));
                    }
                },
                None => match receiver.recv() {
                    Ok(event) => Some(event),
                    Err(_) => break Err(anyhow::anyhow!("rebase worker stopped unexpectedly")),
                },
            };
            match event {
                Some(RebaseWorkerEvent::Progress(progress)) => latest = Some(progress),
                Some(RebaseWorkerEvent::Complete(result)) => break result,
                None => {}
            }
            let now = Instant::now();
            if todo_progress_visible(now.duration_since(started))
                && latest != rendered
                && now.duration_since(last_draw) >= FRAME_INTERVAL
            {
                let progress = latest.expect("a changed progress snapshot is available");
                if let Err(err) = terminal.draw(|frame| ui::draw_todo_progress(frame, progress)) {
                    break Err(err).context("could not draw rebase progress");
                }
                rendered = latest;
                last_draw = now;
            }
        };
        if worker.join().is_err() {
            return Err(anyhow::anyhow!("rebase worker panicked"));
        }
        result
    })
}

fn todo_progress_visible(elapsed: Duration) -> bool {
    elapsed >= TODO_PROGRESS_DELAY
}

#[derive(Clone, Copy)]
enum CreateMode {
    Insert,
    InsertEmpty,
    Fork,
}

#[tracing::instrument(skip_all, fields(parent = ?parent, fork = matches!(mode, CreateMode::Fork)))]
fn create_commit(
    terminal: &mut ratatui::DefaultTerminal,
    repository_path: &Path,
    bare: bool,
    graph: &HistoryGraph,
    parent: Option<gix::ObjectId>,
    mode: CreateMode,
    enhanced_keyboard: bool,
) -> Result<Option<edit::rebase::Perform>> {
    let mut repository =
        open_repository(repository_path, bare, false).context("could not open repository before creating commit")?;
    repository.object_cache_size(None);
    let prepared = if matches!(mode, CreateMode::InsertEmpty) {
        edit::create::prepare_empty(repository, parent)?
    } else {
        edit::create::prepare(repository, parent)?
    };
    if matches!(mode, CreateMode::Insert) && prepared.is_empty {
        anyhow::bail!("the new commit would be empty; use new-empty instead");
    }
    let Some(edited) = edit::edit_document(
        terminal,
        &prepared.editor,
        &prepared.document,
        &format!("tix-commit-{}.md", std::process::id()),
        enhanced_keyboard,
    )?
    else {
        return Ok(None);
    };
    let outcome = match mode {
        CreateMode::Insert | CreateMode::InsertEmpty => run_with_todo_progress(terminal, move |report| {
            let mut repository = open_repository(repository_path, bare, false)
                .context("could not reopen repository after editing commit")?;
            repository.object_cache_size(None);
            edit::create::apply_conflict_reporting(repository, graph, prepared, &edited, report)
        }),
        CreateMode::Fork => {
            let mut repository = open_repository(repository_path, bare, false)
                .context("could not reopen repository after editing commit")?;
            repository.object_cache_size(None);
            edit::create::apply_fork_reporting(repository, graph, prepared, &edited)
                .map(edit::rebase::Perform::Complete)
        }
    }?;
    Ok(Some(outcome))
}

#[tracing::instrument(skip_all)]
fn split_commit(
    terminal: &mut ratatui::DefaultTerminal,
    repository_path: &Path,
    bare: bool,
    graph: &HistoryGraph,
    enhanced_keyboard: bool,
) -> Result<Option<edit::rebase::Outcome>> {
    let mut repository =
        open_repository(repository_path, bare, false).context("could not open repository before splitting HEAD")?;
    repository.object_cache_size(None);
    let prepared = edit::split::prepare(repository, false)?;
    let Some(edited) = edit::edit_document(
        terminal,
        &prepared.editor,
        &prepared.document,
        &format!("tix-split-{}.md", std::process::id()),
        enhanced_keyboard,
    )?
    else {
        return Ok(None);
    };
    run_with_todo_progress(terminal, move |report| {
        let mut repository =
            open_repository(repository_path, bare, false).context("could not reopen repository after editing split")?;
        repository.object_cache_size(None);
        edit::split::apply_reporting(repository, graph, prepared, &edited, report)
    })
    .map(Some)
}

#[tracing::instrument(skip_all, fields(commit_id = %id))]
fn forget_commit(
    terminal: &mut ratatui::DefaultTerminal,
    repository_path: &Path,
    bare: bool,
    graph: &HistoryGraph,
    id: gix::ObjectId,
) -> Result<edit::forget::Perform> {
    run_with_todo_progress(terminal, move |report| {
        let mut repository = open_repository(repository_path, bare, false)
            .context("could not open repository before forgetting commit")?;
        repository.object_cache_size(None);
        edit::forget::perform_conflict(repository, graph, id, report)
    })
}

fn run_external_diff(
    terminal: &mut ratatui::DefaultTerminal,
    mut command: gix::diff::blob::platform::prepare_diff_command::Command,
    enhanced_keyboard: bool,
) -> Result<()> {
    with_suspended_terminal(terminal, enhanced_keyboard, || {
        let status = command.status().context("could not launch external diff")?;
        external_diff_status(status)
    })
}

fn run_pager(
    terminal: &mut ratatui::DefaultTerminal,
    mut command: Command,
    diff: &BuiltInDiff,
    enhanced_keyboard: bool,
) -> Result<()> {
    with_suspended_terminal(terminal, enhanced_keyboard, || {
        let start = Instant::now();
        let mut child = command.spawn().context("could not launch diff pager")?;
        let write_result = child.stdin.take().map_or_else(
            || Err(io::Error::other("pager stdin was not piped")),
            |mut stdin| diff.write_to(&mut stdin),
        );
        let status = child.wait().context("could not wait for diff pager");
        pager_write_result(write_result)?;
        pager_status(status?)?;
        if pager_needs_acknowledgement(start.elapsed()) {
            wait_for_keypress()?;
        }
        Ok(())
    })
}

fn wait_for_keypress() -> Result<()> {
    terminal::enable_raw_mode().context("could not read pager acknowledgement")?;
    loop {
        if matches!(
            event::read().context("could not read pager acknowledgement")?,
            TerminalEvent::Key(KeyEvent {
                kind: KeyEventKind::Press,
                ..
            })
        ) {
            return Ok(());
        }
    }
}

fn with_suspended_terminal<T>(
    terminal: &mut ratatui::DefaultTerminal,
    enhanced_keyboard: bool,
    operation: impl FnOnce() -> Result<T>,
) -> Result<T> {
    let suspend = disable_input(terminal.backend_mut(), enhanced_keyboard)
        .and_then(|()| terminal.show_cursor())
        .and_then(|()| terminal::disable_raw_mode())
        .and_then(|()| {
            execute!(
                terminal.backend_mut(),
                ResetColor,
                cursor::MoveTo(0, 0),
                Clear(ClearType::All)
            )
        });
    if let Err(err) = suspend {
        let _ = terminal::enable_raw_mode();
        let _ = enable_input(terminal.backend_mut(), enhanced_keyboard);
        let _ = terminal.hide_cursor();
        return Err(err).context("could not suspend terminal for external program");
    }

    let result = operation();
    let restore = terminal::enable_raw_mode()
        .and_then(|()| enable_input(terminal.backend_mut(), enhanced_keyboard))
        .and_then(|()| terminal.hide_cursor())
        .and_then(|()| terminal.clear());
    let value = result?;
    restore.context("could not restore terminal after external program")?;
    Ok(value)
}

struct RemoteDeleteOutcome {
    deleted: usize,
    failures: Vec<String>,
}

fn push_remote_deletions(repository_path: &Path, groups: &[ref_tree::RemoteDeletion]) -> RemoteDeleteOutcome {
    let mut outcome = RemoteDeleteOutcome {
        deleted: 0,
        failures: Vec::new(),
    };
    for group in groups {
        let mut command = Command::new("git");
        command
            .arg("-C")
            .arg(repository_path)
            .arg("push")
            .arg(gix::path::from_bstr(group.remote.as_bstr()).as_ref());
        for reference in &group.references {
            let mut refspec = b":".to_vec();
            refspec.extend_from_slice(reference.as_bstr());
            command.arg(gix::path::from_bstr(refspec.as_bstr()).as_ref());
        }
        match command.status() {
            Ok(status) if status.success() => outcome.deleted += group.references.len(),
            Ok(status) => outcome.failures.push(format!("{} exited with {status}", group.remote)),
            Err(err) => outcome.failures.push(format!("{}: {err}", group.remote)),
        }
    }
    outcome
}

fn external_diff_status(status: ExitStatus) -> Result<()> {
    if status.success() || status.code() == Some(1) {
        Ok(())
    } else {
        anyhow::bail!("external diff exited with {status}")
    }
}

fn pager_write_result(result: io::Result<()>) -> Result<()> {
    match result {
        Err(err) if err.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        result => result.context("could not write diff to pager"),
    }
}

fn pager_status(status: ExitStatus) -> Result<()> {
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("diff pager exited with {status}")
    }
}

fn pager_needs_acknowledgement(elapsed: Duration) -> bool {
    elapsed <= IMMEDIATE_PAGER_EXIT
}

fn show_builtin_diff(terminal: &mut ratatui::DefaultTerminal, diff: &BuiltInDiff) -> Result<bool> {
    let mut offset = 0usize;
    let mut horizontal_offset = 0usize;
    let mut focused = true;
    loop {
        let size = terminal.size().context("could not determine diff viewport")?;
        let page = usize::from(size.height.saturating_sub(2)).max(1);
        let max = diff.display_line_count().saturating_sub(page);
        let horizontal_page = usize::from(size.width).max(1);
        let horizontal_max = diff.max_width.saturating_sub(horizontal_page);
        offset = offset.min(max);
        horizontal_offset = horizontal_offset.min(horizontal_max);
        terminal
            .draw(|frame| ui::draw_file_diff(frame, diff, offset, horizontal_offset))
            .context("could not draw file diff")?;
        let event = event::read().context("could not read file diff input")?;
        let key = match event {
            TerminalEvent::FocusLost => {
                focused = false;
                continue;
            }
            TerminalEvent::FocusGained => {
                focused = true;
                continue;
            }
            TerminalEvent::Resize(_, _) => continue,
            TerminalEvent::Key(key) if focused && key.kind != KeyEventKind::Release => key,
            _ => continue,
        };
        match action(key) {
            Some(Action::OpenDiff) => return Ok(false),
            Some(Action::ForceQuit | Action::Quit | Action::Cancel) => return Ok(true),
            Some(Action::MoveUp) => offset = offset.saturating_sub(1),
            Some(Action::MoveDown) => offset = offset.saturating_add(1).min(max),
            Some(Action::PageUp) => offset = offset.saturating_sub(page),
            Some(Action::PageDown) => offset = offset.saturating_add(page).min(max),
            Some(Action::HalfPageUp) => offset = offset.saturating_sub((page / 2).max(1)),
            Some(Action::HalfPageDown) => offset = offset.saturating_add((page / 2).max(1)).min(max),
            Some(Action::First) => offset = 0,
            Some(Action::Last) => offset = max,
            Some(Action::ScrollLeft) => horizontal_offset = horizontal_offset.saturating_sub(horizontal_page),
            Some(Action::ScrollRight) => {
                horizontal_offset = horizontal_offset.saturating_add(horizontal_page).min(horizontal_max);
            }
            _ => {}
        }
    }
}

fn load_commit_message(repository: &gix::Repository, id: gix::ObjectId) -> Result<BString> {
    let commit = repository.find_commit(id).context("could not load commit message")?;
    Ok(commit.message_raw_sloppy().to_owned())
}

fn load_changes(
    repository: &gix::Repository,
    target: app::TreeDiffTarget,
    line_diff_pool: &mut LineDiffPool,
) -> Result<Changes> {
    let mut out = load_changes_without_lines(repository, target)?;
    let diffs = std::mem::take(&mut out.diffs);
    for (path, (change, lines)) in out.paths.iter_mut().zip(line_diff_pool.line_counts(diffs)?) {
        path.lines = lines;
        if let Some((insertions, removals)) = lines {
            out.lines_added += u64::from(insertions);
            out.lines_removed += u64::from(removals);
        }
        out.diffs.push(change);
    }
    Ok(out)
}

fn load_changes_without_lines(repository: &gix::Repository, target: app::TreeDiffTarget) -> Result<Changes> {
    let app::TreeDiffTarget::Commit {
        id,
        parent: requested_parent,
    } = target
    else {
        let app::TreeDiffTarget::Branch { base, tip } = target else {
            unreachable!("all tree diff targets are covered")
        };
        let old_tree = repository
            .find_commit(base)
            .context("could not load branch base")?
            .tree()
            .context("could not load branch base tree")?;
        let new_tree = repository
            .find_commit(tip)
            .context("could not load branch tip")?
            .tree()
            .context("could not load branch tip tree")?;
        let mut changes = load_tree_changes_without_lines(repository, Some(&old_tree), &new_tree, None)?;
        changes.range = Some(app::ComparedRange { base, tip });
        return Ok(changes);
    };
    let commit = repository.find_commit(id).context("could not load changed paths")?;
    let marked_parent = {
        let decoded = commit.decode().context("could not decode changed commit")?;
        edit::rebase::marked_parent_ref(&decoded)?
    };
    let parents: Vec<_> = commit.parent_ids().map(gix::Id::detach).collect();
    let parent_index = requested_parent.checked_rem(parents.len()).unwrap_or_default();
    let selected_parent = parents.get(parent_index).copied();
    let (parent, compared_parent) = match marked_parent {
        Some(parent) => (parent, None),
        None => (
            selected_parent,
            (parents.len() > 1).then(|| ComparedParent {
                index: parent_index,
                total: parents.len(),
                id: selected_parent.expect("a merge has parents"),
            }),
        ),
    };
    let new_tree = commit.tree().context("could not load changed commit tree")?;
    let old_tree = match parent {
        Some(parent) => Some(
            repository
                .find_commit(parent)
                .context("could not load parent commit")?
                .tree()
                .context("could not load parent commit tree")?,
        ),
        None => None,
    };
    load_tree_changes_without_lines(repository, old_tree.as_ref(), &new_tree, compared_parent)
}

fn load_tree_changes_without_lines(
    repository: &gix::Repository,
    old_tree: Option<&gix::Tree<'_>>,
    new_tree: &gix::Tree<'_>,
    parent: Option<ComparedParent>,
) -> Result<Changes> {
    let changes = repository
        .diff_tree_to_tree(old_tree, Some(new_tree), None)
        .context("could not diff commit trees")?;
    let mut out = Changes {
        parent,
        ..Changes::default()
    };
    for change in changes {
        use gix::object::tree::diff::ChangeDetached;
        let (kind, source, path, is_tree) = match &change {
            ChangeDetached::Addition {
                entry_mode, location, ..
            } => (ChangeKind::Added, None, location.clone(), entry_mode.is_tree()),
            ChangeDetached::Deletion {
                entry_mode, location, ..
            } => (ChangeKind::Deleted, None, location.clone(), entry_mode.is_tree()),
            ChangeDetached::Modification {
                previous_entry_mode,
                entry_mode,
                location,
                ..
            } => (
                if previous_entry_mode.kind() == entry_mode.kind() {
                    ChangeKind::Modified
                } else {
                    ChangeKind::TypeChanged
                },
                None,
                location.clone(),
                previous_entry_mode.is_tree() && entry_mode.is_tree(),
            ),
            ChangeDetached::Rewrite {
                source_location,
                source_entry_mode,
                entry_mode,
                location,
                copy,
                ..
            } => (
                if *copy { ChangeKind::Copied } else { ChangeKind::Renamed },
                Some(source_location.clone()),
                location.clone(),
                source_entry_mode.is_tree() || entry_mode.is_tree(),
            ),
        };
        if is_tree {
            continue;
        }
        out.paths.push(PathChange {
            kind,
            group: ChangeGroup::Tree,
            source,
            path,
            lines: None,
        });
        out.diffs.push(FileChange::Tree(change));
    }
    Ok(out)
}

fn add_line_counts(repository: &gix::Repository, changes: &mut Changes) -> Result<Vec<LineCounts>> {
    let mut cache = repository
        .diff_resource_cache_for_tree_diff()
        .context("could not initialize commit diff summary")?;
    let mut counts = Vec::with_capacity(changes.diffs.len());
    for (path, change) in changes.paths.iter_mut().zip(&changes.diffs) {
        let lines = line_counts_for_change(repository, change, &mut cache, None)?;
        path.lines = lines;
        if let Some((added, removed)) = lines {
            changes.lines_added += u64::from(added);
            changes.lines_removed += u64::from(removed);
        }
        counts.push(lines);
        cache.clear_resource_cache_keep_allocation();
    }
    Ok(counts)
}

fn entry_mode(mode: gix::index::entry::Mode) -> Result<gix::objs::tree::EntryMode> {
    mode.to_tree_entry_mode()
        .context("status entry cannot be represented in a tree")
}

fn staged_change(change: gix::diff::index::Change) -> Result<(PathChange, FileChange)> {
    use gix::diff::index::Change;
    use gix::object::tree::diff::ChangeDetached;

    let (kind, source, path, diff) = match change {
        Change::Addition {
            location,
            entry_mode: mode,
            id,
            ..
        } => {
            let entry_mode = entry_mode(mode)?;
            let path = location.into_owned();
            let diff = ChangeDetached::Addition {
                location: path.clone(),
                entry_mode,
                relation: None,
                id: id.into_owned(),
            };
            (ChangeKind::Added, None, path, diff)
        }
        Change::Deletion {
            location,
            entry_mode: mode,
            id,
            ..
        } => {
            let entry_mode = entry_mode(mode)?;
            let path = location.into_owned();
            let diff = ChangeDetached::Deletion {
                location: path.clone(),
                entry_mode,
                relation: None,
                id: id.into_owned(),
            };
            (ChangeKind::Deleted, None, path, diff)
        }
        Change::Modification {
            location,
            previous_entry_mode,
            previous_id,
            entry_mode: mode,
            id,
            ..
        } => {
            let previous_entry_mode = entry_mode(previous_entry_mode)?;
            let current_entry_mode = entry_mode(mode)?;
            let path = location.into_owned();
            let kind = if previous_entry_mode.kind() == current_entry_mode.kind() {
                ChangeKind::Modified
            } else {
                ChangeKind::TypeChanged
            };
            let diff = ChangeDetached::Modification {
                location: path.clone(),
                previous_entry_mode,
                previous_id: previous_id.into_owned(),
                entry_mode: current_entry_mode,
                id: id.into_owned(),
            };
            (kind, None, path, diff)
        }
        Change::Rewrite {
            source_location,
            source_entry_mode,
            source_id,
            location,
            entry_mode: mode,
            id,
            copy,
            ..
        } => {
            let source_entry_mode = entry_mode(source_entry_mode)?;
            let current_entry_mode = entry_mode(mode)?;
            let source = source_location.into_owned();
            let path = location.into_owned();
            let diff = ChangeDetached::Rewrite {
                source_location: source.clone(),
                source_entry_mode,
                source_relation: None,
                source_id: source_id.into_owned(),
                diff: None,
                entry_mode: current_entry_mode,
                id: id.into_owned(),
                location: path.clone(),
                relation: None,
                copy,
            };
            (
                if copy { ChangeKind::Copied } else { ChangeKind::Renamed },
                Some(source),
                path,
                diff,
            )
        }
    };
    let unavailable = matches!(diff, ChangeDetached::Addition { entry_mode, .. } if entry_mode.is_commit())
        || matches!(diff, ChangeDetached::Deletion { entry_mode, .. } if entry_mode.is_commit())
        || matches!(diff, ChangeDetached::Modification { previous_entry_mode, entry_mode, .. } if previous_entry_mode.is_commit() || entry_mode.is_commit())
        || matches!(diff, ChangeDetached::Rewrite { source_entry_mode, entry_mode, .. } if source_entry_mode.is_commit() || entry_mode.is_commit());
    Ok((
        PathChange {
            kind,
            group: ChangeGroup::Staged,
            source,
            path,
            lines: None,
        },
        if unavailable {
            FileChange::Unavailable("submodule changes don't have a file diff")
        } else {
            FileChange::Tree(diff)
        },
    ))
}

fn worktree_resource(entry: &gix::index::Entry, path: &gix::bstr::BStr) -> Result<DiffResource> {
    Ok(DiffResource {
        id: entry.id,
        mode: entry_mode(entry.mode)?,
        path: path.to_owned(),
    })
}

fn unstaged_change(
    item: gix::status::index_worktree::Item,
    object_hash: gix::hash::Kind,
) -> Result<Option<(PathChange, FileChange, bool)>> {
    use gix::status::index_worktree::Item;
    use gix::status::plumbing::index_as_worktree::{Change, EntryStatus};

    let (kind, source, path, diff, tracked) = match item {
        Item::Modification {
            entry,
            rela_path,
            status,
            ..
        } => {
            let old = worktree_resource(&entry, rela_path.as_bstr())?;
            match status {
                EntryStatus::Conflict { .. } => (
                    ChangeKind::Unmerged,
                    None,
                    rela_path,
                    FileChange::Unavailable("an unmerged path has no single file diff"),
                    true,
                ),
                EntryStatus::IntentToAdd => (
                    ChangeKind::Added,
                    None,
                    rela_path.clone(),
                    FileChange::Worktree {
                        old: None,
                        new: Some(DiffResource {
                            id: entry.id.kind().null(),
                            mode: old.mode,
                            path: rela_path,
                        }),
                    },
                    true,
                ),
                EntryStatus::NeedsUpdate(_) => return Ok(None),
                EntryStatus::Change(Change::Removed) => (
                    ChangeKind::Deleted,
                    None,
                    rela_path,
                    FileChange::Worktree {
                        old: Some(old),
                        new: None,
                    },
                    true,
                ),
                EntryStatus::Change(Change::Type { worktree_mode }) => {
                    let new_mode = entry_mode(worktree_mode)?;
                    (
                        ChangeKind::TypeChanged,
                        None,
                        rela_path.clone(),
                        FileChange::Worktree {
                            old: Some(old),
                            new: Some(DiffResource {
                                id: entry.id.kind().null(),
                                mode: new_mode,
                                path: rela_path,
                            }),
                        },
                        true,
                    )
                }
                EntryStatus::Change(Change::Modification {
                    executable_bit_changed, ..
                }) => {
                    let mode = if executable_bit_changed {
                        if old.mode.is_executable() {
                            gix::objs::tree::EntryKind::Blob
                        } else {
                            gix::objs::tree::EntryKind::BlobExecutable
                        }
                        .into()
                    } else {
                        old.mode
                    };
                    (
                        ChangeKind::Modified,
                        None,
                        rela_path.clone(),
                        FileChange::Worktree {
                            old: Some(old),
                            new: Some(DiffResource {
                                id: entry.id.kind().null(),
                                mode,
                                path: rela_path,
                            }),
                        },
                        true,
                    )
                }
                EntryStatus::Change(Change::SubmoduleModification(_)) => (
                    ChangeKind::Modified,
                    None,
                    rela_path,
                    FileChange::Unavailable("submodule changes don't have a file diff"),
                    true,
                ),
            }
        }
        Item::DirectoryContents { entry, .. } => {
            let mode = match entry.disk_kind {
                Some(gix::dir::entry::Kind::File) => gix::objs::tree::EntryKind::Blob.into(),
                Some(gix::dir::entry::Kind::Symlink) => gix::objs::tree::EntryKind::Link.into(),
                _ => return Ok(None),
            };
            let path = entry.rela_path;
            (
                ChangeKind::Added,
                None,
                path.clone(),
                FileChange::Worktree {
                    old: None,
                    new: Some(DiffResource {
                        id: object_hash.null(),
                        mode,
                        path,
                    }),
                },
                false,
            )
        }
        Item::Rewrite {
            source,
            dirwalk_entry,
            copy,
            ..
        } => {
            let source = source.rela_path().to_owned();
            let path = dirwalk_entry.rela_path;
            (
                if copy { ChangeKind::Copied } else { ChangeKind::Renamed },
                Some(source),
                path,
                FileChange::Unavailable("unstaged rewrite diffs aren't available"),
                true,
            )
        }
    };
    Ok(Some((
        PathChange {
            kind,
            group: ChangeGroup::Unstaged,
            source,
            path,
            lines: None,
        },
        diff,
        tracked,
    )))
}

fn load_worktree_changes_without_lines(repository: &gix::Repository) -> Result<Changes> {
    let mut status = repository
        .status(gix::progress::Discard)
        .context("could not initialize worktree status")?
        .untracked_files(gix::status::UntrackedFiles::Files)
        .index_worktree_options_mut(|options| {
            options.sorting = Some(gix::status::plumbing::index_as_worktree_with_renames::Sorting::ByPathCaseSensitive);
        })
        .into_iter(Vec::<BString>::new())
        .context("could not start worktree status")?;
    let mut staged = Vec::new();
    let mut unstaged = Vec::new();
    let mut has_tracked_changes = false;
    for item in status.by_ref() {
        match item.context("could not obtain worktree status")? {
            gix::status::Item::TreeIndex(change) => {
                has_tracked_changes = true;
                staged.push(staged_change(change)?);
            }
            gix::status::Item::IndexWorktree(item) => {
                if let Some((path, diff, tracked)) = unstaged_change(item, repository.object_hash())? {
                    has_tracked_changes |= tracked;
                    unstaged.push((path, diff));
                }
            }
        }
    }
    drop(status);
    staged.sort_by(|(a, _), (b, _)| a.path.cmp(&b.path));
    unstaged.sort_by(|(a, _), (b, _)| a.path.cmp(&b.path));
    staged.extend(unstaged);

    let (paths, diffs): (Vec<_>, Vec<_>) = staged.into_iter().unzip();
    Ok(Changes {
        paths,
        diffs,
        has_tracked_changes,
        ..Changes::default()
    })
}

fn load_unstaged_changes_without_lines(repository: &gix::Repository, patterns: Vec<BString>) -> Result<Changes> {
    let mut status = repository
        .status(gix::progress::Discard)
        .context("could not initialize incremental worktree status")?
        .untracked_files(gix::status::UntrackedFiles::Files)
        .into_index_worktree_iter(patterns)
        .context("could not start incremental worktree status")?;
    let mut unstaged = Vec::new();
    let mut has_tracked_changes = false;
    for item in status.by_ref() {
        if let Some((path, diff, tracked)) = unstaged_change(
            item.context("could not obtain incremental worktree status")?,
            repository.object_hash(),
        )? {
            has_tracked_changes |= tracked;
            unstaged.push((path, diff));
        }
    }
    drop(status);
    unstaged.sort_by(|(a, _), (b, _)| a.path.cmp(&b.path));
    let (paths, diffs) = unstaged.into_iter().unzip();
    Ok(Changes {
        paths,
        diffs,
        has_tracked_changes,
        ..Changes::default()
    })
}

fn load_staged_changes_without_lines(repository: &gix::Repository) -> Result<Changes> {
    let head_tree = repository
        .head_tree_id_or_empty()
        .context("could not resolve HEAD tree for staged status")?;
    let index = repository
        .index_or_empty()
        .context("could not open index for staged status")?;
    let mut pathspec = repository
        .pathspec(
            false,
            None::<&str>,
            false,
            &index,
            gix::worktree::stack::state::attributes::Source::IdMapping,
        )
        .context("could not initialize staged status pathspec")?;
    let mut raw = Vec::new();
    repository
        .tree_index_status(
            &head_tree,
            &index,
            Some(&mut pathspec),
            gix::status::tree_index::TrackRenames::AsConfigured,
            |change, _, _| {
                raw.push(change.into_owned());
                Ok::<_, std::convert::Infallible>(std::ops::ControlFlow::Continue(()))
            },
        )
        .context("could not obtain staged status")?;
    let mut staged = raw.into_iter().map(staged_change).collect::<Result<Vec<_>>>()?;
    let has_tracked_changes = !staged.is_empty();
    staged.sort_by(|(a, _), (b, _)| a.path.cmp(&b.path));
    let (paths, diffs) = staged.into_iter().unzip();
    Ok(Changes {
        paths,
        diffs,
        has_tracked_changes,
        ..Changes::default()
    })
}

fn add_worktree_line_counts(mut out: Changes, line_diff_pool: &mut LineDiffPool) -> Result<Changes> {
    let diffs = std::mem::take(&mut out.diffs);
    for (path, (change, lines)) in out.paths.iter_mut().zip(line_diff_pool.line_counts(diffs)?) {
        path.lines = lines;
        if let Some((insertions, removals)) = lines {
            out.lines_added += u64::from(insertions);
            out.lines_removed += u64::from(removals);
        }
        out.diffs.push(change);
    }
    Ok(out)
}

fn load_worktree_changes(repository: &gix::Repository, line_diff_pool: &mut LineDiffPool) -> Result<Changes> {
    add_worktree_line_counts(load_worktree_changes_without_lines(repository)?, line_diff_pool)
}

fn literal_status_patterns(repository: &gix::Repository, scopes: &HashSet<BString>) -> Result<Option<Vec<BString>>> {
    let defaults = repository
        .pathspec_defaults()
        .context("could not load pathspec defaults for incremental status")?;
    if defaults.literal || defaults.signature.contains(gix::pathspec::MagicSignature::ICASE) {
        return Ok(None);
    }
    Ok(Some(
        scopes
            .iter()
            .map(|scope| {
                gix::pathspec::Pattern::from_literal(scope.as_slice(), gix::pathspec::MagicSignature::TOP).to_bstring()
            })
            .collect(),
    ))
}

fn path_is_in_status_scope(path: &BString, scope: &BString, ignore_case: bool) -> bool {
    let Some(prefix) = path.get(..scope.len()) else {
        return false;
    };
    let prefix_matches = if ignore_case {
        prefix.eq_ignore_ascii_case(scope.as_slice())
    } else {
        prefix == scope.as_slice()
    };
    prefix_matches && (path.len() == scope.len() || path.get(scope.len()) == Some(&b'/'))
}

fn replace_cached_changes(
    repository: &gix::Repository,
    cached: &mut Changes,
    replacement: Changes,
    mut replace: impl FnMut(&PathChange) -> bool,
) -> Result<()> {
    let mut pairs: Vec<_> = std::mem::take(&mut cached.paths)
        .into_iter()
        .zip(std::mem::take(&mut cached.diffs))
        .filter(|(change, _)| !replace(change))
        .collect();
    pairs.extend(replacement.paths.into_iter().zip(replacement.diffs));
    pairs.sort_by(|(a, _), (b, _)| {
        let rank = |group| match group {
            ChangeGroup::Staged => 0,
            ChangeGroup::Unstaged => 1,
            ChangeGroup::Tree => 2,
        };
        rank(a.group).cmp(&rank(b.group)).then_with(|| a.path.cmp(&b.path))
    });
    (cached.paths, cached.diffs) = pairs.into_iter().unzip();
    (cached.lines_added, cached.lines_removed) = cached
        .paths
        .iter()
        .filter_map(|change| change.lines)
        .fold((0, 0), |(added, removed), (a, r)| {
            (added + u64::from(a), removed + u64::from(r))
        });
    let index = repository
        .index_or_empty()
        .context("could not open index after incremental status")?;
    cached.has_tracked_changes = cached.paths.iter().any(|change| {
        change.group == ChangeGroup::Staged
            || change.group == ChangeGroup::Unstaged
                && (index.entry_by_path(change.path.as_bstr()).is_some()
                    || change
                        .source
                        .as_ref()
                        .is_some_and(|source| index.entry_by_path(source.as_bstr()).is_some()))
    });
    Ok(())
}

fn update_worktree_changes(
    repository: &gix::Repository,
    cached: &mut Changes,
    parts: &WorktreeStatusParts,
    line_diff_pool: &mut LineDiffPool,
) -> Result<bool> {
    if parts.staged {
        let staged = add_worktree_line_counts(load_staged_changes_without_lines(repository)?, line_diff_pool)?;
        replace_cached_changes(repository, cached, staged, |change| change.group == ChangeGroup::Staged)?;
    }
    if !parts.scopes.is_empty() {
        let Some(patterns) = literal_status_patterns(repository, &parts.scopes)? else {
            *cached = load_worktree_changes(repository, line_diff_pool)?;
            return Ok(true);
        };
        let unstaged = add_worktree_line_counts(
            load_unstaged_changes_without_lines(repository, patterns)?,
            line_diff_pool,
        )?;
        let ignore_case = repository.filesystem_options()?.ignore_case;
        replace_cached_changes(repository, cached, unstaged, |change| {
            change.group == ChangeGroup::Unstaged
                && parts
                    .scopes
                    .iter()
                    .any(|scope| path_is_in_status_scope(&change.path, scope, ignore_case))
        })?;
    }
    Ok(false)
}

fn actor_bytes(author: &app::Author) -> Vec<u8> {
    let mut out = Vec::with_capacity(author.name.len() + author.email.len() + 3);
    out.extend_from_slice(author.name);
    out.extend_from_slice(b" <");
    out.extend_from_slice(author.email);
    out.push(b'>');
    out
}

fn should_draw(dirty: bool, streaming: bool, since_draw: Duration) -> bool {
    dirty && (!streaming || since_draw >= FRAME_INTERVAL)
}

fn history_is_ready_to_draw(state: State, commits: usize) -> bool {
    commits != 0 || state != State::Loading
}

fn poll_timeout(
    streaming: bool,
    events: usize,
    dirty: bool,
    since_draw: Duration,
    wake_after: Option<Duration>,
) -> Option<Duration> {
    let frame_timeout = streaming.then(|| {
        if events == EVENT_BATCH_SIZE {
            Duration::ZERO
        } else if dirty {
            FRAME_INTERVAL.saturating_sub(since_draw)
        } else {
            FRAME_INTERVAL
        }
    });
    match (frame_timeout, wake_after) {
        (Some(frame), Some(wake_after)) => Some(frame.min(wake_after)),
        (Some(frame), None) => Some(frame),
        (None, wake_after) => wake_after,
    }
}

fn action(key: KeyEvent) -> Option<Action> {
    action_with_shortcut_groups(key, false, false, false, false)
}

#[derive(Debug, Eq, PartialEq)]
enum CommandMenuInput {
    Pass,
    Handled,
    Submit(Action),
}

fn command_picker_items(commands: &[MenuCommand]) -> Vec<MenuItem<'_, CommandId>> {
    commands
        .iter()
        .map(|command| {
            MenuItem::with_search_prefix(command.label, command.group.label(), command.group.prefix(), command.id)
        })
        .collect()
}

fn opens_command_menu(event: &TerminalEvent, actions_expanded: bool, command_menu_open: bool) -> bool {
    !actions_expanded
        && !command_menu_open
        && matches!(
            event,
            TerminalEvent::Key(KeyEvent {
                code: KeyCode::Char('p'),
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                ..
            })
        )
}

fn swallow_command_menu_key_event(event: &TerminalEvent, suppressed: &mut Option<KeyCode>) -> bool {
    let Some(expected) = suppressed.take() else {
        return false;
    };
    match event {
        TerminalEvent::Key(key) if key.code == expected && key.kind == KeyEventKind::Repeat => {
            *suppressed = Some(expected);
            true
        }
        TerminalEvent::Key(key) if key.code == expected && key.kind == KeyEventKind::Release => true,
        _ => false,
    }
}

fn command_menu_input(event: &TerminalEvent, menu: &mut Menu<CommandId>, commands: &[MenuCommand]) -> CommandMenuInput {
    let items = command_picker_items(commands);
    match event {
        TerminalEvent::FocusGained | TerminalEvent::FocusLost | TerminalEvent::Resize(_, _) => CommandMenuInput::Pass,
        TerminalEvent::Mouse(_) => CommandMenuInput::Handled,
        TerminalEvent::Paste(text) => {
            menu.paste(text, &items);
            CommandMenuInput::Handled
        }
        TerminalEvent::Key(key) if key.kind == KeyEventKind::Release => CommandMenuInput::Handled,
        TerminalEvent::Key(key) if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) => {
            CommandMenuInput::Pass
        }
        TerminalEvent::Key(key) => {
            let selected = match key.code {
                KeyCode::Esc => {
                    menu.close();
                    return CommandMenuInput::Handled;
                }
                KeyCode::Enter => menu.submit_selected(&items),
                KeyCode::Up => {
                    menu.up(&items);
                    None
                }
                KeyCode::Down => {
                    menu.down(&items);
                    None
                }
                KeyCode::Left => {
                    menu.left();
                    None
                }
                KeyCode::Right => {
                    menu.right();
                    None
                }
                KeyCode::Home => {
                    menu.home();
                    None
                }
                KeyCode::End => {
                    menu.end();
                    None
                }
                KeyCode::Backspace => {
                    menu.backspace(&items);
                    None
                }
                KeyCode::Delete => {
                    menu.delete(&items);
                    None
                }
                KeyCode::Char('p') if key.kind == KeyEventKind::Repeat && menu.query().is_empty() => None,
                KeyCode::Char(digit)
                    if digit.is_ascii_digit()
                        && !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    menu.submit_digit(digit, &items)
                }
                KeyCode::Char(character) if !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
                    menu.insert(character, &items);
                    None
                }
                _ => None,
            };
            selected.map_or(CommandMenuInput::Handled, |id| {
                CommandMenuInput::Submit(
                    commands
                        .iter()
                        .find(|command| command.id == id)
                        .expect("a submitted command came from the current catalog")
                        .action
                        .clone(),
                )
            })
        }
    }
}

fn resolve_pasted_commit(repository: &gix::Repository, pasted: &str) -> Result<gix::ObjectId> {
    let hash = pasted.trim();
    anyhow::ensure!(
        !hash.is_empty() && hash.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "expected exactly one hexadecimal commit ID"
    );
    let object = repository
        .rev_parse(hash.as_bytes().as_bstr())
        .context("could not resolve pasted commit ID")?
        .single()
        .context("pasted commit ID is ambiguous")?
        .object()
        .context("could not read pasted object")?;
    anyhow::ensure!(
        object.kind == gix::object::Kind::Commit,
        "pasted object is not a commit"
    );
    Ok(object.id)
}

fn diagnostic_key(character: char) -> KeyEvent {
    let code = match character {
        '\t' => KeyCode::Tab,
        '\n' | '\r' => KeyCode::Enter,
        '\u{1b}' => KeyCode::Esc,
        character => KeyCode::Char(character),
    };
    let modifiers = if character.is_uppercase() {
        KeyModifiers::SHIFT
    } else {
        KeyModifiers::NONE
    };
    KeyEvent::new(code, modifiers)
}

fn next_diagnostic_input(inputs: &mut VecDeque<KeyEvent>, state: State, lane_computing: bool) -> Option<KeyEvent> {
    (state == State::Complete && !lane_computing)
        .then(|| inputs.pop_front())
        .flatten()
}

fn diagnostic_action(key: KeyEvent, app: &App) -> Option<Action> {
    app_action(key, app).filter(|action| {
        (action_allowed_during_rebase_continuation(Some(action), app.changes_focus.is_some())
            || matches!(action, Action::ToggleActions | Action::ToggleEnrich))
            && !matches!(
                action,
                Action::Copy | Action::CopyPath(_) | Action::CopyAuthor | Action::ForceQuit | Action::Quit
            )
    })
}

fn app_action(key: KeyEvent, app: &App) -> Option<Action> {
    if key.kind != KeyEventKind::Release {
        let shifted =
            key.modifiers.contains(KeyModifiers::SHIFT) || matches!(key.code, KeyCode::Char('H' | 'J' | 'K' | 'L'));
        if shifted {
            if app.changes_focus.is_none() {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k' | 'K') => return Some(Action::TopologicalUp),
                    KeyCode::Down | KeyCode::Char('j' | 'J') => return Some(Action::TopologicalDown),
                    KeyCode::Left | KeyCode::Char('h' | 'H') => return Some(Action::PreviousChild),
                    KeyCode::Right | KeyCode::Char('l' | 'L') => return Some(Action::NextChild),
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k' | 'K') => return Some(Action::MoveUp),
                    KeyCode::Down | KeyCode::Char('j' | 'J') => return Some(Action::MoveDown),
                    KeyCode::Left | KeyCode::Char('h' | 'H') => return Some(Action::ScrollLeft),
                    KeyCode::Right | KeyCode::Char('l' | 'L') => return Some(Action::ScrollRight),
                    _ => {}
                }
            }
        }

        let paging = match key.code {
            KeyCode::PageUp => Some((Action::PageUp, app.viewport_rows.max(1), false)),
            KeyCode::PageDown => Some((Action::PageDown, app.viewport_rows.max(1), true)),
            KeyCode::Char('u' | 'U') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some((Action::HalfPageUp, (app.viewport_rows / 2).max(1), false))
            }
            KeyCode::Char('d' | 'D') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some((Action::HalfPageDown, (app.viewport_rows / 2).max(1), true))
            }
            KeyCode::Char('b' | 'B') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some((Action::PageUp, app.viewport_rows.max(1), false))
            }
            KeyCode::Char('f' | 'F') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some((Action::PageDown, app.viewport_rows.max(1), true))
            }
            _ => None,
        };
        if let Some((cursor_action, distance, down)) = paging {
            let shifted =
                key.modifiers.contains(KeyModifiers::SHIFT) || matches!(key.code, KeyCode::Char('U' | 'D' | 'B' | 'F'));
            if !shifted || app.changes_focus.is_some() || app.commit_paging_active() {
                return Some(cursor_action);
            }
            return Some(if down {
                Action::PanDownBy(distance)
            } else {
                Action::PanUpBy(distance)
            });
        }
    }
    action_with_shortcut_groups(
        key,
        app.history_display_expanded,
        app.actions_expanded,
        app.enrich_expanded,
        app.information_expanded || app.changes_focus.is_some(),
    )
}

fn action_allowed_during_rebase_continuation(action: Option<&Action>, changes_focused: bool) -> bool {
    matches!(
        action,
        None | Some(
            Action::MoveUp
                | Action::MoveDown
                | Action::MoveUpBy(_)
                | Action::MoveDownBy(_)
                | Action::PanUpBy(_)
                | Action::PanDownBy(_)
                | Action::TopologicalUp
                | Action::TopologicalDown
                | Action::PreviousChild
                | Action::NextChild
                | Action::CycleDuplicate
                | Action::ScrollLeft
                | Action::ScrollRight
                | Action::HalfPageUp
                | Action::HalfPageDown
                | Action::PageUp
                | Action::PageDown
                | Action::First
                | Action::Last
                | Action::ToggleDate
                | Action::CycleIds
                | Action::ToggleName
                | Action::ToggleEmail
                | Action::ToggleTrailers
                | Action::ToggleMailmap
                | Action::CycleRefs
                | Action::ToggleRefs
                | Action::ToggleHistoryDisplay
                | Action::ToggleInformation
                | Action::ToggleAlign
                | Action::ToggleCommit
                | Action::ToggleChanges
                | Action::ToggleChangesFocus
                | Action::CycleChangesParent
                | Action::Copy
                | Action::CopyPath(_)
                | Action::CopyAuthor
                | Action::ForceQuit
                | Action::Quit
        )
    ) || (changes_focused && matches!(action, Some(Action::OpenDiff | Action::Cancel | Action::Quit)))
}

fn action_with_shortcut_groups(
    key: KeyEvent,
    history_display_expanded: bool,
    actions_expanded: bool,
    enrich_expanded: bool,
    information_expanded: bool,
) -> Option<Action> {
    if key.kind == KeyEventKind::Release {
        return None;
    }
    match key.code {
        KeyCode::Tab => Some(Action::ToggleChangesFocus),
        KeyCode::Enter => Some(Action::OpenDiff),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::ForceQuit),
        KeyCode::Char('v') => Some(Action::ToggleHistoryDisplay),
        KeyCode::Char('a') => Some(Action::ToggleActions),
        KeyCode::Char('?') => Some(Action::ToggleInformation),
        KeyCode::Char('/') if key.modifiers.contains(KeyModifiers::SHIFT) => Some(Action::ToggleInformation),
        KeyCode::Char('b') if actions_expanded && !key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::Rebase)
        }
        KeyCode::Char('U') => Some(Action::Redo),
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::SHIFT) => Some(Action::Redo),
        KeyCode::Char('u')
            if actions_expanded && !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::SHIFT) =>
        {
            Some(Action::RebaseUpdate)
        }
        KeyCode::Char('u') if !key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::Undo),
        KeyCode::Char('r') if actions_expanded => Some(Action::Review),
        KeyCode::Char('s') if actions_expanded => Some(Action::Squash),
        KeyCode::Char('y') if actions_expanded => Some(Action::CopyInsert),
        KeyCode::Char('m') if actions_expanded => Some(Action::MoveInsert),
        KeyCode::Char('t') if actions_expanded => Some(Action::StackInsert),
        KeyCode::Char('f') if actions_expanded && !key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::ForkCommit)
        }
        KeyCode::Char('h') if actions_expanded => Some(Action::Attach),
        KeyCode::Char('z') if actions_expanded => Some(Action::Stash),
        KeyCode::Char('o') if actions_expanded => Some(Action::Reword),
        KeyCode::Char('w') if actions_expanded => Some(Action::NewCommit),
        KeyCode::Char('n') if actions_expanded => Some(Action::NewEmptyCommit),
        KeyCode::Char('e') if actions_expanded => Some(Action::Amend),
        KeyCode::Char('l') if actions_expanded => Some(Action::Spill),
        KeyCode::Char('p') if actions_expanded && !key.modifiers.contains(KeyModifiers::SHIFT) => Some(Action::Split),
        KeyCode::Char('d') if actions_expanded => Some(Action::Forget),
        KeyCode::Char('i') if actions_expanded => Some(Action::TogglePin),
        KeyCode::Char('n') => Some(Action::ToggleEnrich),
        KeyCode::Char('P') => Some(Action::CycleChangesParent),
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::SHIFT) => Some(Action::CycleChangesParent),
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Esc => Some(Action::Cancel),
        KeyCode::Up | KeyCode::Char('k') => Some(Action::MoveUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::MoveDown),
        KeyCode::Char('x') => Some(Action::CycleDuplicate),
        KeyCode::Char('h') if history_display_expanded => Some(Action::ToggleHidden),
        KeyCode::Char('h') => Some(Action::ScrollLeft),
        KeyCode::Char('l') => Some(Action::ScrollRight),
        KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::PageUp),
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::PageDown),
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::HalfPageUp),
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::HalfPageDown),
        KeyCode::PageUp => Some(Action::PageUp),
        KeyCode::PageDown => Some(Action::PageDown),
        KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::SHIFT) => Some(Action::Last),
        KeyCode::Char('g') if enrich_expanded => Some(Action::EditGitNote),
        KeyCode::Home | KeyCode::Char('g') => Some(Action::First),
        KeyCode::End | KeyCode::Char('G') => Some(Action::Last),
        KeyCode::Char('d') if history_display_expanded => Some(Action::ToggleDate),
        KeyCode::Char('i') if history_display_expanded => Some(Action::CycleIds),
        KeyCode::Char('s') if history_display_expanded => Some(Action::ToggleEmail),
        KeyCode::Char('e') if history_display_expanded => Some(Action::ToggleName),
        KeyCode::Char('t') if history_display_expanded => Some(Action::ToggleTrailers),
        KeyCode::Char('m') if history_display_expanded => Some(Action::ToggleMailmap),
        KeyCode::Char('R') => Some(Action::Refresh),
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::SHIFT) => Some(Action::Refresh),
        KeyCode::Char('r') if history_display_expanded => Some(Action::CycleRefs),
        KeyCode::Char('e') if enrich_expanded => Some(Action::ToggleChecksPass),
        KeyCode::Char('t') if enrich_expanded => Some(Action::ToggleTodo),
        KeyCode::Char('o') if enrich_expanded => Some(Action::EditNote),
        KeyCode::Char('e') if information_expanded => Some(Action::ToggleChanges),
        KeyCode::Char('@') => Some(Action::TimeTravel),
        KeyCode::Char('2') if key.modifiers.contains(KeyModifiers::SHIFT) => Some(Action::TimeTravel),
        KeyCode::Char('m') => Some(Action::ToggleCommit),
        KeyCode::Char('r') => Some(Action::ToggleRefs),
        KeyCode::Char('s') => Some(Action::VerifySignatures),
        KeyCode::Char('t') => Some(Action::ToggleRefTree),
        KeyCode::Char('[') => Some(Action::ToggleAlign),
        KeyCode::Char(']') => Some(Action::ToggleCommit),
        KeyCode::Char('Y') => Some(Action::CopyAuthor),
        KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::SHIFT) => Some(Action::CopyAuthor),
        KeyCode::Char('y') => Some(Action::Copy),
        _ => None,
    }
}

fn copy_selected_path_action(
    action: Action,
    app: &App,
    tree_changes: Option<&Changes>,
    worktree_changes: Option<&Changes>,
) -> Action {
    if action != Action::Copy {
        return action;
    }
    let (pane, changes) = match app.changes_focus {
        Some(pane @ ChangePane::Tree) => (pane, tree_changes),
        Some(pane @ ChangePane::Worktree) => (pane, worktree_changes),
        None => return action,
    };
    changes
        .and_then(|changes| changes.paths.get(app.changes(pane).selected))
        .map_or(action, |change| Action::CopyPath(change.path.clone()))
}

fn repeats_viewport(action: &Action) -> bool {
    matches!(
        action,
        Action::MoveUp
            | Action::MoveDown
            | Action::MoveUpBy(_)
            | Action::MoveDownBy(_)
            | Action::PanUpBy(_)
            | Action::PanDownBy(_)
            | Action::TopologicalUp
            | Action::TopologicalDown
            | Action::HalfPageUp
            | Action::HalfPageDown
            | Action::PageUp
            | Action::PageDown
            | Action::First
            | Action::Last
    )
}

fn retains_fill_repository(kind: KeyEventKind, action: Option<&Action>, changes_focused: bool) -> bool {
    !changes_focused && kind == KeyEventKind::Repeat && action.is_some_and(repeats_viewport)
}

fn mouse_scroll_action(
    kind: MouseEventKind,
    modifiers: KeyModifiers,
    distance: usize,
    changes_focused: bool,
) -> Option<Action> {
    let shifted = modifiers.contains(KeyModifiers::SHIFT);
    match kind {
        MouseEventKind::ScrollUp if shifted || changes_focused => Some(Action::MoveUpBy(distance.max(1))),
        MouseEventKind::ScrollDown if shifted || changes_focused => Some(Action::MoveDownBy(distance.max(1))),
        MouseEventKind::ScrollUp => Some(Action::PanUpBy(distance.max(1))),
        MouseEventKind::ScrollDown => Some(Action::PanDownBy(distance.max(1))),
        MouseEventKind::ScrollLeft if shifted => Some(Action::PreviousChild),
        MouseEventKind::ScrollRight if shifted => Some(Action::NextChild),
        MouseEventKind::ScrollLeft => Some(Action::ScrollLeft),
        MouseEventKind::ScrollRight => Some(Action::ScrollRight),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pasted_commit_ids_are_hex_only_and_must_name_commit_objects() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = test_repository::open(fixture.path())?;
        let commit = repository.rev_parse_single("topic")?.detach();
        let abbreviated = commit.to_hex_with_len(8).to_string();

        assert_eq!(
            resolve_pasted_commit(&repository, &format!("\n{abbreviated}\n"))?,
            commit
        );
        assert_eq!(resolve_pasted_commit(&repository, &commit.to_string())?, commit);
        for invalid in ["topic", "dead beef", ""] {
            assert!(
                resolve_pasted_commit(&repository, invalid).is_err(),
                "{invalid:?} is not exactly one hexadecimal object ID"
            );
        }
        let blob = repository.write_blob(b"not a commit")?.detach();
        assert!(
            resolve_pasted_commit(&repository, &blob.to_string()).is_err(),
            "an existing non-commit object is rejected"
        );
        Ok(())
    }

    #[test]
    fn command_menu_intercepts_text_paste_and_recalls_exact_submissions() {
        let app = App::new(1);
        let commands = command_menu::commands(&app, &Decorations::default(), false);
        let items = command_picker_items(&commands);
        let mut menu = Menu::default();
        menu.open(&items);

        assert_eq!(
            command_menu_input(
                &TerminalEvent::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
                &mut menu,
                &commands,
            ),
            CommandMenuInput::Handled,
            "the first opening has no default command"
        );
        assert_eq!(
            command_menu_input(&TerminalEvent::Paste("r\nf\tt".into()), &mut menu, &commands),
            CommandMenuInput::Handled,
            "paste edits the query instead of reaching commit paste"
        );
        assert_eq!(menu.query(), "rft");
        assert_eq!(
            command_menu_input(
                &TerminalEvent::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
                &mut menu,
                &commands,
            ),
            CommandMenuInput::Submit(Action::ToggleRefTree)
        );

        menu.open(&items);
        assert_eq!(
            command_menu_input(
                &TerminalEvent::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
                &mut menu,
                &commands,
            ),
            CommandMenuInput::Submit(Action::ToggleRefTree),
            "reopening recalls the exact submitted command"
        );
        menu.open(&items);
        assert_eq!(
            command_menu_input(
                &TerminalEvent::Key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE)),
                &mut menu,
                &commands,
            ),
            CommandMenuInput::Submit(Action::ToggleAlign),
            "digits execute their visible row"
        );
    }

    #[test]
    fn command_menu_opener_does_not_steal_prefixed_or_shifted_p() {
        assert!(opens_command_menu(
            &TerminalEvent::Key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE)),
            false,
            false,
        ));
        assert!(!opens_command_menu(
            &TerminalEvent::Key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE)),
            true,
            false,
        ));
        assert!(!opens_command_menu(
            &TerminalEvent::Key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE)),
            false,
            true,
        ));
        assert!(!opens_command_menu(
            &TerminalEvent::Key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::SHIFT)),
            false,
            false,
        ));
        assert!(!opens_command_menu(
            &TerminalEvent::Key(KeyEvent::new(KeyCode::Char('P'), KeyModifiers::NONE)),
            false,
            false,
        ));
        assert!(!opens_command_menu(
            &TerminalEvent::Key(KeyEvent::new_with_kind(
                KeyCode::Char('p'),
                KeyModifiers::NONE,
                KeyEventKind::Repeat,
            )),
            false,
            false,
        ));

        let app = App::new(1);
        let commands = command_menu::commands(&app, &Decorations::default(), false);
        let items = command_picker_items(&commands);
        let mut menu = Menu::default();
        menu.open(&items);
        assert_eq!(
            command_menu_input(
                &TerminalEvent::Key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE)),
                &mut menu,
                &commands,
            ),
            CommandMenuInput::Handled
        );
        assert_eq!(menu.query(), "p", "an open command menu receives p as query text");
    }

    #[test]
    fn command_menu_closing_key_repeats_do_not_reach_the_main_view() {
        let mut suppressed = Some(KeyCode::Enter);
        assert!(swallow_command_menu_key_event(
            &TerminalEvent::Key(KeyEvent::new_with_kind(
                KeyCode::Enter,
                KeyModifiers::NONE,
                KeyEventKind::Repeat,
            )),
            &mut suppressed,
        ));
        assert_eq!(suppressed, Some(KeyCode::Enter));
        assert!(swallow_command_menu_key_event(
            &TerminalEvent::Key(KeyEvent::new_with_kind(
                KeyCode::Enter,
                KeyModifiers::NONE,
                KeyEventKind::Release,
            )),
            &mut suppressed,
        ));
        assert_eq!(suppressed, None);

        let mut suppressed = Some(KeyCode::Esc);
        assert!(!swallow_command_menu_key_event(
            &TerminalEvent::Key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)),
            &mut suppressed,
        ));
        assert_eq!(suppressed, None, "another key ends suppression and remains actionable");
    }

    #[test]
    fn shades_terminal_background_by_one_sixteenth() {
        assert_eq!(shade_terminal_background((0, 0, 0), true), (15, 15, 15));
        assert_eq!(shade_terminal_background((255, 255, 255), false), (240, 240, 240));
        assert_eq!(shade_terminal_background((32, 64, 128), true), (45, 75, 135));
        assert_eq!(shade_terminal_background((32, 64, 128), false), (30, 60, 120));
    }

    #[test]
    fn scans_change_ids_only_for_the_current_filtered_view() {
        let mut app = App::new(1);
        assert!(!change_id_scan_needed(&app), "unrestricted history needs no scan");

        app.configure_hidden_filter(true);
        assert!(
            change_id_scan_needed(&app),
            "excluded hidden tips make the view limited"
        );

        app.show_hidden = true;
        assert!(
            !change_id_scan_needed(&app),
            "expanding hidden history makes the current view unrestricted"
        );
    }

    #[test]
    fn pending_changes_are_recorded_before_they_are_cleared() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = test_repository::open(fixture.path())?;
        let id = repository.rev_parse_single("topic")?.detach();
        let name: gix::refs::FullName = "refs/heads/cancelled-conflict".try_into()?;
        let status = Command::new("git")
            .current_dir(fixture.path())
            .args(["update-ref", name.as_bstr().to_str_lossy().as_ref(), &id.to_string()])
            .status()?;
        assert!(
            status.success(),
            "the cancelled operation has an applied reference change"
        );
        let mut changes = vec![edit::undo::RefChange {
            name: name.clone(),
            before: edit::undo::State::Missing,
            after: edit::undo::State::Object(id),
        }];

        record_and_clear_pending_undo(fixture.path(), false, "materialize rebase conflict", &mut changes)?;
        assert!(changes.is_empty(), "the cancellation releases its accumulator");

        let repository = test_repository::open(fixture.path())?;
        edit::undo::plan_undo(&repository)?
            .expect("the cancelled operation was journalled")
            .apply(&repository)?;
        assert!(
            repository.try_find_reference(name.as_ref())?.is_none(),
            "the recorded cancellation remains undoable"
        );
        Ok(())
    }

    #[test]
    fn recognizes_an_external_conflict_amend_and_keeps_it_undoable() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_conflict.sh")?;
        let repository = test_repository::open(fixture.path())?;
        let original = repository.head_id()?.detach();
        let mut commit = repository.find_commit(original)?.decode()?.into_owned()?;
        let original_parent = commit
            .parents
            .first()
            .copied()
            .context("the fixture tip has a parent")?;
        commit
            .extra_headers
            .push(("tix-rebase-parent".into(), original_parent.to_string().into()));
        let accepted = repository.write_object(&commit)?.detach();
        let status = Command::new("git")
            .arg("-C")
            .arg(fixture.path())
            .args([
                "update-ref",
                "refs/heads/main",
                &accepted.to_string(),
                &original.to_string(),
            ])
            .status()?;
        assert!(status.success(), "the fixture checks out a materialized pending commit");
        let head = conflict_head(fixture.path(), false, accepted)?;
        let mut pending = Some(PendingConflictResolution {
            commit: accepted,
            head: Some(head),
            ref_changes: Vec::new(),
            record_undo: true,
        });
        let status = Command::new("git")
            .arg("-C")
            .arg(fixture.path())
            .args(["commit", "--amend", "-qm", "externally resolved"])
            .status()?;
        assert!(status.success(), "git performs the external amend");
        let replacement = test_repository::open(fixture.path())?.head_id()?.detach();
        assert_ne!(replacement, accepted, "the external amend replaces HEAD");
        let replacement_commit = test_repository::open(fixture.path())?
            .find_commit(replacement)?
            .decode()?
            .into_owned()?;
        assert!(
            edit::rebase::is_pending(&replacement_commit),
            "git preserves the pending marker while amending"
        );

        let mut app = App::new(1);
        assert_eq!(
            reconcile_external_conflict_reporting(&mut app, fixture.path(), false, &mut pending),
            ConflictReconcileStatus::Complete,
            "a clean same-parent replacement completes conflict resolution"
        );
        assert!(pending.is_none(), "completed conflict state is released");

        let repository = test_repository::open(fixture.path())?;
        let finalized = repository.head_id()?.detach();
        assert_ne!(finalized, replacement, "Tix strips the preserved pending marker");
        assert!(
            !edit::rebase::is_pending(&repository.find_commit(finalized)?.decode()?.into_owned()?),
            "the recognized resolution is fully materialized"
        );
        edit::undo::plan_undo(&repository)?
            .expect("the external amend was added to the undo queue")
            .apply(&repository)?;
        assert_eq!(
            repository.head_id()?.detach(),
            accepted,
            "undo restores the materialized conflict commit"
        );
        Ok(())
    }

    #[test]
    fn external_conflict_resolution_requires_a_commit_and_rejects_unrelated_head_moves() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_conflict.sh")?;
        let accepted = test_repository::open(fixture.path())?.head_id()?.detach();
        let head = conflict_head(fixture.path(), false, accepted)?;
        let mut pending = Some(PendingConflictResolution {
            commit: accepted,
            head: Some(head),
            ref_changes: Vec::new(),
            record_undo: true,
        });
        std::fs::write(fixture.path().join("file"), "resolved but not committed\n")?;
        let status = Command::new("git")
            .arg("-C")
            .arg(fixture.path())
            .args(["add", "file"])
            .status()?;
        assert!(status.success(), "the resolution is staged");
        let mut app = App::new(1);
        assert_eq!(
            reconcile_external_conflict_reporting(&mut app, fixture.path(), false, &mut pending),
            ConflictReconcileStatus::Amend,
            "staging alone still requires an amend"
        );
        assert!(pending.is_some(), "staged state remains mandatory");

        let status = Command::new("git")
            .arg("-C")
            .arg(fixture.path())
            .args(["reset", "--hard", "HEAD~1"])
            .status()?;
        assert!(status.success(), "git moves HEAD away from the conflict checkout");
        assert_eq!(
            reconcile_external_conflict_reporting(&mut app, fixture.path(), false, &mut pending),
            ConflictReconcileStatus::Blocked,
            "an unrelated external HEAD move is not mistaken for conflict resolution"
        );
        assert!(
            pending.is_some(),
            "invalid external movement remains explicit until exit"
        );
        Ok(())
    }

    #[test]
    fn retains_unseen_filesystem_redraws_until_focus_returns() {
        assert!(!unseen_filesystem_redraw(false, false, false));
        assert!(unseen_filesystem_redraw(false, false, true));
        assert!(unseen_filesystem_redraw(true, false, false));
        assert!(!unseen_filesystem_redraw(true, true, true));
    }

    #[test]
    fn follows_a_reference_across_a_rewrite() {
        let old = gix::ObjectId::Sha1([1; 20]);
        let new = gix::ObjectId::Sha1([2; 20]);
        let decoration = history::Decoration {
            name: "refs/patches/topic/selected".into(),
            kind: history::DecorationKind::Special,
        };
        let current = Decorations::from([(old, vec![decoration.clone()])]);
        let next = Decorations::from([(new, vec![decoration])]);

        assert_eq!(decoration_successor(old, &current, &next), Some(new));
    }

    #[test]
    fn finds_the_current_worktree_branch_in_attached_and_remembered_states() {
        let id = gix::ObjectId::Sha1([1; 20]);
        let mut refs = history::RefSnapshot {
            view: Default::default(),
            hidden: Default::default(),
            view_tips: Vec::new(),
            hidden_tips: Vec::new(),
            pins: Vec::new(),
            worktrees: vec![history::WorktreeCheckout {
                id,
                label_id: id,
                checkout_name: "main".into(),
                reference: Some("refs/heads/main".try_into().expect("valid branch name")),
                is_current: true,
                is_detached: false,
            }],
        };
        assert_eq!(current_worktree_branch(&refs), Some((id, false)));

        refs.worktrees[0].is_detached = true;
        assert_eq!(current_worktree_branch(&refs), Some((id, true)));
        refs.worktrees[0].reference = None;
        assert_eq!(
            current_worktree_branch(&refs),
            None,
            "manual detachment has no branch to move"
        );
    }

    #[test]
    fn reference_watcher_observes_new_loose_refs() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = test_repository::open(fixture.path())?;
        let watcher = start_ref_watcher(repository.git_dir(), repository.common_dir())?;
        let topic = repository.rev_parse_single("topic")?.detach();
        let status = Command::new("git")
            .current_dir(fixture.path())
            .args(["update-ref", "refs/heads/watched", &topic.to_hex().to_string()])
            .status()?;
        assert!(status.success(), "git updates a loose reference");

        let deadline = Instant::now() + Duration::from_secs(5);
        let mut paths = Vec::new();
        let watched = repository.git_dir().join("refs/heads/watched");
        while Instant::now() < deadline {
            let event = watcher
                .events
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))??;
            if !notification_is_actionable(&event) {
                continue;
            }
            paths.extend(event.paths);
            if watched.is_file() {
                break;
            }
        }
        assert!(
            watched.is_file(),
            "the completed loose-reference transaction is actionable: {paths:?}"
        );
        Ok(())
    }

    #[test]
    fn worktree_status_head_changes_only_with_head() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = test_repository::open(fixture.path())?;
        let baseline = worktree_status_head(&repository)?;
        let main = repository.rev_parse_single("main")?.detach();
        let topic = repository.rev_parse_single("topic")?.detach();
        drop(repository);

        let update_ref = |name: &str, target: gix::ObjectId| -> gix_testtools::Result {
            let status = Command::new("git")
                .current_dir(fixture.path())
                .args(["update-ref", name, &target.to_string()])
                .status()?;
            assert!(status.success(), "git updates {name}");
            Ok(())
        };
        update_ref("refs/heads/unrelated", topic)?;
        assert_eq!(
            worktree_status_head(&test_repository::open(fixture.path())?)?,
            baseline,
            "an unrelated ref does not affect worktree status"
        );

        update_ref("refs/heads/alias", main)?;
        let status = Command::new("git")
            .current_dir(fixture.path())
            .args(["symbolic-ref", "HEAD", "refs/heads/alias"])
            .status()?;
        assert!(status.success(), "git reattaches HEAD to the alias");
        let alias = worktree_status_head(&test_repository::open(fixture.path())?)?;
        assert_ne!(alias, baseline, "the symbolic referent is part of the status baseline");
        assert_eq!(
            alias.target, baseline.target,
            "the alias initially names the same commit"
        );
        let mut cached = Some(baseline.clone());
        remember_worktree_status_head(&mut cached, false, Ok(alias.clone()));
        assert_eq!(
            cached,
            Some(baseline.clone()),
            "an unstaged-only refresh preserves the HEAD baseline"
        );
        remember_worktree_status_head(&mut cached, true, Ok(alias.clone()));
        assert_eq!(cached, Some(alias.clone()), "a staged refresh advances the baseline");

        update_ref("refs/heads/alias", topic)?;
        let moved = worktree_status_head(&test_repository::open(fixture.path())?)?;
        assert_ne!(
            moved.target, alias.target,
            "moving the checked-out ref changes the baseline"
        );

        let status = Command::new("git")
            .current_dir(fixture.path())
            .args(["symbolic-ref", "HEAD", "refs/heads/unborn"])
            .status()?;
        assert!(status.success(), "git makes HEAD unborn");
        let unborn = worktree_status_head(&test_repository::open(fixture.path())?)?;
        assert_eq!(
            unborn.reference.as_ref().map(gix::refs::FullName::as_bstr),
            Some("refs/heads/unborn".into())
        );
        assert_eq!(unborn.target, None, "an unborn HEAD has no peeled target");
        Ok(())
    }

    #[test]
    fn caches_recent_tree_changes_by_commit_and_parent() {
        let id = |value| {
            let mut bytes = [0; 20];
            bytes[19] = value;
            gix::ObjectId::Sha1(bytes)
        };
        let mut cache = TreeChangesCache::default();
        cache.insert((
            app::TreeDiffTarget::Commit { id: id(42), parent: 0 },
            Changes::default(),
        ));
        cache.insert((
            app::TreeDiffTarget::Commit { id: id(42), parent: 1 },
            Changes {
                lines_added: 42,
                ..Changes::default()
            },
        ));
        assert!(cache.activate(app::TreeDiffTarget::Commit { id: id(42), parent: 0 }));
        assert_eq!(
            cache.as_ref().map(|(target, _)| *target),
            Some(app::TreeDiffTarget::Commit { id: id(42), parent: 0 })
        );
        assert!(cache.activate(app::TreeDiffTarget::Commit { id: id(42), parent: 1 }));
        assert_eq!(
            cache.as_ref().map(|(_, changes)| changes.lines_added),
            Some(42),
            "each merge parent retains its own diff result"
        );
        cache.insert((
            app::TreeDiffTarget::Branch {
                base: id(42),
                tip: id(43),
            },
            Changes {
                lines_removed: 43,
                ..Changes::default()
            },
        ));
        assert!(cache.activate(app::TreeDiffTarget::Commit { id: id(42), parent: 1 }));
        assert_eq!(
            cache.as_ref().map(|(_, changes)| changes.lines_added),
            Some(42),
            "a branch range cannot replace the base commit's ordinary diff"
        );
        cache.clear();

        for value in 0..=TREE_CHANGES_CACHE_SIZE as u8 {
            cache.insert((
                app::TreeDiffTarget::Commit {
                    id: id(value),
                    parent: usize::from(value),
                },
                Changes {
                    lines_added: u64::from(value),
                    ..Changes::default()
                },
            ));
        }

        assert!(
            cache.activate(app::TreeDiffTarget::Commit { id: id(1), parent: 1 }),
            "a recently viewed commit and parent restores its computed diff"
        );
        assert_eq!(cache.as_ref().map(|(_, changes)| changes.lines_added), Some(1));
        assert!(
            !cache.activate(app::TreeDiffTarget::Commit { id: id(0), parent: 0 }),
            "the oldest entry is evicted at the bound"
        );
        cache.clear();
        assert!(
            cache.as_ref().is_none(),
            "closing the changes view releases cached diffs"
        );
    }

    #[test]
    fn copies_the_selected_path_from_the_focused_changes_block() {
        let mut app = App::new(1);
        app.changes_focus = Some(ChangePane::Tree);
        app.set_changes_bounds(ChangePane::Tree, 2, 2, None, 80, 0);
        drop(app.update(Action::MoveDown));
        let changes = Changes {
            paths: ["first", "dir/second"]
                .into_iter()
                .map(|path| app::PathChange {
                    kind: ChangeKind::Modified,
                    group: ChangeGroup::Tree,
                    source: None,
                    path: path.into(),
                    lines: None,
                })
                .collect(),
            ..Changes::default()
        };

        assert_eq!(
            copy_selected_path_action(Action::Copy, &app, Some(&changes), None),
            Action::CopyPath("dir/second".into())
        );
        app.changes_focus = None;
        assert_eq!(
            copy_selected_path_action(Action::Copy, &app, Some(&changes), None),
            Action::Copy,
            "history retains commit-id copying"
        );
    }

    #[test]
    fn loads_commit_messages_from_an_existing_repository() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_read_only("history.sh")?;
        let repository = test_repository::open(&fixture)?;
        let id = repository.rev_parse_single("topic")?.detach();

        assert!(
            load_commit_message(&repository, id)?.starts_with(b"topic\n\n--- agent\n\nCo-authored-by:"),
            "on-demand loading retains the full commit message"
        );
        Ok(())
    }

    #[test]
    fn creates_replaces_and_removes_a_git_note() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = test_repository::open(fixture.path())?;
        let id = repository.rev_parse_single("topic")?.detach();
        let notes = repository.notes()?;
        let reference = notes
            .default_ref()
            .expect("the test repository has a default notes ref")
            .to_owned();

        for expected in [b"first".as_slice(), b"second".as_slice()] {
            set_git_note(&repository, reference.as_ref(), id, Some(expected))?;
            let mut notes = repository.notes()?.with_refs([reference.as_bstr()])?;
            assert_eq!(
                notes.get(id)?.first().map(|note| note.blob.data.as_slice()),
                Some(expected),
                "the default note is created or replaced"
            );
        }

        set_git_note(&repository, reference.as_ref(), id, None)?;
        let mut notes = repository.notes()?.with_refs([reference.as_bstr()])?;
        assert!(notes.get(id)?.is_empty(), "empty editor content removes the note");
        Ok(())
    }

    #[test]
    fn selection_relation_prefers_tracking_counts_and_handles_missing_upstreams() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_read_only("history.sh")?;
        let repository = test_repository::open(&fixture)?;
        let topic = repository.rev_parse_single("topic")?.detach();
        let main = repository.rev_parse_single("main")?.detach();
        let authors =
            gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(Authors::default()));
        let mut graph = None;
        history::load(
            &repository,
            &[OsString::from("topic"), OsString::from("main")],
            &[],
            false,
            &authors,
            &AtomicBool::new(false),
            |event| {
                if let Event::Complete(value) = event {
                    graph = Some(value);
                }
                true
            },
        )?;
        let mut graph = graph.expect("history traversal returns its graph");
        let tracking = SelectionRef {
            name: "topic".into(),
            upstream: Some(Some(main)),
        };
        assert_eq!(
            graph.selection_relation(topic, &[tracking.clone(), tracking], &[]),
            Some(SelectionRelation::Tracking { ahead: 1, behind: 2 }),
            "one upstream comparison wins over the visible-history fallback"
        );
        assert_eq!(
            graph.selection_relation(
                topic,
                &[SelectionRef {
                    name: "topic".into(),
                    upstream: Some(None),
                }],
                &[],
            ),
            None,
            "a configured but missing tracking ref does not masquerade as an untracked branch"
        );
        assert_eq!(
            graph.selection_relation(
                topic,
                &[SelectionRef {
                    name: "tag: topic".into(),
                    upstream: None,
                }],
                &[main],
            ),
            Some(SelectionRelation::Visible(1))
        );
        Ok(())
    }

    #[test]
    fn selection_refs_resolve_the_configured_fetch_tracking_branch() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let path = fixture.path();
        for args in [
            ["config", "remote.origin.url", "https://example.com/repo"],
            ["config", "remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*"],
            ["config", "branch.topic.remote", "origin"],
            ["config", "branch.topic.merge", "refs/heads/main"],
        ] {
            let status = std::process::Command::new("git")
                .current_dir(path)
                .args(args)
                .status()?;
            assert!(status.success(), "git config prepares the tracking relationship");
        }
        let repository = test_repository::open(path)?;
        let topic = repository.rev_parse_single("topic")?.detach();
        let main = repository.rev_parse_single("main")?.detach();
        let status = std::process::Command::new("git")
            .current_dir(path)
            .args(["update-ref", "refs/remotes/origin/main", &main.to_hex().to_string()])
            .status()?;
        assert!(status.success(), "the configured tracking ref exists");
        let repository = test_repository::open(path)?;
        let authors =
            gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(Authors::default()));
        let mut graph = None;
        history::load(
            &repository,
            &[OsString::from("topic")],
            &[],
            false,
            &authors,
            &AtomicBool::new(false),
            |event| {
                if let Event::Complete(value) = event {
                    graph = Some(value);
                }
                true
            },
        )?;
        let mut graph = graph.expect("history traversal returns its graph");
        let refs = graph.selection_refs(topic, &history::decorations(&repository, &[], &[])?);
        assert_eq!(refs[0].upstream, Some(Some(main)));
        assert_eq!(
            graph.selection_relation(topic, &refs, &[]),
            Some(SelectionRelation::Tracking { ahead: 1, behind: 2 }),
            "the dynamically scheduled upstream has enough cached ancestry for comparison"
        );
        Ok(())
    }

    #[test]
    fn pending_rebase_changes_use_the_recorded_parent() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = crate::test_repository::open(fixture.path())?;
        let actual_parent = repository.rev_parse_single("main")?.detach();

        let topic = repository.rev_parse_single("topic")?.detach();
        let mut marked = repository.find_commit(topic)?.decode()?.into_owned()?;
        let original_parent = marked.parents[0];
        marked.parents = [actual_parent].into_iter().collect();
        marked
            .extra_headers
            .push(("tix-rebase-parent".into(), original_parent.to_hex().to_string().into()));
        let marked_topic = repository.write_object(&marked)?.detach();
        let changes = load_changes_without_lines(
            &repository,
            app::TreeDiffTarget::Commit {
                id: marked_topic,
                parent: 0,
            },
        )?;
        assert_eq!(
            changes
                .paths
                .iter()
                .map(|change| change.path.as_bstr())
                .collect::<Vec<_>>(),
            ["topic", "topic-extra"],
            "the recorded parent preserves the commit's original changes"
        );
        assert_eq!(
            changes.parent, None,
            "the actual parent isn't presented as the comparison base"
        );

        let root = repository.rev_parse_single("v1^{}")?.detach();
        let mut marked_root = repository.find_commit(root)?.decode()?.into_owned()?;
        marked_root.parents = [actual_parent].into_iter().collect();
        marked_root.extra_headers.push((
            "tix-rebase-parent".into(),
            gix::ObjectId::null(repository.object_hash())
                .to_hex()
                .to_string()
                .into(),
        ));
        let marked_root = repository.write_object(&marked_root)?.detach();
        let changes = load_changes_without_lines(
            &repository,
            app::TreeDiffTarget::Commit {
                id: marked_root,
                parent: 0,
            },
        )?;
        assert_eq!(
            changes
                .paths
                .iter()
                .map(|change| change.path.as_bstr())
                .collect::<Vec<_>>(),
            ["root"],
            "a recorded null parent compares the pending root to the empty tree"
        );
        Ok(())
    }

    #[test]
    fn loads_changes_against_each_merge_parent() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_read_only("history.sh")?;
        let repository = crate::test_repository::open(&fixture)?;
        let mut line_diff_pool_slot = None;
        sync_line_diff_pool(&mut line_diff_pool_slot, true, &fixture, false, 2);
        let line_diff_pool = line_diff_pool_slot.as_mut().expect("changes enable the line diff pool");
        assert!(
            line_diff_pool.active.is_none(),
            "workers start only with an uncached diff"
        );

        let root = load_changes(
            &repository,
            app::TreeDiffTarget::Commit {
                id: repository.rev_parse_single("v1^{}")?.detach(),
                parent: 0,
            },
            line_diff_pool,
        )?;
        assert_eq!(
            line_diff_pool.active.as_ref().map(|active| active.workers.len()),
            Some(2),
            "an uncached diff creates the requested workers"
        );
        let started = Instant::now();
        line_diff_pool.last_used = Some(started);
        let just_before_expiry = started + LINE_DIFF_POOL_IDLE.saturating_sub(Duration::from_nanos(1));
        assert_eq!(
            line_diff_pool.idle_timeout(just_before_expiry),
            Some(Duration::from_nanos(1)),
            "the event loop can wake exactly when workers expire"
        );
        assert!(
            !line_diff_pool.expire(just_before_expiry),
            "workers remain available until their idle timeout"
        );
        assert!(
            line_diff_pool.expire(started + LINE_DIFF_POOL_IDLE),
            "workers expire exactly at their idle timeout"
        );
        assert!(line_diff_pool.active.is_none(), "expiry releases every worker");
        assert_eq!(
            root.paths,
            [PathChange {
                kind: ChangeKind::Added,
                group: ChangeGroup::Tree,
                source: None,
                path: "root".into(),
                lines: Some((1, 0)),
            }],
            "root commits are compared to the empty tree"
        );
        assert_eq!((root.parent, root.lines_added, root.lines_removed), (None, 1, 0));
        assert_eq!(root.diffs.len(), 1, "the original change is retained for file diffs");
        match prepare_file_diff_with_repository(&repository, &root.diffs[0], &root.paths[0])? {
            FileDiff::BuiltIn(diff) => {
                assert_eq!(diff.title, "A root");
                assert!(diff.lines.iter().any(|line| line == "+root"));
            }
            FileDiff::External(_) => unreachable!("isolated repositories have no external diff"),
            FileDiff::Pager { .. } => unreachable!("isolated repositories have no pager"),
        }

        let external_repository = crate::test_repository::open_with(&fixture, ["diff.external=test --flag"])?;
        match prepare_file_diff_with_repository(&external_repository, &root.diffs[0], &root.paths[0])? {
            FileDiff::External(command) => assert!(
                command
                    .get_args()
                    .any(|arg| arg.to_string_lossy().contains("test --flag")),
                "the configured helper is prepared with shell semantics"
            ),
            FileDiff::BuiltIn(_) => unreachable!("configured external diffs take precedence"),
            FileDiff::Pager { .. } => unreachable!("configured external diffs take precedence"),
        }

        let pager_repository = crate::test_repository::open_with(&fixture, ["core.pager=delta --dark"])?;
        match prepare_file_diff_with_repository(&pager_repository, &root.diffs[0], &root.paths[0])? {
            FileDiff::Pager { command, diff } => {
                assert!(
                    command
                        .get_args()
                        .any(|arg| arg.to_string_lossy().contains("delta --dark")),
                    "the configured pager is prepared with shell semantics"
                );
                let mut patch = Vec::new();
                diff.write_to(&mut patch)?;
                assert!(patch.starts_with(b"--- /dev/null\n+++ b/root\n"));
                assert!(patch.ends_with(b"\n"), "pagers receive a complete final line");
            }
            FileDiff::BuiltIn(_) | FileDiff::External(_) => {
                unreachable!("configured pagers receive built-in diffs")
            }
        }

        for setting in ["core.pager=", "core.pager=cat"] {
            let repository = test_repository::open_with(&fixture, [setting])?;
            assert!(
                matches!(
                    prepare_file_diff_with_repository(&repository, &root.diffs[0], &root.paths[0])?,
                    FileDiff::BuiltIn(_)
                ),
                "disabled pagers retain the built-in viewer"
            );
        }

        let topic_id = repository.rev_parse_single("topic")?.detach();
        let topic_target = app::TreeDiffTarget::Commit {
            id: topic_id,
            parent: 0,
        };
        let topic = load_changes(&repository, topic_target, line_diff_pool)?;
        assert!(
            line_diff_pool.active.is_some(),
            "the next uncached diff recreates workers"
        );
        assert_eq!(
            topic.paths,
            [
                PathChange {
                    kind: ChangeKind::Added,
                    group: ChangeGroup::Tree,
                    source: None,
                    path: "topic".into(),
                    lines: Some((1, 0)),
                },
                PathChange {
                    kind: ChangeKind::Added,
                    group: ChangeGroup::Tree,
                    source: None,
                    path: "topic-extra".into(),
                    lines: Some((1, 0)),
                }
            ],
            "parallel line diffs retain tree-diff order and status"
        );
        assert_eq!((topic.lines_added, topic.lines_removed), (2, 0));
        let title: BString = format!("{} author topic", topic_id.to_hex_with_len(7)).into();
        let commit_diff = prepare_commit_diff_with_repository(&repository, topic_target, None, title.clone())?;
        assert!(commit_diff.external.is_empty());
        let FileDiff::BuiltIn(diff) = commit_diff.internal else {
            unreachable!("an isolated repository uses the built-in commit viewer")
        };
        assert_eq!(diff.title, title);
        let summary = diff
            .summary
            .as_ref()
            .expect("whole-commit diffs have a summary")
            .last()
            .expect("the aggregate follows path statistics")
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert!(
            summary.contains("A 2 · +2"),
            "the existing diff pass supplies aggregate counts"
        );
        let topic_position = diff
            .lines
            .iter()
            .position(|line| line == "+++ b/topic")
            .expect("the first path is present");
        let extra_position = diff
            .lines
            .iter()
            .position(|line| line == "+++ b/topic-extra")
            .expect("the second path is present");
        assert!(
            topic_position < extra_position,
            "whole-commit patches retain tree-diff order"
        );
        let base = repository.rev_parse_single("v1^{}")?.detach();
        let branch_target = app::TreeDiffTarget::Branch { base, tip: topic_id };
        let branch = load_changes(&repository, branch_target, line_diff_pool)?;
        assert_eq!(branch.range, Some(app::ComparedRange { base, tip: topic_id }));
        assert_eq!(
            branch
                .paths
                .iter()
                .map(|change| change.path.as_bstr())
                .collect::<Vec<_>>(),
            ["main", "topic", "topic-extra"],
            "branch diffs compare the boundary tree directly to its unique leaf"
        );
        let branch_diff =
            prepare_commit_diff_with_repository(&repository, branch_target, Some(&branch), "branch".into())?;
        let FileDiff::BuiltIn(branch_diff) = branch_diff.internal else {
            unreachable!("an isolated repository uses the built-in branch viewer")
        };
        assert!(
            branch_diff
                .summary
                .expect("branch diffs have a summary")
                .last()
                .expect("the branch aggregate follows path statistics")
                .to_string()
                .contains(&format!("{}..{}", base.to_hex_with_len(7), topic_id.to_hex_with_len(7))),
            "the whole-diff viewer identifies the compared range"
        );
        let empty =
            prepare_commit_diff_with_repository(&repository, topic_target, Some(&Changes::default()), title.clone())?;
        let FileDiff::BuiltIn(empty) = empty.internal else {
            unreachable!("empty commits retain the built-in viewer")
        };
        assert!(empty.lines.is_empty(), "an empty commit opens an empty patch");
        assert!(
            empty
                .summary
                .expect("empty commits have a summary")
                .iter()
                .flat_map(|line| &line.spans)
                .any(|span| span.content.contains("No changes")),
            "empty commits explain the absent patch"
        );

        let pager_diff =
            prepare_commit_diff_with_repository(&pager_repository, topic_target, Some(&topic), title.clone())?;
        assert!(pager_diff.external.is_empty());
        let FileDiff::Pager { diff, .. } = pager_diff.internal else {
            unreachable!("one configured pager receives the aggregate commit patch")
        };
        let mut streamed = Vec::new();
        diff.write_to(&mut streamed)?;
        assert!(
            streamed.starts_with(
                format!("{title}\n topic       | 1 + +1\n topic-extra | 1 + +1\nroot · A 2 · +2 \n\n").as_bytes()
            ),
            "the pager receives path statistics and the aggregate before the patch"
        );
        let external_diff =
            prepare_commit_diff_with_repository(&external_repository, topic_target, Some(&topic), title.clone())?;
        assert_eq!(
            external_diff.external.len(),
            2,
            "external diff commands remain per-path"
        );
        let FileDiff::BuiltIn(summary) = external_diff.internal else {
            unreachable!("an all-external commit still shows its summary")
        };
        assert!(
            summary.lines.is_empty(),
            "external patches aren't duplicated internally"
        );

        let merge = repository.rev_parse_single("main")?.detach();
        let first_parent_target = app::TreeDiffTarget::Commit { id: merge, parent: 0 };
        let first_parent = load_changes(&repository, first_parent_target, line_diff_pool)?;
        assert_eq!(
            first_parent.parent,
            Some(ComparedParent {
                index: 0,
                total: 2,
                id: repository.rev_parse_single("main^1")?.detach(),
            })
        );
        assert_eq!(
            first_parent.paths,
            [PathChange {
                kind: ChangeKind::Added,
                group: ChangeGroup::Tree,
                source: None,
                path: "merged".into(),
                lines: Some((1, 0)),
            }],
            "the default merge diff compares the result to its first parent"
        );

        let second_parent_target = app::TreeDiffTarget::Commit { id: merge, parent: 1 };
        let second_parent = load_changes(&repository, second_parent_target, line_diff_pool)?;
        assert_eq!(
            second_parent.parent,
            Some(ComparedParent {
                index: 1,
                total: 2,
                id: repository.rev_parse_single("main^2")?.detach(),
            })
        );
        assert_eq!(
            second_parent.paths,
            [PathChange {
                kind: ChangeKind::Added,
                group: ChangeGroup::Tree,
                source: None,
                path: "main".into(),
                lines: Some((1, 0)),
            }],
            "later parents can be selected independently"
        );
        let second_parent_diff = prepare_commit_diff_with_repository(
            &repository,
            second_parent_target,
            Some(&second_parent),
            "merge title".into(),
        )?;
        let FileDiff::BuiltIn(diff) = second_parent_diff.internal else {
            unreachable!("an isolated repository uses the built-in commit viewer")
        };
        assert!(
            diff.summary
                .expect("merge diff has a summary")
                .iter()
                .flat_map(|line| &line.spans)
                .any(|span| span.content.contains("vs parent 2/2")),
            "the commit viewer identifies the selected merge parent"
        );
        assert_eq!(
            load_changes(
                &repository,
                app::TreeDiffTarget::Commit { id: merge, parent: 2 },
                line_diff_pool,
            )?
            .parent,
            first_parent.parent,
            "parent selection wraps around"
        );
        sync_line_diff_pool(&mut line_diff_pool_slot, false, &fixture, false, 2);
        assert!(
            line_diff_pool_slot.is_none(),
            "hiding changes immediately releases the pool"
        );
        Ok(())
    }

    #[test]
    fn configures_a_common_repository_as_bare_for_tree_changes() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_read_only("history.sh")?;
        let git_dir = test_repository::open(&fixture)?.git_dir().to_owned();
        let repository = open_repository(&git_dir, true, false)?;

        assert_eq!(
            repository.config_snapshot().boolean("core.bare"),
            Some(true),
            "repository configuration suppresses worktree operations"
        );
        let mut line_diff_pool = LineDiffPool::new(&git_dir, true, 1);
        let root = repository.rev_parse_single("v1^{}")?.detach();
        assert_eq!(
            load_changes(
                &repository,
                app::TreeDiffTarget::Commit { id: root, parent: 0 },
                &mut line_diff_pool,
            )?
            .paths
            .len(),
            1,
            "tree changes remain available without a worktree"
        );
        Ok(())
    }

    #[test]
    fn detects_a_removed_per_worktree_repository_even_if_the_current_directory_resolves() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_read_only("history.sh")?;
        assert!(
            std::env::current_dir().is_ok(),
            "the process directory remains available"
        );
        let missing = fixture.join("missing-worktree-git-dir");
        assert!(worktree_repository_is_gone(&missing));
        let Err(err) = recover_common_repository(&missing) else {
            panic!("a missing common repository cannot be recovered")
        };
        assert!(
            format!("{err:#}").contains("could not change directory to common repository"),
            "recovery failures retain actionable context"
        );
        Ok(())
    }

    #[test]
    fn normalizes_a_common_directory_through_a_missing_per_worktree_directory() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_read_only("history.sh")?;
        let git_dir = test_repository::open(&fixture)?.git_dir().to_owned();
        let indirect = git_dir.join("worktrees/missing/../..");
        assert!(
            !git_dir.join("worktrees/missing").exists(),
            "the intermediate path is absent"
        );
        assert_eq!(normalize_common_dir(indirect)?, git_dir);
        Ok(())
    }

    #[test]
    fn startup_validation_returns_only_detached_hidden_revision_data() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_read_only("history.sh")?;
        let repository = test_repository::open(&fixture)?;
        let mut git_dir = repository.git_dir().to_owned();
        let common_dir = repository.common_dir().to_owned();
        drop(repository);

        let (hide, unavailable) = validate_hidden_revisions(&mut git_dir, &common_dir, &[OsString::from("main")])?;
        assert_eq!(hide, [OsString::from("main")]);
        assert!(unavailable.is_empty(), "the fixture's main branch resolves");
        Ok(())
    }

    #[test]
    fn opens_the_common_repository_when_the_initial_worktree_is_already_gone() -> gix_testtools::Result {
        const COMMON_DIR: &str = "GIX_TIX_TEST_REMOVED_WORKTREE_COMMON_DIR";
        if let Some(git_dir) = std::env::var_os(COMMON_DIR).map(PathBuf::from) {
            let mut stale_git_dir = git_dir.join("worktrees/deleted");
            let (repository, recovered) = open_history_repository(&mut stale_git_dir, &git_dir)?;

            assert!(
                recovered,
                "a missing per-worktree repository uses the common repository"
            );
            assert_eq!(stale_git_dir, git_dir, "future opens use the surviving repository");
            assert_eq!(
                repository.config_snapshot().boolean("core.bare"),
                Some(true),
                "recovery configures the common repository as bare"
            );

            let mut stale_git_dir = git_dir.join("worktrees/deleted-during-event-loop");
            let mut bare = false;
            assert!(
                recover_event_loop_repository(&mut stale_git_dir, &git_dir, &mut bare)?.is_some(),
                "the event-loop boundary recovers before its next action"
            );
            assert_eq!(
                stale_git_dir, git_dir,
                "future event-loop opens use the common repository"
            );
            assert!(bare, "future event-loop opens treat the common repository as bare");
            return Ok(());
        }

        let fixture = gix_testtools::scripted_fixture_read_only("history.sh")?;
        let git_dir = test_repository::open(&fixture)?.git_dir().canonicalize()?;
        let status = Command::new(std::env::current_exe()?)
            .env(COMMON_DIR, git_dir)
            .args([
                "--exact",
                "tests::opens_the_common_repository_when_the_initial_worktree_is_already_gone",
            ])
            .status()?;
        assert!(status.success(), "the isolated recovery process completes successfully");
        Ok(())
    }

    #[test]
    fn loads_staged_and_unstaged_worktree_changes() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let path = fixture.path();
        let git = |args: &[&str]| -> std::io::Result<std::process::ExitStatus> {
            std::process::Command::new("git")
                .current_dir(path)
                .args(["-c", "commit.gpgsign=false"])
                .args(args)
                .status()
        };

        assert!(git(&["switch", "-q", "-c", "conflict-other"])?.success());
        std::fs::write(path.join("root"), "other\n")?;
        assert!(git(&["commit", "-qam", "other"])?.success());
        assert!(git(&["switch", "-q", "main"])?.success());
        std::fs::write(path.join("root"), "ours\n")?;
        assert!(git(&["commit", "-qam", "ours"])?.success());
        assert!(
            !git(&["merge", "--no-edit", "conflict-other"])?.success(),
            "the fixture deliberately leaves an unresolved path"
        );

        std::fs::write(path.join("staged"), "staged\n")?;
        std::fs::write(path.join("both"), "index\n")?;
        assert!(git(&["add", "staged", "both"])?.success());
        std::fs::write(path.join("both"), "index\nworktree\n")?;
        std::fs::write(path.join("untracked"), "untracked\n")?;
        std::fs::write(path.join(".git/info/exclude"), "ignored\n")?;
        std::fs::write(path.join("ignored"), "ignored\n")?;

        let repository = test_repository::open(path)?;
        let mut line_diff_pool = LineDiffPool::new(path, false, 2);
        let changes = load_worktree_changes(&repository, &mut line_diff_pool)?;
        let rows: Vec<_> = changes
            .paths
            .iter()
            .map(|change| (change.group, change.kind, change.path.to_string()))
            .collect();
        assert_eq!(
            rows,
            [
                (ChangeGroup::Staged, ChangeKind::Added, "both".into()),
                (ChangeGroup::Staged, ChangeKind::Added, "staged".into()),
                (ChangeGroup::Unstaged, ChangeKind::Added, ".mailmap".into()),
                (ChangeGroup::Unstaged, ChangeKind::Modified, "both".into()),
                (ChangeGroup::Unstaged, ChangeKind::Unmerged, "root".into()),
                (ChangeGroup::Unstaged, ChangeKind::Added, "untracked".into()),
            ],
            "status is partitioned, path-sorted, includes conflicts and untracked files, and excludes ignored files"
        );
        assert!(changes.lines_added > 0, "available file diffs contribute line counts");
        assert!(
            changes.has_tracked_changes,
            "staged and tracked worktree changes are classified once"
        );
        for (path, diff) in changes.paths.iter().zip(&changes.diffs) {
            if path.kind != ChangeKind::Unmerged {
                prepare_file_diff_with_repository(&repository, diff, path)
                    .with_context(|| format!("{} should produce a staged or worktree diff", path.path))?;
            }
        }
        let conflict = changes
            .paths
            .iter()
            .position(|change| change.kind == ChangeKind::Unmerged)
            .expect("the conflict is visible");
        assert!(
            prepare_file_diff_with_repository(&repository, &changes.diffs[conflict], &changes.paths[conflict])
                .err()
                .expect("conflicts cannot produce a single file diff")
                .to_string()
                .contains("no single file diff"),
            "opening an unresolved path produces actionable feedback"
        );
        Ok(())
    }

    #[test]
    fn incremental_worktree_status_matches_a_full_refresh() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let path = fixture.path();
        let repository = test_repository::open(path)?;
        let mut pool = LineDiffPool::new(path, false, 2);
        let mut cached = load_worktree_changes(&repository, &mut pool)?;

        std::fs::write(path.join("main"), "changed in worktree\n")?;
        std::fs::write(path.join("literal[*]"), "untracked\n")?;
        let parts = WorktreeStatusParts {
            staged: false,
            scopes: HashSet::from([BString::from("main"), BString::from("literal[*]")]),
        };
        update_worktree_changes(&repository, &mut cached, &parts, &mut pool)?;
        assert_eq!(
            cached,
            load_worktree_changes(&repository, &mut pool)?,
            "path-limited changes equal a fresh status"
        );

        let status = Command::new("git").current_dir(path).args(["add", "main"]).status()?;
        assert!(status.success(), "git stages the tracked change");
        std::fs::write(path.join("main"), "changed in index\nand worktree\n")?;
        let parts = WorktreeStatusParts {
            staged: true,
            scopes: HashSet::from([BString::from("main")]),
        };
        update_worktree_changes(&repository, &mut cached, &parts, &mut pool)?;
        assert_eq!(
            cached,
            load_worktree_changes(&repository, &mut pool)?,
            "combined staged and worktree replacement equals a fresh status"
        );

        std::fs::create_dir_all(path.join("nested"))?;
        std::fs::write(path.join("nested/untracked"), "untracked\n")?;
        let parts = WorktreeStatusParts {
            staged: false,
            scopes: HashSet::from([BString::from("nested")]),
        };
        update_worktree_changes(&repository, &mut cached, &parts, &mut pool)?;
        assert_eq!(
            cached,
            load_worktree_changes(&repository, &mut pool)?,
            "directory scopes include their descendants"
        );

        std::fs::write(path.join("nested/.gitignore"), "untracked\n")?;
        update_worktree_changes(&repository, &mut cached, &parts, &mut pool)?;
        assert_eq!(
            cached,
            load_worktree_changes(&repository, &mut pool)?,
            "a scoped ignore change removes newly ignored cache rows"
        );

        let topic = repository.rev_parse_single("topic")?.detach();
        drop(repository);
        let status = Command::new("git")
            .current_dir(path)
            .args(["update-ref", "refs/heads/main", &topic.to_string()])
            .status()?;
        assert!(status.success(), "git moves the checked-out branch");
        let repository = test_repository::open(path)?;
        update_worktree_changes(
            &repository,
            &mut cached,
            &WorktreeStatusParts {
                staged: true,
                scopes: HashSet::new(),
            },
            &mut pool,
        )?;
        assert_eq!(
            cached,
            load_worktree_changes(&repository, &mut pool)?,
            "a staged-only replacement follows the new HEAD tree"
        );
        Ok(())
    }

    #[test]
    fn streams_diff_bytes_and_accepts_early_pager_exit() -> gix_testtools::Result {
        let diff = BuiltInDiff::new(
            "M file".into(),
            vec![BString::from("--- a/file"), BString::from(vec![b'+', 0xff])],
        );
        let mut patch = Vec::new();

        diff.write_to(&mut patch)?;

        assert_eq!(patch, b"--- a/file\n+\xff\n", "patch bytes reach the pager unchanged");
        pager_write_result(Err(io::Error::new(io::ErrorKind::BrokenPipe, "pager quit")))
            .expect("an early pager exit is normal");
        assert!(
            pager_write_result(Err(io::Error::other("write failed"))).is_err(),
            "other write failures remain visible"
        );
        #[cfg(unix)]
        assert!(
            pager_status(std::os::unix::process::ExitStatusExt::from_raw(1 << 8)).is_err(),
            "a failing pager remains visible"
        );
        assert!(
            pager_needs_acknowledgement(Duration::ZERO),
            "an immediately closing pager leaves its output visible"
        );
        assert!(
            pager_needs_acknowledgement(Duration::from_millis(250)),
            "the threshold is inclusive"
        );
        assert!(
            !pager_needs_acknowledgement(Duration::from_millis(251)),
            "longer-running pagers restore tix immediately"
        );
        Ok(())
    }

    #[test]
    fn maps_navigation_and_control_c() {
        assert_eq!(
            action(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
            Some(Action::ToggleChangesFocus)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Some(Action::OpenDiff)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE)),
            Some(Action::PageUp)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL)),
            Some(Action::PageUp)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL)),
            Some(Action::PageDown)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL)),
            Some(Action::HalfPageUp)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::NONE)),
            Some(Action::Undo)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('U'), KeyModifiers::NONE)),
            Some(Action::Redo)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::SHIFT)),
            Some(Action::Redo)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL)),
            Some(Action::HalfPageDown)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)),
            Some(Action::ScrollLeft)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE)),
            Some(Action::ScrollRight)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::SHIFT)),
            Some(Action::Last),
            "terminals that report shifted letters in lowercase still map Shift-G to the first commit"
        );
        assert_eq!(action(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE)), None);
        assert_eq!(action(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE)), None);
        assert_eq!(action(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE)), None);
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)),
            Some(Action::ToggleActions)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE)),
            Some(Action::ToggleEnrich)
        );
        assert_eq!(
            action_with_shortcut_groups(
                KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE),
                false,
                false,
                true,
                false
            ),
            Some(Action::ToggleChecksPass)
        );
        assert_eq!(
            action_with_shortcut_groups(
                KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE),
                false,
                false,
                true,
                false
            ),
            Some(Action::ToggleTodo)
        );
        assert_eq!(
            action_with_shortcut_groups(
                KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE),
                false,
                false,
                true,
                false
            ),
            Some(Action::EditNote)
        );
        assert_eq!(
            action_with_shortcut_groups(
                KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
                false,
                false,
                true,
                false
            ),
            Some(Action::EditGitNote)
        );
        assert_eq!(
            action_with_shortcut_groups(
                KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE),
                false,
                false,
                true,
                false
            ),
            Some(Action::ToggleEnrich)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE)),
            Some(Action::ToggleCommit)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE)),
            Some(Action::ToggleRefs)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE)),
            Some(Action::ToggleRefTree)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('@'), KeyModifiers::NONE)),
            Some(Action::TimeTravel),
            "the terminal's direct at-sign event invokes time travel"
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::SHIFT)),
            Some(Action::TimeTravel),
            "terminals which preserve the base character map Shift-2 to time travel"
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE)),
            None,
            "an unshifted 2 has no time-travel behavior"
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::SHIFT)),
            Some(Action::Refresh),
            "terminals which preserve lowercase shifted letters map Shift-R to refresh"
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('R'), KeyModifiers::NONE)),
            Some(Action::Refresh),
            "terminals which encode Shift-R as an uppercase letter map it to refresh"
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE)),
            Some(Action::ToggleHistoryDisplay)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::SHIFT)),
            Some(Action::ToggleInformation)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::SHIFT)),
            Some(Action::ToggleInformation),
            "terminals which preserve the base character map Shift-/ to information"
        );
        assert_eq!(
            action(KeyEvent::new(
                KeyCode::Modifier(crossterm::event::ModifierKeyCode::LeftShift),
                KeyModifiers::SHIFT,
            )),
            None,
            "standalone Shift has no application behavior"
        );
        for (key, expected) in [
            ('d', Action::ToggleDate),
            ('i', Action::CycleIds),
            ('s', Action::ToggleEmail),
            ('e', Action::ToggleName),
            ('t', Action::ToggleTrailers),
            ('m', Action::ToggleMailmap),
            ('r', Action::CycleRefs),
            ('h', Action::ToggleHidden),
        ] {
            assert_eq!(
                action_with_shortcut_groups(
                    KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE),
                    true,
                    false,
                    false,
                    false
                ),
                Some(expected),
                "{key} is available after the view prefix"
            );
        }
        for (history, actions, enrich, information) in [
            (true, false, false, false),
            (false, true, false, false),
            (false, false, true, false),
            (false, false, false, true),
        ] {
            for (key, expected) in [
                ('v', Action::ToggleHistoryDisplay),
                ('a', Action::ToggleActions),
                ('?', Action::ToggleInformation),
            ] {
                assert_eq!(
                    action_with_shortcut_groups(
                        KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE),
                        history,
                        actions,
                        enrich,
                        information,
                    ),
                    Some(expected),
                    "{key} switches prefix menus regardless of the active menu"
                );
            }
            assert_eq!(
                action_with_shortcut_groups(
                    KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE),
                    history,
                    actions,
                    enrich,
                    information,
                ),
                Some(if actions {
                    Action::NewEmptyCommit
                } else {
                    Action::ToggleEnrich
                }),
                "the actions shortcut takes priority over the enrich prefix"
            );
        }
        assert_eq!(
            action_with_shortcut_groups(
                KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE),
                true,
                false,
                false,
                false
            ),
            Some(Action::ToggleHistoryDisplay),
            "v closes the view shortcut group"
        );
        for (key, expected) in [
            ('o', Action::Reword),
            ('w', Action::NewCommit),
            ('n', Action::NewEmptyCommit),
            ('e', Action::Amend),
            ('l', Action::Spill),
            ('p', Action::Split),
            ('d', Action::Forget),
            ('i', Action::TogglePin),
        ] {
            assert_eq!(
                action_with_shortcut_groups(
                    KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE),
                    false,
                    true,
                    false,
                    false
                ),
                Some(expected),
                "{key} is available on the commit line after the actions prefix"
            );
        }
        for (key, expected) in [
            ('b', Action::Rebase),
            ('u', Action::RebaseUpdate),
            ('r', Action::Review),
            ('s', Action::Squash),
            ('y', Action::CopyInsert),
            ('m', Action::MoveInsert),
            ('t', Action::StackInsert),
            ('f', Action::ForkCommit),
            ('h', Action::Attach),
            ('z', Action::Stash),
        ] {
            assert_eq!(
                action_with_shortcut_groups(
                    KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE),
                    false,
                    true,
                    false,
                    false
                ),
                Some(expected),
                "{key} is available after the actions prefix"
            );
        }
        for (history, actions, enrich, expected) in [
            (true, false, false, Action::ToggleName),
            (false, true, false, Action::Amend),
            (false, false, true, Action::ToggleChecksPass),
        ] {
            assert_eq!(
                action_with_shortcut_groups(
                    KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE),
                    history,
                    actions,
                    enrich,
                    true,
                ),
                Some(expected),
                "an open submenu takes priority over the focused changes shortcut"
            );
        }
        assert_eq!(
            action_with_shortcut_groups(
                KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE),
                false,
                true,
                false,
                false
            ),
            Some(Action::StackInsert),
            "the actions shortcut takes priority over the direct ref-tree key"
        );
        assert_eq!(
            action_with_shortcut_groups(
                KeyEvent::new(KeyCode::Char('@'), KeyModifiers::NONE),
                false,
                true,
                false,
                false
            ),
            Some(Action::TimeTravel),
            "the direct time-travel key remains available while actions are expanded"
        );
        assert_eq!(
            action_with_shortcut_groups(
                KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
                false,
                true,
                false,
                false
            ),
            Some(Action::PageUp),
            "navigation keeps priority over the actions shortcut"
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE)),
            Some(Action::VerifySignatures)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE)),
            Some(Action::ToggleAlign)
        );
        assert_eq!(action(KeyEvent::new(KeyCode::Char('\\'), KeyModifiers::NONE)), None);
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE)),
            Some(Action::ToggleCommit)
        );
        assert_eq!(action(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE)), None);
        assert_eq!(
            action_with_shortcut_groups(
                KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
                false,
                true,
                false,
                false
            ),
            Some(Action::Review),
            "the actions shortcut takes priority over the direct reference toggle"
        );
        assert_eq!(action(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE)), None);
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('P'), KeyModifiers::NONE)),
            Some(Action::CycleChangesParent)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::SHIFT)),
            Some(Action::CycleChangesParent)
        );
        assert_eq!(action(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE)), None);
        assert_eq!(
            action_with_shortcut_groups(
                KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE),
                false,
                false,
                false,
                true
            ),
            Some(Action::ToggleChanges),
            "e changes is scoped to the information prefix"
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('Y'), KeyModifiers::SHIFT)),
            Some(Action::CopyAuthor)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(Action::ForceQuit)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
            Some(Action::CycleDuplicate)
        );
    }

    #[test]
    fn diagnostic_inputs_replay_only_read_only_actions() {
        let mut app = App::new(1);
        assert_eq!(diagnostic_action(diagnostic_key('j'), &app), Some(Action::MoveDown));
        assert_eq!(diagnostic_action(diagnostic_key('l'), &app), Some(Action::ScrollRight));
        assert_eq!(diagnostic_action(diagnostic_key('G'), &app), Some(Action::Last));
        assert_eq!(
            diagnostic_action(diagnostic_key('u'), &app),
            None,
            "undo is not replayed"
        );

        app.actions_expanded = true;
        assert_eq!(
            diagnostic_action(diagnostic_key('b'), &app),
            None,
            "repository-changing submenu actions are not replayed"
        );
    }

    #[test]
    fn shift_applies_topology_to_directions_and_viewport_movement_to_pages() {
        let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
        let shifted = |code| KeyEvent::new(code, KeyModifiers::SHIFT);
        let mut app = App::new(8);
        app.information_expanded = true;
        assert_eq!(
            app_action(key(KeyCode::Char('o')), &app),
            None,
            "the topo toggle is gone"
        );
        app.information_expanded = false;

        for (key, expected) in [
            (shifted(KeyCode::Up), Action::TopologicalUp),
            (shifted(KeyCode::Char('k')), Action::TopologicalUp),
            (key(KeyCode::Char('K')), Action::TopologicalUp),
            (shifted(KeyCode::Down), Action::TopologicalDown),
            (shifted(KeyCode::Char('j')), Action::TopologicalDown),
            (key(KeyCode::Char('J')), Action::TopologicalDown),
            (shifted(KeyCode::Left), Action::PreviousChild),
            (shifted(KeyCode::Char('h')), Action::PreviousChild),
            (key(KeyCode::Char('H')), Action::PreviousChild),
            (shifted(KeyCode::Right), Action::NextChild),
            (shifted(KeyCode::Char('l')), Action::NextChild),
            (key(KeyCode::Char('L')), Action::NextChild),
        ] {
            assert_eq!(app_action(key, &app), Some(expected));
        }

        assert_eq!(app_action(key(KeyCode::Up), &app), Some(Action::MoveUp));
        app.history_display_expanded = true;
        assert_eq!(app_action(key(KeyCode::Char('h')), &app), Some(Action::ToggleHidden));
        assert_eq!(
            app_action(shifted(KeyCode::Char('h')), &app),
            Some(Action::PreviousChild),
            "shifted navigation outranks an open shortcut group"
        );
        app.history_display_expanded = false;

        let control = |character| KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL);
        let control_shift =
            |character| KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL | KeyModifiers::SHIFT);
        for (key, expected) in [
            (key(KeyCode::PageUp), Action::PageUp),
            (key(KeyCode::PageDown), Action::PageDown),
            (control('u'), Action::HalfPageUp),
            (control('d'), Action::HalfPageDown),
            (control('b'), Action::PageUp),
            (control('f'), Action::PageDown),
            (shifted(KeyCode::PageUp), Action::PanUpBy(8)),
            (shifted(KeyCode::PageDown), Action::PanDownBy(8)),
            (control_shift('u'), Action::PanUpBy(4)),
            (control_shift('d'), Action::PanDownBy(4)),
            (control_shift('b'), Action::PanUpBy(8)),
            (control_shift('f'), Action::PanDownBy(8)),
        ] {
            assert_eq!(app_action(key, &app), Some(expected));
        }
        assert_eq!(
            app_action(KeyEvent::new(KeyCode::Char('U'), KeyModifiers::CONTROL), &app),
            Some(Action::PanUpBy(4)),
            "uppercase Ctrl paging retains its Shift meaning instead of invoking redo"
        );

        app.changes_focus = Some(ChangePane::Tree);
        app.history_display_expanded = true;
        assert_eq!(
            app_action(key(KeyCode::Char('H')), &app),
            Some(Action::ScrollLeft),
            "shifted directions remain pane-local while changes are focused"
        );
        assert_eq!(
            app_action(key(KeyCode::PageUp), &app),
            Some(Action::PageUp),
            "focused panes retain their own paging"
        );

        app.changes_focus = None;
        app.show_commit = true;
        app.set_commit_bounds(2, 1);
        assert_eq!(
            app_action(key(KeyCode::PageUp), &app),
            Some(Action::PageUp),
            "overflowing commit messages retain their own paging"
        );
    }

    #[test]
    fn diagnostic_inputs_wait_for_completed_lanes() {
        let key = diagnostic_key('j');
        let mut inputs = VecDeque::from([key]);
        assert!(next_diagnostic_input(&mut inputs, State::Loading, false).is_none());
        assert!(next_diagnostic_input(&mut inputs, State::Complete, true).is_none());
        assert_eq!(inputs.len(), 1, "premature checks do not consume input");
        assert_eq!(next_diagnostic_input(&mut inputs, State::Complete, false), Some(key));
    }

    #[test]
    fn tree_is_direct_while_trailers_remain_scoped_to_the_view_prefix() {
        let key = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE);
        assert_eq!(
            action_with_shortcut_groups(key, true, false, false, false),
            Some(Action::ToggleTrailers),
            "v t retains its trailer action"
        );
        assert_eq!(
            action_with_shortcut_groups(key, false, false, false, false),
            Some(Action::ToggleRefTree),
            "plain t toggles the ref-tree"
        );
    }

    #[test]
    fn retains_the_fill_repository_only_for_repeated_viewport_navigation() {
        for action in [Action::MoveDown, Action::PageDown] {
            assert!(retains_fill_repository(KeyEventKind::Repeat, Some(&action), false));
        }
        assert!(!retains_fill_repository(
            KeyEventKind::Repeat,
            Some(&Action::MoveDown),
            true
        ));
        assert!(!retains_fill_repository(
            KeyEventKind::Press,
            Some(&Action::MoveDown),
            false
        ));
        assert!(!retains_fill_repository(
            KeyEventKind::Release,
            Some(&Action::MoveDown),
            false
        ));
        assert!(!retains_fill_repository(
            KeyEventKind::Repeat,
            Some(&Action::ScrollRight),
            false
        ));
        assert!(!retains_fill_repository(
            KeyEventKind::Repeat,
            Some(&Action::ToggleDate),
            false
        ));
    }

    #[test]
    fn enhanced_keyboard_reports_repeats_for_printable_navigation_keys() {
        let flags = keyboard_enhancement_flags();
        assert!(
            flags.contains(KeyboardEnhancementFlags::REPORT_EVENT_TYPES),
            "enhanced input distinguishes presses, repeats, and releases"
        );
        assert!(
            flags.contains(KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES),
            "printable j/k keys must use enhanced input for repeat events"
        );
    }

    #[test]
    fn key_releases_do_not_cancel_suspended_operations() {
        let key = |kind| TerminalEvent::Key(KeyEvent::new_with_kind(KeyCode::Char('t'), KeyModifiers::NONE, kind));
        assert!(is_key_press(&key(KeyEventKind::Press)));
        assert!(is_key_press(&key(KeyEventKind::Repeat)));
        assert!(!is_key_press(&key(KeyEventKind::Release)));
    }

    #[test]
    fn materialized_rebases_allow_inspection_but_block_repository_changes() {
        for action in [
            Action::MoveDown,
            Action::CycleDuplicate,
            Action::ToggleChangesFocus,
            Action::ToggleCommit,
            Action::Copy,
            Action::ForceQuit,
        ] {
            assert!(action_allowed_during_rebase_continuation(Some(&action), false));
        }
        for action in [
            Action::Undo,
            Action::Redo,
            Action::Refresh,
            Action::ToggleHidden,
            Action::ToggleRefTree,
            Action::ToggleActions,
            Action::Amend,
            Action::Spill,
            Action::Rebase,
            Action::TimeTravel,
            Action::VerifySignatures,
        ] {
            assert!(
                !action_allowed_during_rebase_continuation(Some(&action), false),
                "{action:?} cannot invalidate a materialized rebase"
            );
        }
        assert!(
            action_allowed_during_rebase_continuation(Some(&Action::Quit), false),
            "history q always remains available"
        );
        for action in [Action::OpenDiff, Action::Cancel, Action::Quit] {
            assert!(
                action_allowed_during_rebase_continuation(Some(&action), true),
                "{action:?} retains its changes-pane behavior"
            );
        }
        assert!(
            !action_allowed_during_rebase_continuation(Some(&Action::OpenDiff), false),
            "history Enter is reserved for continuation"
        );
        assert!(
            !action_allowed_during_rebase_continuation(Some(&Action::Cancel), false),
            "history Escape is reserved for stopping"
        );
    }

    #[test]
    fn shift_switches_mouse_scrolling_from_viewport_to_cursor_navigation() {
        assert_eq!(
            mouse_scroll_action(MouseEventKind::ScrollUp, KeyModifiers::NONE, 4, false),
            Some(Action::PanUpBy(4))
        );
        assert_eq!(
            mouse_scroll_action(MouseEventKind::ScrollDown, KeyModifiers::SHIFT, 3, false),
            Some(Action::MoveDownBy(3))
        );
        assert_eq!(
            mouse_scroll_action(MouseEventKind::ScrollLeft, KeyModifiers::NONE, 1, false),
            Some(Action::ScrollLeft)
        );
        assert_eq!(
            mouse_scroll_action(MouseEventKind::ScrollRight, KeyModifiers::SHIFT, 1, false),
            Some(Action::NextChild)
        );
        assert_eq!(
            mouse_scroll_action(MouseEventKind::ScrollUp, KeyModifiers::NONE, 2, true),
            Some(Action::MoveUpBy(2)),
            "focused changes retain cursor navigation"
        );
        assert_eq!(
            mouse_scroll_action(MouseEventKind::Moved, KeyModifiers::NONE, 1, false),
            None
        );
        assert!(repeats_viewport(
            &mouse_scroll_action(MouseEventKind::ScrollDown, KeyModifiers::NONE, 2, false)
                .expect("vertical scrolling has an action")
        ));
        assert!(!repeats_viewport(
            &mouse_scroll_action(MouseEventKind::ScrollRight, KeyModifiers::NONE, 1, false)
                .expect("horizontal scrolling has an action")
        ));
    }

    #[test]
    fn copies_parsed_author_bytes_without_validation() {
        let author = app::Author {
            name: b"Author > Name".as_bstr(),
            email: b"author<@example.com".as_bstr(),
        };

        assert_eq!(
            actor_bytes(&author),
            b"Author > Name <author<@example.com>",
            "parsed author bytes are copied even if they aren't valid serialization tokens"
        );
    }

    #[test]
    fn rendering_is_reactive_and_capped_while_streaming() {
        assert!(
            !history_is_ready_to_draw(State::Loading, 0),
            "the initial empty frame remains outside terminal scrollback"
        );
        assert!(
            history_is_ready_to_draw(State::Loading, 1),
            "the first commit makes loading history renderable"
        );
        assert!(
            history_is_ready_to_draw(State::Computing, 0),
            "an empty completed traversal remains renderable"
        );
        assert!(
            !should_draw(false, false, Duration::MAX),
            "clean frames are never redrawn"
        );
        assert!(
            should_draw(true, false, Duration::ZERO),
            "idle changes redraw immediately"
        );
        assert!(
            !should_draw(true, true, FRAME_INTERVAL.saturating_sub(Duration::from_nanos(1))),
            "streaming frames wait for the 60 fps deadline"
        );
        assert!(
            should_draw(true, true, FRAME_INTERVAL),
            "streaming frames draw at the deadline"
        );
        assert_eq!(
            poll_timeout(false, 0, false, Duration::ZERO, None),
            None,
            "idle waits reactively for terminal input"
        );
        assert_eq!(
            poll_timeout(true, EVENT_BATCH_SIZE, true, Duration::ZERO, None),
            Some(Duration::ZERO),
            "saturated history batches keep processing"
        );
        assert_eq!(
            poll_timeout(true, 1, true, Duration::from_millis(10), None),
            Some(FRAME_INTERVAL.saturating_sub(Duration::from_millis(10))),
            "dirty streaming frames wait only until their deadline"
        );
        assert_eq!(
            poll_timeout(false, 0, false, Duration::ZERO, Some(REPEAT_IDLE)),
            Some(REPEAT_IDLE),
            "repeat-idle restoration wakes an otherwise idle event loop"
        );
        assert_eq!(
            poll_timeout(true, 1, true, Duration::from_millis(10), Some(REPEAT_IDLE)),
            Some(FRAME_INTERVAL.saturating_sub(Duration::from_millis(10))),
            "the earlier frame deadline takes precedence over repeat-idle restoration"
        );
    }

    #[test]
    fn filters_worktree_watch_events_and_invalidates_cached_status() {
        use notify::event::{AccessKind, CreateKind, Flag, ModifyKind, RemoveKind, RenameMode};

        let workdir = Path::new("/repo");
        let dot_git = workdir.join(".git");
        let git_dir = dot_git.clone();
        let index = git_dir.join("index");
        let modified =
            |path: &Path| notify::Event::new(notify::EventKind::Modify(ModifyKind::Any)).add_path(path.to_owned());
        assert!(worktree_event_is_relevant(
            &modified(&workdir.join("src/lib.rs")),
            workdir,
            &dot_git,
            &git_dir,
            &index
        ));
        assert!(worktree_event_is_relevant(
            &modified(&index),
            workdir,
            &dot_git,
            &git_dir,
            &index
        ));
        assert!(!worktree_event_is_relevant(
            &modified(&git_dir.join("HEAD")),
            workdir,
            &dot_git,
            &git_dir,
            &index
        ));
        let access =
            notify::Event::new(notify::EventKind::Access(AccessKind::Any)).add_path(workdir.join("src/lib.rs"));
        assert!(!worktree_event_is_relevant(
            &access, workdir, &dot_git, &git_dir, &index
        ));
        assert!(!notification_is_actionable(&access));
        let lock_only = modified(&git_dir.join("index.lock"));
        assert!(!notification_is_actionable(&lock_only));
        let completed_lock_rename = notify::Event::new(notify::EventKind::Modify(ModifyKind::Name(RenameMode::Any)))
            .add_path(git_dir.join("index.lock"));
        assert!(notification_is_actionable(&completed_lock_rename));
        let completed_lock_update = lock_only.add_path(index.clone());
        assert!(notification_is_actionable(&completed_lock_update));
        let rescan = notify::Event::new(notify::EventKind::Other).set_flag(Flag::Rescan);
        assert!(worktree_event_is_relevant(&rescan, workdir, &dot_git, &git_dir, &index));
        assert!(notification_is_actionable(&rescan));
        let empty = notify::Event::new(notify::EventKind::Modify(ModifyKind::Any));
        assert!(
            worktree_event_is_relevant(&empty, workdir, &dot_git, &git_dir, &index),
            "an event without paths conservatively refreshes all status"
        );

        let worktrees = git_dir.join("worktrees");
        let linked = worktrees.join("linked");
        assert!(reference_event_is_relevant(
            &modified(&linked.join("HEAD")),
            &git_dir,
            &worktrees
        ));
        assert!(reference_event_is_relevant(
            &modified(&linked.join("gitdir")),
            &git_dir,
            &worktrees
        ));
        assert!(!reference_event_is_relevant(
            &modified(&linked.join("index")),
            &git_dir,
            &worktrees
        ));
        assert!(!reference_event_is_relevant(
            &modified(&linked.join("logs/HEAD")),
            &git_dir,
            &worktrees
        ));
        let current_linked = worktrees.join("current");
        assert!(!reference_event_is_relevant(
            &modified(&current_linked.join("index")),
            &current_linked,
            &worktrees
        ));
        assert!(!reference_event_is_relevant(
            &modified(&git_dir.join("index")),
            &current_linked,
            &worktrees
        ));
        assert!(!reference_event_is_relevant(
            &modified(&git_dir.join("index")),
            &git_dir,
            &worktrees
        ));
        assert!(reference_event_is_relevant(
            &modified(&git_dir.join("refs/heads/other")),
            &git_dir,
            &worktrees
        ));
        assert!(reference_event_changes_status_configuration(
            &modified(&git_dir.join("config")),
            &git_dir,
            &worktrees
        ));
        assert!(reference_event_changes_status_configuration(
            &modified(&current_linked.join("config.worktree")),
            &current_linked,
            &worktrees
        ));
        assert!(
            !reference_event_changes_status_configuration(
                &modified(&git_dir.join("refs/heads/other")),
                &git_dir,
                &worktrees
            ),
            "unrelated refs don't invalidate worktree status through configuration"
        );
        assert!(reference_event_is_relevant(
            &modified(&current_linked.join("refs/worktree/tix/pins/abcd")),
            &current_linked,
            &worktrees
        ));
        assert!(!reference_event_is_relevant(
            &modified(&linked.join("refs/worktree/tix/pins/abcd")),
            &current_linked,
            &worktrees
        ));
        assert!(reference_watch_set_may_change(
            &modified(&worktrees.join("new-linked")),
            &worktrees
        ));
        assert!(!reference_watch_set_may_change(
            &modified(&linked.join("HEAD")),
            &worktrees
        ));

        let directories = HashSet::from([workdir.join("src")]);
        let mut watch_refresh = WorktreeWatchRefresh::default();
        watch_refresh.observe(&modified(&workdir.join("src/lib.rs")), workdir, &index, &directories);
        assert!(watch_refresh.is_empty(), "ordinary file changes don't touch watches");
        let file_rename = notify::Event::new(notify::EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
            .add_path(workdir.join("src/old"))
            .add_path(workdir.join("src/new"));
        watch_refresh.observe(&file_rename, workdir, &index, &directories);
        assert!(watch_refresh.is_empty(), "ordinary file renames don't touch watches");
        watch_refresh.observe(&modified(&index), workdir, &index, &directories);
        assert!(watch_refresh.index, "index changes request a projection comparison");

        let mut watch_refresh = WorktreeWatchRefresh::default();
        watch_refresh.observe(
            &modified(&workdir.join("src/.gitignore")),
            workdir,
            &index,
            &directories,
        );
        assert_eq!(watch_refresh.scopes, directories, "nested ignores rescan their parent");
        watch_refresh.observe(&modified(&workdir.join(".gitignore")), workdir, &index, &directories);
        assert!(watch_refresh.full, "a root ignore change affects the whole worktree");

        let create_directory =
            notify::Event::new(notify::EventKind::Create(CreateKind::Folder)).add_path(workdir.join("new"));
        let remove_directory =
            notify::Event::new(notify::EventKind::Remove(RemoveKind::Folder)).add_path(workdir.join("src"));
        let mut watch_refresh = WorktreeWatchRefresh::default();
        watch_refresh.observe(&create_directory, workdir, &index, &directories);
        watch_refresh.observe(&remove_directory, workdir, &index, &directories);
        assert_eq!(
            watch_refresh.scopes,
            HashSet::from([workdir.join("new"), workdir.join("src")]),
            "directory topology is reconciled by scope"
        );
        watch_refresh.observe(&rescan, workdir, &index, &directories);
        assert!(watch_refresh.full, "rescans compare the complete desired watch set");

        assert_eq!(
            worktree_status_event_scopes(
                &modified(&workdir.join("src/lib.rs")),
                workdir,
                &dot_git,
                &git_dir,
                &index
            ),
            Some(vec!["src/lib.rs".into()]),
            "file events become literal repository-relative scopes"
        );
        assert_eq!(
            worktree_status_event_scopes(
                &modified(&workdir.join("src/.gitignore")),
                workdir,
                &dot_git,
                &git_dir,
                &index
            ),
            Some(vec!["src".into()]),
            "ignore changes refresh their subtree"
        );
        assert!(
            worktree_status_event_scopes(
                &modified(&workdir.join("src/.gitattributes")),
                workdir,
                &dot_git,
                &git_dir,
                &index
            )
            .is_none(),
            "attribute changes require full status, including staged line counts"
        );
        assert!(
            worktree_status_event_scopes(
                &modified(&workdir.join(".gitmodules")),
                workdir,
                &dot_git,
                &git_dir,
                &index
            )
            .is_none(),
            "submodule configuration changes require full status"
        );
        assert!(
            worktree_status_event_scopes(&modified(&index), workdir, &dot_git, &git_dir, &index).is_none(),
            "index events require full status"
        );
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            let raw = OsString::from_vec(vec![b'n', 0xff]);
            assert_eq!(
                worktree_status_event_scopes(&modified(&workdir.join(raw)), workdir, &dot_git, &git_dir, &index),
                Some(vec![BString::from(vec![b'n', 0xff])]),
                "event paths remain byte-preserving"
            );
        }

        let mut changes = Some((WORKTREE_STATUS_CURRENT, Changes::default()));
        let mut parts = WorktreeStatusParts::default();
        assert!(invalidate_worktree_status_parts(
            &mut changes,
            &mut parts,
            false,
            [BString::from("src/lib.rs")]
        ));
        assert_eq!(
            changes.as_ref().map(|(marker, _)| *marker),
            Some(WORKTREE_STATUS_PARTIAL)
        );
        assert!(invalidate_worktree_changes(&mut changes));
        assert_eq!(changes.as_ref().map(|(marker, _)| *marker), Some(WORKTREE_STATUS_FULL));
        assert!(!invalidate_worktree_changes(&mut changes));
    }

    #[test]
    fn worktree_watch_directories_follow_git_ignores() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let root = fixture.path();
        std::fs::create_dir_all(root.join("visible/nested"))?;
        std::fs::create_dir_all(root.join("visible/ignored/nested"))?;
        std::fs::create_dir_all(root.join("target/nested"))?;
        std::fs::write(root.join(".gitignore"), "target/\nvisible/ignored/\n")?;

        let repository = test_repository::open(root)?;
        let directories = worktree_watch_directories(&repository)?;
        let root = repository.workdir().expect("the fixture has a worktree");
        assert!(directories.contains(root), "the worktree root is always watched");
        assert!(
            directories.contains(&root.join("visible")),
            "visible directories are watched"
        );
        assert!(
            directories.contains(&root.join("visible/nested")),
            "visible descendants are watched"
        );
        assert!(
            !directories.contains(&root.join("target")),
            "ignored directories aren't watched"
        );
        assert!(
            !directories.contains(&root.join("target/nested")),
            "ignored descendants aren't traversed"
        );
        assert!(
            !directories.contains(&root.join("visible/ignored")),
            "nested ignore rules are honored"
        );
        Ok(())
    }

    #[test]
    fn worktree_watches_apply_only_directory_set_differences() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let root = fixture.path();
        let mut watcher = start_worktree_watcher(root, false)?;
        let initial_projection = watcher.index_projection.clone();
        std::fs::create_dir_all(root.join("new/nested"))?;
        std::fs::create_dir_all(root.join("staged"))?;
        std::fs::write(root.join("staged/tracked"), "new\n")?;
        let status = Command::new("git")
            .current_dir(root)
            .args(["add", "staged/tracked"])
            .status()?;
        assert!(status.success(), "git adds a path outside the refresh scope");

        let refresh = WorktreeWatchRefresh {
            scopes: HashSet::from([root.join("new")]),
            ..WorktreeWatchRefresh::default()
        };
        assert_eq!(
            reconcile_worktree_watcher(&mut watcher, root, false, refresh)?,
            (0, 2),
            "the new subtree adds exactly its two directories"
        );
        assert!(watcher.directories.contains(&root.join("new/nested")));
        assert_eq!(
            watcher.index_projection, initial_projection,
            "a scoped worktree refresh doesn't consume a pending index change"
        );

        assert_eq!(
            reconcile_worktree_watcher(
                &mut watcher,
                root,
                false,
                WorktreeWatchRefresh {
                    index: true,
                    ..WorktreeWatchRefresh::default()
                }
            )?,
            (0, 1),
            "the later index refresh still adds its directory watch"
        );
        assert!(watcher.directories.contains(&root.join("staged")));

        let refresh = WorktreeWatchRefresh {
            scopes: HashSet::from([root.join("new")]),
            ..WorktreeWatchRefresh::default()
        };
        assert_eq!(
            reconcile_worktree_watcher(&mut watcher, root, false, refresh)?,
            (0, 0),
            "an unchanged desired set never mutates the watcher"
        );

        std::fs::write(root.join(".gitignore"), "new/\n")?;
        assert_eq!(
            reconcile_worktree_watcher(
                &mut watcher,
                root,
                false,
                WorktreeWatchRefresh {
                    full: true,
                    ..WorktreeWatchRefresh::default()
                }
            )?,
            (2, 0),
            "new ignore rules remove only the newly ignored subtree"
        );
        assert!(!watcher.directories.contains(&root.join("new")));
        assert!(
            watcher
                .directories
                .contains(watcher.index.parent().expect("index path has a parent")),
            "the index directory always remains watched"
        );
        Ok(())
    }

    #[test]
    fn index_watch_projection_ignores_content_but_tracks_topology() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let root = fixture.path();
        let repository = test_repository::open(root)?;
        let index = repository.index_or_empty()?;
        let before = index_watch_projection(&index);
        drop(index);
        drop(repository);

        std::fs::write(root.join("main"), "new contents\n")?;
        let status = Command::new("git").current_dir(root).args(["add", "main"]).status()?;
        assert!(status.success(), "git stages new contents for an existing path");
        let repository = test_repository::open(root)?;
        let index = repository.index_or_empty()?;
        let after_content = index_watch_projection(&index);
        drop(index);
        drop(repository);
        assert_eq!(
            before, after_content,
            "object and stat changes do not affect directory watches"
        );

        std::fs::create_dir_all(root.join("new"))?;
        std::fs::write(root.join("new/tracked"), "new\n")?;
        let status = Command::new("git")
            .current_dir(root)
            .args(["add", "new/tracked"])
            .status()?;
        assert!(status.success(), "git adds a path in a new directory");
        let repository = test_repository::open(root)?;
        let index = repository.index_or_empty()?;
        let after_path = index_watch_projection(&index);
        assert_eq!(
            changed_index_watch_scopes(&after_content, &after_path, root),
            HashSet::from([root.join("new")]),
            "index topology changes identify the affected top-level directory"
        );
        Ok(())
    }

    #[test]
    fn starts_worktree_watching_for_the_combined_view() {
        assert!(worktree_watcher_needed(false, Some(ChangesMode::Both)));
        assert!(!worktree_watcher_needed(false, Some(ChangesMode::Tree)));
        assert!(!worktree_watcher_needed(false, None));
        assert!(!worktree_watcher_needed(true, Some(ChangesMode::Both)));
    }

    #[test]
    fn restores_changed_path_selection_after_reordering() {
        let path = |path: &str| PathChange {
            kind: ChangeKind::Modified,
            group: ChangeGroup::Unstaged,
            source: None,
            path: path.into(),
            lines: None,
        };
        let previous = Changes {
            paths: ["a", "b", "selected"].into_iter().map(path).collect(),
            ..Changes::default()
        };
        let mut view = app::ChangesView::default();
        view.selected = 2;
        view.offset = 1;
        let remembered = remembered_change_selection(&view, Some(&previous));
        let refreshed = Changes {
            paths: ["x", "y", "z", "selected"].into_iter().map(path).collect(),
            ..Changes::default()
        };

        restore_change_selection(&mut view, &refreshed, remembered);

        assert_eq!(view.selected, 3, "the same path remains selected");
        assert_eq!(view.offset, 2, "the path retains its relative viewport row");
    }

    #[test]
    fn event_deadlines_coalesce_without_extending_and_can_be_retried() {
        let now = Instant::now();
        let mut deadline = None;
        assert!(schedule_once(&mut deadline, now, Duration::ZERO));
        let first = deadline;
        assert!(!schedule_once(
            &mut deadline,
            now + Duration::from_millis(50),
            Duration::ZERO
        ));
        assert_eq!(deadline, first, "queued worktree events share an immediate deadline");
        assert!(take_due(&mut deadline, now));
        assert_eq!(deadline, None);

        assert!(schedule_once(&mut deadline, now, WATCH_RETRY_INTERVAL));
        assert!(!take_due(&mut deadline, now + Duration::from_secs(4)));
        assert!(take_due(&mut deadline, now + WATCH_RETRY_INTERVAL));

        assert!(
            schedule_once(&mut deadline, now, HISTORY_STATUS_DELAY),
            "background progress gets its own deadline"
        );
        assert!(
            !take_due(&mut deadline, now + Duration::from_millis(499)),
            "the completed footer remains visible before 500 ms"
        );
        assert!(
            take_due(&mut deadline, now + HISTORY_STATUS_DELAY),
            "background progress becomes visible at 500 ms"
        );

        let last_event = now + Duration::from_millis(75);
        deadline = Some(last_event + REF_EVENT_IDLE);
        assert!(
            !take_due(&mut deadline, now + REF_EVENT_IDLE),
            "reference inspection waits for the final transaction event"
        );
        assert!(take_due(&mut deadline, last_event + REF_EVENT_IDLE));
    }

    #[test]
    fn todo_progress_appears_at_three_hundred_milliseconds() {
        assert!(!todo_progress_visible(Duration::from_millis(299)));
        assert!(todo_progress_visible(TODO_PROGRESS_DELAY));
    }

    #[test]
    fn fast_time_travel_draws_the_first_and_latest_rebased_commits() -> Result<()> {
        let ids = [
            gix::ObjectId::Sha1([1; 20]),
            gix::ObjectId::Sha1([2; 20]),
            gix::ObjectId::Sha1([3; 20]),
        ];
        let mut rendered = Vec::new();

        run_with_rebase_selection(
            |report| {
                for id in ids {
                    report(id);
                }
                Ok(())
            },
            |id| {
                rendered.push(id);
                Ok(())
            },
        )?;

        assert_eq!(rendered.first(), Some(&ids[0]), "the first completed rebase is drawn");
        assert_eq!(rendered.last(), Some(&ids[2]), "the latest completed rebase is drawn");
        Ok(())
    }

    #[test]
    fn slow_time_travel_draws_every_rebased_selection() -> Result<()> {
        let ids = [gix::ObjectId::Sha1([1; 20]), gix::ObjectId::Sha1([2; 20])];
        let mut rendered = Vec::new();

        run_with_rebase_selection(
            |report| {
                for id in ids {
                    report(id);
                    std::thread::sleep(FRAME_INTERVAL * 2);
                }
                Ok(())
            },
            |id| {
                rendered.push(id);
                Ok(())
            },
        )?;

        assert_eq!(rendered, ids, "slow travel renders each completed rebase");
        Ok(())
    }

    #[test]
    fn failed_animation_frame_preserves_completed_time_travel() -> Result<()> {
        let completed = run_with_rebase_selection(
            |report| {
                report(gix::ObjectId::Sha1([1; 20]));
                Ok(42)
            },
            |_| Err(anyhow::anyhow!("frame failed")),
        )?;

        assert_eq!(completed, 42, "animation failure cannot hide a completed mutation");
        Ok(())
    }

    #[test]
    fn continuing_a_conflict_commits_the_complete_resolved_index() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_conflict.sh")?;
        let git = |args: &[&str]| Command::new("git").arg("-C").arg(fixture.path()).args(args).output();
        #[cfg(unix)]
        let conflict_path = ":(glob)*";
        #[cfg(not(unix))]
        let conflict_path = "file";
        let deleted_path = "deleted-conflict";
        std::fs::write(fixture.path().join(conflict_path), b"conflict base\n")?;
        std::fs::write(fixture.path().join(deleted_path), b"delete base\n")?;
        assert!(
            git(&["--literal-pathspecs", "add", "--", conflict_path, deleted_path,])?
                .status
                .success()
        );
        assert!(git(&["commit", "-qm", "conflict base"])?.status.success());
        assert!(git(&["checkout", "-q", "-b", "conflict-side"])?.status.success());
        std::fs::write(fixture.path().join(conflict_path), b"side\n")?;
        std::fs::write(fixture.path().join(deleted_path), b"side\n")?;
        assert!(git(&["commit", "-qam", "side"])?.status.success());
        assert!(git(&["checkout", "-q", "main"])?.status.success());
        std::fs::write(fixture.path().join(conflict_path), b"main\n")?;
        std::fs::write(fixture.path().join(deleted_path), b"main\n")?;
        assert!(git(&["commit", "-qam", "main"])?.status.success());
        assert!(
            !git(&["merge", "--no-edit", "conflict-side"])?.status.success(),
            "the fixture produces an ordinary unmerged index"
        );
        std::fs::write(fixture.path().join(conflict_path), b"resolved\n")?;
        std::fs::remove_file(fixture.path().join(deleted_path))?;
        std::fs::write(fixture.path().join("already-staged"), b"staged\n")?;
        assert!(git(&["add", "already-staged"])?.status.success());
        std::fs::write(fixture.path().join("unrelated"), b"unstaged\n")?;

        let repository = test_repository::open(fixture.path())?;
        stage_resolved_conflict_paths(&repository)?;

        let index = repository.index_or_empty()?;
        assert!(
            index
                .entries()
                .iter()
                .all(|entry| entry.stage() == gix::index::entry::Stage::Unconflicted),
            "resolved paths no longer retain conflict stages"
        );
        let staged = git(&["diff", "--cached", "--name-only"])?.stdout;
        assert_eq!(
            staged.as_bstr().lines().collect::<HashSet<_>>(),
            HashSet::from([
                b"already-staged".as_slice(),
                conflict_path.as_bytes(),
                deleted_path.as_bytes(),
            ]),
            "edited and deleted resolutions join changes that were already staged"
        );

        let head = repository.head_id()?.detach();
        let parent = repository
            .find_commit(head)?
            .parent_ids()
            .next()
            .map(gix::Id::detach)
            .context("the conflicted commit has a parent")?;
        let plan = edit::rebase::Plan {
            base: parent,
            scope: vec![head],
            steps: vec![edit::rebase::PlanStep {
                parent: edit::rebase::PlanParent::Existing(parent),
                commit: edit::rebase::PlanCommit::Resolved(head),
                squash: Vec::new(),
            }],
            checkout: Some(edit::rebase::PlanCheckout {
                target: edit::rebase::PlanParent::Step(0),
                reference: None,
            }),
            expected_refs: Vec::new(),
        };
        let graph = HistoryGraph::for_commits(&repository, &plan.scope)?;
        let outcome = edit::rebase::perform_plan(&repository, &graph, plan)?.complete()?;
        let resolved = outcome.map(head).context("the conflicted commit is retained")?;
        edit::time_travel::checkout_plan(fixture.path(), false, &outcome, &[], false)?;

        let repository = test_repository::open(fixture.path())?;
        assert_eq!(
            repository.head_id()?.detach(),
            resolved,
            "HEAD selects the resolved commit"
        );
        for (path, expected) in [
            (conflict_path, b"resolved\n".as_slice()),
            ("already-staged", b"staged\n"),
        ] {
            let object = format!("{resolved}:{path}");
            assert_eq!(
                git(&["show", &object])?.stdout,
                expected,
                "{path} is absorbed into HEAD"
            );
        }
        let deleted = format!("{resolved}:{deleted_path}");
        assert!(
            !git(&["cat-file", "-e", &deleted])?.status.success(),
            "a deleted conflict is absent from HEAD"
        );
        assert!(
            git(&["diff", "--cached", "--quiet"])?.status.success(),
            "the resolved commit leaves no staged changes"
        );
        assert!(
            repository
                .index_or_empty()?
                .entries()
                .iter()
                .all(|entry| entry.stage() == gix::index::entry::Stage::Unconflicted),
            "the final index has no conflict stages"
        );
        assert!(
            history::all_pins(&repository)?.iter().all(history::Pin::is_head),
            "continuing a resolved plan creates no ordinary pin for its superseded conflict checkout"
        );
        assert!(
            !git(&["ls-files", "--error-unmatch", "unrelated"])?.status.success(),
            "unrelated unstaged paths remain untouched"
        );
        Ok(())
    }

    #[test]
    fn rebase_todo_loads_metadata_for_the_entire_editable_scope() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_conflict.sh")?;
        let repository = test_repository::open(fixture.path())?;
        let scope = ["HEAD", "HEAD~1", "HEAD~2"]
            .into_iter()
            .map(|revision| Ok(repository.rev_parse_single(revision)?.detach()))
            .collect::<gix_testtools::Result<Vec<_>>>()?;
        let unloaded_author = Box::leak(Box::new(app::Author {
            name: b"".as_bstr(),
            email: b"".as_bstr(),
        }));
        let rows = scope
            .iter()
            .map(|id| {
                let commit = repository.find_commit(*id)?;
                Ok(app::Commit {
                    id: *id,
                    parent_ids: commit.parent_ids().map(gix::Id::detach).collect(),
                    committer_time: gix::date::Time::default(),
                    author_time: gix::date::Time::default(),
                    author: unloaded_author,
                    attributions: 0..0,
                    title: BString::default(),
                    metadata_loaded: false,
                    has_agent_marker: false,
                    is_review: false,
                    signature: app::SignatureState::Unsigned,
                })
            })
            .collect::<gix_testtools::Result<Vec<_>>>()?;
        let mut app = App::new(1);
        app.extend_commits(rows);
        let authors =
            gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(Authors::default()));

        let commits = load_rebase_todo_commits(&repository, &mut app, &authors, &scope)?;

        assert!(app.rows.iter().all(|row| row.metadata_loaded));
        for (commit, title) in commits.iter().zip(["tip", "middle", "base"]) {
            assert!(
                commit.info.contains("author"),
                "author metadata is loaded: {}",
                commit.info
            );
            assert!(commit.info.contains(title), "the real title is loaded: {}", commit.info);
            assert!(
                !commit.info.contains("1970-01-01"),
                "missing dates never leak into the editor: {}",
                commit.info
            );
        }
        Ok(())
    }

    #[test]
    fn todo_conflict_preview_selects_the_partial_result_in_memory() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_conflict.sh")?;
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=preview author", "user.email=preview@example.com"],
        )?;
        let graph = edit::loaded_graph(&repo)?;
        let base = repo.rev_parse_single("HEAD~2")?.detach();
        let middle = repo.rev_parse_single("HEAD~1")?.detach();
        let tip = repo.head_id()?.detach();
        let edit::rebase::PlanPerform::Conflict(conflict) = edit::rebase::perform_plan(
            &repo,
            &graph,
            edit::rebase::Plan {
                base,
                scope: vec![middle, tip],
                steps: vec![edit::rebase::PlanStep {
                    parent: edit::rebase::PlanParent::Existing(base),
                    commit: edit::rebase::PlanCommit::Pick(tip),
                    squash: Vec::new(),
                }],
                checkout: Some(edit::rebase::PlanCheckout {
                    target: edit::rebase::PlanParent::Step(0),
                    reference: None,
                }),
                expected_refs: edit::rebase::capture_refs(&repo, &[middle, tip], &[tip])?,
            },
        )?
        else {
            return Err("the reordered history should conflict".into());
        };
        let authors =
            gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(Authors::default()));
        let mut app = App::new(usize::MAX);
        preview_todo_rebase_conflict(&mut app, &conflict, &authors, &[tip], &[])?;
        app.arm_rebase_conflict(conflict.commit());
        app.select_commit(conflict.commit());

        assert_eq!(
            app.rows
                .get(app.selected.expect("the conflict is selected"))
                .map(|row| row.id),
            Some(conflict.commit()),
            "the displayed conflict row is the prepared result, not its original source"
        );
        Ok(())
    }
}
