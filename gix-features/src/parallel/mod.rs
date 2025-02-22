//! Run computations in parallel, or not based the `parallel` feature toggle.
//!
//! ### `in_parallel`(…)
//!
//! The [`in_parallel(…)`][in_parallel()] is the typical fan-out-fan-in mode of parallelism, with thread local storage
//! made available to a `consume(…)` function to process input. The result is sent to the [`Reduce`] running in the calling
//! thread to aggregate the results into a single output, which is returned by [`in_parallel()`].
//!
//! Interruptions can be achieved by letting the reducers [`feed(…)`][Reduce::feed()] method fail.
//!
//! It gets a boost in usability as it allows threads to borrow variables from the stack, most commonly the repository itself
//! or the data to work on.
//!
//! This mode of operation doesn't lend itself perfectly to being wrapped for `async` as it appears like a single long-running
//! operation which runs as fast as possible, which is cancellable only by merit of stopping the input or stopping the output
//! aggregation.
//!
//! ### `reduce::Stepwise`
//!
//! The [`Stepwise`][reduce::Stepwise] iterator works exactly as [`in_parallel()`] except that the processing of the output produced by
//! `consume(I, &mut State) -> O` is made accessible by the `Iterator` trait's `next()` method. As produced work is not
//! buffered, the owner of the iterator controls the progress made.
//!
//! Getting the final output of the [`Reduce`] is achieved through the consuming [`Stepwise::finalize()`][reduce::Stepwise::finalize()] method, which
//! is functionally equivalent to calling [`in_parallel()`].
//!
//! In an `async` context this means that progress is only made each time `next()` is called on the iterator, while merely dropping
//! the iterator will wind down the computation without any result.
//!
//! #### Maintaining Safety
//!
//! In order to assure that threads don't outlive the data they borrow because their handles are leaked, we enforce
//! the `'static` lifetime for its inputs, making it less intuitive to use. It is, however, possible to produce
//! suitable input iterators as long as they can hold something on the heap.
#[cfg(feature = "parallel")]
mod in_parallel;
#[cfg(feature = "parallel")]
pub use in_parallel::{
    build_thread, in_parallel, in_parallel_with_finalize, in_parallel_with_slice, join, threads, Scope,
};

mod serial;
#[cfg(not(feature = "parallel"))]
pub use serial::{build_thread, in_parallel, in_parallel_with_finalize, in_parallel_with_slice, join, threads, Scope};

mod in_order;
pub use in_order::{InOrderIter, SequenceId};

mod eager_iter;
pub use eager_iter::{EagerIter, EagerIterIf};

/// A no-op returning the input _(`desired_chunk_size`, `Some(thread_limit)`, `thread_limit)_ used
/// when the `parallel` feature toggle is not set.
#[cfg(not(feature = "parallel"))]
pub fn optimize_chunk_size_and_thread_limit(
    desired_chunk_size: usize,
    _num_items: Option<usize>,
    thread_limit: Option<usize>,
    _available_threads: Option<usize>,
) -> (usize, Option<usize>, usize) {
    (desired_chunk_size, thread_limit, num_threads(thread_limit))
}

