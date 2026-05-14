use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

use futures_io::AsyncWrite;
use pin_project_lite::pin_project;

use crate::{
    client::{MessageKind, WriteMode, async_io::ExtendedBufRead},
    packetline::async_io::{Writer, encode},
};

pin_project! {
    /// A [`Write`][io::Write] implementation optimized for writing packet lines.
    /// A type implementing `Write` for packet lines, which when done can be transformed into a `Read` for
    /// obtaining the response.
    pub struct RequestWriter<'a> {
        on_into_read: MessageKind,
        #[pin]
        writer: Writer<Box<dyn AsyncWrite + Unpin + 'a>>,
        reader: Box<dyn ExtendedBufRead<'a> + Unpin + 'a>,
        trace: bool,
    }
}
impl futures_io::AsyncWrite for RequestWriter<'_> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        self.project().writer.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.project().writer.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.project().writer.poll_close(cx)
    }
}

/// methods with bonds to IO
impl<'a> RequestWriter<'a> {
    /// Create a new instance from a `writer` (commonly a socket), a `reader` into which to transform once the
    /// writes are finished, along with configuration for the `write_mode` and information about which message to write
    /// when this instance is converted [into a `reader`][RequestWriter::into_read()] to read the request's response.
    /// If `trace` is true, `gix_trace` will be used on every written message or data.
    pub fn new_from_bufread<W: AsyncWrite + Unpin + 'a>(
        writer: W,
        reader: Box<dyn ExtendedBufRead<'a> + Unpin + 'a>,
        write_mode: WriteMode,
        on_into_read: MessageKind,
        trace: bool,
    ) -> Self {
        let mut writer = Writer::new(Box::new(writer) as Box<dyn AsyncWrite + Unpin>);
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
    pub async fn write_message(&mut self, message: MessageKind) -> io::Result<()> {
        match message {
            MessageKind::Flush => {
                if self.trace {
                    gix_features::trace::trace!(">> FLUSH");
                }
                encode::write_packet_line(&gix_packetline::PacketLineRef::Flush, self.writer.inner_mut()).await
            }
            MessageKind::Delimiter => {
                if self.trace {
                    gix_features::trace::trace!(">> DELIM");
                }
                encode::write_packet_line(&gix_packetline::PacketLineRef::Delimiter, self.writer.inner_mut()).await
            }
            MessageKind::ResponseEnd => {
                if self.trace {
                    gix_features::trace::trace!(">> RESPONSE_END");
                }
                encode::write_packet_line(&gix_packetline::PacketLineRef::ResponseEnd, self.writer.inner_mut()).await
            }
            MessageKind::Text(t) => {
                #[allow(unused_variables, unused_imports)]
                if self.trace {
                    use bstr::ByteSlice;
                    gix_features::trace::trace!(">> {}", t.as_bstr());
                }
                encode::write_text(&gix_packetline::TextRef::from(t), self.writer.inner_mut()).await
            }
        }
        .map(|_| ())
    }
    /// Discard the ability to write and turn this instance into the reader for obtaining the other side's response.
    ///
    /// Doing so will also write the message type this instance was initialized with.
    pub async fn into_read(mut self) -> std::io::Result<Box<dyn ExtendedBufRead<'a> + Unpin + 'a>> {
        use futures_lite::AsyncWriteExt;
        self.write_message(self.on_into_read).await?;
        self.writer.inner_mut().flush().await?;
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
    pub fn into_parts(
        self,
    ) -> (
        Box<dyn AsyncWrite + Unpin + 'a>,
        Box<dyn ExtendedBufRead<'a> + Unpin + 'a>,
    ) {
        (self.writer.into_inner(), self.reader)
    }
}
