use gix_object::bstr::ByteSlice;

use crate::{
    file,
    file::loose::reference::logiter::must_be_io_err,
    store_impl::file::{log, log::iter::decode::LineNumber},
    FullNameRef,
};

///
pub mod decode {
    use crate::store_impl::file::log;

    /// The error returned by items in the [forward][super::forward()] and [reverse][super::reverse()] iterators
    #[derive(Debug)]
    pub struct Error {
        inner: log::line::decode::Error,
        line: LineNumber,
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "In line {}: {}", self.line, self.inner)
        }
    }

    impl std::error::Error for Error {}

    impl Error {
        pub(crate) fn new(err: log::line::decode::Error, line: LineNumber) -> Self {
            Error { line, inner: err }
        }
    }

    #[derive(Debug)]
    pub(crate) enum LineNumber {
        FromStart(usize),
        FromEnd(usize),
    }

    impl std::fmt::Display for LineNumber {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let (line, suffix) = match self {
                LineNumber::FromStart(line) => (line, ""),
                LineNumber::FromEnd(line) => (line, " from the end"),
            };
            write!(f, "{}{}", line + 1, suffix)
        }
    }
}

/// Returns a forward iterator over the given `lines`, starting from the first line in the file and ending at the last.
///
/// Note that `lines` are an entire reflog file.
///
/// This iterator is useful when the ref log file is going to be rewritten which forces processing of the entire file.
/// It will continue parsing even if individual log entries failed to parse, leaving it to the driver to decide whether to
/// abort or continue.
pub fn forward(lines: &[u8]) -> Forward<'_> {
    Forward {
        inner: lines.as_bstr().lines().enumerate(),
    }
}

/// An iterator yielding parsed lines in a file from start to end, oldest to newest.
pub struct Forward<'a> {
    inner: std::iter::Enumerate<gix_object::bstr::Lines<'a>>,
}

impl<'a> Iterator for Forward<'a> {
    type Item = Result<log::LineRef<'a>, decode::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(ln, line)| {
            log::LineRef::from_bytes(line).map_err(|err| decode::Error::new(err, decode::LineNumber::FromStart(ln)))
        })
    }
}

/// A platform to store a buffer to hold ref log lines for iteration.
#[must_use = "Iterators should be obtained from this platform"]
pub struct Platform<'a, 's> {
    /// The store containing the reflogs
    pub store: &'s file::Store,
    /// The full name of the reference whose reflog to retrieve.
    pub name: &'a FullNameRef,
    /// A reusable buffer for storing log lines read from disk.
    pub buf: Vec<u8>,
}

impl Platform<'_, '_> {
    /// Return a forward iterator over all log-lines, most recent to oldest.
    pub fn rev(&mut self) -> std::io::Result<Option<log::iter::Reverse<'_, std::fs::File>>> {
        self.buf.clear();
        self.buf.resize(1024 * 4, 0);
        self.store
            .reflog_iter_rev(self.name, &mut self.buf)
            .map_err(must_be_io_err)
    }

    /// Return a forward iterator over all log-lines, oldest to most recent.
    pub fn all(&mut self) -> std::io::Result<Option<log::iter::Forward<'_>>> {
        self.buf.clear();
        self.store.reflog_iter(self.name, &mut self.buf).map_err(must_be_io_err)
    }
}

/// An iterator yielding parsed lines in a file in reverse, most recent to oldest.
pub struct Reverse<'a, F> {
    buf: &'a mut [u8],
    count: usize,
    read_and_pos: Option<(F, u64)>,
    last_nl_pos: Option<usize>,
}

