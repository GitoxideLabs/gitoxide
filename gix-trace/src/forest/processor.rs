//! Trait for processing log trees on completion.
//!
//! See [`Processor`] for more details.
use crate::forest::{Processor, Tree};
use std::{error, fmt, sync::Arc};

/// Error type returned if a [`Processor`] fails.
#[derive(Debug)]
pub struct Error {
    /// The recoverable [`Tree`] type that couldn't be processed.
    pub tree: Tree,

    source: gix_error::Error,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.source, f)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Create an error for when a [`Processor`] fails to process a [`Tree`].
pub fn error(tree: Tree, source: gix_error::Error) -> Error {
    Error { tree, source }
}

/// The result type of [`Processor::process`].
pub type Result = std::result::Result<(), Error>;

/// A [`Processor`] composed of a primary and a fallback `Processor`.
///
/// This type is returned by [`Processor::or`].
#[derive(Debug)]
pub struct WithFallback<P, F> {
    pub(super) primary: P,
    pub(super) fallback: F,
}

/// A [`Processor`] that ignores any incoming logs.
///
/// This processor cannot fail.
#[derive(Debug)]
pub struct Sink;

/// A [`Processor`] that processes incoming logs via a function.
///
/// Instances of `FromFn` are returned by the [`from_fn`] function.
#[derive(Debug)]
pub struct FromFn<F>(F);

/// Create a processor that processes incoming logs via a function.
///
/// # Examples
///
/// Send completed trees across a channel for another thread to process.
/// ```
/// use gix_error::ErrorExt;
/// use gix_trace::forest::processor;
///
/// let (tx, rx) = std::sync::mpsc::channel();
///
/// let sender_processor = processor::from_fn(move |tree| tx
///     .send(tree)
///     .map_err(|err| {
///         let msg = gix_error::message!("{err}").raise();
///         processor::error(err.0, msg)
///     })
/// );
/// ```
pub fn from_fn<F>(f: F) -> FromFn<F>
where
    F: 'static + Fn(Tree) -> Result,
{
    FromFn(f)
}

impl<P, F> Processor for WithFallback<P, F>
where
    P: Processor,
    F: Processor,
{
    fn process(&self, tree: Tree) -> Result {
        self.primary.process(tree).or_else(|err| {
            eprintln!("{err}, using fallback processor...");
            self.fallback.process(err.tree)
        })
    }
}

impl Processor for Sink {
    fn process(&self, _tree: Tree) -> Result {
        Ok(())
    }
}

impl<F> Processor for FromFn<F>
where
    F: 'static + Fn(Tree) -> Result,
{
    fn process(&self, tree: Tree) -> Result {
        (self.0)(tree)
    }
}

impl<P: Processor> Processor for Box<P> {
    fn process(&self, tree: Tree) -> Result {
        self.as_ref().process(tree)
    }
}

impl<P: Processor> Processor for Arc<P> {
    fn process(&self, tree: Tree) -> Result {
        self.as_ref().process(tree)
    }
}
