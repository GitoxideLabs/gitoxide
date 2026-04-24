//! The send-pack "response" side is the [`Report`] — the `report-status`
//! pkt-line stream that the server sends back after applying the push.
//!
//! Re-exported from [`crate::send_pack::types`] for API symmetry with
//! [`crate::fetch::Response`].

pub use crate::send_pack::Report;

/// Blocking-IO helpers for reading the report from the server.
#[cfg(feature = "blocking-client")]
pub mod blocking_io {
    use gix_transport::client::blocking_io::ExtendedBufRead;

    /// Enable sideband band-1 demultiplexing on `reader` in-place.
    ///
    /// Call this after `RequestWriter::into_read()` and before parsing the
    /// `report-status` stream, whenever `side-band` or `side-band-64k` was
    /// negotiated.  Band-2 (progress) and band-3 (error) bytes are silently
    /// discarded — send-pack callers have no use for them at parse time.
    ///
    /// Mirrors the pattern used in `gix-protocol/src/fetch/response/blocking_io.rs`,
    /// where the fetch path enables a sideband reader before consuming pack data.
    pub fn enable_sideband_demux(reader: &mut (dyn ExtendedBufRead<'_> + Unpin)) {
        reader.set_progress_handler(Some(Box::new(|_is_err, _text| std::ops::ControlFlow::Continue(()))));
    }
}
