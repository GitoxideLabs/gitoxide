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

/// A shared owner and a path to an error in an upstream exception tree.
///
/// Upstream frames are read-only. Keeping the entire tree alive lets flattened nodes borrow their original errors
/// without cloning errors or losing types. Frame indices and native source depths avoid self-referential references.
pub(crate) struct ErrorHandle {
    owner: Arc<crate::Frame>,
    frame_path: Vec<usize>,
    source_depth: usize,
}

impl ErrorHandle {
    pub(crate) fn new(owner: Arc<crate::Frame>) -> Self {
        Self {
            owner,
            frame_path: Vec::new(),
            source_depth: 0,
        }
    }

    fn frame(&self) -> &crate::Frame {
        self.frame_path
            .iter()
            .fold(self.owner.as_ref(), |frame, &index| &frame.children()[index])
    }

    pub(crate) fn error(&self) -> &(dyn std::error::Error + 'static) {
        let mut error: &(dyn std::error::Error + 'static) = crate::error::frame_error(self.frame());
        for _ in 0..self.source_depth {
            error = error
                .source()
                .expect("native source paths remain stable while their owner is alive");
        }
        error
    }

    pub(crate) fn location(&self) -> &'static Location<'static> {
        self.frame().location()
    }

    pub(crate) fn children(&self) -> Vec<Self> {
        use crate::error::explicit_children;
        let mut children = Vec::new();
        if !self.error().is::<crate::Error>() && self.error().source().is_some() {
            children.push(Self {
                owner: Arc::clone(&self.owner),
                frame_path: self.frame_path.clone(),
                source_depth: self.source_depth + 1,
            });
        }
        if self.source_depth == 0 {
            let frame = self.frame();
            let explicit = explicit_children(frame);
            let offset = frame.children().len() - explicit.len();
            for index in offset..frame.children().len() {
                let mut path = self.frame_path.clone();
                path.push(index);
                children.push(Self {
                    owner: Arc::clone(&self.owner),
                    frame_path: path,
                    source_depth: 0,
                });
            }
        }
        children
    }

    #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
    pub(crate) fn is_native_source(&self) -> bool {
        self.source_depth > 0
    }
}

impl<E: std::error::Error + Send + Sync + 'static + ?Sized> From<crate::Exn<E>> for ChainedError {
    fn from(error: crate::Exn<E>) -> Self {
        Self::from_frame(crate::error::into_frame(error))
    }
}

impl ChainedError {
    pub(crate) fn from_frame(root: Box<crate::Frame>) -> Self {
        use crate::error::{iter_error_nodes, probable_cause_node};
        use std::collections::VecDeque;
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
