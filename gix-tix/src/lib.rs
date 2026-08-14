//! A fast, interactive commit graph for terminals.

#![forbid(unsafe_code)]

mod animation;
mod app;
mod edit;
mod history;
mod logging;
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
use crossterm::{
    clipboard::CopyToClipboard,
    cursor,
    event::{
        self, DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture, Event as TerminalEvent,
        KeyCode, KeyEvent, KeyEventKind, KeyModifiers, KeyboardEnhancementFlags, ModifierKeyCode, MouseEventKind,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
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
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use ratatui::{backend::CrosstermBackend, text::Line};

const EVENT_BATCH_SIZE: usize = 256;
const OBJECT_CACHE_SIZE: usize = 4 * 1024 * 1024;
const FRAME_INTERVAL: Duration = Duration::from_nanos(16_666_667);
const HISTORY_STATUS_DELAY: Duration = Duration::from_millis(500);
const REPEAT_IDLE: Duration = Duration::from_millis(75);
const WORKTREE_EVENT_IDLE: Duration = Duration::from_millis(75);
const REF_EVENT_IDLE: Duration = Duration::from_millis(100);
const IMMEDIATE_PAGER_EXIT: Duration = Duration::from_millis(250);
const REF_EVENT_INTERVAL: Duration = Duration::from_millis(250);
const WATCH_RETRY_INTERVAL: Duration = Duration::from_secs(5);
const THEME_QUERY_TIMEOUT: Duration = Duration::from_millis(100);

struct FillRepository {
    path: PathBuf,
    bare: bool,
    retained: Option<gix::Repository>,
    retain: bool,
}

struct WorktreeWatcher {
    _watcher: RecommendedWatcher,
    events: mpsc::Receiver<notify::Result<notify::Event>>,
    directories: HashSet<PathBuf>,
    workdir: PathBuf,
    dot_git: PathBuf,
    git_dir: PathBuf,
    index: PathBuf,
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

    fn watch_set_may_change(&self, event: &notify::Event) -> bool {
        worktree_watch_set_may_change(event, &self.index, &self.directories)
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
            && event.paths.iter().any(|path| {
                path == index || (path.starts_with(workdir) && !path.starts_with(dot_git) && !path.starts_with(git_dir))
            }))
}

fn worktree_watch_set_may_change(event: &notify::Event, index: &Path, directories: &HashSet<PathBuf>) -> bool {
    if event.need_rescan()
        || event
            .paths
            .iter()
            .any(|path| path == index || path.file_name().is_some_and(|name| name == ".gitignore"))
    {
        return true;
    }
    match event.kind {
        notify::EventKind::Create(notify::event::CreateKind::Folder)
        | notify::EventKind::Remove(notify::event::RemoveKind::Folder)
        | notify::EventKind::Modify(notify::event::ModifyKind::Name(_)) => true,
        notify::EventKind::Create(notify::event::CreateKind::Any | notify::event::CreateKind::Other)
        | notify::EventKind::Any => event
            .paths
            .iter()
            .any(|path| path.is_dir() || directories.contains(path)),
        notify::EventKind::Remove(notify::event::RemoveKind::Any | notify::event::RemoveKind::Other) => {
            event.paths.iter().any(|path| directories.contains(path))
        }
        _ => false,
    }
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
    notification_is_actionable(event)
        && (event.need_rescan()
            || event.paths.is_empty()
            || event.paths.iter().any(|path| {
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

#[derive(Default)]
struct MotionState {
    shown: Option<animation::Snapshot>,
    pending: Option<animation::Snapshot>,
    active: Option<animation::Emphasis>,
    last_tick: Option<Instant>,
}

impl MotionState {
    fn capture(&mut self) {
        if self.pending.is_none() {
            let source = self
                .active
                .as_ref()
                .map(|emphasis| emphasis.target().clone())
                .or_else(|| self.shown.clone());
            if let Some(source) = source {
                self.pending = Some(source);
            }
        }
    }

    fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    fn transition_ids(&self, target: &animation::Snapshot) -> Vec<gix::ObjectId> {
        let Some(source) = &self.pending else {
            return Vec::new();
        };
        if source
            .rows
            .iter()
            .map(|row| row.id)
            .eq(target.rows.iter().map(|row| row.id))
        {
            return Vec::new();
        }
        source.rows.iter().chain(&target.rows).map(|row| row.id).collect()
    }

    fn begin(
        &mut self,
        target: animation::Snapshot,
        trees: &HashMap<gix::ObjectId, gix::ObjectId>,
        now: Instant,
    ) -> Option<ratatui::buffer::Buffer> {
        let Some(mut source) = self.pending.take() else {
            self.shown = Some(target);
            return None;
        };
        if source.buffer.area != target.buffer.area || source.buffer == target.buffer {
            self.shown = Some(target);
            self.last_tick = None;
            return None;
        }
        source.set_trees(trees);
        let mut target = target;
        target.set_trees(trees);
        let Some(emphasis) = animation::Emphasis::new(source, target.clone()) else {
            self.shown = Some(target);
            self.active = None;
            self.last_tick = None;
            return None;
        };
        let displayed = emphasis.displayed().clone();
        self.active = Some(emphasis);
        self.last_tick = Some(now);
        Some(displayed)
    }

    fn timeout(&self, now: Instant) -> Option<Duration> {
        self.active.as_ref().map(|emphasis| {
            let since_tick = self
                .last_tick
                .map_or(Duration::ZERO, |last_tick| now.saturating_duration_since(last_tick));
            emphasis.timeout().saturating_sub(since_tick)
        })
    }

    fn advance(&mut self, now: Instant) -> Option<ratatui::buffer::Buffer> {
        let elapsed = self
            .last_tick
            .replace(now)
            .map_or(Duration::ZERO, |previous| now.saturating_duration_since(previous));
        let emphasis = self.active.as_mut()?;
        let frame = emphasis.advance(elapsed).cloned();
        if emphasis.is_complete() {
            self.shown = Some(emphasis.target().clone());
            self.active = None;
            self.last_tick = None;
        }
        frame
    }

    fn finish(&mut self) -> Option<ratatui::buffer::Buffer> {
        let emphasis = self.active.take()?;
        let target = emphasis.target().clone();
        let buffer = target.buffer.clone();
        self.shown = Some(target);
        self.last_tick = None;
        Some(buffer)
    }

    fn show(&mut self, target: animation::Snapshot) -> ratatui::buffer::Buffer {
        self.active = None;
        self.last_tick = None;
        let buffer = target.buffer.clone();
        self.shown = Some(target);
        buffer
    }

    fn cancel_pending(&mut self) {
        self.pending = None;
    }
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
    jobs: Vec<mpsc::Sender<LineDiffMessage>>,
    results: mpsc::Receiver<LineDiffResult>,
    workers: Vec<std::thread::JoinHandle<()>>,
}

type LineDiffState = (
    gix::Repository,
    gix::diff::blob::Platform,
    Option<gix::diff::blob::Platform>,
);

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

fn open_line_diff_state(repository_path: &Path, bare: bool) -> Result<LineDiffState> {
    let mut repository =
        open_repository(repository_path, bare, false).context("could not open repository for parallel line diffs")?;
    repository.object_cache_size(OBJECT_CACHE_SIZE);
    let tree_cache = repository
        .diff_resource_cache_for_tree_diff()
        .context("could not initialize parallel line diffs")?;
    let worktree_cache = if bare {
        None
    } else {
        worktree_diff_cache(&repository, gix::diff::blob::pipeline::Mode::ToGit)?
    };
    Ok((repository, tree_cache, worktree_cache))
}

impl LineDiffPool {
    fn new(repository_path: &Path, bare: bool, parallelism: usize) -> Result<Self> {
        drop(open_line_diff_state(repository_path, bare)?);
        let (result_sender, results) = mpsc::channel();
        let mut jobs = Vec::with_capacity(parallelism);
        let workers = (0..parallelism)
            .map(|_| {
                let (job_sender, job_receiver) = mpsc::channel();
                jobs.push(job_sender);
                let result_sender = result_sender.clone();
                let repository_path = repository_path.to_owned();
                std::thread::spawn(move || {
                    let mut state: Option<LineDiffState> = None;
                    while let Ok(message) = job_receiver.recv() {
                        match message {
                            LineDiffMessage::Job(job) => {
                                let result = (|| {
                                    if state.is_none() {
                                        state = Some(open_line_diff_state(&repository_path, bare)?);
                                    }
                                    let (repository, tree_cache, worktree_cache) =
                                        state.as_mut().expect("line diff state was just initialized");
                                    let result = line_counts_for_change(
                                        repository,
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
        Ok(LineDiffPool { jobs, results, workers })
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

impl Drop for LineDiffPool {
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
) -> Result<()> {
    if visible && pool.is_none() {
        *pool = Some(LineDiffPool::new(repository_path, bare, parallelism.max(1))?);
    } else if !visible {
        *pool = None;
    }
    Ok(())
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
    /// Exit once all commits and graph lanes have been computed.
    pub quit_on_finish: bool,
    /// Revisions whose reachable commits should initially be hidden.
    pub hide: Vec<OsString>,
    /// Add every successfully resolved worktree HEAD as a visible traversal tip.
    pub worktrees: bool,
}

/// An edit applied to the commit checked out by the current worktree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HeadEdit {
    /// Add staged changes, or worktree changes when nothing is staged, to `HEAD`.
    Amend,
    /// Move the changes introduced by `HEAD` into the worktree.
    Spill,
}

/// Apply `edit` to the current worktree's `HEAD` without starting the terminal UI.
pub fn edit_head(repository: gix::ThreadSafeRepository, edit: HeadEdit) -> Result<Option<gix::ObjectId>> {
    let _log_guard = logging::init().context("could not initialize tix diagnostics")?;
    let repository = repository.to_thread_local();
    let graph = edit::loaded_graph(&repository)?;
    edit::head::perform(
        repository,
        &graph,
        match edit {
            HeadEdit::Amend => edit::head::Kind::Amend,
            HeadEdit::Spill => edit::head::Kind::Spill,
        },
        None,
    )
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
    let _log_guard = match logging::init() {
        Ok(guard) => Some(guard),
        Err(err) => {
            eprintln!("warning: could not initialize tix diagnostics: {err:#}");
            None
        }
    };
    let mut repository_path = repository.git_dir().to_owned();
    let common_dir = normalize_common_dir(repository.common_dir.clone().unwrap_or_else(|| repository_path.clone()))?;
    let (validation_repository, _) = open_history_repository(&mut repository_path, &common_dir)?;
    let (hide, unavailable) = history::available_hidden_revisions(&validation_repository, &options.hide)?;
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
        include_worktrees = options.worktrees,
        "starting tix"
    );
    let commit_pane_background = detect_commit_pane_background();
    let mut terminal = ratatui::try_init().context("could not initialize terminal")?;
    let enhanced_keyboard = terminal::supports_keyboard_enhancement().unwrap_or(false);
    let keyboard_setup = enable_input(terminal.backend_mut(), enhanced_keyboard);
    let result = keyboard_setup
        .context("could not enable enhanced keyboard events")
        .and_then(|()| {
            event_loop(
                &mut terminal,
                repository,
                revisions,
                options,
                enhanced_keyboard,
                commit_pane_background,
            )
        });
    let keyboard_restore = disable_input(terminal.backend_mut(), enhanced_keyboard);
    let restore = ratatui::try_restore().context("could not restore terminal");
    let lane_time = result?;
    keyboard_restore.context("could not restore keyboard events")?;
    restore?;
    if let Some(lane_time) = lane_time {
        eprintln!("lane computation: {:.3}s", lane_time.as_secs_f64());
    }
    Ok(())
}

fn enable_input(backend: &mut CrosstermBackend<std::io::Stdout>, enhanced_keyboard: bool) -> std::io::Result<()> {
    execute!(backend, EnableFocusChange, EnableMouseCapture)?;
    if enhanced_keyboard {
        execute!(
            backend,
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                    | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
            )
        )?;
    }
    Ok(())
}

fn disable_input(backend: &mut CrosstermBackend<std::io::Stdout>, enhanced_keyboard: bool) -> std::io::Result<()> {
    if enhanced_keyboard {
        execute!(backend, PopKeyboardEnhancementFlags)?;
    }
    execute!(backend, DisableMouseCapture, DisableFocusChange)
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
        quit_on_finish,
        hide,
        worktrees,
    } = options;
    let mut repository_path = repository.git_dir().to_owned();
    let common_dir = normalize_common_dir(repository.common_dir.clone().unwrap_or_else(|| repository_path.clone()))?;
    let (mut view_repository, recovered_at_startup) = open_history_repository(&mut repository_path, &common_dir)?;
    view_repository.object_cache_size(None);
    let (mut repository_is_bare, mut mailmap, mut ref_snapshot, mut worktree_head_unborn) = {
        let bare = view_repository.workdir().is_none();
        let mailmap = view_repository.open_mailmap();
        let refs = history::snapshot(&view_repository, &revisions, &hide, worktrees)?;
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
    let (cancelled, receiver) = start_history(
        repository,
        &revisions,
        &hide,
        worktrees,
        gix::features::threading::OwnShared::clone(&authors),
    );

    let mut app = App::new(1);
    app.set_worktree_head_unborn(worktree_head_unborn);
    app.commit_pane_background = commit_pane_background;
    if recovered_at_startup {
        app.leave_message("worktree removed; using the common repository without worktree changes");
    }
    app.manual_refresh = ref_watcher.is_none();
    let mut lane_receiver = None;
    let mut refresh_receiver: Option<mpsc::Receiver<(HistoryGraph, Result<history::Refresh>)>> = None;
    let mut refresh_pending = false;
    let mut refresh_from_filesystem = false;
    let mut ref_refresh_deadline: Option<Instant> = None;
    let mut refresh_expand_hidden = false;
    let mut verification_receiver = None;
    let mut commit_message = None;
    let mut tree_changes = TreeChangesCache::default();
    let mut worktree_changes = None;
    let mut worktree_watcher: Option<WorktreeWatcher> = None;
    let mut worktree_refresh_deadline: Option<Instant> = None;
    let mut worktree_watch_set_changed = false;
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
    )?;
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
    let mut motion = MotionState::default();
    let mut filesystem_responses = logging::FilesystemResponses::default();
    let mut focused = true;
    draw(
        terminal,
        &mut app,
        &decorations,
        &mailmap,
        &authors,
        &mut fill_repository,
        &mut commit_message,
        &mut tree_changes,
        &mut worktree_changes,
        &mut history_graph,
        &mut selection_relation,
        &mut line_diff_pool,
        &mut motion,
        focused,
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
    let result: Result<Option<Duration>> = (|| loop {
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
            worktree_watcher = None;
            worktree_refresh_deadline = None;
            worktree_watch_set_changed = false;
            filesystem_responses.cancel_pending_worktree("worktree-unavailable");
            worktree_changes = None;
            line_diff_pool = None;
            sync_line_diff_pool(
                &mut line_diff_pool,
                app.changes_mode.is_some(),
                &repository_path,
                true,
                line_diff_parallelism,
            )?;
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
            app.manual_refresh = ref_watcher.is_none();
            app.leave_message("worktree removed; using the common repository without worktree changes");
            if history_graph.is_some() {
                refresh_pending = true;
                refresh_from_filesystem = true;
            }
            dirty = true;
            urgent = true;
        }
        let mut worktree_watch_error = None;
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
                            worktree_watch_set_changed |= watcher.watch_set_may_change(&event);
                            filesystem_responses.observe_worktree(&event, &watcher.workdir, &watcher.index);
                            schedule_once(&mut worktree_refresh_deadline, Instant::now(), WORKTREE_EVENT_IDLE);
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
        }
        if let Some(err) = worktree_watch_error {
            tracing::warn!(error = %err, "worktree watcher failed");
            filesystem_responses.fail_pending_worktree();
            app.worktree_changes.error = Some(format!("worktree watch: {err}"));
            worktree_watcher = None;
            worktree_refresh_deadline = None;
            worktree_watch_set_changed = false;
            schedule_once(&mut watcher_retry_deadline, Instant::now(), WATCH_RETRY_INTERVAL);
            dirty = true;
            urgent = true;
        }
        if take_due(&mut worktree_refresh_deadline, Instant::now()) {
            if std::mem::take(&mut worktree_watch_set_changed) {
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
            let invalidated = invalidate_worktree_changes(&mut worktree_changes);
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
            app.manual_refresh = true;
            schedule_once(&mut watcher_retry_deadline, Instant::now(), WATCH_RETRY_INTERVAL);
        }
        if take_due(&mut ref_refresh_deadline, Instant::now()) {
            if std::mem::take(&mut ref_watch_set_changed) {
                match start_ref_watcher(&repository_path, &common_dir) {
                    Ok(watcher) => {
                        ref_watcher = Some(watcher);
                        app.manual_refresh = false;
                    }
                    Err(err) => {
                        tracing::warn!(error = %err, "reference watcher rebuild failed");
                        ref_watcher = None;
                        app.manual_refresh = true;
                        schedule_once(&mut watcher_retry_deadline, Instant::now(), WATCH_RETRY_INTERVAL);
                    }
                }
            }
            let response_ids = filesystem_responses.references_due();
            refresh_pending = true;
            refresh_from_filesystem = true;
            let invalidated = invalidate_worktree_changes(&mut worktree_changes);
            filesystem_responses.phase(&response_ids, "reference-worktree-cache-invalidation");
            if invalidated {
                filesystem_responses.queue_frame(&response_ids, "reference-worktree-cache-invalidation");
                dirty = true;
                urgent = true;
            }
        }
        if take_due(&mut watcher_retry_deadline, Instant::now()) {
            let mut retry = false;
            if ref_watcher.is_none() {
                match start_ref_watcher(&repository_path, &common_dir) {
                    Ok(watcher) => {
                        tracing::info!("reference watcher recovered");
                        ref_watcher = Some(watcher);
                        app.manual_refresh = false;
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
                    app.finish_lane_computation(rows, graph, lane_time);
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
                        return Ok(app.lane_time);
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
                Ok((graph, result)) => {
                    app.set_known_descendants(graph.commits_with_descendants());
                    app.set_known_merge_descendants(graph.commits_with_merge_descendants());
                    history_graph = Some(graph);
                    let result = result?;
                    tracing::info!(commit_count = result.commits.rows.len(), "history refresh completed");
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
            let next = history::snapshot(&repository, &revisions, &hide, worktrees)?;
            let hidden_changed = next.hidden != ref_snapshot.hidden;
            let worktree_tips_changed = worktrees && next.worktrees != ref_snapshot.worktrees;
            let tips_changed = next.view != ref_snapshot.view || hidden_changed || worktree_tips_changed;
            let from_filesystem = std::mem::take(&mut refresh_from_filesystem);
            if tips_changed && from_filesystem {
                motion.capture();
            } else {
                motion.cancel_pending();
            }
            tracing::debug!(
                ?response_ids,
                tips_changed,
                hidden_changed,
                "compared reference snapshot"
            );
            ref_snapshot = next;
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
                worktrees,
                expand,
                gix::features::threading::OwnShared::clone(&authors),
                history_graph
                    .take()
                    .expect("refresh starts only with a cached history graph"),
            ));
            refresh_expand_hidden = false;
            app.deferred_history_state = Some(app.state);
            history_status_deadline = Some(refresh_started + HISTORY_STATUS_DELAY);
            app.state = State::Loading;
            filesystem_responses.phase(&response_ids, "history-refresh-started");
            tracing::info!(?response_ids, "started history refresh");
        }
        let now = Instant::now();
        if motion.timeout(now) == Some(Duration::ZERO)
            && let Some(frame) = motion.advance(now)
        {
            if present_buffer(terminal, &frame)? {
                filesystem_responses.emphasis_finished("history-emphasis-settled", "completed");
                filesystem_responses.frame_presented();
            } else {
                filesystem_responses.emphasis_aborted("terminal-area-changed");
            }
            last_draw = now;
        }
        if urgent {
            draw(
                terminal,
                &mut app,
                &decorations,
                &mailmap,
                &authors,
                &mut fill_repository,
                &mut commit_message,
                &mut tree_changes,
                &mut worktree_changes,
                &mut history_graph,
                &mut selection_relation,
                &mut line_diff_pool,
                &mut motion,
                focused,
                &mut filesystem_responses,
            )?;
            last_draw = Instant::now();
            dirty = false;
            urgent = false;
            if repeat_deadline.is_none() {
                fill_repository.retain = false;
                fill_repository.retained = None;
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
                &decorations,
                &mailmap,
                &authors,
                &mut fill_repository,
                &mut commit_message,
                &mut tree_changes,
                &mut worktree_changes,
                &mut history_graph,
                &mut selection_relation,
                &mut line_diff_pool,
                &mut motion,
                focused,
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
        let animation_timeout = motion.timeout(Instant::now());
        let wake_after = [
            repeat_timeout,
            watcher_timeout,
            ref_refresh_timeout,
            worktree_timeout,
            retry_timeout,
            history_status_timeout,
            animation_timeout,
        ]
        .into_iter()
        .flatten()
        .min();
        let terminal_event = match pending_terminal_event.take() {
            Some(event) => Some(event),
            None => match poll_timeout(streaming, events, dirty, last_draw.elapsed(), wake_after) {
                Some(timeout) if event::poll(timeout)? => Some(event::read()?),
                Some(_) => None,
                None => Some(event::read()?),
            },
        };
        let Some(terminal_event) = terminal_event else {
            continue;
        };
        let key_pressed = is_key_press(&terminal_event);
        let (action, repeats_history, is_repeat, throttles_draw) = match terminal_event {
            TerminalEvent::Key(key) => {
                let action = action_with_shortcut_groups(key, app.history_display_expanded, app.edit_expanded);
                let repeats_history = retains_fill_repository(key.kind, action.as_ref(), app.changes_focus.is_some());
                (action, repeats_history, key.kind == KeyEventKind::Repeat, false)
            }
            TerminalEvent::Mouse(mouse) => {
                let kind = mouse.kind;
                let mut distance = 1;
                if matches!(kind, MouseEventKind::ScrollUp | MouseEventKind::ScrollDown) {
                    while distance < EVENT_BATCH_SIZE && event::poll(Duration::ZERO)? {
                        let next = event::read()?;
                        match next {
                            TerminalEvent::Mouse(next) if next.kind == kind => distance += 1,
                            next => {
                                pending_terminal_event = Some(next);
                                break;
                            }
                        }
                    }
                }
                let Some(action) = mouse_scroll_action(kind, distance) else {
                    continue;
                };
                let repeats_history = app.changes_focus.is_none() && repeats_viewport(&action);
                (Some(action), repeats_history, true, true)
            }
            TerminalEvent::FocusLost => {
                if let Some(frame) = motion.finish() {
                    if present_buffer(terminal, &frame)? {
                        filesystem_responses.emphasis_finished("emphasis-interrupted-by-focus", "interrupted-by-focus");
                        filesystem_responses.frame_presented();
                    } else {
                        filesystem_responses.emphasis_aborted("terminal-area-changed");
                    }
                }
                motion.cancel_pending();
                focused = false;
                app.changes_suppressed = false;
                repeat_deadline = None;
                drop(app.update(Action::PreviewAuthorCopy(false)));
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
                if let Some(frame) = motion.finish() {
                    if present_buffer(terminal, &frame)? {
                        filesystem_responses
                            .emphasis_finished("emphasis-interrupted-by-resize", "interrupted-by-resize");
                        filesystem_responses.frame_presented();
                    } else {
                        filesystem_responses.emphasis_aborted("terminal-area-changed");
                    }
                }
                motion.cancel_pending();
                dirty = true;
                urgent = true;
                continue;
            }
            _ => continue,
        };
        if action.as_ref().is_some_and(|action| action != &Action::ForceQuit) {
            if let Some(frame) = motion.finish() {
                if present_buffer(terminal, &frame)? {
                    filesystem_responses.emphasis_finished("emphasis-interrupted-by-input", "interrupted-by-input");
                    filesystem_responses.frame_presented();
                } else {
                    filesystem_responses.emphasis_aborted("terminal-area-changed");
                }
                last_draw = Instant::now();
            }
            motion.cancel_pending();
        }
        if !focused {
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
        if key_pressed && pending_rebase_conflict.is_some() {
            if action == Some(Action::OpenDiff) && app.changes_focus.is_none() {
                let conflict = pending_rebase_conflict
                    .take()
                    .expect("a pending conflict was checked before accepting it");
                let original = conflict.original();
                match conflict.accept() {
                    Ok((notice, id)) => {
                        tracing::info!(commit_id = %original, rewritten_id = %id, "accepted suspended rebase conflict");
                        app.begin_conflict_resolution();
                        app.leave_message(notice);
                        app.select_commit_after_refresh(id);
                    }
                    Err(err) => {
                        tracing::warn!(commit_id = %original, error = %err, "suspended rebase conflict checkout failed");
                        app.clear_rebase_conflict();
                        app.leave_message(format!("conflict checkout: {err:#}"));
                    }
                }
                sync_line_diff_pool(
                    &mut line_diff_pool,
                    app.changes_mode.is_some(),
                    &repository_path,
                    repository_is_bare,
                    line_diff_parallelism,
                )?;
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
            let conflict = pending_rebase_conflict
                .take()
                .expect("a pending conflict was checked before discarding it");
            tracing::info!(commit_id = %conflict.original(), ?action, "discarded suspended rebase conflict");
            app.clear_rebase_conflict();
            dirty = true;
            urgent = true;
        }
        let Some(action) = action else {
            continue;
        };
        let action = copy_selected_path_action(
            action,
            &app,
            tree_changes.as_ref().map(|(_, changes)| changes),
            worktree_changes.as_ref().map(|(_, changes)| changes),
        );
        dirty = true;
        urgent |= !throttles_draw;
        let previous_changes_mode = app.changes_mode;
        let toggles_changes = action == Action::ToggleChanges;
        let refreshes_worktree = action == Action::Refresh && app.changes_mode == Some(ChangesMode::Both);
        let effects = app.update(action);
        if refreshes_worktree {
            invalidate_worktree_changes(&mut worktree_changes);
            worktree_watch_set_changed = false;
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
            )?;
            if app.changes_mode == Some(ChangesMode::Both) {
                invalidate_worktree_changes(&mut worktree_changes);
                worktree_watch_set_changed = false;
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
                worktree_watch_set_changed = false;
                filesystem_responses.cancel_pending_worktree("watcher-disabled");
            }
        }
        for effect in effects {
            match effect {
                Effect::Cancel => cancelled.store(true, Ordering::Relaxed),
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
                        Err(err) => app.leave_message(format!("diff: {err:#}")),
                        Ok(false) => {}
                    }
                }
                Effect::Reword(id) => {
                    let result = history_graph
                        .as_ref()
                        .context("reword requires a completed history graph")
                        .and_then(|graph| {
                            reword_commit(
                                terminal,
                                &repository_path,
                                repository_is_bare,
                                graph,
                                id,
                                enhanced_keyboard,
                            )
                        });
                    match result {
                        Ok(Some(new_id)) => {
                            app.leave_message(format!(
                                "reworded {} as {}",
                                id.to_hex_with_len(7),
                                new_id.to_hex_with_len(7)
                            ));
                            app.select_commit_after_refresh(new_id);
                            refresh_pending = true;
                        }
                        Ok(None) => {}
                        Err(err) => app.leave_message(format!("reword: {err:#}")),
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
                        Ok(Some(new_id)) => {
                            app.leave_message(format!("created {}", new_id.to_hex_with_len(7)));
                            app.select_commit_after_refresh(new_id);
                            refresh_pending = true;
                        }
                        Ok(None) => app.leave_message("no commit created: no input was provided"),
                        Err(err) => app.leave_message(format!("new commit: {err:#}")),
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
                        Ok(Some(new_id)) => {
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
                                        worktrees,
                                    )
                                });
                            match travel {
                                Ok(edit::time_travel::Perform::Complete(notice)) => {
                                    app.leave_message(notice.map_or_else(
                                        || format!("created fork {}", new_id.to_hex_with_len(7)),
                                        |notice| format!("created fork {}; {notice}", new_id.to_hex_with_len(7)),
                                    ));
                                    app.select_commit_after_refresh(new_id);
                                    invalidate_worktree_changes(&mut worktree_changes);
                                    refresh_pending = true;
                                }
                                Ok(edit::time_travel::Perform::Conflict(conflict)) => {
                                    let original = conflict.original();
                                    app.arm_rebase_conflict(original);
                                    app.select_commit(original);
                                    pending_rebase_conflict = Some(conflict);
                                }
                                Err(err) => {
                                    app.leave_message(format!(
                                        "created fork {}, but checkout failed: {err:#}",
                                        new_id.to_hex_with_len(7)
                                    ));
                                    refresh_pending = true;
                                }
                            }
                        }
                        Ok(None) => app.leave_message("no fork created: no input was provided"),
                        Err(err) => app.leave_message(format!("fork: {err:#}")),
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
                        Ok(Some(new_id)) => {
                            app.leave_message(format!(
                                "split {} as {}",
                                id.to_hex_with_len(7),
                                new_id.to_hex_with_len(7)
                            ));
                            invalidate_worktree_changes(&mut worktree_changes);
                            app.select_commit_after_refresh(new_id);
                            refresh_pending = true;
                        }
                        Ok(None) => app.leave_message("no split performed: no input was provided"),
                        Err(err) => app.leave_message(format!("split: {err:#}")),
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
                    let path = (kind == edit::head::Kind::Spill && app.changes_focus == Some(ChangePane::Tree))
                        .then(|| {
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
                                .context("selected tree path is no longer available")
                        })
                        .transpose();
                    let result = history_graph
                        .as_ref()
                        .context("editing HEAD requires a completed history graph")
                        .and_then(|graph| {
                            path.and_then(|path| {
                                edit::head::perform(
                                    open_repository(&repository_path, repository_is_bare, false)
                                        .context("could not open repository for HEAD edit")?,
                                    graph,
                                    kind,
                                    path.as_ref().map(|(path, parent)| (path, *parent)),
                                )
                            })
                        });
                    match result {
                        Ok(Some(new_id)) => {
                            app.leave_message(format!(
                                "{verb}ed {} as {}",
                                id.to_hex_with_len(7),
                                new_id.to_hex_with_len(7)
                            ));
                            invalidate_worktree_changes(&mut worktree_changes);
                            app.select_commit_after_refresh(new_id);
                            refresh_pending = true;
                        }
                        Ok(None) => app.leave_message(format!("nothing to {verb}")),
                        Err(err) => app.leave_message(format!("{verb}: {err:#}")),
                    }
                }
                Effect::Forget(id) => {
                    fill_repository.retain = false;
                    fill_repository.retained = None;
                    let result = history_graph
                        .as_ref()
                        .context("forget requires a completed history graph")
                        .and_then(|graph| forget_commit(&repository_path, repository_is_bare, graph, id));
                    match result {
                        Ok(parent) => {
                            app.leave_message(format!("forgot {}", id.to_hex_with_len(7)));
                            if let Some(parent) = parent {
                                app.select_commit(parent);
                            }
                            invalidate_worktree_changes(&mut worktree_changes);
                            refresh_pending = true;
                        }
                        Err(err) => app.leave_message(format!("forget: {err:#}")),
                    }
                }
                Effect::Rebase {
                    base,
                    onto,
                    commits,
                    head,
                } => {
                    fill_repository.retain = false;
                    fill_repository.retained = None;
                    let todo_commits = commits
                        .iter()
                        .map(|id| {
                            let row = app
                                .rows
                                .iter()
                                .find(|row| row.id == *id)
                                .context("an editable commit disappeared from the view")?;
                            Ok(edit::todo::Commit {
                                id: *id,
                                parents: row.parent_ids.to_vec(),
                                info: ui::todo_metadata(&app, row, &decorations, &mailmap),
                            })
                        })
                        .collect::<Result<Vec<_>>>();
                    let result = history_graph
                        .as_ref()
                        .context("rebasing requires a completed history graph")
                        .and_then(|graph| {
                            let anchor_title = ui::todo_title(
                                &app,
                                app.commit(onto)
                                    .context("the rebase anchor disappeared from the view")?,
                            );
                            rebase_history(
                                terminal,
                                &repository_path,
                                repository_is_bare,
                                graph,
                                base,
                                onto,
                                anchor_title,
                                todo_commits?,
                                head,
                                enhanced_keyboard,
                            )
                        });
                    match result {
                        Ok(Some(edit::rebase::Perform::Complete(outcome))) => {
                            let checkout = outcome.selected;
                            let notice = checkout
                                .map(|selected| {
                                    edit::time_travel::checkout_without_replay(
                                        &repository_path,
                                        repository_is_bare,
                                        selected,
                                        &revisions,
                                        worktrees,
                                    )
                                })
                                .transpose()
                                .map(Option::flatten);
                            match notice {
                                Ok(notice) => {
                                    app.leave_message(notice.unwrap_or_else(|| "rebased history".to_owned()));
                                    app.select_commit_after_refresh(base);
                                    invalidate_worktree_changes(&mut worktree_changes);
                                    refresh_pending = true;
                                }
                                Err(err) => {
                                    app.leave_message(format!("rebase applied, checkout failed: {err:#}"));
                                    invalidate_worktree_changes(&mut worktree_changes);
                                    refresh_pending = true;
                                }
                            }
                        }
                        Ok(Some(edit::rebase::Perform::Conflict(conflict))) => {
                            match edit::time_travel::Conflict::from_rebase(
                                &repository_path,
                                repository_is_bare,
                                conflict,
                                &revisions,
                                worktrees,
                            ) {
                                Ok(conflict) => {
                                    let original = conflict.original();
                                    app.arm_rebase_conflict(original);
                                    app.select_commit(original);
                                    pending_rebase_conflict = Some(conflict);
                                }
                                Err(err) => app.leave_message(format!("rebase conflict: {err:#}")),
                            }
                        }
                        Ok(None) => app.leave_message("no rebase performed: the todo was unchanged"),
                        Err(err) => app.leave_message(format!("rebase: {err:#}")),
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
                            app.leave_message(format!(
                                "started review {} at {}",
                                started.reference.shorten(),
                                started.commit.to_hex_with_len(7)
                            ));
                            app.select_commit_after_refresh(started.commit);
                            invalidate_worktree_changes(&mut worktree_changes);
                            refresh_pending = true;
                        }
                        Err(err) => app.leave_message(format!("review: {err:#}")),
                    }
                }
                Effect::FinishReview(id) => {
                    fill_repository.retain = false;
                    fill_repository.retained = None;
                    let result = history_graph
                        .as_ref()
                        .context("finishing review requires a completed history graph")
                        .and_then(|graph| {
                            let mut repo = open_repository(&repository_path, repository_is_bare, false)
                                .context("could not open repository to finish review")?;
                            repo.object_cache_size(None);
                            edit::review::finish(repo, graph, id)
                        });
                    match result {
                        Ok(new_id) => {
                            app.leave_message(format!("finished review as {}", new_id.to_hex_with_len(7)));
                            app.select_commit_after_refresh(new_id);
                            invalidate_worktree_changes(&mut worktree_changes);
                            refresh_pending = true;
                        }
                        Err(err) => app.leave_message(format!("finish review: {err:#}")),
                    }
                }
                Effect::TimeTravel(id) => {
                    fill_repository.retain = false;
                    fill_repository.retained = None;
                    let review_roots: Vec<_> = app.rows.iter().filter(|row| row.is_review).map(|row| row.id).collect();
                    let result = history_graph
                        .as_ref()
                        .context("time-travel requires a completed history graph")
                        .and_then(|graph| {
                            edit::time_travel::perform(
                                &repository_path,
                                repository_is_bare,
                                id,
                                graph,
                                &review_roots,
                                &revisions,
                                worktrees,
                            )
                        });
                    match result {
                        Ok(edit::time_travel::Perform::Complete(Some(notice))) => {
                            tracing::info!(selected = %id, %notice, "completed time-travel action");
                            app.leave_message(notice);
                            if let Ok(head) = open_repository(&repository_path, repository_is_bare, false)
                                .and_then(|repo| Ok(repo.head_id()?.detach()))
                            {
                                app.select_commit_after_refresh(head);
                            }
                            invalidate_worktree_changes(&mut worktree_changes);
                            refresh_pending = true;
                        }
                        Ok(edit::time_travel::Perform::Complete(None)) => {}
                        Ok(edit::time_travel::Perform::Conflict(conflict)) => {
                            let original = conflict.original();
                            app.arm_rebase_conflict(original);
                            app.select_commit(original);
                            pending_rebase_conflict = Some(conflict);
                        }
                        Err(err) => {
                            app.leave_message(format!("time-travel: {err:#}"));
                            invalidate_worktree_changes(&mut worktree_changes);
                            refresh_pending = true;
                        }
                    }
                }
                Effect::VerifySignatures(ids) => {
                    verification_receiver = Some(start_signature_verification(
                        repository_path.clone(),
                        repository_is_bare,
                        ids,
                    ));
                }
                Effect::Quit => return Ok(None),
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
) -> mpsc::Receiver<(HistoryGraph, Result<history::Refresh>)> {
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
        let _ = sender.send((graph, result));
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
    let index = repository.index_path();
    let git_dir = repository.git_dir().to_owned();
    let dot_git = workdir.join(gix::discover::DOT_GIT_DIR);
    let dirwalk_started = Instant::now();
    let directories = worktree_watch_directories(&repository)?;
    let dirwalk_ms = dirwalk_started.elapsed().as_millis();
    let registration_started = Instant::now();
    let (sender, events) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = sender.send(event);
    })
    .context("could not initialize worktree watcher")?;
    let index_parent = index.parent().context("index path has no parent")?;
    {
        let mut paths = watcher.paths_mut();
        for directory in &directories {
            paths
                .add(directory, RecursiveMode::NonRecursive)
                .with_context(|| format!("could not watch worktree directory at {}", directory.display()))?;
        }
        if !directories.contains(index_parent) {
            paths
                .add(index_parent, RecursiveMode::NonRecursive)
                .with_context(|| format!("could not watch index at {}", index_parent.display()))?;
        }
        paths.commit().context("could not apply worktree watches")?;
    }
    tracing::info!(
        workdir = %workdir.display(),
        index = %index.display(),
        directories = directories.len(),
        dirwalk_ms,
        registration_ms = registration_started.elapsed().as_millis(),
        setup_ms = started.elapsed().as_millis(),
        "watching worktree changes"
    );
    Ok(WorktreeWatcher {
        _watcher: watcher,
        events,
        directories,
        workdir,
        dot_git,
        git_dir,
        index,
    })
}

fn worktree_watch_directories(repository: &gix::Repository) -> Result<HashSet<PathBuf>> {
    let root = repository
        .workdir()
        .context("cannot walk a bare repository")?
        .to_owned();
    let index = repository
        .index_or_empty()
        .context("could not open index for worktree watcher")?;
    let options = repository
        .dirwalk_options()
        .context("could not configure worktree directory walk")?;
    let mut directories = WorktreeDirectories {
        root: root.clone(),
        paths: HashSet::from([root]),
    };
    repository
        .dirwalk(&index, None::<&str>, &AtomicBool::default(), options, &mut directories)
        .context("could not enumerate worktree directories")?;
    Ok(directories.paths)
}

fn invalidate_worktree_changes(changes: &mut Option<(usize, Changes)>) -> bool {
    if let Some((marker, _)) = changes {
        if *marker == usize::MAX {
            return false;
        }
        *marker = usize::MAX;
        return true;
    }
    false
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

#[expect(clippy::too_many_arguments, reason = "drawing needs the complete view state")]
fn draw(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    decorations: &Decorations,
    mailmap: &gix::mailmap::Snapshot,
    authors: &SharedAuthors,
    fill_repository: &mut FillRepository,
    commit_message: &mut Option<(gix::ObjectId, BString)>,
    tree_changes: &mut TreeChangesCache,
    worktree_changes: &mut Option<(usize, Changes)>,
    history_graph: &mut Option<HistoryGraph>,
    selection_cache: &mut Option<SelectionRelationCache>,
    line_diff_pool: &mut Option<LineDiffPool>,
    motion: &mut MotionState,
    focused: bool,
    filesystem_responses: &mut logging::FilesystemResponses,
) -> Result<()> {
    let render_rows = terminal.get_frame().area().height.saturating_sub(1) as usize;
    if !history_is_ready_to_draw(app.state, app.rows.len()) {
        return Ok(());
    }
    app.unseen_filesystem_redraw = unseen_filesystem_redraw(
        app.unseen_filesystem_redraw,
        focused,
        filesystem_responses.has_queued_frame(),
    );
    app.viewport_rows = app.viewport_rows.min(render_rows.max(1));
    app.ensure_visible();
    let start = app.offset.min(app.rows.len());
    let end = start.saturating_add(render_rows).min(app.rows.len());
    let notes_to_load: Vec<_> = app.rows[start..end]
        .iter()
        .map(|row| row.id)
        .filter(|id| !app.notes_loaded(*id))
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
            .is_none_or(|(marker, _)| *marker == usize::MAX);
    let worktree_selection = worktree_changes_to_load
        .then(|| {
            remembered_change_selection(
                &app.worktree_changes,
                worktree_changes.as_ref().map(|(_, changes)| changes),
            )
        })
        .flatten();
    if !app.show_commit || selected.is_none() {
        *commit_message = None;
    }
    if app.changes_mode.is_none() {
        tree_changes.clear();
        *worktree_changes = None;
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
        || app.rows[start..end].iter().any(|row| !row.metadata_loaded)
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
            let mut notes = repository
                .notes()
                .map_err(gix::Exn::into_error)
                .context("could not open Git notes")?;
            for id in notes_to_load {
                let loaded = notes
                    .get(id)
                    .map_err(gix::Exn::into_error)
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
        for index in start..end {
            if app.rows[index].metadata_loaded {
                continue;
            }
            let (metadata, attributions) = history::load_metadata(repository, app.rows[index].id, authors)
                .context("could not load visible commit")?;
            app.set_metadata(index, metadata, attributions);
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
            let loaded = load_worktree_changes(
                repository,
                line_diff_pool
                    .as_mut()
                    .context("line diff pool is missing while the changes pane is visible")?,
            );
            repository.object_cache_size(None);
            match loaded {
                Ok(loaded) => {
                    tracing::debug!(
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
                    restore_change_selection(&mut app.worktree_changes, &loaded, worktree_selection);
                    *worktree_changes = Some((0, loaded));
                }
                Err(err) => {
                    tracing::warn!(error = %err, "could not load worktree changes");
                    app.worktree_changes.error = Some(format!("status: {err:#}"));
                    if let Some((marker, _)) = worktree_changes.as_mut() {
                        *marker = 0;
                    } else {
                        *worktree_changes = Some((0, Changes::default()));
                    }
                }
            }
        }
    }
    let message = commit_message.as_ref().map(|(_, message)| message.as_bstr());
    let tree_changes = tree_changes.as_ref().map(|(_, changes)| changes);
    let worktree_changes = worktree_changes.as_ref().map(|(_, changes)| changes);
    terminal
        .autoresize()
        .context("could not resize the terminal before drawing")?;
    let layout = {
        let mut frame = terminal.get_frame();
        ui::draw_with_worktree(
            &mut frame,
            app,
            decorations,
            mailmap,
            message,
            tree_changes,
            worktree_changes,
        )
    };
    let target = animation::Snapshot::new(terminal.current_buffer_mut().clone(), layout);
    let ready = matches!(app.state, State::Complete | State::Cancelled);
    let presented = if motion.has_pending() && ready {
        let ids = motion.transition_ids(&target);
        let trees = load_transition_trees(&fill_repository.path, fill_repository.bare, &ids);
        let started = motion.begin(target.clone(), &trees, Instant::now());
        if started.is_some() {
            filesystem_responses.emphasis_started();
        }
        started.unwrap_or_else(|| target.buffer.clone())
    } else {
        filesystem_responses.emphasis_finished("history-emphasis-superseded", "superseded");
        let frame = motion.show(target);
        if ready {
            let response_ids = filesystem_responses.active_reference_ids().to_vec();
            filesystem_responses.finish_after_frame(&response_ids, "completed");
        }
        frame
    };
    terminal.current_buffer_mut().clone_from(&presented);
    terminal
        .apply_buffer_with_cursor(None)
        .context("could not draw terminal frame")?;
    filesystem_responses.frame_presented();
    Ok(())
}

fn load_transition_trees(
    repository_path: &Path,
    bare: bool,
    ids: &[gix::ObjectId],
) -> HashMap<gix::ObjectId, gix::ObjectId> {
    if ids.is_empty() {
        return HashMap::new();
    }
    let loaded = (|| -> Result<_> {
        let mut repository = open_repository(repository_path, bare, true)?;
        repository.object_cache_size(None);
        let cache = repository
            .commit_graph_if_enabled()
            .context("could not open commit-graph for transition matching")?;
        let mut buf = Vec::new();
        let mut trees = HashMap::with_capacity(ids.len());
        for id in ids {
            let commit = match gix::traverse::commit::find(cache.as_ref(), &repository.objects, id, &mut buf) {
                Ok(commit) => commit,
                Err(err) => {
                    tracing::debug!(%id, error = %err, "could not load transition commit");
                    continue;
                }
            };
            match commit.tree_id() {
                Ok(tree) => {
                    trees.insert(*id, tree);
                }
                Err(err) => tracing::debug!(%id, error = %err, "could not decode transition commit tree"),
            }
        }
        Ok(trees)
    })();
    match loaded {
        Ok(trees) => trees,
        Err(err) => {
            tracing::warn!(error = %err, "transition tree matching unavailable");
            HashMap::new()
        }
    }
}

fn present_buffer(terminal: &mut ratatui::DefaultTerminal, buffer: &ratatui::buffer::Buffer) -> Result<bool> {
    if terminal.get_frame().area() != buffer.area {
        return Ok(false);
    }
    terminal.current_buffer_mut().clone_from(buffer);
    terminal
        .apply_buffer_with_cursor(None)
        .context("could not draw animation frame")?;
    Ok(true)
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
fn reword_commit(
    terminal: &mut ratatui::DefaultTerminal,
    repository_path: &Path,
    bare: bool,
    graph: &HistoryGraph,
    id: gix::ObjectId,
    enhanced_keyboard: bool,
) -> Result<Option<gix::ObjectId>> {
    let (editor, document) = {
        let mut repository =
            open_repository(repository_path, bare, false).context("could not open repository before editing commit")?;
        repository.object_cache_size(None);
        edit::reword::document(&repository, id)?
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

    let mut repository =
        open_repository(repository_path, bare, false).context("could not reopen repository after editing commit")?;
    repository.object_cache_size(None);
    edit::reword::apply(repository, graph, id, &edited)
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
    anchor_title: String,
    commits: Vec<edit::todo::Commit>,
    head: Option<gix::ObjectId>,
    enhanced_keyboard: bool,
) -> Result<Option<edit::rebase::Perform>> {
    let prepared = {
        let mut repository =
            open_repository(repository_path, bare, false).context("could not open repository before rebasing")?;
        repository.object_cache_size(None);
        edit::todo::prepare(&repository, base, onto, &anchor_title, &commits, head)?
    };
    let edited = edit::edit_document(
        terminal,
        &prepared.editor,
        &prepared.document,
        &format!("tix-rebase-{}.md", std::process::id()),
        enhanced_keyboard,
    )?;
    let edited = match edited {
        Some(edited) => edited,
        None if prepared.has_pending => prepared.document.clone(),
        None => return Ok(None),
    };
    let mut repository =
        open_repository(repository_path, bare, false).context("could not reopen repository after editing rebase")?;
    repository.object_cache_size(None);
    let plan = edit::todo::parse(&repository, prepared, &edited)?;
    edit::rebase::perform_plan(&repository, graph, plan).map(Some)
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
) -> Result<Option<gix::ObjectId>> {
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
    let mut repository =
        open_repository(repository_path, bare, false).context("could not reopen repository after editing commit")?;
    repository.object_cache_size(None);
    match mode {
        CreateMode::Insert | CreateMode::InsertEmpty => edit::create::apply(repository, graph, prepared, &edited),
        CreateMode::Fork => edit::create::apply_fork(repository, graph, prepared, &edited),
    }
    .map(Some)
}

#[tracing::instrument(skip_all)]
fn split_commit(
    terminal: &mut ratatui::DefaultTerminal,
    repository_path: &Path,
    bare: bool,
    graph: &HistoryGraph,
    enhanced_keyboard: bool,
) -> Result<Option<gix::ObjectId>> {
    let mut repository =
        open_repository(repository_path, bare, false).context("could not open repository before splitting HEAD")?;
    repository.object_cache_size(None);
    let prepared = edit::split::prepare(repository)?;
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
    let mut repository =
        open_repository(repository_path, bare, false).context("could not reopen repository after editing split")?;
    repository.object_cache_size(None);
    edit::split::apply(repository, graph, prepared, &edited).map(Some)
}

#[tracing::instrument(skip_all, fields(commit_id = %id))]
fn forget_commit(
    repository_path: &Path,
    bare: bool,
    graph: &HistoryGraph,
    id: gix::ObjectId,
) -> Result<Option<gix::ObjectId>> {
    let mut repository =
        open_repository(repository_path, bare, false).context("could not open repository before forgetting commit")?;
    repository.object_cache_size(None);
    edit::forget::perform(repository, graph, id)
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
    let parents: Vec<_> = commit.parent_ids().collect();
    let parent_index = requested_parent.checked_rem(parents.len()).unwrap_or_default();
    let parent = parents.get(parent_index).copied();
    let new_tree = commit.tree().context("could not load changed commit tree")?;
    let old_tree = match parent {
        Some(parent) => Some(
            parent
                .object()
                .context("could not load parent commit")?
                .try_into_commit()
                .context("parent is not a commit")?
                .tree()
                .context("could not load parent commit tree")?,
        ),
        None => None,
    };
    load_tree_changes_without_lines(
        repository,
        old_tree.as_ref(),
        &new_tree,
        (parents.len() > 1).then(|| ComparedParent {
            index: parent_index,
            total: parents.len(),
            id: parent.expect("a merge has parents").detach(),
        }),
    )
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

fn load_worktree_changes(repository: &gix::Repository, line_diff_pool: &mut LineDiffPool) -> Result<Changes> {
    let mut out = load_worktree_changes_without_lines(repository)?;
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
    action_with_shortcut_groups(key, false, false)
}

fn action_with_shortcut_groups(key: KeyEvent, history_display_expanded: bool, edit_expanded: bool) -> Option<Action> {
    if key.kind == KeyEventKind::Release
        && !matches!(
            key.code,
            KeyCode::Modifier(ModifierKeyCode::LeftShift | ModifierKeyCode::RightShift)
        )
    {
        return None;
    }
    match key.code {
        KeyCode::Modifier(ModifierKeyCode::LeftShift | ModifierKeyCode::RightShift) => {
            Some(Action::PreviewAuthorCopy(key.kind != KeyEventKind::Release))
        }
        KeyCode::Tab => Some(Action::ToggleChangesFocus),
        KeyCode::Enter => Some(Action::OpenDiff),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::ForceQuit),
        KeyCode::Char('c') => Some(Action::ToggleChanges),
        KeyCode::Char('p') if edit_expanded => Some(Action::Split),
        KeyCode::Char('b') if edit_expanded && !key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::Rebase),
        KeyCode::Char('u') if edit_expanded && !key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::RebaseUpdate)
        }
        KeyCode::Char('f') if edit_expanded && !key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::ForkCommit)
        }
        KeyCode::Char('p') => Some(Action::CycleChangesParent),
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Esc => Some(Action::Cancel),
        KeyCode::Up | KeyCode::Char('k') => Some(Action::MoveUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::MoveDown),
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
        KeyCode::Home | KeyCode::Char('g') => Some(Action::First),
        KeyCode::End | KeyCode::Char('G') => Some(Action::Last),
        KeyCode::Char('d') if history_display_expanded => Some(Action::ToggleDate),
        KeyCode::Char('e') if history_display_expanded => Some(Action::ToggleEmail),
        KeyCode::Char('n') if history_display_expanded => Some(Action::ToggleName),
        KeyCode::Char('t') if history_display_expanded => Some(Action::ToggleTrailers),
        KeyCode::Char('m') if history_display_expanded => Some(Action::ToggleMailmap),
        KeyCode::Char('R') => Some(Action::Refresh),
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::SHIFT) => Some(Action::Refresh),
        KeyCode::Char('r') if history_display_expanded => Some(Action::CycleRefs),
        KeyCode::Char('r') if edit_expanded => Some(Action::Reword),
        KeyCode::Char('n') if edit_expanded => Some(Action::NewCommit),
        KeyCode::Char('m') if edit_expanded => Some(Action::NewEmptyCommit),
        KeyCode::Char('a') if edit_expanded => Some(Action::Amend),
        KeyCode::Char('s') if edit_expanded => Some(Action::Spill),
        KeyCode::Char('d') if edit_expanded => Some(Action::Forget),
        KeyCode::Char('t') if edit_expanded => Some(Action::TimeTravel),
        KeyCode::Char('v') if edit_expanded => Some(Action::Review),
        KeyCode::Char('m') => Some(Action::ToggleCommit),
        KeyCode::Char('r') => Some(Action::ToggleRefs),
        KeyCode::Char('s') => Some(Action::VerifySignatures),
        KeyCode::Char('v') => Some(Action::ToggleHistoryDisplay),
        KeyCode::Char('e') => Some(Action::ToggleEdit),
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

fn mouse_scroll_action(kind: MouseEventKind, distance: usize) -> Option<Action> {
    match kind {
        MouseEventKind::ScrollUp => Some(Action::MoveUpBy(distance.max(1))),
        MouseEventKind::ScrollDown => Some(Action::MoveDownBy(distance.max(1))),
        MouseEventKind::ScrollLeft => Some(Action::ScrollLeft),
        MouseEventKind::ScrollRight => Some(Action::ScrollRight),
        _ => None,
    }
}

#[cfg(test)]
fn open_test_repository(path: impl AsRef<Path>) -> Result<gix::Repository, gix::open::Error> {
    gix::open_opts(path.as_ref(), gix::open::Options::isolated())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shades_terminal_background_by_one_sixteenth() {
        assert_eq!(shade_terminal_background((0, 0, 0), true), (15, 15, 15));
        assert_eq!(shade_terminal_background((255, 255, 255), false), (240, 240, 240));
        assert_eq!(shade_terminal_background((32, 64, 128), true), (45, 75, 135));
        assert_eq!(shade_terminal_background((32, 64, 128), false), (30, 60, 120));
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
    fn reference_watcher_observes_new_loose_refs() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = open_test_repository(fixture.path())?;
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
        app.set_changes_bounds(ChangePane::Tree, 2, 2, 80, 0);
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
        let repository = open_test_repository(&fixture)?;
        let id = repository.rev_parse_single("topic")?.detach();

        assert!(
            load_commit_message(&repository, id)?.starts_with(b"topic\n\n--- agent\n\nCo-authored-by:"),
            "on-demand loading retains the full commit message"
        );
        Ok(())
    }

    #[test]
    fn selection_relation_prefers_tracking_counts_and_handles_missing_upstreams() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_read_only("history.sh")?;
        let repository = open_test_repository(&fixture)?;
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
        let repository = open_test_repository(path)?;
        let topic = repository.rev_parse_single("topic")?.detach();
        let main = repository.rev_parse_single("main")?.detach();
        let status = std::process::Command::new("git")
            .current_dir(path)
            .args(["update-ref", "refs/remotes/origin/main", &main.to_hex().to_string()])
            .status()?;
        assert!(status.success(), "the configured tracking ref exists");
        let repository = open_test_repository(path)?;
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
    fn loads_changes_against_each_merge_parent() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_read_only("history.sh")?;
        let repository = gix::open_opts(&fixture, gix::open::Options::isolated())?;
        let mut line_diff_pool = None;
        sync_line_diff_pool(&mut line_diff_pool, true, &fixture, false, 2)?;
        assert_eq!(
            line_diff_pool.as_ref().map(|pool| pool.workers.len()),
            Some(2),
            "showing changes creates the requested worker pool"
        );
        sync_line_diff_pool(&mut line_diff_pool, false, &fixture, false, 2)?;
        assert!(line_diff_pool.is_none(), "hiding changes destroys the worker pool");
        sync_line_diff_pool(&mut line_diff_pool, true, &fixture, false, 2)?;
        let line_diff_pool = line_diff_pool
            .as_mut()
            .expect("showing changes recreates the worker pool");

        let root = load_changes(
            &repository,
            app::TreeDiffTarget::Commit {
                id: repository.rev_parse_single("v1^{}")?.detach(),
                parent: 0,
            },
            line_diff_pool,
        )?;
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

        let external_repository = gix::open_opts(
            &fixture,
            gix::open::Options::isolated().config_overrides(["diff.external=test --flag"]),
        )?;
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

        let pager_repository = gix::open_opts(
            &fixture,
            gix::open::Options::isolated().config_overrides(["core.pager=delta --dark"]),
        )?;
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
            let repository = gix::open_opts(&fixture, gix::open::Options::isolated().config_overrides([setting]))?;
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
        Ok(())
    }

    #[test]
    fn configures_a_common_repository_as_bare_for_tree_changes() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_read_only("history.sh")?;
        let git_dir = open_test_repository(&fixture)?.git_dir().to_owned();
        let repository = open_repository(&git_dir, true, false)?;

        assert_eq!(
            repository.config_snapshot().boolean("core.bare"),
            Some(true),
            "repository configuration suppresses worktree operations"
        );
        assert!(
            LineDiffPool::new(&git_dir, true, 1).is_ok(),
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
        let git_dir = open_test_repository(&fixture)?.git_dir().to_owned();
        let indirect = git_dir.join("worktrees/missing/../..");
        assert!(
            !git_dir.join("worktrees/missing").exists(),
            "the intermediate path is absent"
        );
        assert_eq!(normalize_common_dir(indirect)?, git_dir);
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
        let git_dir = open_test_repository(&fixture)?.git_dir().canonicalize()?;
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

        let repository = open_test_repository(path)?;
        let mut line_diff_pool = LineDiffPool::new(path, false, 2)?;
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
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE)),
            Some(Action::ToggleEdit)
        );
        assert_eq!(action(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE)), None);
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE)),
            Some(Action::ToggleCommit)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE)),
            Some(Action::ToggleRefs)
        );
        assert_eq!(action(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE)), None);
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
        for (key, expected) in [
            ('d', Action::ToggleDate),
            ('e', Action::ToggleEmail),
            ('n', Action::ToggleName),
            ('t', Action::ToggleTrailers),
            ('m', Action::ToggleMailmap),
            ('r', Action::CycleRefs),
            ('h', Action::ToggleHidden),
        ] {
            assert_eq!(
                action_with_shortcut_groups(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE), true, false),
                Some(expected),
                "{key} is available after the view prefix"
            );
        }
        assert_eq!(
            action_with_shortcut_groups(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE), true, false),
            Some(Action::ToggleHistoryDisplay),
            "v closes the view shortcut group"
        );
        for (key, expected) in [
            ('b', Action::Rebase),
            ('u', Action::RebaseUpdate),
            ('r', Action::Reword),
            ('n', Action::NewCommit),
            ('m', Action::NewEmptyCommit),
            ('f', Action::ForkCommit),
            ('a', Action::Amend),
            ('s', Action::Spill),
            ('p', Action::Split),
            ('d', Action::Forget),
            ('t', Action::TimeTravel),
        ] {
            assert_eq!(
                action_with_shortcut_groups(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE), false, true),
                Some(expected),
                "{key} is available after the edit prefix"
            );
        }
        assert_eq!(
            action_with_shortcut_groups(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL), false, true,),
            Some(Action::PageUp),
            "navigation keeps priority over the edit shortcut"
        );
        assert_eq!(
            action_with_shortcut_groups(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE), false, true),
            Some(Action::ToggleEdit),
            "e closes the edit shortcut group"
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE)),
            Some(Action::VerifySignatures)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE)),
            Some(Action::ToggleAlign)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE)),
            Some(Action::ToggleCommit)
        );
        assert_eq!(action(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE)), None);
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE)),
            Some(Action::CycleChangesParent)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE)),
            Some(Action::ToggleChanges)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('Y'), KeyModifiers::SHIFT)),
            Some(Action::CopyAuthor)
        );
        assert_eq!(
            action(KeyEvent::new_with_kind(
                KeyCode::Modifier(crossterm::event::ModifierKeyCode::LeftShift),
                KeyModifiers::SHIFT,
                KeyEventKind::Press,
            )),
            Some(Action::PreviewAuthorCopy(true))
        );
        assert_eq!(
            action(KeyEvent::new_with_kind(
                KeyCode::Modifier(crossterm::event::ModifierKeyCode::LeftShift),
                KeyModifiers::NONE,
                KeyEventKind::Release,
            )),
            Some(Action::PreviewAuthorCopy(false))
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(Action::ForceQuit)
        );
        assert_eq!(action(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)), None);
    }

    #[test]
    fn retains_the_fill_repository_only_for_repeated_viewport_navigation() {
        assert!(retains_fill_repository(
            KeyEventKind::Repeat,
            Some(&Action::MoveDown),
            false
        ));
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
    fn key_releases_do_not_cancel_suspended_operations() {
        let key = |kind| TerminalEvent::Key(KeyEvent::new_with_kind(KeyCode::Char('t'), KeyModifiers::NONE, kind));
        assert!(is_key_press(&key(KeyEventKind::Press)));
        assert!(is_key_press(&key(KeyEventKind::Repeat)));
        assert!(!is_key_press(&key(KeyEventKind::Release)));
    }

    #[test]
    fn maps_continuous_mouse_scrolling_to_navigation() {
        assert_eq!(
            mouse_scroll_action(MouseEventKind::ScrollUp, 4),
            Some(Action::MoveUpBy(4))
        );
        assert_eq!(
            mouse_scroll_action(MouseEventKind::ScrollDown, 3),
            Some(Action::MoveDownBy(3))
        );
        assert_eq!(
            mouse_scroll_action(MouseEventKind::ScrollLeft, 1),
            Some(Action::ScrollLeft)
        );
        assert_eq!(
            mouse_scroll_action(MouseEventKind::ScrollRight, 1),
            Some(Action::ScrollRight)
        );
        assert_eq!(mouse_scroll_action(MouseEventKind::Moved, 1), None);
        assert!(repeats_viewport(
            &mouse_scroll_action(MouseEventKind::ScrollDown, 2).expect("vertical scrolling has an action")
        ));
        assert!(!repeats_viewport(
            &mouse_scroll_action(MouseEventKind::ScrollRight, 1).expect("horizontal scrolling has an action")
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
        assert!(reference_event_is_relevant(
            &modified(&current_linked.join("index")),
            &current_linked,
            &worktrees
        ));
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
        assert!(!worktree_watch_set_may_change(
            &modified(&workdir.join("src/lib.rs")),
            &index,
            &directories
        ));
        assert!(worktree_watch_set_may_change(&modified(&index), &index, &directories));
        assert!(worktree_watch_set_may_change(
            &modified(&workdir.join(".gitignore")),
            &index,
            &directories
        ));
        let create_directory =
            notify::Event::new(notify::EventKind::Create(CreateKind::Folder)).add_path(workdir.join("new"));
        assert!(worktree_watch_set_may_change(&create_directory, &index, &directories));
        let remove_directory =
            notify::Event::new(notify::EventKind::Remove(RemoveKind::Folder)).add_path(workdir.join("src"));
        assert!(worktree_watch_set_may_change(&remove_directory, &index, &directories));
        assert!(worktree_watch_set_may_change(&rescan, &index, &directories));

        let mut changes = Some((0, Changes::default()));
        assert!(invalidate_worktree_changes(&mut changes));
        assert_eq!(changes.as_ref().map(|(marker, _)| *marker), Some(usize::MAX));
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

        let repository = open_test_repository(root)?;
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
        assert!(schedule_once(&mut deadline, now, WORKTREE_EVENT_IDLE));
        let first = deadline;
        assert!(!schedule_once(
            &mut deadline,
            now + Duration::from_millis(50),
            WORKTREE_EVENT_IDLE
        ));
        assert_eq!(deadline, first, "later events do not extend the debounce window");
        assert!(!take_due(&mut deadline, now + Duration::from_millis(74)));
        assert!(take_due(&mut deadline, now + WORKTREE_EVENT_IDLE));
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
}
