use std::{
    io,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        mpsc,
    },
};

use gix_error::{ErrorExt, Result, ResultExt, bail, message};
use gix_transport::{
    Protocol,
    client::{
        MessageKind, WriteMode,
        blocking_io::{ExtendedBufRead, Transport},
    },
    packetline::{self, PacketLineRef, blocking_io::encode},
};

use super::{Request, Response};

const UPLOAD_FAILED: u8 = 1;
const RESPONSE_FAILED: u8 = 2;

/// Transport and shallow-boundary settings for [`Request::send()`].
#[derive(Clone, Copy, Debug)]
pub struct SendOptions<'a> {
    /// The protocol version selected by the receive-pack handshake.
    pub server_protocol: Protocol,
    /// The source repository's shallow commit boundaries, advertised before commands.
    pub shallow: &'a [gix_hash::ObjectId],
    /// Trace outgoing packet lines using `gix-trace`.
    pub trace_packetlines: bool,
}

impl Default for SendOptions<'_> {
    fn default() -> Self {
        SendOptions {
            server_protocol: Protocol::V1,
            shallow: &[],
            trace_packetlines: false,
        }
    }
}

impl Request {
    /// Send the commands and a pack using a transport already handshaken with receive-pack.
    ///
    /// `write_pack` runs on an upload thread while this thread drains remote progress
    /// through `progress(is_error, bytes)`. It must write one complete pack whenever
    /// a command creates or updates a reference; it is not called for deletion-only
    /// requests. Respect [`Self::features()`] when generating deltas.
    ///
    /// `should_interrupt` is checked between packet reads and writes. On errors,
    /// stateful transports are aborted to unblock both directions; such transports
    /// must provide a request abort callback. Stateless transports close their owned
    /// request body when the upload ends. This operation consumes the receive-pack
    /// exchange, and the connection must not be reused afterward.
    ///
    /// Return `None` for an empty command list, otherwise return the complete status
    /// report. Per-reference rejections are represented in the report, while I/O,
    /// pack-production, and malformed-response failures are returned as errors.
    pub fn send(
        &self,
        transport: &mut impl Transport,
        write_pack: impl FnOnce(&mut dyn io::Write) -> Result<()> + Send,
        mut progress: impl FnMut(bool, &[u8]) -> io::Result<()>,
        should_interrupt: &AtomicBool,
        options: SendOptions<'_>,
    ) -> Result<Option<Response>> {
        if options.server_protocol == Protocol::V2 {
            bail!(message("Receive-pack does not support protocol version 2").unsupported());
        }
        if should_interrupt.load(Ordering::Relaxed) {
            bail!(message("Push interrupted before sending commands").cancelled());
        }
        if options
            .shallow
            .iter()
            .any(|id| id.kind() != self.object_hash || id.is_null())
        {
            bail!(message("Shallow boundary IDs must be non-null and use the repository's object format").validation());
        }
        if self.commands.is_empty() {
            crate::indicate_end_of_interaction(transport, options.trace_packetlines)
                .or_raise(|| message("Could not finish the empty push"))?;
            return Ok(None);
        }
        let stateful = transport.connection_persists_across_multiple_requests();
        let mut request = transport
            .request(WriteMode::Binary, MessageKind::Flush, options.trace_packetlines)
            .or_raise(|| message("Could not open a push request"))?;
        let abort = request.take_abort();
        if stateful && abort.is_none() {
            bail!(message("The transport cannot abort an incomplete push upload").unsupported());
        }
        let (writer, mut reader) = request.into_parts_with_send()?;
        let failed = AtomicU8::new(0);
        let (written, received) = std::thread::scope(|scope| {
            let (cancel, cancellation) = mpsc::channel();
            scope.spawn(move || {
                if cancellation.recv().is_ok()
                    && let Some(abort) = abort
                {
                    // Preserve the original producer or consumer failure.
                    let _ = abort();
                }
            });
            let failed = &failed;
            let upload_cancel = cancel.clone();
            let upload = scope.spawn(move || {
                let mut on_failure = AbortOnFailure::new(upload_cancel, failed, UPLOAD_FAILED);
                let mut writer = InterruptibleWriter {
                    inner: writer,
                    failed,
                    should_interrupt,
                };
                let result = (|| {
                    for id in options.shallow {
                        write_packet(
                            format!("shallow {id}").as_bytes(),
                            &mut writer,
                            options.trace_packetlines,
                        )?;
                    }
                    for line in self.command_lines() {
                        write_packet(&line, &mut writer, options.trace_packetlines)?;
                    }
                    encode::flush_to_write(&mut writer).or_raise(|| message("Could not flush push arguments"))?;
                    if !self.push_options.is_empty() {
                        for option in &self.push_options {
                            write_packet(option, &mut writer, options.trace_packetlines)?;
                        }
                        encode::flush_to_write(&mut writer).or_raise(|| message("Could not flush push arguments"))?;
                    }
                    if self.commands.iter().any(|command| !command.new_id.is_null()) {
                        write_pack(&mut writer).or_raise(|| message("Could not produce the push pack"))?;
                    }
                    io::Write::flush(&mut writer).or_raise(|| message("Could not finish the push upload"))?;
                    Ok(())
                })();
                if result.is_ok() {
                    on_failure.disarm();
                }
                result
            });
            let mut on_failure = AbortOnFailure::new(cancel, failed, RESPONSE_FAILED);
            let received = self.receive_response(&mut reader, &mut progress, should_interrupt);
            if received.is_ok() {
                on_failure.disarm();
            }
            drop(on_failure);
            drop(reader);
            let written = upload
                .join()
                .unwrap_or_else(|_| Err(message("The push pack producer panicked").raise()));
            (written, received)
        });
        if failed.load(Ordering::Relaxed) == RESPONSE_FAILED {
            return received.map(Some);
        }
        written?;
        received.map(Some)
    }