/// An iterator over entries of the `log` file in reverse, using `buf` as sliding window.
///
/// Note that `buf` must be big enough to capture typical line length or else partial lines will be parsed and probably fail
/// in the process.
///
/// This iterator is very expensive in terms of I/O operations and shouldn't be used to read more than the last few entries of the log.
/// Use a forward iterator instead for these cases.
///
/// It will continue parsing even if individual log entries failed to parse, leaving it to the driver to decide whether to
/// abort or continue.
pub fn reverse<F>(mut log: F, buf: &mut [u8]) -> std::io::Result<Reverse<'_, F>>
where
    F: std::io::Read + std::io::Seek,
{
    let pos = log.seek(std::io::SeekFrom::End(0))?;
    if buf.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Zero sized buffers are not allowed, use 256 bytes or more for typical logs",
        ));
    }
    Ok(Reverse {
        buf,
        count: 0,
        read_and_pos: Some((log, pos)),
        last_nl_pos: None,
    })
}

///
pub mod reverse {

    use super::decode;

    /// The error returned by the [`Reverse`][super::Reverse] iterator
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("The buffer could not be filled to make more lines available")]
        Io(#[from] std::io::Error),
        #[error("Could not decode log line")]
        Decode(#[from] decode::Error),
    }
}

impl<F> Iterator for Reverse<'_, F>
where
    F: std::io::Read + std::io::Seek,
{
    type Item = Result<crate::log::Line, reverse::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.last_nl_pos.take(), self.read_and_pos.take()) {
            // Initial state - load first data block
            (None, Some((mut read, pos))) => {
                let npos = pos.saturating_sub(self.buf.len() as u64);
                if let Err(err) = read.seek(std::io::SeekFrom::Start(npos)) {
                    return Some(Err(err.into()));
                }

                let n = (pos - npos) as usize;
                if n == 0 {
                    return None;
                }
                let buf = &mut self.buf[..n];
                if let Err(err) = read.read_exact(buf) {
                    return Some(Err(err.into()));
                }

                let last_byte = *buf.last().expect("we have read non-zero bytes before");
                self.last_nl_pos = Some(if last_byte != b'\n' { buf.len() } else { buf.len() - 1 });
                self.read_and_pos = Some((read, npos));
                self.next()
            }
            // Has data block and can extract lines from it, load new blocks as needed
            (Some(end), Some(read_and_pos)) => match self.buf[..end].rfind_byte(b'\n') {
                Some(start) => {
                    self.read_and_pos = Some(read_and_pos);
                    self.last_nl_pos = Some(start);
                    let buf = &self.buf[start + 1..end];
                    let res = Some(
                        log::LineRef::from_bytes(buf)
                            .map_err(|err| {
                                reverse::Error::Decode(decode::Error::new(err, LineNumber::FromEnd(self.count)))
                            })
                            .map(Into::into),
                    );
                    self.count += 1;
                    res
                }
                None => {
                    let (mut read, last_read_pos) = read_and_pos;
                    if last_read_pos == 0 {
                        let buf = &self.buf[..end];
                        Some(
                            log::LineRef::from_bytes(buf)
                                .map_err(|err| {
                                    reverse::Error::Decode(decode::Error::new(err, LineNumber::FromEnd(self.count)))
                                })
                                .map(Into::into),
                        )
                    } else {
                        let npos = last_read_pos.saturating_sub((self.buf.len() - end) as u64);
                        if npos == last_read_pos {
                            return Some(Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                format!("buffer too small for line size, got until {:?}", self.buf.as_bstr()),
                            )
                            .into()));
                        }
                        let n = (last_read_pos - npos) as usize;
                        self.buf.copy_within(0..end, n);
                        if let Err(err) = read.seek(std::io::SeekFrom::Start(npos)) {
                            return Some(Err(err.into()));
                        }
                        if let Err(err) = read.read_exact(&mut self.buf[..n]) {
                            return Some(Err(err.into()));
                        }
                        self.read_and_pos = Some((read, npos));
                        self.last_nl_pos = Some(n + end);
                        self.next()
                    }
                }
            },
            // depleted
            (None, None) => None,
            (Some(_), None) => unreachable!("BUG: Invalid state: we never discard only our file, always both."),
        }
    }
}
