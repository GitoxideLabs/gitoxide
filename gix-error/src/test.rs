use crate::Error;

/// An error for test functions that accepts any error supported by a boxed standard error.
///
/// Unlike [`Error`], this type deliberately does not implement [`std::error::Error`]. This allows it to accept arbitrary
/// errors via [`From`] without conflicting with the standard library's identity conversion.
/// Its [`Debug`](std::fmt::Debug) output includes the complete error tree or chain and all captured caller locations.
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
        return write!(f, "{:?}", self.0.inner.frame());

        #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
        {
            let mut errors = self.0.iter_errors_with_locations();
            if let Some(error) = errors.next() {
                write!(f, "{error}")?;
                let mut errors = errors.peekable();
                if errors.peek().is_some() {
                    write!(f, "\n\nCaused by:")?;
                    for (index, error) in errors.enumerate() {
                        write!(f, "\n    {index}: {error}")?;
                    }
                }
            }
            Ok(())
        }
    }
}
