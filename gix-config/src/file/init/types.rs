use crate::{file::init, parse, parse::EventRef, path::interpolate};

/// The error returned by [`File::from_bytes_no_includes()`][crate::File::from_bytes_no_includes()].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    Parse(#[from] parse::Error),
    #[error(transparent)]
    Interpolate(#[from] interpolate::Error),
    #[error(transparent)]
    Includes(#[from] init::includes::Error),
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
