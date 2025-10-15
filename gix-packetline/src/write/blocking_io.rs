use std::io;

use crate::{MAX_DATA_LEN, U16_HEX_BYTES};

/// An implementor of [`Write`][io::Write] which passes all input to an inner `Write` in packet line data encoding,
/// one line per `write(…)` call or as many lines as it takes if the data doesn't fit into the maximum allowed line length.
pub struct Writer<T> {
    /// the `Write` implementation to which to propagate packet lines
    inner: T,
    binary: bool,
}

impl<T: io::Write> Writer<T> {
    /// Create a new instance from the given `write`
    pub fn new(write: T) -> Self {
        Writer {
            inner: write,
            binary: true,
        }
    }
}

/// Non-IO methods
impl<T> Writer<T> {
    /// If called, each call to [`write()`][io::Write::write()] will write bytes as is.
    pub fn enable_binary_mode(&mut self) {
        self.binary = true;
    }
    /// If called, each call to [`write()`][io::Write::write()] will write the input as text, appending a trailing newline
    /// if needed before writing.
    pub fn enable_text_mode(&mut self) {
        self.binary = false;
    }
    /// Return the inner writer, consuming self.
    pub fn into_inner(self) -> T {
        self.inner
    }
    /// Return a mutable reference to the inner writer, useful if packet lines should be serialized directly.
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T: io::Write> io::Write for Writer<T> {
    fn write(&mut self, mut buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Err(io::Error::other(
                "empty packet lines are not permitted as '0004' is invalid",
            ));
        }

        let mut written = 0;
        while !buf.is_empty() {
            let (data, rest) = buf.split_at(buf.len().min(MAX_DATA_LEN));
            written += if self.binary {
                crate::encode::blocking_io::data_to_write(data, &mut self.inner)
            } else {
                crate::encode::blocking_io::text_to_write(data, &mut self.inner)
            }?;
            // subtract header (and trailing NL) because write-all can't handle writing more than it passes in
            written -= U16_HEX_BYTES + usize::from(!self.binary);
            buf = rest;
        }
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
