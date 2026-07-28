//! A fast, interactive commit graph for terminals.

#![forbid(unsafe_code)]

mod app;
mod history;
mod ui;

use std::{
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
use app::{Action, App, ChangeKind, Changes, CommitRow, ComparedParent, Effect, PathChange, State};
use crossterm::{
    clipboard::CopyToClipboard,
    cursor,
    event::{
        self, DisableFocusChange, EnableFocusChange, Event as TerminalEvent, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers, KeyboardEnhancementFlags, ModifierKeyCode, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    execute,
    style::{Print, ResetColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use gix::{
    bstr::{BString, ByteSlice},
    prelude::TreeDiffChangeExt,
};
use history::{Authors, Decorations, Event, SharedAuthors};
use ratatui::{TerminalOptions, Viewport, backend::CrosstermBackend, text::Line};

const EVENT_BATCH_SIZE: usize = 256;
const OBJECT_CACHE_SIZE: usize = 4 * 1024 * 1024;
const FRAME_INTERVAL: Duration = Duration::from_nanos(16_666_667);
const REPEAT_IDLE: Duration = Duration::from_millis(75);
const IMMEDIATE_PAGER_EXIT: Duration = Duration::from_millis(250);

struct FillRepository<'a> {
    path: &'a Path,
    retained: Option<gix::Repository>,
    retain: bool,
}

type LineCounts = Option<(u32, u32)>;
type LineDiffResult = (usize, gix::object::tree::diff::ChangeDetached, Result<LineCounts>);

struct LineDiffJob {
    index: usize,
    change: gix::object::tree::diff::ChangeDetached,
}

struct LineDiffPool {
    jobs: Option<mpsc::Sender<LineDiffJob>>,
    results: mpsc::Receiver<LineDiffResult>,
    workers: Vec<std::thread::JoinHandle<()>>,
}

impl LineDiffPool {
    fn new(repository_path: &Path, parallelism: usize) -> Result<Self> {
        let repository = gix::open(repository_path)
            .context("could not open repository for parallel line diffs")?
            .into_sync();
        let mut worker_state = Vec::with_capacity(parallelism);
        for _ in 0..parallelism {
            let mut repository = repository.to_thread_local();
            repository.object_cache_size(OBJECT_CACHE_SIZE);
            let resource_cache = repository
                .diff_resource_cache_for_tree_diff()
                .context("could not initialize parallel line diffs")?;
            worker_state.push((repository, resource_cache));
        }

        let (jobs, job_receiver) = mpsc::channel::<LineDiffJob>();
        let job_receiver =
            gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(job_receiver));
        let (result_sender, results) = mpsc::channel();
        let workers = worker_state
            .into_iter()
            .map(|(repository, mut resource_cache)| {
                let job_receiver = gix::features::threading::OwnShared::clone(&job_receiver);
                let result_sender = result_sender.clone();
                std::thread::spawn(move || {
                    loop {
                        let Ok(job) = gix::features::threading::lock(&job_receiver).recv() else {
                            break;
                        };
                        let result = job
                            .change
                            .attach(&repository, &repository)
                            .diff(&mut resource_cache)
                            .context("could not prepare line diff")
                            .and_then(|mut diff| {
                                diff.line_counts()
                                    .context("could not count changed lines")
                                    .map(|counts| counts.map(|counts| (counts.insertions, counts.removals)))
                            });
                        resource_cache.clear_resource_cache_keep_allocation();
                        if result_sender.send((job.index, job.change, result)).is_err() {
                            break;
                        }
                    }
                })
            })
            .collect();
        Ok(LineDiffPool {
            jobs: Some(jobs),
            results,
            workers,
        })
    }

    fn line_counts(
        &mut self,
        changes: Vec<gix::object::tree::diff::ChangeDetached>,
    ) -> Result<Vec<(gix::object::tree::diff::ChangeDetached, LineCounts)>> {
        let len = changes.len();
        let jobs = self.jobs.as_ref().context("line diff pool is shutting down")?;
        for (index, change) in changes.into_iter().enumerate() {
            jobs.send(LineDiffJob { index, change })
                .context("line diff workers stopped unexpectedly")?;
        }

        let mut out: Vec<_> = std::iter::repeat_with(|| None).take(len).collect();
        let mut first_error = None;
        for _ in 0..len {
            let (index, change, result) = self.results.recv().context("line diff workers stopped unexpectedly")?;
            match result {
                Ok(lines) => {
                    *out.get_mut(index)
                        .context("line diff worker returned an invalid result index")? = Some((change, lines));
                }
                Err(err) if first_error.is_none() => first_error = Some(err),
                Err(_) => {}
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
        drop(self.jobs.take());
        for worker in self.workers.drain(..) {
            drop(worker.join());
        }
    }
}

fn sync_line_diff_pool(
    pool: &mut Option<LineDiffPool>,
    visible: bool,
    repository_path: &Path,
    parallelism: usize,
) -> Result<()> {
    if visible && pool.is_none() {
        *pool = Some(LineDiffPool::new(repository_path, parallelism.max(1))?);
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

pub(crate) struct BuiltInDiff {
    title: BString,
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
            lines,
            max_width,
        }
    }

    fn write_to(&self, mut out: impl Write) -> io::Result<()> {
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
    /// How much of the terminal to use.
    pub screen: Screen,
}

/// How `gix-tix` occupies the terminal.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Screen {
    /// Use the main screen for short histories, otherwise the alternate screen.
    #[default]
    Auto,
    /// Always use the alternate screen.
    Always,
    /// Use half of the main screen.
    Half,
}

/// Run the interactive commit graph for `repository`.
pub fn run(repository: gix::ThreadSafeRepository, revisions: Vec<OsString>, options: Options) -> Result<()> {
    let terminal_height = match options.screen {
        Screen::Always => 0,
        Screen::Auto | Screen::Half => terminal::size().context("could not determine terminal size")?.1,
    };
    let visible_commits = match options.screen {
        Screen::Auto | Screen::Half => history::count_up_to(
            &repository.to_thread_local(),
            &revisions,
            &options.hide,
            half_height(terminal_height) as usize,
        )?,
        Screen::Always => 0,
    };
    let inline_height = inline_height(options.screen, terminal_height, visible_commits);
    let mut terminal = match inline_height {
        Some(height) => ratatui::try_init_with_options(TerminalOptions {
            viewport: Viewport::Inline(height),
        }),
        None => ratatui::try_init(),
    }
    .context("could not initialize terminal")?;
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
                inline_height.is_some(),
                enhanced_keyboard,
            )
        });
    let keyboard_restore = disable_input(terminal.backend_mut(), enhanced_keyboard);
    let restore = restore_terminal(&mut terminal, inline_height.is_some());
    let lane_time = result?;
    keyboard_restore.context("could not restore keyboard events")?;
    restore?;
    if let Some(lane_time) = lane_time {
        eprintln!("lane computation: {:.3}s", lane_time.as_secs_f64());
    }
    Ok(())
}

fn enable_input(backend: &mut CrosstermBackend<std::io::Stdout>, enhanced_keyboard: bool) -> std::io::Result<()> {
    execute!(backend, EnableFocusChange)?;
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
    execute!(backend, DisableFocusChange)
}

fn half_height(terminal_height: u16) -> u16 {
    (terminal_height / 2).max(1)
}

fn inline_height(screen: Screen, terminal_height: u16, visible_commits: usize) -> Option<u16> {
    let half = half_height(terminal_height);
    let compact = u16::try_from(visible_commits).unwrap_or(u16::MAX).saturating_add(3);
    match screen {
        Screen::Always => None,
        Screen::Half => Some(compact.min(half)),
        Screen::Auto if visible_commits < half as usize => Some(compact),
        Screen::Auto => None,
    }
}

fn restore_terminal(terminal: &mut ratatui::DefaultTerminal, inline: bool) -> Result<()> {
    if !inline {
        return ratatui::try_restore().context("could not restore terminal");
    }

    let cursor = (|| {
        let area = terminal.get_frame().area();
        let terminal_height = terminal.size()?.height;
        execute!(
            terminal.backend_mut(),
            cursor::MoveTo(0, area.bottom().saturating_sub(1)),
            Clear(ClearType::CurrentLine)
        )?;
        if area.bottom() < terminal_height {
            execute!(terminal.backend_mut(), cursor::MoveTo(0, area.bottom()))
        } else {
            execute!(
                terminal.backend_mut(),
                cursor::MoveTo(0, terminal_height.saturating_sub(1)),
                Print("\r\n")
            )
        }
        .and_then(|()| terminal.show_cursor())
    })();
    let raw_mode = terminal::disable_raw_mode();
    cursor.context("could not restore terminal cursor")?;
    raw_mode.context("could not disable terminal raw mode")?;
    Ok(())
}

fn enter_alternate_screen(
    terminal: &mut ratatui::DefaultTerminal,
    enhanced_keyboard: bool,
) -> std::io::Result<ratatui::DefaultTerminal> {
    disable_input(terminal.backend_mut(), enhanced_keyboard)?;
    let alternate = ratatui::Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    let inline = std::mem::replace(terminal, alternate);
    enable_input(terminal.backend_mut(), enhanced_keyboard)?;
    Ok(inline)
}

fn leave_alternate_screen(
    terminal: &mut ratatui::DefaultTerminal,
    inline: ratatui::DefaultTerminal,
    enhanced_keyboard: bool,
) -> std::io::Result<()> {
    disable_input(terminal.backend_mut(), enhanced_keyboard)?;
    drop(std::mem::replace(terminal, inline));
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    enable_input(terminal.backend_mut(), enhanced_keyboard)?;
    terminal.hide_cursor()
}

fn should_switch_screen(started_inline: bool, needs_alternate_screen: bool, in_alternate_screen: bool) -> bool {
    started_inline && needs_alternate_screen != in_alternate_screen
}

fn configure_initial_screen(app: &mut App, inline: bool) {
    app.inline = inline;
    if inline {
        app.show_changes = false;
    }
}

fn history_needs_alternate_screen(screen: Screen, terminal_height: u16, commits: usize) -> bool {
    screen == Screen::Auto && inline_height(screen, terminal_height, commits).is_none()
}

fn needs_alternate_screen(
    show_panel: bool,
    history_requires_alternate_screen: bool,
    current_inline_height: Option<u16>,
) -> bool {
    show_panel || history_requires_alternate_screen || current_inline_height.is_none()
}

fn resize_inline_screen(terminal: &mut ratatui::DefaultTerminal, height: u16) -> std::io::Result<()> {
    if terminal.get_frame().area().height == height {
        return Ok(());
    }
    let resized = ratatui::Terminal::with_options(
        CrosstermBackend::new(std::io::stdout()),
        TerminalOptions {
            viewport: Viewport::Inline(height),
        },
    )?;
    drop(std::mem::replace(terminal, resized));
    terminal.hide_cursor()
}

#[expect(
    clippy::too_many_arguments,
    reason = "screen transitions need the complete terminal state"
)]
fn sync_screen(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    screen: Screen,
    started_inline: bool,
    history_requires_alternate_screen: bool,
    resize_inline: bool,
    inline_terminal: &mut Option<ratatui::DefaultTerminal>,
    enhanced_keyboard: bool,
) -> Result<()> {
    let inline_height = inline_height(screen, terminal::size()?.1, app.rows.len());
    let needs_alternate_screen = needs_alternate_screen(
        app.show_commit || app.show_changes,
        history_requires_alternate_screen,
        inline_height,
    );
    if !should_switch_screen(started_inline, needs_alternate_screen, inline_terminal.is_some()) {
        if let (true, Some(height)) = (started_inline && app.inline && resize_inline, inline_height) {
            resize_inline_screen(terminal, height).context("could not resize the inline history")?;
        }
        return Ok(());
    }
    if needs_alternate_screen {
        *inline_terminal =
            Some(enter_alternate_screen(terminal, enhanced_keyboard).context("could not enter the alternate screen")?);
        app.inline = false;
    } else if let Some(inline) = inline_terminal.take() {
        leave_alternate_screen(terminal, inline, enhanced_keyboard).context("could not leave the alternate screen")?;
        app.inline = true;
        if let Some(height) = inline_height {
            resize_inline_screen(terminal, height).context("could not resize the inline history")?;
        }
    }
    Ok(())
}

