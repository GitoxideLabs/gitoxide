//! Suppress terminal-query replies that arrive after startup color detection.

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

#[derive(Clone, Copy)]
pub(crate) enum Sequence {
    Ground,
    Escape,
    Osc,
    #[cfg_attr(not(unix), allow(dead_code))]
    OscEscape,
    Csi,
}

pub(crate) struct ReplyFilter {
    sequence: Sequence,
    pending: bool,
}

impl Default for ReplyFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplyFilter {
    pub(crate) const fn new() -> Self {
        Self {
            sequence: Sequence::Ground,
            pending: false,
        }
    }

    pub(crate) fn arm(&mut self) {
        self.clear();
        self.pending = true;
    }

    #[cfg(any(unix, test))]
    pub(crate) fn set_sequence(&mut self, sequence: Sequence) {
        self.sequence = sequence;
    }

    pub(crate) fn clear(&mut self) {
        self.sequence = Sequence::Ground;
        self.pending = false;
    }

    /// Return whether this event belongs to a reply instead of user input.
    pub(crate) fn consume(&mut self, event: &Event) -> bool {
        let Event::Key(key) = event else {
            return false;
        };
        if key.kind == KeyEventKind::Release {
            return false;
        }
        if key.code == KeyCode::Esc {
            // A short terminal read can split either an OSC opener or its ST terminator.
            self.sequence = if self.pending || matches!(self.sequence, Sequence::Osc | Sequence::OscEscape) {
                Sequence::Escape
            } else {
                Sequence::Ground
            };
            return false;
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.clear();
            return false;
        }
        // Crossterm decodes an OSC opener as Alt+], followed by ordinary character events.
        if key.code == KeyCode::Char(']') && key.modifiers == KeyModifiers::ALT {
            self.sequence = Sequence::Osc;
            return true;
        }
        if matches!(self.sequence, Sequence::Osc | Sequence::OscEscape)
            && ((key.code == KeyCode::Char('g') && key.modifiers == KeyModifiers::CONTROL)
                || (key.code == KeyCode::Char('\\') && key.modifiers == KeyModifiers::ALT))
        {
            self.sequence = Sequence::Ground;
            return true;
        }
        if matches!(self.sequence, Sequence::Ground) {
            self.pending = false;
            return false;
        }
        let KeyCode::Char(character) = key.code else {
            return false;
        };
        if key.modifiers.intersects(!KeyModifiers::SHIFT) {
            return false;
        }
        match self.sequence {
            Sequence::Ground => false,
            Sequence::Escape => {
                self.sequence = match character {
                    ']' if self.pending => Sequence::Osc,
                    '[' if self.pending => Sequence::Csi,
                    '\\' => Sequence::Ground,
                    _ => {
                        self.clear();
                        return false;
                    }
                };
                true
            }
            Sequence::OscEscape if character == '\\' => {
                self.sequence = Sequence::Ground;
                true
            }
            // Keep no payload buffer, and never release an oversized reply's tail as commands.
            Sequence::Osc | Sequence::OscEscape => {
                self.sequence = Sequence::Osc;
                true
            }
            Sequence::Csi => {
                if ('@'..='~').contains(&character) {
                    self.clear();
                }
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;

    fn key(character: char, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(KeyCode::Char(character), modifiers))
    }

    #[test]
    fn delayed_osc_palette_and_default_color_replies_never_become_commands() {
        for body in [
            "4;0;rgb:ff/00/80",
            "21;0=#ff0080;1=rgb:f/8/0",
            "10;rgb:ff/ff/ff",
            "11;rgb:0/0/0",
        ] {
            for terminator in [key('g', KeyModifiers::CONTROL), key('\\', KeyModifiers::ALT)] {
                let mut filter = ReplyFilter::new();
                assert!(
                    filter.consume(&key(']', KeyModifiers::ALT)),
                    "OSC is recognized even after the raw query has returned"
                );
                for character in body.chars() {
                    assert!(
                        filter.consume(&key(character, KeyModifiers::NONE)),
                        "reply payload never reaches command handling"
                    );
                }
                assert!(filter.consume(&terminator), "BEL and ST both finish the reply");
                assert!(
                    !filter.consume(&key('q', KeyModifiers::NONE)),
                    "typing after the reply remains available"
                );
            }
        }
    }

    #[test]
    fn partial_raw_query_sequences_continue_in_the_event_reader() {
        for (sequence, tail) in [
            (Sequence::Escape, "]4;0;rgb:f/0/0"),
            (Sequence::Osc, "f/0/0"),
            (Sequence::OscEscape, "\\"),
            (Sequence::Escape, "[?62;4;22c"),
            (Sequence::Csi, ";4;22c"),
        ] {
            let mut filter = ReplyFilter::new();
            filter.arm();
            filter.set_sequence(sequence);
            for character in tail.chars() {
                assert!(
                    filter.consume(&key(character, KeyModifiers::NONE)),
                    "the consumed raw prefix cannot expose its remaining payload as keys"
                );
            }
            if matches!(filter.sequence, Sequence::Osc) {
                assert!(filter.consume(&key('\\', KeyModifiers::ALT)));
            }
            assert!(
                !filter.consume(&key('j', KeyModifiers::NONE)),
                "ordinary navigation resumes after a complete reply"
            );
        }
    }

    #[test]
    fn split_st_preserves_escape_recovery_without_delivering_its_backslash() {
        for next in ['\\', 'q'] {
            let mut filter = ReplyFilter::new();
            assert!(filter.consume(&key(']', KeyModifiers::ALT)));
            assert!(
                !filter.consume(&Event::Key(KeyCode::Esc.into())),
                "Escape remains available for recovery"
            );
            assert!(!filter.consume(&Event::Resize(80, 24)));
            assert!(!filter.consume(&Event::Key(KeyCode::Left.into())));
            assert_eq!(
                filter.consume(&key(next, KeyModifiers::NONE)),
                next == '\\',
                "only a split ST terminator is suppressed after Escape"
            );
            assert!(
                !filter.consume(&key('a', KeyModifiers::NONE)),
                "typing resumes after either completion or recovery"
            );
        }
    }

    #[test]
    fn split_osc_opener_is_suppressed_but_escape_then_navigation_is_not() {
        let mut filter = ReplyFilter::new();
        filter.arm();
        assert!(!filter.consume(&Event::Key(KeyCode::Esc.into())));
        for character in "]4;0;rgb:ff/00/80".chars() {
            assert!(
                filter.consume(&key(character, KeyModifiers::NONE)),
                "fragmenting an OSC opener must not expose reply characters as commands"
            );
        }
        assert!(filter.consume(&key('\\', KeyModifiers::ALT)));
        assert!(!filter.consume(&Event::Key(KeyCode::Esc.into())));
        assert!(
            !filter.consume(&key('j', KeyModifiers::NONE)),
            "Escape followed by ordinary navigation remains user input"
        );
        assert!(
            !filter.consume(&key('d', KeyModifiers::NONE)),
            "navigation leaves no pending escape state"
        );
    }

    #[test]
    fn completed_queries_and_ordinary_input_restore_escape_then_alignment() {
        for after_query in [false, true] {
            let mut filter = ReplyFilter::new();
            if after_query {
                filter.arm();
                filter.clear();
            }
            for event in [
                Event::Key(KeyCode::Esc.into()),
                key('[', KeyModifiers::NONE),
                key('j', KeyModifiers::NONE),
            ] {
                assert!(
                    !filter.consume(&event),
                    "Escape, alignment toggle, and navigation remain normal shortcuts without a pending query"
                );
            }
        }
        let mut filter = ReplyFilter::new();
        filter.arm();
        for body in ["]4;0;rgb:f/0/0", "]4;1;rgb:0/f/0"] {
            assert!(!filter.consume(&Event::Key(KeyCode::Esc.into())));
            for character in body.chars() {
                assert!(
                    filter.consume(&key(character, KeyModifiers::NONE)),
                    "one completed OSC must not expose later replies from the same query"
                );
            }
            assert!(filter.consume(&key('\\', KeyModifiers::ALT)));
        }
        assert!(
            !filter.consume(&key('j', KeyModifiers::NONE)),
            "unambiguous user input ends pending startup detection"
        );
        assert!(!filter.consume(&Event::Key(KeyCode::Esc.into())));
        assert!(
            !filter.consume(&key('[', KeyModifiers::NONE)),
            "ordinary input restores Escape followed by alignment"
        );
    }

    #[test]
    fn malformed_replies_use_constant_memory_and_escape_or_interrupt_recovers() {
        for recovery in [Event::Key(KeyCode::Esc.into()), key('c', KeyModifiers::CONTROL)] {
            let mut filter = ReplyFilter::new();
            assert!(filter.consume(&key(']', KeyModifiers::ALT)));
            for _ in 0..32_768 {
                assert!(
                    filter.consume(&key('j', KeyModifiers::NONE)),
                    "oversized and malformed replies stay suppressed until a delimiter or explicit recovery"
                );
            }
            assert!(
                !filter.consume(&Event::Resize(80, 24)),
                "non-key events remain available"
            );
            assert!(
                !filter.consume(&Event::Key(KeyCode::Left.into())),
                "navigation is not a reply byte"
            );
            assert!(
                filter.consume(&key('d', KeyModifiers::NONE)),
                "non-payload events do not expose the remaining reply tail"
            );
            assert!(
                !filter.consume(&recovery),
                "Escape and Ctrl-C always reach the application and abort a truncated reply"
            );
            for event in [
                key('j', KeyModifiers::NONE),
                key(']', KeyModifiers::NONE),
                key('x', KeyModifiers::ALT),
                Event::Key(KeyCode::Enter.into()),
            ] {
                assert!(!filter.consume(&event), "ordinary keyboard input is unaffected");
            }
        }
    }
}
