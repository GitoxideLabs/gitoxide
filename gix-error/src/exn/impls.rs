// Copyright 2025 FastLabs Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::VecDeque;
use std::error::Error;
use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;
use std::panic::Location;

use crate::concrete::chain::ErrorHandle;
use crate::{Metadata, types::ChainedError, write_location};

/// An exception type that can hold an [error tree](Exn::raise_all) and the call site.
///
/// While an error chain, a list, is automatically created when [raise](Exn::raise)
/// and friends are invoked, one can also use [`Exn::raise_all`] to create an error
/// that has multiple causes.
///
/// # Native error sources
///
/// Values reached through [`std::error::Error::source()`] remain owned by their original errors and are traversed by
/// reference, preserving their concrete types. They aren't exception frames and therefore have no captured call site of
/// their own.
///
/// In diagnostic reports, custom [`std::io::Error`] wrappers show their kind instead of repeating the payload's
/// diagnostic. The payload is reported separately as a cause; both remain available for inspection and classification.
///
/// # `Exn` == `Exn<Untyped>`
///
/// `Exn` act's like `Box<dyn std::error::Error + Send + Sync + 'static>`, but with the capability
/// to store a tree of errors along with their *call sites*.
///
/// # Visualisation
///
/// Linearized trees during display make a list of 3 children indistinguishable from
/// 3 errors where each is the child of the other.
///
/// ## Debug
///
/// * locations: ✔️
/// * error display: Display
/// * tree mode: linearized
///
/// ## Debug + Alternate
///
/// * locations: ❌
/// * error display: Display
/// * tree mode: linearized
///
/// ## Display
///
/// * locations: ❌
/// * error display: Debug
/// * tree mode: None
///
/// ## Display + Alternate
///
/// * locations: ❌
/// * error display: Debug
/// * tree mode: verbatim
pub struct Exn<E: std::error::Error + Send + Sync + 'static = Untyped> {
    // trade one more indirection for less stack size
    frame: Box<Frame>,
    phantom: PhantomData<E>,
}

/// Reuse an existing public error's tree only where its concrete wrapper type is no longer required.
#[track_caller]
#[expect(
    clippy::unnecessary_box_returns,
    reason = "erasure retains the existing frame allocation"
)]
pub(super) fn into_frame<E: Error + Send + Sync + 'static>(error: E) -> Box<Frame> {
    // Keep chains wrapped so adding context doesn't reconstruct their existing nodes.
    #[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
    let error = {
        // Downcast an Option on the stack so recognizing Error doesn't itself require another box.
        let mut error = Some(error);
        if let Some(error) = (&mut error as &mut dyn std::any::Any).downcast_mut::<Option<crate::Error>>() {
            return error.take().expect("the error has not been consumed").into_frame();
        }
        error.expect("a different error type was not taken")
    };
    Exn::new(error).frame
}

impl<E: Error + Send + Sync + 'static> From<E> for Exn<E> {
    #[track_caller]
    fn from(error: E) -> Self {
        Exn::new(error)
    }
}