fn event_loop(
    terminal: &mut ratatui::DefaultTerminal,
    repository: gix::ThreadSafeRepository,
    revisions: Vec<OsString>,
    options: Options,
    started_inline: bool,
    enhanced_keyboard: bool,
) -> Result<Option<Duration>> {
    let Options {
        quit_on_finish,
        hide,
        screen,
    } = options;
    let repository_path = repository.git_dir().to_owned();
    let mailmap = gix::open(&repository_path)
        .context("could not open repository for mailmap")?
        .open_mailmap();
    let authors = gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(Authors::default()));
    let (mut cancelled, mut receiver) = start_history(
        repository,
        &revisions,
        &hide,
        gix::features::threading::OwnShared::clone(&authors),
    );

    let mut app = App::new(1);
    let mut lane_receiver = None;
    let mut verification_receiver = None;
    let mut commit_message = None;
    let mut changes = None;
    let line_diff_parallelism = std::thread::available_parallelism().map_or(1, Into::into);
    let mut line_diff_pool = None;
    let mut fill_repository = FillRepository {
        path: &repository_path,
        retained: None,
        retain: false,
    };
    configure_initial_screen(&mut app, started_inline);
    app.configure_hidden_filter(!hide.is_empty());
    sync_line_diff_pool(
        &mut line_diff_pool,
        app.show_changes,
        &repository_path,
        line_diff_parallelism,
    )?;
    let mut decorations = Decorations::new();
    draw(
        terminal,
        &mut app,
        &decorations,
        &mailmap,
        &authors,
        &mut fill_repository,
        &mut commit_message,
        &mut changes,
        &mut line_diff_pool,
    )?;
    let mut last_draw = Instant::now();
    let mut dirty = false;
    let mut urgent = false;
    let mut inline_terminal = None;
    let mut history_requires_alternate_screen = false;
    let mut focused = true;
    let mut repeat_deadline: Option<Instant> = None;
    let result: Result<Option<Duration>> = (|| loop {
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
        if urgent {
            draw(
                terminal,
                &mut app,
                &decorations,
                &mailmap,
                &authors,
                &mut fill_repository,
                &mut commit_message,
                &mut changes,
                &mut line_diff_pool,
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
        let mut resize_inline = false;
        while events < EVENT_BATCH_SIZE {
            let message = match receiver.try_recv() {
                Ok(message) => message,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected)
                    if matches!(app.state, State::Computing | State::Complete | State::Cancelled) =>
                {
                    break;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    anyhow::bail!("history worker stopped unexpectedly")
                }
            };
            events += 1;
            dirty = true;
            match message? {
                Event::Decorations(value) => decorations = value,
                Event::Commits(rows) => {
                    app.extend_commits(rows);
                    if history_needs_alternate_screen(screen, terminal::size()?.1, app.rows.len()) {
                        history_requires_alternate_screen = true;
                    }
                }
                Event::HiddenCommits(rows) => {
                    app.extend_hidden_commits(rows);
                    if history_needs_alternate_screen(screen, terminal::size()?.1, app.rows.len()) {
                        history_requires_alternate_screen = true;
                    }
                }
                Event::Complete => {
                    resize_inline = true;
                    history_requires_alternate_screen =
                        history_needs_alternate_screen(screen, terminal::size()?.1, app.rows.len());
                    if let Some(rows) = app.start_lane_computation() {
                        lane_receiver = Some(start_lane_worker(rows));
                    }
                }
                Event::Cancelled => drop(app.update(Action::Cancelled)),
            }
        }
        sync_screen(
            terminal,
            &mut app,
            screen,
            started_inline,
            history_requires_alternate_screen,
            resize_inline,
            &mut inline_terminal,
            enhanced_keyboard,
        )?;
        let streaming = matches!(app.state, State::Loading | State::Cancelling | State::Computing)
            || verification_receiver.is_some();
        if should_draw(dirty, streaming, last_draw.elapsed()) {
            draw(
                terminal,
                &mut app,
                &decorations,
                &mailmap,
                &authors,
                &mut fill_repository,
                &mut commit_message,
                &mut changes,
                &mut line_diff_pool,
            )?;
            last_draw = Instant::now();
            dirty = false;
        }
        let repeat_timeout = repeat_deadline.map(|deadline| deadline.saturating_duration_since(Instant::now()));
        let terminal_event = match poll_timeout(streaming, events, dirty, last_draw.elapsed(), repeat_timeout) {
            Some(timeout) if event::poll(timeout)? => Some(event::read()?),
            Some(_) => None,
            None => Some(event::read()?),
        };
        let Some(terminal_event) = terminal_event else {
            continue;
        };
        let key = match terminal_event {
            TerminalEvent::Key(key) => key,
            TerminalEvent::FocusLost => {
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
                continue;
            }
            TerminalEvent::Resize(_, _) => {
                dirty = true;
                urgent = true;
                continue;
            }
            _ => continue,
        };
        if !focused {
            continue;
        }
        let action = action(key);
        let repeats_history = retains_fill_repository(key.kind, action.as_ref(), app.changes_focused);
        if repeats_history {
            fill_repository.retain = true;
            repeat_deadline = Some(Instant::now() + REPEAT_IDLE);
        } else if key.kind != KeyEventKind::Repeat {
            fill_repository.retain = false;
            fill_repository.retained = None;
        }
        if repeats_history && app.show_changes {
            app.changes_suppressed = true;
        } else if key.kind != KeyEventKind::Repeat && app.changes_suppressed {
            app.changes_suppressed = false;
            repeat_deadline = None;
            dirty = true;
            urgent = true;
        }
        let Some(action) = action else {
            continue;
        };
        if action == Action::ToggleChangesFocus && !changes_focusable(changes.as_ref().map(|(_, _, changes)| changes)) {
            continue;
        }
        dirty = true;
        urgent = true;
        let toggles_changes = action == Action::ToggleChanges;
        let effects = app.update(action);
        if toggles_changes {
            sync_line_diff_pool(
                &mut line_diff_pool,
                app.show_changes,
                &repository_path,
                line_diff_parallelism,
            )?;
        }
        for effect in effects {
            match effect {
                Effect::Cancel => cancelled.store(true, Ordering::Relaxed),
                Effect::CopyId(id) => execute!(
                    terminal.backend_mut(),
                    CopyToClipboard::to_clipboard_from(id.to_hex().to_string())
                )?,
                Effect::CopyAuthor(author) => {
                    let actor = actor_bytes(author);
                    execute!(terminal.backend_mut(), CopyToClipboard::to_clipboard_from(actor))?;
                }
                Effect::Reload(show_hidden) => {
                    cancelled.store(true, Ordering::Relaxed);
                    app.reload(show_hidden);
                    decorations.clear();
                    let hidden = if show_hidden { &[][..] } else { hide.as_slice() };
                    (cancelled, receiver) = start_history(
                        gix::ThreadSafeRepository::open_opts(&repository_path, gix::open::Options::isolated())
                            .context("could not reopen repository for history reload")?,
                        &revisions,
                        hidden,
                        gix::features::threading::OwnShared::clone(&authors),
                    );
                }
                Effect::OpenDiff(index) => {
                    let result = changes
                        .as_ref()
                        .and_then(|(_, _, changes)| changes.diffs.get(index).zip(changes.paths.get(index)))
                        .context("selected path no longer has diff resources")
                        .and_then(|(change, path)| prepare_file_diff(&repository_path, change, path))
                        .and_then(|diff| match diff {
                            FileDiff::External(command) => {
                                run_external_diff(terminal, command, enhanced_keyboard).map(|()| false)
                            }
                            FileDiff::Pager { command, diff } => {
                                run_pager(terminal, command, &diff, enhanced_keyboard).map(|()| false)
                            }
                            FileDiff::BuiltIn(diff) => show_builtin_diff(terminal, &diff),
                        });
                    match result {
                        Ok(true) => app.focus_history(),
                        Err(err) => app.diff_error = Some(format!("{err:#}")),
                        Ok(false) => {}
                    }
                }
                Effect::VerifySignatures(ids) => {
                    verification_receiver = Some(start_signature_verification(repository_path.clone(), ids));
                }
                Effect::Quit => return Ok(None),
            }
        }
        sync_screen(
            terminal,
            &mut app,
            screen,
            started_inline,
            history_requires_alternate_screen,
            false,
            &mut inline_terminal,
            enhanced_keyboard,
        )?;
    })();
    let restore = inline_terminal
        .map(|inline| leave_alternate_screen(terminal, inline, enhanced_keyboard))
        .transpose();
    let outcome = result?;
    restore.context("could not restore the inline terminal")?;
    if outcome.is_none() && started_inline {
        prepare_inline_exit(&mut app);
        sync_line_diff_pool(&mut line_diff_pool, false, &repository_path, line_diff_parallelism)?;
        draw(
            terminal,
            &mut app,
            &decorations,
            &mailmap,
            &authors,
            &mut fill_repository,
            &mut commit_message,
            &mut changes,
            &mut line_diff_pool,
        )?;
    }
    Ok(outcome)
}

