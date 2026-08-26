//! Worktree discovery and the asynchronously loaded information shown by the worktrunk picker.

pub(crate) mod remove;
pub(crate) mod shell;

use std::{
    ffi::{OsStr, OsString},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
};

use anyhow::{Context, Result};
use gix::bstr::{BString, ByteSlice};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
};

use crate::menu::{Item as MenuItem, Menu};

/// Information that takes no repository traversal to obtain.
#[derive(Debug)]
pub(crate) struct Row {
    /// The absolute path of the worktree.
    pub(crate) path: PathBuf,
    /// A compact label derived from [`path`](Self::path).
    pub(crate) label: String,
    /// Whether this is the worktree from which Tix was launched.
    pub(crate) is_current: bool,
    /// Whether this is the repository's main worktree.
    pub(crate) is_main: bool,
    /// Whether Git currently considers this worktree locked.
    pub(crate) locked: bool,
    /// Whether the row is still loading, ready, or failed to load.
    pub(crate) state: LoadState,
    /// The logical Tix head, once loaded.
    pub(crate) head: Option<LogicalHead>,
    /// Whether the index or worktree differs from `HEAD`, once loaded.
    pub(crate) dirty: Option<bool>,
    /// Ahead/behind counts relative to the configured upstream or the Tix base.
    pub(crate) relation: Option<Relation>,
    /// Lines added compared to the unambiguous Tix base, if there is one.
    pub(crate) lines_added: Option<u64>,
    /// Lines removed compared to the unambiguous Tix base, if there is one.
    pub(crate) lines_removed: Option<u64>,
}

impl Row {
    pub(crate) fn removal_blocker(&self) -> Option<&'static str> {
        if self.is_current {
            Some("the worktree from which tix was launched cannot be removed")
        } else if self.is_main {
            Some("the main worktree cannot be removed")
        } else if self.locked {
            Some("locked worktrees require `tix wt remove -ff`")
        } else {
            None
        }
    }
}

/// Loading state of a [`Row`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum LoadState {
    Loading,
    Ready,
    Error(String),
}

/// The branch and commit Tix treats as the worktree's head.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LogicalHead {
    /// A local branch, including `refs/heads/`, when one is attached or remembered by Tix.
    pub(crate) branch: Option<gix::refs::FullName>,
    /// The branch tip or physical detached commit. This is absent for an unborn branch.
    pub(crate) commit_id: Option<gix::ObjectId>,
    /// Whether Git's physical `HEAD` is detached.
    pub(crate) is_detached: bool,
}

/// Commit counts on each side of the comparison point.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Relation {
    pub(crate) ahead: usize,
    pub(crate) behind: usize,
}

#[derive(Debug)]
struct Update {
    index: usize,
    result: Result<bool, String>,
}

/// History-derived information for one worktree row.
#[derive(Debug)]
pub(crate) struct GraphMetadata {
    head: LogicalHead,
    relation: Option<Relation>,
    diffstat: Option<(u64, u64)>,
}

/// Stable worktree rows whose expensive fields are filled by one short-lived worker per row.
pub(crate) struct Worktrees {
    rows: Vec<Row>,
    selected: usize,
    previewed: Option<usize>,
    previewing: bool,
    search_origin: Option<usize>,
    search: Menu<usize>,
    updates: mpsc::Receiver<Update>,
    cancel: Arc<AtomicBool>,
    workers: Vec<thread::JoinHandle<()>>,
    workers_suspended: bool,
}

impl Worktrees {
    /// Inventory worktrees; the first update drain begins loading rows in parallel.
    pub(crate) fn start(repository: &gix::Repository) -> Result<Self> {
        let (_sender, updates) = mpsc::channel();
        Ok(Worktrees {
            rows: inventory(repository)?,
            selected: 0,
            previewed: Some(0),
            previewing: false,
            search_origin: None,
            search: Menu::default(),
            updates,
            cancel: Arc::new(AtomicBool::new(false)),
            workers: Vec::new(),
            workers_suspended: false,
        })
    }

    pub(crate) fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub(crate) fn selected(&self) -> Option<&Row> {
        self.selected_index().and_then(|index| self.rows.get(index))
    }

    pub(crate) fn selected_index(&self) -> Option<usize> {
        if self.search.is_open() {
            self.search.selected_index()
        } else {
            (!self.rows.is_empty()).then_some(self.selected)
        }
    }

    pub(crate) fn selected_path(&self) -> Option<&Path> {
        self.selected().map(|row| row.path.as_path())
    }

    pub(crate) fn select(&mut self, index: usize) {
        if !self.rows.is_empty() {
            self.selected = index.min(self.rows.len() - 1);
        }
    }

    pub(crate) fn preview_pending(&self) -> bool {
        self.previewing || self.selected_index() != self.previewed
    }

    pub(crate) fn begin_preview(&mut self) {
        self.previewing = true;
    }

    pub(crate) fn cancel_preview(&mut self) {
        self.previewing = false;
    }

    pub(crate) fn mark_previewed(&mut self, index: usize) {
        self.previewed = Some(index);
        self.previewing = false;
    }

    pub(crate) fn set_graph_metadata(&mut self, index: usize, result: Result<GraphMetadata, String>) {
        let Some(row) = self.rows.get_mut(index) else {
            return;
        };
        match result {
            Ok(GraphMetadata {
                head,
                relation,
                diffstat,
            }) => {
                row.head = Some(head);
                row.relation = relation;
                (row.lines_added, row.lines_removed) = diffstat
                    .map(|(added, removed)| (Some(added), Some(removed)))
                    .unwrap_or_default();
                if row.dirty.is_some() && !matches!(row.state, LoadState::Error(_)) {
                    row.state = LoadState::Ready;
                }
            }
            Err(err) => row.state = LoadState::Error(err),
        }
    }

    pub(crate) fn invalidate_graph_metadata(&mut self) {
        for row in &mut self.rows {
            let dirty_error = row.head.is_some() && row.dirty.is_none() && matches!(row.state, LoadState::Error(_));
            if !dirty_error {
                row.state = LoadState::Loading;
            }
            row.head = None;
            row.relation = None;
            row.lines_added = None;
            row.lines_removed = None;
        }
    }

    pub(crate) fn search_is_open(&self) -> bool {
        self.search.is_open()
    }

    pub(crate) fn search_query(&self) -> &str {
        self.search.query()
    }

    pub(crate) fn search_cursor(&self) -> usize {
        self.search.cursor()
    }

    pub(crate) fn open_search(&mut self) {
        self.search_origin = Some(self.selected);
        let items = menu_items(&self.rows);
        self.search.open_selected(&items, Some(&self.selected));
    }

    pub(crate) fn cancel_search(&mut self) -> Option<PathBuf> {
        let origin = self.search_origin.take()?;
        self.search.close();
        if self.selected == origin {
            return None;
        }
        self.selected = origin;
        self.selected_path().map(ToOwned::to_owned)
    }

    pub(crate) fn cancel_search_needs_rebind(&self) -> bool {
        self.search_origin.is_some_and(|origin| origin != self.selected)
    }

    pub(crate) fn submit_search(&mut self) -> Option<PathBuf> {
        let items = menu_items(&self.rows);
        let selected = self.search.submit_selected(&items)?;
        self.search_origin = None;
        self.selected = selected;
        self.selected_path().map(ToOwned::to_owned)
    }

