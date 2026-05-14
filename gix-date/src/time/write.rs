use crate::{SecondsSinceUnixEpoch, Time};

/// Serialize this instance as string, similar to what [`write_to()`](Self::write_to()) would do.
impl std::fmt::Display for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buf = Vec::with_capacity(Time::MIN.size());
        self.write_to(&mut buf).expect("write to memory cannot fail");
        // Time serializes as ASCII, which is a subset of UTF-8.
        // Use `from_utf8()` (validated) instead of `from_utf8_unchecked()` for safety,
        // with the option to switch to an unsafe version if 30% performance boost are needed here.
        let raw = std::str::from_utf8(&buf).expect("time serializes as valid UTF-8");
        f.write_str(raw)
    }
}

/// Serialization with standard `git` format
impl Time {
    /// Serialize this instance to `out` in a format suitable for use in header fields of serialized git commits or tags.
    pub fn write_to(&self, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        const SECONDS_PER_HOUR: u32 = 60 * 60;
        let offset = self.offset.unsigned_abs();
        let hours = offset / SECONDS_PER_HOUR;
        let minutes = (offset - (hours * SECONDS_PER_HOUR)) / 60;

        if hours > 99 {
            return Err(std::io::Error::other("Cannot represent offsets larger than +-9900"));
        }

        let mut itoa = itoa::Buffer::new();
        out.write_all(itoa.format(self.seconds).as_bytes())?;
        out.write_all(b" ")?;
        out.write_all(if self.offset < 0 { b"-" } else { b"+" })?;

        const ZERO: &[u8; 1] = b"0";

        if hours < 10 {
            out.write_all(ZERO)?;
        }
        out.write_all(itoa.format(hours).as_bytes())?;

        if minutes < 10 {
            out.write_all(ZERO)?;
        }
        out.write_all(itoa.format(minutes).as_bytes()).map(|_| ())
    }

    /// Computes the number of bytes necessary to write it using [`Time::write_to()`].
    pub const fn size(&self) -> usize {
        let is_negative = self.seconds < 0;
        Self::count_positive_digits(self.seconds.unsigned_abs()) + is_negative as usize + 6
        // space + offset sign + hours (2) + minutes (2)
    }

    /// Count the number of decimal digits in a positive integer.
    const fn count_positive_digits(n: u64) -> usize {
        // Powers of 10 for comparison
        const POW10: [u64; 20] = [
            1,
            10,
            100,
            1_000,
            10_000,
            100_000,
            1_000_000,
            10_000_000,
            100_000_000,
            1_000_000_000,
            10_000_000_000,
            100_000_000_000,
            1_000_000_000_000,
            10_000_000_000_000,
            100_000_000_000_000,
            1_000_000_000_000_000,
            10_000_000_000_000_000,
            100_000_000_000_000_000,
            1_000_000_000_000_000_000,
            10_000_000_000_000_000_000,
        ];

        // Binary search would be nice but not const-fn friendly, so use simple loop
        let mut digits = 1;
        while digits < 20 && n >= POW10[digits] {
            digits += 1;
        }
        digits
    }

    /// The numerically largest possible time instance.
    pub const MAX: Time = Time {
        seconds: SecondsSinceUnixEpoch::MAX,
        offset: 99 * 60 * 60 + 59 * 60 + 59,
    };

    /// The numerically smallest possible time instance.
    pub const MIN: Time = Time {
        seconds: SecondsSinceUnixEpoch::MIN,
        offset: -(99 * 60 * 60 + 59 * 60 + 59),
    };
}
