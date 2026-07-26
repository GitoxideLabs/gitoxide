//! A fast, interactive commit graph for terminals.

#![forbid(unsafe_code)]

mod app;
mod history;
mod ui;

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use app::{Action, App, CommitRow, Effect, State};
use crossterm::{
    clipboard::CopyToClipboard,
    cursor,
    event::{
        self, DisableFocusChange, EnableFocusChange, Event as TerminalEvent, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers, KeyboardEnhancementFlags, ModifierKeyCode, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    execute,
    style::Print,
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use gix::bstr::{BString, ByteSlice};
use history::{Authors, Decorations, Event, SharedAuthors};
use ratatui::{TerminalOptions, Viewport, backend::CrosstermBackend};

const EVENT_BATCH_SIZE: usize = 256;
const OBJECT_CACHE_SIZE: usize = 4 * 1024 * 1024;
const FRAME_INTERVAL: Duration = Duration::from_nanos(16_666_667);

struct FillRepository<'a> {
    path: &'a Path,
    retained: Option<gix::Repository>,
    retain: bool,
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

fn history_needs_alternate_screen(screen: Screen, terminal_height: u16, commits: usize) -> bool {
    screen == Screen::Auto && inline_height(screen, terminal_height, commits).is_none()
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
    let needs_alternate_screen = app.show_commit || history_requires_alternate_screen;
    if !should_switch_screen(started_inline, needs_alternate_screen, inline_terminal.is_some()) {
        if started_inline && app.inline && resize_inline {
            let height = inline_height(screen, terminal::size()?.1, app.rows.len())
                .expect("an inline history always has an inline height");
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
        let height = inline_height(screen, terminal::size()?.1, app.rows.len())
            .expect("an inline history always has an inline height");
        resize_inline_screen(terminal, height).context("could not resize the inline history")?;
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
    let mut fill_repository = FillRepository {
        path: &repository_path,
        retained: None,
        retain: false,
    };
    app.inline = started_inline;
    app.has_hidden_filter = !hide.is_empty();
    let mut decorations = Decorations::new();
    draw(
        terminal,
        &mut app,
        &decorations,
        &mailmap,
        &authors,
        &mut fill_repository,
        &mut commit_message,
    )?;
    let mut last_draw = Instant::now();
    let mut dirty = false;
    let mut urgent = false;
    let mut inline_terminal = None;
    let mut history_requires_alternate_screen = false;
    let mut focused = true;
    let result: Result<Option<Duration>> = (|| loop {
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
            )?;
            last_draw = Instant::now();
            dirty = false;
            urgent = false;
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
            )?;
            last_draw = Instant::now();
            dirty = false;
        }
        let terminal_event = match poll_timeout(streaming, events, dirty, last_draw.elapsed()) {
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
        fill_repository.retain = retains_fill_repository(key.kind, action.as_ref());
        if !fill_repository.retain {
            fill_repository.retained = None;
        }
        let Some(action) = action else {
            continue;
        };
        dirty = true;
        urgent = true;
        let effects = app.update(action);
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
                Effect::VerifySignatures(ids) => {
                    verification_receiver = Some(start_signature_verification(repository_path.clone(), ids));
                }
                Effect::Quit => {
                    if app.inline {
                        app.show_selection_tail = false;
                        draw(
                            terminal,
                            &mut app,
                            &decorations,
                            &mailmap,
                            &authors,
                            &mut fill_repository,
                            &mut commit_message,
                        )?;
                    }
                    return Ok(None);
                }
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
    Ok(outcome)
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

fn draw(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    decorations: &Decorations,
    mailmap: &gix::mailmap::Snapshot,
    authors: &SharedAuthors,
    fill_repository: &mut FillRepository<'_>,
    commit_message: &mut Option<(gix::ObjectId, BString)>,
) -> Result<()> {
    app.viewport_rows = terminal
        .get_frame()
        .area()
        .height
        .saturating_sub(1 + 2 * u16::from(app.inline)) as usize;
    app.ensure_visible();
    let start = app.offset.min(app.rows.len());
    let end = start.saturating_add(app.viewport_rows).min(app.rows.len());
    let selected = app
        .show_commit
        .then(|| app.selected.and_then(|index| app.rows.get(index)).map(|row| row.id))
        .flatten();
    let message_to_load = selected.filter(|id| commit_message.as_ref().map(|(cached, _)| cached) != Some(id));
    if selected.is_none() {
        *commit_message = None;
    }
    if app.rows[start..end].iter().any(|row| !row.metadata_loaded) || message_to_load.is_some() {
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
    }
    let message = commit_message.as_ref().map(|(_, message)| message.as_bstr());
    terminal.draw(|frame| ui::draw(frame, app, decorations, mailmap, message))?;
    Ok(())
}

fn open_fill_repository(repository_path: &Path) -> Result<gix::Repository> {
    let mut repository = gix::open(repository_path).context("could not open repository for history view")?;
    repository.object_cache_size(None);
    Ok(repository)
}

fn load_commit_message(repository: &gix::Repository, id: gix::ObjectId) -> Result<BString> {
    let commit = repository.find_commit(id).context("could not load commit message")?;
    Ok(commit.message_raw_sloppy().to_owned())
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

fn poll_timeout(streaming: bool, events: usize, dirty: bool, since_draw: Duration) -> Option<Duration> {
    streaming.then(|| {
        if events == EVENT_BATCH_SIZE {
            Duration::ZERO
        } else if dirty {
            FRAME_INTERVAL.saturating_sub(since_draw)
        } else {
            FRAME_INTERVAL
        }
    })
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
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::Quit),
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

fn retains_fill_repository(kind: KeyEventKind, action: Option<&Action>) -> bool {
    kind == KeyEventKind::Repeat && action.is_some_and(repeats_viewport)
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
            !history_needs_alternate_screen(Screen::Half, 20, usize::MAX),
            "half-screen mode never switches because history grows"
        );
    }

    #[test]
    fn maps_navigation_and_control_c() {
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
            Some(Action::Quit)
        );
        assert_eq!(action(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)), None);
    }

    #[test]
    fn retains_the_fill_repository_only_for_repeated_viewport_navigation() {
        assert!(retains_fill_repository(KeyEventKind::Repeat, Some(&Action::MoveDown)));
        assert!(!retains_fill_repository(KeyEventKind::Press, Some(&Action::MoveDown)));
        assert!(!retains_fill_repository(KeyEventKind::Release, Some(&Action::MoveDown)));
        assert!(!retains_fill_repository(
            KeyEventKind::Repeat,
            Some(&Action::ScrollRight)
        ));
        assert!(!retains_fill_repository(
            KeyEventKind::Repeat,
            Some(&Action::ToggleDate)
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
            poll_timeout(false, 0, false, Duration::ZERO),
            None,
            "idle waits reactively for terminal input"
        );
        assert_eq!(
            poll_timeout(true, EVENT_BATCH_SIZE, true, Duration::ZERO),
            Some(Duration::ZERO),
            "saturated history batches keep processing"
        );
        assert_eq!(
            poll_timeout(true, 1, true, Duration::from_millis(10)),
            Some(FRAME_INTERVAL.saturating_sub(Duration::from_millis(10))),
            "dirty streaming frames wait only until their deadline"
        );
    }
}
