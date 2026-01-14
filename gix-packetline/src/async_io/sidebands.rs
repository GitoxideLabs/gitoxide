use std::{
    future::Future,
    pin::Pin,
    task::{ready, Context, Poll},
};

use futures_io::{AsyncBufRead, AsyncRead};

use super::read::StreamingPeekableIter;
use crate::{decode, read::ProgressAction, BandRef, PacketLineRef, TextRef, U16_HEX_BYTES};

type ReadLineResult<'a> = Option<std::io::Result<Result<PacketLineRef<'a>, decode::Error>>>;
/// An implementor of [`AsyncBufRead`] yielding packet lines on each call to `read_line()`.
/// It's also possible to hide the underlying packet lines using the [`Read`](AsyncRead) implementation which is useful
/// if they represent binary data, like the one of a pack file.
pub struct WithSidebands<'a, T, F>
where
    T: AsyncRead,
{
    state: State<'a, T>,
    handle_progress: Option<F>,
    pos: usize,
    cap: usize,
}

impl<T, F> Drop for WithSidebands<'_, T, F>
where
    T: AsyncRead,
{
    fn drop(&mut self) {
        if let State::Idle { ref mut parent } = self.state {
            parent
                .as_mut()
                .expect("parent is always available if we are idle")
                .reset();
        }
    }
}

impl<'a, T> WithSidebands<'a, T, fn(bool, &[u8]) -> ProgressAction>
where
    T: AsyncRead,
{
    /// Create a new instance with the given provider as `parent`.
    pub fn new(parent: &'a mut StreamingPeekableIter<T>) -> Self {
        WithSidebands {
            state: State::Idle { parent: Some(parent) },
            handle_progress: None,
            pos: 0,
            cap: 0,
        }
    }
}

enum State<'a, T> {
    Idle {
        parent: Option<&'a mut StreamingPeekableIter<T>>,
    },
    ReadLine {
        read_line: Pin<Box<dyn Future<Output = ReadLineResult<'a>> + 'a>>,
        parent_inactive: Option<*mut StreamingPeekableIter<T>>,
    },
}

/// # SAFETY
/// It's safe because T is `Send` and we have a test that assures that our `StreamingPeekableIter` is `Send` as well,
/// hence the `*mut _` is `Send`.
/// `read_line` isn't send and we can't declare it as such as it forces `Send` in all places (BUT WHY IS THAT A PROBLEM, I don't recall).
/// However, it's only used when pinned and thus isn't actually sent anywhere, it's a secondary state of the future used after it was Send
/// to a thread possibly.
// TODO: Is it possible to declare it as it should be?
#[allow(unsafe_code, clippy::non_send_fields_in_send_ty)]
unsafe impl<T> Send for State<'_, T> where T: Send {}

