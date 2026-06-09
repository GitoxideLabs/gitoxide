//! Async transport integration for upload-pack server plumbing.
//!
//! This module bridges async byte streams into the blocking upload-pack implementation
//! using `futures_lite::io::BlockOn`, following the same pattern as `receive_pack::async_io`.

use std::io::Write as _;

use futures_io::{AsyncRead, AsyncWrite};
use futures_lite::{AsyncReadExt as _, AsyncWriteExt as _};
use gix_transport::packetline::blocking_io::{Writer, encode};
use gix_transport::packetline::Channel;

use crate::fetch::response::{Acknowledgement, ShallowUpdate, WantedRef};
use crate::handshake::Ref;

/// Serve one protocol V2 upload-pack request over async transport streams.
///
/// This adapts async readers/writers to the existing blocking upload-pack plumbing.
pub async fn serve_v2<R, W, D>(
    input: &mut R,
    output: &mut W,
    delegate: &mut D,
    config: &super::ServerConfig,
) -> Result<super::Outcome, super::Error>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
    D: super::Delegate,
{
    let outcome = {
        let mut blocking_input = futures_lite::io::BlockOn::new(input);
        let mut blocking_output = futures_lite::io::BlockOn::new(&mut *output);
        super::serve_v2(&mut blocking_input, &mut blocking_output, delegate, config)?
    };
    output.flush().await?;
    Ok(outcome)
}

/// Parse a single protocol V2 upload-pack request from an async reader.
///
/// This adapts the async reader to the existing blocking parser using `BlockOn`.
#[allow(clippy::unused_async)]
pub async fn parse_v2_request<R>(input: &mut R, config: &super::ServerConfig) -> Result<super::Request, super::Error>
where
    R: AsyncRead + Unpin,
{
    let mut blocking_input = futures_lite::io::BlockOn::new(input);
    super::parse_v2_request(&mut blocking_input, config)
}

/// Write a `ls-refs` response body over an async writer.
///
/// Returns the number of refs written.
pub async fn write_ls_refs_response<W>(
    output: &mut W,
    request: &super::LsRefs,
    refs: &[Ref],
) -> Result<usize, super::Error>
where
    W: AsyncWrite + Unpin,
{
    let refs_sent = {
        let mut blocking_output = futures_lite::io::BlockOn::new(&mut *output);
        super::write_ls_refs_response(&mut blocking_output, request, refs)?
    };
    output.flush().await?;
    Ok(refs_sent)
}

/// Write a protocol V2 capability advertisement over an async writer.
///
/// Includes the `version 2` greeting line followed by each capability.
pub async fn write_v2_capability_advertisement<W>(
    output: &mut W,
    capabilities: &[super::Capability],
) -> Result<(), super::Error>
where
    W: AsyncWrite + Unpin,
{
    {
        let mut blocking_output = futures_lite::io::BlockOn::new(&mut *output);
        super::write_v2_capability_advertisement(&mut blocking_output, capabilities)?;
    }
    output.flush().await?;
    Ok(())
}

/// Output payload for an async `fetch` response.
///
/// This mirrors [`super::FetchOutput`] but uses an async reader for the pack data source,
/// allowing pack bytes to be streamed without blocking the async executor.
pub struct AsyncFetchOutput {
    /// Negotiation acknowledgements to return in the `acknowledgments` section.
    pub acknowledgements: Vec<Acknowledgement>,
    /// Optional shallow boundary updates to return in the `shallow-info` section.
    pub shallow_updates: Vec<ShallowUpdate>,
    /// Optional `wanted-refs` section entries.
    pub wanted_refs: Vec<WantedRef>,
    /// If present, pack data streamed as sideband channel 1 in the `packfile` section.
    pub pack_data: Option<Box<dyn AsyncRead + Unpin + Send>>,
}

impl AsyncFetchOutput {
    /// Create a response output with async `pack_data` and no additional sections.
    pub fn new(pack_data: impl AsyncRead + Unpin + Send + 'static) -> Self {
        Self {
            acknowledgements: Vec::new(),
            shallow_updates: Vec::new(),
            wanted_refs: Vec::new(),
            pack_data: Some(Box::new(pack_data)),
        }
    }

    /// Create a response output without pack data.
    pub fn without_pack() -> Self {
        Self {
            acknowledgements: Vec::new(),
            shallow_updates: Vec::new(),
            wanted_refs: Vec::new(),
            pack_data: None,
        }
    }
}

/// Write a V2 `fetch` response with async pack streaming.
///
/// Metadata sections (acknowledgments, shallow-info, wanted-refs) are written using
/// `BlockOn` since they are small buffered writes. Pack data is streamed natively
/// async to avoid blocking the executor during large transfers.
///
/// Returns the number of raw pack bytes sent on sideband channel 1.
pub async fn write_fetch_response<W>(output: &mut W, response: &mut AsyncFetchOutput) -> Result<u64, super::Error>
where
    W: AsyncWrite + Unpin,
{
    // Write metadata sections (acks, shallow-info, wanted-refs) using BlockOn
    // since these are small buffered writes.
    {
        let mut blocking_output = futures_lite::io::BlockOn::new(&mut *output);
        super::write_fetch_metadata_sections(
            &mut blocking_output,
            &response.acknowledgements,
            &response.shallow_updates,
            &response.wanted_refs,
            response.pack_data.is_some(),
        )?;
    }

    // Stream pack data natively async.
    let mut pack_bytes_sent = 0u64;
    if let Some(pack_data) = response.pack_data.as_mut() {
        // Write "packfile" section header via BlockOn (single small write).
        {
            let mut blocking_output = futures_lite::io::BlockOn::new(&mut *output);
            let mut writer = Writer::new(&mut blocking_output);
            writer.enable_text_mode();
            writer.write_all(b"packfile")?;
        }

        // Stream pack data as sideband channel 1 packets using native async I/O.
        let mut buffer = vec![0u8; super::MAX_SIDEBAND_DATA_BYTES];
        let mut packet_buf = Vec::with_capacity(super::MAX_SIDEBAND_DATA_BYTES + 10);
        loop {
            let bytes_read = pack_data.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            pack_bytes_sent += bytes_read as u64;
            // Encode sideband packet into a reused buffer, then async-write it.
            packet_buf.clear();
            encode::band_to_write(Channel::Data, &buffer[..bytes_read], &mut packet_buf)?;
            output.write_all(&packet_buf).await?;
        }
    }

    // Write flush packet and flush the stream.
    let mut flush_buf = Vec::new();
    encode::flush_to_write(&mut flush_buf)?;
    output.write_all(&flush_buf).await?;
    output.flush().await?;
    Ok(pack_bytes_sent)
}


