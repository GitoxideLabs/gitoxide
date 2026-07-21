use crate::{file::init, parse, parse::EventRef, path::interpolate};

/// The error returned by [`File::from_bytes_no_includes()`][crate::File::from_bytes_no_includes()].
// TODO(review): all variants were `#[error(transparent)]`: `Display` and `source()` forward to the
//                wrapped error, exactly like the `thiserror`-generated code did.
#[derive(Debug)]
#[expect(missing_docs)]
pub enum Error {
    Parse(parse::Error),
    Interpolate(interpolate::Error),
    Includes(init::includes::Error),
    Span(parse::span::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Parse(err) => std::fmt::Display::fmt(err, f),
            Error::Interpolate(err) => std::fmt::Display::fmt(err, f),
            Error::Includes(err) => std::fmt::Display::fmt(err, f),
            Error::Span(err) => std::fmt::Display::fmt(err, f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Parse(err) => err.source(),
            Error::Interpolate(err) => err.source(),
            Error::Includes(err) => err.source(),
            Error::Span(err) => err.source(),
        }
    }
}

impl From<parse::Error> for Error {
    fn from(err: parse::Error) -> Self {
        Error::Parse(err)
    }
}

impl From<interpolate::Error> for Error {
    fn from(err: interpolate::Error) -> Self {
        Error::Interpolate(err)
    }
}

impl From<init::includes::Error> for Error {
    fn from(err: init::includes::Error) -> Self {
        Error::Includes(err)
    }
}

impl From<parse::span::Error> for Error {
    fn from(err: parse::span::Error) -> Self {
        Error::Span(err)
    }
}

/// Options when loading git config using [`File::from_paths_metadata()`][crate::File::from_paths_metadata()].
#[derive(Clone, Copy, Default)]
pub struct Options<'a> {
    /// Configure how to follow includes while handling paths.
    pub includes: init::includes::Options<'a>,
    /// If true, only value-bearing parse events will be kept to reduce memory usage and increase performance.
    ///
    /// Note that doing so will degenerate [`write_to()`][crate::File::write_to()] and strip it off its comments
    /// and additional whitespace entirely, but will otherwise be a valid configuration file.
    pub lossy: bool,
    /// If true, any IO error happening when reading a configuration file will be ignored.
    ///
    /// That way it's possible to pass multiple files and read as many as possible, to have 'something' instead of nothing.
    pub ignore_io_errors: bool,
}

impl Options<'_> {
    pub(crate) fn to_event_filter(self) -> Option<fn(EventRef<'_>) -> bool> {
        if self.lossy {
            Some(discard_nonessential_events)
        } else {
            None
        }
    }
}

fn discard_nonessential_events(e: EventRef<'_>) -> bool {
    match e {
        EventRef::Whitespace(_) | EventRef::Comment { .. } | EventRef::Newline(_) => false,
        EventRef::SectionHeader { .. }
        | EventRef::SectionValueName(_)
        | EventRef::KeyValueSeparator
        | EventRef::Value(_)
        | EventRef::ValueNotDone(_)
        | EventRef::ValueDone(_) => true,
    }
}