    pub(crate) fn edit_search(&mut self, input: SearchInput) {
        let items = menu_items(&self.rows);
        let had_query = !self.search.query().is_empty();
        match input {
            SearchInput::Insert(ch) => self.search.insert(ch, &items),
            SearchInput::Paste(text) => self.search.paste(&text, &items),
            SearchInput::Left => self.search.left(),
            SearchInput::Right => self.search.right(),
            SearchInput::Home => self.search.home(),
            SearchInput::End => self.search.end(),
            SearchInput::Backspace => self.search.backspace(&items),
            SearchInput::Delete => self.search.delete(&items),
            SearchInput::Up(amount) => self.search.up_by(amount, &items),
            SearchInput::Down(amount) => self.search.down_by(amount, &items),
        }
        if had_query && self.search.query().is_empty() {
            self.search
                .open_selected(&items, self.search_origin.as_ref().or(Some(&self.selected)));
        }
    }

    pub(crate) fn preview_search_selection(&mut self) -> Option<PathBuf> {
        let selected = self.search.selected_index()?;
        if selected == self.selected {
            return None;
        }
        self.selected = selected;
        self.rows.get(selected).map(|row| row.path.clone())
    }

    pub(crate) fn search_selection_needs_preview(&self) -> bool {
        self.search
            .selected_index()
            .is_some_and(|selected| selected != self.selected)
    }

    pub(crate) fn display_row_count(&self) -> usize {
        if self.search.is_open() {
            self.search.matching_indices().len().max(1)
        } else {
            self.rows.len()
        }
    }

    pub(crate) fn is_loading(&self) -> bool {
        self.rows.iter().any(|row| matches!(row.state, LoadState::Loading))
    }

    fn visible_indices(&self, visible: usize) -> Vec<usize> {
        let indices: Vec<_> = if self.search.is_open() {
            self.search.matching_indices().to_vec()
        } else {
            (0..self.rows.len()).collect()
        };
        let selected = if self.search.is_open() {
            self.search.selected_match().unwrap_or_default()
        } else {
            self.selected
        };
        let start = selected
            .saturating_sub(visible / 2)
            .min(indices.len().saturating_sub(visible));
        indices.into_iter().skip(start).take(visible).collect()
    }

    /// Apply all currently available worker messages without blocking.
    pub(crate) fn drain_updates(&mut self) -> bool {
        if !self.workers_suspended && self.workers.is_empty() && self.rows.iter().any(|row| row.dirty.is_none()) {
            self.start_workers();
        }
        let updates: Vec<_> = self.updates.try_iter().collect();
        let changed = !updates.is_empty();
        for update in updates {
            self.apply_update(update);
        }
        changed
    }

    fn finish_workers(&mut self) -> Result<()> {
        let mut panicked = false;
        for worker in std::mem::take(&mut self.workers) {
            panicked |= worker.join().is_err();
        }
        anyhow::ensure!(!panicked, "a worktree status worker panicked");
        for update in self.updates.try_iter().collect::<Vec<_>>() {
            self.apply_update(update);
        }
        Ok(())
    }

    fn apply_update(&mut self, update: Update) {
        let Some(row) = self.rows.get_mut(update.index) else {
            return;
        };
        match update.result {
            Ok(dirty) => {
                row.dirty = Some(dirty);
                if row.head.is_some() && !matches!(row.state, LoadState::Error(_)) {
                    row.state = LoadState::Ready;
                }
            }
            Err(err) => row.state = LoadState::Error(err),
        }
    }

    /// Reload the expensive fields while preserving row order and selection.
    pub(crate) fn refresh(&mut self) {
        self.cancel_and_join_workers();
        self.workers_suspended = false;
        self.previewed = None;
        self.previewing = false;
        self.invalidate_graph_metadata();
        for row in &mut self.rows {
            row.state = LoadState::Loading;
            row.dirty = None;
        }
    }

    /// Stop every indexed loader before its checkout can disappear.
    pub(crate) fn suspend_workers_for_removal(&mut self) {
        self.workers_suspended = true;
        self.cancel_and_join_workers();
    }

    /// Rebuild the inventory after a removal and return the selected survivor.
    pub(crate) fn reinventory_after_removal(&mut self, repository: &gix::Repository) -> Result<Option<PathBuf>> {
        let launch_path = self.rows.iter().find(|row| row.is_current).map(|row| row.path.clone());
        let selected_path = self.selected_path().map(ToOwned::to_owned);
        let selected = self.selected;

        self.cancel_and_join_workers();
        self.workers_suspended = false;
        self.rows = inventory(repository)?;
        if let Some(launch_path) = launch_path {
            for row in &mut self.rows {
                row.is_current = row.path == launch_path;
            }
            sort_rows(&mut self.rows);
        }
        self.selected = selected_path
            .and_then(|path| self.rows.iter().position(|row| row.path == path))
            .unwrap_or_else(|| selected.min(self.rows.len().saturating_sub(1)));
        self.previewed = None;
        self.previewing = false;
        self.search_origin = None;
        self.search.close();
        Ok(self.selected_path().map(ToOwned::to_owned))
    }

    fn start_workers(&mut self) {
        self.cancel = Arc::new(AtomicBool::new(false));
        let (sender, receiver) = mpsc::channel();
        self.updates = receiver;
        // ponytail: four fixed lanes bound history-walk memory; use a queue if load balancing becomes measurable.
        const MAX_WORKERS: usize = 4;
        for lane in 0..self.rows.len().min(MAX_WORKERS) {
            let rows = self
                .rows
                .iter()
                .enumerate()
                .skip(lane)
                .step_by(MAX_WORKERS)
                .map(|(index, row)| (index, row.path.clone()))
                .collect::<Vec<_>>();
            let sender = sender.clone();
            let cancel = Arc::clone(&self.cancel);
            self.workers.push(thread::spawn(move || {
                for (index, path) in rows {
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    let result = gix::open(&path)
                        .with_context(|| format!("could not open worktree {}", path.display()))
                        .and_then(|repository| is_dirty(&repository, &cancel))
                        .map_err(|err| format!("{err:#}"));
                    if cancel.load(Ordering::Relaxed) || sender.send(Update { index, result }).is_err() {
                        break;
                    }
                }
            }));
        }
    }

    fn cancel_and_join_workers(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
        let (_sender, updates) = mpsc::channel();
        self.updates = updates;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SearchInput {
    Insert(char),
    Paste(String),
    Left,
    Right,
    Home,
    End,
    Backspace,
    Delete,
    Up(usize),
    Down(usize),
}

fn menu_items(rows: &[Row]) -> Vec<MenuItem<'_, usize>> {
    rows.iter()
        .enumerate()
        .map(|(index, row)| MenuItem::new(&row.label, index))
        .collect()
}

impl Drop for Worktrees {
    fn drop(&mut self) {
        self.cancel_and_join_workers();
    }
}

fn inventory(repository: &gix::Repository) -> Result<Vec<Row>> {
    let current = repository
        .worktree()
        .map(|worktree| worktree.id().map(ToOwned::to_owned));
    let mut entries = Vec::new();
    let main = repository.main_repo().context("could not open the main repository")?;
    if let Some(path) = main.workdir() {
        let path = absolute(path)?;
        if path.is_dir() {
            entries.push(row(path, current == Some(None), true, false));
        }
    }
    for proxy in repository.worktrees().context("could not list linked worktrees")? {
        let id = proxy.id().to_owned();
        let locked = proxy.is_locked();
        let path = absolute(&proxy.base().context("could not read a linked worktree path")?)?;
        if !path.is_dir() {
            continue;
        }
        let is_current = current.as_ref().and_then(Option::as_ref) == Some(&id);
        entries.push(row(path, is_current, false, locked));
    }
    sort_rows(&mut entries);
    Ok(entries)
}

fn sort_rows(entries: &mut [Row]) {
    entries.sort_by(|a, b| {
        b.is_current
            .cmp(&a.is_current)
            .then_with(|| b.is_main.cmp(&a.is_main))
            .then_with(|| a.path.cmp(&b.path))
    });
}

fn row(path: PathBuf, is_current: bool, is_main: bool, locked: bool) -> Row {
    let label = path
        .file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned();
    Row {
        path,
        label,
        is_current,
        is_main,
        locked,
        state: LoadState::Loading,
        head: None,
        dirty: None,
        relation: None,
        lines_added: None,
        lines_removed: None,
    }
}

fn absolute(path: &Path) -> std::io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_owned())
    } else {
        std::path::absolute(path)
    }
}

