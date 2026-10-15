//! Startup-only ANSI palette queries, before Crossterm owns terminal input.
//!
//! A DA1 reply finishes each query without reading subsequent input. An incomplete
//! query retains its parser state for suppressing late replies in the event loop.

use std::{
    io::{self, Read, Write},
    time::{Duration, Instant},
};

use crate::terminal_reply::{ReplyFilter, Sequence};

type Palette = [[u8; 3]; 16];
const MAX_SEQUENCE: usize = 1024;
const MAX_RESPONSE: usize = 16 * 1024;
const DA1: &[u8] = b"\x1b[c";

pub(crate) fn query(timeout: Duration, pending: &mut ReplyFilter) -> io::Result<Option<Palette>> {
    use std::os::unix::fs::OpenOptionsExt;

    if timeout.is_zero() {
        return Ok(None);
    }
    let started = Instant::now();
    let mut tty = Tty {
        input: std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(rustix::fs::OFlags::NONBLOCK.bits() as i32)
            .open("/dev/tty")?,
        output: std::fs::OpenOptions::new().write(true).open("/dev/tty")?,
    };
    let mut raw_mode = RawMode::enter()?;
    // Leave already queued keystrokes for Crossterm instead of starting a competing query.
    let result = if rustix::io::ioctl_fionread(&tty.input)? > 0 {
        Ok(None)
    } else {
        query_with(&mut tty, started, timeout, pending)
    };
    let restored = raw_mode.restore();
    result.and_then(|palette| restored.map(|()| palette))
}

struct Tty {
    input: std::fs::File,
    output: std::fs::File,
}

impl Read for Tty {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.input.read(buffer)
    }
}

impl Write for Tty {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.output.write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }
}

struct RawMode(bool);

impl RawMode {
    fn enter() -> io::Result<Self> {
        let restore = !crossterm::terminal::is_raw_mode_enabled()?;
        if restore {
            crossterm::terminal::enable_raw_mode()?;
        }
        Ok(Self(restore))
    }

