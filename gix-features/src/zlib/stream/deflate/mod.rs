use std::ffi::c_int;

const BUF_SIZE: usize = 4096 * 8;

/// A utility to zlib compress anything that is written via its [Write][std::io::Write] implementation.
///
/// Be sure to call `flush()` when done to finalize the deflate stream.
pub struct Write<W> {
    compressor: zlib_rs::Deflate,
    inner: W,
    buf: [u8; BUF_SIZE],
}

impl<W> Clone for Write<W>
where
    W: Clone,
{
    fn clone(&self) -> Self {
        Write {
            compressor: impls::new_compress(),
            inner: self.inner.clone(),
            buf: self.buf,
        }
    }
}

/// The error produced by [`Compress::compress()`].
#[derive(Debug, thiserror::Error)]
#[error("{msg}")]
#[allow(missing_docs)]
pub enum CompressError {
    #[error("stream error")]
    StreamError,
    #[error("Not enough memory")]
    InsufficientMemory,
    #[error("An unknown error occurred: {err}")]
    Unknown { err: c_int },
}

impl From<zlib_rs::DeflateError> for CompressError {
    fn from(value: zlib_rs::DeflateError) -> Self {
        match value {
            zlib_rs::DeflateError::StreamError => Self::StreamError,
            zlib_rs::DeflateError::MemError => Self::InsufficientMemory,
            zlib_rs::DeflateError::DataError => Self::Unknown { err: value as c_int },
        }
    }
}

mod impls {
    use std::io;

    use crate::zlib::stream::deflate::{self, CompressError};
    use crate::zlib::Status;

    pub(crate) fn new_compress() -> zlib_rs::Deflate {
        zlib_rs::Deflate::new(1, true, 15)
    }

    impl<W> deflate::Write<W>
    where
        W: io::Write,
    {
        /// Create a new instance writing compressed bytes to `inner`.
        pub fn new(inner: W) -> deflate::Write<W> {
            deflate::Write {
                compressor: new_compress(),
                inner,
                buf: [0; deflate::BUF_SIZE],
            }
        }

        /// Reset the compressor, starting a new compression stream.
        ///
        /// That way multiple streams can be written to the same inner writer.
        pub fn reset(&mut self) {
            self.compressor.reset();
        }

        /// Consume `self` and return the inner writer.
        pub fn into_inner(self) -> W {
            self.inner
        }

        fn write_inner(&mut self, mut buf: &[u8], flush: zlib_rs::DeflateFlush) -> io::Result<usize> {
            let total_in_when_start = self.compressor.total_in();
            loop {
                let last_total_in = self.compressor.total_in();
                let last_total_out = self.compressor.total_out();

                let status = self
                    .compressor
                    .compress(buf, &mut self.buf, flush)
                    .map_err(CompressError::from)
                    .map_err(io::Error::other)?;

                let written = self.compressor.total_out() - last_total_out;
                if written > 0 {
                    self.inner.write_all(&self.buf[..written as usize])?;
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

    impl<W: io::Write> io::Write for deflate::Write<W> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.write_inner(buf, zlib_rs::DeflateFlush::NoFlush)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.write_inner(&[], zlib_rs::DeflateFlush::Finish).map(|_| ())
        }
    }
}

#[cfg(test)]
mod tests;
