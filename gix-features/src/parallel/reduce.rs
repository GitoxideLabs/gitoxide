#[cfg(feature = "parallel")]
mod stepped {
    use crate::parallel::num_threads;

    /// An iterator adaptor to allow running computations using [`in_parallel()`][crate::parallel::in_parallel()] in a step-wise manner, see the [module docs][crate::parallel]
    /// for details.
    pub struct Stepwise<Reduce: super::Reduce> {
        /// This field is first to assure it's dropped first and cause threads that are dropped next to stop their loops
        /// as sending results fails when the receiver is dropped.
        receive_result: std::sync::mpsc::Receiver<Reduce::Input>,
        /// `join()` will be called on these guards to assure every thread tries to send through a closed channel. When
        /// that happens, they break out of their loops.
        threads: Vec<std::thread::JoinHandle<()>>,
        /// The reducer is called only in the thread using the iterator, dropping it has no side effects.
        reducer: Option<Reduce>,
    }

    impl<Reduce: super::Reduce> Drop for Stepwise<Reduce> {
        fn drop(&mut self) {
            let (_, sink) = std::sync::mpsc::channel();
            drop(std::mem::replace(&mut self.receive_result, sink));

            let mut last_err = None;
            for handle in std::mem::take(&mut self.threads) {
                if let Err(err) = handle.join() {
                    last_err = Some(err);
                }
            }
            if let Some(thread_err) = last_err {
                std::panic::resume_unwind(thread_err);
            }
        }
    }

    impl<Reduce: super::Reduce> Stepwise<Reduce> {
        /// Instantiate a new iterator and start working in threads.
        /// For a description of parameters, see [`in_parallel()`][crate::parallel::in_parallel()].
        pub fn new<InputIter, ThreadStateFn, ConsumeFn, I, O, S>(
            input: InputIter,
            thread_limit: Option<usize>,
            new_thread_state: ThreadStateFn,
            consume: ConsumeFn,
            reducer: Reduce,
        ) -> Self
        where
            InputIter: Iterator<Item = I> + Send + 'static,
            ThreadStateFn: Fn(usize) -> S + Send + Clone + 'static,
            ConsumeFn: Fn(I, &mut S) -> O + Send + Clone + 'static,
            Reduce: super::Reduce<Input = O> + 'static,
            I: Send + 'static,
            O: Send + 'static,
        {
            let num_threads = num_threads(thread_limit);
            let mut threads = Vec::with_capacity(num_threads + 1);
            let receive_result = {
                let (send_input, receive_input) = crossbeam_channel::bounded::<I>(num_threads);
                let (send_result, receive_result) = std::sync::mpsc::sync_channel::<O>(num_threads);
                for thread_id in 0..num_threads {
                    let handle = std::thread::spawn({
                        let send_result = send_result.clone();
                        let receive_input = receive_input.clone();
                        let new_thread_state = new_thread_state.clone();
                        let consume = consume.clone();
                        move || {
                            let mut state = new_thread_state(thread_id);
                            for item in receive_input {
                                if send_result.send(consume(item, &mut state)).is_err() {
                                    break;
                                }
                            }
                        }
                    });
                    threads.push(handle);
                }
                threads.push(std::thread::spawn(move || {
                    for item in input {
                        if send_input.send(item).is_err() {
                            break;
                        }
                    }
                }));
                receive_result
            };
            Stepwise {
                threads,
                receive_result,
                reducer: Some(reducer),
            }
        }

        /// Consume the iterator by finishing its iteration and calling [`Reduce::finalize()`][crate::parallel::Reduce::finalize()].
        pub fn finalize(mut self) -> Result<Reduce::Output, Reduce::Error> {
            for value in self.by_ref() {
                drop(value?);
            }
            self.reducer
                .take()
                .expect("this is the last call before consumption")
                .finalize()
        }
    }

    impl<Reduce: super::Reduce> Iterator for Stepwise<Reduce> {
        type Item = Result<Reduce::FeedProduce, Reduce::Error>;

        fn next(&mut self) -> Option<<Self as Iterator>::Item> {
            self.receive_result
                .recv()
                .ok()
                .and_then(|input| self.reducer.as_mut().map(|r| r.feed(input)))
        }
    }

    impl<R: super::Reduce> super::Finalize for Stepwise<R> {
        type Reduce = R;

        fn finalize(
            self,
        ) -> Result<
            <<Self as super::Finalize>::Reduce as super::Reduce>::Output,
            <<Self as super::Finalize>::Reduce as super::Reduce>::Error,
        > {
            Stepwise::finalize(self)
        }
    }
}

#[cfg(not(feature = "parallel"))]
mod stepped {
    /// An iterator adaptor to allow running computations using [`in_parallel()`][crate::parallel::in_parallel()] in a step-wise manner, see the [module docs][crate::parallel]
    /// for details.
    pub struct Stepwise<InputIter, ConsumeFn, ThreadState, Reduce> {
        input: InputIter,
        consume: ConsumeFn,
        thread_state: ThreadState,
        reducer: Reduce,
    }

