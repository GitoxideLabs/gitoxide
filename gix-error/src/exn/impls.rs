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

use std::error::Error;
use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;
use std::panic::Location;

use crate::Exn;

impl<E: Error + Send + Sync + 'static> From<E> for Exn<E> {
    #[track_caller]
    fn from(error: E) -> Self {
        Exn::new(error)
    }
}

impl<E: Error + Send + Sync + 'static> Exn<E> {
    /// Create a new exception with the given error.
    ///
    /// This will automatically walk the [source chain of the error] and add them as children
    /// frames.
    ///
    /// See also [`ErrorExt::raise`](crate::ErrorExt) for a fluent way to convert an error into an `Exn` instance.
    ///
    /// Note that **sources of `error` are degenerated to their string representation** and all type information is erased.
    ///
    /// [source chain of the error]: Error::source
    #[track_caller]
    pub fn new(error: E) -> Self {
        fn walk_sources(error: &dyn Error, location: &'static Location<'static>) -> Vec<Frame> {
            if let Some(source) = error.source() {
                let children = vec![Frame {
                    error: Box::new(SourceError::new(source)),
                    location,
                    children: walk_sources(source, location),
                }];
                children
            } else {
                vec![]
            }
        }

        let location = Location::caller();
        let children = walk_sources(&error, location);
        let frame = Frame {
            error: Box::new(error),
            location,
            children,
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

    /// Drain all sources of this error as untyped [`Exn`].
    ///
    /// This is useful if one wants to re-organise errors, and the error layout is well known.
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

    /// Return the underlying exception frame.
    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    /// Iterate over all frames in breadth-first order. The first frame is this instance,
    /// followed by all of its children.
    pub fn iter(&self) -> impl Iterator<Item = &Frame> {
        self.frame().iter()
    }

    /// Iterate over all frames and find one that downcasts into error of type `T`.
    /// Note that the search includes this instance as well.
    pub fn downcast_any_ref<T: Error + 'static>(&self) -> Option<&T> {
        self.iter().find_map(|e| e.downcast())
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
    match err_mode {
        ErrorMode::Display => fmt::Display::fmt(frame.error(), f),
        ErrorMode::Debug => {
            write!(f, "{:?}", frame.error())
        }
    }?;
    if !f.alternate() {
        write_location(f, frame)?;
    }

    let children = frame.children();
    let children_len = children.len();

    for (cidx, child) in children.iter().enumerate() {
        write!(f, "\n{prefix}|")?;
        write!(f, "\n{prefix}└─ ")?;

        let child_child_len = child.children().len();
        let may_linearize_chain = matches!(tree_mode, TreeMode::Linearize) && children_len == 1 && child_child_len == 1;
        if may_linearize_chain {
            write_frame_recursive(f, child, prefix, err_mode, tree_mode)?;
        } else if cidx < children_len - 1 {
            write_frame_recursive(f, child, &format!("{prefix}|   "), err_mode, tree_mode)?;
        } else {
            write_frame_recursive(f, child, &format!("{prefix}    "), err_mode, tree_mode)?;
        }
    }

    Ok(())
}

fn write_location(f: &mut fmt::Formatter<'_>, exn: &Frame) -> fmt::Result {
    let location = exn.location();
    write!(f, ", at {}:{}:{}", location.file(), location.line(), location.column())
}

impl<E: Error + Send + Sync + 'static> fmt::Display for Exn<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.frame, f)
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
    /// Child exception frames that provide additional context or source errors.
    children: Vec<Frame>,
}

impl Frame {
    /// Return the error as a reference to [`Error`].
    pub fn error(&self) -> &(dyn Error + Send + Sync + 'static) {
        &*self.error
    }

    /// Try to downcast this error into the exact type T, or return `None`
    pub fn downcast<T: Error + 'static>(&self) -> Option<&T> {
        self.error.downcast_ref()
    }

    /// Return the source code location where this exception frame was created.
    pub fn location(&self) -> &'static Location<'static> {
        self.location
    }

    /// Return a slice of the children of the exception.
    pub fn children(&self) -> &[Frame] {
        &self.children
    }
}

/// Navigation
impl Frame {
    /// Find the best possible cause:
    ///
    /// * in a linear chain of a single error each, it's the last-most child frame
    /// * in trees, find the deepest-possible frame that has the most leafs as children
    ///
    /// Return `None` if there are no children.
    pub fn probable_cause(&self) -> Option<&Frame> {
        fn walk<'a>(frame: &'a Frame, depth: usize) -> (usize, usize, &'a Frame) {
            if frame.children.is_empty() {
                return (1, depth, frame);
            }

            let mut total_leafs = 0;
            let mut best: Option<(usize, usize, &'a Frame)> = None;

            for child in &frame.children {
                let (leafs, d, f) = walk(child, depth + 1);
                total_leafs += leafs;

                match best {
                    None => best = Some((leafs, d, f)),
                    Some((bl, bd, _)) => {
                        if leafs > bl || (leafs == bl && d > bd) {
                            best = Some((leafs, d, f));
                        }
                    }
                }
            }

            let self_candidate = (total_leafs, depth, frame);
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

        // Special case: a simple error just with children would render the same as a chain,
        //               so pretend the last child is the cause.
        if self.children().iter().all(|c| c.children().is_empty()) {
            if let Some(last) = self.children().last() {
                return Some(last);
            }
        }

        let res = walk(self, 0).2;
        if std::ptr::addr_eq(res, self) {
            None
        } else {
            Some(res)
        }
    }

    /// Iterate over all frames in breadth-first order. The first frame is this instance,
    /// followed by all of its children.
    pub fn iter(&self) -> impl Iterator<Item = &Frame> + '_ {
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(self);
        BreadthFirstFrames { queue }
    }
}

/// Breadth-first iterator over `Frame`s.
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

impl Error for Untyped {}

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

/// A way to keep all information of errors returned by `source()` chains.
struct SourceError {
    display: String,
    alt_display: String,
    debug: String,
    alt_debug: String,
}

impl fmt::Debug for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dbg = if f.alternate() { &self.alt_debug } else { &self.debug };
        f.write_str(dbg)
    }
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ds = if f.alternate() {
            &self.alt_display
        } else {
            &self.display
        };
        f.write_str(ds)
    }
}

impl Error for SourceError {}

impl SourceError {
    fn new(err: &dyn Error) -> Self {
        SourceError {
            display: err.to_string(),
            alt_display: format!("{err:#}"),
            debug: format!("{err:?}"),
            alt_debug: format!("{err:#?}"),
        }
    }
}
