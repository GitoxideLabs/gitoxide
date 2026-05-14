//! Utility types for traversing the git commit-graph.
//!
//! This crate considers itself very much *plumbing* and is meant for consumption by other plumbing crates.
#![deny(missing_docs)]
#![forbid(unsafe_code)]

/// A graph of commits which additionally allows to associate data with commits.
///
/// It starts empty, but each access may fill it with commit information.
/// Note that the traversal can be accelerated if a [commit-graph][gix_commitgraph::Graph] is also made available.
///
/// ### About replacements
///
/// Object replacements is an object database feature to substitute one object with another. We assume that this is transparently
/// implemented by the `find` function that returns objects. Also we assume that the commitgraph as been written with replacements
/// active to provide a consistent view.
///
/// ### Odb or `find` configuration
///
/// The `find` handle should be setup to *quickly determine if an object exists or not* to assure quick operation *on shallow repositories*.
/// This typically means that it should not re-read the odb if there is an object miss.
///
/// Most usage of the Graph will benefit from fast ODB lookups, so setting up an object cache will be beneficial. If that's not the case,
/// the method docs will inform about that.
///
/// Additionally, and only if `T` is [`Commit<T>`][graph::Commit], there is *no need for an object cache* as we keep track of
/// everything related to commit traversal in our own hashmap.
pub struct Graph<'find, 'cache, T> {
    /// A way to resolve a commit from the object database.
    find: Box<dyn gix_object::Find + 'find>,
    /// A way to speedup commit access, essentially a multi-file commit database.
    cache: Option<&'cache gix_commitgraph::Graph>,
    /// The set of cached commits that we have seen once, along with data associated with them.
    map: graph::IdMap<T>,
    /// A buffer for writing commit data into.
    buf: Vec<u8>,
    /// Another buffer we typically use to store parents.
    parent_buf: Vec<u8>,
}

///
pub mod graph;

/// A utility type implementing a queue which can be used to automatically sort data by its time in ascending order.
///
/// Note that the performance of this queue is very relevant to overall algorithm performance of many graph-walking algorithms,
/// and as it stands our implementation is about 6% slower in practice, probably also depending on the size of the stored data.
#[derive(Default)]
pub struct PriorityQueue<K: Ord, T>(std::collections::BinaryHeap<queue::Item<K, T>>);
mod queue;
