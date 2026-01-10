use crate::{exn, Error, Exn};
use std::fmt::Formatter;

pub(super) enum Inner {
    Boxed(Box<dyn std::error::Error + Send + Sync>),
    Exn(Box<exn::Frame>),
}

impl Error {
    /// Create a new instance representing the given `error`.
    pub fn from_error(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Error {
            inner: Inner::Boxed(Box::new(error)),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            Inner::Boxed(err) => std::fmt::Display::fmt(&*err, f),
            Inner::Exn(frame) => std::fmt::Display::fmt(frame, f),
        }
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            Inner::Boxed(err) => std::fmt::Debug::fmt(&*err, f),
            Inner::Exn(frame) => std::fmt::Debug::fmt(frame, f),
        }
    }
}

impl std::error::Error for Error {
    /// Return the first source of an [Exn](crate::Exn) error, or the source of a boxed error.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.inner {
            Inner::Boxed(err) => err.source(),
            Inner::Exn(frame) => frame.children().first().map(|f| f.as_error()),
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
