use gix_error::{Message, ResultExt, message};

use crate::{FlushDecompress, Inflate, Status};

/// The error returned by various [Inflate methods][super::Inflate]
pub type Error = gix_error::Exn<Message>;

impl Inflate {
    /// Run the decompressor exactly once. Cannot be run multiple times
    pub fn once(&mut self, input: &[u8], out: &mut [u8]) -> Result<(Status, usize, usize), Error> {
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