impl<E: Error + Send + Sync + 'static> Exn<E> {
    /// Create a new exception with the given error.
    ///
    /// Its [source chain](Error::source) is retained by `error` and traversed lazily for formatting, downcasting, and
    /// conversion. Native sources are not copied into owned [`Frame`] values and keep their concrete types.
    ///
    /// See also [`ErrorExt::raise`](crate::ErrorExt) for a fluent way to convert an error into an `Exn` instance.
    #[track_caller]
    pub fn new(error: E) -> Self {
        let frame = Frame {
            error: Box::new(error),
            location: Location::caller(),
            children: Vec::new(),
        };

        Self {
            frame: Box::new(frame),
            phantom: PhantomData,
        }
    }

    #[track_caller]
    pub(super) fn with_cause(cause: impl Error + Send + Sync + 'static, error: E) -> Self {
        let cause = into_frame(cause);
        let mut exn = Exn::new(error);
        exn.frame.children.push(*cause);
        exn
    }

    /// Create a new exception with the given error and children.
    #[track_caller]
    pub fn raise_all<T, I>(children: I, err: E) -> Self
    where
        T: Error + Send + Sync + 'static,
        I: IntoIterator,
        I::Item: Into<Exn<T>>,
    {
        let mut new_exn = Exn::new(err);
        for exn in children {
            let exn = exn.into();
            new_exn.frame.children.push(*exn.frame);
        }
        new_exn
    }

    /// Raise a new exception; this will make the current exception a child of the new one.
    #[track_caller]
    pub fn raise<T: Error + Send + Sync + 'static>(self, err: T) -> Exn<T> {
        let mut new_exn = Exn::new(err);
        new_exn.frame.children.push(*self.frame);
        new_exn
    }

    /// Use the current exception as the head of a chain, adding `err` to its children.
    #[track_caller]
    pub fn chain<T: Error + Send + Sync + 'static>(mut self, err: impl Into<Exn<T>>) -> Exn<E> {
        let err = err.into();
        self.frame.children.push(*err.frame);
        self
    }

    /// Use the current exception the head of a chain, adding `errors` to its children.
    #[track_caller]
    pub fn chain_all<T, I>(mut self, errors: I) -> Exn<E>
    where
        T: Error + Send + Sync + 'static,
        I: IntoIterator,
        I::Item: Into<Exn<T>>,
    {
        for err in errors {
            let err = err.into();
            self.frame.children.push(*err.frame);
        }
        self
    }

    /// Drain all explicitly added child frames of this error as untyped [`Exn`].
    ///
    /// Native [`Error::source()`] values remain owned by their error and aren't drainable frames. This is useful if one
    /// wants to re-organise explicitly raised errors and the error layout is well known.
    pub fn drain_children(&mut self) -> impl Iterator<Item = Exn> + '_ {
        self.frame.children.drain(..).map(Exn::from)
    }

    /// Erase the type of this instance and turn it into a bare `Exn`.
    /// Reuse the frame allocation; already erased exceptions require no new allocation.
    pub fn erased(self) -> Exn {
        Exn::from_boxed_frame(self.frame)
    }

    /// Return the current exception.
    pub fn error(&self) -> &E {
        self.frame
            .error
            .downcast_ref()
            .expect("the owned frame always matches the compile-time error type")
    }

    /// Discard all error context and return the underlying error in a Box.
    ///
    /// This is useful to retain the allocation, as internally it's also stored in a box,
    /// when comparing it to [`Self::into_inner()`].
    pub fn into_box(self) -> Box<E> {
        match self.frame.error.downcast() {
            Ok(err) => err,
            Err(_) => unreachable!("The type in the frame is always the type of this instance"),
        }
    }

    /// Discard all error context and return the underlying error.
    ///
    /// This may be needed to obtain something that once again implements `Error`.
    /// Note that this destroys the internal Box and moves the value back onto the stack.
    pub fn into_inner(self) -> E {
        *self.into_box()
    }

    /// Turn ourselves into a top-level [Error] that implements [`std::error::Error`].
    ///
    /// [Error]: crate::Error
    pub fn into_error(self) -> crate::Error {
        self.into()
    }

    /// Convert this error tree into a chain of errors, breadth first, which flattens the tree
    /// but retains all type dynamic type information.
    ///
    /// This is useful for inter-op with `anyhow`.
    pub fn into_chain(self) -> ChainedError {
        self.into()
    }

    /// Return the underlying exception frame.
    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    /// Iterate over all explicitly created frames in breadth-first order. The first frame is this instance, followed by
    /// all explicitly raised children. Native [`Error::source()`] values are not frames.
    pub fn iter(&self) -> impl Iterator<Item = &Frame> {
        self.frame().iter_frames()
    }

    /// Lazily visit stored errors and native sources in logical breadth-first order, expanding nested [`crate::Error`] values.
    /// [Classification-only markers](crate::ClassificationMarker) are skipped; other concrete types remain available for downcasting,
    /// as with [`crate::Error::iter_errors()`].
    pub fn iter_errors(&self) -> impl Iterator<Item = &(dyn Error + 'static)> + '_ {
        self.frame.iter_errors_with_locations().map(|source| source.error())
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

    /// Return the error that is most likely the root cause, based on [`Frame::probable_cause()`].
    ///
    /// Return the stored error if there is no unique causal child. Nested [`crate::Error`] graphs participate alongside
    /// native sources and explicit children, matching [`crate::Error::probable_cause()`] without consuming this exception.
    pub fn probable_cause(&self) -> &(dyn Error + 'static) {
        self.frame.probable_cause().unwrap_or_else(|| self.frame.error())
    }

    /// Find the first diagnostic error that downcasts to `T` in logical breadth-first order.
    /// Classification-only markers are omitted, as in [`Self::iter_errors()`].
    ///
    /// Nested [`crate::Error`] values are inspected recursively, matching [`crate::Error::downcast_any_ref()`].
    pub fn downcast_any_ref<T: Error + 'static>(&self) -> Option<&T> {
        self.iter_errors().find_map(|error| error.downcast_ref())
    }
}

