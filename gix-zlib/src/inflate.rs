use gix_error::Result;
use gix_error::{ResultExt, message};

use crate::{FlushDecompress, Inflate, Status};

impl Inflate {
    /// Decompress the zlib stream at the start of `input` into `out`, returning `(status, consumed_in, written_out)`.
    ///
    /// If `out` can hold the whole decompressed stream, the status is [`Status::StreamEnd`]. Otherwise this behaves
    /// like [`Self::prefix()`]. Use [`Self::prefix()`] directly if `out` is only meant to receive the beginning of the
    /// stream, as with the `sirocco` feature, this first tries to decode the whole stream.
    /// Cannot be run multiple times without [resetting](Self::reset()) in between.
    pub fn once(&mut self, input: &[u8], out: &mut [u8]) -> Result<(Status, usize, usize)> {
        #[cfg(feature = "sirocco")]
        if self.state.total_in() == 0 {
            let whole = self
                .whole
                .get_or_insert_with(|| Box::new(sirocco::Decompressor::new(sirocco::Format::Zlib)));
            // On any error, like an output buffer that is too small or corrupt input,
            // let `zlib-rs` produce the partial output or the error.
            if let Ok(res) = whole.decompress_into(input, out) {
                return Ok((Status::StreamEnd, res.consumed, res.written));
            }
        }
        self.prefix(input, out)
    }

    /// Decompress as much of the zlib stream at the start of `input` as fits into `out` in a single step,
    /// returning `(status, consumed_in, written_out)`.
    ///
    /// The decompression can be continued with [`state`](Self::state).
    pub fn prefix(&mut self, input: &[u8], out: &mut [u8]) -> Result<(Status, usize, usize)> {
        let before_in = self.state.total_in();
        let before_out = self.state.total_out();
        let status = self
            .state
            .decompress(input, out, FlushDecompress::None)
            .or_raise(|| message("Could not decode zip stream"))?;
        Ok((
            status,
            (self.state.total_in() - before_in) as usize,
            (self.state.total_out() - before_out) as usize,
        ))
    }

    /// Ready this instance for decoding another data stream.
    pub fn reset(&mut self) {
        self.state.reset();
    }
}