impl<'a, T, F> WithSidebands<'a, T, F>
where
    T: AsyncRead + Unpin,
    F: FnMut(bool, &[u8]) -> ProgressAction + Unpin,
{
    /// Create a new instance with the given `parent` provider and the `handle_progress` function.
    ///
    /// Progress or error information will be passed to the given `handle_progress(is_error, text)` function, with `is_error: bool`
    /// being true in case the `text` is to be interpreted as error.
    pub fn with_progress_handler(parent: &'a mut StreamingPeekableIter<T>, handle_progress: F) -> Self {
        WithSidebands {
            state: State::Idle { parent: Some(parent) },
            handle_progress: Some(handle_progress),
            pos: 0,
            cap: 0,
        }
    }

    /// Create a new instance without a progress handler.
    pub fn without_progress_handler(parent: &'a mut StreamingPeekableIter<T>) -> Self {
        WithSidebands {
            state: State::Idle { parent: Some(parent) },
            handle_progress: None,
            pos: 0,
            cap: 0,
        }
    }

    /// Forwards to the parent [`StreamingPeekableIter::reset_with()`](crate::read::StreamingPeekableIterState::reset_with()).
    pub fn reset_with(&mut self, delimiters: &'static [PacketLineRef<'static>]) {
        if let State::Idle { ref mut parent } = self.state {
            parent
                .as_mut()
                .expect("parent is always available if we are idle")
                .reset_with(delimiters);
        }
    }

    /// Forwards to the parent [`StreamingPeekableIterState::stopped_at()`](crate::read::StreamingPeekableIterState::stopped_at()).
    pub fn stopped_at(&self) -> Option<PacketLineRef<'static>> {
        match self.state {
            State::Idle { ref parent } => {
                parent
                    .as_ref()
                    .expect("parent is always available if we are idle")
                    .stopped_at
            }
            _ => None,
        }
    }

    /// Set or unset the progress handler.
    pub fn set_progress_handler(&mut self, handle_progress: Option<F>) {
        self.handle_progress = handle_progress;
    }

    /// Effectively forwards to the parent [`StreamingPeekableIter::peek_line()`], allowing to see what would be returned
    /// next on a call to `read_line()`.
    ///
    /// # Warning
    ///
    /// This skips all sideband handling and may return an unprocessed line with sidebands still contained in it.
    pub async fn peek_data_line(&mut self) -> Option<std::io::Result<Result<&[u8], decode::Error>>> {
        match self.state {
            State::Idle { ref mut parent } => match parent
                .as_mut()
                .expect("parent is always available if we are idle")
                .peek_line()
                .await
            {
                Some(Ok(Ok(PacketLineRef::Data(line)))) => Some(Ok(Ok(line))),
                Some(Ok(Err(err))) => Some(Ok(Err(err))),
                Some(Err(err)) => Some(Err(err)),
                _ => None,
            },
            _ => None,
        }
    }

    /// Read a packet line as string line.
    pub fn read_line_to_string<'b>(&'b mut self, buf: &'b mut String) -> ReadLineFuture<'a, 'b, T, F> {
        ReadLineFuture { parent: self, buf }
    }

    /// Read a packet line from the underlying packet reader, returning empty lines if a stop-packetline was reached.
    ///
    /// # Warning
    ///
    /// This skips all sideband handling and may return an unprocessed line with sidebands still contained in it.
    pub async fn read_data_line(&mut self) -> Option<std::io::Result<Result<PacketLineRef<'_>, decode::Error>>> {
        match &mut self.state {
            State::Idle { parent: Some(parent) } => {
                assert_eq!(
                    self.cap, 0,
                    "we don't support partial buffers right now - read-line must be used consistently"
                );
                parent.read_line().await
            }
            _ => None,
        }
    }
}

#[allow(dead_code)]
pub struct ReadDataLineFuture<'a, 'b, T: AsyncRead, F> {
    parent: &'b mut WithSidebands<'a, T, F>,
    buf: &'b mut Vec<u8>,
}

impl<T, F> Future for ReadDataLineFuture<'_, '_, T, F>
where
    T: AsyncRead + Unpin,
    F: FnMut(bool, &[u8]) -> ProgressAction + Unpin,
{
    type Output = std::io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        assert_eq!(
            self.parent.cap, 0,
            "we don't support partial buffers right now - read-line must be used consistently"
        );
        let Self { buf, parent } = &mut *self;
        let line = ready!(Pin::new(parent).poll_fill_buf(cx))?;
        buf.clear();
        buf.extend_from_slice(line);
        let bytes = line.len();
        self.parent.cap = 0;
        Poll::Ready(Ok(bytes))
    }
}

pub struct ReadLineFuture<'a, 'b, T: AsyncRead, F> {
    parent: &'b mut WithSidebands<'a, T, F>,
    buf: &'b mut String,
}

impl<T, F> Future for ReadLineFuture<'_, '_, T, F>
where
    T: AsyncRead + Unpin,
    F: FnMut(bool, &[u8]) -> ProgressAction + Unpin,
{
    type Output = std::io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        assert_eq!(
            self.parent.cap, 0,
            "we don't support partial buffers right now - read-line must be used consistently"
        );
        let Self { buf, parent } = &mut *self;
        let line = std::str::from_utf8(ready!(Pin::new(parent).poll_fill_buf(cx))?).map_err(std::io::Error::other)?;
        buf.clear();
        buf.push_str(line);
        let bytes = line.len();
        self.parent.cap = 0;
        Poll::Ready(Ok(bytes))
    }
}

