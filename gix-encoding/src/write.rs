use std::io::{self, Write};

use crate::{Converter, Encoding, convert::Status};

/// A writer converting between encodings with bounded buffering.
///
/// Call [`finish()`](Self::finish) or [`try_finish()`](Self::try_finish) to check for a
/// truncated final character and write any pending output. Dropping does not perform I/O.
/// [`flush()`](Write::flush) writes complete characters without ending the conversion.
/// Short writes, interruptions, and retryable sink errors retain unwritten output.
pub struct Transcoder<W> {
    inner: W,
    converter: Converter,
    input: [u8; 16],
    input_len: usize,
    output: Vec<u8>,
    output_pos: usize,
    output_len: usize,
    finished: bool,
}

impl<W: Write> Transcoder<W> {
    /// Wrap `inner` in a strict conversion stream.
    pub fn new(inner: W, from: Encoding, to: Encoding) -> Self {
        Self {
            inner,
            converter: Converter::new(from, to),
            input: [0; 16],
            input_len: 0,
            output: vec![0; 8192],
            output_pos: 0,
            output_len: 0,
            finished: false,
        }
    }

    /// Access the underlying writer.
    pub fn get_ref(&self) -> &W {
        &self.inner
    }

    /// Access the writer mutably. Direct writes may disrupt buffered output ordering.
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    fn drain(&mut self) -> io::Result<()> {
        while self.output_pos < self.output_len {
            match self.inner.write(&self.output[self.output_pos..self.output_len]) {
                Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
                Ok(len) => self.output_pos += len,
                Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                Err(err) => return Err(err),
            }
        }
        self.output_pos = 0;
        self.output_len = 0;
        Ok(())
    }

    /// Finish and flush without consuming the writer, allowing retries after sink errors.
    /// No more input can be written after conversion finishes.
    pub fn try_finish(&mut self) -> io::Result<()> {
        self.drain()?;
        if !self.finished {
            let progress = self
                .converter
                .convert(&self.input[..self.input_len], &mut self.output, true)
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
            self.input.copy_within(progress.read..self.input_len, 0);
            self.input_len -= progress.read;
            self.output_len = progress.written;
            self.finished = progress.status == Status::Finished;
            self.drain()?;
        }
        self.inner.flush()
    }

    /// Finish the stream and return its writer. Use [`try_finish()`](Self::try_finish) when
    /// the writer must be recoverable after an error.
    pub fn finish(mut self) -> io::Result<W> {
        self.try_finish()?;
        Ok(self.inner)
    }
}

impl<W: Write> Write for Transcoder<W> {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        if input.is_empty() {
            return Ok(0);
        }
        if self.finished {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        self.drain()?;
        if self.input_len != 0 {
            let copied = input.len().min(self.input.len() - self.input_len);
            self.input[self.input_len..self.input_len + copied].copy_from_slice(&input[..copied]);
            self.input_len += copied;
            let progress = self
                .converter
                .convert(&self.input[..self.input_len], &mut self.output, false)
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
            self.input.copy_within(progress.read..self.input_len, 0);
            self.input_len -= progress.read;
            self.output_len = progress.written;
            return Ok(copied);
        }
        let progress = self
            .converter
            .convert(input, &mut self.output, false)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        self.output_len = progress.written;
        if progress.read != 0 {
            return Ok(progress.read);
        }
        // No character could be read: preserve its prefix until the next write or finish.
        let copied = input.len().min(self.input.len());
        self.input[..copied].copy_from_slice(&input[..copied]);
        self.input_len = copied;
        Ok(copied)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.drain()?;
        self.inner.flush()
    }
}

macro_rules! adapter {
    ($name:ident, $doc:literal, $from:expr, $to:expr) => {
        #[doc = $doc]
        pub struct $name<W>(Transcoder<W>);

        impl<W: Write> $name<W> {
            /// Create a strict stream using `encoding` on the non-UTF-8 side.
            pub fn new(inner: W, encoding: Encoding) -> Self {
                Self(Transcoder::new(inner, $from(encoding), $to(encoding)))
            }
            /// Access the underlying writer.
            pub fn get_ref(&self) -> &W {
                self.0.get_ref()
            }
            /// Access the writer mutably. Direct writes can disrupt output ordering.
            pub fn get_mut(&mut self) -> &mut W {
                self.0.get_mut()
            }
            /// Check the last character and flush, retaining the adapter if the sink fails.
            pub fn try_finish(&mut self) -> io::Result<()> {
                self.0.try_finish()
            }
            /// Finish conversion and return the writer.
            pub fn finish(self) -> io::Result<W> {
                self.0.finish()
            }
        }

        impl<W: Write> Write for $name<W> {
            fn write(&mut self, input: &[u8]) -> io::Result<usize> {
                self.0.write(input)
            }
            fn flush(&mut self) -> io::Result<()> {
                self.0.flush()
            }
        }
    };
}

adapter!(
    Decoder,
    "A writer decoding bytes from `encoding` to UTF-8. Finish explicitly to validate EOF.",
    |encoding| encoding,
    |_| Encoding::Utf8
);
adapter!(
    Encoder,
    "A writer encoding UTF-8 into `encoding`. Finish explicitly to validate EOF.",
    |_| Encoding::Utf8,
    |encoding| encoding
);
