//! Worktree discovery and the asynchronously loaded information shown by the worktrunk picker.

pub(crate) mod shell;

use std::{
    collections::{HashMap, HashSet},
    ffi::{OsStr, OsString},
    fmt::Write as _,
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
struct Loaded {
    head: LogicalHead,
    dirty: bool,
    relation: Option<Relation>,
    diffstat: Option<(u64, u64)>,
}

#[derive(Debug)]
struct Update {
    index: usize,
    result: Result<Loaded, String>,
}

/// Stable worktree rows whose expensive fields are filled by one short-lived worker per row.
pub(crate) struct Worktrees {
    rows: Vec<Row>,
    selected: usize,
    updates: mpsc::Receiver<Update>,
    cancel: Arc<AtomicBool>,
    workers: Vec<thread::JoinHandle<()>>,
}

impl Worktrees {
    /// Inventory worktrees; the first update drain begins loading rows in parallel.
    pub(crate) fn start(repository: &gix::Repository) -> Result<Self> {
        let (_sender, updates) = mpsc::channel();
        Ok(Worktrees {
            rows: inventory(repository)?,
            selected: 0,
            updates,
            cancel: Arc::new(AtomicBool::new(false)),
            workers: Vec::new(),
        })
    }

    pub(crate) fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub(crate) fn selected(&self) -> Option<&Row> {
        self.rows.get(self.selected)
    }

    pub(crate) fn selected_index(&self) -> Option<usize> {
        (!self.rows.is_empty()).then_some(self.selected)
    }

    pub(crate) fn selected_path(&self) -> Option<&Path> {
        self.selected().map(|row| row.path.as_path())
    }

    pub(crate) fn select(&mut self, index: usize) {
        if !self.rows.is_empty() {
            self.selected = index.min(self.rows.len() - 1);
        }
    }

    /// Apply all currently available worker messages without blocking.
    pub(crate) fn drain_updates(&mut self) -> bool {
        if self.workers.is_empty() && self.rows.iter().any(|row| row.state == LoadState::Loading) {
            self.start_workers();
        }
        let updates: Vec<_> = self.updates.try_iter().collect();
        let changed = !updates.is_empty();
        for update in updates {
            let Some(row) = self.rows.get_mut(update.index) else {
                continue;
            };
            match update.result {
                Ok(Loaded {
                    head,
                    dirty,
                    relation,
                    diffstat,
                }) => {
                    row.head = Some(head);
                    row.dirty = Some(dirty);
                    row.relation = relation;
                    (row.lines_added, row.lines_removed) = diffstat
                        .map(|(added, removed)| (Some(added), Some(removed)))
                        .unwrap_or_default();
                    row.state = LoadState::Ready;
                }
                Err(err) => row.state = LoadState::Error(err),
            }
        }
        changed
    }

    /// Reload the expensive fields while preserving row order and selection.
    pub(crate) fn refresh(&mut self) {
        if self.rows.iter().any(|row| row.state == LoadState::Loading) {
            return;
        }
        self.cancel.store(true, Ordering::Relaxed);
        self.workers.clear();
        for row in &mut self.rows {
            row.state = LoadState::Loading;
            row.head = None;
            row.dirty = None;
            row.relation = None;
            row.lines_added = None;
            row.lines_removed = None;
        }
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
                        .and_then(|repository| load(repository, &cancel))
                        .map_err(|err| format!("{err:#}"));
                    if cancel.load(Ordering::Relaxed) || sender.send(Update { index, result }).is_err() {
                        break;
                    }
                }
            }));
        }
    }
}

impl Drop for Worktrees {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.workers.clear();
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
            entries.push(row(path, current == Some(None), true));
        }
    }
    for proxy in repository.worktrees().context("could not list linked worktrees")? {
        let id = proxy.id().to_owned();
        let path = absolute(&proxy.base().context("could not read a linked worktree path")?)?;
        if !path.is_dir() {
            continue;
        }
        let is_current = current.as_ref().and_then(Option::as_ref) == Some(&id);
        entries.push(row(path, is_current, false));
    }
    entries.sort_by(|a, b| {
        b.is_current
            .cmp(&a.is_current)
            .then_with(|| b.is_main.cmp(&a.is_main))
            .then_with(|| a.path.cmp(&b.path))
    });
    Ok(entries)
}

