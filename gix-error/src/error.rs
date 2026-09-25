use crate::Metadata;

// Keep inherent methods on Error and Exn while sharing their implementation and documentation.
macro_rules! classification_predicates {
    () => {
        /// Return `true` if any stored error or native source has an explicit [`crate::Class::Retryable`] classification.
        ///
        /// [`crate::Message`] and [`crate::ClassificationMarker`] can supply this classification.
        /// Nested [`crate::Error`] values are inspected recursively. Unlike [`Self::can_retry()`], this does not infer
        /// retryability from I/O error kinds.
        pub fn is_retryable(&self) -> bool {
            self.classify().is_retryable()
        }

        /// Return `true` if any stored error or native source reports resource exhaustion.
        ///
        /// This recognizes messages or markers with
        /// [`crate::Class::ResourceExhaustion`], [`std::collections::TryReserveError`], and
        /// [`std::io::ErrorKind::OutOfMemory`], including within nested
        /// [`crate::Error`] values.
        pub fn is_resource_exhausted(&self) -> bool {
            self.classify().is_resource_exhausted()
        }

        /// Return `true` if any stored error, or an error in its [`source()`](std::error::Error::source) chain, is:
        ///
        /// * classified as [`crate::Class::Retryable`], or
        /// * classified as [`crate::Class::Io`] with kind `Interrupted` or `TimedOut`.
        ///
        /// Nested [`crate::Error`] values are inspected recursively. `false` only means that no known retryable error was
        /// found; it does not guarantee that retrying cannot succeed.
        pub fn can_retry(&self) -> bool {
            self.classify().can_retry()
        }

        /// Apply [`Self::can_retry()`], also accepting [`std::io::Error`] with kind `UnexpectedEof`, `OutOfMemory`,
        /// `BrokenPipe`, `AddrInUse`, `ConnectionAborted`, `ConnectionReset`, or `ConnectionRefused`.
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
/// Errors owned by a [`crate::exn::Frame`] have the location captured when that frame was created. The first real source
/// beneath transparent classification markers inherits their frame's location. Other native
/// [`std::error::Error::source()`] values have no location because no caller location was captured for them.
///
/// Unlike [`crate::exn::Frame`], this type neither owns the error nor represents relationships in an error tree. This lets
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

    /// Return the captured or inherited caller location, or `None` for an ordinary native error source.
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
    /// The stored error is first unless it is a classification marker. A frame's native source precedes its explicitly
    /// raised children. Concrete error types remain available for downcasting, except for classification markers,
    /// which are always transparent to traversal.
    /// Use [`Self::classify()`] to inspect classifications.
    pub fn iter_errors(&self) -> impl Iterator<Item = &(dyn std::error::Error + 'static)> + '_ {
        self.iter_errors_with_locations().map(|source| source.error)
    }

    /// Visit the same errors as [`Self::iter_errors()`], with caller locations for explicitly raised frames.
    /// The first real source beneath transparent classification markers inherits their frame's location; other native
    /// sources have no caller location of their own. [`DisplaySource`] can render either representation.
    pub fn iter_errors_with_locations(&self) -> impl Iterator<Item = DisplaySource<'_>> + '_ {
        Errors::new(self.iter_root())
            .map(Node::display)
            .filter(|source| !is_transparent_marker(source.error))
    }

    /// Find the first diagnostic error that downcasts to `T` in logical breadth-first order.
    /// Classification markers are omitted, as in [`Self::iter_errors()`].
    pub fn downcast_any_ref<T: std::error::Error + 'static>(&self) -> Option<&T> {
        self.iter_errors().find_map(|error| error.downcast_ref())
    }

    /// Follow the unique causal path to a leaf or aggregate, as in [`crate::exn::Frame::probable_cause()`].
    ///
    /// Classification markers are always transparent to selection. Nested error graphs and explicitly raised children
    /// both participate, so a selected boundary at a branch is not replaced by one of its nested causes.
    /// If selection stays at the root, return the stored error, including a classification-only root.
    pub fn probable_cause(&self) -> &(dyn std::error::Error + 'static) {
        self.iter_root().probable_cause().unwrap_or_else(|| self.error())
    }

    /// Visit the non-empty [`Metadata`] dictionaries of [`crate::Message`] contexts in error traversal order.
    /// Dictionaries remain separate. Functions returning metadata document the keys in each context.
    ///
    /// To match a class and values on the same message, use [`Self::classify()`] and
    /// [`Classification::error()`](crate::types::Classification::error) instead of combining independent classification
    /// and metadata searches.
    pub fn metadata(&self) -> impl Iterator<Item = &Metadata> + '_ {
        self.iter_errors()
            .filter_map(|error| error.downcast_ref::<crate::Message>())
            .map(|error| &error.values)
            .filter(|values| !values.is_empty())
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
/// let error = std::io::Error::other(gix_error::not_found("missing object"));
/// assert!(gix_error::classify(&error).is_not_found());
/// ```
pub fn classify<'a>(err: &'a (dyn std::error::Error + 'static)) -> Classifications<'a> {
    Classifications(Errors::new(err.downcast_ref::<crate::Error>().map_or(
        Node::Source {
            error: err,
            location: None,
            source_owner: None,
        },
        crate::Error::iter_root,
    )))
}

/// A lazy iterator over classified causes. Its predicates consume the remaining iterator and stop at the first match.
pub struct Classifications<'a>(Errors<'a>);

impl<'a> Iterator for Classifications<'a> {
    type Item = Classification<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.find_map(classify_one)
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

    /// Return whether any remaining cause has exactly `class`.
    pub fn has(mut self, class: Class) -> bool {
        self.any(|classification| classification.class() == class)
    }
}

impl<'a> Classification<'a> {
    /// Return the semantic class.
    pub fn class(&self) -> Class {
        self.class
    }

    /// Return the concrete error which established the classification.
    ///
    /// A source-bearing [`crate::ClassificationMarker`] identifies its wrapped error. A class-only marker
    /// supplied through a native [`source()`](std::error::Error::source) identifies the error that owns it.
    /// A standalone marker without an identifiable subject retains the marker itself as a fallback.
    pub fn error(&self) -> &'a (dyn std::error::Error + 'static) {
        self.error
    }

    /// Return the original I/O error kind, if the underlying error is an [`std::io::Error`].
    pub fn io_kind(&self) -> Option<std::io::ErrorKind> {
        self.error.downcast_ref::<std::io::Error>().map(std::io::Error::kind)
    }
}

fn classify_one(node: Node<'_>) -> Option<Classification<'_>> {
    let mut error = node.display().error;
    let class = if let Some(marker) = error.downcast_ref::<crate::ClassificationMarker>() {
        let source_owner = match node {
            Node::Frame(_) => None,
            Node::Source { source_owner, .. } => source_owner,
            #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
            Node::Chain { source_owner, .. } => source_owner,
        };
        error = std::error::Error::source(marker).or(source_owner).unwrap_or(error);
        marker.class()
    } else if let Some(error) = error.downcast_ref::<crate::Message>() {
        error.class?
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
    Frame(&'a crate::exn::Frame),
    Source {
        error: &'a (dyn std::error::Error + 'static),
        location: Option<&'static std::panic::Location<'static>>,
        source_owner: Option<&'a (dyn std::error::Error + 'static)>,
    },
    #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
    Chain {
        node: &'a crate::types::ChainedError,
        index: usize,
        cursor: Option<usize>,
        source_owner: Option<&'a (dyn std::error::Error + 'static)>,
    },
}

impl<'a> Node<'a> {
    fn display(self) -> DisplaySource<'a> {
        let (error, location) = match self {
            Node::Frame(frame) => (
                frame.error() as &(dyn std::error::Error + 'static),
                Some(frame.location()),
            ),
            Node::Source { error, location, .. } => (error, location),
            #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
            Node::Chain { node, .. } => (node.err.error(), node.err.has_frame_location().then_some(node.location)),
        };
        DisplaySource { error, location }
    }

    fn children(self) -> std::collections::VecDeque<Node<'a>> {
        // Cause selection follows a path rather than breadth-first order, so each query needs its own chain cursor.
        #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
        let root = match self {
            Node::Chain {
                node,
                index,
                source_owner,
                ..
            } => Node::Chain {
                node,
                index,
                cursor: None,
                source_owner,
            },
            root => root,
        };
        #[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
        let root = self;
        let mut traversal = Errors::new(root);
        traversal.children(root);
        traversal.pending
    }

    fn probable_cause(self) -> Option<&'a (dyn std::error::Error + 'static)> {
        let mut node = self;
        // Track traversal, not error addresses: a native source can share its owner's address.
        let mut cause = None;
        loop {
            let mut pending = node.children();
            let mut only_child = None;
            while let Some(child) = pending.pop_front() {
                if is_transparent_marker(child.display().error) {
                    // Marker frames (including nested boundaries storing markers) are transparent, not dead ends.
                    pending.extend(child.children());
                } else if only_child.replace(child).is_some() {
                    return cause;
                }
            }
            node = match only_child {
                Some(child) => child,
                None => return cause,
            };
            cause = Some(node.display().error);
        }
    }
}