pub(crate) fn graph_metadata(
    repository: &gix::Repository,
    graph: &crate::history::HistoryGraph,
    refs: &crate::history::RefSnapshot,
) -> Result<GraphMetadata> {
    let head = logical_head(repository)?;
    graph_metadata_for_head(repository, graph, &refs.hidden_tips, head)
}

fn graph_metadata_for_head(
    repository: &gix::Repository,
    graph: &crate::history::HistoryGraph,
    hidden_tips: &[gix::ObjectId],
    head: LogicalHead,
) -> Result<GraphMetadata> {
    let Some(head_id) = head.commit_id else {
        return Ok(GraphMetadata {
            head,
            relation: None,
            diffstat: None,
        });
    };
    let relation = relation(repository, graph, &head, hidden_tips)?;
    let diffstat = unique_tix_boundary(graph, head_id, hidden_tips)
        .map(|base_id| diffstat(repository, base_id, head_id))
        .transpose()?;
    Ok(GraphMetadata {
        head,
        relation,
        diffstat,
    })
}

fn logical_head(repository: &gix::Repository) -> Result<LogicalHead> {
    let mut head = repository.head().context("could not read HEAD")?;
    let is_detached = head.is_detached();
    let physical_id = head
        .try_peel_to_id()
        .context("could not resolve HEAD")?
        .map(gix::Id::detach);
    if !is_detached {
        return Ok(LogicalHead {
            branch: head.referent_name().map(ToOwned::to_owned),
            commit_id: physical_id,
            is_detached,
        });
    }
    let remembered = repository
        .try_find_reference(crate::history::HEAD_PIN_NAME.as_bstr())
        .context("could not read the Tix HEAD pin")?
        .and_then(|mut pin| {
            let name = pin.target().try_name()?.to_owned();
            (name.category() == Some(gix::refs::Category::LocalBranch))
                .then(|| pin.peel_to_id().ok().map(gix::Id::detach))
                .flatten()
                .filter(|id| {
                    repository
                        .find_header(*id)
                        .is_ok_and(|header| header.kind() == gix::object::Kind::Commit)
                })
                .map(|id| (name, id))
        });
    let (branch, commit_id) = remembered.map_or((None, physical_id), |(branch, id)| (Some(branch), Some(id)));
    Ok(LogicalHead {
        branch,
        commit_id,
        is_detached,
    })
}

fn is_dirty(repository: &gix::Repository, cancel: &AtomicBool) -> Result<bool> {
    anyhow::ensure!(!cancel.load(Ordering::Relaxed), "worktree loading was cancelled");
    let mut status = repository
        .status(gix::progress::Discard)
        .context("could not initialize worktree status")?
        .untracked_files(gix::status::UntrackedFiles::Files)
        .into_iter(Vec::<BString>::new())
        .context("could not start worktree status")?;
    let dirty = status
        .next()
        .transpose()
        .context("could not obtain worktree status")
        .map(|item| item.is_some())?;
    drop(status);
    anyhow::ensure!(!cancel.load(Ordering::Relaxed), "worktree loading was cancelled");
    Ok(dirty)
}

fn relation(
    repository: &gix::Repository,
    graph: &crate::history::HistoryGraph,
    head: &LogicalHead,
    hidden_tips: &[gix::ObjectId],
) -> Result<Option<Relation>> {
    let Some(head_id) = head.commit_id else {
        return Ok(None);
    };
    if let Some(branch) = head.branch.as_ref()
        && let Some(upstream) =
            repository.branch_remote_tracking_ref_name(branch.as_ref(), gix::remote::Direction::Fetch)
    {
        let upstream = upstream.context("could not resolve the configured upstream")?;
        let Some(mut upstream) = repository
            .try_find_reference(upstream.as_bstr())
            .with_context(|| format!("could not read configured upstream {upstream}"))?
        else {
            return Ok(None);
        };
        let upstream_id = upstream
            .peel_to_id()
            .context("could not resolve the configured upstream")?
            .detach();
        return Ok(graph
            .ahead_behind(head_id, &[upstream_id])
            .map(|(ahead, behind)| Relation { ahead, behind }));
    }
    Ok((hidden_tips.len() == 1)
        .then(|| graph.ahead_behind(head_id, hidden_tips))
        .flatten()
        .map(|(ahead, behind)| Relation { ahead, behind }))
}

fn unique_tix_boundary(
    graph: &crate::history::HistoryGraph,
    head_id: gix::ObjectId,
    hidden_tips: &[gix::ObjectId],
) -> Option<gix::ObjectId> {
    let (_, boundary) = crate::history::view_scope(&[head_id], hidden_tips, |id, out| {
        if let Some(parents) = graph.parents_of(id) {
            out.extend(parents);
        }
    });
    (boundary.len() == 1).then(|| *boundary.iter().next().expect("one boundary exists"))
}

fn diffstat(repository: &gix::Repository, base_id: gix::ObjectId, head_id: gix::ObjectId) -> Result<(u64, u64)> {
    let base = repository
        .find_commit(base_id)
        .context("could not read the Tix base commit")?
        .tree()
        .context("could not read the Tix base tree")?;
    let head = repository
        .find_commit(head_id)
        .context("could not read the logical head commit")?
        .tree()
        .context("could not read the logical head tree")?;
    let stats = base
        .changes()
        .context("could not initialize the Tix base diff")?
        .options(|options| {
            options.track_rewrites(None);
        })
        .stats(&head)
        .context("could not calculate the Tix base diffstat")?;
    Ok((stats.lines_added, stats.lines_removed))
}

/// Resolve a worktree or local branch, optionally creating the branch, then its worktree.
pub(crate) fn resolve_or_create<P>(
    repository: &gix::Repository,
    target: &OsStr,
    path_override: Option<&Path>,
    create_branch_if_missing: bool,
    progress: P,
    interrupt: &AtomicBool,
) -> Result<PathBuf>
where
    P: gix::progress::NestedProgress,
    P::SubProgress: gix::progress::NestedProgress + 'static,
{
    let rows = inventory(repository)?;
    if !create_branch_if_missing {
        let target_path = absolute(Path::new(target))?;
        if let Some(row) = rows.iter().find(|row| row.path == target_path) {
            return Ok(row.path.clone());
        }
    }

    let target = gix::path::os_str_into_bstr(target)
        .with_context(|| format!("branch name {} is not valid UTF-8", target.to_string_lossy()))?;
    let target_bytes: &gix::bstr::BStr = target;
    let branch: gix::refs::FullName = if target_bytes.starts_with(b"refs/heads/") {
        target_bytes
            .try_into()
            .context("target is not a valid local branch name")?
    } else {
        gix::refs::Category::LocalBranch
            .to_full_name(target_bytes)
            .context("target is not a valid local branch name")?
    };
    gix::validate::reference::branch_name(branch.as_bstr()).context("target is not a valid local branch name")?;
    let branch_exists = repository
        .try_find_reference(branch.as_bstr())
        .with_context(|| format!("could not read local branch {branch}"))?
        .is_some();
    if !branch_exists {
        anyhow::ensure!(
            create_branch_if_missing,
            "target is neither an existing worktree path nor a local branch: {target}"
        );
        let head_id = logical_head(repository)?
            .commit_id
            .context("cannot create a branch from an unborn logical HEAD")?;
        repository
            .reference(
                branch.clone(),
                head_id,
                gix::refs::transaction::PreviousValue::MustNotExist,
                "tix worktrunk: create branch",
            )
            .with_context(|| format!("could not create local branch {branch}"))?;
    }
    for row in &rows {
        let Ok(worktree) = gix::open(&row.path) else {
            continue;
        };
        if logical_head(&worktree)?.branch.as_ref() == Some(&branch) {
            return Ok(row.path.clone());
        }
    }

    let destination = match path_override {
        Some(path) => absolute(path)?,
        None => default_path(repository, branch.shorten())?,
    };
    let (worktree, _) = repository
        .create_worktree(
            &destination,
            gix::worktree::create::Head::Attached(branch),
            progress,
            interrupt,
        )
        .with_context(|| format!("could not create worktree at {}", destination.display()))?;
    Ok(worktree
        .workdir()
        .context("created worktree has no worktree directory")?
        .to_owned())
}

