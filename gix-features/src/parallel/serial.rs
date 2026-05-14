use crate::parallel::Reduce;

#[cfg(not(feature = "parallel"))]
mod not_parallel {
    use std::sync::atomic::{AtomicBool, AtomicIsize};

    /// Runs `left` and then `right`, one after another, returning their output when both are done.
    pub fn join<O1, O2>(left: impl FnOnce() -> O1, right: impl FnOnce() -> O2) -> (O1, O2) {
        (left(), right())
    }

    /// A scope for spawning threads.
    pub struct Scope<'scope, 'env: 'scope> {
        _scope: std::marker::PhantomData<&'scope mut &'scope ()>,
        _env: std::marker::PhantomData<&'env mut &'env ()>,
    }

    pub struct ThreadBuilder;

    /// Create a builder for threads which allows them to be spawned into a scope and configured prior to spawning.
    pub fn build_thread() -> ThreadBuilder {
        ThreadBuilder
    }

    #[allow(unsafe_code)]
    unsafe impl Sync for Scope<'_, '_> {}

    impl ThreadBuilder {
        pub fn name(self, _new: String) -> Self {
            self
        }
        pub fn spawn_scoped<'scope, 'env, F, T>(
            &self,
            scope: &'scope Scope<'scope, 'env>,
            f: F,
        ) -> std::io::Result<ScopedJoinHandle<'scope, T>>
        where
            F: FnOnce() -> T + 'scope,
            T: 'scope,
        {
            Ok(scope.spawn(f))
        }
    }

    impl<'scope> Scope<'scope, '_> {
        /// Provided with this scope, let `f` start new threads that live within it.
        pub fn spawn<F, T>(&'scope self, f: F) -> ScopedJoinHandle<'scope, T>
        where
            F: FnOnce() -> T + 'scope,
            T: 'scope,
        {
            ScopedJoinHandle {
                result: f(),
                _marker: Default::default(),
            }
        }
    }

    /// Runs `f` with a scope to be used for spawning threads that will not outlive the function call.
    /// Note that this implementation will run the spawned functions immediately.
    pub fn threads<'env, F, R>(f: F) -> R
    where
        F: for<'scope> FnOnce(&'scope Scope<'scope, 'env>) -> R,
    {
        f(&Scope {
            _scope: Default::default(),
            _env: Default::default(),
        })
    }

    /// A handle that can be used to join its scoped thread.
    ///
    /// This struct is created by the [`Scope::spawn`] method and the
    /// [`ScopedThreadBuilder::spawn`] method.
    pub struct ScopedJoinHandle<'scope, T> {
        /// Holds the result of the inner closure.
        result: T,
        _marker: std::marker::PhantomData<&'scope mut &'scope ()>,
    }

    impl<T> ScopedJoinHandle<'_, T> {
        pub fn join(self) -> std::thread::Result<T> {
            Ok(self.result)
        }
        pub fn is_finished(&self) -> bool {
            true
        }
    }

    /// An experiment to have fine-grained per-item parallelization with built-in aggregation via thread state.
    /// This is only good for operations where near-random access isn't detrimental, so it's not usually great
    /// for file-io as it won't make use of sorted inputs well.
    // TODO: better docs
    pub fn in_parallel_with_slice<I, S, R, E>(
        input: &mut [I],
        _thread_limit: Option<usize>,
        new_thread_state: impl FnOnce(usize) -> S + Clone,
        mut consume: impl FnMut(&mut I, &mut S, &AtomicIsize, &AtomicBool) -> Result<(), E> + Clone,
        mut periodic: impl FnMut() -> Option<std::time::Duration>,
        state_to_rval: impl FnOnce(S) -> R + Clone,
    ) -> Result<Vec<R>, E> {
        let mut state = new_thread_state(0);
        let should_interrupt = &AtomicBool::default();
        let threads_left = &AtomicIsize::default();
        for item in input {
            consume(item, &mut state, threads_left, should_interrupt)?;
            if periodic().is_none() {
                break;
            }
        }
        Ok(vec![state_to_rval(state)])
    }
}

#[cfg(not(feature = "parallel"))]
pub use not_parallel::{Scope, build_thread, in_parallel_with_slice, join, threads};

/// Read items from `input` and `consume` them in a single thread, producing an output to be collected by a `reducer`,
/// whose task it is to aggregate these outputs into the final result returned by this function.
///
/// * `new_thread_state(thread_number) -> State` produces thread-local state once per thread to be passed to `consume`
/// * `consume(Item, &mut State) -> Output` produces an output given an input along with mutable state.
/// * For `reducer`, see the [`Reduce`] trait
/// * `thread_limit` has no effect as everything is run on the main thread, but is present to keep the signature
///   similar to the parallel version.
///
/// **This serial version performing all calculations on the current thread.**
pub fn in_parallel<I, S, O, R>(
    input: impl Iterator<Item = I>,
    _thread_limit: Option<usize>,
    new_thread_state: impl FnOnce(usize) -> S,
    mut consume: impl FnMut(I, &mut S) -> O,
    mut reducer: R,
) -> Result<<R as Reduce>::Output, <R as Reduce>::Error>
where
    R: Reduce<Input = O>,
{
    let mut state = new_thread_state(0);
    for item in input {
        drop(reducer.feed(consume(item, &mut state))?);
    }
    reducer.finalize()
}

/// Read items from `input` and `consume` them in multiple threads,
/// whose output is collected by a `reducer`. Its task is to
/// aggregate these outputs into the final result returned by this function with the benefit of not having to be thread-safe.
/// Call `finalize` to finish the computation, once per thread, if there was no error sending results earlier.
///
/// * if `thread_limit` is `Some`, the given number of threads will be used. If `None`, all logical cores will be used.
/// * `new_thread_state(thread_number) -> State` produces thread-local state once per thread to be passed to `consume`
/// * `consume(Item, &mut State) -> Output` produces an output given an input obtained by `input` along with mutable state initially
///   created by `new_thread_state(…)`.
/// * `finalize(State) -> Output` is called to potentially process remaining work that was placed in `State`.
/// * For `reducer`, see the [`Reduce`] trait
#[cfg(not(feature = "parallel"))]
pub fn in_parallel_with_finalize<I, S, O, R>(
    input: impl Iterator<Item = I>,
    _thread_limit: Option<usize>,
    new_thread_state: impl FnOnce(usize) -> S,
    mut consume: impl FnMut(I, &mut S) -> O,
    finalize: impl FnOnce(S) -> O + Send + Clone,
    mut reducer: R,
) -> Result<<R as Reduce>::Output, <R as Reduce>::Error>
where
    R: Reduce<Input = O>,
{
    let mut state = new_thread_state(0);
    for item in input {
        drop(reducer.feed(consume(item, &mut state))?);
    }
    reducer.feed(finalize(state))?;
    reducer.finalize()
}
