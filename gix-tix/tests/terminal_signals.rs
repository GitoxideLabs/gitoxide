#![cfg(all(feature = "sha1", any(target_os = "linux", target_os = "macos")))]

use std::{
    fs::File,
    io::{self, Read, Write},
    os::unix::process::CommandExt,
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

use gix_testtools::{git, tempfile::TempDir};
use rustix::{
    fs::{Mode, OFlags},
    process::{Pid, Signal},
    pty::OpenptFlags,
    termios::{LocalModes, SpecialCodeIndex, Termios, Winsize},
};

const WAIT: Duration = Duration::from_secs(10);
const PTY_CHILD: &str = "TIX_TEST_PTY_CHILD";
const NO_CONTROLLING_TERMINAL: &str = "TIX_TEST_NO_CONTROLLING_TERMINAL";

#[test]
fn normal_quit_restores_the_terminal() -> gix_error::TestResult {
    check_shutdown(None, View::Graph)
}

#[test]
fn disabled_graphics_keeps_the_text_gutter() -> gix_error::TestResult {
    let repository = fixture()?;
    let (mut terminal, _) = Pty::spawn(repository.path(), true)?;
    // Ghostty with image-storage-limit=0 still advertises itself, but ignores graphics queries.
    terminal.graphics = false;
    terminal.wait_for(|output| contains(output, b"second") && contains(output, "●".as_bytes()))?;
    assert!(
        contains(&terminal.output, b"\x1b_Ga=q,"),
        "startup probes the advertised graphics protocol"
    );
    assert!(
        !contains(&terminal.output, b"\x1b_Ga=t,") && !contains(&terminal.output, b"\x1b_Ga=p,"),
        "a terminal that never acknowledges graphics retains the text graph without image uploads or placements"
    );
    terminal.master.write_all(b"qq")?;
    assert!(terminal.wait_for_exit()?.success(), "the text view exits normally");
    Ok(())
}

#[test]
fn sigterm_restores_an_idle_terminal() -> gix_error::TestResult {
    check_shutdown(Some(Signal::TERM), View::Graph)
}

#[test]
fn sigint_restores_an_idle_terminal() -> gix_error::TestResult {
    check_shutdown(Some(Signal::INT), View::Graph)
}

#[test]
fn sighup_restores_an_idle_terminal() -> gix_error::TestResult {
    check_shutdown(Some(Signal::HUP), View::Graph)
}

#[test]
fn sigquit_restores_an_idle_terminal() -> gix_error::TestResult {
    check_shutdown(Some(Signal::QUIT), View::Graph)
}

#[test]
fn sigterm_restores_the_terminal_from_the_builtin_diff() -> gix_error::TestResult {
    check_shutdown(Some(Signal::TERM), View::BuiltinDiff)
}

#[test]
fn sigint_restores_the_terminal_while_acknowledging_an_external_pager() -> gix_error::TestResult {
    check_shutdown(Some(Signal::INT), View::PagerAcknowledgement)
}

#[test]
fn sigterm_restores_the_terminal_from_the_structural_diff() -> gix_error::TestResult {
    check_shutdown(Some(Signal::TERM), View::StructuralDiff)
}

#[test]
fn sigint_restores_the_terminal_from_the_structural_diff() -> gix_error::TestResult {
    check_shutdown(Some(Signal::INT), View::StructuralDiff)
}

#[test]
fn tab_reopens_the_file_tree_beside_a_scrolled_structural_diff() -> gix_error::TestResult {
    let repository = structural_fixture()?;
    let (mut terminal, original) = Pty::spawn(repository.path(), true)?;
    terminal.wait_for(|output| contains(output, b"second") && contains(output, b"\x1b_Ga=p,"))?;
    let opened = terminal.output.len();
    terminal.master.write_all(b"\x1b[13;2u")?;
    terminal.wait_for(|output| contains(&output[opened..], b"declaration renames:on"))?;
    let scrolled = terminal.output.len();
    terminal.master.write_all(b"\x1b[F")?;
    terminal.wait_for(|output| contains(&output[scrolled..], b"69"))?;
    let shown = terminal.output.len();
    terminal.master.write_all(b"\t")?;
    terminal.wait_for(|output| contains(&output[shown..], b"Files") && contains(&output[shown..], b"\x1b_Ga=p,"))?;
    let clicked = terminal.output.len();
    terminal.master.write_all(b"\x1b[<0;8;4M\x1b[<0;8;4m")?;
    terminal.wait_for(|output| contains(&output[clicked..], b"u32"))?;
    let hidden = terminal.output.len();
    terminal.master.write_all(b"\t")?;
    terminal.wait_for(|output| contains(&output[hidden..], b"Structural"))?;
    terminal.master.write_all(b"qqq")?;
    assert!(
        terminal.wait_for_exit()?.success(),
        "panel navigation returns to history and exits normally"
    );
    assert_eq!(
        rustix::termios::tcgetattr(&terminal.master)?.local_modes,
        original.local_modes,
        "panel navigation restores terminal modes"
    );
    Ok(())
}

#[test]
fn mouse_clicks_and_wheels_navigate_the_structural_diff() -> gix_error::TestResult {
    let repository = structural_fixture()?;
    let (mut terminal, original) = Pty::spawn(repository.path(), true)?;
    terminal.wait_for(|output| contains(output, b"second") && contains(output, b"\x1b_Ga=p,"))?;
    let opened = terminal.output.len();
    terminal.master.write_all(b"\x1b[13;2u")?;
    terminal.wait_for(|output| contains(&output[opened..], b"declaration renames:on"))?;
    let hovered = terminal.output.len();
    terminal.master.write_all(b"\x1b[<35;5;3M")?;
    terminal.wait_for(|output| contains(&output[hovered..], b"+71 -1"))?;
    let clicked = terminal.output.len();
    terminal.master.write_all(b"\x1b[<0;8;5M\x1b[<0;8;5m")?;
    // Cell diffs can reuse the file-header prefix; the jump reveals later declarations.
    terminal.wait_for(|output| contains(&output[clicked..], b"added_016"))?;
    for (wheel, expected) in [
        (b"\x1b[<65;10;10M".as_slice(), b"added_".as_slice()),
        (b"\x1b[<64;10;10M".as_slice(), b"100644".as_slice()),
    ] {
        let scrolled = terminal.output.len();
        for _ in 0..10 {
            // Drain each redraw rather than overflowing the nonblocking PTY input queue.
            let redraw = terminal.output.len();
            terminal.master.write_all(wheel)?;
            terminal.wait_for(|output| output.len() > redraw)?;
        }
        terminal.wait_for(|output| contains(&output[scrolled..], expected))?;
    }
    terminal.master.write_all(b"qqq")?;
    assert!(
        terminal.wait_for_exit()?.success(),
        "mouse navigation returns to history and exits normally"
    );
    assert_eq!(
        rustix::termios::tcgetattr(&terminal.master)?.local_modes,
        original.local_modes,
        "mouse navigation preserves terminal restoration"
    );
    Ok(())
}

#[test]
fn shift_enter_scrolls_the_continuous_structural_diff_and_keeps_live_settings() -> gix_error::TestResult {
    let repository = structural_fixture()?;
    std::fs::write(repository.path().join("src/api.rs"), "fn worktree_only() {}\n")?;
    configure_difftool(repository.path(), "printf TIX_UNEXPECTED_TOOL; exit 99")?;
    git(
        repository.path(),
        "config core.pager 'cat; printf TIX_TEST_LEGACY_PAGER'",
    )?;
    let (mut terminal, original) = Pty::spawn(repository.path(), true)?;
    terminal.wait_for(|output| contains(output, b"second") && contains(output, b"\x1b_Ga=p,"))?;
    let view_start = terminal.output.len();
    terminal.master.write_all(b"\x1b[13;2u")?;
    terminal.wait_for(|output| contains(&output[view_start..], b"r declaration renames:on"))?;
    let first_page = &terminal.output[view_start..];
    assert!(contains(first_page, b"src/"), "the file tree starts the built-in view");
    assert!(
        contains(first_page, b"c context:0"),
        "the first structural view starts with zero context"
    );
    assert!(
        contains(first_page, b"---") && contains(first_page, b"+++") && contains(first_page, b"b/src/api.rs"),
        "patch headers follow the file tree on the same page"
    );
    assert!(
        !contains(first_page, b"TIX_UNEXPECTED_TOOL"),
        "the structural view does not launch the configured difftool"
    );
    assert!(
        !contains(first_page, b"TIX_TEST_LEGACY_PAGER"),
        "the structural view owns the TUI"
    );
    assert!(
        !contains(first_page, b"worktree_only"),
        "history comparisons use committed resources"
    );
    assert!(
        !contains(first_page, b"secret_"),
        "structural output keeps implementation text hidden"
    );
    let last_page = terminal.output.len();
    terminal.master.write_all(b"\x1b[F")?;
    // The last row reuses the already drawn 'added_0' prefix and only writes '69'.
    terminal.wait_for(|output| contains(&output[last_page..], b"69"))?;
    let first_page = terminal.output.len();
    terminal.master.write_all(b"\x1b[H")?;
    terminal.wait_for(|output| contains(&output[first_page..], b"files ("))?;
    let words_changed = terminal.output.len();
    terminal.master.write_all(b"w")?;
    // Syntax styles split declarations into several writes; the footer changes on -> off.
    terminal.wait_for(|output| contains(&output[words_changed..], b"ff"))?;
    let renames_changed = terminal.output.len();
    terminal.master.write_all(b"r")?;
    // Cell diffs emit the changed suffix of 'on' -> 'off', not the entire footer.
    terminal.wait_for(|output| contains(&output[renames_changed..], b"ff"))?;
    let back = terminal.output.len();
    terminal.master.write_all(b"cq")?;
    terminal.wait_for(|output| contains(&output[back..], b"second"))?;
    let reopened = terminal.output.len();
    terminal.master.write_all(b"\x1b[13;2u")?;
    terminal.wait_for(|output| contains(&output[reopened..], b"declaration renames:off"))?;
    assert!(
        contains(&terminal.output[reopened..], b"words:off"),
        "settings persist when reopening the view"
    );
    assert!(
        contains(&terminal.output[reopened..], b"c context:1"),
        "the context key is routed to structural mode and persists when reopening"
    );
    terminal.master.write_all(b"\r")?;
    let legacy_start = terminal.output.len();
    terminal.master.write_all(b"\r")?;
    terminal.wait_for(|output| contains(&output[legacy_start..], b"TIX_TEST_LEGACY_PAGER"))?;
    terminal.master.write_all(b"\rqq")?;
    assert!(
        terminal.wait_for_exit()?.success(),
        "ordinary Enter still uses the configured pager and returns to Tix"
    );
    let restored = rustix::termios::tcgetattr(&terminal.master)?;
    assert_eq!(
        restored.local_modes, original.local_modes,
        "normal exit restores terminal modes"
    );
    Ok(())
}

#[test]
fn production_only_shortcut_filters_the_tree_and_survives_reopening() -> gix_error::TestResult {
    let repository = gix_testtools::scripted_fixture_writable("structural-diff-production.sh")?;
    let (mut terminal, original) = Pty::spawn(repository.path(), true)?;
    terminal.wait_for(|output| contains(output, b"second") && contains(output, b"\x1b_Ga=p,"))?;
    let opened = terminal.output.len();
    terminal.master.write_all(b"\x1b[13;2u")?;
    terminal.wait_for(|output| contains(&output[opened..], b"p production only:off"))?;
    assert!(
        contains(&terminal.output[opened..], b"integration.rs"),
        "the initial tree contains test files"
    );
    let last_page = terminal.output.len();
    terminal.master.write_all(b"\x1b[F")?;
    terminal.wait_for(|output| contains(&output[last_page..], b"Outline unavailable:"))?;
    let filtered = terminal.output.len();
    terminal.master.write_all(b"pq")?;
    terminal.wait_for(|output| contains(&output[filtered..], b"second"))?;
    let reopened = terminal.output.len();
    terminal.master.write_all(b"\x1b[13;2u")?;
    terminal.wait_for(|output| contains(&output[reopened..], b"p production only:on"))?;
    let output = &terminal.output[reopened..];
    assert!(
        contains(output, b"Changed files (5)"),
        "the reopened tree counts only production changes"
    );
    // History redraws can precede the diff; its count identifies the filtered screen.
    assert!(
        !contains(&terminal.output[filtered..], b"\x1b]21"),
        "filtering and reopening never query terminal colors"
    );
    let back = terminal.output.len();
    terminal.master.write_all(b"pq")?;
    terminal.wait_for(|output| contains(&output[back..], b"second"))?;
    let restored = terminal.output.len();
    terminal.master.write_all(b"\x1b[13;2u")?;
    terminal.wait_for(|output| contains(&output[restored..], b"p production only:off"))?;
    assert!(
        contains(&terminal.output[restored..], b"Changed files (13)"),
        "p restores the complete file tree"
    );
    assert!(
        contains(&terminal.output[restored..], b"integration.rs"),
        "p restores hidden files from the cached diff"
    );
    terminal.master.write_all(b"qqq")?;
    assert!(terminal.wait_for_exit()?.success(), "filtered views exit normally");
    assert_eq!(
        rustix::termios::tcgetattr(&terminal.master)?.local_modes,
        original.local_modes,
        "production filtering restores terminal modes"
    );
    Ok(())
}

#[test]
fn empty_commits_open_an_empty_structural_view() -> gix_error::TestResult {
    let repository = fixture()?;
    git(repository.path(), "commit --allow-empty -m empty")?;
    let (mut terminal, original) = Pty::spawn(repository.path(), true)?;
    terminal.wait_for(|output| contains(output, b"empty") && contains(output, b"\x1b_Ga=p,"))?;
    let start = terminal.output.len();
    terminal.master.write_all(b"\x1b[13;2u")?;
    terminal.wait_for(|output| contains(&output[start..], b"files (0)"))?;
    assert!(
        contains(&terminal.output[start..], b"changes."),
        "the empty comparison explains that there are no file changes"
    );
    terminal.master.write_all(b"qqq")?;
    assert!(
        terminal.wait_for_exit()?.success(),
        "the empty view returns to the graph and exits normally"
    );
    assert_eq!(
        rustix::termios::tcgetattr(&terminal.master)?.local_modes,
        original.local_modes,
        "empty views retain terminal ownership correctly"
    );
    Ok(())
}

#[test]
fn text_only_structural_diffs_use_the_light_terminal_theme() -> gix_error::TestResult {
    let repository = structural_fixture()?;
    let (mut terminal, original) = Pty::spawn(repository.path(), true)?;
    terminal.graphics = false;
    terminal.foreground = [24, 28, 32];
    terminal.background = [240, 244, 248];
    terminal.palette = [[120; 3]; 16];
    terminal.palette[1] = [180, 24, 40];
    terminal.palette[2] = [24, 150, 80];
    terminal.palette[4] = [30, 70, 180];
    terminal.wait_for(|output| contains(output, b"second") && contains(output, "●".as_bytes()))?;
    let palette_query = terminal
        .output
        .windows(b"\x1b]21".len())
        .position(|bytes| bytes == b"\x1b]21")
        .expect("text-only views query the Kitty color palette");
    let graphics_query = terminal
        .output
        .windows(b"\x1b_Ga=q".len())
        .position(|bytes| bytes == b"\x1b_Ga=q")
        .expect("the fixture advertises the graphics protocol");
    assert!(
        palette_query < graphics_query,
        "graphics detection cannot consume the color-query budget"
    );
    let view_start = terminal.output.len();
    terminal.master.write_all(b"\x1b[13;2u")?;
    terminal.wait_for(|output| contains(&output[view_start..], b"r declaration renames:on"))?;
    let view_output = &terminal.output[view_start..];
    assert!(
        emitted_rgb_colors(view_output, "38;2;").contains(&terminal.foreground),
        "diff text uses the sampled terminal foreground: {:?}",
        emitted_rgb_colors(view_output, "38;2;")
    );
    let backgrounds = emitted_rgb_colors(view_output, "48;2;");
    assert!(!backgrounds.is_empty(), "structural changes have RGB background tints");
    assert!(
        backgrounds
            .iter()
            .all(|color| color.iter().all(|channel| *channel > 130)),
        "a light theme never receives dark diff backgrounds: {backgrounds:?}"
    );
    let scrolled = terminal.output.len();
    terminal.master.write_all(b"\x1b[F")?;
    terminal.wait_for(|output| contains(&output[scrolled..], b"69"))?;
    assert!(
        !contains(&terminal.output[scrolled..], b"\x1b]21"),
        "scrolling never queries terminal colors"
    );
    terminal.master.write_all(b"qqq")?;
    assert!(terminal.wait_for_exit()?.success(), "the colored view exits normally");
    assert_eq!(
        rustix::termios::tcgetattr(&terminal.master)?.local_modes,
        original.local_modes,
        "theme detection and diff rendering restore terminal modes"
    );
    Ok(())
}

#[test]
fn terminal_hangup_exits_without_a_signal() -> gix_error::TestResult {
    let repository = fixture()?;
    // Without a controlling terminal, closing the master generates no SIGHUP.
    let (mut terminal, _) = Pty::spawn(repository.path(), false)?;
    terminal.wait_for(|output| contains(output, b"second") && contains(output, b"\x1b[?25l"))?;
    let idle_until = Instant::now() + Duration::from_millis(300);
    while Instant::now() < idle_until {
        terminal.pump()?;
        std::thread::sleep(Duration::from_millis(10));
    }
    drop(std::mem::replace(&mut terminal.master, File::open("/dev/null")?));
    let status = terminal.wait_for_exit()?;
    assert_eq!(
        status.code(),
        Some(1),
        "closed input must exit with an error, without a signal: {status}"
    );
    let home = repository.path().join(".git/tix-test-home");
    #[cfg(target_os = "macos")]
    let directory = home.join("Library/Logs/org.GitoxideLabs.tix");
    #[cfg(target_os = "linux")]
    let directory = home.join("state/tix");
    let entry = std::fs::read_dir(directory)?
        .next()
        .expect("the invocation creates a daily log")?;
    let log = std::fs::read_to_string(entry.path())?;
    assert!(
        log.contains("tix invocation failed"),
        "the input error is recorded after terminal teardown: {log}"
    );
    Ok(())
}

// Re-execution gives the child a controlling terminal without unsafe pre_exec/fork code.
#[test]
#[ignore = "entry point for the parent PTY tests"]
fn launch_in_pty() -> gix_error::TestResult {
    if std::env::var_os(PTY_CHILD).is_none() {
        return Ok(());
    }
    rustix::process::setsid()?;
    if std::env::var_os(NO_CONTROLLING_TERMINAL).is_none() {
        rustix::process::ioctl_tiocsctty(io::stdin())?;
    }
    rustix::process::setrlimit(
        rustix::process::Resource::Core,
        rustix::process::Rlimit {
            current: Some(0),
            maximum: Some(0),
        },
    )?;
    let mut command = Command::new(env!("CARGO_BIN_EXE_tix"));
    let current_dir = std::env::current_dir()?;
    let home = current_dir.join(".git/tix-test-home");
    gix_testtools::configure_git_environment(&mut command, &current_dir)
        .env("HOME", &home)
        .env("XDG_STATE_HOME", home.join("state"))
        .arg("--trace");
    Err(command.exec().into())
}

enum View {
    Graph,
    BuiltinDiff,
    PagerAcknowledgement,
    StructuralDiff,
}

fn check_shutdown(signal: Option<Signal>, view: View) -> gix_error::TestResult {
    let repository = fixture()?;
    if matches!(view, View::PagerAcknowledgement) {
        git(
            repository.path(),
            "config core.pager 'cat; printf TIX_TEST_PAGER_FINISHED'",
        )?;
    }
    let (mut terminal, original) = Pty::spawn(repository.path(), true)?;
    terminal.wait_for(|output| contains(output, b"\x1b_Ga=p,") && contains(output, b"\x1b[?2026l"))?;
    let viewer_start = terminal.output.len();
    if !matches!(view, View::Graph) {
        terminal.master.write_all(if matches!(view, View::StructuralDiff) {
            b"\t\x1b[13;2u"
        } else {
            b"\t\r"
        })?;
        let marker = match view {
            View::BuiltinDiff => b"/q/Esc back".as_slice(),
            View::PagerAcknowledgement => b"TIX_TEST_PAGER_FINISHED",
            View::StructuralDiff => b"r declaration renames:on",
            View::Graph => unreachable!("only diff views need more input"),
        };
        terminal.wait_for(|output| contains(output, marker))?;
    }
    // Leave the UI waiting for input; termination must not need another keystroke.
    let idle_until = Instant::now() + Duration::from_millis(300);
    while Instant::now() < idle_until {
        terminal.pump()?;
        std::thread::sleep(Duration::from_millis(10));
    }
    let raw = rustix::termios::tcgetattr(&terminal.master)?;
    assert!(
        !raw.local_modes
            .intersects(LocalModes::ICANON | LocalModes::ECHO | LocalModes::ISIG),
        "the regression must exercise an initialized raw terminal"
    );
    if matches!(view, View::PagerAcknowledgement) {
        let pager_output = &terminal.output[viewer_start..];
        assert!(contains(pager_output, b"\x1b[?2004l"), "the pager suspended UI input");
        assert!(
            !contains(pager_output, b"\x1b[?2004h"),
            "the fast pager is still waiting for acknowledgement, before resuming UI input"
        );
    }
    let shutdown_start = terminal.output.len();
    if let Some(signal) = signal {
        rustix::process::kill_process(Pid::from_child(&terminal.child), signal)?;
    } else {
        // The first q closes the initial changes pane; the second exits.
        terminal.master.write_all(b"qq")?;
    }
    let status = terminal.wait_for_exit()?;
    assert!(status.success(), "{signal:?} exits cleanly: {status}");
    let shutdown = &terminal.output[shutdown_start..];
    for sequence in [
        b"\x1b[?1049l".as_slice(),
        b"\x1b[?1000l",
        b"\x1b[?1002l",
        b"\x1b[?1003l",
        b"\x1b[?1006l",
        b"\x1b[?1015l",
        b"\x1b[?2004l",
        b"\x1b[?1004l",
        b"\x1b[?25h",
        b"\x1b]22;text\x1b\\\x1b]22;\x1b\\",
    ] {
        assert!(
            contains(shutdown, sequence),
            "{signal:?} must restore {:?} after exiting with {status}",
            String::from_utf8_lossy(sequence)
        );
    }
    assert_eq!(
        contains(shutdown, b"\x1b[<1u"),
        contains(&terminal.output[..shutdown_start], b"\x1b[>11u"),
        "shutdown restores the keyboard stack when enhanced input was negotiated"
    );
    let output = String::from_utf8_lossy(&terminal.output);
    let image_numbers: Vec<_> = output
        .split("\x1b_Ga=t,t=d,f=32,I=")
        .skip(1)
        .filter_map(|upload| upload.split_once(',').map(|(number, _)| number))
        .collect();
    assert!(!image_numbers.is_empty(), "the graph uploaded owned Kitty images");
    for number in image_numbers {
        assert!(
            output.contains(&format!("\x1b_Ga=d,d=N,I={number},")),
            "{signal:?} must delete owned image {number}"
        );
    }
    // macOS detaches the slave at exit, but the master still exposes its termios.
    let restored = rustix::termios::tcgetattr(&terminal.master)?;
    assert_eq!(restored.input_modes, original.input_modes, "input modes are restored");
    assert_eq!(
        restored.output_modes, original.output_modes,
        "output modes are restored"
    );
    assert_eq!(
        restored.control_modes, original.control_modes,
        "control modes are restored"
    );
    assert_eq!(restored.local_modes, original.local_modes, "local modes are restored");
    for code in [SpecialCodeIndex::VMIN, SpecialCodeIndex::VTIME] {
        assert_eq!(
            restored.special_codes[code], original.special_codes[code],
            "raw-mode read settings are restored"
        );
    }
    Ok(())
}

fn fixture() -> gix_error::TestResult<TempDir> {
    let repository = TempDir::new()?;
    let path = repository.path();
    git(path, "init --initial-branch=main")?;
    git(path, "config core.pager cat")?;
    // The tip has a real file diff, so both the graph and built-in viewer own terminal state.
    std::fs::write(path.join("tracked.txt"), "before\n")?;
    git(path, "add tracked.txt")?;
    git(path, "commit -m first")?;
    std::fs::write(path.join("tracked.txt"), "after\n")?;
    git(path, "commit -am second")?;
    Ok(repository)
}

fn structural_fixture() -> gix_error::TestResult<TempDir> {
    use std::fmt::Write as _;

    let repository = fixture()?;
    let path = repository.path();
    git(path, "checkout --detach HEAD^")?;
    std::fs::create_dir_all(path.join("src"))?;
    std::fs::write(
        path.join("src/api.rs"),
        "pub fn old(value: i32) -> i32 { let secret_before = value + 1; secret_before }\n",
    )?;
    git(path, "add src/api.rs")?;
    git(path, "commit --amend --no-edit")?;
    git(path, "checkout -B main")?;
    std::fs::write(
        path.join("src/api.rs"),
        "pub fn new(value: u32) -> u32 { let secret_after = value + 2; secret_after }\n",
    )?;
    std::fs::write(path.join("tracked.txt"), "after\n")?;
    let mut declarations = String::new();
    for index in 0..70 {
        writeln!(declarations, "fn added_{index:03}() {{ secret_body(); }}")?;
    }
    std::fs::write(path.join("src/many.rs"), declarations)?;
    git(path, "add .")?;
    git(path, "commit -m second")?;
    Ok(repository)
}

fn configure_difftool(repository: &Path, command: &str) -> gix_error::TestResult {
    git(repository, "config diff.tool capture")?;
    let output = gix_testtools::git_command(repository)
        .args(["config", "difftool.capture.cmd", command])
        .output()?;
    assert!(
        output.status.success(),
        "fixture tool configuration succeeds: {output:?}"
    );
    Ok(())
}

struct Pty {
    child: Child,
    master: File,
    output: Vec<u8>,
    parsed: usize,
    graphics: bool,
    foreground: [u8; 3],
    background: [u8; 3],
    palette: [[u8; 3]; 16],
}

impl Pty {
    fn spawn(repository: &Path, controlling_terminal: bool) -> gix_error::TestResult<(Self, Termios)> {
        let master = rustix::pty::openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY)?;
        rustix::io::fcntl_setfd(&master, rustix::io::FdFlags::CLOEXEC)?;
        rustix::pty::grantpt(&master)?;
        rustix::pty::unlockpt(&master)?;
        let slave = rustix::fs::open(
            rustix::pty::ptsname(&master, Vec::new())?,
            OFlags::RDWR | OFlags::NOCTTY | OFlags::CLOEXEC,
            Mode::empty(),
        )?;
        let original = rustix::termios::tcgetattr(&slave)?;
        rustix::termios::tcsetwinsize(
            &slave,
            Winsize {
                ws_row: 24,
                ws_col: 100,
                ws_xpixel: 800,
                ws_ypixel: 384,
            },
        )?;
        rustix::fs::fcntl_setfl(&master, OFlags::NONBLOCK)?;
        let slave = File::from(slave);
        let mut command = Command::new(std::env::current_exe()?);
        gix_testtools::configure_git_environment(&mut command, repository)
            .current_dir(repository)
            .args([
                "--exact",
                "launch_in_pty",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("TERM", "xterm-ghostty")
            .env("TERM_PROGRAM", "ghostty")
            .env(PTY_CHILD, "1")
            .env_remove("TMUX")
            .env_remove("STY")
            .env_remove("NO_COLOR")
            .stdin(Stdio::from(slave.try_clone()?))
            .stdout(Stdio::from(slave.try_clone()?))
            .stderr(Stdio::from(slave));
        command.env_remove(NO_CONTROLLING_TERMINAL);
        if !controlling_terminal {
            command.env(NO_CONTROLLING_TERMINAL, "1");
        }
        Ok((
            Self {
                child: command.spawn()?,
                master: File::from(master),
                output: Vec::new(),
                parsed: 0,
                graphics: true,
                foreground: [221; 3],
                background: [0; 3],
                palette: std::array::from_fn(|index| [index as u8 * 17, 136, 255]),
            },
            original,
        ))
    }

    fn wait_for(&mut self, ready: impl Fn(&[u8]) -> bool) -> io::Result<()> {
        let deadline = Instant::now() + WAIT;
        loop {
            self.pump()?;
            if ready(&self.output) {
                return Ok(());
            }
            if self.child.try_wait()?.is_some() || Instant::now() >= deadline {
                return Err(self.failure("tix did not draw the expected screen"));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn wait_for_exit(&mut self) -> io::Result<ExitStatus> {
        let deadline = Instant::now() + WAIT;
        loop {
            self.pump()?;
            if let Some(status) = self.child.try_wait()? {
                self.pump()?;
                return Ok(status);
            }
            if Instant::now() >= deadline {
                return Err(self.failure("tix did not exit without additional input"));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn failure(&self, message: &str) -> io::Error {
        io::Error::other(format!(
            "{message}; terminal output ends with {:?}",
            String::from_utf8_lossy(&self.output[self.output.len().saturating_sub(1024)..])
        ))
    }

    fn pump(&mut self) -> io::Result<()> {
        let mut buffer = [0; 16 * 1024];
        loop {
            match self.master.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => self.output.extend_from_slice(&buffer[..count]),
                Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                // Linux reports EIO instead of EOF when the last slave handle closes.
                Err(err)
                    if err.kind() == io::ErrorKind::WouldBlock
                        || err.raw_os_error() == Some(rustix::io::Errno::IO.raw_os_error()) =>
                {
                    break;
                }
                Err(err) => return Err(err),
            }
        }
        // Only startup queries need replies; preserve partial control strings between reads.
        while self.parsed < self.output.len() {
            let pending = &self.output[self.parsed..];
            if pending[0] != 0x1b {
                self.parsed += 1;
                continue;
            }
            let Some(&kind) = pending.get(1) else {
                break;
            };
            let end = match kind {
                b'[' => pending[2..]
                    .iter()
                    .position(|byte| (0x40..=0x7e).contains(byte))
                    .map(|end| end + 3),
                b']' | b'_' => pending[2..].iter().enumerate().find_map(|(index, byte)| match byte {
                    0x07 => Some(index + 3),
                    0x1b if pending.get(index + 3) == Some(&b'\\') => Some(index + 4),
                    _ => None,
                }),
                _ => Some(2),
            };
            let Some(end) = end else {
                break;
            };
            let sequence = &pending[..end];
            match sequence {
                _ if sequence.starts_with(b"\x1b_Ga=q,") && self.graphics => {
                    let controls = sequence
                        .strip_prefix(b"\x1b_G")
                        .expect("a graphics query has its APC opener");
                    let id = controls
                        .split(|byte| *byte == b',')
                        .find_map(|field| field.strip_prefix(b"i="))
                        .and_then(|field| field.split(|byte| *byte == b';').next())
                        .expect("the graphics probe identifies its reply");
                    self.master.write_all(b"\x1b_Gi=")?;
                    self.master.write_all(id)?;
                    self.master.write_all(b";OK\x1b\\")?;
                }
                b"\x1b[c" | b"\x1b[0c" => self.master.write_all(b"\x1b[?62;4;22c")?,
                b"\x1b[?u" => self.master.write_all(b"\x1b[?0u")?,
                b"\x1b[6n" => self.master.write_all(b"\x1b[1;1R")?,
                _ if sequence.starts_with(b"\x1b]10;?") => {
                    let [red, green, blue] = self.foreground;
                    write!(self.master, "\x1b]10;rgb:{red:02x}/{green:02x}/{blue:02x}\x1b\\")?;
                }
                _ if sequence.starts_with(b"\x1b]11;?") => {
                    let [red, green, blue] = self.background;
                    write!(self.master, "\x1b]11;rgb:{red:02x}/{green:02x}/{blue:02x}\x1b\\")?;
                }
                _ if sequence.starts_with(b"\x1b]21;") => {
                    let mut palette = b"\x1b]21".to_vec();
                    for (index, [red, green, blue]) in self.palette.iter().enumerate() {
                        write!(palette, ";{index}=rgb:{red:02x}/{green:02x}/{blue:02x}")?;
                    }
                    palette.extend_from_slice(b"\x1b\\");
                    self.master.write_all(&palette)?;
                }
                _ => {}
            }
            self.parsed += end;
        }
        Ok(())
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn contains(output: &[u8], sequence: &[u8]) -> bool {
    output.windows(sequence.len()).any(|window| window == sequence)
}

fn emitted_rgb_colors(output: &[u8], prefix: &str) -> Vec<[u8; 3]> {
    String::from_utf8_lossy(output)
        .split(prefix)
        .skip(1)
        .filter_map(|color| {
            let mut channels = color.split([';', 'm']);
            Some([
                channels.next()?.parse::<u8>().ok()?,
                channels.next()?.parse::<u8>().ok()?,
                channels.next()?.parse::<u8>().ok()?,
            ])
        })
        .collect()
}
