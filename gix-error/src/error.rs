// Keep inherent methods on Error and Exn while sharing their implementation and documentation.
macro_rules! classification_predicates {
    () => {
        /// Return `true` if any stored error or native source is explicitly marked with [`crate::RetryableError`].
        ///
        /// Nested [`crate::Error`] values are inspected recursively. Unlike [`Self::can_retry()`], this does not infer
        /// retryability from I/O error kinds.
        pub fn is_retryable(&self) -> bool {
            self.classify().is_retryable()
        }

        /// Return `true` if any stored error or native source reports resource exhaustion.
        ///
        /// This recognizes [`crate::ResourceExhaustionError`] of any kind, [`std::collections::TryReserveError`], and
        /// [`std::io::ErrorKind::OutOfMemory`], including within nested [`crate::Error`] values.
        pub fn is_resource_exhausted(&self) -> bool {
            self.classify().is_resource_exhausted()
        }

        /// Return `true` if any stored error, or an error in its [`source()`](std::error::Error::source) chain, is:
        ///
        /// * explicitly marked with [`RetryableError`](crate::RetryableError), or
        /// * an [`std::io::Error`] with kind `Interrupted` or `TimedOut`.
        ///
        /// Nested [`crate::Error`] values are inspected recursively. `false` only means that no known retryable error was
        /// found; it does not guarantee that retrying cannot succeed.
        pub fn can_retry(&self) -> bool {
            self.classify().can_retry()
        }

        /// Return `true` if any stored error, or an error in its [`source()`](std::error::Error::source) chain, is:
        ///
        /// * explicitly marked with [`RetryableError`](crate::RetryableError), or
        /// * an [`std::io::Error`] with kind `Interrupted`, `UnexpectedEof`, `OutOfMemory`, `TimedOut`, `BrokenPipe`,
        ///   `AddrInUse`, `ConnectionAborted`, `ConnectionReset`, or `ConnectionRefused`.
        ///
        /// This applies a more lenient policy than [`Self::can_retry`]. Nested [`crate::Error`] values are inspected recursively.
        /// `false` only means that no known retryable error was found; it does not guarantee that retrying cannot succeed.
        pub fn can_retry_lenient(&self) -> bool {
            self.classify().can_retry_lenient()
        }

        /// Return `true` if malformed or internally inconsistent data caused the failure.
        pub fn is_corrupted(&self) -> bool {
            self.classify().is_corrupted()
        }

        /// Return `true` if a requested resource was not found.
        pub fn is_not_found(&self) -> bool {
            self.classify().is_not_found()
        }

        /// Return `true` if invalid input caused the failure.
        pub fn is_validation(&self) -> bool {
            self.classify().is_validation()
        }
    };
}

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
        if !f.alternate()
            && let Some(location) = self.location
        {
            crate::write_location(f, location)?;
        }
        Ok(())
    }
}

impl crate::Error {
    /// Lazily visit stored errors and native sources in logical breadth-first order, expanding nested [`crate::Error`] values.
    ///
    /// The stored error is first. A frame's native source precedes its explicitly raised children. Concrete error types
    /// remain available for downcasting. Traversal stops when the iterator is dropped.
    pub fn iter_errors(&self) -> impl Iterator<Item = &(dyn std::error::Error + 'static)> + '_ {
        self.iter_errors_with_locations().map(|source| source.error)
    }

    /// Visit the same errors as [`Self::iter_errors()`], with caller locations for explicitly raised frames.
    /// Native sources have no caller location of their own. [`DisplaySource`] can render either representation.
    pub fn iter_errors_with_locations(&self) -> impl Iterator<Item = DisplaySource<'_>> + '_ {
        Errors::new(self.iter_root())
    }

    /// Find the first stored error or native source that downcasts to `T` in logical breadth-first order.
    pub fn downcast_any_ref<T: std::error::Error + 'static>(&self) -> Option<&T> {
        self.iter_errors().find_map(|error| error.downcast_ref())
    }

