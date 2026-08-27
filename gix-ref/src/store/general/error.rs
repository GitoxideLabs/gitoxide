use std::{error::Error, fmt};

/// An adapter-independent failure from a reference-storage operation.
///
/// The concrete adapter error remains available through the standard error
/// source chain without becoming part of the public store contract.
#[derive(Debug)]
pub struct BackendError {
    operation: &'static str,
    source: Box<dyn Error + Send + Sync + 'static>,
}

impl BackendError {
    pub(crate) fn new(operation: &'static str, source: impl Error + Send + Sync + 'static) -> Self {
        BackendError {
            operation,
            source: Box::new(source),
        }
    }

    /// Return the logical operation that failed.
    pub fn operation(&self) -> &'static str {
        self.operation
    }
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Could not {}", self.operation)
    }
}

impl Error for BackendError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}
