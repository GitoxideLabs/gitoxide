//! Decompression support for reading zlib streams into caller-provided buffers.

use gix_error::ErrorExt;
use std::{io, io::BufRead};

use crate::{Decompress, FlushDecompress, Status};

/// Read bytes from `rd` and decompress them using `state` into a pre-allocated fitting buffer `dst`, returning the amount of bytes written.
pub fn read(rd: &mut impl BufRead, state: &mut Decompress, mut dst: &mut [u8]) -> io::Result<usize> {
    let mut total_written = 0;
    loop {
        let (written, consumed, ret, eof);
        {
            let input = rd.fill_buf()?;
            eof = input.is_empty();
            let before_out = state.total_out();
            let before_in = state.total_in();
            let flush = if eof {
                FlushDecompress::Finish
            } else {
                FlushDecompress::None
            };
            ret = state.decompress(input, dst, flush);
            written = (state.total_out() - before_out) as usize;
            total_written += written;
            dst = &mut dst[written..];
            consumed = (state.total_in() - before_in) as usize;
        }
        rd.consume(consumed);

        match ret {
            // The stream has officially ended, nothing more to do here.
            Ok(Status::StreamEnd) => return Ok(total_written),
            // Either input our output are depleted even though the stream is not depleted yet.
            Ok(Status::Ok | Status::BufError) if eof || dst.is_empty() => return Ok(total_written),
            // Some progress was made in both the input and the output, it must continue to reach the end.
            Ok(Status::Ok | Status::BufError) if consumed != 0 || written != 0 => continue,
            // A strange state, where zlib makes no progress but isn't done either. Call it out.
            Ok(Status::Ok | Status::BufError) => unreachable!("Definitely a bug somewhere"),
            // Keep the underlying zlib error so callers can tell a checksum mismatch
            // (`incorrect data check`) apart from genuine stream corruption.
            Err(err) if err.is_resource_exhausted() => {
                return Err(io::Error::other(err));
            }
            Err(err) => {
                let cause = state.error_message().map_or_else(|| err.to_string(), String::from);
                let err = err.and_raise(gix_error::message!("corrupt deflate stream: {cause}").corrupted());
                return Err(io::Error::new(io::ErrorKind::InvalidInput, err));
            }
        }
    }
}

/// Reads zlib streams from a [`BufRead`] to learn their compressed and decompressed sizes,
/// without keeping the decompressed bytes.
///
/// With the `sirocco` feature, the decompressed bytes aren't produced at all. Every symbol is still decoded,
/// so corrupt streams are rejected, but the Adler-32 checksum covers the decompressed bytes and isn't checked.
/// Without it, `zlib-rs` decompresses into a scratch buffer and checks the checksum.
#[derive(Default)]
pub struct Measure {
    #[cfg(feature = "sirocco")]
    state: Option<Box<sirocco::StreamMeasurer>>,
    #[cfg(not(feature = "sirocco"))]
    state: Decompress,
}

impl Measure {
    /// Read one zlib stream from `rd`, leaving whatever follows it unread, and return
    /// `(decompressed_size, compressed_size)`.
    ///
    /// If `rd` ends before the stream does, the decompressed size is what could be decompressed until then,
    /// or with the `sirocco` feature, an [`UnexpectedEof`](io::ErrorKind::UnexpectedEof) error.
    pub fn read(&mut self, rd: &mut impl BufRead) -> io::Result<(u64, u64)> {
        #[cfg(feature = "sirocco")]
        {
            let state = self
                .state
                .get_or_insert_with(|| Box::new(sirocco::StreamMeasurer::new(sirocco::Format::Zlib)));
            let decompressed = state.measure_reader(rd).map_err(|err| {
                if err.kind() == io::ErrorKind::InvalidData
                    && matches!(err.get_ref(), Some(inner) if inner.is::<sirocco::DecompressError>())
                {
                    let err = gix_error::message!("corrupt deflate stream: {err}").corrupted().raise();
                    io::Error::new(io::ErrorKind::InvalidInput, err)
                } else {
                    err
                }
            })?;
            Ok((decompressed, state.total_in()))
        }
        #[cfg(not(feature = "sirocco"))]
        {
            self.state.reset();
            let mut buf = [0; 8 * 1024];
            let mut decompressed = 0;
            loop {
                match read(rd, &mut self.state, &mut buf)? {
                    0 => return Ok((decompressed, self.state.total_in())),
                    n => decompressed += n as u64,
                }
            }
        }
    }
}
