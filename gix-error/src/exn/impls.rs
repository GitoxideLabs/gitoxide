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
use crate::{ChainedError, Exn, write_location};

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
    pub fn erased(self) -> Exn {
        let untyped_frame = {
            let Frame {
                error,
                location,
                children,
            } = *self.frame;
            // Unfortunately, we have to double-box here.
            // TODO: figure out tricks to make this unnecessary.
            let error = Untyped(error);
            Frame {
                error: Box::new(error),
                location,
                children,
            }
        };
        Exn {
            frame: Box::new(untyped_frame),
            phantom: Default::default(),
        }
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
    pub fn into_chain(self) -> crate::ChainedError {
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

    /// Find the first stored error or native source that downcasts to `T` in breadth-first order.
    pub fn downcast_any_ref<T: Error + 'static>(&self) -> Option<&T> {
        self.frame
            .iter_error_nodes()
            .find_map(|node| node.error().downcast_ref())
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
enum ErrorMode {
    Display,
    Debug,
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
    write_error_node_recursive(f, ErrorNode::Frame(frame), prefix, err_mode, tree_mode)
}

fn write_error_node_recursive(
    f: &mut fmt::Formatter<'_>,
    node: ErrorNode<'_>,
    prefix: &str,
    err_mode: ErrorMode,
    tree_mode: TreeMode,
) -> fmt::Result {
    match err_mode {
        ErrorMode::Display => fmt::Display::fmt(node.error(), f),
        ErrorMode::Debug => write!(f, "{:?}", node.error()),
    }?;
    if !f.alternate() {
        write_location(f, node.location())?;
    }

    if let Some(err) = node.error().downcast_ref::<crate::Error>() {
        for source in err.iter_errors().filter(|source| !source.is::<crate::Error>()).skip(1) {
            write!(f, "\n{prefix}|\n{prefix}└─ {source}")?;
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
        while let Some(erased) = error.downcast_ref::<Untyped>() {
            error = &*erased.0;
        }
        error
    }

    /// Return the source code location where this exception frame was created.
    /// Return the frame location used when formatting this node.
    ///
    /// A frame returns its own captured location. A native source inherits the location of the frame whose error owns its
    /// source chain, providing formatting context even though no location was captured for the source itself. In contrast,
    /// `captured_location()` reports only locations belonging to the node itself.
    pub fn location(self) -> &'static Location<'static> {
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
}

impl<'a> ErrorNode<'a> {
    pub(crate) fn error(self) -> &'a (dyn Error + 'static) {
        match self {
            ErrorNode::Frame(frame) => frame.error(),
            ErrorNode::Source { error, .. } => error,
        }
    }

    /// Return the frame location used when formatting this node.
    ///
    /// A frame returns its own captured location. A native source inherits the location of the frame whose error owns its
    /// source chain, providing formatting context even though no location was captured for the source itself. In contrast,
    /// `captured_location()` reports only locations belonging to the node itself.
    pub(crate) fn location(self) -> &'static Location<'static> {
        match self {
            ErrorNode::Frame(frame) => frame.location,
            ErrorNode::Source { location, .. } => location,
        }
    }

    /// Return the location captured for this node itself.
    ///
    /// This is `Some` for an explicitly created frame and `None` for a native source. Unlike [`Self::location()`], it does
    /// not return the owning frame's location as inherited formatting context for a source.
    #[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
    pub(crate) fn captured_location(self) -> Option<&'static Location<'static>> {
        match self {
            ErrorNode::Frame(frame) => Some(frame.location),
            ErrorNode::Source { .. } => None,
        }
    }

    /// Return this node's immediate logical children in traversal order.
    ///
    /// A direct native [`Error::source()`] is first and inherits this node's formatting location. For a frame, explicitly
    /// raised child frames follow it in insertion order. The compatibility `source()` of a nested [`crate::Error`] is
    /// skipped because that wrapper retains an internal error graph which its own traversal APIs expand separately;
    /// following the compatibility source here would expose only one path and duplicate that expansion.
    pub(crate) fn children(self) -> Vec<ErrorNode<'a>> {
        let error = self.error();
        let location = self.location();
        let mut children = Vec::new();
        if !error.is::<crate::Error>() {
            if let Some(error) = error.source() {
                children.push(ErrorNode::Source { error, location });
            }
        }
        if let ErrorNode::Frame(frame) = self {
            children.extend(frame.children.iter().map(ErrorNode::Frame));
        }
        children
    }

    fn same(self, other: ErrorNode<'_>) -> bool {
        std::ptr::addr_eq(self.error(), other.error())
    }
}

/// Navigation
impl Frame {
    /// Find the best possible cause:
    ///
    /// * in a linear chain of a single error each, it's the last-most error
    /// * in trees, find the deepest-possible error that has the most leafs as children
    ///
    /// Native [`Error::source()`] values participate as borrowed children. Return `None` if there are no children.
    pub fn probable_cause(&self) -> Option<&(dyn Error + 'static)> {
        self.probable_cause_node().map(ErrorNode::error)
    }

    pub(crate) fn probable_cause_node(&self) -> Option<ErrorNode<'_>> {
        /// Perform a recursive depth-first, post-order walk to select a probable-cause candidate.
        ///
        /// The returned tuple contains the number of leaves below `node`, the depth of the selected candidate, and the
        /// candidate itself. After visiting all children, the current node competes with the best descendant: the candidate
        /// representing more leaves wins, with greater depth breaking ties. Exact ties between siblings retain the first
        /// child in traversal order.
        fn walk(node: ErrorNode<'_>, depth: usize) -> (usize, usize, ErrorNode<'_>) {
            let children = node.children();
            if children.is_empty() {
                return (1, depth, node);
            }

            let mut total_leafs = 0;
            let mut best: Option<(usize, usize, ErrorNode<'_>)> = None;

            for child in children {
                let (leafs, child_depth, candidate) = walk(child, depth + 1);
                total_leafs += leafs;

                match best {
                    None => best = Some((leafs, child_depth, candidate)),
                    Some((best_leafs, best_depth, _)) => {
                        if leafs > best_leafs || (leafs == best_leafs && child_depth > best_depth) {
                            best = Some((leafs, child_depth, candidate));
                        }
                    }
                }
            }

            let self_candidate = (total_leafs, depth, node);
            match best {
                None => self_candidate,
                Some(best_child) => {
                    if total_leafs > best_child.0 || (total_leafs == best_child.0 && depth > best_child.1) {
                        self_candidate
                    } else {
                        best_child
                    }
                }
            }
        }

        let root = ErrorNode::Frame(self);
        let children = root.children();
        if children.iter().all(|child| child.children().is_empty()) {
            if let Some(last) = children.last() {
                return Some(*last);
            }
        }

        let cause = walk(root, 0).2;
        (!cause.same(root)).then_some(cause)
    }

    /// Iterate over all explicitly created frames in breadth-first order. The first frame is this instance, followed by
    /// all explicitly raised children. Native [`Error::source()`] values are not frames.
    pub fn iter_frames(&self) -> impl Iterator<Item = &Frame> + '_ {
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(self);
        BreadthFirstFrames { queue }
    }

    pub(crate) fn iter_error_nodes(&self) -> BreadthFirstErrorNodes<'_> {
        let mut queue = VecDeque::new();
        queue.push_back(ErrorNode::Frame(self));
        BreadthFirstErrorNodes { queue }
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

pub(crate) struct BreadthFirstErrorNodes<'a> {
    queue: VecDeque<ErrorNode<'a>>,
}

impl<'a> Iterator for BreadthFirstErrorNodes<'a> {
    type Item = ErrorNode<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.queue.pop_front()?;
        self.queue.extend(node.children());
        Some(node)
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
        Exn {
            frame: Box::new(frame),
            phantom: Default::default(),
        }
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

/// An error that merely says that something is wrong.
pub struct Something;

impl fmt::Display for Something {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Something went wrong")
    }
}

impl fmt::Debug for Something {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self, f)
    }
}

impl Error for Something {}

impl<E> From<Exn<E>> for ChainedError
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(err: Exn<E>) -> Self {
        let probable_cause = err
            .frame
            .probable_cause_node()
            .and_then(|cause| err.frame.iter_error_nodes().position(|node| node.same(cause)));
        let flattened = flatten_error_nodes(*err.frame);
        let mut source = None;
        let leaves_to_root = flattened.into_iter().enumerate().rev();
        for (index, node) in leaves_to_root {
            source = Some(Box::new(ChainedError {
                err: node.error,
                location: node.location,
                is_probable_cause: probable_cause.map_or(index == 0, |cause| cause == index),
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
                if !error.error().is::<crate::Error>() {
                    if let Some(source) = error.source() {
                        queue.push_back(Pending::Source {
                            error: source,
                            location,
                            logical_parent: node_index,
                        });
                    }
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
                if !error.error().is::<crate::Error>() {
                    if let Some(source) = error.source() {
                        queue.push_back(Pending::Source {
                            error: source,
                            location,
                            logical_parent: node_index,
                        });
                    }
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
