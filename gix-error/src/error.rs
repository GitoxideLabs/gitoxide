/// A borrowed error together with its optional caller location, intended for diagnostic display.
///
/// Errors owned by a [`crate::Frame`] have the location captured when that frame was created. Native
/// [`std::error::Error::source()`] values have no location because no caller location was captured for them.
///
/// Unlike [`crate::Frame`], this type neither owns the error nor represents relationships in an error tree. This lets
/// [`crate::Error::iter_errors_with_locations()`] provide the same lightweight view for the tree-backed and flattened-chain
/// representations.
///
/// Its normal [`Display`](std::fmt::Display) output appends the location when one is available. Alternate formatting
/// (`{source:#}`) forwards alternate formatting to the underlying error and always omits the location.
#[derive(Clone, Copy, Debug)]
pub struct DisplaySource<'a> {
    error: &'a (dyn std::error::Error + 'static),
    location: Option<&'static std::panic::Location<'static>>,
}

impl<'a> DisplaySource<'a> {
    /// Return the stored error, preserving its concrete type for downcasting.
    pub fn error(&self) -> &'a (dyn std::error::Error + 'static) {
        self.error
    }

    /// Return the caller location captured for this error frame, or `None` for a native error source.
    pub fn location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.location
    }
}

impl std::fmt::Display for DisplaySource<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.error, f)?;
        if !f.alternate() {
            if let Some(location) = self.location {
                crate::write_location(f, location)?;
            }
        }
        Ok(())
    }
}

impl crate::Error {
    /// Find the first stored error or native source that downcasts to `T` in logical breadth-first order.
    pub fn downcast_any_ref<T: std::error::Error + 'static>(&self) -> Option<&T> {
        self.iter_errors().find_map(|error| error.downcast_ref())
    }
}

#[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
mod _impl {
    use crate::{DisplaySource, Error, Exn};
    use std::fmt::Formatter;

    /// Utilities
    impl Error {
        /// Return the error stored at this error boundary.
        ///
        /// This is the first error yielded by [`Self::iter_errors()`] and is distinct from
        /// [`Self::probable_cause()`].
        pub fn error(&self) -> &(dyn std::error::Error + 'static) {
            self.inner.frame().error()
        }

        /// Return the error that is most likely the root cause, based on heuristics.
        /// Note that if there is nothing but this error, i.e. no source or children, this error is returned.
        pub fn probable_cause(&self) -> &(dyn std::error::Error + 'static) {
            let root = self.inner.frame();
            let cause = root.probable_cause().unwrap_or_else(|| root.error());
            cause.downcast_ref::<Error>().map_or(cause, Error::probable_cause)
        }

        /// Return the stored error and all frame errors and native sources reachable from it, recursively expanding nested
        /// [`Error`] values.
        ///
        /// The first item is the error stored inside this [`Error`], not this `Error` wrapper. Remaining errors are ordered
        /// logically breadth-first; a frame's direct native source precedes its explicitly raised child frames.
        /// These are references to the underlying errors, so their concrete types remain available for *downcasting* and
        /// their own [`Display`](std::fmt::Display) implementations can be used. Use
        /// [`Self::iter_errors_with_locations()`] to access caller locations.
        pub fn iter_errors(&self) -> impl Iterator<Item = &(dyn std::error::Error + 'static)> + '_ {
            self.collect_errors_with_locations()
                .into_iter()
                .map(|source| source.error)
        }

        /// Return the same errors as [`Self::iter_errors()`], paired with their captured caller locations where available
        /// and in the same order.
        pub fn iter_errors_with_locations(&self) -> impl Iterator<Item = DisplaySource<'_>> + '_ {
            self.collect_errors_with_locations().into_iter()
        }

        fn collect_errors_with_locations(&self) -> Vec<DisplaySource<'_>> {
            let mut queue = std::collections::VecDeque::from([crate::exn::ErrorNode::Frame(self.inner.frame())]);
            let mut out = Vec::new();
            while let Some(node) = queue.pop_front() {
                let error = node.error();
                out.push(DisplaySource {
                    error,
                    location: node.captured_location(),
                });
                if let Some(error) = error.downcast_ref::<Error>() {
                    queue.push_back(crate::exn::ErrorNode::Frame(error.inner.frame()));
                }
                queue.extend(node.children());
            }
            out
        }

        /// Return `true` if any stored error, or an error in its [`source()`](std::error::Error::source) chain, is:
        ///
        /// * explicitly marked with [`RetryableError`](crate::RetryableError), or
        /// * an [`std::io::Error`] with kind `Interrupted`, `UnexpectedEof`, `OutOfMemory`, `TimedOut`, `BrokenPipe`,
        ///   `AddrInUse`, `ConnectionAborted`, `ConnectionReset`, or `ConnectionRefused`.
        ///
        /// Nested [`Error`] values are inspected recursively. `false` only means that no known retryable error was
        /// found; it does not guarantee that retrying cannot succeed.
        pub fn can_retry(&self) -> bool {
            self.iter_errors().any(super::is_retryable)
        }