    /// Return all known classifications in the same logical breadth-first order as [`Self::iter_errors()`].
    ///
    /// Unknown errors are omitted. Classifications aren't deduplicated because distinct errors may independently have
    /// the same meaning. Each item retains the classified error for downcasting and origin inspection.
    pub fn classify(&self) -> Classifications<'_> {
        classify(self)
    }

    classification_predicates!();
}

/// Classification helpers for inspecting an exception without consuming it or losing its typed outer error.
///
/// The corresponding helpers on [`crate::Error`] would require consuming the exception with
/// [`into_error()`](crate::Exn::into_error), while dereferencing an exception only exposes its outer error `E`, not
/// the full error tree. These helpers inspect that tree directly, so callers can recognize a failure's meaning
/// even when it is wrapped in context, and still propagate the original exception afterward.
impl<E: std::error::Error + Send + Sync + 'static> crate::Exn<E> {
    /// Return all known classifications in logical breadth-first order, including native sources and nested
    /// [`crate::Error`] values.
    ///
    /// As with [`crate::Error::classify()`], unknown errors are omitted, classifications aren't deduplicated, and each
    /// item retains the classified error for downcasting and origin inspection.
    pub fn classify(&self) -> Classifications<'_> {
        Classifications(Errors::new(Node::Frame(self.frame())))
    }

    classification_predicates!();
}

/// The semantic class of an error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Class {
    /// Function or method input was invalid.
    Validation,
    /// Stored or streamed data was malformed or internally inconsistent.
    Corruption,
    /// A requested resource does not exist.
    NotFound,
    /// Retrying the operation may succeed.
    Retryable,
    /// A finite resource was exhausted.
    ResourceExhaustion(crate::ResourceExhaustionKind),
    /// An I/O failure not normalized to another semantic class.
    Io(std::io::ErrorKind),
}

/// A semantic class together with the concrete error which established it.
#[derive(Clone, Copy, Debug)]
pub struct Classification<'a> {
    class: Class,
    error: &'a (dyn std::error::Error + 'static),
}

/// Lazily inspect the classifications of any borrowed error, including its native sources, I/O payloads and nested
/// [`crate::Error`] values. Unknown errors are omitted and distinct causes may yield the same classification.
///
/// ```
/// let error = std::io::Error::other(gix_error::NotFoundError::new("missing object"));
/// assert!(gix_error::classify(&error).is_not_found());
/// ```
pub fn classify<'a>(err: &'a (dyn std::error::Error + 'static)) -> Classifications<'a> {
    Classifications(Errors::new(
        err.downcast_ref::<crate::Error>()
            .map_or(Node::Source(err), crate::Error::iter_root),
    ))
}

/// A lazy iterator over classified causes. Its predicates consume the remaining iterator and stop at the first match.
pub struct Classifications<'a>(Errors<'a>);

impl<'a> Iterator for Classifications<'a> {
    type Item = Classification<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.find_map(|source| classify_one(source.error))
    }
}

impl Classifications<'_> {
    /// Return whether any remaining cause is explicitly marked as retryable.
    pub fn is_retryable(self) -> bool {
        self.has(Class::Retryable)
    }

    /// Apply the conservative retry policy of [`crate::Error::can_retry()`] to the remaining causes.
    pub fn can_retry(mut self) -> bool {
        self.any(|classification| class_can_retry(classification.class()))
    }

    /// Apply the broader I/O policy of [`crate::Error::can_retry_lenient()`] to the remaining causes.
    pub fn can_retry_lenient(mut self) -> bool {
        self.any(classification_can_retry_lenient)
    }

    /// Return whether any remaining cause reports a missing resource.
    pub fn is_not_found(self) -> bool {
        self.has(Class::NotFound)
    }

    /// Return whether any remaining cause reports invalid input.
    pub fn is_validation(self) -> bool {
        self.has(Class::Validation)
    }

    /// Return whether any remaining cause reports malformed or inconsistent data.
    pub fn is_corrupted(self) -> bool {
        self.has(Class::Corruption)
    }

    /// Return whether any remaining cause reports resource exhaustion.
    pub fn is_resource_exhausted(mut self) -> bool {
        self.any(|classification| matches!(classification.class(), Class::ResourceExhaustion(_)))
    }

    fn has(mut self, class: Class) -> bool {
        self.any(|classification| classification.class() == class)
    }
}

