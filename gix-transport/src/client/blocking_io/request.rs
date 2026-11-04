use std::{io, io::Write};

use gix_error::Result;

use crate::{
    client::{MessageKind, WriteMode, blocking_io::ExtendedBufRead},
    packetline::blocking_io::{Writer, encode},
};

/// Close an in-progress request after a producer or consumer fails.
///
/// The callback must interrupt pending I/O in either direction. Dropping it
/// without calling it leaves the connection intact.
pub type Abort<'a> = Box<dyn FnOnce() -> io::Result<()> + Send + 'a>;

/// A [`Write`][io::Write] implementation optimized for writing packet lines.
/// A type implementing `Write` for packet lines, which when done can be transformed into a `Read` for
/// obtaining the response.
pub struct RequestWriter<'a> {
    on_into_read: MessageKind,
    writer: Writer<WriterKind<'a>>,
    reader: Box<dyn ExtendedBufRead<'a> + Unpin + 'a>,
    trace: bool,
    abort: Option<Abort<'a>>,
}

enum WriterKind<'a> {
    Local(Box<dyn io::Write + 'a>),
    Send(Box<dyn io::Write + Send + 'a>),
}

impl io::Write for WriterKind<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Local(writer) => writer.write(buf),
            Self::Send(writer) => writer.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Local(writer) => writer.flush(),
            Self::Send(writer) => writer.flush(),
        }
    }
}

impl io::Write for RequestWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.trace {
            gix_trace::trace!(">> {}", bstr::BStr::new(buf));
        }
        self.writer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

/// methods with bonds to IO
impl<'a> RequestWriter<'a> {
    /// Create a new instance from a `writer` (commonly a socket), a `reader` into which to transform once the
    /// writes are finished, along with configuration for the `write_mode` and information about which message to write
    /// when this instance is converted into a `reader` to read the request's response.
    /// If `trace` is true, `gix_trace` will be used on every written message or data.
    pub fn new_from_bufread<W: io::Write + 'a>(
        writer: W,
        reader: Box<dyn ExtendedBufRead<'a> + Unpin + 'a>,
        write_mode: WriteMode,
        on_into_read: MessageKind,
        trace: bool,
    ) -> Self {
        Self::with_writer(
            WriterKind::Local(Box::new(writer)),
            reader,
            write_mode,
            on_into_read,
            trace,
        )
    }

    /// Like [`Self::new_from_bufread()`], retaining the writer's `Send` bound so
    /// [`Self::into_parts_with_send()`] can upload data on another thread while
    /// the calling thread drains the response. This avoids deadlocks when the
    /// remote emits progress before the upload finishes.
    pub fn new_from_bufread_with_send<W: io::Write + Send + 'a>(
        writer: W,
        reader: Box<dyn ExtendedBufRead<'a> + Unpin + 'a>,
        write_mode: WriteMode,
        on_into_read: MessageKind,
        trace: bool,
    ) -> Self {
        Self::with_writer(
            WriterKind::Send(Box::new(writer)),
            reader,
            write_mode,
            on_into_read,
            trace,
        )
    }

    fn with_writer(
        writer: WriterKind<'a>,
        reader: Box<dyn ExtendedBufRead<'a> + Unpin + 'a>,
        write_mode: WriteMode,
        on_into_read: MessageKind,
        trace: bool,
    ) -> Self {
        let mut writer = Writer::new(writer);
        match write_mode {
            WriteMode::Binary => writer.enable_binary_mode(),
            WriteMode::OneLfTerminatedLinePerWriteCall => writer.enable_text_mode(),
        }
        RequestWriter {
            on_into_read,
            writer,
            reader,
            trace,
            abort: None,
        }
    }

    /// Install a callback that closes the connection if a concurrent upload or
    /// response fails. Callers obtain it with [`Self::take_abort()`].
    pub fn with_abort(mut self, abort: impl FnOnce() -> io::Result<()> + Send + 'a) -> Self {
        self.abort = Some(Box::new(abort));
        self
    }

    /// Remove the abort callback, without invoking it.
    pub fn take_abort(&mut self) -> Option<Abort<'a>> {
        self.abort.take()
    }

    /// Write the given message as packet line.
    pub fn write_message(&mut self, message: MessageKind) -> io::Result<()> {
        match message {
            MessageKind::Flush => {
                if self.trace {
                    gix_trace::trace!(">> FLUSH");
                }
                encode::write_packet_line(&gix_packetline::PacketLineRef::Flush, self.writer.inner_mut())
            }
            MessageKind::Delimiter => {
                if self.trace {
                    gix_trace::trace!(">> DELIM");
                }
                encode::write_packet_line(&gix_packetline::PacketLineRef::Delimiter, self.writer.inner_mut())
            }
            MessageKind::ResponseEnd => {
                if self.trace {
                    gix_trace::trace!(">> RESPONSE_END");
                }
                encode::write_packet_line(&gix_packetline::PacketLineRef::ResponseEnd, self.writer.inner_mut())
            }
            MessageKind::Text(t) => {
                if self.trace {
                    gix_trace::trace!(">> {}", bstr::BStr::new(t));
                }
                encode::write_text(&gix_packetline::TextRef::from(t), self.writer.inner_mut())
            }
        }
        .map(|_| ())
    }

    /// Discard the ability to write and turn this instance into the reader for obtaining the other side's response.
    ///
    /// Doing so will also write the message type this instance was initialized with.
    pub fn into_read(mut self) -> std::io::Result<Box<dyn ExtendedBufRead<'a> + Unpin + 'a>> {
        self.write_message(self.on_into_read)?;
        self.writer.inner_mut().flush()?;
        Ok(self.reader)
    }

    /// Dissolve this instance into its write and read handles without any message-writing side-effect as in [`RequestWriter::into_read()`].
    ///
    /// Furthermore, the writer will not encode everything it writes as packetlines, but write everything verbatim into the
    /// underlying channel.
    ///
    /// # Note
    ///
    /// It's of utmost importance to drop the request writer before reading the response as these might be inter-dependent, depending on
    /// the underlying transport mechanism. Failure to do so may result in a deadlock depending on how the write and read mechanism
    /// is implemented.
    pub fn into_parts(self) -> (Box<dyn io::Write + 'a>, Box<dyn ExtendedBufRead<'a> + Unpin + 'a>) {
        let writer: Box<dyn io::Write + 'a> = match self.writer.into_inner() {
            WriterKind::Local(writer) => writer,
            WriterKind::Send(writer) => writer,
        };
        (writer, self.reader)
    }

    /// Like [`Self::into_parts()`], retaining the writer's `Send` bound for
    /// concurrent uploads. No delimiter is written and bytes written to the
    /// returned writer are sent verbatim.
    ///
    /// Returns an unsupported-operation error if this request was constructed
    /// without [`Self::new_from_bufread_with_send()`].
    pub fn into_parts_with_send(
        self,
    ) -> Result<(
        Box<dyn io::Write + Send + 'a>,
        Box<dyn ExtendedBufRead<'a> + Unpin + 'a>,
    )> {
        match self.writer.into_inner() {
            WriterKind::Send(writer) => Ok((writer, self.reader)),
            WriterKind::Local(_) => {
                Err(gix_error::message("The transport does not support concurrent request writes").unsupported_error())
            }
        }
    }
}