    fn restore(&mut self) -> io::Result<()> {
        if self.0 {
            crossterm::terminal::disable_raw_mode()?;
            self.0 = false;
        }
        Ok(())
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

#[derive(Clone, Copy)]
enum Protocol {
    Kitty,
    Xterm,
}

fn query_with(
    tty: &mut (impl Read + Write),
    started: Instant,
    timeout: Duration,
    pending: &mut ReplyFilter,
) -> io::Result<Option<Palette>> {
    let mut received = 0;
    for protocol in [Protocol::Kitty, Protocol::Xterm] {
        if started.elapsed() >= timeout {
            return Ok(None);
        }
        let mut request = match protocol {
            Protocol::Kitty => b"\x1b]21".to_vec(),
            Protocol::Xterm => b"\x1b]4".to_vec(),
        };
        for index in 0..16 {
            match protocol {
                Protocol::Kitty => write!(request, ";{index}=?")?,
                Protocol::Xterm => write!(request, ";{index};?")?,
            }
        }
        request.extend_from_slice(b"\x1b\\");
        request.extend_from_slice(DA1);
        pending.arm();
        // Requests use a blocking handle so the read deadline cannot truncate an outgoing OSC.
        tty.write_all(&request)?;
        tty.flush()?;
        let mut replies = Replies::new(protocol);
        loop {
            if started.elapsed() >= timeout || received == MAX_RESPONSE {
                return Ok(None);
            }
            // Reading a byte at a time leaves anything after the completion marker queued.
            let mut byte = [0];
            match tty.read(&mut byte) {
                Ok(0) => return Ok(None),
                Ok(_) => {
                    received += 1;
                    let complete = replies.push(byte[0]);
                    pending.set_sequence(replies.sequence);
                    if complete {
                        pending.clear();
                        if let Some(palette) = replies.palette() {
                            return Ok(Some(palette));
                        }
                        break;
                    }
                }
                Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => pause(started, timeout),
                Err(err) => return Err(err),
            }
        }
    }
    Ok(None)
}

fn pause(started: Instant, timeout: Duration) {
    std::thread::sleep(timeout.saturating_sub(started.elapsed()).min(Duration::from_millis(1)));
}

struct Replies {
    protocol: Protocol,
    sequence: Sequence,
    body: Vec<u8>,
    oversized: bool,
    colors: [Option<[u8; 3]>; 16],
    invalid: bool,
}

impl Replies {
    fn new(protocol: Protocol) -> Self {
        Self {
            protocol,
            sequence: Sequence::Ground,
            body: Vec::new(),
            oversized: false,
            colors: [None; 16],
            invalid: false,
        }
    }

    fn push(&mut self, byte: u8) -> bool {
        use Sequence::*;
        match (self.sequence, byte) {
            (Ground | Escape, 0x1b) => self.sequence = Escape,
            (Escape, b']') | (Ground, 0x9d) => self.sequence = Osc,
            (Escape, b'[') | (Ground, 0x9b) => self.sequence = Csi,
            (Osc | OscEscape, 0x07 | 0x9c) | (OscEscape, b'\\') => {
                if self.oversized {
                    self.invalid = true;
                } else {
                    self.apply_osc();
                }
                self.reset();
            }
            (Osc | OscEscape, 0x1b) => self.sequence = OscEscape,
            (OscEscape, _) => {
                self.oversized = true;
                self.sequence = Osc;
            }
            (Csi, 0x40..=0x7e) => {
                let done = byte == b'c'
                    && !self.oversized
                    && self.body.strip_prefix(b"?").is_some_and(|parameters| {
                        parameters
                            .split(|byte| *byte == b';')
                            .all(|parameter| !parameter.is_empty() && parameter.iter().all(u8::is_ascii_digit))
                    });
                self.reset();
                return done;
            }
            (Csi, 0x1b) => {
                self.reset();
                self.sequence = Escape;
            }
            (Osc | Csi, _) => {
                if self.body.len() < MAX_SEQUENCE {
                    self.body.push(byte);
                } else {
                    self.oversized = true;
                }
            }
            (Ground | Escape, _) => self.reset(),
        }
        false
    }

    fn reset(&mut self) {
        self.sequence = Sequence::Ground;
        self.body.clear();
        self.oversized = false;
    }

    fn apply_osc(&mut self) {
        let prefix = match self.protocol {
            Protocol::Kitty => b"21;".as_slice(),
            Protocol::Xterm => b"4;".as_slice(),
        };
        let Some(body) = self.body.strip_prefix(prefix) else {
            return;
        };
        let parsed = (|| {
            let body = std::str::from_utf8(body).ok()?;
            let mut fields = body.split(';');
            while let Some(field) = fields.next() {
                let (index, color) = match self.protocol {
                    Protocol::Kitty => field.split_once('=')?,
                    Protocol::Xterm => (field, fields.next()?),
                };
                if index.is_empty() || index.len() > 2 || !index.bytes().all(|byte| byte.is_ascii_digit()) {
                    return None;
                }
                let index = index.parse::<usize>().ok()?;
                let slot = self.colors.get_mut(index)?;
                let color = parse_color(color)?;
                if slot.is_some_and(|previous| previous != color) {
                    return None;
                }
                *slot = Some(color);
            }
            Some(())
        })();
        self.invalid |= parsed.is_none();
    }

    fn palette(&self) -> Option<Palette> {
        if self.invalid {
            return None;
        }
        let mut palette = [[0; 3]; 16];
        for (color, parsed) in palette.iter_mut().zip(self.colors) {
            *color = parsed?;
        }
        Some(palette)
    }
}

fn parse_color(value: &str) -> Option<[u8; 3]> {
    fn channel(value: &str) -> Option<u8> {
        if !(1..=4).contains(&value.len()) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        let number = u32::from_str_radix(value, 16).ok()?;
        let maximum = (1 << (4 * value.len())) - 1;
        Some(((number * 255 + maximum / 2) / maximum) as u8)
    }
    let mut channels = value.strip_prefix("rgb:")?.split('/');
    let color = [
        channel(channels.next()?)?,
        channel(channels.next()?)?,
        channel(channels.next()?)?,
    ];
    channels.next().is_none().then_some(color)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::VecDeque, fmt::Write as _};

    const MARKER: &[u8] = b"\x1b[?62;4;22c";

    #[derive(Default)]
    struct Terminal {
        responses: VecDeque<Vec<u8>>,
        input: VecDeque<u8>,
        output: Vec<u8>,
        fail_when_empty: bool,
    }

    impl Read for Terminal {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            match self.input.pop_front() {
                Some(byte) => {
                    buffer[0] = byte;
                    Ok(1)
                }
                None if self.fail_when_empty => Err(io::ErrorKind::BrokenPipe.into()),
                None => Err(io::ErrorKind::WouldBlock.into()),
            }
        }
    }

