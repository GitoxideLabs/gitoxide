use std::{
    io,
    ops::{Deref, DerefMut},
};

use gix_packetline::{read::ProgressAction, PacketLineRef};

use crate::{
    client::{Error, MessageKind},
    Protocol,
};
/// A function `f(is_error, text)` receiving progress or error information.
pub type HandleProgress<'a> = Box<dyn FnMut(bool, &[u8]) -> ProgressAction + 'a>;

/// This trait exists to get a version of a `gix_packetline::Provider` without type parameters,
/// but leave support for reading lines directly without forcing them through `String`.
///
/// For the sake of usability, it also implements [`std::io::BufRead`] making it trivial to
/// read pack files while keeping open the option to read individual lines with low overhead.
pub trait ReadlineBufRead: io::BufRead {
    /// Read a packet line into the internal buffer and return it.
    ///
    /// Returns `None` if the end of iteration is reached because of one of the following:
    ///
    ///  * natural EOF
    ///  * ERR packet line encountered
    ///  * A `delimiter` packet line encountered
    fn readline(
        &mut self,
    ) -> Option<io::Result<Result<gix_packetline::PacketLineRef<'_>, gix_packetline::decode::Error>>>;

    /// Read a line similar to `BufRead::read_line()`, but assure it doesn't try to find newlines
    /// which might concatenate multiple distinct packet lines.
    ///
    /// Making this a trait method allows to handle differences between async and blocking.
    fn readline_str(&mut self, line: &mut String) -> io::Result<usize>;
}

/// Provide even more access to the underlying packet reader.
pub trait ExtendedBufRead<'a>: ReadlineBufRead {
    /// Set the handler to which progress will be delivered.
    ///
    /// Note that this is only possible if packet lines are sent in side band mode.
    fn set_progress_handler(&mut self, handle_progress: Option<HandleProgress<'a>>);
    /// Peek the next data packet line. Maybe None if the next line is a packet we stop at, queryable using
    /// [`stopped_at()`][ExtendedBufRead::stopped_at()].
    fn peek_data_line(&mut self) -> Option<io::Result<Result<&[u8], Error>>>;
    /// Resets the reader to allow reading past a previous stop, and sets delimiters according to the
    /// given protocol.
    fn reset(&mut self, version: Protocol);
    /// Return the kind of message at which the reader stopped.
    fn stopped_at(&self) -> Option<MessageKind>;
}

impl<T: ReadlineBufRead + ?Sized> ReadlineBufRead for Box<T> {
    fn readline(&mut self) -> Option<io::Result<Result<PacketLineRef<'_>, gix_packetline::decode::Error>>> {
        ReadlineBufRead::readline(self.deref_mut())
    }
    fn readline_str(&mut self, line: &mut String) -> io::Result<usize> {
        ReadlineBufRead::readline_str(self.deref_mut(), line)
    }
}

impl<'a, T: ExtendedBufRead<'a> + ?Sized + 'a> ExtendedBufRead<'a> for Box<T> {
    fn set_progress_handler(&mut self, handle_progress: Option<HandleProgress<'a>>) {
        self.deref_mut().set_progress_handler(handle_progress);
    }

    fn peek_data_line(&mut self) -> Option<io::Result<Result<&[u8], Error>>> {
        self.deref_mut().peek_data_line()
    }

    fn reset(&mut self, version: Protocol) {
        self.deref_mut().reset(version);
    }

    fn stopped_at(&self) -> Option<MessageKind> {
        self.deref().stopped_at()
    }
}

impl<T: io::Read> ReadlineBufRead
    for gix_packetline::read::blocking_io::WithSidebands<'_, T, fn(bool, &[u8]) -> ProgressAction>
{
    fn readline(&mut self) -> Option<io::Result<Result<PacketLineRef<'_>, gix_packetline::decode::Error>>> {
        self.read_data_line()
    }

    fn readline_str(&mut self, line: &mut String) -> io::Result<usize> {
        self.read_line_to_string(line)
    }
}

impl<'a, T: io::Read> ReadlineBufRead for gix_packetline::read::blocking_io::WithSidebands<'a, T, HandleProgress<'a>> {
    fn readline(&mut self) -> Option<io::Result<Result<PacketLineRef<'_>, gix_packetline::decode::Error>>> {
        self.read_data_line()
    }

    fn readline_str(&mut self, line: &mut String) -> io::Result<usize> {
        self.read_line_to_string(line)
    }
}

impl<'a, T: io::Read> ExtendedBufRead<'a>
    for gix_packetline::read::blocking_io::WithSidebands<'a, T, HandleProgress<'a>>
{
    fn set_progress_handler(&mut self, handle_progress: Option<HandleProgress<'a>>) {
        self.set_progress_handler(handle_progress);
    }
    fn peek_data_line(&mut self) -> Option<io::Result<Result<&[u8], Error>>> {
        match self.peek_data_line() {
            Some(Ok(Ok(line))) => Some(Ok(Ok(line))),
            Some(Ok(Err(err))) => Some(Ok(Err(err.into()))),
            Some(Err(err)) => Some(Err(err)),
            None => None,
        }
    }
    fn reset(&mut self, version: Protocol) {
        match version {
            Protocol::V0 | Protocol::V1 => self.reset_with(&[gix_packetline::PacketLineRef::Flush]),
            Protocol::V2 => self.reset_with(&[
                gix_packetline::PacketLineRef::Delimiter,
                gix_packetline::PacketLineRef::Flush,
            ]),
        }
    }
    fn stopped_at(&self) -> Option<MessageKind> {
        self.stopped_at().map(|l| match l {
            gix_packetline::PacketLineRef::Flush => MessageKind::Flush,
            gix_packetline::PacketLineRef::Delimiter => MessageKind::Delimiter,
            gix_packetline::PacketLineRef::ResponseEnd => MessageKind::ResponseEnd,
            gix_packetline::PacketLineRef::Data(_) => unreachable!("data cannot be a delimiter"),
        })
    }
}