impl<E> Deref for Exn<E>
where
    E: Error + Send + Sync + 'static,
{
    type Target = E;

    fn deref(&self) -> &Self::Target {
        self.error()
    }
}

impl<E: Error + Send + Sync + 'static> fmt::Debug for Exn<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_frame_recursive(f, self.frame(), "", ErrorMode::Display, TreeMode::Linearize)
    }
}

impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_frame_recursive(f, self, "", ErrorMode::Display, TreeMode::Linearize)
    }
}

#[derive(Copy, Clone)]
pub(crate) enum ErrorMode {
    Display,
    Debug,
}

impl ErrorMode {
    pub(crate) fn fmt(self, error: &(dyn Error + 'static), f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(io) = error.downcast_ref::<std::io::Error>()
            && io.get_ref().is_some()
        {
            // The traversal reports the payload separately, so neither Display nor Debug may expand it here.
            return write!(f, "I/O error ({:?})", io.kind());
        }
        // The outer alternate flag controls report layout, not the formatting of individual diagnostics.
        match self {
            ErrorMode::Display => write!(f, "{error}"),
            ErrorMode::Debug => write!(f, "{error:?}"),
        }
    }
}

#[derive(Copy, Clone)]
enum TreeMode {
    Linearize,
    Verbatim,
}

fn write_frame_recursive(
    f: &mut fmt::Formatter<'_>,
    frame: &Frame,
    prefix: &str,
    err_mode: ErrorMode,
    tree_mode: TreeMode,
) -> fmt::Result {
    if crate::error::is_transparent_marker(frame.error()) {
        let children = ErrorNode::Frame(frame).children();
        if !children.is_empty() {
            for (index, child) in children.into_iter().enumerate() {
                if index != 0 {
                    writeln!(f)?;
                }
                write_error_node_recursive(f, child, prefix, err_mode, tree_mode)?;
            }
            return Ok(());
        }
    }
    write_error_node_recursive(f, ErrorNode::Frame(frame), prefix, err_mode, tree_mode)
}

fn write_error_node_recursive(
    f: &mut fmt::Formatter<'_>,
    node: ErrorNode<'_>,
    prefix: &str,
    err_mode: ErrorMode,
    tree_mode: TreeMode,
) -> fmt::Result {
    let mut root_error = node.error();
    while let Some(error) = root_error.downcast_ref::<crate::Error>() {
        root_error = error.error();
    }
    err_mode.fmt(root_error, f)?;
    if !f.alternate() {
        write_location(f, node.location())?;
    }

    if let Some(err) = node.error().downcast_ref::<crate::Error>() {
        let mut skipped_root = false;
        for source in err
            .iter_errors_with_locations()
            .filter(|source| !source.error().is::<crate::Error>())
        {
            // Nested boundaries can have children before the innermost root in breadth-first order.
            if !skipped_root && std::ptr::eq(source.error(), root_error) {
                skipped_root = true;
                continue;
            }
            write!(f, "\n{prefix}|\n{prefix}└─ ")?;
            err_mode.fmt(source.error(), f)?;
            if !f.alternate() {
                write_location(f, source.location().unwrap_or_else(|| node.location()))?;
            }
        }
    }

    let children = node.children();
    let children_len = children.len();

    for (child_index, child) in children.into_iter().enumerate() {
        write!(f, "\n{prefix}|")?;
        write!(f, "\n{prefix}└─ ")?;

        let child_child_len = if child
            .error()
            .downcast_ref::<crate::Error>()
            .is_some_and(|err| err.iter_errors().filter(|source| !source.is::<crate::Error>()).count() > 1)
        {
            1
        } else {
            child.children().len()
        };
        let may_linearize_chain = matches!(tree_mode, TreeMode::Linearize) && children_len == 1 && child_child_len == 1;
        if may_linearize_chain {
            write_error_node_recursive(f, child, prefix, err_mode, tree_mode)?;
        } else if child_index < children_len - 1 {
            write_error_node_recursive(f, child, &format!("{prefix}|   "), err_mode, tree_mode)?;
        } else {
            write_error_node_recursive(f, child, &format!("{prefix}    "), err_mode, tree_mode)?;
        }
    }

    Ok(())
}

impl<E: Error + Send + Sync + 'static> fmt::Display for Exn<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.frame, f)
    }
}

