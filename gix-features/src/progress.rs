//! Various `prodash` types along with various utilities for comfort.
use std::io;

#[cfg(feature = "progress-unit-bytes")]
pub use bytesize;
pub use prodash::{
    self, BoxedDynNestedProgress, Count, DynNestedProgress, DynNestedProgressToNestedProgress, NestedProgress,
    Progress, Unit,
    messages::MessageLevel,
    progress::{
        AtomicStep, Discard, DoOrDiscard, Either, Id, Step, StepShared, Task, ThroughputOnDrop, UNKNOWN, Value,
    },
    unit,
};
/// A stub for the portions of the `bytesize` crate that we use internally in `gitoxide`.
#[cfg(not(feature = "progress-unit-bytes"))]
pub mod bytesize {
    /// A stub for the `ByteSize` wrapper.
    pub struct ByteSize(pub u64);

    impl std::fmt::Display for ByteSize {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }
}

/// A unit for displaying bytes with throughput and progress percentage.
#[cfg(feature = "progress-unit-bytes")]
pub fn bytes() -> Option<Unit> {
    Some(unit::dynamic_and_mode(
        unit::Bytes,
        unit::display::Mode::with_throughput().and_percentage(),
    ))
}

/// A unit for displaying bytes with throughput and progress percentage.
#[cfg(not(feature = "progress-unit-bytes"))]
pub fn bytes() -> Option<Unit> {
    Some(unit::label_and_mode(
        "B",
        unit::display::Mode::with_throughput().and_percentage(),
    ))
}

/// A unit for displaying human readable numbers with throughput and progress percentage, and a single decimal place.
pub fn count(name: &'static str) -> Option<Unit> {
    count_with_decimals(name, 1)
}

/// A unit for displaying human readable numbers with `name` suffix,
/// with throughput and progress percentage, and `decimals` decimal places.
#[cfg(feature = "progress-unit-human-numbers")]
pub fn count_with_decimals(name: &'static str, decimals: usize) -> Option<Unit> {
    Some(unit::dynamic_and_mode(
        unit::Human::new(
            {
                let mut f = unit::human::Formatter::new();
                f.with_decimals(decimals);
                f
            },
            name,
        ),
        unit::display::Mode::with_throughput().and_percentage(),
    ))
}

/// A unit for displaying human readable numbers with `name` suffix,
/// with throughput and progress percentage, and `decimals` decimal places.
#[cfg(not(feature = "progress-unit-human-numbers"))]
pub fn count_with_decimals(name: &'static str, _decimals: usize) -> Option<Unit> {
    Some(unit::label_and_mode(
        name,
        unit::display::Mode::with_throughput().and_percentage(),
    ))
}

/// A predefined unit for displaying a multi-step progress
pub fn steps() -> Option<Unit> {
    Some(unit::dynamic(unit::Range::new("steps")))
}

/// A structure passing every [`read`](std::io::Read::read()) call through to the contained Progress instance using [`inc_by(bytes_read)`](Count::inc_by()).
pub struct Read<T, P> {
    /// The implementor of [`std::io::Read`] to which progress is added
    pub inner: T,
    /// The progress instance receiving progress information on each invocation of `reader`
    pub progress: P,
}

impl<T, P> io::Read for Read<T, P>
where
    T: io::Read,
    P: Progress,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let bytes_read = self.inner.read(buf)?;
        self.progress.inc_by(bytes_read);
        Ok(bytes_read)
    }
}

impl<T, P> io::BufRead for Read<T, P>
where
    T: io::BufRead,
    P: Progress,
{
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.inner.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.inner.consume(amt);
    }
}

/// A structure passing every [`write`][std::io::Write::write()] call through to the contained Progress instance using [`inc_by(bytes_written)`](Count::inc_by()).
///
/// This is particularly useful if the final size of the bytes to write is known or can be estimated precisely enough.
pub struct Write<T, P> {
    /// The implementor of [`std::io::Write`] to which progress is added
    pub inner: T,
    /// The progress instance receiving progress information on each invocation of `reader`
    pub progress: P,
}

impl<T, P> io::Write for Write<T, P>
where
    T: io::Write,
    P: Progress,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(buf)?;
        self.progress.inc_by(written);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<T, P> io::Seek for Write<T, P>
where
    T: io::Seek,
{
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}
