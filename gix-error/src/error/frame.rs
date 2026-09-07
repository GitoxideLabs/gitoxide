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
use std::panic::Location;

use crate::{Exn, Frame};

#[expect(
    clippy::unnecessary_box_returns,
    reason = "reuse the upstream root allocation at the reporting boundary"
)]
pub(crate) fn into_frame<E: Error + Send + Sync + 'static + ?Sized>(error: Exn<E>) -> Box<Frame> {
    // exn 0.4 transfers its root Frame through the boxed standard-error conversion.
    let boxed: Box<dyn Error + Send + Sync> = error.into();
    boxed.downcast().expect("exn's boxed conversion owns its root Frame")
}

/// Unwrap the adapter used to store already boxed standard errors.
pub(crate) fn frame_error(frame: &Frame) -> &(dyn Error + Send + Sync + 'static) {
    let mut error = frame.error();
    while let Some(boxed) = error.downcast_ref::<BoxedError>() {
        error = boxed.0.as_ref();
    }
    error
}

/// Return raised children without upstream's native-source snapshot subtree.
pub(crate) fn explicit_children(frame: &Frame) -> &[Frame] {
    // exn prepends one snapshot subtree whenever a root has a native source. Traverse the original source instead.
    // Like the source-path handles used below, this assumes native source relationships remain stable.
    let skip = usize::from(frame.error().source().is_some());
    &frame.children()[skip..]
}

pub(crate) fn iter_error_nodes(frame: &Frame) -> impl Iterator<Item = ErrorNode<'_>> {
    let mut queue = VecDeque::from([ErrorNode::Frame(frame)]);
    std::iter::from_fn(move || {
        let node = queue.pop_front()?;
        queue.extend(node.children());
        Some(node)
    })
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
            children.extend(explicit_children(frame).iter().map(ErrorNode::Frame));
        }
        children
    }

    pub(crate) fn same(self, other: ErrorNode<'_>) -> bool {
        std::ptr::addr_eq(self.error(), other.error())
    }
}

pub(crate) fn probable_cause_node(frame: &Frame) -> Option<ErrorNode<'_>> {
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

/// Own a boxed standard error while retaining access to its original runtime type.
pub(crate) struct BoxedError(Box<dyn Error + Send + Sync + 'static>);

impl BoxedError {
    pub(crate) fn from_boxed(error: Box<dyn Error + Send + Sync + 'static>) -> Self {
        BoxedError(error)
    }
}

impl fmt::Display for BoxedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for BoxedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl Error for BoxedError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.source()
    }
}