impl<E: Error + Send + Sync + 'static> PartialEq<str> for Exn<E> {
    fn eq(&self, other: &str) -> bool {
        crate::root_error_eq(self.frame().error(), other)
    }
}

impl<E: Error + Send + Sync + 'static> PartialEq<&str> for Exn<E> {
    fn eq(&self, other: &&str) -> bool {
        <Self as PartialEq<str>>::eq(self, other)
    }
}

impl<E: Error + Send + Sync + 'static> PartialEq<String> for Exn<E> {
    fn eq(&self, other: &String) -> bool {
        <Self as PartialEq<str>>::eq(self, other)
    }
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            // Avoid printing alternate versions of the debug info, keep it in one line, also print the tree.
            write_frame_recursive(f, self, "", ErrorMode::Debug, TreeMode::Verbatim)
        } else {
            if crate::error::is_transparent_marker(self.error())
                && let Some(diagnostic) = self.iter_errors_with_locations().next()
            {
                return fmt::Display::fmt(diagnostic.error(), f);
            }
            fmt::Display::fmt(self.error(), f)
        }
    }
}

/// A frame in the exception tree.
pub struct Frame {
    /// The error that occurred at this frame.
    error: Box<dyn Error + Send + Sync + 'static>,
    /// The source code location where this exception frame was created.
    location: &'static Location<'static>,
    /// Explicitly raised child exception frames.
    children: Vec<Frame>,
}

impl Frame {
    /// Return the error as a reference to [`Error`].
    ///
    /// If the error was [erased](crate::Exn::erased), this is the original error,
    /// so it can still be downcast to its actual type.
    pub fn error(&self) -> &(dyn Error + Send + Sync + 'static) {
        let mut error = &*self.error;
        loop {
            if let Some(erased) = error.downcast_ref::<Untyped>() {
                error = &*erased.0;
            } else if let Some(shared) = error.downcast_ref::<ErrorHandle>() {
                error = shared.owned_error();
            } else {
                return error;
            }
        }
    }

    /// Return the source code location where this exception frame was created.
    pub fn location(&self) -> &'static Location<'static> {
        self.location
    }

