//! Utilities to cause interruptions in common traits, like Read/Write and Iterator.
use std::{
    io,
    sync::atomic::{AtomicBool, Ordering},
};

/// A wrapper for an inner iterator which will check for interruptions on each iteration, stopping the iteration when
/// that is requested.
pub struct Iter<'a, I> {
    /// The actual iterator to yield elements from.
    pub inner: I,
    should_interrupt: &'a AtomicBool,
}

impl<'a, I> Iter<'a, I>
where
    I: Iterator,
{
    /// Create a new iterator over `inner` which checks for interruptions on each iteration on `should_interrupt`.
    ///
    /// Note that this means the consumer of the iterator data should also be able to access `should_interrupt` and
    /// consider it when producing the final result to avoid claiming success even though the operation is only partially
    /// complete.
    pub fn new(inner: I, should_interrupt: &'a AtomicBool) -> Self {
        Iter {
            inner,
            should_interrupt,
        }
    }
}

impl<I> Iterator for Iter<'_, I>
where
    I: Iterator,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.should_interrupt.load(Ordering::Relaxed) {
            return None;
        }
        self.inner.next()
    }
}

/// A wrapper for an inner iterator which will check for interruptions on each iteration.
pub struct IterWithErr<'a, I, EFN> {
    /// The actual iterator to yield elements from.
    pub inner: I,
    make_err: Option<EFN>,
    should_interrupt: &'a AtomicBool,
}

impl<'a, I, EFN, E> IterWithErr<'a, I, EFN>
where
    I: Iterator,
    EFN: FnOnce() -> E,
{
    /// Create a new iterator over `inner` which checks for interruptions on each iteration and calls `make_err()` to
    /// signal an interruption happened, causing no further items to be iterated from that point on.
    pub fn new(inner: I, make_err: EFN, should_interrupt: &'a AtomicBool) -> Self {
        IterWithErr {
            inner,
            make_err: Some(make_err),
            should_interrupt,
        }
    }
}

impl<I, EFN, E> Iterator for IterWithErr<'_, I, EFN>
where
    I: Iterator,
    EFN: FnOnce() -> E,
{
    type Item = Result<I::Item, E>;

    fn next(&mut self) -> Option<Self::Item> {
        self.make_err.as_ref()?;
        if self.should_interrupt.load(Ordering::Relaxed) {
            return self.make_err.take().map(|f| Err(f()));
        }
        match self.inner.next() {
            Some(next) => Some(Ok(next)),
            None => {
                self.make_err = None;
                None
            }
        }
    }
}

/// A wrapper for implementers of [`std::io::Read`] or [`std::io::BufRead`] with interrupt support.
///
/// It fails a [read][std::io::Read::read] while an interrupt was requested.
pub struct Read<'a, R> {
    /// The actual implementor of [`std::io::Read`] to which interrupt support will be added.
    pub inner: R,
    /// The flag to trigger interruption
    pub should_interrupt: &'a AtomicBool,
}

impl<R> io::Read for Read<'_, R>
where
    R: io::Read,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.should_interrupt.load(Ordering::Relaxed) {
            return Err(std::io::Error::other(gix_error::cancelled("Interrupted")));
        }
        self.inner.read(buf)
    }
}

impl<R> io::BufRead for Read<'_, R>
where
    R: io::BufRead,
{
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.inner.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.inner.consume(amt);
    }
}

/// A wrapper for implementers of [`std::io::Write`] with interrupt checks on each write call.
///
/// It fails a [write][std::io::Write::write] while an interrupt was requested.
pub struct Write<'a, W> {
    /// The actual implementor of [`std::io::Write`] to which interrupt support will be added.
    pub inner: W,
    /// The flag to trigger interruption
    pub should_interrupt: &'a AtomicBool,
}

impl<W> io::Write for Write<'_, W>
where
    W: std::io::Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.should_interrupt.load(Ordering::Relaxed) {
            return Err(std::io::Error::other(gix_error::cancelled("Interrupted")));
        }
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        // Don't interrupt here, allow flushes to happen to prefer disk consistency.
        self.inner.flush()
    }
}

impl<W> io::Seek for Write<'_, W>
where
    W: std::io::Seek,
{
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}
