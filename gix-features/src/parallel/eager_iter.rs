/// Evaluate any iterator in their own thread.
///
/// This is particularly useful if the wrapped iterator performs IO and/or heavy computations.
/// Use [`EagerIter::new()`] for instantiation.
pub struct EagerIter<I: Iterator> {
    receiver: std::sync::mpsc::Receiver<Vec<I::Item>>,
    chunk: Option<std::vec::IntoIter<I::Item>>,
    size_hint: (usize, Option<usize>),
}

impl<I> EagerIter<I>
where
    I: Iterator + Send + 'static,
    <I as Iterator>::Item: Send,
{
    /// Return a new `EagerIter` which evaluates `iter` in its own thread,
    /// with a given `chunk_size` allowing a maximum `chunks_in_flight`.
    ///
    /// * `chunk_size` describes how many items returned by `iter` will be a single item of this `EagerIter`.
    ///   This helps to reduce the overhead imposed by transferring many small items.
    ///   If this number is 1, each item will become a single chunk. 0 is invalid.
    /// * `chunks_in_flight` describes how many chunks can be kept in memory in case the consumer of the `EagerIter`s items
    ///   isn't consuming them fast enough. Setting this number to 0 effectively turns off any caching, but blocks `EagerIter`
    ///   if its items aren't consumed fast enough.
    pub fn new(iter: I, chunk_size: usize, chunks_in_flight: usize) -> Self {
        let (sender, receiver) = std::sync::mpsc::sync_channel(chunks_in_flight);
        let size_hint = iter.size_hint();
        assert!(chunk_size > 0, "non-zero chunk size is needed");

        std::thread::spawn(move || {
            let mut out = Vec::with_capacity(chunk_size);
            for item in iter {
                out.push(item);
                if out.len() == chunk_size {
                    if sender.send(out).is_err() {
                        return;
                    }
                    out = Vec::with_capacity(chunk_size);
                }
            }
            if !out.is_empty() {
                sender.send(out).ok();
            }
        });
        EagerIter {
            receiver,
            chunk: None,
            size_hint,
        }
    }

    fn fill_buf_and_pop(&mut self) -> Option<I::Item> {
        self.chunk = self.receiver.recv().ok().map(|v| {
            assert!(!v.is_empty());
            v.into_iter()
        });
        self.chunk.as_mut().and_then(Iterator::next)
    }
}

impl<I> Iterator for EagerIter<I>
where
    I: Iterator + Send + 'static,
    <I as Iterator>::Item: Send,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self.chunk.as_mut() {
            Some(chunk) => chunk.next().or_else(|| self.fill_buf_and_pop()),
            None => self.fill_buf_and_pop(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.size_hint
    }
}

/// An conditional `EagerIter`, which may become a just-in-time iterator running in the main thread depending on a condition.
pub enum EagerIterIf<I: Iterator> {
    /// A separate thread will eagerly evaluate iterator `I`.
    Eager(EagerIter<I>),
    /// The current thread evaluates `I`.
    OnDemand(I),
}

impl<I> EagerIterIf<I>
where
    I: Iterator + Send + 'static,
    <I as Iterator>::Item: Send,
{
    /// Return a new `EagerIterIf` if `condition()` returns true.
    ///
    /// For all other parameters, please see [`EagerIter::new()`].
    pub fn new(condition: impl FnOnce() -> bool, iter: I, chunk_size: usize, chunks_in_flight: usize) -> Self {
        if condition() {
            EagerIterIf::Eager(EagerIter::new(iter, chunk_size, chunks_in_flight))
        } else {
            EagerIterIf::OnDemand(iter)
        }
    }
}
impl<I> Iterator for EagerIterIf<I>
where
    I: Iterator + Send + 'static,
    <I as Iterator>::Item: Send,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            EagerIterIf::OnDemand(i) => i.next(),
            EagerIterIf::Eager(i) => i.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            EagerIterIf::OnDemand(i) => i.size_hint(),
            EagerIterIf::Eager(i) => i.size_hint(),
        }
    }
}
