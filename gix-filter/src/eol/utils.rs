use crate::eol::{AttributesDigest, AutoCrlf, Configuration, Mode, Stats};

impl Default for Mode {
    fn default() -> Self {
        if cfg!(windows) { Mode::CrLf } else { Mode::Lf }
    }
}

impl AttributesDigest {
    /// Return the end-of-line mode this digest would require, or `None` if no conversion would be performed.
    pub fn to_eol(&self, config: Configuration) -> Option<Mode> {
        Some(match self {
            AttributesDigest::Binary => return None,
            AttributesDigest::TextInput | AttributesDigest::TextAutoInput => Mode::Lf,
            AttributesDigest::TextCrlf | AttributesDigest::TextAutoCrlf => Mode::CrLf,
            AttributesDigest::Text | AttributesDigest::TextAuto => config.to_eol(),
        })
    }

    /// Return true if this digest allows for auto-determination of CRLF text conversion.
    pub fn is_auto_text(&self) -> bool {
        matches!(
            self,
            AttributesDigest::TextAuto | AttributesDigest::TextAutoCrlf | AttributesDigest::TextAutoInput
        )
    }
}

impl Configuration {
    /// Return the line-ending mode that is configured here.
    pub fn to_eol(&self) -> Mode {
        match self.auto_crlf {
            AutoCrlf::Enabled => Mode::CrLf,
            AutoCrlf::Input => Mode::Lf,
            AutoCrlf::Disabled => self.eol.unwrap_or_default(),
        }
    }
}

impl Stats {
    /// Gather statistics from the given `bytes`.
    ///
    /// Note that the entire buffer will be scanned.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut bytes = bytes.iter().peekable();
        let mut null = 0;
        let mut lone_cr = 0;
        let mut lone_lf = 0;
        let mut crlf = 0;
        let mut printable = 0;
        let mut non_printable = 0;
        while let Some(b) = bytes.next() {
            if *b == b'\r' {
                match bytes.peek() {
                    Some(n) if **n == b'\n' => {
                        bytes.next();
                        crlf += 1;
                    }
                    _ => lone_cr += 1,
                }
                continue;
            }
            if *b == b'\n' {
                lone_lf += 1;
                continue;
            }
            if *b == 127 {
                non_printable += 1;
            } else if *b < 32 {
                match *b {
                    8 /* \b */ | b'\t' | 27 /* \033 */ | 12 /* \014 */ => printable += 1,
                    0 => {
                        non_printable += 1;
                        null += 1;
                    },
                    _ => non_printable += 1,
                }
            } else {
                printable += 1;
            }
        }

        Self {
            null,
            lone_cr,
            lone_lf,
            crlf,
            printable,
            non_printable,
        }
    }

    /// Returns `true` if these statistics are typical for a binary file.
    pub fn is_binary(&self) -> bool {
        self.lone_cr > 0 || self.null > 0 || (self.printable >> 7) < self.non_printable
    }

    /// Return `true` if we would convert the buffer from which these stats are derived, knowing only the digest
    pub fn will_convert_lf_to_crlf(&self, digest: AttributesDigest, config: Configuration) -> bool {
        if digest.to_eol(config) != Some(Mode::CrLf) {
            return false;
        }

        // nothing to do?
        if self.lone_lf == 0 {
            return false;
        }

        if digest.is_auto_text() {
            if self.is_binary() {
                return false;
            }
            // Lone `\r` or mixed LF and CRLF isn't safe as it won't round-trip, and in auto-mode we don't touch it.
            if self.lone_cr > 0 || self.crlf > 0 {
                return false;
            }
        }
        true
    }
}