/// Return the 'optimal' _(`size of chunks`,  `amount of threads as Option`, `amount of threads`)_ to use in [`in_parallel()`] for the given
/// `desired_chunk_size`, `num_items`, `thread_limit` and `available_threads`.
///
/// * `desired_chunk_size` is the amount of items per chunk you think should be used.
/// * `num_items` is the total amount of items in the iteration, if `Some`.
///   Otherwise this knowledge will not affect the output of this function.
/// * `thread_limit` is the amount of threads to use at most, if `Some`.
///   Otherwise this knowledge will not affect the output of this function.
/// * `available_threads` is the total amount of threads available, if `Some`.
///   Otherwise the actual amount of available threads is determined by querying the system.
///
/// `Note` that this implementation is available only if the `parallel` feature toggle is set.
#[cfg(feature = "parallel")]
pub fn optimize_chunk_size_and_thread_limit(
    desired_chunk_size: usize,
    num_items: Option<usize>,
    thread_limit: Option<usize>,
    available_threads: Option<usize>,
) -> (usize, Option<usize>, usize) {
    let available_threads =
        available_threads.unwrap_or_else(|| std::thread::available_parallelism().map_or(1, Into::into));
    let available_threads = thread_limit.map_or(available_threads, |l| if l == 0 { available_threads } else { l });

    let (lower, upper) = (50, 1000);
    let (chunk_size, thread_limit) = num_items.map_or(
        {
            let chunk_size = if available_threads == 1 {
                desired_chunk_size
            } else if desired_chunk_size < lower {
                lower
            } else {
                desired_chunk_size.min(upper)
            };
            (chunk_size, available_threads)
        },
        |num_items| {
            let desired_chunks_per_thread_at_least = 2;
            let items = num_items;
            let chunk_size = (items / (available_threads * desired_chunks_per_thread_at_least)).clamp(1, upper);
            let num_chunks = items / chunk_size;
            let thread_limit = if num_chunks <= available_threads {
                (num_chunks / desired_chunks_per_thread_at_least).max(1)
            } else {
                available_threads
            };
            (chunk_size, thread_limit)
        },
    );
    (chunk_size, Some(thread_limit), thread_limit)
}

/// Always returns 1, available when the `parallel` feature toggle is unset.
#[cfg(not(feature = "parallel"))]
pub fn num_threads(_thread_limit: Option<usize>) -> usize {
    1
}

/// Returns the amount of threads the system can effectively use as the amount of its logical cores.
///
/// Only available with the `parallel` feature toggle set.
#[cfg(feature = "parallel")]
pub fn num_threads(thread_limit: Option<usize>) -> usize {
    let logical_cores = std::thread::available_parallelism().map_or(1, Into::into);
    thread_limit.map_or(logical_cores, |l| if l == 0 { logical_cores } else { l })
}

/// Run [`in_parallel()`] only if the given `condition()` returns true when eagerly evaluated.
///
/// For parameters, see the documentation of [`in_parallel()`]
#[cfg(feature = "parallel")]
pub fn in_parallel_if<I, S, O, R>(
    condition: impl FnOnce() -> bool,
    input: impl Iterator<Item = I> + Send,
    thread_limit: Option<usize>,
    new_thread_state: impl FnOnce(usize) -> S + Send + Clone,
    consume: impl FnMut(I, &mut S) -> O + Send + Clone,
    reducer: R,
) -> Result<<R as Reduce>::Output, <R as Reduce>::Error>
where
    R: Reduce<Input = O>,
    I: Send,
    O: Send,
{
    if num_threads(thread_limit) > 1 && condition() {
        in_parallel(input, thread_limit, new_thread_state, consume, reducer)
    } else {
        serial::in_parallel(input, thread_limit, new_thread_state, consume, reducer)
    }
}

/// Run [`in_parallel()`] only if the given `condition()` returns true when eagerly evaluated.
///
/// For parameters, see the documentation of [`in_parallel()`]
///
/// Note that the non-parallel version is equivalent to [`in_parallel()`].
#[cfg(not(feature = "parallel"))]
pub fn in_parallel_if<I, S, O, R>(
    _condition: impl FnOnce() -> bool,
    input: impl Iterator<Item = I>,
    thread_limit: Option<usize>,
    new_thread_state: impl FnOnce(usize) -> S,
    consume: impl FnMut(I, &mut S) -> O,
    reducer: R,
) -> Result<<R as Reduce>::Output, <R as Reduce>::Error>
where
    R: Reduce<Input = O>,
    I: Send,
    O: Send,
{
    serial::in_parallel(input, thread_limit, new_thread_state, consume, reducer)
}

///
pub mod reduce;
pub use reduce::Reduce;