    impl<InputIter, ConsumeFn, Reduce, I, O, S> Stepwise<InputIter, ConsumeFn, S, Reduce>
    where
        InputIter: Iterator<Item = I>,
        ConsumeFn: Fn(I, &mut S) -> O,
        Reduce: super::Reduce<Input = O>,
    {
        /// Instantiate a new iterator.
        /// For a description of parameters, see [`in_parallel()`][crate::parallel::in_parallel()].
        pub fn new<ThreadStateFn>(
            input: InputIter,
            _thread_limit: Option<usize>,
            new_thread_state: ThreadStateFn,
            consume: ConsumeFn,
            reducer: Reduce,
        ) -> Self
        where
            ThreadStateFn: Fn(usize) -> S,
        {
            Stepwise {
                input,
                consume,
                thread_state: new_thread_state(0),
                reducer,
            }
        }

        /// Consume the iterator by finishing its iteration and calling [`Reduce::finalize()`][crate::parallel::Reduce::finalize()].
        pub fn finalize(mut self) -> Result<Reduce::Output, Reduce::Error> {
            for value in self.by_ref() {
                drop(value?);
            }
            self.reducer.finalize()
        }
    }

    impl<InputIter, ConsumeFn, ThreadState, Reduce, I, O> Iterator for Stepwise<InputIter, ConsumeFn, ThreadState, Reduce>
    where
        InputIter: Iterator<Item = I>,
        ConsumeFn: Fn(I, &mut ThreadState) -> O,
        Reduce: super::Reduce<Input = O>,
    {
        type Item = Result<Reduce::FeedProduce, Reduce::Error>;

        fn next(&mut self) -> Option<<Self as Iterator>::Item> {
            self.input
                .next()
                .map(|input| self.reducer.feed((self.consume)(input, &mut self.thread_state)))
        }
    }

    impl<InputIter, ConsumeFn, R, I, O, S> super::Finalize for Stepwise<InputIter, ConsumeFn, S, R>
    where
        InputIter: Iterator<Item = I>,
        ConsumeFn: Fn(I, &mut S) -> O,
        R: super::Reduce<Input = O>,
    {
        type Reduce = R;

        fn finalize(
            self,
        ) -> Result<
            <<Self as super::Finalize>::Reduce as super::Reduce>::Output,
            <<Self as super::Finalize>::Reduce as super::Reduce>::Error,
        > {
            Stepwise::finalize(self)
        }
    }
}

use std::marker::PhantomData;

pub use stepped::Stepwise;

/// An trait for aggregating items commonly produced in threads into a single result, without itself
/// needing to be thread safe.
pub trait Reduce {
    /// The type fed to the reducer in the [`feed()`][Reduce::feed()] method.
    ///
    /// It's produced by a function that may run on multiple threads.
    type Input;
    /// The type produced in Ok(…) by [`feed()`][Reduce::feed()].
    /// Most reducers by nature use `()` here as the value is in the aggregation.
    /// However, some may use it to collect statistics only and return their Input
    /// in some form as a result here for [`Stepwise`] to be useful.
    type FeedProduce;
    /// The type produced once by the [`finalize()`][Reduce::finalize()] method.
    ///
    /// For traditional reducers, this is the value produced by the entire operation.
    /// For those made for step-wise iteration this may be aggregated statistics.
    type Output;
    /// The error type to use for all methods of this trait.
    type Error;
    /// Called each time a new `item` was produced in order to aggregate it into the final result.
    ///
    /// If an `Error` is returned, the entire operation will be stopped.
    fn feed(&mut self, item: Self::Input) -> Result<Self::FeedProduce, Self::Error>;
    /// Called once for all items that were passed to `feed()`, producing the final `Output` of the operation or an `Error`.
    fn finalize(self) -> Result<Self::Output, Self::Error>;
}

/// An identity reducer for those who want to use [`Stepwise`] or [`in_parallel()`][crate::parallel::in_parallel()]
/// without the use of non-threaded reduction of products created in threads.
pub struct IdentityWithResult<Input, Error> {
    _input: PhantomData<Input>,
    _error: PhantomData<Error>,
}

impl<Input, Error> Default for IdentityWithResult<Input, Error> {
    fn default() -> Self {
        IdentityWithResult {
            _input: Default::default(),
            _error: Default::default(),
        }
    }
}

impl<Input, Error> Reduce for IdentityWithResult<Input, Error> {
    type Input = Result<Input, Self::Error>;
    type FeedProduce = Input;
    type Output = ();
    type Error = Error;

    fn feed(&mut self, item: Self::Input) -> Result<Self::FeedProduce, Self::Error> {
        item
    }

    fn finalize(self) -> Result<Self::Output, Self::Error> {
        Ok(())
    }
}

/// A trait reflecting the `finalize()` method of [`Reduce`] implementations
pub trait Finalize {
    /// An implementation of [`Reduce`]
    type Reduce: self::Reduce;

    /// Similar to the [`Reduce::finalize()`] method
    fn finalize(
        self,
    ) -> Result<<<Self as Finalize>::Reduce as self::Reduce>::Output, <<Self as Finalize>::Reduce as self::Reduce>::Error>;
}
