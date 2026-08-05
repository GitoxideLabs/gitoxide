use crate::write_location;
use std::fmt::{Debug, Display, Formatter};
use std::panic::Location;

/// A generic error which represents a linked-list of errors and exposes it with [source()](std::error::Error::source).
/// It's meant to be the target of a conversion of any [Exn](crate::Exn) error tree.
///
/// It's useful for inter-op with other error handling crates like `anyhow` which offer simplified access to the error chain,
/// and thus is expected to be wrapped in one of their types intead of being used directly.
pub struct ChainedError {
    pub(crate) err: Box<dyn std::error::Error + Send + Sync + 'static>,
    pub(crate) location: &'static Location<'static>,
    #[cfg_attr(
        not(all(feature = "auto-chain-error", not(feature = "tree-error"))),
        allow(dead_code)
    )]
    pub(crate) is_probable_cause: bool,
    pub(crate) source: Option<Box<ChainedError>>,
}

impl Debug for ChainedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.err, f)
    }
}

impl Display for ChainedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.err, f)?;
        if !f.alternate() {
            write_location(f, self.location)?;
        }
        Ok(())
    }
}

impl std::error::Error for ChainedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| e as &(dyn std::error::Error + 'static))
    }
}
