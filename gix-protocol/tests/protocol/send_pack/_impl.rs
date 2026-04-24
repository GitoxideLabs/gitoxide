//! Mock `Transport` for send_pack integration tests.
//!
//! Holds a pre-loaded s2c byte stream in a `Cursor` (replayed as the server's
//! read side) and captures everything the client writes for post-hoc assertion.
//!
//! Modelled on `gix-protocol/tests/protocol/fetch/_impl.rs` — adapted for the
//! receive-pack direction where there is no multi-round negotiation.

use std::{any::Any, borrow::Cow, io::Cursor};

use bstr::{BStr, ByteSlice as _};
use gix_packetline::{blocking_io::StreamingPeekableIter, PacketLineRef};
use gix_transport::client::{
    blocking_io::{ExtendedBufRead, RequestWriter, SetServiceResponse},
    Error, MessageKind, TransportWithoutIO, WriteMode,
};

/// A mock blocking transport whose server-to-client bytes are replayed from
/// a pre-loaded buffer and whose client-to-server bytes are captured.
pub struct MockTransport {
    /// Pre-loaded s2c byte stream (post-handshake: the report-status portion).
    line_provider: StreamingPeekableIter<Cursor<Vec<u8>>>,
    /// Bytes written by the client (command list + pack).
    pub captured: Vec<u8>,
}

impl MockTransport {
    /// Build a mock whose server side replays `s2c_bytes`.
    pub fn new(s2c_bytes: Vec<u8>) -> Self {
        MockTransport {
            line_provider: StreamingPeekableIter::new(
                Cursor::new(s2c_bytes),
                &[PacketLineRef::Flush],
                false, // no tracing
            ),
            captured: Vec::new(),
        }
    }

    /// Return a slice of all bytes the client has written so far.
    pub fn captured_bytes(&self) -> &[u8] {
        &self.captured
    }
}

impl TransportWithoutIO for MockTransport {
    fn to_url(&self) -> Cow<'_, BStr> {
        Cow::Borrowed(b"mock://send-pack-test".as_bstr())
    }

    fn connection_persists_across_multiple_requests(&self) -> bool {
        true
    }

    fn configure(&mut self, _config: &dyn Any) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        Ok(())
    }
}

impl gix_transport::client::blocking_io::Transport for MockTransport {
    fn handshake<'a>(
        &mut self,
        _service: gix_transport::Service,
        _extra_parameters: &'a [(&'a str, Option<&'a str>)],
    ) -> Result<SetServiceResponse<'_>, Error> {
        // The orchestrator receives a post-handshake transport; handshake is
        // never called on this mock.
        Err(Error::Io(std::io::Error::other(
            "MockTransport::handshake must not be called — transport is post-handshake",
        )))
    }

    fn request(
        &mut self,
        write_mode: WriteMode,
        on_into_read: MessageKind,
        trace: bool,
    ) -> Result<RequestWriter<'_>, Error> {
        // The write side captures everything into `self.captured`.
        // The read side replays the s2c bytes via `line_provider`.
        let reader: Box<dyn ExtendedBufRead<'_> + Unpin + '_> =
            Box::new(self.line_provider.as_read_without_sidebands());
        Ok(RequestWriter::new_from_bufread(
            CaptureWriter(&mut self.captured),
            reader,
            write_mode,
            on_into_read,
            trace,
        ))
    }
}

/// A `Write` impl that appends to a `Vec<u8>`.
struct CaptureWriter<'a>(&'a mut Vec<u8>);

impl std::io::Write for CaptureWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

/// Extract the OID advertised for `refname` in the first pkt-line of a
/// receive-pack ref advertisement (`s2c_bytes`).
///
/// The first pkt-line has the form:
/// ```text
/// <40-hex-oid> <refname>\0<capabilities>\n
/// ```
/// Returns `None` if `refname` is not found or the bytes are malformed.
pub fn advertised_oid(s2c_bytes: &[u8], refname: &[u8]) -> Option<gix_hash::ObjectId> {
    // Walk pkt-lines until we find one containing refname.
    let mut pos = 0;
    while pos + 4 <= s2c_bytes.len() {
        let len = usize::from_str_radix(std::str::from_utf8(&s2c_bytes[pos..pos + 4]).ok()?, 16).ok()?;
        if len == 0 {
            break; // flush — no more ref advertisement lines
        }
        if len < 4 || pos + len > s2c_bytes.len() {
            return None;
        }
        let content = &s2c_bytes[pos + 4..pos + len];
        // Strip trailing LF if present.
        let content = if content.last() == Some(&b'\n') {
            &content[..content.len() - 1]
        } else {
            content
        };
        // Capability-bearing lines have a NUL after the refname; strip at NUL.
        let content = content.split(|&b| b == 0).next().unwrap_or(content);
        // Format: "<oid> <refname>"
        if content.len() >= 41 && content[40] == b' ' {
            let line_ref = &content[41..];
            if line_ref == refname {
                let hex = &content[..40];
                return gix_hash::ObjectId::from_hex(hex).ok();
            }
        }
        pos += len;
    }
    None
}

/// Extract the new OID the captured git client sent for `refname` from c2s bytes.
///
/// The command-list pkt-line format (for the first command) is:
/// ```text
/// <old-oid> <new-oid> <refname>\0<capabilities>
/// ```
/// Subsequent commands omit the capabilities. We scan all command pkt-lines.
pub fn sent_new_oid(c2s_bytes: &[u8], refname: &[u8]) -> Option<gix_hash::ObjectId> {
    let mut pos = 0;
    while pos + 4 <= c2s_bytes.len() {
        let len = usize::from_str_radix(std::str::from_utf8(&c2s_bytes[pos..pos + 4]).ok()?, 16).ok()?;
        if len == 0 {
            break; // flush
        }
        if len < 4 || pos + len > c2s_bytes.len() {
            return None;
        }
        let content = &c2s_bytes[pos + 4..pos + len];
        // Strip trailing LF.
        let content = if content.last() == Some(&b'\n') {
            &content[..content.len() - 1]
        } else {
            content
        };
        // Strip capabilities after NUL.
        let content = content.split(|&b| b == 0).next().unwrap_or(content);
        // Format: "<old-oid> <new-oid> <refname>"
        if content.len() >= 82 && content[40] == b' ' && content[81] == b' ' {
            let line_ref = &content[82..];
            if line_ref == refname {
                let new_hex = &content[41..81];
                return gix_hash::ObjectId::from_hex(new_hex).ok();
            }
        }
        pos += len;
    }
    None
}
