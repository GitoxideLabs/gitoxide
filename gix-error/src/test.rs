use crate::Error;

/// An error for test functions that accepts any error supported by a boxed standard error.
///
/// Unlike [`Error`], this type deliberately does not implement [`std::error::Error`]. This allows it to accept arbitrary
/// errors via [`From`] without conflicting with the standard library's identity conversion.
/// Its [`Debug`](std::fmt::Debug) output includes the complete diagnostic tree or chain, omitting classification markers
/// unless only markers are available. Custom I/O wrappers show their kind, with their payloads reported separately.
/// Captured caller locations are included unless alternate formatting is used.
pub struct TestError(Error);

/// A result type for test functions whose errors are reported through [`TestError`]'s complete diagnostics.
pub type TestResult<T = ()> = std::result::Result<T, TestError>;

impl<E> From<E> for TestError
where
    E: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
{
    #[track_caller]
    fn from(error: E) -> Self {
        let error = error.into();
        TestError(match error.downcast::<Error>() {
            Ok(error) => *error,
            Err(error) => Error::from_boxed(error),
        })
    }
}

impl From<TestError> for Error {
    fn from(error: TestError) -> Self {
        error.0
    }
}

impl std::fmt::Debug for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
        return std::fmt::Debug::fmt(self.0.inner.frame(), f);

        #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
        {
            let write_error =
                |error: crate::error::DisplaySource<'_>, f: &mut std::fmt::Formatter<'_>| -> std::fmt::Result {
                    crate::exn::impls::ErrorMode::Display.fmt(error.error(), f)?;
                    if !f.alternate()
                        && let Some(location) = error.location()
                    {
                        crate::write_location(f, location)?;
                    }
                    Ok(())
                };
            let mut errors = self
                .0
                .iter_errors_with_locations()
                // Boundary contents are emitted separately by the iterator.
                .filter(|source| !source.error().is::<Error>())
                .peekable();
            let Some(error) = errors.next() else {
                return std::fmt::Display::fmt(&self.0.inner, f);
            };
            write_error(error, f)?;
            if errors.peek().is_some() {
                write!(f, "\n\nCaused by:")?;
                for (index, error) in errors.enumerate() {
                    write!(f, "\n    {index}: ")?;
                    write_error(error, f)?;
                }
            }
            Ok(())
        }
    }
}
