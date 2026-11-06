use gix_error::{ErrorExt, Result};

use crate::{Converter, Encoding, legacy, unicode};

/// The reason a strict conversion could not continue.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    /// The encoding is only available for output, as with Git's UTF-16BE-BOM extension.
    UnsupportedSource,
    /// The source contains an invalid byte sequence.
    InvalidSequence,
    /// The last input ends inside a character or escape sequence.
    IncompleteSequence,
    /// The destination cannot represent a source character.
    Unrepresentable,
    /// The converter has already finished or failed and must be reset.
    Closed,
}

/// A conversion failure, available through [`gix_error::Error::downcast_any_ref()`].
#[derive(Clone, Debug)]
pub struct ConversionError {
    /// The source encoding.
    pub from: Encoding,
    /// The destination encoding.
    pub to: Encoding,
    /// The byte offset in the entire source stream.
    pub offset: u64,
    /// Why conversion failed.
    pub kind: ErrorKind,
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let reason = match self.kind {
            ErrorKind::UnsupportedSource => "encoding only supports output",
            ErrorKind::InvalidSequence => "invalid byte sequence",
            ErrorKind::IncompleteSequence => "incomplete byte sequence",
            ErrorKind::Unrepresentable => "unrepresentable character",
            ErrorKind::Closed => "converter already finished or failed",
        };
        write!(
            f,
            "{reason} at byte {} converting {} to {}",
            self.offset, self.from, self.to
        )
    }
}

impl std::error::Error for ConversionError {}

/// Why a streaming conversion stopped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    /// More input is needed. Resubmit any unconsumed, incomplete character with the next chunk.
    InputEmpty,
    /// More output space is needed. Resubmit the unconsumed input with a fresh output buffer.
    OutputFull,
    /// All input and buffered output have been converted, including end-of-stream validation.
    Finished,
}

/// Input and output progress for one call to [`Converter::convert()`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Progress {
    /// The number of source bytes consumed in this call.
    pub read: usize,
    /// The number of destination bytes initialized in this call.
    pub written: usize,
    /// Why conversion stopped.
    pub status: Status,
}

impl Converter {
    /// Create a new conversion stream.
    pub fn new(from: Encoding, to: Encoding) -> Self {
        Self {
            from,
            to,
            source: legacy::State::default(),
            destination: legacy::State::default(),
            unicode_source: unicode::State::default(),
            output_started: false,
            offset: 0,
            finished: false,
            failed: false,
        }
    }

    /// Begin an independent stream with the same encodings.
    pub fn reset(&mut self) {
        *self = Self::new(self.from, self.to);
    }

    /// The source encoding.
    pub fn from(&self) -> Encoding {
        self.from
    }

    /// The destination encoding.
    pub fn to(&self) -> Encoding {
        self.to
    }

    fn error(&mut self, kind: ErrorKind, read: usize) -> gix_error::Error {
        self.failed = true;
        ConversionError {
            from: self.from,
            to: self.to,
            offset: self.offset.saturating_add(read as u64),
            kind,
        }
        .raise()
    }

