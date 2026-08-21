use crate::write_location;
use std::fmt::{Debug, Display, Formatter};
use std::panic::Location;
use std::sync::Arc;

/// A generic error which represents a linked-list of errors and exposes it with [source()](std::error::Error::source).
/// It's meant to be the target of a conversion of any [Exn](crate::Exn) error tree.
///
/// It's useful for inter-op with other error handling crates like `anyhow` which offer simplified access to the error chain,
/// and thus is expected to be wrapped in one of their types intead of being used directly.
pub struct ChainedError {
    /// The error exposed at this flattened frame, preserving its concrete type for downcasting.
    pub(crate) err: ErrorHandle,
    /// The call site captured when the corresponding error frame was created.
    pub(crate) location: &'static Location<'static>,
    #[cfg_attr(
        not(all(feature = "auto-chain-error", not(feature = "tree-error"))),
        expect(dead_code, reason = "used only by the auto-chain Error representation")
    )]
    /// Whether this frame was selected as the probable cause before flattening the error tree, using the root as fallback.
    pub(crate) is_probable_cause: bool,
    #[cfg_attr(
        not(all(feature = "auto-chain-error", not(feature = "tree-error"))),
        expect(dead_code, reason = "used only by the auto-chain Error representation")
    )]
    /// The index of this node's logical parent in the breadth-first flattened chain, or `None` for the root.
    pub(crate) logical_parent: Option<usize>,
    /// The next frame in the flattened error chain, kept wrapped to retain its location and subsequent frames.
    pub(crate) source: Option<Box<ChainedError>>,
}

impl Debug for ChainedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self.err.error(), f)
    }
}

impl Display for ChainedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.err.error(), f)?;
        if !f.alternate() {
            write_location(f, self.location)?;
        }
        Ok(())
    }
}

impl std::error::Error for ChainedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // Expose the next `ChainedError`, rather than only its inner error, so standard source-chain walkers continue
        // through the remaining flattened frames and retain each frame's location. Once that synthetic chain ends,
        // continue with the inner error's native source chain so sources not represented by another frame remain visible.
        self.source
            .as_deref()
            .map(|err| err as &(dyn std::error::Error + 'static))
            .or_else(|| self.err.error().source())
    }
}

/// An owning handle to either an error or one of its borrowed native sources.
///
/// Keeping the source-chain root in an [`Arc`] makes every source reachable for the lifetime of the flattened chain.
/// A handle cannot store both that owner and a reference borrowed from its [`std::error::Error::source()`] chain without
/// becoming self-referential. Instead, `source_depth` records how many `source()` links lead from `owner` to the error
/// represented by this handle: zero represents `owner`, one represents `owner.source()`, and so on. [`Self::error()`]
/// follows that path whenever the borrowed error is needed.
///
/// Resolving a handle assumes that an error's source chain remains stable while the owning error is alive, as conventional
/// [`std::error::Error`] implementations do.
pub(crate) struct ErrorHandle {
    /// The error that owns the complete native source chain.
    owner: Arc<dyn std::error::Error + Send + Sync + 'static>,
    /// The number of [`std::error::Error::source()`] links to follow from `owner` to reach this handle's error.
    source_depth: usize,
}

impl ErrorHandle {
    pub(crate) fn new(error: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
        ErrorHandle {
            owner: error.into(),
            source_depth: 0,
        }
    }

    pub(crate) fn error(&self) -> &(dyn std::error::Error + 'static) {
        let mut error: &(dyn std::error::Error + 'static) = self.owner.as_ref();
        for _ in 0..self.source_depth {
            error = error
                .source()
                .expect("a captured source path remains stable while its owning error is alive");
        }
        error
    }

    pub(crate) fn source(&self) -> Option<Self> {
        self.error().source()?;
        Some(ErrorHandle {
            owner: Arc::clone(&self.owner),
            source_depth: self.source_depth + 1,
        })
    }

    #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
    pub(crate) fn is_native_source(&self) -> bool {
        self.source_depth > 0
    }
}