fn default_path(repository: &gix::Repository, short_branch: &gix::bstr::BStr) -> Result<PathBuf> {
    let main = repository.main_repo().context("could not open the main repository")?;
    let base = main.workdir().unwrap_or_else(|| main.git_dir());
    let parent = base.parent().context("repository path has no parent directory")?;
    let name = base.file_name().context("repository path has no file name")?;
    let mut destination = name.to_os_string();
    destination.push(".");
    let mut suffix = short_branch.to_owned();
    for byte in suffix.iter_mut() {
        if matches!(*byte, b'/' | b'\\') {
            *byte = b'-';
        }
    }
    destination.push(gix::path::from_bstr(suffix.as_bstr()).as_ref());
    Ok(parent.join(destination))
}

/// Run the picker, or resolve an explicit switch target without opening the terminal UI.
pub(crate) fn run(
    repository: gix::ThreadSafeRepository,
    target: Option<OsString>,
    path: Option<PathBuf>,
    create_branch_if_missing: bool,
    quit_on_finish: Option<String>,
) -> Result<()> {
    let repository = repository.to_thread_local();
    if let Some(target) = target {
        let selected = resolve_or_create(
            &repository,
            &target,
            path.as_deref(),
            create_branch_if_missing,
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;
        if !write_shell_handoff(&selected, false)? {
            println!("{}", selected.display());
        }
        return Ok(());
    }

    let mut worktrees = Worktrees::start(&repository)?;
    anyhow::ensure!(!worktrees.rows().is_empty(), "this repository has no worktrees");
    let selected = crate::pick_worktree(repository.into_sync(), &mut worktrees, quit_on_finish)?;
    let Some(selected) = selected else {
        return Ok(());
    };
    if write_shell_handoff(&selected, true)? {
        return Ok(());
    }
    drop(worktrees);
    std::env::set_current_dir(&selected).with_context(|| format!("could not enter worktree {}", selected.display()))?;
    crate::run_without_logging(
        gix::open(&selected)
            .with_context(|| format!("could not open worktree {}", selected.display()))?
            .into_sync(),
        Vec::new(),
        crate::Options::default(),
    )
}

/// Print the fully populated worktree picker table without opening the terminal UI.
pub(crate) fn show(repository: &gix::Repository, mut out: impl Write) -> Result<()> {
    let (hidden, unavailable) = crate::history::available_hidden_revisions(repository, &[], true)?;
    for (revision, err) in unavailable {
        eprintln!(
            "warning: ignoring unavailable hidden revision {}: {err}",
            revision.to_string_lossy()
        );
    }

    let mut worktrees = Worktrees::start(repository)?;
    anyhow::ensure!(!worktrees.rows().is_empty(), "this repository has no worktrees");
    worktrees.drain_updates();
    let mut graph = crate::history::HistoryGraph::default();
    let mut opened = Vec::with_capacity(worktrees.rows().len());
    let mut head_ids = Vec::new();
    for index in 0..worktrees.rows().len() {
        let path = worktrees.rows()[index].path.clone();
        match gix::open(&path)
            .with_context(|| format!("could not open worktree {}", path.display()))
            .and_then(|repository| logical_head(&repository).map(|head| (repository, head)))
        {
            Ok((repository, head)) => {
                if let Some(head_id) = head.commit_id
                    && !head_ids.contains(&head_id)
                {
                    head_ids.push(head_id);
                }
                opened.push((index, repository, head));
            }
            Err(err) => worktrees.set_graph_metadata(index, Err(format!("{err:#}"))),
        }
    }
    let revisions = head_ids
        .iter()
        .map(|id| OsString::from(id.to_hex().to_string()))
        .collect::<Vec<_>>();
    let hidden_tips = if revisions.is_empty() {
        Vec::new()
    } else {
        graph.refresh_graph(repository, &revisions, &hidden)?.hidden_tips
    };
    for (index, repository, head) in opened {
        let result = graph_metadata_for_head(&repository, &graph, &hidden_tips, head).map_err(|err| format!("{err:#}"));
        worktrees.set_graph_metadata(index, result);
    }
    worktrees.finish_workers()?;

    if let Some((row, err)) = worktrees.rows().iter().find_map(|row| match &row.state {
        LoadState::Error(err) => Some((row, err)),
        LoadState::Loading | LoadState::Ready => None,
    }) {
        anyhow::bail!("could not load worktree {}: {err}", row.path.display());
    }
    write_table(&worktrees, &mut out)
}

fn write_shell_handoff(path: &Path, fullscreen: bool) -> Result<bool> {
    let Some(cd_file) = std::env::var_os(shell::CD_FILE_ENV) else {
        return Ok(false);
    };
    let path = path
        .to_str()
        .with_context(|| format!("cannot hand off non-Unicode worktree path {}", path.display()))?;
    std::fs::write(&cd_file, path.as_bytes()).with_context(|| {
        format!(
            "could not write worktree selection to {}",
            Path::new(&cd_file).display()
        )
    })?;
    if fullscreen && let Some(marker) = std::env::var_os(shell::FULLSCREEN_FILE_ENV) {
        std::fs::write(&marker, b"1")
            .with_context(|| format!("could not write fullscreen marker to {}", Path::new(&marker).display()))?;
    }
    Ok(true)
}

/// Split `area` into a worktree list and a history area that always retains at least half the height.
pub(crate) fn areas(area: Rect, row_count: usize) -> [Rect; 2] {
    let list_height = u16::try_from(row_count)
        .unwrap_or(u16::MAX)
        .saturating_add(2)
        .min(area.height / 2);
    [
        Rect::new(area.x, area.y, area.width, list_height),
        Rect::new(
            area.x,
            area.y.saturating_add(list_height),
            area.width,
            area.height.saturating_sub(list_height),
        ),
    ]
}

/// Draw the streamed worktree inventory in its assigned area.
pub(crate) fn draw(frame: &mut Frame<'_>, area: Rect, worktrees: &Worktrees, focused: bool) {
    if area.is_empty() {
        return;
    }
    frame.render_widget(Clear, area);
    let status = if worktrees.search_is_open() {
        search_line(worktrees, focused)
    } else {
        let help = worktrees
            .selected()
            .and_then(|row| match &row.state {
                LoadState::Error(err) => Some((format!(" error: {err}"), Color::Red)),
                LoadState::Loading | LoadState::Ready => None,
            })
            .unwrap_or_else(|| {
                if worktrees.preview_pending() {
                    return (" loading preview; previous history is read-only".into(), Color::Yellow);
                }
                if focused {
                    (
                        " worktrees  j/k select  dd remove  DD force  / search  enter switch  tab history".into(),
                        Color::Cyan,
                    )
                } else {
                    (" worktrees  esc return".into(), Color::DarkGray)
                }
            });
        Line::from(Span::styled(
            help.0,
            Style::default().fg(help.1).add_modifier(Modifier::BOLD),
        ))
    };
    let visible = usize::from(area.height.saturating_sub(2));
    let visible_indices = worktrees.visible_indices(visible);
    let mut lines = table_lines(
        worktrees.rows(),
        usize::from(area.width),
        &visible_indices,
        worktrees.selected_index(),
        focused,
    );
    if visible_indices.is_empty() && visible > 0 {
        lines.push(Line::from(Span::styled(
            "   no matching worktrees",
            Style::default().fg(Color::DarkGray),
        )));
    }
    lines.push(status);
    frame.render_widget(Paragraph::new(lines), area);
}

fn table_lines(
    rows: &[Row],
    width: usize,
    indices: &[usize],
    selected: Option<usize>,
    focused: bool,
) -> Vec<Line<'static>> {
    let status_width = Line::raw("Status").width();
    let (base_width, commits_width) = statistic_widths(rows);
    let fixed_width = 3 + 2 + status_width + 2 + base_width + 2 + commits_width;
    let worktree_width = width.saturating_sub(fixed_width);
    let mut header = String::from("   ");
    push_cell(&mut header, "Worktree", worktree_width, true);
    header.push_str("  ");
    push_cell(&mut header, "Status", status_width, false);
    header.push_str("  ");
    push_cell(&mut header, "Base ±", base_width, false);
    header.push_str("  ");
    push_cell(&mut header, "Commits ↕", commits_width, false);
    let mut lines = vec![Line::from(Span::styled(
        header,
        Style::default()
            .fg(if focused { Color::Cyan } else { Color::DarkGray })
            .add_modifier(Modifier::BOLD),
    ))];
    for &index in indices {
        let row = &rows[index];
        let is_selected = selected == Some(index);
        let mut text = format!(
            "{}{} ",
            if is_selected { '>' } else { ' ' },
            if row.is_current {
                '@'
            } else if row.is_main {
                '^'
            } else {
                '+'
            }
        );
        push_cell(&mut text, &row.label, worktree_width, true);
        text.push_str("  ");
        push_cell(
            &mut text,
            match &row.state {
                LoadState::Loading => "…",
                LoadState::Ready if row.dirty == Some(true) => "*",
                LoadState::Ready => "",
                LoadState::Error(_) => "!",
            },
            status_width,
            false,
        );
        text.push_str("  ");
        let style = if is_selected {
            Style::default()
                .fg(if focused { Color::Black } else { Color::White })
                .bg(if focused { Color::Cyan } else { Color::DarkGray })
        } else {
            Style::default()
        };
        let mut spans = vec![Span::raw(text)];
        let base_width_used = match (row.lines_added, row.lines_removed) {
            (Some(added), Some(removed)) => {
                push_positive_negative(&mut spans, format!("+{added}"), format!("-{removed}"))
            }
            _ => 0,
        };
        spans.push(Span::raw(" ".repeat(base_width.saturating_sub(base_width_used) + 2)));
        let commits_width_used = row.relation.map_or(0, |relation| {
            push_positive_negative(
                &mut spans,
                format!("↑{}", relation.ahead),
                format!("↓{}", relation.behind),
            )
        });
        spans.push(Span::raw(" ".repeat(commits_width.saturating_sub(commits_width_used))));
        lines.push(Line::from(spans).style(style));
    }
    lines
}