    impl Write for Terminal {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            let length = buffer.len().min(3);
            self.output.extend_from_slice(&buffer[..length]);
            if self.output.ends_with(DA1) {
                self.input.extend(self.responses.pop_front().unwrap_or_default());
            }
            Ok(length)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn response(protocol: Protocol, count: usize) -> Vec<u8> {
        let mut response = String::new();
        match protocol {
            Protocol::Kitty => {
                response.push_str("\x1b]21");
                for index in 0..count {
                    write!(response, ";{index}=rgb:{index:x}/80/ffff").expect("writing to a String cannot fail");
                }
                response.push_str("\x1b\\");
            }
            Protocol::Xterm => {
                for index in 0..count {
                    write!(response, "\x1b]4;{index};rgb:{index:x}/80/ffff\x07")
                        .expect("writing to a String cannot fail");
                }
            }
        }
        response.into_bytes()
    }

    #[test]
    fn fragmented_palette_consumes_the_marker_and_preserves_following_input() -> gix_error::TestResult {
        for protocol in [Protocol::Kitty, Protocol::Xterm] {
            let mut reply = response(protocol, 16);
            reply.extend_from_slice(MARKER);
            reply.push(b'j');
            let mut terminal = Terminal::default();
            if matches!(protocol, Protocol::Xterm) {
                terminal.responses.push_back(MARKER.to_vec());
            }
            terminal.responses.push_back(reply);
            let palette = query_with(
                &mut terminal,
                Instant::now(),
                Duration::from_secs(1),
                &mut Default::default(),
            )?
            .expect("a complete palette and marker are available");
            for (index, color) in palette.into_iter().enumerate() {
                assert_eq!(
                    color,
                    [index as u8 * 17, 128, 255],
                    "each indexed color is scaled to eight bits"
                );
            }
            assert_eq!(
                terminal.input,
                [b'j'],
                "the completion marker does not consume later typing"
            );
            let requests = String::from_utf8(terminal.output)?;
            assert_eq!(requests.matches("\x1b]21").count(), 1, "OSC 21 is queried once");
            assert_eq!(
                requests.matches("\x1b]4").count(),
                usize::from(matches!(protocol, Protocol::Xterm)),
                "OSC 4 is only sent after the first marker reports an incomplete palette"
            );
        }
        Ok(())
    }

    #[test]
    fn unsupported_partial_and_malformed_responses_never_supply_a_palette() -> gix_error::TestResult {
        for mut reply in [
            Vec::new(),
            response(Protocol::Kitty, 15),
            response(Protocol::Kitty, 17),
            b"\x1b]21;0=rgb:zz/00/00\x07".to_vec(),
            b"\x1b]21;0=rgb:0/0/0;0=rgb:f/f/f\x07".to_vec(),
        ] {
            reply.extend_from_slice(MARKER);
            let mut terminal = Terminal {
                responses: [reply, MARKER.to_vec()].into(),
                ..Terminal::default()
            };
            assert_eq!(
                query_with(
                    &mut terminal,
                    Instant::now(),
                    Duration::from_secs(1),
                    &mut Default::default()
                )?,
                None,
                "unsupported, incomplete, and invalid palettes retain the caller's fallback"
            );
            assert!(terminal.input.is_empty(), "both completion markers are consumed");
        }
        Ok(())
    }

