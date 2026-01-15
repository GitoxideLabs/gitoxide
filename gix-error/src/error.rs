use crate::{exn, Error, Exn};
use std::fmt::Formatter;

/// Utilities
impl Error {
    /// Return the error that is most likely the root cause, based on heuristics.
    /// Note that if there is nothing but this error, i.e. no source or children, this error is returned.
    pub fn probable_cause(&self) -> &(dyn std::error::Error + 'static) {
        let root = self.inner.as_frame();
        root.probable_cause().unwrap_or(root).as_error()
    }

    /// Return an iterator over all errors in the tree in breadth-first order, starting with this one.
    pub fn iter_frames(&self) -> impl Iterator<Item = &exn::Frame> + '_ {
        self.inner.as_frame().iter()
    }
}

pub(super) enum Inner {
    ExnAsError(Box<exn::Frame>),
    Exn(Box<exn::Frame>),
}

impl Inner {
    fn as_frame(&self) -> &exn::Frame {
        match self {
            Inner::ExnAsError(f) | Inner::Exn(f) => f,
        }
    }
}

impl Error {
    /// Create a new instance representing the given `error`.
    #[track_caller]
    pub fn from_error(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Error {
            inner: Inner::ExnAsError(Exn::new(error).into()),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            Inner::ExnAsError(err) => std::fmt::Display::fmt(err.as_error(), f),
            Inner::Exn(frame) => std::fmt::Display::fmt(frame, f),
        }
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            Inner::ExnAsError(err) => std::fmt::Debug::fmt(err.as_error(), f),
            Inner::Exn(frame) => std::fmt::Debug::fmt(frame, f),
        }
    }
}

impl std::error::Error for Error {
    /// Return the first source of an [Exn] error, or the source of a boxed error.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.inner {
            Inner::ExnAsError(frame) | Inner::Exn(frame) => {
                frame.children().first().map(exn::Frame::as_error_no_send_sync)
            }
        }
    }
}

impl<E> From<Exn<E>> for Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(err: Exn<E>) -> Self {
        Error {
            inner: Inner::Exn(err.into()),
        }
    }
}
