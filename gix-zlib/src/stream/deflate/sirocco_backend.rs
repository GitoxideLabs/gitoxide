//! The writer's backend with the `sirocco` feature.

use std::cell::RefCell;
use std::io;

use gix_error::{ErrorExt, message};
use sirocco::deflate::{Encoder, Flush, Level};

use crate::Compression;

/// The most input buffered before compressing. Objects up to this size are compressed in one
/// call, which lets `sirocco` size its tables to them; larger ones stream through a buffer this
/// large.
const MAX_INPUT: usize = 1 << 20;
const _: () = assert!(MAX_INPUT >= 2 * sirocco::deflate::MIN_BUFFER_LEN);
/// The most input capacity kept with a pooled encoder, so that idle pools stay small.
const MAX_POOLED_INPUT: usize = 64 * 1024;
const OUT_LEN: usize = 4096 * 8;
/// The encoders kept for reuse per thread.
const POOL_LEN: usize = 2;

/// An encoder and its buffers. A writer is made per object, so these are reused across writers
/// on a thread: an encoder's tables are about 340 KiB.
struct State {
    encoder: Encoder,
    input: Vec<u8>,
    out: Vec<u8>,
}

thread_local! {
    static POOL: RefCell<Vec<State>> = const { RefCell::new(Vec::new()) };
}

impl State {
    /// A state for `level` from this thread's pool, or a new one.
    fn take(level: Level) -> State {
        let pooled = POOL.with(|pool| {
            let mut pool = pool.borrow_mut();
            let i = pool.iter().position(|s| s.encoder.level() == level)?;
            Some(pool.swap_remove(i))
        });
        pooled.unwrap_or_else(|| State {
            encoder: Encoder::new(sirocco::Format::Zlib, level),
            input: Vec::new(),
            out: vec![0; OUT_LEN],
        })
    }

    /// Return the state to this thread's pool, if it has room.
    fn give_back(mut self) {
        self.encoder.reset();
        self.input.clear();
        if self.input.capacity() > MAX_POOLED_INPUT {
            self.input = Vec::new();
        }
        // The pool is gone if the thread is exiting.
        let _ = POOL.try_with(|pool| {
            let mut pool = pool.borrow_mut();
            if pool.len() < POOL_LEN {
                pool.push(self);
            }
        });
    }
}

pub(super) struct Backend {
    /// Only `None` while dropping.
    state: Option<State>,
    /// The stream is finished: it takes no more input until reset.
    finished: bool,
}

impl Drop for Backend {
    fn drop(&mut self) {
        if let Some(state) = self.state.take() {
            state.give_back();
        }
    }
}

impl Backend {
    pub(super) fn new(level: Compression) -> Self {
        // `Compression` is 0 to 9, as `sirocco`'s levels.
        let level = u8::try_from(level.level().clamp(0, 9)).map_or(Level::DEFAULT, Level::new);
        Backend {
            state: Some(State::take(level)),
            finished: false,
        }
    }

    /// The state, which is only gone while dropping.
    fn state(&mut self) -> io::Result<&mut State> {
        self.state
            .as_mut()
            .ok_or_else(|| io::Error::other(message("the compressor is gone").raise_erased()))
    }

    pub(super) fn reset(&mut self) {
        if let Some(state) = self.state.as_mut() {
            state.encoder.reset();
            state.input.clear();
        }
        self.finished = false;
    }

    pub(super) fn write(&mut self, inner: &mut impl io::Write, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.finished {
            // As `zlib-rs` does.
            return Err(io::Error::other(message("stream error").raise_erased()));
        }
        if self.state()?.input.len() == MAX_INPUT {
            // Compress what is buffered and keep only what later input may refer to.
            self.run(inner, Flush::None)?;
            let state = self.state()?;
            let dropped = state.encoder.make_room(&mut state.input);
            state.input.truncate(MAX_INPUT - dropped);
        }
        let input = &mut self.state()?.input;
        let n = buf.len().min(MAX_INPUT - input.len());
        input.extend_from_slice(&buf[..n]);
        Ok(n)
    }

    pub(super) fn finish(&mut self, inner: &mut impl io::Write) -> io::Result<()> {
        if self.finished {
            return Ok(());
        }
        self.run(inner, Flush::Finish)?;
        self.finished = true;
        Ok(())
    }

    /// Compress the buffered input with `flush`, writing the output to `inner`.
    fn run(&mut self, inner: &mut impl io::Write, flush: Flush) -> io::Result<()> {
        let state = self.state()?;
        loop {
            let progress = state.encoder.compress(&state.input, &mut state.out, flush);
            let full = progress.full;
            inner.write_all(progress.output)?;
            if !full {
                return Ok(());
            }
        }
    }
}
