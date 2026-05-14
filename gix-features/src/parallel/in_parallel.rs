use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicUsize, Ordering};

use crate::parallel::{Reduce, num_threads};

/// A scope to start threads within.
pub type Scope<'scope, 'env> = std::thread::Scope<'scope, 'env>;

/// Runs `left` and `right` in parallel, returning their output when both are done.
pub fn join<O1: Send, O2: Send>(left: impl FnOnce() -> O1 + Send, right: impl FnOnce() -> O2 + Send) -> (O1, O2) {
    std::thread::scope(|s| {
        let left = std::thread::Builder::new()
            .name("gitoxide.join.left".into())
            .spawn_scoped(s, left)
            .expect("valid name");
        let right = std::thread::Builder::new()
            .name("gitoxide.join.right".into())
            .spawn_scoped(s, right)
            .expect("valid name");
        (left.join().unwrap(), right.join().unwrap())
    })
}

/// Runs `f` with a scope to be used for spawning threads that will not outlive the function call.
/// That way it's possible to handle threads without needing the 'static lifetime for data they interact with.
///
/// Note that the threads should not rely on actual parallelism as threading might be turned off entirely, hence should not
/// connect each other with channels as deadlock would occur in single-threaded mode.
pub fn threads<'env, F, R>(f: F) -> R
where
    F: for<'scope> FnOnce(&'scope std::thread::Scope<'scope, 'env>) -> R,
{
    std::thread::scope(f)
}

/// Create a builder for threads which allows them to be spawned into a scope and configured prior to spawning.
pub fn build_thread() -> std::thread::Builder {
    std::thread::Builder::new()
}

/// Read items from `input` and `consume` them in multiple threads,
/// whose output is collected by a `reducer`. Its task is to
/// aggregate these outputs into the final result returned by this function, with the benefit of not having to be thread-safe.
///
/// * if `thread_limit` is `Some`, then the given number of threads will be used. If `None`, all logical cores will be used.
/// * `new_thread_state(thread_number) -> State` produces thread-local state once per thread to be passed to `consume`
/// * `consume(Item, &mut State) -> Output` produces an output given an input obtained by `input` along with mutable state initially
///   created by `new_thread_state(…)`.
/// * For `reducer`, see the [`Reduce`] trait
pub fn in_parallel<I, S, O, R>(
    input: impl Iterator<Item = I> + Send,
    thread_limit: Option<usize>,
    new_thread_state: impl FnOnce(usize) -> S + Send + Clone,
    consume: impl FnMut(I, &mut S) -> O + Send + Clone,
    mut reducer: R,
) -> Result<<R as Reduce>::Output, <R as Reduce>::Error>
where
    R: Reduce<Input = O>,
    I: Send,
    O: Send,
{
    let num_threads = num_threads(thread_limit);
    std::thread::scope(move |s| {
        let receive_result = {
            let (send_input, receive_input) = crossbeam_channel::bounded::<I>(num_threads);
            let (send_result, receive_result) = crossbeam_channel::bounded::<O>(num_threads);
            for thread_id in 0..num_threads {
                std::thread::Builder::new()
                    .name(format!("gitoxide.in_parallel.produce.{thread_id}"))
                    .spawn_scoped(s, {
                        let send_result = send_result.clone();
                        let receive_input = receive_input.clone();
                        let new_thread_state = new_thread_state.clone();
                        let mut consume = consume.clone();
                        move || {
                            let mut state = new_thread_state(thread_id);
                            for item in receive_input {
                                if send_result.send(consume(item, &mut state)).is_err() {
                                    break;
                                }
                            }
                        }
                    })
                    .expect("valid name");
            }
            std::thread::Builder::new()
                .name("gitoxide.in_parallel.feed".into())
                .spawn_scoped(s, move || {
                    for item in input {
                        if send_input.send(item).is_err() {
                            break;
                        }
                    }
                })
                .expect("valid name");
            receive_result
        };

        for item in receive_result {
            drop(reducer.feed(item)?);
        }
        reducer.finalize()
    })
}

/// Read items from `input` and `consume` them in multiple threads,
/// whose output is collected by a `reducer`. Its task is to
/// aggregate these outputs into the final result returned by this function with the benefit of not having to be thread-safe.
/// Call `finalize` to finish the computation, once per thread, if there was no error sending results earlier.
///
/// * if `thread_limit` is `Some`, then the given number of threads will be used. If `None`, all logical cores will be used.
/// * `new_thread_state(thread_number) -> State` produces thread-local state once per thread to be passed to `consume`
/// * `consume(Item, &mut State) -> Output` produces an output given an input obtained by `input` along with mutable state initially
///   created by `new_thread_state(…)`.
/// * `finalize(State) -> Output` is called to potentially process remaining work that was placed in `State`.
/// * For `reducer`, see the [`Reduce`] trait
pub fn in_parallel_with_finalize<I, S, O, R>(
    input: impl Iterator<Item = I> + Send,
    thread_limit: Option<usize>,
    new_thread_state: impl FnOnce(usize) -> S + Send + Clone,
    consume: impl FnMut(I, &mut S) -> O + Send + Clone,
    finalize: impl FnOnce(S) -> O + Send + Clone,
    mut reducer: R,
) -> Result<<R as Reduce>::Output, <R as Reduce>::Error>
where
    R: Reduce<Input = O>,
    I: Send,
    O: Send,
{
    let num_threads = num_threads(thread_limit);
    std::thread::scope(move |s| {
        let receive_result = {
            let (send_input, receive_input) = crossbeam_channel::bounded::<I>(num_threads);
            let (send_result, receive_result) = crossbeam_channel::bounded::<O>(num_threads);
            for thread_id in 0..num_threads {
                std::thread::Builder::new()
                    .name(format!("gitoxide.in_parallel.produce.{thread_id}"))
                    .spawn_scoped(s, {
                        let send_result = send_result.clone();
                        let receive_input = receive_input.clone();
                        let new_thread_state = new_thread_state.clone();
                        let mut consume = consume.clone();
                        let finalize = finalize.clone();
                        move || {
                            let mut state = new_thread_state(thread_id);
                            let mut can_send = true;
                            for item in receive_input {
                                if send_result.send(consume(item, &mut state)).is_err() {
                                    can_send = false;
                                    break;
                                }
                            }
                            if can_send {
                                send_result.send(finalize(state)).ok();
                            }
                        }
                    })
                    .expect("valid name");
            }
            std::thread::Builder::new()
                .name("gitoxide.in_parallel.feed".into())
                .spawn_scoped(s, move || {
                    for item in input {
                        if send_input.send(item).is_err() {
                            break;
                        }
                    }
                })
                .expect("valid name");
            receive_result
        };

        for item in receive_result {
            drop(reducer.feed(item)?);
        }
        reducer.finalize()
    })
}