fn prepare_inline_exit(app: &mut App) {
    app.inline = true;
    app.show_commit = false;
    app.show_changes = false;
    app.changes_suppressed = false;
    app.changes_focused = false;
    app.reset_changes_view();
    app.show_selection_tail = false;
}

fn start_lane_worker(rows: Vec<CommitRow>) -> mpsc::Receiver<(Vec<CommitRow>, app::Graph, Duration)> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(app::compute_lanes(rows));
    });
    receiver
}

type SignatureVerification = (gix::ObjectId, bool);

fn start_signature_verification(
    repository_path: PathBuf,
    ids: Vec<gix::ObjectId>,
) -> mpsc::Receiver<Vec<SignatureVerification>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let results = match gix::open(repository_path) {
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

#[expect(clippy::too_many_arguments, reason = "drawing needs the complete view state")]
fn draw(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    decorations: &Decorations,
    mailmap: &gix::mailmap::Snapshot,
    authors: &SharedAuthors,
    fill_repository: &mut FillRepository<'_>,
    commit_message: &mut Option<(gix::ObjectId, BString)>,
    changes: &mut Option<(gix::ObjectId, usize, Changes)>,
    line_diff_pool: &mut Option<LineDiffPool>,
) -> Result<()> {
    app.viewport_rows = terminal
        .get_frame()
        .area()
        .height
        .saturating_sub(1 + 2 * u16::from(app.inline)) as usize;
    if !history_is_ready_to_draw(app.state, app.rows.len()) {
        return Ok(());
    }
    app.ensure_visible();
    let start = app.offset.min(app.rows.len());
    let end = start.saturating_add(app.viewport_rows).min(app.rows.len());
    let changes_visible = app.changes_visible();
    let selected = (app.show_commit || changes_visible)
        .then(|| app.selected.and_then(|index| app.rows.get(index)).map(|row| row.id))
        .flatten();
    let message_to_load = app
        .show_commit
        .then_some(selected)
        .flatten()
        .filter(|id| commit_message.as_ref().map(|(cached, _)| cached) != Some(id));
    if changes_visible && selected.is_some() && changes.as_ref().map(|(cached, _, _)| *cached) != selected {
        app.changes_parent = 0;
    }
    let changes_to_load = changes_visible.then_some(selected).flatten().filter(|id| {
        changes
            .as_ref()
            .is_none_or(|(cached, parent, _)| cached != id || *parent != app.changes_parent)
    });
    if changes_to_load.is_some() {
        app.reset_changes_view();
    }
    if !app.show_commit || selected.is_none() {
        *commit_message = None;
    }
    if !app.show_changes || app.selected.is_none() {
        *changes = None;
    }
    if app.rows[start..end].iter().any(|row| !row.metadata_loaded)
        || message_to_load.is_some()
        || changes_to_load.is_some()
    {
        let mut one_shot_repository = None;
        let repository = if fill_repository.retain {
            match &mut fill_repository.retained {
                Some(repository) => repository,
                slot @ None => slot.insert(open_fill_repository(fill_repository.path)?),
            }
        } else {
            one_shot_repository.insert(open_fill_repository(fill_repository.path)?)
        };
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
        if let Some(id) = changes_to_load {
            repository.object_cache_size(OBJECT_CACHE_SIZE);
            let loaded = load_changes(
                repository,
                id,
                app.changes_parent,
                line_diff_pool
                    .as_mut()
                    .context("line diff pool is missing while the changes pane is visible")?,
            );
            repository.object_cache_size(None);
            let loaded = loaded?;
            app.changes_parent = loaded.parent.map_or(0, |parent| parent.index);
            *changes = Some((id, app.changes_parent, loaded));
        }
    }
    let message = commit_message.as_ref().map(|(_, message)| message.as_bstr());
    let changes = changes.as_ref().map(|(_, _, changes)| changes);
    terminal.draw(|frame| ui::draw(frame, app, decorations, mailmap, message, changes))?;
    Ok(())
}

fn open_fill_repository(repository_path: &Path) -> Result<gix::Repository> {
    let mut repository = gix::open(repository_path).context("could not open repository for history view")?;
    repository.object_cache_size(None);
    Ok(repository)
}

fn prepare_file_diff(
    repository_path: &Path,
    change: &gix::object::tree::diff::ChangeDetached,
    path: &PathChange,
) -> Result<FileDiff> {
    let mut repository = gix::open(repository_path).context("could not open repository for file diff")?;
    repository.object_cache_size(OBJECT_CACHE_SIZE);
    prepare_file_diff_with_repository(&repository, change, path)
}

fn prepare_file_diff_with_repository(
    repository: &gix::Repository,
    change: &gix::object::tree::diff::ChangeDetached,
    path: &PathChange,
) -> Result<FileDiff> {
    let global_command = repository
        .config_snapshot()
        .trusted_program(gix::config::tree::Diff::EXTERNAL)
        .map(gix::path::os_string_into_bstring)
        .transpose()
        .context("external diff command is not representable on this platform")?;
    let mut resources = repository
        .diff_resource_cache(
            gix::diff::blob::pipeline::Mode::ToGitUnlessBinaryToTextIsPresent,
            Default::default(),
        )
        .context("could not initialize file diff")?;
    resources.options.skip_internal_diff_if_external_is_configured = true;
    change
        .attach(repository, repository)
        .diff(&mut resources)
        .context("could not prepare selected file")?;
    let prepared = resources.prepare_diff().context("could not prepare selected diff")?;
    match prepared.operation {
        gix::diff::blob::platform::prepare_diff::Operation::ExternalCommand { command } => {
            let command = command.to_owned();
            prepare_external_diff(repository, &resources, command)
        }
        gix::diff::blob::platform::prepare_diff::Operation::InternalDiff { algorithm } => {
            if let Some(command) = global_command {
                return prepare_external_diff(repository, &resources, command);
            }
            let input = prepared.interned_input();
            let diff = gix::diff::blob::diff_with_slider_heuristics(algorithm, &input);
            let rendered = gix::diff::blob::UnifiedDiff::new(
                &diff,
                &input,
                gix::diff::blob::unified_diff::ConsumeBinaryHunk::new(BString::default(), "\n"),
                gix::diff::blob::unified_diff::ContextSize::symmetrical(3),
            )
            .consume()
            .context("could not render selected diff")?;
            prepare_pager(repository, built_in_diff(path, change, Some(rendered), false))
        }
        gix::diff::blob::platform::prepare_diff::Operation::SourceOrDestinationIsBinary => {
            prepare_pager(repository, built_in_diff(path, change, None, true))
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
) -> Result<FileDiff> {
    Ok(FileDiff::External(
        resources
            .prepare_diff_command(
                command,
                repository
                    .command_context()
                    .context("could not prepare external diff environment")?,
                0,
                1,
            )
            .context("could not prepare external diff command")?,
    ))
}

fn built_in_diff(
    path: &PathChange,
    change: &gix::object::tree::diff::ChangeDetached,
    rendered: Option<BString>,
    binary: bool,
) -> BuiltInDiff {
    use gix::object::tree::diff::ChangeDetached;

    let (old_path, new_path, old_mode, new_mode) = match change {
        ChangeDetached::Addition { entry_mode, .. } => (None, Some(path.path.as_bstr()), None, Some(*entry_mode)),
        ChangeDetached::Deletion { entry_mode, .. } => (Some(path.path.as_bstr()), None, Some(*entry_mode), None),
        ChangeDetached::Modification {
            previous_entry_mode,
            entry_mode,
            ..
        } => (
            Some(path.path.as_bstr()),
            Some(path.path.as_bstr()),
            Some(*previous_entry_mode),
            Some(*entry_mode),
        ),
        ChangeDetached::Rewrite {
            source_entry_mode,
            entry_mode,
            ..
        } => (
            path.source.as_ref().map(|path| path.as_bstr()),
            Some(path.path.as_bstr()),
            Some(*source_entry_mode),
            Some(*entry_mode),
        ),
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
        let max = diff.lines.len().saturating_sub(page);
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
    id: gix::ObjectId,
    requested_parent: usize,
    line_diff_pool: &mut LineDiffPool,
) -> Result<Changes> {
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
    let changes = repository
        .diff_tree_to_tree(old_tree.as_ref(), Some(&new_tree), None)
        .context("could not diff commit trees")?;
    let mut out = Changes {
        parent: (parents.len() > 1).then(|| ComparedParent {
            index: parent_index,
            total: parents.len(),
            id: parent.expect("a merge has parents").detach(),
        }),
        ..Changes::default()
    };
    let mut diffs = Vec::new();
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
            source,
            path,
            lines: None,
        });
        diffs.push(change);
    }
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
        KeyCode::Char('p') => Some(Action::CycleChangesParent),
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Esc => Some(Action::Cancel),
        KeyCode::Up | KeyCode::Char('k') => Some(Action::MoveUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::MoveDown),
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
        KeyCode::Char('d') => Some(Action::ToggleDate),
        KeyCode::Char('e') => Some(Action::ToggleEmail),
        KeyCode::Char('n') => Some(Action::ToggleName),
        KeyCode::Char('t') => Some(Action::ToggleTrailers),
        KeyCode::Char('m') => Some(Action::ToggleMailmap),
        KeyCode::Char('r') => Some(Action::ToggleRefs),
        KeyCode::Char('s') => Some(Action::VerifySignatures),
        KeyCode::Char('v') => Some(Action::ToggleHidden),
        KeyCode::Char('[') => Some(Action::ToggleAlign),
        KeyCode::Char(']' | 'o') => Some(Action::ToggleCommit),
        KeyCode::Char('Y') => Some(Action::CopyAuthor),
        KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::SHIFT) => Some(Action::CopyAuthor),
        KeyCode::Char('y') => Some(Action::Copy),
        _ => None,
    }
}

