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
use std::ops::Deref;
use std::panic::Location;

use crate::concrete::chain::ErrorHandle;
use crate::{ChainedError, Exn, Frame, write_location};

impl<E: Error + Send + Sync + 'static> From<E> for Exn<E> {
    #[track_caller]
    fn from(error: E) -> Self {
        Self::new(error)
    }
}

impl<E: Error + Send + Sync + 'static> From<E> for Exn {
    #[track_caller]
    fn from(error: E) -> Self {
        Exn { inner: error.into() }
    }
}

impl<E: Error + Send + Sync + 'static> From<Exn<E>> for Exn {
    fn from(error: Exn<E>) -> Self {
        Exn {
            inner: error.inner.into(),
        }
    }
}

impl<E: Error + Send + Sync + 'static> From<Exn<E>> for ::exn::Exn {
    fn from(error: Exn<E>) -> Self {
        error.inner.into()
    }
}

impl<E: Error + Send + Sync + 'static + ?Sized> From<Exn<E>> for ::exn::Exn<E> {
    fn from(error: Exn<E>) -> Self {
        error.inner
    }
}

impl<E: Error + Send + Sync + 'static> Exn<E> {
    /// Construct an exception using upstream storage and caller tracking.
    ///
    /// Upstream snapshots native source messages. Gitoxide's traversal and formatting use the original native
    /// sources instead, preserving their concrete types and distinguishing them from explicitly raised frames.
    #[track_caller]
    pub fn new(error: E) -> Self {
        Self::from_upstream(::exn::Exn::new(error))
    }

    /// Construct a parent over all errors or exceptions in `children`, in iteration order.
    #[track_caller]
    pub fn raise_all(children: impl IntoIterator<Item: Into<::exn::Exn>>, error: E) -> Self {
        use ::exn::IteratorExt;
        Self::from_upstream(children.into_iter().raise(error))
    }
}

impl<E: Error + Send + Sync + 'static + ?Sized> Exn<E> {
    /// Wrap an upstream exception without reallocating its tree.
    pub fn from_upstream(inner: ::exn::Exn<E>) -> Self {
        Exn { inner }
    }

    /// Add a new typed parent to this exception.
    #[track_caller]
    pub fn raise<T: Error + Send + Sync + 'static>(self, error: T) -> Exn<T> {
        Exn::from_upstream(self.inner.raise(error))
    }

    /// Erase the root marker without allocating or changing the frame tree.
    pub fn erased(self) -> Exn
    where
        ::exn::Exn<E>: Into<::exn::Exn>,
    {
        Exn {
            inner: self.inner.into(),
        }
    }

    /// Return the current root error.
    pub fn error(&self) -> &E
    where
        ::exn::Exn<E>: Deref<Target = E>,
    {
        &self.inner
    }

    /// Convert to gitoxide's standard-error boundary, preserving the complete error graph.
    pub fn into_error(self) -> crate::Error {
        self.into()
    }

    /// Flatten the complete graph into a standard source chain in breadth-first order.
    pub fn into_chain(self) -> ChainedError {
        self.into()
    }

    /// Return the upstream frame, including its native-source snapshots.
    pub fn frame(&self) -> &Frame {
        self.inner.frame()
    }

    /// Iterate over explicitly raised frames in breadth-first order.
    pub fn iter(&self) -> impl Iterator<Item = &Frame> {
        self.frame().iter_frames()
    }

    /// Find a stored error or native source of type `T` in breadth-first order.
    pub fn downcast_any_ref<T: Error + 'static>(&self) -> Option<&T> {
        iter_error_nodes(self.frame()).find_map(|node| node.error().downcast_ref())
    }
}

impl<E: Error + Send + Sync + 'static + ?Sized> Deref for Exn<E>
where
    ::exn::Exn<E>: Deref<Target = E>,
{
    type Target = E;
    fn deref(&self) -> &E {
        &self.inner
    }
}

impl<E: Error + Send + Sync + 'static + ?Sized> fmt::Debug for Exn<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        debug_frame(self.frame(), f)
    }
}

pub(crate) fn debug_frame(frame: &Frame, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write_frame_recursive(f, frame, "", ErrorMode::Display, TreeMode::Linearize)
}

pub(crate) fn display_frame(frame: &Frame, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if f.alternate() {
        write_frame_recursive(f, frame, "", ErrorMode::Debug, TreeMode::Verbatim)
    } else {
        fmt::Display::fmt(frame_error(frame), f)
    }
}

impl<E: Error + Send + Sync + 'static + ?Sized> fmt::Display for Exn<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        display_frame(self.frame(), f)
    }
}

impl<E: Error + Send + Sync + 'static + ?Sized> PartialEq<str> for Exn<E> {
    fn eq(&self, other: &str) -> bool {
        crate::root_error_eq(frame_error(self.frame()), other)
    }
}

impl<E: Error + Send + Sync + 'static + ?Sized> PartialEq<&str> for Exn<E> {
    fn eq(&self, other: &&str) -> bool {
        <Self as PartialEq<str>>::eq(self, other)
    }
}

impl<E: Error + Send + Sync + 'static + ?Sized> PartialEq<String> for Exn<E> {
    fn eq(&self, other: &String) -> bool {
        <Self as PartialEq<str>>::eq(self, other)
    }
}

impl<E: Error + Send + Sync + 'static + ?Sized> From<Exn<E>> for Box<Frame> {
    fn from(error: Exn<E>) -> Self {
        // exn 0.4's boxed standard-error conversion transfers its root Frame without another allocation.
        let boxed: Box<dyn Error + Send + Sync> = error.inner.into();
        boxed.downcast().expect("exn's boxed conversion owns its root Frame")
    }
}