impl<T, F> AsyncBufRead for WithSidebands<'_, T, F>
where
    T: AsyncRead + Unpin,
    F: FnMut(bool, &[u8]) -> ProgressAction + Unpin,
{
    fn poll_fill_buf(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<&[u8]>> {
        use std::io;

        use futures_lite::FutureExt;
        {
            let this = self.as_mut().get_mut();
            if this.pos >= this.cap {
                let (ofs, cap) = loop {
                    match this.state {
                        State::Idle { ref mut parent } => {
                            let parent = parent.take().expect("parent to be present here");
                            let inactive = parent as *mut _;
                            this.state = State::ReadLine {
                                read_line: parent.read_line().boxed_local(),
                                parent_inactive: Some(inactive),
                            }
                        }
                        State::ReadLine {
                            ref mut read_line,
                            ref mut parent_inactive,
                        } => {
                            let line = ready!(read_line.poll(cx));

                            this.state = {
                                let parent = parent_inactive.take().expect("parent pointer always set");
                                // SAFETY: It's safe to recover the original mutable reference (from which
                                // the `read_line` future was created as the latter isn't accessible anymore
                                // once the state is set to Idle. In other words, either one or the other are
                                // accessible, never both at the same time.
                                // Also: We keep a pointer around which is protected by borrowcheck since it's created
                                // from a legal mutable reference which is moved into the read_line future - if it was manually
                                // implemented we would be able to re-obtain it from there.
                                #[allow(unsafe_code)]
                                let parent = unsafe { &mut *parent };
                                State::Idle { parent: Some(parent) }
                            };

                            let line = match line {
                                Some(line) => line?.map_err(io::Error::other)?,
                                None => break (0, 0),
                            };

                            match this.handle_progress.as_mut() {
                                Some(handle_progress) => {
                                    let band = line.decode_band().map_err(io::Error::other)?;
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
                                            if handle_progress(false, text).is_break() {
                                                return Poll::Ready(Err(io::Error::other("interrupted by user")));
                                            }
                                        }
                                        BandRef::Error(d) => {
                                            let text = TextRef::from(d).0;
                                            if handle_progress(true, text).is_break() {
                                                return Poll::Ready(Err(io::Error::other("interrupted by user")));
                                            }
                                        }
                                    }
                                }
                                None => {
                                    break match line.as_slice() {
                                        Some(d) => (U16_HEX_BYTES, d.len()),
                                        None => {
                                            return Poll::Ready(Err(io::Error::other(
                                                "encountered non-data line in a data-line only context",
                                            )))
                                        }
                                    }
                                }
                            }
                        }
                    }
                };
                this.cap = cap + ofs;
                this.pos = ofs;
            }
        }
        let range = self.pos..self.cap;
        match &self.get_mut().state {
            State::Idle { parent } => Poll::Ready(Ok(&parent.as_ref().expect("parent always available").buf[range])),
            State::ReadLine { .. } => unreachable!("at least in theory"),
        }
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        let this = self.get_mut();
        this.pos = std::cmp::min(this.pos + amt, this.cap);
    }
}

impl<T, F> AsyncRead for WithSidebands<'_, T, F>
where
    T: AsyncRead + Unpin,
    F: FnMut(bool, &[u8]) -> ProgressAction + Unpin,
{
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {
        use std::io::Read;
        let mut rem = ready!(self.as_mut().poll_fill_buf(cx))?;
        let nread = rem.read(buf)?;
        self.consume(nread);
        Poll::Ready(Ok(nread))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn receiver<T: Send>(_i: T) {}

    /// We want to declare items containing pointers of `StreamingPeekableIter` `Send` as well, so it must be `Send` itself.
    #[test]
    fn streaming_peekable_iter_is_send() {
        receiver(StreamingPeekableIter::new(&[][..], &[], false));
    }

    #[test]
    fn state_is_send() {
        let mut s = StreamingPeekableIter::new(&[][..], &[], false);
        receiver(State::Idle { parent: Some(&mut s) });
    }
}
