/// The error returned by [`spec::parse()`][crate::spec::parse()].
pub use gix_error::ParseError as Error;

///
pub mod delegate;

/// A delegate to be informed about parse events, with methods split into categories.
///
/// - **Anchors** - which revision to use as starting point for…
/// - **Navigation** - where to go once from the initial revision
/// - **Range** - to learn if the specification is for a single or multiple references, and how to combine them.
pub trait Delegate: delegate::Revision + delegate::Navigate + delegate::Kind {
    /// Called at the end of a successful parsing operation.
    /// It can be used as a marker to finalize internal data structures.
    ///
    /// Note that it will not be called if there is unconsumed input.
    fn done(&mut self);
}

pub(crate) mod function;
