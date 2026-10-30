use std::io;

use crate::stream::deflate::{Compress, FlushCompress};
use crate::{Compression, Status};

const BUF_SIZE: usize = 4096 * 8;

pub(super) struct Backend {
    compressor: Compress,
    buf: [u8; BUF_SIZE],
}

impl Backend {
    pub(super) fn new(level: Compression) -> Self {
        Backend {
            compressor: Compress::new(level),
            buf: [0; BUF_SIZE],
        }
    }

    pub(super) fn reset(&mut self) {
        self.compressor.reset();
    }

    pub(super) fn write(&mut self, inner: &mut impl io::Write, buf: &[u8]) -> io::Result<usize> {
        self.write_inner(inner, buf, FlushCompress::None)
    }

    pub(super) fn finish(&mut self, inner: &mut impl io::Write) -> io::Result<()> {
        self.write_inner(inner, &[], FlushCompress::Finish).map(|_| ())
    }

    fn write_inner(&mut self, inner: &mut impl io::Write, mut buf: &[u8], flush: FlushCompress) -> io::Result<usize> {
        let total_in_when_start = self.compressor.total_in();
        loop {
            let last_total_in = self.compressor.total_in();
            let last_total_out = self.compressor.total_out();

            let status = self
                .compressor
                .compress(buf, &mut self.buf, flush)
                .map_err(io::Error::other)?;

            let written = self.compressor.total_out() - last_total_out;
            if written > 0 {
                inner.write_all(&self.buf[..written as usize])?;
            }

            match status {
                Status::StreamEnd => return Ok((self.compressor.total_in() - total_in_when_start) as usize),
                Status::Ok | Status::BufError => {
                    let consumed = self.compressor.total_in() - last_total_in;
                    buf = &buf[consumed as usize..];

                    // output buffer still makes progress
                    if self.compressor.total_out() > last_total_out {
                        continue;
                    }
                    // input still makes progress
                    if self.compressor.total_in() > last_total_in {
                        continue;
                    }
                    // input also makes no progress anymore, need more so leave with what we have
                    return Ok((self.compressor.total_in() - total_in_when_start) as usize);
                }
            }
        }
    }
}
