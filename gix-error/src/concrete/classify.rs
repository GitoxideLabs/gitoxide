use std::fmt::{Display, Formatter};

/// A transparent wrapper for dependency-specific errors known to be retryable.
#[derive(Debug)]
pub struct RetryableError(Box<dyn std::error::Error + Send + Sync + 'static>);

impl RetryableError {
    /// Mark `source` as retryable.
    pub fn new(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        RetryableError(Box::new(source))
    }
}

impl Display for RetryableError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl std::error::Error for RetryableError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref())
    }
}