struct Errors<'a> {
    root: Option<Node<'a>>,
    previous: Option<Node<'a>>,
    pending: std::collections::VecDeque<Node<'a>>,
    #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
    chains: Vec<(usize, Option<&'a crate::types::ChainedError>)>,
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

    fn source(
        &mut self,
        error: &'a (dyn std::error::Error + 'static),
        location: Option<&'static std::panic::Location<'static>>,
    ) {
        if let Some(error) = error.downcast_ref::<crate::Error>() {
            self.pending.push_back(error.iter_root());
        } else if let Some(source) = native_source(error) {
            self.pending.push_back(Node::Source {
                error: source,
                location: location.filter(|_| is_transparent_marker(error)),
                source_owner: Some(error),
            });
        }
    }

    fn children(&mut self, node: Node<'a>) {
        match node {
            Node::Frame(frame) => {
                self.source(frame.error(), Some(frame.location()));
                self.pending.extend(frame.children().iter().map(Node::Frame));
            }
            Node::Source { error, location, .. } => self.source(error, location),
            #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
            Node::Chain {
                node, index, cursor, ..
            } => {
                if let Some(error) = node.err.error().downcast_ref::<crate::Error>() {
                    self.pending.push_back(error.iter_root());
                }
                let cursor = match cursor {
                    Some(cursor) => cursor,
                    None if node.source.is_none() => return,
                    None => {
                        self.chains.push((index + 1, node.source.as_deref()));
                        self.chains.len() - 1
                    }
                };
                // Flattened parents occur in increasing order. One cursor per boundary streams each child once,
                // even when other error trees are interleaved at their logical breadth-first positions.
                let (child_index, next) = &mut self.chains[cursor];
                // A fresh cursor for cause selection can start among siblings belonging to earlier parents.
                while let Some(child) = next.filter(|child| child.logical_parent.is_some_and(|parent| parent < index)) {
                    *child_index += 1;
                    *next = child.source.as_deref();
                }
                while let Some(child) = next.filter(|child| child.logical_parent == Some(index)) {
                    self.pending.push_back(Node::Chain {
                        node: child,
                        index: *child_index,
                        cursor: Some(cursor),
                        source_owner: child.err.is_native_source().then(|| node.err.error()),
                    });
                    *child_index += 1;
                    *next = child.source.as_deref();
                }
            }
        }
    }
}