fn statistic_widths(rows: &[Row]) -> (usize, usize) {
    let base = rows
        .iter()
        .filter_map(|row| match (row.lines_added, row.lines_removed) {
            (Some(added), Some(removed)) => Some(format!("+{added} -{removed}")),
            _ => None,
        })
        .map(|value| Line::raw(value).width())
        .max()
        .unwrap_or_default()
        .max(Line::raw("Base ±").width());
    let commits = rows
        .iter()
        .filter_map(|row| {
            row.relation
                .map(|relation| format!("↑{} ↓{}", relation.ahead, relation.behind))
        })
        .map(|value| Line::raw(value).width())
        .max()
        .unwrap_or_default()
        .max(Line::raw("Commits ↕").width());
    (base, commits)
}

fn write_table(worktrees: &Worktrees, mut out: impl Write) -> Result<()> {
    let worktree_width = worktrees
        .rows()
        .iter()
        .map(|row| Line::raw(&row.label).width())
        .max()
        .unwrap_or_default()
        .max(Line::raw("Worktree").width());
    let status_width = Line::raw("Status").width();
    let (base_width, commits_width) = statistic_widths(worktrees.rows());
    let width = 3 + worktree_width + 2 + status_width + 2 + base_width + 2 + commits_width;
    let indices = (0..worktrees.rows().len()).collect::<Vec<_>>();
    for line in table_lines(worktrees.rows(), width, &indices, None, false) {
        let plain = line.spans.iter().map(|span| span.content.as_ref()).collect::<String>();
        writeln!(out, "{}", plain.trim_end()).context("could not write worktree table")?;
    }
    Ok(())
}

fn search_line(worktrees: &Worktrees, focused: bool) -> Line<'static> {
    let query = worktrees.search_query();
    let cursor = worktrees.search_cursor();
    let before: String = query.chars().take(cursor).collect();
    let current = query.chars().nth(cursor);
    let after: String = query.chars().skip(cursor + usize::from(current.is_some())).collect();
    let color = if focused { Color::Cyan } else { Color::DarkGray };
    Line::from(vec![
        Span::styled("/ ", Style::default().fg(color).add_modifier(Modifier::BOLD)),
        Span::raw(before),
        Span::styled(
            current.map_or_else(|| " ".into(), |ch| ch.to_string()),
            Style::default().add_modifier(Modifier::REVERSED),
        ),
        Span::raw(after),
    ])
}

fn push_cell(line: &mut String, value: &str, width: usize, truncate_from_left: bool) {
    let value = if truncate_from_left {
        truncate_left(value, width)
    } else {
        value.to_owned()
    };
    let padding = width.saturating_sub(Line::raw(&value).width());
    line.push_str(&value);
    line.extend(std::iter::repeat_n(' ', padding));
}

fn push_positive_negative(spans: &mut Vec<Span<'static>>, positive: String, negative: String) -> usize {
    let width = Line::raw(&positive).width() + 1 + Line::raw(&negative).width();
    spans.extend([
        Span::styled(positive, Style::default().fg(Color::Green)),
        Span::raw(" "),
        Span::styled(negative, Style::default().fg(Color::LightRed)),
    ]);
    width
}

fn truncate_left(value: &str, width: usize) -> String {
    if Line::raw(value).width() <= width {
        return value.to_owned();
    }
    if width == 0 {
        return String::new();
    }
    let mut start = value.len();
    for (candidate, _) in value.char_indices().rev() {
        if Line::raw(format!("…{}", &value[candidate..])).width() > width {
            break;
        }
        start = candidate;
    }
    format!("…{}", &value[start..])
}

#[cfg(test)]
mod tests {
    use std::{process::Command, time::Duration};

    use gix::refs::transaction::PreviousValue;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn fixture() -> gix_testtools::Result<(gix_testtools::tempfile::TempDir, gix::Repository)> {
        let temp = gix_testtools::tempfile::TempDir::new()?;
        let path = temp.path().join("repo");
        std::fs::create_dir(&path)?;
        git(&path, &["init", "--initial-branch=main"])?;
        git(&path, &["config", "user.name", "Tix Test"])?;
        git(&path, &["config", "user.email", "tix@example.com"])?;
        git(&path, &["config", "commit.gpgSign", "false"])?;
        std::fs::write(path.join("tracked"), "base\n")?;
        git(&path, &["add", "tracked"])?;
        git(&path, &["commit", "-m", "base"])?;
        let repository = crate::test_repository::open(path)?;
        Ok((temp, repository))
    }

