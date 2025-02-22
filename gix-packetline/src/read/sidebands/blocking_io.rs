use std::{io, io::BufRead};

use crate::{read::ProgressAction, BandRef, PacketLineRef, StreamingPeekableIter, TextRef, U16_HEX_BYTES};

/// An implementor of [`BufRead`][io::BufRead] yielding packet lines on each call to [`read_line()`][io::BufRead::read_line()].
/// It's also possible to hide the underlying packet lines using the [`Read`][io::Read] implementation which is useful
/// if they represent binary data, like the one of a pack file.
pub struct WithSidebands<'a, T, F>
where
    T: io::Read,
{
    parent: &'a mut StreamingPeekableIter<T>,
    handle_progress: Option<F>,
    pos: usize,
    cap: usize,
}

impl<T, F> Drop for WithSidebands<'_, T, F>
where
    T: io::Read,
{
    fn drop(&mut self) {
        self.parent.reset();
    }
}

impl<'a, T> WithSidebands<'a, T, fn(bool, &[u8]) -> ProgressAction>
where
    T: io::Read,
{
    /// Create a new instance with the given provider as `parent`.
    pub fn new(parent: &'a mut StreamingPeekableIter<T>) -> Self {
        WithSidebands {
            parent,
            handle_progress: None,
            pos: 0,
            cap: 0,
        }
    }
}

impl<'a, T, F> WithSidebands<'a, T, F>
where
    T: io::Read,
    F: FnMut(bool, &[u8]) -> ProgressAction,
{
    /// Create a new instance with the given `parent` provider and the `handle_progress` function.
    ///
    /// Progress or error information will be passed to the given `handle_progress(is_error, text)` function, with `is_error: bool`
    /// being true in case the `text` is to be interpreted as error.
    pub fn with_progress_handler(parent: &'a mut StreamingPeekableIter<T>, handle_progress: F) -> Self {
        WithSidebands {
            parent,
            handle_progress: Some(handle_progress),
            pos: 0,
            cap: 0,
        }
    }

    /// Create a new instance without a progress handler.
    pub fn without_progress_handler(parent: &'a mut StreamingPeekableIter<T>) -> Self {
        WithSidebands {
            parent,
            handle_progress: None,
            pos: 0,
            cap: 0,
        }
    }

    /// Forwards to the parent [`StreamingPeekableIter::reset_with()`]
    pub fn reset_with(&mut self, delimiters: &'static [PacketLineRef<'static>]) {
        self.parent.reset_with(delimiters);
    }

    /// Forwards to the parent [`StreamingPeekableIter::stopped_at()`]
    pub fn stopped_at(&self) -> Option<PacketLineRef<'static>> {
        self.parent.stopped_at
    }

    /// Set or unset the progress handler.
    pub fn set_progress_handler(&mut self, handle_progress: Option<F>) {
        self.handle_progress = handle_progress;
    }

    /// Effectively forwards to the parent [`StreamingPeekableIter::peek_line()`], allowing to see what would be returned
    /// next on a call to [`read_line()`][io::BufRead::read_line()].
    ///
    /// # Warning
    ///
    /// This skips all sideband handling and may return an unprocessed line with sidebands still contained in it.
    pub fn peek_data_line(&mut self) -> Option<io::Result<Result<&[u8], crate::decode::Error>>> {
        match self.parent.peek_line() {
            Some(Ok(Ok(PacketLineRef::Data(line)))) => Some(Ok(Ok(line))),
            Some(Ok(Err(err))) => Some(Ok(Err(err))),
            Some(Err(err)) => Some(Err(err)),
            _ => None,
        }
    }

    /// Read a whole packetline from the underlying reader, with empty lines indicating a stop packetline.
    ///
    /// # Warning
    ///
    /// This skips all sideband handling and may return an unprocessed line with sidebands still contained in it.
    pub fn read_data_line(&mut self) -> Option<io::Result<Result<PacketLineRef<'_>, crate::decode::Error>>> {
        assert_eq!(
            self.cap, 0,
            "we don't support partial buffers right now - read-line must be used consistently"
        );
        self.parent.read_line()
    }

    /// Like `BufRead::read_line()`, but will only read one packetline at a time.
    ///
    /// It will also be easier to call as sometimes it's unclear which implementation we get on a type like this with
    /// plenty of generic parameters.
    pub fn read_line_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        assert_eq!(
            self.cap, 0,
            "we don't support partial buffers right now - read-line must be used consistently"
        );
        let line = std::str::from_utf8(self.fill_buf()?).map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        buf.push_str(line);
        let bytes = line.len();
        self.cap = 0;
        Ok(bytes)
    }
}

impl<T, F> BufRead for WithSidebands<'_, T, F>
where
    T: io::Read,
    F: FnMut(bool, &[u8]) -> ProgressAction,
{
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.pos >= self.cap {
            let (ofs, cap) = loop {
                let line = match self.parent.read_line() {
                    Some(line) => line?.map_err(|err| io::Error::new(io::ErrorKind::Other, err))?,
                    None => break (0, 0),
                };
                match self.handle_progress.as_mut() {
                    Some(handle_progress) => {
                        let band = line
                            .decode_band()
                            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
                        const ENCODED_BAND: usize = 1;
                        match band {
                            BandRef::Data(d) => {
                                if d.is_empty() {
                                    continue;
                                }
                                break (U16_HEX_BYTES + ENCODED_BAND, d.len());
                            }
                            BandRef::Progress(d) => {
                                let text = TextRef::from(d).0;
                                match handle_progress(false, text) {
                                    ProgressAction::Continue => {}
                                    ProgressAction::Interrupt => {
                                        return Err(std::io::Error::new(
                                            std::io::ErrorKind::Other,
                                            "interrupted by user",
                                        ))
                                    }
                                }
                            }
                            BandRef::Error(d) => {
                                let text = TextRef::from(d).0;
                                match handle_progress(true, text) {
                                    ProgressAction::Continue => {}
                                    ProgressAction::Interrupt => {
                                        return Err(std::io::Error::new(
                                            std::io::ErrorKind::Other,
                                            "interrupted by user",
                                        ))
                                    }
                                }
                            }
                        }
                    }
                    None => {
                        break match line.as_slice() {
                            Some(d) => (U16_HEX_BYTES, d.len()),
                            None => {
                                return Err(io::Error::new(
                                    io::ErrorKind::UnexpectedEof,
                                    "encountered non-data line in a data-line only context",
                                ))
                            }
                        }
                    }
                }
            };
            self.cap = cap + ofs;
            self.pos = ofs;
        }
        Ok(&self.parent.buf[self.pos..self.cap])
    }

    fn consume(&mut self, amt: usize) {
        self.pos = std::cmp::min(self.pos + amt, self.cap);
    }
}

impl<T, F> io::Read for WithSidebands<'_, T, F>
where
    T: io::Read,
    F: FnMut(bool, &[u8]) -> ProgressAction,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut rem = self.fill_buf()?;
        let nread = rem.read(buf)?;
        self.consume(nread);
        Ok(nread)
    }
}