impl<'a> Classification<'a> {
    /// Return the semantic class.
    pub fn class(&self) -> Class {
        self.class
    }

    /// Return the concrete error which established the classification.
    pub fn error(&self) -> &'a (dyn std::error::Error + 'static) {
        self.error
    }

    /// Return the original I/O error kind, if the underlying error is an [`std::io::Error`].
    pub fn io_kind(&self) -> Option<std::io::ErrorKind> {
        self.error.downcast_ref::<std::io::Error>().map(std::io::Error::kind)
    }
}

fn classify_one<'a>(error: &'a (dyn std::error::Error + 'static)) -> Option<Classification<'a>> {
    let class = if error.is::<crate::ValidationError>() {
        Class::Validation
    } else if error.is::<crate::CorruptionError>() {
        Class::Corruption
    } else if error.is::<crate::NotFoundError>() {
        Class::NotFound
    } else if error.is::<crate::RetryableError>() {
        Class::Retryable
    } else if let Some(error) = error.downcast_ref::<crate::ResourceExhaustionError>() {
        Class::ResourceExhaustion(error.kind())
    } else if error.is::<std::collections::TryReserveError>() {
        Class::ResourceExhaustion(crate::ResourceExhaustionKind::AllocationFailure)
    } else {
        let error = error.downcast_ref::<std::io::Error>()?;
        match error.kind() {
            std::io::ErrorKind::NotFound => Class::NotFound,
            std::io::ErrorKind::OutOfMemory => {
                Class::ResourceExhaustion(crate::ResourceExhaustionKind::AllocationFailure)
            }
            kind => Class::Io(kind),
        }
    };
    Some(Classification { class, error })
}

fn class_can_retry(class: Class) -> bool {
    matches!(
        class,
        Class::Retryable | Class::Io(std::io::ErrorKind::Interrupted | std::io::ErrorKind::TimedOut)
    )
}

fn classification_can_retry_lenient(classification: Classification<'_>) -> bool {
    class_can_retry(classification.class())
        || classification.io_kind().is_some_and(|kind| {
            use std::io::ErrorKind::*;
            matches!(
                kind,
                UnexpectedEof
                    | OutOfMemory
                    | BrokenPipe
                    | AddrInUse
                    | ConnectionAborted
                    | ConnectionReset
                    | ConnectionRefused
            )
        })
}

#[derive(Clone, Copy)]
enum Node<'a> {
    Frame(&'a crate::Frame),
    Source(&'a (dyn std::error::Error + 'static)),
    #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
    Chain {
        node: &'a crate::ChainedError,
        index: usize,
        cursor: Option<usize>,
    },
}

impl<'a> Node<'a> {
    fn display(self) -> DisplaySource<'a> {
        let (error, location) = match self {
            Node::Frame(frame) => (
                frame.error() as &(dyn std::error::Error + 'static),
                Some(frame.location()),
            ),
            Node::Source(error) => (error, None),
            #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
            Node::Chain { node, .. } => (
                node.err.error(),
                (!node.err.is_native_source()).then_some(node.location),
            ),
        };
        DisplaySource { error, location }
    }
}

struct Errors<'a> {
    root: Option<Node<'a>>,
    previous: Option<Node<'a>>,
    pending: std::collections::VecDeque<Node<'a>>,
    #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
    chains: Vec<(usize, Option<&'a crate::ChainedError>)>,
}

impl<'a> Errors<'a> {
    fn new(root: Node<'a>) -> Self {
        Errors {
            root: Some(root),
            previous: None,
            pending: Default::default(),
            #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
            chains: Vec::new(),
        }
    }

