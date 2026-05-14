use std::{
    io,
    ops::{Deref, DerefMut},
};

use bstr::ByteSlice;
use futures_io::AsyncRead;
use futures_lite::AsyncReadExt;

pub use super::sidebands::WithSidebands;
use crate::{
    MAX_LINE_LEN, PacketLineRef, U16_HEX_BYTES, decode,
    read::{ExhaustiveOutcome, ProgressAction, StreamingPeekableIterState},
};

/// Read pack lines one after another, without consuming more than needed from the underlying
/// [`AsyncRead`]. [`Flush`](PacketLineRef::Flush) lines cause the reader to stop producing lines forever,
/// leaving [`AsyncRead`] at the start of whatever comes next.
///
/// This implementation tries hard not to allocate at all which leads to quite some added complexity and plenty of extra memory copies.
pub struct StreamingPeekableIter<T> {
    pub(super) state: StreamingPeekableIterState<T>,
}

/// Non-IO methods
impl<T> StreamingPeekableIter<T>
where
    T: AsyncRead + Unpin,
{
    /// Return a new instance from `read` which will stop decoding packet lines when receiving one of the given `delimiters`.
    /// If `trace` is `true`, all packetlines received or sent will be passed to the facilities of the `gix-trace` crate.
    pub fn new(read: T, delimiters: &'static [PacketLineRef<'static>], trace: bool) -> Self {
        Self {
            state: StreamingPeekableIterState::new(read, delimiters, trace),
        }
    }

    async fn read_line_inner<'a>(
        reader: &mut T,
        buf: &'a mut [u8],
    ) -> io::Result<Result<PacketLineRef<'a>, decode::Error>> {
        let (hex_bytes, data_bytes) = buf.split_at_mut(4);
        reader.read_exact(hex_bytes).await?;
        let num_data_bytes = match decode::hex_prefix(hex_bytes) {
            Ok(decode::PacketLineOrWantedSize::Line(line)) => return Ok(Ok(line)),
            Ok(decode::PacketLineOrWantedSize::Wanted(additional_bytes)) => additional_bytes as usize,
            Err(err) => return Ok(Err(err)),
        };
        if num_data_bytes > data_bytes.len() {
            return Ok(Err(decode::Error::DataLengthLimitExceeded {
                length_in_bytes: num_data_bytes + U16_HEX_BYTES,
            }));
        }

        let (data_bytes, _) = data_bytes.split_at_mut(num_data_bytes);
        reader.read_exact(data_bytes).await?;
        match decode::to_data_line(data_bytes) {
            Ok(line) => Ok(Ok(line)),
            Err(err) => Ok(Err(err)),
        }
    }

    /// This function is needed to help the borrow checker allow us to return references all the time
    /// It contains a bunch of logic shared between peek and `read_line` invocations.
    async fn read_line_inner_exhaustive<'a>(
        reader: &mut T,
        buf: &'a mut Vec<u8>,
        delimiters: &[PacketLineRef<'static>],
        fail_on_err_lines: bool,
        buf_resize: bool,
        trace: bool,
    ) -> ExhaustiveOutcome<'a> {
        (
            false,
            None,
            Some(match Self::read_line_inner(reader, buf).await {
                Ok(Ok(line)) => {
                    if trace {
                        match line {
                            #[allow(unused_variables)]
                            PacketLineRef::Data(d) => {
                                gix_trace::trace!("<< {}", d.as_bstr().trim().as_bstr());
                            }
                            PacketLineRef::Flush => {
                                gix_trace::trace!("<< FLUSH");
                            }
                            PacketLineRef::Delimiter => {
                                gix_trace::trace!("<< DELIM");
                            }
                            PacketLineRef::ResponseEnd => {
                                gix_trace::trace!("<< RESPONSE_END");
                            }
                        }
                    }
                    if delimiters.contains(&line) {
                        let stopped_at = delimiters.iter().find(|l| **l == line).copied();
                        buf.clear();
                        return (true, stopped_at, None);
                    } else if fail_on_err_lines {
                        if let Some(err) = line.check_error() {
                            let err = err.0.as_bstr().to_owned();
                            buf.clear();
                            return (
                                true,
                                None,
                                Some(Err(io::Error::other(crate::read::Error { message: err }))),
                            );
                        }
                    }
                    let len = line.as_slice().map_or(U16_HEX_BYTES, |s| s.len() + U16_HEX_BYTES);
                    if buf_resize {
                        buf.resize(len, 0);
                    }
                    Ok(Ok(crate::decode(buf).expect("only valid data here")))
                }
                Ok(Err(err)) => {
                    buf.clear();
                    Ok(Err(err))
                }
                Err(err) => {
                    buf.clear();
                    Err(err)
                }
            }),
        )
    }

    /// Read a packet line into the internal buffer and return it.
    ///
    /// Returns `None` if the end of iteration is reached because of one of the following:
    ///
    ///  * natural EOF
    ///  * ERR packet line encountered if [`fail_on_err_lines()`](StreamingPeekableIterState::fail_on_err_lines()) is true.
    ///  * A `delimiter` packet line encountered
    pub async fn read_line(&mut self) -> Option<io::Result<Result<PacketLineRef<'_>, decode::Error>>> {
        let state = &mut self.state;
        if state.is_done {
            return None;
        }
        if !state.peek_buf.is_empty() {
            std::mem::swap(&mut state.peek_buf, &mut state.buf);
            state.peek_buf.clear();
            Some(Ok(Ok(crate::decode(&state.buf).expect("only valid data in peek buf"))))
        } else {
            if state.buf.len() != MAX_LINE_LEN {
                state.buf.resize(MAX_LINE_LEN, 0);
            }
            let (is_done, stopped_at, res) = Self::read_line_inner_exhaustive(
                &mut state.read,
                &mut state.buf,
                state.delimiters,
                state.fail_on_err_lines,
                false,
                state.trace,
            )
            .await;
            state.is_done = is_done;
            state.stopped_at = stopped_at;
            res
        }
    }

    /// Peek the next packet line without consuming it. Returns `None` if a stop-packet or an error
    /// was encountered.
    ///
    /// Multiple calls to peek will return the same packet line, if there is one.
    pub async fn peek_line(&mut self) -> Option<io::Result<Result<PacketLineRef<'_>, decode::Error>>> {
        let state = &mut self.state;
        if state.is_done {
            return None;
        }
        if state.peek_buf.is_empty() {
            state.peek_buf.resize(MAX_LINE_LEN, 0);
            let (is_done, stopped_at, res) = Self::read_line_inner_exhaustive(
                &mut state.read,
                &mut state.peek_buf,
                state.delimiters,
                state.fail_on_err_lines,
                true,
                state.trace,
            )
            .await;
            state.is_done = is_done;
            state.stopped_at = stopped_at;
            res
        } else {
            Some(Ok(Ok(crate::decode(&state.peek_buf).expect("only valid data here"))))
        }
    }

    /// Same as [`as_read_with_sidebands(…)`](StreamingPeekableIter::as_read_with_sidebands()), but for channels without side band support.
    ///
    /// Due to the preconfigured function type this method can be called without 'turbofish'.
    #[allow(clippy::type_complexity)]
    pub fn as_read(&mut self) -> WithSidebands<'_, T, fn(bool, &[u8]) -> ProgressAction> {
        WithSidebands::new(self)
    }

    /// Return this instance as implementor of [`Read`](io::Read) assuming sidebands to be used in all received packet lines.
    /// Each invocation of [`read_line()`](io::BufRead::read_line()) returns a packet line.
    ///
    /// Progress or error information will be passed to the given `handle_progress(is_error, text)` function, with `is_error: bool`
    /// being true in case the `text` is to be interpreted as error.
    ///
    /// _Please note_ that sidebands need to be negotiated with the server.
    pub fn as_read_with_sidebands<F: FnMut(bool, &[u8]) -> ProgressAction + Unpin>(
        &mut self,
        handle_progress: F,
    ) -> WithSidebands<'_, T, F> {
        WithSidebands::with_progress_handler(self, handle_progress)
    }

    /// Same as [`as_read_with_sidebands(…)`](StreamingPeekableIter::as_read_with_sidebands()), but for channels without side band support.
    ///
    /// The type parameter `F` needs to be configured for this method to be callable using the 'turbofish' operator.
    /// Use [`as_read()`](StreamingPeekableIter::as_read()).
    pub fn as_read_without_sidebands<F: FnMut(bool, &[u8]) -> ProgressAction + Unpin>(
        &mut self,
    ) -> WithSidebands<'_, T, F> {
        WithSidebands::without_progress_handler(self)
    }
}

impl<T> StreamingPeekableIter<T> {
    /// Return the inner read
    pub fn into_inner(self) -> T {
        self.state.read
    }
}

impl<T> Deref for StreamingPeekableIter<T> {
    type Target = StreamingPeekableIterState<T>;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<T> DerefMut for StreamingPeekableIter<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}