    #[test]
    fn timeout_never_starts_fallback_without_the_first_marker() -> gix_error::TestResult {
        assert_eq!(
            query(Duration::ZERO, &mut Default::default())?,
            None,
            "zero budget never opens a terminal"
        );
        for reply in [Vec::new(), response(Protocol::Kitty, 16)] {
            let mut terminal = Terminal {
                responses: [reply].into(),
                ..Terminal::default()
            };
            assert_eq!(
                query_with(&mut terminal, Instant::now(), Duration::ZERO, &mut Default::default())?,
                None
            );
            assert!(terminal.output.is_empty(), "an exhausted budget performs no I/O");
            assert_eq!(
                query_with(
                    &mut terminal,
                    Instant::now(),
                    Duration::from_millis(2),
                    &mut Default::default()
                )?,
                None,
                "even all sixteen colors need a completion marker"
            );
            assert!(!terminal.output.windows(4).any(|bytes| bytes == b"\x1b]4;"));
        }
        Ok(())
    }

    #[test]
    fn read_errors_preserve_partial_reply_state_for_the_input_filter() {
        use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

        for (prefix, next) in [
            (b"\x1b".as_slice(), ']'),
            (b"\x1b]21;0=rgb:f/", 'f'),
            (b"\x1b]21;0=rgb:f/f/f\x1b", '\\'),
            (b"\x1b[?62;", '4'),
        ] {
            let mut terminal = Terminal {
                responses: [prefix.to_vec()].into(),
                fail_when_empty: true,
                ..Terminal::default()
            };
            let mut pending = ReplyFilter::default();
            assert!(
                query_with(&mut terminal, Instant::now(), Duration::from_secs(1), &mut pending).is_err(),
                "the transport fails after the partial reply"
            );
            assert!(
                pending.consume(&Event::Key(KeyEvent::new(KeyCode::Char(next), KeyModifiers::NONE))),
                "the next reply byte cannot become an application command"
            );
        }
    }

    #[test]
    fn response_storage_is_bounded_and_color_components_are_strict() {
        let mut replies = Replies::new(Protocol::Kitty);
        for byte in b"\x1b]21;"
            .iter()
            .copied()
            .chain(std::iter::repeat_n(b'0', MAX_RESPONSE))
        {
            replies.push(byte);
        }
        assert_eq!(
            replies.body.len(),
            MAX_SEQUENCE,
            "a malformed string cannot grow without bound"
        );
        replies.push(0x07);
        for byte in response(Protocol::Kitty, 16) {
            replies.push(byte);
        }
        assert!(
            MARKER.iter().copied().any(|byte| replies.push(byte)),
            "oversized OSC still drains to DA1"
        );
        assert_eq!(
            replies.palette(),
            None,
            "later valid colors cannot salvage a malformed response"
        );
        for (value, expected) in [("rgb:f/80/ffff", [255, 128, 255]), ("rgb:0000/1234/ffff", [0, 18, 255])] {
            assert_eq!(
                parse_color(value),
                Some(expected),
                "supported hexadecimal color: {value}"
            );
        }
        for value in [
            "rgb:/0/0",
            "rgb:10000/0/0",
            "rgb:0/0/0/0",
            "rgb:+1/0/0",
            "rgb:gg/0/0",
            "#f08",
            "#ffff",
            "#é00",
            "red",
        ] {
            assert_eq!(parse_color(value), None, "invalid color: {value}");
        }
    }
}