    /// Return explicitly raised child frames.
    ///
    /// Native [`Error::source()`] values are borrowed from [`Self::error()`] and traversed lazily, so they aren't owned
    /// `Frame` children.
    pub fn children(&self) -> &[Frame] {
        &self.children
    }
}

/// A borrowed node that lets one traversal visit both explicit exception frames and native [`Error::source()`] chains.
///
/// Explicitly raised errors are stored as [`Frame`] values, whereas native sources remain owned by their errors and
/// must be borrowed when traversed. `Source` represents such a borrowed native error and carries forward the location
/// of its owning frame for internal formatting without turning the source into a frame or losing its concrete type.
#[derive(Clone, Copy)]
pub(crate) enum ErrorNode<'a> {
    Frame(&'a Frame),
    Source {
        error: &'a (dyn Error + 'static),
        location: &'static Location<'static>,
    },
    /// A source from a nested boundary's already flattened iterator; its descendants are emitted separately.
    FlatSource {
        error: &'a (dyn Error + 'static),
        location: &'static Location<'static>,
    },
}

impl<'a> ErrorNode<'a> {
    pub(crate) fn error(self) -> &'a (dyn Error + 'static) {
        match self {
            ErrorNode::Frame(frame) => frame.error(),
            ErrorNode::Source { error, .. } | ErrorNode::FlatSource { error, .. } => error,
        }
    }

    /// Return the frame location used when formatting this node.
    ///
    /// A frame returns its own captured location. A native source inherits the location of the frame whose error owns its
    /// source chain, providing formatting context even though no location was captured for the source itself.
    pub(crate) fn location(self) -> &'static Location<'static> {
        match self {
            ErrorNode::Frame(frame) => frame.location,
            ErrorNode::Source { location, .. } | ErrorNode::FlatSource { location, .. } => location,
        }
    }

    /// Return this node's diagnostic children in traversal order, promoting descendants of classification markers.
    ///
    /// A direct native [`Error::source()`] or I/O payload is first and inherits this node's formatting location.
    /// For a frame, explicitly raised child frames follow it in insertion order. The compatibility `source()` of a nested [`crate::Error`] is
    /// skipped because that wrapper retains an internal error graph which its own traversal APIs expand separately;
    /// following the compatibility source here would expose only one path and duplicate that expansion.
    pub(crate) fn children(self) -> Vec<ErrorNode<'a>> {
        if matches!(self, ErrorNode::FlatSource { .. }) {
            return Vec::new();
        }
        let error = self.error();
        let location = self.location();
        let mut children = Vec::new();
        if let Some(nested) = error.downcast_ref::<crate::Error>() {
            if crate::error::is_transparent_marker(error) {
                children.extend(
                    nested
                        .iter_errors_with_locations()
                        .filter(|source| !source.error().is::<crate::Error>())
                        .map(|source| ErrorNode::FlatSource {
                            error: source.error(),
                            location: source.location().unwrap_or(location),
                        }),
                );
            }
        } else if let Some(error) = crate::error::native_source(error) {
            children.push(ErrorNode::Source { error, location });
        }
        if let ErrorNode::Frame(frame) = self {
            children.extend(frame.children.iter().map(ErrorNode::Frame));
        }
        let mut diagnostics = Vec::new();
        for child in children {
            if crate::error::is_transparent_marker(child.error()) {
                diagnostics.extend(child.children());
            } else {
                diagnostics.push(child);
            }
        }
        diagnostics
    }
}

