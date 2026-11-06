use std::io::{self, Read};

use crate::{Converter, Encoding, convert::Status};

/// A reader that converts from one encoding to another with at most 16 KiB of buffering.
///
/// Errors from the source are propagated. Encoding errors have [`io::ErrorKind::InvalidData`].
/// It may read ahead; [`into_inner()`](Self::into_inner) discards buffered source and output.
pub struct Transcoder<R> {
    inner: R,
    converter: Converter,
    input: Vec<u8>,
    input_len: usize,
    output: Vec<u8>,
    output_pos: usize,
    output_len: usize,
    need_input: bool,
    eof: bool,
    finished: bool,
    failed: bool,
}

impl<R: Read> Transcoder<R> {
    /// Wrap `inner` in a strict conversion stream.
    pub fn new(inner: R, from: Encoding, to: Encoding) -> Self {
        Self {
            inner,
            converter: Converter::new(from, to),
            input: vec![0; 8192],
            input_len: 0,
            output: vec![0; 8192],
            output_pos: 0,
            output_len: 0,
            need_input: true,
            eof: false,
            finished: false,
            failed: false,
        }
    }

    /// Access the underlying reader.
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    /// Access the underlying reader mutably. Reading from it may skip buffered stream data.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Extract the reader, discarding any buffered bytes.
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R: Read> Read for Transcoder<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        loop {
            if self.output_pos < self.output_len {
                let len = out.len().min(self.output_len - self.output_pos);
                out[..len].copy_from_slice(&self.output[self.output_pos..self.output_pos + len]);
                self.output_pos += len;
                return Ok(len);
            }
            if self.failed {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "encoding reader failed"));
            }
            if self.finished {
                return Ok(0);
            }
            if self.need_input && !self.eof {
                let len = loop {
                    match self.inner.read(&mut self.input[self.input_len..]) {
                        Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                        result => break result?,
                    }
                };
                self.input_len += len;
                self.eof = len == 0;
            }
            let progress = match self
                .converter
                .convert(&self.input[..self.input_len], &mut self.output, self.eof)
            {
                Ok(progress) => progress,
                Err(err) => {
                    self.failed = true;
                    return Err(io::Error::new(io::ErrorKind::InvalidData, err));
                }
            };
            self.input.copy_within(progress.read..self.input_len, 0);
            self.input_len -= progress.read;
            self.output_pos = 0;
            self.output_len = progress.written;
            self.need_input = progress.status == Status::InputEmpty;
            self.finished = progress.status == Status::Finished;
        }
    }
}

macro_rules! adapter {
    ($name:ident, $doc:literal, $from:expr, $to:expr) => {
        #[doc = $doc]
        pub struct $name<R>(Transcoder<R>);

        impl<R: Read> $name<R> {
            /// Create a strict stream using `encoding` on the non-UTF-8 side.
            pub fn new(inner: R, encoding: Encoding) -> Self {
                Self(Transcoder::new(inner, $from(encoding), $to(encoding)))
            }
            /// Access the underlying reader.
            pub fn get_ref(&self) -> &R {
                self.0.get_ref()
            }
            /// Access the underlying reader mutably. Direct reads may skip buffered data.
            pub fn get_mut(&mut self) -> &mut R {
                self.0.get_mut()
            }
            /// Extract the reader, discarding buffered bytes.
            pub fn into_inner(self) -> R {
                self.0.into_inner()
            }
        }

        impl<R: Read> Read for $name<R> {
            fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
                self.0.read(out)
            }
        }
    };
}

adapter!(
    Decoder,
    "A reader decoding bytes from `encoding` to UTF-8.",
    |encoding| encoding,
    |_| Encoding::Utf8
);
adapter!(
    Encoder,
    "A reader encoding UTF-8 bytes into `encoding`.",
    |_| Encoding::Utf8,
    |encoding| encoding
);