    fn receive_response<'a>(
        &self,
        reader: &mut (impl ExtendedBufRead<'a> + ?Sized),
        progress: &mut impl FnMut(bool, &[u8]) -> io::Result<()>,
        should_interrupt: &AtomicBool,
    ) -> Result<Response> {
        let mut lines = Vec::new();
        let mut multiplexed = Vec::new();
        while let Some(line) = reader.readline() {
            if should_interrupt.load(Ordering::Relaxed) {
                bail!(message("Push interrupted while receiving the status report").cancelled());
            }
            let line = line.or_raise(|| message("Could not read the push status report"))??;
            if self.features.side_band_64k {
                match line.decode_band()? {
                    packetline::BandRef::Data(data) => multiplexed.extend_from_slice(data),
                    packetline::BandRef::Progress(data) => {
                        progress(false, data).or_raise(|| message("Could not deliver remote push progress"))?;
                    }
                    packetline::BandRef::Error(data) => {
                        progress(true, data).or_raise(|| message("Could not deliver the remote push error"))?;
                        bail!("Remote push failed: {}", bstr::BStr::new(data));
                    }
                }
            } else {
                match line {
                    PacketLineRef::Data(data) => lines.push(data.to_vec()),
                    _ => bail!(message("Unexpected delimiter in push status report").corrupted()),
                }
            }
        }
        if reader.stopped_at() != Some(MessageKind::Flush) {
            bail!(message("The push status report ended without a flush packet").corrupted());
        }
        if self.features.side_band_64k {
            let mut bytes = multiplexed.as_slice();
            loop {
                match packetline::decode::streaming(bytes)? {
                    packetline::decode::Stream::Complete { line, bytes_consumed } => {
                        bytes = &bytes[bytes_consumed..];
                        match line {
                            PacketLineRef::Data(data) => lines.push(data.to_vec()),
                            PacketLineRef::Flush if bytes.is_empty() => break,
                            _ => bail!(message("Unexpected data after the push status report").corrupted()),
                        }
                    }
                    packetline::decode::Stream::Incomplete { .. } => {
                        bail!(message("Incomplete sideband push status report").corrupted())
                    }
                }
            }
        }
        self.parse_response(lines)
    }
}

fn write_packet(data: &[u8], writer: &mut impl io::Write, trace: bool) -> Result<()> {
    if trace {
        gix_trace::trace!(">> {}", bstr::BStr::new(data));
    }
    encode::data_to_write(data, writer).or_raise(|| message("Could not send push arguments"))?;
    Ok(())
}

struct AbortOnFailure<'a> {
    cancel: Option<mpsc::Sender<()>>,
    failed: &'a AtomicU8,
    source: u8,
}

impl<'a> AbortOnFailure<'a> {
    fn new(cancel: mpsc::Sender<()>, failed: &'a AtomicU8, source: u8) -> Self {
        Self {
            cancel: Some(cancel),
            failed,
            source,
        }
    }
    fn disarm(&mut self) {
        self.cancel = None;
    }
}

impl Drop for AbortOnFailure<'_> {
    fn drop(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            let _ = self
                .failed
                .compare_exchange(0, self.source, Ordering::Relaxed, Ordering::Relaxed);
            let _ = cancel.send(());
        }
    }
}

struct InterruptibleWriter<'a> {
    inner: Box<dyn io::Write + Send + 'a>,
    failed: &'a AtomicU8,
    should_interrupt: &'a AtomicBool,
}

impl io::Write for InterruptibleWriter<'_> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if self.failed.load(Ordering::Relaxed) != 0 || self.should_interrupt.load(Ordering::Relaxed) {
            return Err(io::Error::other(message("Push upload interrupted").cancelled()));
        }
        self.inner.write(data)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
