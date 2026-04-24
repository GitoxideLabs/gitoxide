//! The send-pack orchestrator: given a post-handshake Transport and a
//! Request + pack entries, drive the exchange and return the parsed report.
//!
//! Reference: `vendor/git/send-pack.c` (function `send_pack`, ~line 560).
//! Sequence:
//!   1. Acquire a request handle and immediately dissolve it into (raw_writer, reader).
//!   2. Write the command list as pkt-line frames directly to raw_writer.
//!   3. If any command is a non-delete, write the pack as raw bytes to raw_writer.
//!   4. Drop raw_writer so the transport's read path is unblocked.
//!   5. Reset the reader; enable sideband demux.
//!   6. Read and parse the report-status response.

#[cfg(feature = "blocking-client")]
pub use blocking::send_pack;

#[cfg(feature = "blocking-client")]
mod blocking {
    use gix_transport::client::{blocking_io::Transport, MessageKind, WriteMode};

    use crate::send_pack::{
        command_list::encode_into, pack_writer::write_pack, report::parse as parse_report,
        response::blocking_io::enable_sideband_demux, Error, Options, Outcome, Request,
    };

    /// Drive one send-pack exchange over `transport`.
    ///
    /// Preconditions: `transport` must already have completed the `receive-pack`
    /// handshake and the caller must have consumed the server's ref advertisement.
    ///
    /// Sequence:
    ///   1. Acquire a request handle from the transport.
    ///   2. Immediately dissolve it via `into_parts()` into `(raw_writer, reader)`.
    ///      `raw_writer` writes bytes verbatim to the wire (no pkt-line framing);
    ///      `reader` reads from the server's response stream.
    ///   3. Write the command list as pkt-line frames through `encode_into`, which
    ///      calls `data_to_write` / `flush_to_write` directly on `raw_writer`.
    ///   4. If any command is a non-delete, write the PACK as raw bytes through
    ///      `write_pack` on `raw_writer`.
    ///   5. Flush and drop `raw_writer` before switching to reads.
    ///   6. Reset the reader past any stop-state; enable sideband demux.
    ///   7. Parse the `report-status` pkt-line stream.
    ///
    /// # Why `into_parts()` immediately?
    ///
    /// `RequestWriter::write()` routes through an internal `Writer` that adds
    /// pkt-line framing to every `write()` call.  `encode_into` already applies
    /// pkt-line framing itself (via `data_to_write`); routing through a second
    /// layer would double-frame the command bytes on the wire.
    ///
    /// `into_parts()` extracts the verbatim inner writer without emitting any
    /// additional protocol message (unlike `into_read()` which writes the
    /// `on_into_read` sentinel, adding a spurious `0000` before the pack).
    pub fn send_pack<T, I>(
        transport: &mut T,
        request: Request,
        pack_entries: I,
        _options: Options,
        hash_kind: gix_hash::Kind,
    ) -> Result<Outcome, Error>
    where
        T: Transport,
        I: IntoIterator<Item = gix_pack::data::output::Entry>,
    {
        let has_updates = request.commands.iter().any(|c| !c.is_delete());

        // Acquire a request handle and immediately dissolve it.
        //
        // `WriteMode::Binary` and `MessageKind::Flush` are placeholders — we
        // never use the `Writer`'s framing or the `on_into_read` sentinel because
        // we call `into_parts()` before any data is written.
        //
        // `into_parts()` returns (verbatim_writer, reader):
        //   - verbatim_writer: writes bytes straight to the socket, no framing.
        //   - reader: reads from the server's stdout (a `WithSidebands` wrapping
        //     a `StreamingPeekableIter` backed by the child process's stdout).
        let (mut raw_writer, mut reader) = transport
            .request(WriteMode::Binary, MessageKind::Flush, false)?
            .into_parts();

        // Step 1 — command list.
        //
        // `encode_into` calls `data_to_write` and `flush_to_write` directly on
        // `raw_writer`, adding exactly one layer of pkt-line framing for each
        // command and writing the mandatory `0000` flush separator.
        encode_into(&request, hash_kind, &mut raw_writer)?;

        // Step 2 — pack (only when there is at least one non-delete command).
        //
        // `write_pack` writes raw PACK bytes.  No pkt-line framing must be added
        // here — git-receive-pack reads the pack stream verbatim after the `0000`.
        if has_updates {
            write_pack(pack_entries, hash_kind, &mut raw_writer)?;
        }

        // Step 3 — flush and drop the write handle.
        //
        // Flush to ensure all bytes (command list + pack) reach git-receive-pack
        // before we switch to reading its response.  Drop immediately to avoid
        // deadlocking on process-spawned transports where the write and read
        // channels share a single pipe pair.
        use std::io::Write as _;
        raw_writer.flush().ok();
        drop(raw_writer);

        // Step 4 — reset the reader past the handshake stop-state.
        //
        // After the handshake the underlying `StreamingPeekableIter` may have
        // stopped at the flush-pkt that terminated the ref advertisement.
        // `ExtendedBufRead::reset()` clears the `is_done` flag so subsequent
        // reads proceed.
        reader.reset(crate::transport::Protocol::V0);

        // Step 5 — enable sideband demux.
        //
        // git-receive-pack wraps the report-status stream in sideband band-1
        // frames when `side-band` / `side-band-64k` is in effect.  Setting a
        // progress handler switches `WithSidebands::fill_buf` into band-decode
        // mode: band-1 bytes are yielded as data; band-2/3 are discarded.
        enable_sideband_demux(&mut *reader);

        // Step 6 — parse the report-status stream.
        let report = parse_report(&mut reader)?;
        Ok(Outcome { report })
    }
}