impl<E: Error + Send + Sync + 'static + ?Sized> From<Exn<E>> for Box<dyn Error + Send + Sync> {
    fn from(error: Exn<E>) -> Self {
        Box::new(error.into_error())
    }
}

#[cfg(feature = "anyhow")]
impl<E: Error + Send + Sync + 'static + ?Sized> From<Exn<E>> for anyhow::Error {
    fn from(error: Exn<E>) -> Self {
        error.into_chain().into()
    }
}

/// Unwrap the adapter used to store already boxed standard errors.
pub(crate) fn frame_error(frame: &Frame) -> &(dyn Error + Send + Sync + 'static) {
    let mut error = frame.error();
    while let Some(boxed) = error.downcast_ref::<Untyped>() {
        error = boxed.0.as_ref();
    }
    error
}

/// Gitoxide's navigation policy for upstream exception frames.
pub trait FrameExt {
    /// Return explicitly raised children, excluding upstream snapshots of native sources.
    fn explicit_children(&self) -> &[Frame];
    /// Iterate over explicitly raised frames in breadth-first order, starting at this frame.
    fn iter_frames(&self) -> impl Iterator<Item = &Frame>;
    /// Select the most likely cause, including concrete native sources, or `None` for a leaf.
    fn probable_cause(&self) -> Option<&(dyn Error + 'static)>;
}

impl FrameExt for Frame {
    fn explicit_children(&self) -> &[Frame] {
        // exn prepends one snapshot subtree whenever a root has a native source. Traverse the original source instead.
        // Like the source-path handles used below, this assumes native source relationships remain stable.
        let skip = usize::from(self.error().source().is_some());
        &self.children()[skip..]
    }

    fn iter_frames(&self) -> impl Iterator<Item = &Frame> {
        let mut queue = VecDeque::from([self]);
        std::iter::from_fn(move || {
            let frame = queue.pop_front()?;
            queue.extend(frame.explicit_children());
            Some(frame)
        })
    }

    fn probable_cause(&self) -> Option<&(dyn Error + 'static)> {
        probable_cause_node(self).map(ErrorNode::error)
    }
}

fn iter_error_nodes(frame: &Frame) -> impl Iterator<Item = ErrorNode<'_>> {
    let mut queue = VecDeque::from([ErrorNode::Frame(frame)]);
    std::iter::from_fn(move || {
        let node = queue.pop_front()?;
        queue.extend(node.children());
        Some(node)
    })
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
    // Nested boundaries are expanded below; formatting their whole graph here would print the same causes twice.
    let mut error = node.error();
    while let Some(nested) = error.downcast_ref::<crate::Error>() {
        error = nested.error();
    }
    match err_mode {
        ErrorMode::Display => fmt::Display::fmt(error, f),
        ErrorMode::Debug => write!(f, "{error:?}"),
    }?;
    if !f.alternate() {
        write_location(f, node.location())?;
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

    if let Some(err) = node.error().downcast_ref::<crate::Error>() {
        for source in err.iter_errors().filter(|source| !source.is::<crate::Error>()).skip(1) {
            write!(f, "\n{prefix}|\n{prefix}└─ ")?;
            match err_mode {
                ErrorMode::Display => fmt::Display::fmt(source, f),
                ErrorMode::Debug => write!(f, "{source:?}"),
            }?;
        }
    }

    Ok(())
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
            ErrorNode::Frame(frame) => frame_error(frame),
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
            ErrorNode::Frame(frame) => frame.location(),
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
            ErrorNode::Frame(frame) => Some(frame.location()),
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
        if !error.is::<crate::Error>()
            && let Some(error) = error.source()
        {
            children.push(ErrorNode::Source { error, location });
        }
        if let ErrorNode::Frame(frame) = self {
            children.extend(frame.explicit_children().iter().map(ErrorNode::Frame));
        }
        children
    }

    fn same(self, other: ErrorNode<'_>) -> bool {
        std::ptr::addr_eq(self.error(), other.error())
    }
}

fn probable_cause_node(frame: &Frame) -> Option<ErrorNode<'_>> {
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

    let root = ErrorNode::Frame(frame);
    let children = root.children();
    if children.iter().all(|child| child.children().is_empty())
        && let Some(last) = children.last()
    {
        return Some(*last);
    }

    let cause = walk(root, 0).2;
    (!cause.same(root)).then_some(cause)
}

/// An adapter for storing an already boxed standard error. Bare [`Exn`] uses upstream type erasure instead.
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

impl<E: Error + Send + Sync + 'static + ?Sized> From<Exn<E>> for ChainedError {
    fn from(error: Exn<E>) -> Self {
        let root: Box<Frame> = error.into();
        let probable_cause =
            probable_cause_node(&root).and_then(|cause| iter_error_nodes(&root).position(|node| node.same(cause)));
        let mut queue = VecDeque::from([(ErrorHandle::new(root.into()), None)]);
        let mut flattened = Vec::new();
        while let Some((error, parent)) = queue.pop_front() {
            let index = flattened.len();
            queue.extend(error.children().into_iter().map(|child| (child, Some(index))));
            flattened.push((error, parent));
        }
        let mut source = None;
        for (index, (error, parent)) in flattened.into_iter().enumerate().rev() {
            source = Some(Box::new(ChainedError {
                location: error.location(),
                err: error,
                is_probable_cause: probable_cause.map_or(index == 0, |cause| cause == index),
                logical_parent: parent,
                source,
            }));
        }
        *source.expect("an exception always contains a root frame")
    }
}