/// Navigation
impl Frame {
    /// Follow the unique causal child until reaching a leaf or a branch.
    ///
    /// Native [`Error::source()`] values, I/O payloads, nested [`crate::Error`] graphs, and explicitly raised frames all
    /// participate.
    ///
    /// An *aggregate* is the error at a branch that groups two or more diagnostic causes. This is a role in the error
    /// tree, not a special concrete error type. For example, [`Exn::raise_all`] can attach multiple failed operations
    /// to a shared `"batch failed"` [`crate::Message`]. That shared message is the aggregate, so selection stops there
    /// rather than arbitrarily choosing one operation's error:
    ///
    /// ```text
    /// outer context
    /// └─ batch failed  (aggregate, selected)
    ///    ├─ first operation failed
    ///    └─ second operation failed
    /// ```
    ///
    /// All [`crate::ClassificationMarker`] values are ignored. Their frames, including nested boundaries,
    /// are transparent: their real descendants count as children of the nearest non-marker parent instead.
    /// Return `None` if selection stays at this frame, allowing callers to fall back to [`Self::error()`], even for a
    /// classification-only root.
    pub fn probable_cause(&self) -> Option<&(dyn Error + 'static)> {
        self.probable_cause_inner()
    }

    /// Iterate over all explicitly created frames in breadth-first order. The first frame is this instance, followed by
    /// all explicitly raised children. Native [`Error::source()`] values are not frames.
    pub fn iter_frames(&self) -> impl Iterator<Item = &Frame> + '_ {
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(self);
        BreadthFirstFrames { queue }
    }
}

/// Breadth-first iterator over explicitly created `Frame`s.
pub struct BreadthFirstFrames<'a> {
    queue: std::collections::VecDeque<&'a Frame>,
}

impl<'a> Iterator for BreadthFirstFrames<'a> {
    type Item = &'a Frame;

    fn next(&mut self) -> Option<Self::Item> {
        let frame = self.queue.pop_front()?;
        for child in frame.children() {
            self.queue.push_back(child);
        }
        Some(frame)
    }
}

impl<E> From<Exn<E>> for Box<Frame>
where
    E: Error + Send + Sync + 'static,
{
    fn from(err: Exn<E>) -> Self {
        err.frame
    }
}

impl<E> From<Exn<E>> for Box<dyn Error + Send + Sync + 'static>
where
    E: Error + Send + Sync + 'static,
{
    fn from(err: Exn<E>) -> Self {
        Box::new(err.into_error())
    }
}

#[cfg(feature = "anyhow")]
impl<E> From<Exn<E>> for anyhow::Error
where
    E: Error + Send + Sync + 'static,
{
    fn from(err: Exn<E>) -> Self {
        anyhow::Error::from(err.into_chain())
    }
}

impl<E> From<Exn<E>> for Frame
where
    E: Error + Send + Sync + 'static,
{
    fn from(err: Exn<E>) -> Self {
        *err.frame
    }
}

impl From<Frame> for Exn {
    fn from(frame: Frame) -> Self {
        Exn::from_boxed_frame(Box::new(frame))
    }
}

impl Exn {
    pub(crate) fn from_boxed_frame(mut frame: Box<Frame>) -> Self {
        if !frame.error.is::<Untyped>() {
            frame.error = Box::new(Untyped(frame.error));
        }
        Exn {
            frame,
            phantom: Default::default(),
        }
    }
}

#[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
impl Exn {
    pub(crate) fn from_chain(chain: ChainedError) -> Self {
        let mut frames = Vec::new();
        let mut next = Some(chain);
        while let Some(node) = next {
            next = node.source.map(|source| *source);
            let frame = (!node.err.is_native_source()).then(|| Frame {
                error: node.err.into_owned_error(),
                location: node.location,
                children: Vec::new(),
            });
            frames.push((frame, node.logical_parent));
        }
        // Native sources remain owned by their explicit frame; only those frames are rebuilt.
        while let Some((frame, parent)) = frames.pop() {
            let Some(mut frame) = frame else { continue };
            frame.children.reverse();
            match parent {
                Some(parent) => frames[parent]
                    .0
                    .as_mut()
                    .expect("an explicit frame has an explicit parent")
                    .children
                    .push(frame),
                None => return frame.into(),
            }
        }
        unreachable!("an error chain always contains its root frame")
    }
}