    fn git(path: &Path, args: &[&str]) -> gix_testtools::Result {
        let output = Command::new("git").current_dir(path).args(args).output()?;
        if !output.status.success() {
            return Err(format!(
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        Ok(())
    }

    fn create_branch(repository: &gix::Repository, name: &str) -> gix_testtools::Result {
        repository.reference(
            format!("refs/heads/{name}"),
            repository.head_id()?.detach(),
            PreviousValue::MustNotExist,
            "worktrunk test branch",
        )?;
        Ok(())
    }

    fn wait_until_dirty_loaded(worktrees: &mut Worktrees) {
        for _ in 0..500 {
            worktrees.drain_updates();
            if worktrees.rows().iter().all(|row| row.dirty.is_some()) {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("worktree dirty states did not finish loading");
    }

    fn test_row(label: &str, state: LoadState) -> Row {
        Row {
            path: PathBuf::from("/worktrees").join(label),
            label: label.into(),
            is_current: false,
            is_main: false,
            locked: false,
            state,
            head: None,
            dirty: None,
            relation: None,
            lines_added: None,
            lines_removed: None,
        }
    }

    fn test_worktrees(rows: Vec<Row>) -> Worktrees {
        let (_sender, updates) = mpsc::channel();
        Worktrees {
            rows,
            selected: 0,
            previewed: Some(0),
            previewing: false,
            search_origin: None,
            search: Menu::default(),
            updates,
            cancel: Arc::new(AtomicBool::default()),
            workers: Vec::new(),
            workers_suspended: false,
        }
    }

    fn rendered_line(terminal: &Terminal<TestBackend>, y: u16) -> String {
        (0..terminal.backend().buffer().area.width).fold(String::new(), |mut out, x| {
            out.push_str(terminal.backend().buffer()[(x, y)].symbol());
            out
        })
    }

    #[test]
    fn creation_reuses_paths_and_claimed_branches_and_inventory_order_is_stable() -> gix_testtools::Result {
        let (temp, repository) = fixture()?;
        for branch in ["zeta", "topic", "alpha", "gone"] {
            create_branch(&repository, branch)?;
        }
        let interrupt = AtomicBool::default();
        let zeta = resolve_or_create(
            &repository,
            OsStr::new("zeta"),
            None,
            false,
            gix::progress::Discard,
            &interrupt,
        )?;
        let topic = resolve_or_create(
            &repository,
            OsStr::new("topic"),
            None,
            false,
            gix::progress::Discard,
            &interrupt,
        )?;
        let alpha = resolve_or_create(
            &repository,
            OsStr::new("alpha"),
            None,
            false,
            gix::progress::Discard,
            &interrupt,
        )?;
        let gone = resolve_or_create(
            &repository,
            OsStr::new("gone"),
            None,
            false,
            gix::progress::Discard,
            &interrupt,
        )?;
        std::fs::remove_dir_all(&gone)?;
        let expected_topic = gix::path::realpath(
            repository
                .workdir()
                .expect("fixture has a worktree")
                .with_file_name("repo.topic"),
        )?;
        assert_eq!(
            topic, expected_topic,
            "newly created worktrees use the canonical path stored by Git"
        );

        let unused = topic.with_file_name("unused");
        assert_eq!(
            resolve_or_create(
                &repository,
                OsStr::new("topic"),
                Some(&unused),
                false,
                gix::progress::Discard,
                &interrupt,
            )?,
            topic,
            "a logically claimed branch reuses its worktree"
        );
        assert_eq!(
            resolve_or_create(
                &repository,
                topic.as_os_str(),
                None,
                false,
                gix::progress::Discard,
                &interrupt,
            )?,
            topic,
            "an exact worktree path takes precedence over branch resolution"
        );
        assert!(
            resolve_or_create(
                &repository,
                topic.as_os_str(),
                None,
                true,
                gix::progress::Discard,
                &interrupt,
            )
            .is_err(),
            "new-branch values are never interpreted as worktree paths"
        );
        assert!(!unused.exists(), "reusing a branch ignores the creation override");

        let rows = inventory(&repository)?;
        assert_eq!(
            rows.iter().map(|row| row.path.as_path()).collect::<Vec<_>>(),
            [
                repository.workdir().expect("fixture has a worktree"),
                alpha.as_path(),
                topic.as_path(),
                zeta.as_path(),
            ],
            "the current worktree is first, then linked worktrees sort lexically"
        );
        assert!(
            rows.iter().all(|row| row.path != gone),
            "registered worktrees without a directory aren't selectable"
        );
        assert!(rows[0].is_current && rows[0].is_main);

        #[cfg(unix)]
        {
            create_branch(&repository, "symlinked")?;
            let actual_parent = temp.path().join("actual");
            let linked_parent = temp.path().join("linked");
            std::fs::create_dir(&actual_parent)?;
            std::os::unix::fs::symlink(&actual_parent, &linked_parent)?;
            let requested = linked_parent.join("worktree");
            let selected = resolve_or_create(
                &repository,
                OsStr::new("symlinked"),
                Some(&requested),
                false,
                gix::progress::Discard,
                &interrupt,
            )?;
            assert_ne!(selected, requested, "the lexical symlink path is not handed off");
            assert_eq!(
                selected,
                gix::path::realpath(actual_parent.join("worktree"))?,
                "the canonical path recorded by Git is handed off"
            );
        }
        Ok(())
    }

    #[test]
    fn reinventory_discards_indexed_state_and_selects_the_neighbor_of_a_removed_row() -> gix_testtools::Result {
        let (_temp, repository) = fixture()?;
        for branch in ["alpha", "topic"] {
            create_branch(&repository, branch)?;
        }
        let interrupt = AtomicBool::default();
        let alpha = resolve_or_create(
            &repository,
            OsStr::new("alpha"),
            None,
            false,
            gix::progress::Discard,
            &interrupt,
        )?;
        let topic = resolve_or_create(
            &repository,
            OsStr::new("topic"),
            None,
            false,
            gix::progress::Discard,
            &interrupt,
        )?;
        let mut worktrees = Worktrees::start(&repository)?;
        let topic_index = worktrees
            .rows()
            .iter()
            .position(|row| row.path == topic)
            .expect("the topic worktree is inventoried");
        worktrees.select(topic_index);
        worktrees.open_search();
        worktrees.drain_updates();
        worktrees.suspend_workers_for_removal();
        assert!(worktrees.workers.is_empty(), "all dirty-state workers are joined");
        assert!(
            worktrees.workers_suspended,
            "drawing cannot restart workers during removal"
        );

        repository.remove_worktree(
            &topic,
            gix::worktree::remove::Force::DiscardChanges,
            gix::progress::Discard,
        )?;
        let selected = worktrees
            .reinventory_after_removal(&repository)?
            .expect("another worktree survives");

        assert_eq!(selected, alpha, "removing the final row selects its predecessor");
        assert!(!worktrees.search_is_open(), "removal closes index-based search state");
        assert!(!worktrees.workers_suspended, "the new inventory can load dirty state");
        assert!(worktrees.preview_pending(), "the survivor receives a fresh preview");
        assert!(worktrees.rows()[0].is_current, "the launch marker survives reinventory");
        assert!(worktrees.rows().iter().all(|row| row.path != topic));
        Ok(())
    }

    #[test]
    fn detached_head_uses_the_symbolic_tix_head_pin() -> gix_testtools::Result {
        let (_temp, repository) = fixture()?;
        create_branch(&repository, "topic")?;
        let interrupt = AtomicBool::default();
        let path = resolve_or_create(
            &repository,
            OsStr::new("topic"),
            None,
            false,
            gix::progress::Discard,
            &interrupt,
        )?;
        git(&path, &["remote", "add", "origin", "."])?;
        git(&path, &["update-ref", "refs/remotes/origin/main", "refs/heads/main"])?;
        git(
            &path,
            &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main"],
        )?;
        git(&path, &["config", "branch.topic.remote", "origin"])?;
        git(&path, &["config", "branch.topic.merge", "refs/heads/main"])?;
        std::fs::write(path.join("tracked"), "base\ntopic\n")?;
        git(&path, &["commit", "-am", "topic"])?;
        git(&path, &["switch", "--detach", "main"])?;
        git(
            &path,
            &["symbolic-ref", "refs/worktree/tix/pins/HEAD", "refs/heads/topic"],
        )?;

        let worktree = crate::test_repository::open(&path)?;
        let topic_id = worktree.rev_parse_single("refs/heads/topic")?.detach();
        let head = logical_head(&worktree)?;
        assert!(head.is_detached, "the physical HEAD remains detached");
        assert_eq!(
            head.branch.as_ref().map(gix::refs::FullName::as_bstr),
            Some(b"refs/heads/topic".as_bstr()),
            "the worktree-private pin supplies the logical branch"
        );
        assert_eq!(head.commit_id, Some(topic_id));
        let hidden = crate::history::available_hidden_revisions(&worktree, &[], true)?.0;
        let authors = gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(
            crate::history::Authors::default(),
        ));
        let mut graph = crate::history::HistoryGraph::default();
        let refresh = graph.refresh(&worktree, &[], &hidden, false, &Default::default(), &authors)?;
        let loaded = graph_metadata(&worktree, &graph, &refresh.refs)?;
        assert_eq!(
            loaded.relation,
            Some(Relation { ahead: 1, behind: 0 }),
            "the remembered branch supplies its upstream relation"
        );
        assert_eq!(
            loaded.diffstat,
            Some((1, 0)),
            "the hidden base supplies the detached worktree diffstat"
        );

        let switch_new = |name| {
            resolve_or_create(
                &worktree,
                OsStr::new(name),
                None,
                true,
                gix::progress::Discard,
                &interrupt,
            )
        };
        assert!(switch_new("child")?.is_dir(), "the new branch gets a worktree");
        assert_eq!(
            worktree.find_reference("refs/heads/child")?.id().detach(),
            topic_id,
            "new branches start at the logical Tix HEAD, not the detached physical HEAD"
        );
        let main_id = worktree.find_reference("refs/heads/main")?.id().detach();
        assert_eq!(
            switch_new("main")?,
            gix::path::realpath(repository.workdir().expect("fixture has a main worktree"))?,
            "new-branch mode switches to an existing branch"
        );
        assert_eq!(
            worktree.find_reference("refs/heads/main")?.id().detach(),
            main_id,
            "an existing branch is never reset"
        );
        Ok(())
    }

    #[test]
    fn picker_layout_never_takes_more_than_half_the_screen() {
        for height in 0..20 {
            let [list, history] = areas(Rect::new(3, 4, 80, height), usize::MAX);
            assert!(list.height <= height / 2);
            assert!(history.height >= height - height / 2);
            assert_eq!(list.height + history.height, height);
            assert_eq!(history.y, list.bottom());
        }
    }

    #[test]
    fn removal_blockers_protect_launch_main_and_locked_worktrees() {
        let mut row = test_row("topic", LoadState::Ready);
        assert_eq!(row.removal_blocker(), None);
        row.locked = true;
        assert_eq!(
            row.removal_blocker(),
            Some("locked worktrees require `tix wt remove -ff`")
        );
        row.is_main = true;
        assert_eq!(row.removal_blocker(), Some("the main worktree cannot be removed"));
        row.is_current = true;
        assert_eq!(
            row.removal_blocker(),
            Some("the worktree from which tix was launched cannot be removed")
        );
    }

    #[test]
    fn picker_draws_aligned_compact_columns_and_worktree_kinds() -> gix_testtools::Result {
        let mut current = test_row("repo", LoadState::Ready);
        current.is_current = true;
        current.is_main = true;
        current.dirty = Some(true);
        current.relation = Some(Relation { ahead: 12, behind: 3 });
        current.lines_added = Some(42);
        current.lines_removed = Some(7);
        let mut main = test_row("main-worktree", LoadState::Loading);
        main.is_main = true;
        let failed = test_row("topic", LoadState::Error("unavailable".into()));
        let mut worktrees = test_worktrees(vec![current, main, failed]);
        let mut terminal = Terminal::new(TestBackend::new(80, 5))?;

        terminal.draw(|frame| draw(frame, frame.area(), &worktrees, true))?;

        assert_eq!(
            rendered_line(&terminal, 4).trim_end(),
            " worktrees  j/k select  dd remove  DD force  / search  enter switch  tab history"
        );
        let header = rendered_line(&terminal, 0);
        assert!(header.starts_with("   Worktree"));
        assert_eq!(header.find("Status"), Some(55));
        assert_eq!(header.find("Base ±"), Some(63));
        assert_eq!(header.find("Commits ↕"), Some(72));
        let selected = rendered_line(&terminal, 1);
        assert!(selected.starts_with(">@ repo"));
        assert_eq!(&selected[55..], "*       +42 -7  ↑12 ↓3   ");
        assert!(rendered_line(&terminal, 2).starts_with(" ^ main-worktree"));
        assert_eq!(terminal.backend().buffer()[(79, 1)].bg, Color::Cyan);
        assert_eq!(terminal.backend().buffer()[(0, 2)].bg, Color::Reset);
        for (value, color) in [
            ("+42", Color::Green),
            ("-7", Color::LightRed),
            ("↑12", Color::Green),
            ("↓3", Color::LightRed),
        ] {
            let byte = selected.find(value).expect("statistic is visible");
            let x = selected[..byte].chars().count() as u16;
            for offset in 0..value.chars().count() as u16 {
                let cell = &terminal.backend().buffer()[(x + offset, 1)];
                assert_eq!(cell.fg, color, "{value} uses its semantic color");
                assert_eq!(cell.bg, Color::Cyan, "{value} retains the selected-row background");
            }
        }

        worktrees.select(2);
        terminal.draw(|frame| draw(frame, frame.area(), &worktrees, true))?;
        assert!(rendered_line(&terminal, 4).starts_with(" error: unavailable"));
        Ok(())
    }

    #[test]
    fn show_prints_every_fully_loaded_row_without_terminal_formatting() -> gix_testtools::Result {
        let (_temp, repository) = fixture()?;
        create_branch(&repository, "topic")?;
        let topic = resolve_or_create(
            &repository,
            OsStr::new("topic"),
            None,
            false,
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;
        let main = repository.workdir().context("fixture has a main worktree")?;
        git(main, &["remote", "add", "origin", "."])?;
        git(main, &["update-ref", "refs/remotes/origin/main", "refs/heads/main"])?;
        git(
            main,
            &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main"],
        )?;
        git(&topic, &["config", "branch.topic.remote", "origin"])?;
        git(&topic, &["config", "branch.topic.merge", "refs/heads/main"])?;
        std::fs::write(topic.join("topic"), "topic\n")?;
        git(&topic, &["add", "topic"])?;
        git(&topic, &["commit", "-m", "topic"])?;
        std::fs::write(topic.join("untracked"), "dirty\n")?;
        let broken_tag = repository.common_dir().join("refs/tags/broken");
        std::fs::create_dir_all(broken_tag.parent().expect("the tag has a parent directory"))?;
        std::fs::write(broken_tag, format!("{}\n", "f".repeat(40)))?;

        let repository = crate::test_repository::open(main)?;
        let mut output = Vec::new();
        show(&repository, &mut output)?;
        let output = String::from_utf8(output)?;
        let lines = output.lines().collect::<Vec<_>>();

        assert_eq!(lines.len(), 3, "the header and both worktrees are printed");
        assert!(lines[0].starts_with("   Worktree"), "the picker header is retained");
        assert!(lines[1].starts_with(" @ repo"), "the current main worktree is marked");
        assert!(
            lines[1].contains("+0 -0") && lines[1].contains("↑0 ↓0"),
            "zero-valued data is filled rather than omitted: {output:?}"
        );
        assert!(lines[2].starts_with(" + repo.topic"), "the linked worktree is marked");
        assert!(lines[2].contains('*'), "the completed dirty state is printed");
        assert!(
            lines[2].contains("+1 -0") && lines[2].contains("↑1 ↓0"),
            "base diffstat and ahead/behind data are complete: {output:?}"
        );
        assert!(
            !output.contains(['…', '\u{1b}']),
            "plain output has no loading or ANSI state"
        );
        Ok(())
    }

    #[test]
    fn picker_truncates_worktree_names_from_the_left() -> gix_testtools::Result {
        let mut row = test_row("repository-with-a-long-name", LoadState::Ready);
        row.is_current = true;
        let worktrees = test_worktrees(vec![row]);
        let mut terminal = Terminal::new(TestBackend::new(40, 3))?;

        terminal.draw(|frame| draw(frame, frame.area(), &worktrees, true))?;

        assert_eq!(rendered_line(&terminal, 0), "   Worktree    Status  Base ±  Commits ↕");
        assert_eq!(rendered_line(&terminal, 1), ">@ …long-name                           ");
        assert_eq!(truncate_left("工作树-überlang", 6), "…rlang");
        Ok(())
    }

    #[test]
    fn picker_search_maps_filtered_positions_to_stable_worktrees() -> gix_testtools::Result {
        let mut worktrees = test_worktrees(
            ["alpha", "beta", "gamma"]
                .into_iter()
                .map(|label| test_row(label, LoadState::Ready))
                .collect(),
        );
        worktrees.open_search();
        worktrees.edit_search(SearchInput::Down(1));
        assert_eq!(
            worktrees.selected_path(),
            Some(Path::new("/worktrees/beta")),
            "an empty query still permits result navigation"
        );
        worktrees.edit_search(SearchInput::Backspace);
        assert_eq!(
            worktrees.selected_path(),
            Some(Path::new("/worktrees/beta")),
            "editing an already-empty query retains its candidate"
        );
        worktrees.cancel_search();

        worktrees.open_search();
        worktrees.edit_search(SearchInput::Paste("aa".into()));
        assert_eq!(worktrees.visible_indices(10), [0, 2]);
        assert_eq!(worktrees.selected_path(), Some(Path::new("/worktrees/alpha")));

        worktrees.edit_search(SearchInput::Down(10));
        assert_eq!(
            worktrees.preview_search_selection(),
            Some(PathBuf::from("/worktrees/gamma")),
            "filtered navigation maps through the inventory instead of using its visible offset"
        );
        assert_eq!(worktrees.cancel_search(), Some(PathBuf::from("/worktrees/alpha")));
        assert_eq!(worktrees.selected_path(), Some(Path::new("/worktrees/alpha")));

        worktrees.open_search();
        worktrees.edit_search(SearchInput::Paste("zzz".into()));
        assert_eq!(worktrees.selected_index(), None);
        assert_eq!(
            worktrees.display_row_count(),
            1,
            "an empty-result message retains one row"
        );
        assert_eq!(worktrees.submit_search(), None);
        assert!(worktrees.search_is_open(), "an empty search cannot be submitted");
        let mut terminal = Terminal::new(TestBackend::new(50, 3))?;
        terminal.draw(|frame| draw(frame, frame.area(), &worktrees, true))?;
        assert!(rendered_line(&terminal, 1).starts_with("   no matching worktrees"));
        assert!(rendered_line(&terminal, 2).starts_with("/ zzz "));
        Ok(())
    }

    #[test]
    fn fallback_relation_requires_one_hidden_tip() -> gix_testtools::Result {
        let (_temp, repository) = fixture()?;
        let head_id = repository.head_id()?.detach();
        let other_id = gix::ObjectId::Sha1([42; 20]);
        let head = LogicalHead {
            branch: None,
            commit_id: Some(head_id),
            is_detached: true,
        };
        let graph = crate::history::HistoryGraph::from_test_commits(&[(head_id, vec![]), (other_id, vec![])]);
        assert_eq!(
            relation(&repository, &graph, &head, &[other_id])?,
            Some(Relation { ahead: 1, behind: 1 })
        );
        assert_eq!(
            relation(&repository, &graph, &head, &[other_id, head_id])?,
            None,
            "unrelated hidden tips don't become one synthetic comparison base"
        );
        Ok(())
    }

    #[test]
    fn workers_stream_and_refresh_dirty_state() -> gix_testtools::Result {
        let (_temp, repository) = fixture()?;
        let mut worktrees = Worktrees::start(&repository)?;
        wait_until_dirty_loaded(&mut worktrees);
        assert_eq!(worktrees.rows().len(), 1);
        assert_eq!(worktrees.rows()[0].state, LoadState::Loading);
        assert_eq!(worktrees.rows()[0].dirty, Some(false));
        let head = logical_head(&repository)?;
        worktrees.set_graph_metadata(
            0,
            Ok(GraphMetadata {
                head,
                relation: None,
                diffstat: None,
            }),
        );
        assert_eq!(worktrees.rows()[0].state, LoadState::Ready);
        assert!(!worktrees.preview_pending());
        worktrees.begin_preview();
        assert!(worktrees.preview_pending(), "lane computation keeps history read-only");
        worktrees.mark_previewed(0);
        assert!(!worktrees.preview_pending());

        std::fs::write(
            repository.workdir().expect("fixture has a worktree").join("untracked"),
            "dirty\n",
        )?;
        worktrees.refresh();
        assert_eq!(worktrees.rows()[0].state, LoadState::Loading);
        assert!(worktrees.preview_pending(), "refresh invalidates the visible preview");
        wait_until_dirty_loaded(&mut worktrees);
        assert_eq!(worktrees.rows()[0].dirty, Some(true));
        worktrees.mark_previewed(0);
        assert!(!worktrees.preview_pending());
        Ok(())
    }

    #[test]
    fn graph_metadata_invalidation_preserves_only_dirty_errors() {
        let head = LogicalHead {
            branch: None,
            commit_id: None,
            is_detached: false,
        };
        let mut dirty_error = test_row("dirty", LoadState::Error("status failed".into()));
        dirty_error.head = Some(head);
        dirty_error.relation = Some(Relation { ahead: 1, behind: 2 });
        dirty_error.lines_added = Some(3);
        dirty_error.lines_removed = Some(4);
        let mut graph_error = test_row("graph", LoadState::Error("history failed".into()));
        graph_error.dirty = Some(false);
        let mut worktrees = test_worktrees(vec![dirty_error, graph_error]);

        worktrees.invalidate_graph_metadata();

        assert_eq!(worktrees.rows()[0].state, LoadState::Error("status failed".into()));
        assert_eq!(worktrees.rows()[1].state, LoadState::Loading);
        for row in worktrees.rows() {
            assert!(row.head.is_none());
            assert!(row.relation.is_none());
            assert!(row.lines_added.is_none());
            assert!(row.lines_removed.is_none());
        }

        worktrees.refresh();
        assert!(worktrees.rows().iter().all(|row| row.state == LoadState::Loading));
        assert!(worktrees.rows().iter().all(|row| row.dirty.is_none()));
    }
}