/// An experiment to have fine-grained per-item parallelization with built-in aggregation via thread state.
/// This is only good for operations where near-random access isn't detrimental, so it's not usually great
/// for file-io as it won't make use of sorted inputs well.
/// Note that `periodic` is not guaranteed to be called in case other threads come up first and finish too fast.
/// `consume(&mut item, &mut stat, &Scope, &threads_available, &should_interrupt)` is called for performing the actual computation.
/// Note that `threads_available` should be decremented to start a thread that can steal your own work (as stored in `item`),
/// which allows callees to implement their own work-stealing in case the work is distributed unevenly.
/// Work stealing should only start after having processed at least one item to give all threads naturally operating on the slice
/// some time to start. Starting threads while slice-workers are still starting up would lead to over-allocation of threads,
/// which is why the number of threads left may turn negative. Once threads are started and stopped, be sure to adjust
/// the thread-count accordingly.
// TODO: better docs
pub fn in_parallel_with_slice<I, S, R, E>(
    input: &mut [I],
    thread_limit: Option<usize>,
    new_thread_state: impl FnOnce(usize) -> S + Send + Clone,
    consume: impl FnMut(&mut I, &mut S, &AtomicIsize, &AtomicBool) -> Result<(), E> + Send + Clone,
    mut periodic: impl FnMut() -> Option<std::time::Duration> + Send,
    state_to_rval: impl FnOnce(S) -> R + Send + Clone,
) -> Result<Vec<R>, E>
where
    I: Send,
    E: Send,
    R: Send,
{
    let num_threads = num_threads(thread_limit);
    let mut results = Vec::with_capacity(num_threads);
    let stop_everything = &AtomicBool::default();
    let index = &AtomicUsize::default();
    let threads_left = &AtomicIsize::new(num_threads as isize);

    std::thread::scope({
        move |s| {
            std::thread::Builder::new()
                .name("gitoxide.in_parallel_with_slice.watch-interrupts".into())
                .spawn_scoped(s, {
                    move || loop {
                        if stop_everything.load(Ordering::Relaxed) {
                            break;
                        }

                        match periodic() {
                            Some(duration) => std::thread::sleep(duration),
                            None => {
                                stop_everything.store(true, Ordering::Relaxed);
                                break;
                            }
                        }
                    }
                })
                .expect("valid name");

            let input_len = input.len();
            struct Input<I>(*mut I)
            where
                I: Send;

            // SAFETY: I is Send, and we only use the pointer for creating new
            // pointers (within the input slice) from the threads.
            #[allow(unsafe_code)]
            unsafe impl<I> Send for Input<I> where I: Send {}

            let threads: Vec<_> = (0..num_threads)
                .map(|thread_id| {
                    std::thread::Builder::new()
                        .name(format!("gitoxide.in_parallel_with_slice.produce.{thread_id}"))
                        .spawn_scoped(s, {
                            let new_thread_state = new_thread_state.clone();
                            let state_to_rval = state_to_rval.clone();
                            let mut consume = consume.clone();
                            let input = Input(input.as_mut_ptr());
                            move || {
                                let _ = &input;
                                threads_left.fetch_sub(1, Ordering::SeqCst);
                                let mut state = new_thread_state(thread_id);
                                let res = (|| {
                                    while let Ok(input_index) =
                                        index.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| {
                                            (x < input_len).then_some(x + 1)
                                        })
                                    {
                                        if stop_everything.load(Ordering::Relaxed) {
                                            break;
                                        }
                                        // SAFETY: our atomic counter for `input_index` is only ever incremented, yielding
                                        //         each item exactly once.
                                        let item = {
                                            #[allow(unsafe_code)]
                                            unsafe {
                                                &mut *input.0.add(input_index)
                                            }
                                        };
                                        if let Err(err) = consume(item, &mut state, threads_left, stop_everything) {
                                            stop_everything.store(true, Ordering::Relaxed);
                                            return Err(err);
                                        }
                                    }
                                    Ok(state_to_rval(state))
                                })();
                                threads_left.fetch_add(1, Ordering::SeqCst);
                                res
                            }
                        })
                        .expect("valid name")
                })
                .collect();
            for thread in threads {
                match thread.join() {
                    Ok(res) => {
                        results.push(res?);
                    }
                    Err(err) => {
                        // a panic happened, stop the world gracefully (even though we panic later)
                        stop_everything.store(true, Ordering::Relaxed);
                        std::panic::resume_unwind(err);
                    }
                }
            }

            stop_everything.store(true, Ordering::Relaxed);
            Ok(results)
        }
    })
}