    fn source(&mut self, error: &'a (dyn std::error::Error + 'static)) {
        if let Some(error) = error.downcast_ref::<crate::Error>() {
            self.pending.push_back(error.iter_root());
        } else if let Some(source) = native_source(error) {
            self.pending.push_back(Node::Source(source));
        }
    }

    fn children(&mut self, node: Node<'a>) {
        match node {
            Node::Frame(frame) => {
                self.source(frame.error());
                self.pending.extend(frame.children().iter().map(Node::Frame));
            }
            Node::Source(error) => self.source(error),
            #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
            Node::Chain { node, index, cursor } => {
                if let Some(error) = node.err.error().downcast_ref::<crate::Error>() {
                    self.pending.push_back(error.iter_root());
                }
                let cursor = match cursor {
                    Some(cursor) => cursor,
                    None if node.source.is_none() => return,
                    None => {
                        self.chains.push((1, node.source.as_deref()));
                        self.chains.len() - 1
                    }
                };
                // Flattened parents occur in increasing order. One cursor per boundary streams each child once,
                // even when other error trees are interleaved at their logical breadth-first positions.
                let (child_index, next) = &mut self.chains[cursor];
                while let Some(child) = next.filter(|child| child.logical_parent == Some(index)) {
                    self.pending.push_back(Node::Chain {
                        node: child,
                        index: *child_index,
                        cursor: Some(cursor),
                    });
                    *child_index += 1;
                    *next = child.source.as_deref();
                }
            }
        }
    }
}

impl<'a> Iterator for Errors<'a> {
    type Item = DisplaySource<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // Defer expansion until the caller asks for another error, so a match need not inspect any of its causes.
        if let Some(previous) = self.previous.take() {
            self.children(previous);
        }
        let node = self.root.take().or_else(|| self.pending.pop_front())?;
        self.previous = Some(node);
        Some(node.display())
    }
}

impl crate::Frame {
    pub(crate) fn iter_errors_with_locations(&self) -> impl Iterator<Item = DisplaySource<'_>> + '_ {
        Errors::new(Node::Frame(self))
    }
}

#[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
mod _impl {
    use crate::{Error, Exn};
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

        pub(super) fn iter_root(&self) -> super::Node<'_> {
            super::Node::Frame(self.inner.frame())
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
                        .then(|| super::native_source(error))
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
    use crate::{Error, Exn};
    use std::fmt::Formatter;

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

        pub(super) fn iter_root(&self) -> super::Node<'_> {
            super::Node::Chain {
                node: &self.inner,
                index: 0,
                cursor: None,
            }
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
/// [`RetryableError`](crate::RetryableError), or is an [`std::io::Error`] whose kind is `Interrupted` or `TimedOut`.
///
/// Nested [`crate::Error`] values are inspected recursively. `false` only means that no known retryable error was found; it
/// does not guarantee that retrying cannot succeed.
pub fn can_retry(err: &(dyn std::error::Error + 'static)) -> bool {
    classify(err).can_retry()
}

/// Return `true` if `err` or any error in its [`source()`](std::error::Error::source) chain is explicitly marked with
/// [`RetryableError`](crate::RetryableError), or is an [`std::io::Error`] whose kind is `Interrupted`, `UnexpectedEof`,
/// `OutOfMemory`, `TimedOut`, `BrokenPipe`, `AddrInUse`, `ConnectionAborted`, `ConnectionReset`, or `ConnectionRefused`.
///
/// This applies a more lenient policy than [`can_retry`]. Nested [`crate::Error`] values are inspected recursively.
/// `false` only means that no known retryable error was found; it does not guarantee that retrying cannot succeed.
pub fn can_retry_lenient(err: &(dyn std::error::Error + 'static)) -> bool {
    classify(err).can_retry_lenient()
}

/// Retain I/O payloads, which `std::io::Error::source()` skips even when they carry a classification or an error tree.
pub(crate) fn native_source<'a>(
    err: &'a (dyn std::error::Error + 'static),
) -> Option<&'a (dyn std::error::Error + 'static)> {
    match err.downcast_ref::<std::io::Error>() {
        Some(err) => err.get_ref().map(|err| err as _),
        None => err.source(),
    }
}