fn row(path: PathBuf, is_current: bool, is_main: bool) -> Row {
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

fn load(mut repository: gix::Repository, cancel: &Arc<AtomicBool>) -> Result<Loaded> {
    repository.object_cache_size(None);
    let head = logical_head(&repository)?;
    let dirty = is_dirty(&repository, cancel)?;
    let Some(head_id) = head.commit_id else {
        return Ok(Loaded {
            head,
            dirty,
            relation: None,
            diffstat: None,
        });
    };
    let hidden_tips = hidden_tips(&repository)?;
    let head_ancestry = ancestry(&repository, [head_id], cancel)?;
    let hidden_ancestry = ancestry(&repository, hidden_tips.iter().copied(), cancel)?;
    let relation = relation(
        &repository,
        &head,
        &head_ancestry,
        &hidden_tips,
        &hidden_ancestry,
        cancel,
    )?;
    let diffstat = unique_tix_boundary(head_id, &hidden_tips, &head_ancestry, &hidden_ancestry)
        .map(|base_id| diffstat(&repository, base_id, head_id))
        .transpose()?;
    Ok(Loaded {
        head,
        dirty,
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

#[derive(Default)]
struct Ancestry {
    ids: HashSet<gix::ObjectId>,
    parents: HashMap<gix::ObjectId, Vec<gix::ObjectId>>,
}

fn ancestry(
    repository: &gix::Repository,
    tips: impl IntoIterator<Item = gix::ObjectId>,
    cancel: &AtomicBool,
) -> Result<Ancestry> {
    let tips: Vec<_> = tips.into_iter().collect();
    if tips.is_empty() {
        return Ok(Ancestry::default());
    }
    let mut out = Ancestry::default();
    for info in repository
        .rev_walk(tips)
        .all()
        .context("could not start history traversal")?
    {
        if cancel.load(Ordering::Relaxed) {
            anyhow::bail!("worktree loading was cancelled");
        }
        let info = info.context("could not traverse history")?;
        out.ids.insert(info.id);
        out.parents.insert(info.id, info.parent_ids.iter().copied().collect());
    }
    Ok(out)
}

fn hidden_tips(repository: &gix::Repository) -> Result<Vec<gix::ObjectId>> {
    let (revisions, _) = crate::history::available_hidden_revisions(repository, &[], true)?;
    let mut tips = revisions
        .iter()
        .map(|revision| {
            let revision = gix::path::os_str_into_bstr(revision)
                .with_context(|| format!("hidden revision {} is not valid UTF-8", revision.to_string_lossy()))?;
            crate::history::resolve_revision(repository, revision).map(|(id, _)| id)
        })
        .collect::<Result<Vec<_>>>()?;
    tips.sort_unstable();
    tips.dedup();
    Ok(tips)
}

fn relation(
    repository: &gix::Repository,
    head: &LogicalHead,
    head_ancestry: &Ancestry,
    hidden_tips: &[gix::ObjectId],
    hidden_ancestry: &Ancestry,
    cancel: &AtomicBool,
) -> Result<Option<Relation>> {
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
        let upstream_ancestry = ancestry(repository, [upstream_id], cancel)?;
        return Ok(Some(compare(head_ancestry, &upstream_ancestry)));
    }
    Ok((hidden_tips.len() == 1).then(|| compare(head_ancestry, hidden_ancestry)))
}

fn compare(left: &Ancestry, right: &Ancestry) -> Relation {
    Relation {
        ahead: left.ids.difference(&right.ids).count(),
        behind: right.ids.difference(&left.ids).count(),
    }
}

fn unique_tix_boundary(
    head_id: gix::ObjectId,
    hidden_tips: &[gix::ObjectId],
    head: &Ancestry,
    hidden: &Ancestry,
) -> Option<gix::ObjectId> {
    let (_, boundary) = crate::history::view_scope(&[head_id], hidden_tips, |id, out| {
        if let Some(parents) = head.parents.get(&id).or_else(|| hidden.parents.get(&id)) {
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

/// Resolve an existing worktree path or local branch, creating a worktree for an unclaimed branch.
pub(crate) fn resolve_or_create<P>(
    repository: &gix::Repository,
    target: &OsStr,
    path_override: Option<&Path>,
    progress: P,
    interrupt: &AtomicBool,
) -> Result<PathBuf>
where
    P: gix::progress::NestedProgress,
    P::SubProgress: gix::progress::NestedProgress + 'static,
{
    let rows = inventory(repository)?;
    let target_path = absolute(Path::new(target))?;
    if let Some(row) = rows.iter().find(|row| row.path == target_path) {
        return Ok(row.path.clone());
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
    anyhow::ensure!(
        repository
            .try_find_reference(branch.as_bstr())
            .with_context(|| format!("could not read local branch {branch}"))?
            .is_some(),
        "target is neither an existing worktree path nor a local branch: {target}"
    );
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
) -> Result<()> {
    let repository = repository.to_thread_local();
    if let Some(target) = target {
        let selected = resolve_or_create(
            &repository,
            &target,
            path.as_deref(),
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
    let selected = crate::pick_worktree(repository.into_sync(), &mut worktrees)?;
    let Some(selected) = selected else {
        return Ok(());
    };
    if write_shell_handoff(&selected, true)? {
        return Ok(());
    }
    drop(worktrees);
    std::env::set_current_dir(&selected).with_context(|| format!("could not enter worktree {}", selected.display()))?;
    crate::run(
        gix::open(&selected)
            .with_context(|| format!("could not open worktree {}", selected.display()))?
            .into_sync(),
        Vec::new(),
        crate::Options::default(),
    )
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
        .saturating_add(1)
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
    let header = if focused {
        " worktrees  j/k select  enter switch  tab history"
    } else {
        " worktrees  esc return"
    };
    let mut lines = vec![Line::from(Span::styled(
        header,
        Style::default()
            .fg(if focused { Color::Cyan } else { Color::DarkGray })
            .add_modifier(Modifier::BOLD),
    ))];
    let visible = usize::from(area.height.saturating_sub(1));
    let selected = worktrees.selected_index().unwrap_or_default();
    let start = selected
        .saturating_sub(visible / 2)
        .min(worktrees.rows().len().saturating_sub(visible));
    for (index, row) in worktrees.rows().iter().enumerate().skip(start).take(visible) {
        let mut text = format!(
            "{}{} {}",
            if index == selected { '>' } else { ' ' },
            if row.is_current { '@' } else { ' ' },
            row.label
        );
        if let Some(head) = &row.head {
            match &head.branch {
                Some(branch) => {
                    text.push(' ');
                    text.push_str(&branch.shorten().to_str_lossy());
                }
                None if head.commit_id.is_some() => text.push_str(" detached"),
                None => text.push_str(" unborn"),
            }
            if head.is_detached && head.branch.is_some() {
                text.push_str(" (detached)");
            }
        }
        if row.dirty == Some(true) {
            text.push_str(" *");
        }
        if let Some(relation) = row.relation {
            let _ = write!(text, " ↑{} ↓{}", relation.ahead, relation.behind);
        }
        if let (Some(added), Some(removed)) = (row.lines_added, row.lines_removed) {
            let _ = write!(text, " +{added} -{removed}");
        }
        match &row.state {
            LoadState::Loading => text.push_str(" …"),
            LoadState::Ready => {}
            LoadState::Error(err) => {
                let _ = write!(text, " ! {err}");
            }
        }
        text.push_str("  ");
        text.push_str(&row.path.to_string_lossy());
        let style = if index == selected {
            Style::default()
                .fg(if focused { Color::Black } else { Color::White })
                .bg(if focused { Color::Cyan } else { Color::DarkGray })
        } else {
            Style::default()
        };
        lines.push(Line::styled(text, style));
    }
    frame.render_widget(Paragraph::new(lines), area);
}

#[cfg(test)]
mod tests {
    use std::{process::Command, time::Duration};

    use gix::refs::transaction::PreviousValue;

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

    fn wait_until_loaded(worktrees: &mut Worktrees) {
        for _ in 0..500 {
            worktrees.drain_updates();
            if worktrees
                .rows()
                .iter()
                .all(|row| !matches!(row.state, LoadState::Loading))
            {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("worktree rows did not finish loading");
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
            gix::progress::Discard,
            &interrupt,
        )?;
        let topic = resolve_or_create(
            &repository,
            OsStr::new("topic"),
            None,
            gix::progress::Discard,
            &interrupt,
        )?;
        let alpha = resolve_or_create(
            &repository,
            OsStr::new("alpha"),
            None,
            gix::progress::Discard,
            &interrupt,
        )?;
        let gone = resolve_or_create(
            &repository,
            OsStr::new("gone"),
            None,
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
                gix::progress::Discard,
                &interrupt,
            )?,
            topic,
            "a logically claimed branch reuses its worktree"
        );
        assert_eq!(
            resolve_or_create(&repository, topic.as_os_str(), None, gix::progress::Discard, &interrupt,)?,
            topic,
            "an exact worktree path takes precedence over branch resolution"
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
    fn detached_head_uses_the_symbolic_tix_head_pin() -> gix_testtools::Result {
        let (_temp, repository) = fixture()?;
        create_branch(&repository, "topic")?;
        let interrupt = AtomicBool::default();
        let path = resolve_or_create(
            &repository,
            OsStr::new("topic"),
            None,
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
        git(&path, &["switch", "--detach"])?;
        git(
            &path,
            &["symbolic-ref", "refs/worktree/tix/pins/HEAD", "refs/heads/topic"],
        )?;

        let worktree = crate::test_repository::open(path)?;
        let topic_id = worktree.rev_parse_single("refs/heads/topic")?.detach();
        let head = logical_head(&worktree)?;
        assert!(head.is_detached, "the physical HEAD remains detached");
        assert_eq!(
            head.branch.as_ref().map(gix::refs::FullName::as_bstr),
            Some(b"refs/heads/topic".as_bstr()),
            "the worktree-private pin supplies the logical branch"
        );
        assert_eq!(head.commit_id, Some(topic_id));
        let loaded = load(worktree, &Arc::new(AtomicBool::default()))?;
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
    fn fallback_relation_requires_one_hidden_tip() -> gix_testtools::Result {
        let (_temp, repository) = fixture()?;
        let head_id = repository.head_id()?.detach();
        let other_id = gix::ObjectId::Sha1([42; 20]);
        let head = LogicalHead {
            branch: None,
            commit_id: Some(head_id),
            is_detached: true,
        };
        let head_ancestry = Ancestry {
            ids: HashSet::from([head_id]),
            ..Ancestry::default()
        };
        let hidden_ancestry = Ancestry {
            ids: HashSet::from([other_id]),
            ..Ancestry::default()
        };
        let cancel = AtomicBool::default();
        assert_eq!(
            relation(
                &repository,
                &head,
                &head_ancestry,
                &[other_id],
                &hidden_ancestry,
                &cancel,
            )?,
            Some(Relation { ahead: 1, behind: 1 })
        );
        assert_eq!(
            relation(
                &repository,
                &head,
                &head_ancestry,
                &[other_id, head_id],
                &hidden_ancestry,
                &cancel,
            )?,
            None,
            "unrelated hidden tips don't become one synthetic comparison base"
        );
        Ok(())
    }

    #[test]
    fn workers_stream_and_refresh_dirty_state() -> gix_testtools::Result {
        let (_temp, repository) = fixture()?;
        let mut worktrees = Worktrees::start(&repository)?;
        wait_until_loaded(&mut worktrees);
        assert_eq!(worktrees.rows().len(), 1);
        assert_eq!(worktrees.rows()[0].state, LoadState::Ready);
        assert_eq!(worktrees.rows()[0].dirty, Some(false));

        std::fs::write(
            repository.workdir().expect("fixture has a worktree").join("untracked"),
            "dirty\n",
        )?;
        worktrees.refresh();
        assert_eq!(worktrees.rows()[0].state, LoadState::Loading);
        wait_until_loaded(&mut worktrees);
        assert_eq!(worktrees.rows()[0].dirty, Some(true));
        Ok(())
    }
}