    /// Convert a chunk, with `last` indicating that no more source bytes will follow it.
    ///
    /// Keep calling after [`Status::OutputFull`], including with empty input if all bytes have
    /// been consumed. Once finished, empty calls are idempotent; new input requires [`reset()`](Self::reset).
    /// After an error the converter must be reset. Output written by a failed call is unspecified.
    pub fn convert(&mut self, input: &[u8], output: &mut [u8], last: bool) -> Result<Progress> {
        if self.from == Encoding::Utf16BeBom {
            return Err(self.error(ErrorKind::UnsupportedSource, 0));
        }
        if self.failed || (self.finished && !input.is_empty()) {
            return Err(self.error(ErrorKind::Closed, 0));
        }
        let mut progress = Progress {
            read: 0,
            written: 0,
            status: Status::InputEmpty,
        };
        if self.finished {
            progress.status = Status::Finished;
            return Ok(progress);
        }
        // These are explicit Git extensions, which prepend a BOM even for empty input.
        if !self.output_started && matches!(self.to, Encoding::Utf16LeBom | Encoding::Utf16BeBom) {
            let bom = unicode::bom(self.to);
            if output.len() < bom.len() {
                progress.status = Status::OutputFull;
                return Ok(progress);
            }
            output[..bom.len()].copy_from_slice(bom);
            progress.written = bom.len();
            self.output_started = true;
        }
        loop {
            if progress.read < input.len() && (self.output_started || unicode::bom(self.to).is_empty()) {
                use Encoding::*;
                let from =
                    match self.from {
                        Utf16 | Utf16LeBom | Utf16BeBom if self.unicode_source.started => {
                            if self.unicode_source.little { Utf16Le } else { Utf16Be }
                        }
                        Utf32 if self.unicode_source.started => {
                            if self.unicode_source.little {
                                Utf32Le
                            } else {
                                Utf32Be
                            }
                        }
                        from => from,
                    };
                let to = match self.to {
                    Utf16 if cfg!(target_endian = "little") => Utf16Le,
                    Utf16 => Utf16Be,
                    Utf16LeBom => Utf16Le,
                    Utf16BeBom => Utf16Be,
                    Utf32 if cfg!(target_endian = "little") => Utf32Le,
                    Utf32 => Utf32Be,
                    to => to,
                };
                let (read, written) =
                    crate::fast::run(from, to, &input[progress.read..], &mut output[progress.written..]);
                progress.read += read;
                progress.written += written;
                if read != 0 {
                    self.output_started = true;
                }
                let (read, written) = legacy::bulk(
                    from,
                    to,
                    &input[progress.read..],
                    &mut output[progress.written..],
                    &mut self.source,
                    &mut self.destination,
                );
                progress.read += read;
                progress.written += written;
                if read != 0 {
                    self.output_started = true;
                }
            }
            let mut source = self.source.clone();
            let mut unicode_source = self.unicode_source.clone();
            let decoded = if progress.read == input.len() {
                if !last || unicode::is_unicode(self.from) {
                    break;
                }
                match legacy::decode_finish(self.from, &mut source) {
                    Some(ch) => Ok(legacy::Decoded::Scalar(ch, 0)),
                    None => break,
                }
            } else if unicode::is_unicode(self.from) {
                unicode::decode(self.from, &input[progress.read..], &mut unicode_source)
            } else {
                legacy::decode(self.from, &input[progress.read..], &mut source)
            };
            let (first, second, consumed) = match decoded {
                Ok(legacy::Decoded::Scalar(ch, consumed)) => (ch, None, consumed),
                Ok(legacy::Decoded::Pair(first, second, consumed)) => (first, Some(second), consumed),
                Ok(legacy::Decoded::Skip(consumed)) => {
                    self.source = source;
                    self.unicode_source = unicode_source;
                    progress.read += consumed;
                    continue;
                }
                Ok(legacy::Decoded::Incomplete) => {
                    if last {
                        return Err(self.error(ErrorKind::IncompleteSequence, progress.read));
                    }
                    break;
                }
                Err(()) => return Err(self.error(ErrorKind::InvalidSequence, progress.read)),
            };
            let mut destination = self.destination.clone();
            let mut encoded = [0; 32];
            let mut written = 0;
            for ch in [Some(first), second].into_iter().flatten() {
                let mut bytes = [0; 16];
                let len = if unicode::is_unicode(self.to) {
                    unicode::encode(self.to, ch, &mut bytes)
                } else {
                    match legacy::encode(self.to, ch, &mut destination, &mut bytes) {
                        Ok(len) => len,
                        Err(()) => return Err(self.error(ErrorKind::Unrepresentable, progress.read)),
                    }
                };
                encoded[written..written + len].copy_from_slice(&bytes[..len]);
                written += len;
            }
            let bom = if self.output_started {
                &[][..]
            } else {
                unicode::bom(self.to)
            };
            if output.len() - progress.written < bom.len() + written {
                progress.status = Status::OutputFull;
                break;
            }
            output[progress.written..progress.written + bom.len()].copy_from_slice(bom);
            progress.written += bom.len();
            output[progress.written..progress.written + written].copy_from_slice(&encoded[..written]);
            progress.written += written;
            progress.read += consumed;
            self.output_started = true;
            self.source = source;
            self.unicode_source = unicode_source;
            self.destination = destination;
        }
        if last && progress.read == input.len() && progress.status != Status::OutputFull {
            let mut destination = self.destination.clone();
            let mut bytes = [0; 16];
            let len = if unicode::is_unicode(self.to) {
                0
            } else {
                match legacy::finish(self.to, &mut destination, &mut bytes) {
                    Ok(len) => len,
                    Err(()) => return Err(self.error(ErrorKind::Unrepresentable, progress.read)),
                }
            };
            if output.len() - progress.written < len {
                progress.status = Status::OutputFull;
            } else {
                output[progress.written..progress.written + len].copy_from_slice(&bytes[..len]);
                progress.written += len;
                self.destination = destination;
                self.finished = true;
                progress.status = Status::Finished;
            }
        }
        self.offset = self.offset.saturating_add(progress.read as u64);
        Ok(progress)
    }
}
