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

#[test]
fn normal_quit_restores_the_terminal() -> gix_error::TestResult {
    check_shutdown(None, View::Graph)
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

// Re-execution gives the child a controlling terminal without unsafe pre_exec/fork code.
#[test]
#[ignore = "entry point for the parent PTY tests"]
fn launch_in_pty() -> gix_error::TestResult {
    if std::env::var_os(PTY_CHILD).is_none() {
        return Ok(());
    }
    rustix::process::setsid()?;
    rustix::process::ioctl_tiocsctty(io::stdin())?;
    rustix::process::setrlimit(
        rustix::process::Resource::Core,
        rustix::process::Rlimit {
            current: Some(0),
            maximum: Some(0),
        },
    )?;
    let mut command = Command::new(env!("CARGO_BIN_EXE_tix"));
    gix_testtools::configure_git_environment(&mut command, std::env::current_dir()?).arg("--trace");
    Err(command.exec().into())
}

enum View {
    Graph,
    BuiltinDiff,
    PagerAcknowledgement,
}

fn check_shutdown(signal: Option<Signal>, view: View) -> gix_error::TestResult {
    let repository = fixture()?;
    if matches!(view, View::PagerAcknowledgement) {
        git(
            repository.path(),
            "config core.pager 'cat; printf TIX_TEST_PAGER_FINISHED'",
        )?;
    }
    let (mut terminal, original) = Pty::spawn(repository.path())?;
    terminal.wait_for(|output| contains(output, b"\x1b_Ga=p,") && contains(output, b"\x1b[?2026l"))?;
    let viewer_start = terminal.output.len();
    if !matches!(view, View::Graph) {
        terminal.master.write_all(b"\t\r")?;
        let marker = match view {
            View::BuiltinDiff => b"/q/Esc back".as_slice(),
            View::PagerAcknowledgement => b"TIX_TEST_PAGER_FINISHED",
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
        b"\x1b[<1u",
        b"\x1b]22;text\x1b\\\x1b]22;\x1b\\",
    ] {
        assert!(
            contains(shutdown, sequence),
            "{signal:?} must restore {:?} after exiting with {status}",
            String::from_utf8_lossy(sequence)
        );
    }
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

struct Pty {
    child: Child,
    master: File,
    output: Vec<u8>,
    parsed: usize,
}

impl Pty {
    fn spawn(repository: &Path) -> gix_error::TestResult<(Self, Termios)> {
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
            .env("TERM", "xterm-kitty")
            .env("TERM_PROGRAM", "kitty")
            .env(PTY_CHILD, "1")
            .env_remove("TMUX")
            .env_remove("STY")
            .stdin(Stdio::from(slave.try_clone()?))
            .stdout(Stdio::from(slave.try_clone()?))
            .stderr(Stdio::from(slave));
        Ok((
            Self {
                child: command.spawn()?,
                master: File::from(master),
                output: Vec::new(),
                parsed: 0,
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
                b"\x1b[c" | b"\x1b[0c" => self.master.write_all(b"\x1b[?62;4;22c")?,
                b"\x1b[?u" => self.master.write_all(b"\x1b[?0u")?,
                b"\x1b[6n" => self.master.write_all(b"\x1b[1;1R")?,
                _ if sequence.starts_with(b"\x1b]10;?") => {
                    self.master.write_all(b"\x1b]10;rgb:dddd/dddd/dddd\x1b\\")?;
                }
                _ if sequence.starts_with(b"\x1b]11;?") => {
                    self.master.write_all(b"\x1b]11;rgb:0000/0000/0000\x1b\\")?;
                }
                _ if sequence.starts_with(b"\x1b]21;") => {
                    let mut palette = b"\x1b]21".to_vec();
                    for index in 0..16 {
                        write!(palette, ";{index}=rgb:{index:x}/8/f")?;
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
