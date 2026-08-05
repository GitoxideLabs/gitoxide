#[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
mod _impl {
    use crate::{Error, Exn};
    use std::fmt::Formatter;

    /// Utilities
    impl Error {
        /// Return the error that is most likely the root cause, based on heuristics.
        /// Note that if there is nothing but this error, i.e. no source or children, this error is returned.
        pub fn probable_cause(&self) -> &(dyn std::error::Error + 'static) {
            let root = self.inner.frame();
            root.probable_cause().unwrap_or(root).error()
        }

        /// Return an iterator over all errors in the tree in breadth-first order, starting with this one.
        pub fn sources(&self) -> impl Iterator<Item = &(dyn std::error::Error + 'static)> + '_ {
            self.inner.frame().iter_frames().map(|f| f.error() as _)
        }

        /// Return `true` if retrying the failed operation might succeed.
        pub fn can_retry(&self) -> bool {
            self.sources().any(super::is_retryable)
        }

        /// Return `true` if malformed or internally inconsistent data caused the failure.
        pub fn is_corrupted(&self) -> bool {
            self.sources().any(super::is_corrupted)
        }

        /// Return `true` if a requested resource was not found.
        pub fn is_not_found(&self) -> bool {
            self.sources().any(super::is_not_found)
        }
    }

    pub(crate) enum Inner {
        ExnAsError(Box<crate::exn::Frame>),
        Exn(Box<crate::exn::Frame>),
    }

    impl Inner {
        fn frame(&self) -> &crate::exn::Frame {
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

        /// Create a new instance representing an already boxed `error`.
        #[track_caller]
        pub fn from_boxed(error: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
            Self::from_error(super::BoxedError(error))
        }
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match &self.inner {
                Inner::ExnAsError(err) => std::fmt::Display::fmt(err.error(), f),
                Inner::Exn(frame) => std::fmt::Display::fmt(frame, f),
            }
        }
    }

    impl std::fmt::Debug for Error {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match &self.inner {
                Inner::ExnAsError(err) => std::fmt::Debug::fmt(err.error(), f),
                Inner::Exn(frame) => std::fmt::Debug::fmt(frame, f),
            }
        }
    }

    impl std::error::Error for Error {
        /// Return the first source of an [Exn] error, or the source of a boxed error.
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match &self.inner {
                Inner::ExnAsError(frame) | Inner::Exn(frame) => frame.children().first().map(|f| f.error() as _),
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
}
#[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
pub(super) use _impl::Inner;

#[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
mod _impl {
    use crate::{Error, Exn};
    use std::fmt::Formatter;

    /// Utilities
    impl Error {
        /// Return the error that is most likely the root cause, based on heuristics.
        /// Note that if there is nothing but this error, i.e. no source or children, this error is returned.
        pub fn probable_cause(&self) -> &(dyn std::error::Error + 'static) {
            use std::error::Error;
            self.inner
                .source()
                .unwrap_or(self as &(dyn std::error::Error + 'static))
        }

        /// Return an iterator over all errors in the tree in breadth-first order, starting with this one.
        pub fn sources(&self) -> impl Iterator<Item = &(dyn std::error::Error + 'static)> + '_ {
            std::iter::successors(Some(&self.inner as &(dyn std::error::Error + 'static)), |err| {
                err.source()
            })
        }

        /// Return `true` if retrying the failed operation might succeed.
        pub fn can_retry(&self) -> bool {
            std::iter::successors(Some(&self.inner), |err| err.source.as_deref())
                .any(|err| super::is_retryable(err.err.as_ref()))
        }

        /// Return `true` if malformed or internally inconsistent data caused the failure.
        pub fn is_corrupted(&self) -> bool {
            std::iter::successors(Some(&self.inner), |err| err.source.as_deref())
                .any(|err| super::is_corrupted(err.err.as_ref()))
        }

        /// Return `true` if a requested resource was not found.
        pub fn is_not_found(&self) -> bool {
            std::iter::successors(Some(&self.inner), |err| err.source.as_deref())
                .any(|err| super::is_not_found(err.err.as_ref()))
        }
    }

    impl Error {
        /// Create a new instance representing the given `error`.
        #[track_caller]
        pub fn from_error(error: impl std::error::Error + Send + Sync + 'static) -> Self {
            Error {
                inner: Exn::new(error).into_chain(),
            }
        }

        /// Create a new instance representing an already boxed `error`.
        #[track_caller]
        pub fn from_boxed(error: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
            Self::from_error(super::BoxedError(error))
        }
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            std::fmt::Display::fmt(&self.inner, f)
        }
    }

    impl std::fmt::Debug for Error {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            std::fmt::Debug::fmt(&self.inner, f)
        }
    }

    impl std::error::Error for Error {
        /// Return the first source of an [Exn] error, or the source of a boxed error.
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            self.inner.source()
        }
    }

    impl<E> From<Exn<E>> for Error
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        fn from(err: Exn<E>) -> Self {
            Error {
                inner: err.into_chain(),
            }
        }
    }
}

struct BoxedError(Box<dyn std::error::Error + Send + Sync + 'static>);

impl std::fmt::Display for BoxedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Debug for BoxedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for BoxedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref())
    }
}

/// Return `true` if `err` or any error in its source chain indicates that retrying might succeed.
pub fn can_retry(err: &(dyn std::error::Error + 'static)) -> bool {
    is_retryable(err)
}

fn is_retryable(err: &(dyn std::error::Error + 'static)) -> bool {
    error_chain(err).any(|err| {
        if err.is::<crate::RetryableError>() {
            return true;
        }
        let Some(err) = err.downcast_ref::<std::io::Error>() else {
            return false;
        };
        use std::io::ErrorKind::*;
        matches!(
            err.kind(),
            Interrupted
                | UnexpectedEof
                | OutOfMemory
                | TimedOut
                | BrokenPipe
                | AddrInUse
                | ConnectionAborted
                | ConnectionReset
                | ConnectionRefused
        )
    })
}

fn is_corrupted(err: &(dyn std::error::Error + 'static)) -> bool {
    error_chain(err).any(|err| err.is::<crate::CorruptionError>())
}

fn is_not_found(err: &(dyn std::error::Error + 'static)) -> bool {
    error_chain(err).any(|err| {
        err.is::<crate::NotFoundError>()
            || err
                .downcast_ref::<std::io::Error>()
                .is_some_and(|err| err.kind() == std::io::ErrorKind::NotFound)
    })
}

fn error_chain<'a>(
    err: &'a (dyn std::error::Error + 'static),
) -> impl Iterator<Item = &'a (dyn std::error::Error + 'static)> {
    std::iter::successors(Some(err), |err| err.source())
}