fn changes_focusable(changes: Option<&Changes>) -> bool {
    changes.is_some_and(Changes::is_visible)
}

fn repeats_viewport(action: &Action) -> bool {
    matches!(
        action,
        Action::MoveUp
            | Action::MoveDown
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

#[cfg(test)]
fn open_test_repository(path: impl AsRef<Path>) -> Result<gix::Repository, gix::open::Error> {
    gix::open_opts(path.as_ref(), gix::open::Options::isolated())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_commit_messages_from_an_existing_repository() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_read_only("history.sh")?;
        let repository = gix::open(&fixture)?;
        let id = repository.rev_parse_single("topic")?.detach();

        assert!(
            load_commit_message(&repository, id)?.starts_with(b"topic\n\n--- agent\n\nCo-authored-by:"),
            "on-demand loading retains the full commit message"
        );
        Ok(())
    }

    #[test]
    fn loads_changes_against_each_merge_parent() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_read_only("history.sh")?;
        let repository = gix::open_opts(&fixture, gix::open::Options::isolated())?;
        let mut line_diff_pool = None;
        sync_line_diff_pool(&mut line_diff_pool, true, &fixture, 2)?;
        assert_eq!(
            line_diff_pool.as_ref().map(|pool| pool.workers.len()),
            Some(2),
            "showing changes creates the requested worker pool"
        );
        sync_line_diff_pool(&mut line_diff_pool, false, &fixture, 2)?;
        assert!(line_diff_pool.is_none(), "hiding changes destroys the worker pool");
        sync_line_diff_pool(&mut line_diff_pool, true, &fixture, 2)?;
        let line_diff_pool = line_diff_pool
            .as_mut()
            .expect("showing changes recreates the worker pool");

        let root = load_changes(
            &repository,
            repository.rev_parse_single("v1^{}")?.detach(),
            0,
            line_diff_pool,
        )?;
        assert_eq!(
            root.paths,
            [PathChange {
                kind: ChangeKind::Added,
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

        let topic = load_changes(
            &repository,
            repository.rev_parse_single("topic")?.detach(),
            0,
            line_diff_pool,
        )?;
        assert_eq!(
            topic.paths,
            [
                PathChange {
                    kind: ChangeKind::Added,
                    source: None,
                    path: "topic".into(),
                    lines: Some((1, 0)),
                },
                PathChange {
                    kind: ChangeKind::Added,
                    source: None,
                    path: "topic-extra".into(),
                    lines: Some((1, 0)),
                }
            ],
            "parallel line diffs retain tree-diff order and status"
        );
        assert_eq!((topic.lines_added, topic.lines_removed), (2, 0));

        let merge = repository.rev_parse_single("main")?.detach();
        let first_parent = load_changes(&repository, merge, 0, line_diff_pool)?;
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
                source: None,
                path: "merged".into(),
                lines: Some((1, 0)),
            }],
            "the default merge diff compares the result to its first parent"
        );

        let second_parent = load_changes(&repository, merge, 1, line_diff_pool)?;
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
                source: None,
                path: "main".into(),
                lines: Some((1, 0)),
            }],
            "later parents can be selected independently"
        );
        assert_eq!(
            load_changes(&repository, merge, 2, line_diff_pool)?.parent,
            first_parent.parent,
            "parent selection wraps around"
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
    fn chooses_screen_from_terminal_and_history_height() {
        assert_eq!(
            inline_height(Screen::Auto, 20, 7),
            Some(10),
            "short histories occupy only their rows, spacers, and footer"
        );
        assert_eq!(
            inline_height(Screen::Auto, 20, 8),
            Some(11),
            "spacers do not force an otherwise short history into the alternate screen"
        );
        assert_eq!(
            inline_height(Screen::Auto, 20, 10),
            None,
            "the auto cutoff remains half the terminal height"
        );
        assert_eq!(
            inline_height(Screen::Half, 21, 3),
            Some(6),
            "half mode shrinks to the rows, spacers, and footer needed by short histories"
        );
        assert_eq!(
            inline_height(Screen::Half, 21, 10),
            Some(10),
            "half mode is capped at half the terminal, rounded down"
        );
        assert_eq!(
            inline_height(Screen::Half, 21, 0),
            Some(3),
            "an empty history only needs its spacers and footer"
        );
        assert_eq!(
            inline_height(Screen::Always, 20, 0),
            None,
            "always mode uses the alternate screen"
        );
    }

    #[test]
    fn switches_screens_for_inline_commit_panes_and_large_histories() {
        let mut inline = App::new(1);
        configure_initial_screen(&mut inline, true);
        assert!(inline.inline);
        assert!(!inline.show_changes, "inline startup hides the default changes view");
        let mut alternate = App::new(1);
        configure_initial_screen(&mut alternate, false);
        assert!(!alternate.inline);
        assert!(
            alternate.show_changes,
            "alternate-screen startup keeps the default changes view"
        );

        assert!(
            should_switch_screen(true, true, false),
            "opening the commit pane from inline mode enters the alternate screen"
        );
        assert!(
            should_switch_screen(true, false, true),
            "closing the commit pane returns to inline mode"
        );
        assert!(
            !should_switch_screen(false, true, true),
            "a session that started in the alternate screen stays there"
        );
        assert!(
            !should_switch_screen(true, true, true),
            "an already-active alternate screen is not re-entered"
        );
        assert!(!history_needs_alternate_screen(Screen::Auto, 20, 7));
        assert!(!history_needs_alternate_screen(Screen::Auto, 20, 8));
        assert!(history_needs_alternate_screen(Screen::Auto, 20, 10));
        assert!(
            needs_alternate_screen(false, false, None),
            "current terminal geometry overrides a stale history-fit flag"
        );
        assert!(
            !needs_alternate_screen(false, false, Some(11)),
            "a fitting current layout may return to inline mode"
        );
        assert!(
            !history_needs_alternate_screen(Screen::Half, 20, usize::MAX),
            "half-screen mode never switches because history grows"
        );
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
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE)),
            Some(Action::ToggleDate)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE)),
            Some(Action::ToggleEmail)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE)),
            Some(Action::ToggleName)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE)),
            Some(Action::ToggleTrailers)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE)),
            Some(Action::ToggleMailmap)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE)),
            Some(Action::ToggleRefs)
        );
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE)),
            Some(Action::ToggleHidden)
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
        assert_eq!(
            action(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE)),
            Some(Action::ToggleCommit)
        );
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
    fn only_visible_changes_can_take_focus() {
        assert!(!changes_focusable(None));
        assert!(!changes_focusable(Some(&Changes::default())));
        let changes = Changes {
            paths: vec![PathChange {
                kind: ChangeKind::Modified,
                source: None,
                path: "file".into(),
                lines: None,
            }],
            ..Changes::default()
        };
        assert!(changes_focusable(Some(&changes)));
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
    fn prepares_a_reduced_selection_after_leaving_the_alternate_screen() {
        let mut app = App::new(1);
        app.show_commit = true;
        app.show_changes = true;
        app.changes_focused = true;

        prepare_inline_exit(&mut app);

        assert!(app.inline, "the final frame is drawn into the restored inline screen");
        assert!(
            !app.show_commit && !app.show_changes,
            "alternate-screen panels are omitted from the final frame"
        );
        assert!(!app.changes_focused, "the hidden panel no longer owns focus");
        assert!(!app.show_selection_tail, "only the left selection marker remains");
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
}
