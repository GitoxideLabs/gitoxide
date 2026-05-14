use std::{io, io::Write};

use crate::{
    client::{MessageKind, WriteMode, blocking_io::ExtendedBufRead},
    packetline::blocking_io::{Writer, encode},
};

/// A [`Write`][io::Write] implementation optimized for writing packet lines.
/// A type implementing `Write` for packet lines, which when done can be transformed into a `Read` for
/// obtaining the response.
pub struct RequestWriter<'a> {
    on_into_read: MessageKind,
    writer: Writer<Box<dyn io::Write + 'a>>,
    reader: Box<dyn ExtendedBufRead<'a> + Unpin + 'a>,
    trace: bool,
}

impl io::Write for RequestWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        #[allow(unused_imports)]
        if self.trace {
            use bstr::ByteSlice;
            gix_features::trace::trace!(">> {}", buf.as_bstr());
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
        let mut writer = Writer::new(Box::new(writer) as Box<dyn io::Write>);
        match write_mode {
            WriteMode::Binary => writer.enable_binary_mode(),
            WriteMode::OneLfTerminatedLinePerWriteCall => writer.enable_text_mode(),
        }
        RequestWriter {
            on_into_read,
            writer,
            reader,
            trace,
        }
    }

    /// Write the given message as packet line.
    pub fn write_message(&mut self, message: MessageKind) -> io::Result<()> {
        match message {
            MessageKind::Flush => {
                if self.trace {
                    gix_features::trace::trace!(">> FLUSH");
                }
                encode::write_packet_line(&gix_packetline::PacketLineRef::Flush, self.writer.inner_mut())
            }
            MessageKind::Delimiter => {
                if self.trace {
                    gix_features::trace::trace!(">> DELIM");
                }
                encode::write_packet_line(&gix_packetline::PacketLineRef::Delimiter, self.writer.inner_mut())
            }
            MessageKind::ResponseEnd => {
                if self.trace {
                    gix_features::trace::trace!(">> RESPONSE_END");
                }
                encode::write_packet_line(&gix_packetline::PacketLineRef::ResponseEnd, self.writer.inner_mut())
            }
            MessageKind::Text(t) => {
                #[allow(unused_variables, unused_imports)]
                if self.trace {
                    use bstr::ByteSlice;
                    gix_features::trace::trace!(">> {}", t.as_bstr());
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
        (self.writer.into_inner(), self.reader)
    }
}