/// A marker to show that type information is not available,
/// while storing all extractable information about the erased type.
/// It's the default type for [Exn].
pub struct Untyped(Box<dyn Error + Send + Sync + 'static>);

impl Untyped {
    pub(crate) fn from_boxed(error: Box<dyn Error + Send + Sync + 'static>) -> Self {
        Untyped(error)
    }
}

impl fmt::Display for Untyped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for Untyped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl Error for Untyped {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.source()
    }
}

impl<E> From<Exn<E>> for ChainedError
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(err: Exn<E>) -> Self {
        let flattened = flatten_error_nodes(*err.frame);
        let mut source = None;
        for node in flattened.into_iter().rev() {
            source = Some(Box::new(ChainedError {
                err: node.error,
                location: node.location,
                logical_parent: node.logical_parent,
                source,
            }));
        }
        *source.expect("an Exn always contains its root error")
    }
}

struct OwnedErrorNode {
    error: ErrorHandle,
    location: &'static Location<'static>,
    logical_parent: Option<usize>,
}

/// Consume an exception-frame tree and flatten its errors into logical breadth-first order for [`ChainedError`].
///
/// Each frame's direct native [`Error::source()`] is queued before its explicitly raised child frames, and subsequent
/// native sources continue as children of the preceding source. Every output node retains an owning [`ErrorHandle`], the
/// frame location used for formatting, and the output index of its logical parent so the tree relationships can later be
/// reconstructed. Native sources inherit their owning frame's location.
///
/// A nested [`crate::Error`] is retained as one node without following its compatibility `source()` chain. Its internal
/// graph is expanded separately by the [`crate::Error`] traversal APIs, avoiding a partial and duplicated representation.
fn flatten_error_nodes(root: Frame) -> Vec<OwnedErrorNode> {
    enum Pending {
        Frame {
            frame: Frame,
            logical_parent: Option<usize>,
        },
        Source {
            error: ErrorHandle,
            location: &'static Location<'static>,
            logical_parent: usize,
        },
    }

    let mut queue = VecDeque::from([Pending::Frame {
        frame: root,
        logical_parent: None,
    }]);
    let mut out = Vec::new();
    while let Some(node) = queue.pop_front() {
        let node_index = out.len();
        match node {
            Pending::Frame {
                frame:
                    Frame {
                        error,
                        location,
                        children,
                    },
                logical_parent,
            } => {
                let error = ErrorHandle::new(unerase(error));
                if !error.error().is::<crate::Error>()
                    && let Some(source) = error.source()
                {
                    queue.push_back(Pending::Source {
                        error: source,
                        location,
                        logical_parent: node_index,
                    });
                }
                queue.extend(children.into_iter().map(|frame| Pending::Frame {
                    frame,
                    logical_parent: Some(node_index),
                }));
                out.push(OwnedErrorNode {
                    error,
                    location,
                    logical_parent,
                });
            }
            Pending::Source {
                error,
                location,
                logical_parent,
            } => {
                if !error.error().is::<crate::Error>()
                    && let Some(source) = error.source()
                {
                    queue.push_back(Pending::Source {
                        error: source,
                        location,
                        logical_parent: node_index,
                    });
                }
                out.push(OwnedErrorNode {
                    error,
                    location,
                    logical_parent: Some(logical_parent),
                });
            }
        }
    }
    out
}

/// Remove all type-erasure markers before storing an error in a [`ChainedError`].
///
/// [`Untyped::source()`] deliberately forwards to the wrapped error's source to keep
/// the marker transparent. Storing the marker itself in the chain would therefore
/// hide a wrapped leaf error from source traversal and classification. Unwrapping it
/// here retains the original runtime type without changing those source semantics.
fn unerase(mut error: Box<dyn Error + Send + Sync + 'static>) -> Box<dyn Error + Send + Sync + 'static> {
    loop {
        match error.downcast::<Untyped>() {
            Ok(untyped) => error = untyped.0,
            Err(typed) => return typed,
        }
    }
}
