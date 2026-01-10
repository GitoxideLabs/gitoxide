use crate::{exn, Error, Exn};
use std::fmt::Formatter;

/// Utilities
impl Error {
    /// Return the last leaf error of this error tree or the error itself, which is equivalent to *one* root cause.
    pub fn leaf(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        let mut current = self.inner.as_frame();
        while let Some(next) = current.children().last() {
            current = next;
        }
        current.as_error_send_sync()
    }
    //
    // /// Return an iterator over all sources, i.e. the linear chain.
    // pub fn iter_sources(&self) -> ErrorIter<'_> {
    //     use std::error::Error;
    //     ErrorIter { current: self.source() }
    // }
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
            Inner::ExnAsError(frame) | Inner::Exn(frame) => frame.children().first().map(exn::Frame::as_error),
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

// TODO: actually use frames, source doesn't really work.
// #[derive(Clone, Copy)]
// pub struct ErrorIter<'a> {
//     current: Option<&'a (dyn std::error::Error + 'static)>,
// }
//
// impl<'a> Iterator for ErrorIter<'a> {
//     type Item = &'a (dyn std::error::Error + 'static);
//
//     fn next(&mut self) -> Option<Self::Item> {
//         let current = self.current;
//         self.current = self.current.and_then(std::error::Error::source);
//         current
//     }
// }