        /// Return `true` if malformed or internally inconsistent data caused the failure.
        pub fn is_corrupted(&self) -> bool {
            self.iter_errors().any(super::is_corrupted)
        }

        /// Return `true` if a requested resource was not found.
        pub fn is_not_found(&self) -> bool {
            self.iter_errors().any(super::is_not_found)
        }

        /// Return `true` if invalid input caused the failure.
        pub fn is_validation(&self) -> bool {
            self.iter_errors().any(super::is_validation)
        }
    }

    pub(crate) enum Inner {
        ExnAsError(Box<crate::exn::Frame>),
        Exn(Box<crate::exn::Frame>),
    }

    impl Inner {
        pub(crate) fn frame(&self) -> &crate::exn::Frame {
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
            Self::from_error(crate::Untyped::from_boxed(error))
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
                Inner::ExnAsError(frame) | Inner::Exn(frame) => {
                    let error = frame.error();
                    (!error.is::<Error>())
                        .then(|| error.source())
                        .flatten()
                        .or_else(|| frame.children().first().map(|frame| frame.error() as _))
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
}
#[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
pub(super) use _impl::Inner;

#[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
mod _impl {
    use crate::{DisplaySource, Error, Exn};
    use std::fmt::Formatter;

    /// A temporary adjacency-list node used to recover the logical error graph from flattened [`crate::ChainedError`]
    /// values.
    ///
    /// Auto-chain storage is a linked list for compatibility with [`std::error::Error::source()`]. The iteration APIs
    /// rebuild its retained parent relationships in a `Vec<ErrorGraphNode>` so nested [`Error`] values can join the same
    /// breadth-first traversal without changing that source-chain representation.
    struct ErrorGraphNode<'a> {
        /// The borrowed error and optional frame location yielded for this node.
        source: DisplaySource<'a>,
        /// Indices of logical children in the temporary node vector, in traversal order.
        children: Vec<usize>,
    }

    /// Utilities
    impl Error {
        /// Return the error stored at this error boundary.
        ///
        /// This is the first error yielded by [`Self::iter_errors()`] and is distinct from
        /// [`Self::probable_cause()`].
        pub fn error(&self) -> &(dyn std::error::Error + 'static) {
            self.inner.err.error()
        }

        /// Return the error that is most likely the root cause, based on heuristics.
        /// Note that if there is nothing but this error, i.e. no source or children, this error is returned.
        pub fn probable_cause(&self) -> &(dyn std::error::Error + 'static) {
            let cause = std::iter::successors(Some(&self.inner), |err| err.source.as_deref())
                .find(|err| err.is_probable_cause)
                .map_or(self as &(dyn std::error::Error + 'static), |err| err.err.error());
            cause.downcast_ref::<Error>().map_or(cause, Error::probable_cause)
        }

        /// Return the stored error and all frame errors and native sources reachable from it, recursively expanding nested
        /// [`Error`] values.
        ///
        /// The first item is the error stored inside this [`Error`], not this `Error` wrapper. Remaining errors retain the
        /// logical breadth-first order captured while flattening; a frame's direct native source precedes its
        /// explicitly raised child frames. These are references to the underlying errors, so their concrete types remain
        /// available for downcasting and their own [`Display`](std::fmt::Display) implementations can be used. Use
        /// [`Self::iter_errors_with_locations()`] to access caller locations.
        pub fn iter_errors(&self) -> impl Iterator<Item = &(dyn std::error::Error + 'static)> + '_ {
            self.collect_errors_with_locations()
                .into_iter()
                .map(|source| source.error)
        }

        /// Return the same errors as [`Self::iter_errors()`], paired with their captured caller locations where available
        /// and in the same order.
        ///
        /// The stored error, rather than this `Error` wrapper, is the first item. Frame errors have the location
        /// captured when their frame was created; native sources have no caller location of their own. This enables opt-in
        /// caller traces without making locations part of the underlying errors or their
        /// [`Display`](std::fmt::Display) implementations. Each [`DisplaySource`] exposes both values for custom rendering,
        /// and its own [`Display`](std::fmt::Display) implementation appends an available location by default.
        pub fn iter_errors_with_locations(&self) -> impl Iterator<Item = DisplaySource<'_>> + '_ {
            self.collect_errors_with_locations().into_iter()
        }

        fn collect_errors_with_locations(&self) -> Vec<DisplaySource<'_>> {
            let mut graph = Vec::new();
            let (root, nested) = self.append_error_chain(&mut graph);
            let mut pending = std::collections::VecDeque::from(nested);
            while let Some((parent, error)) = pending.pop_front() {
                let (nested_root, more_nested) = error.append_error_chain(&mut graph);
                graph[parent].children.insert(0, nested_root);
                pending.extend(more_nested);
            }

            let mut queue = std::collections::VecDeque::from([root]);
            let mut out = Vec::new();
            while let Some(index) = queue.pop_front() {
                let node = &graph[index];
                out.push(node.source);
                queue.extend(node.children.iter().copied());
            }
            out
        }

        /// Append this error boundary's flattened chain to `graph` and reconstruct its local parent-child relationships.
        ///
        /// The first tuple value is the graph index of the stored error. Each entry in the second value contains the graph
        /// index of a node whose error is another [`Error`], paired with that nested boundary. The caller appends those
        /// nested chains iteratively and connects each returned root as the wrapper node's first child, avoiding recursive
        /// graph construction before the completed graph is traversed breadth-first.
        fn append_error_chain<'a>(&'a self, graph: &mut Vec<ErrorGraphNode<'a>>) -> (usize, Vec<(usize, &'a Error)>) {
            let chain = std::iter::successors(Some(&self.inner), |err| err.source.as_deref()).collect::<Vec<_>>();
            let root = graph.len();
            graph.extend(chain.iter().map(|chained| ErrorGraphNode {
                source: DisplaySource {
                    error: chained.err.error(),
                    location: (!chained.err.is_native_source()).then_some(chained.location),
                },
                children: Vec::new(),
            }));

            for (index, chained) in chain.iter().enumerate() {
                if let Some(parent) = chained.logical_parent {
                    graph[root + parent].children.push(root + index);
                }
            }
            let nested = chain
                .into_iter()
                .enumerate()
                .filter_map(|(index, chained)| {
                    chained
                        .err
                        .error()
                        .downcast_ref::<Error>()
                        .map(|error| (root + index, error))
                })
                .collect();
            (root, nested)
        }

        /// Return `true` if any stored error, or an error in its [`source()`](std::error::Error::source) chain, is:
        ///
        /// * explicitly marked with [`RetryableError`](crate::RetryableError), or
        /// * an [`std::io::Error`] with kind `Interrupted`, `UnexpectedEof`, `OutOfMemory`, `TimedOut`, `BrokenPipe`,
        ///   `AddrInUse`, `ConnectionAborted`, `ConnectionReset`, or `ConnectionRefused`.
        ///
        /// Nested [`Error`] values are inspected recursively. `false` only means that no known retryable error was
        /// found; it does not guarantee that retrying cannot succeed.
        pub fn can_retry(&self) -> bool {
            self.iter_errors().any(super::is_retryable)
        }

        /// Return `true` if malformed or internally inconsistent data caused the failure.
        pub fn is_corrupted(&self) -> bool {
            self.iter_errors().any(super::is_corrupted)
        }

        /// Return `true` if a requested resource was not found.
        pub fn is_not_found(&self) -> bool {
            self.iter_errors().any(super::is_not_found)
        }

        /// Return `true` if invalid input caused the failure.
        pub fn is_validation(&self) -> bool {
            self.iter_errors().any(super::is_validation)
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
            Self::from_error(crate::Untyped::from_boxed(error))
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

/// Return `true` if `err` or any error in its [`source()`](std::error::Error::source) chain is explicitly marked with
/// [`RetryableError`](crate::RetryableError), or is an [`std::io::Error`] whose kind is `Interrupted`, `UnexpectedEof`,
/// `OutOfMemory`, `TimedOut`, `BrokenPipe`, `AddrInUse`, `ConnectionAborted`, `ConnectionReset`, or `ConnectionRefused`.
///
/// Nested [`crate::Error`] values are inspected recursively. `false` only means that no known retryable error was found; it
/// does not guarantee that retrying cannot succeed.
pub fn can_retry(err: &(dyn std::error::Error + 'static)) -> bool {
    is_retryable(err)
}

fn is_retryable(err: &(dyn std::error::Error + 'static)) -> bool {
    error_chain(err).any(|err| {
        if let Some(err) = err.downcast_ref::<crate::Error>() {
            return err.can_retry();
        }
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
    error_chain(err).any(|err| {
        err.downcast_ref::<crate::Error>()
            .is_some_and(crate::Error::is_corrupted)
            || err.is::<crate::CorruptionError>()
    })
}

fn is_not_found(err: &(dyn std::error::Error + 'static)) -> bool {
    error_chain(err).any(|err| {
        err.downcast_ref::<crate::Error>()
            .is_some_and(crate::Error::is_not_found)
            || err.is::<crate::NotFoundError>()
            || err
                .downcast_ref::<std::io::Error>()
                .is_some_and(|err| err.kind() == std::io::ErrorKind::NotFound)
    })
}

fn is_validation(err: &(dyn std::error::Error + 'static)) -> bool {
    error_chain(err).any(|err| {
        err.downcast_ref::<crate::Error>()
            .is_some_and(crate::Error::is_validation)
            || err.is::<crate::ValidationError>()
    })
}

fn error_chain<'a>(
    err: &'a (dyn std::error::Error + 'static),
) -> impl Iterator<Item = &'a (dyn std::error::Error + 'static)> {
    std::iter::successors(Some(err), |err| err.source())
}