impl<'a> Iterator for Errors<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // Defer expansion until the caller asks for another error, so a match need not inspect any of its causes.
        if let Some(previous) = self.previous.take() {
            self.children(previous);
        }
        let node = self.root.take().or_else(|| self.pending.pop_front())?;
        self.previous = Some(node);
        Some(node)
    }
}

impl crate::exn::Frame {
    pub(crate) fn probable_cause_inner(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Node::Frame(self).probable_cause()
    }

    pub(crate) fn iter_errors_with_locations(&self) -> impl Iterator<Item = DisplaySource<'_>> + '_ {
        Errors::new(Node::Frame(self))
            .map(Node::display)
            .filter(|source| !is_transparent_marker(source.error))
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
        /// This can be a classification marker hidden from [`Self::iter_errors()`], and is distinct from
        /// [`Self::probable_cause()`].
        pub fn error(&self) -> &(dyn std::error::Error + 'static) {
            self.inner.frame().error()
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
            Self::from_error(crate::exn::Untyped::from_boxed(error))
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
        /// This can be a classification marker hidden from [`Self::iter_errors()`], and is distinct from
        /// [`Self::probable_cause()`].
        pub fn error(&self) -> &(dyn std::error::Error + 'static) {
            self.inner.err.error()
        }

        pub(super) fn iter_root(&self) -> super::Node<'_> {
            super::Node::Chain {
                node: &self.inner,
                index: 0,
                cursor: None,
                source_owner: None,
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
            Self::from_error(crate::exn::Untyped::from_boxed(error))
        }
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            if super::is_transparent_marker(self.error())
                && let Some(diagnostic) = self.iter_errors_with_locations().next()
            {
                return std::fmt::Display::fmt(&diagnostic, f);
            }
            std::fmt::Display::fmt(&self.inner, f)
        }
    }

    impl std::fmt::Debug for Error {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            if super::is_transparent_marker(self.error())
                && let Some(diagnostic) = self.iter_errors().next()
            {
                return std::fmt::Debug::fmt(diagnostic, f);
            }
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

/// Retain I/O payloads, which `std::io::Error::source()` skips even when they carry a classification or an error tree.
pub(crate) fn native_source<'a>(
    err: &'a (dyn std::error::Error + 'static),
) -> Option<&'a (dyn std::error::Error + 'static)> {
    match err.downcast_ref::<std::io::Error>() {
        Some(err) => err.get_ref().map(|err| err as _),
        None => err.source(),
    }
}

pub(crate) fn is_transparent_marker(mut error: &(dyn std::error::Error + 'static)) -> bool {
    while let Some(nested) = error.downcast_ref::<crate::Error>() {
        error = nested.error();
    }
    error.is::<crate::ClassificationMarker>()
}
