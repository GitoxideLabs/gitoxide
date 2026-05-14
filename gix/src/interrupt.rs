//! Process-global interrupt handling
//!
//! This module contains facilities to globally request an interrupt, which will cause supporting computations to
//! abort once it is observed.
//! Such checks for interrupts are provided in custom implementations of various traits to transparently add interrupt
//! support to methods who wouldn't otherwise by injecting it. see [`Read`].

#[cfg(feature = "interrupt")]
mod init {
    use std::{
        io,
        sync::atomic::{AtomicUsize, Ordering},
    };

    static DEREGISTER_COUNT: AtomicUsize = AtomicUsize::new(0);
    static REGISTERED_HOOKS: std::sync::LazyLock<parking_lot::Mutex<Vec<(i32, signal_hook::SigId)>>> =
        std::sync::LazyLock::new(Default::default);
    static DEFAULT_BEHAVIOUR_HOOKS: std::sync::LazyLock<parking_lot::Mutex<Vec<signal_hook::SigId>>> =
        std::sync::LazyLock::new(Default::default);

    /// A type to help deregistering hooks registered with [`init_handler`](super::init_handler());
    #[derive(Default)]
    pub struct Deregister {
        do_reset: bool,
    }
    pub struct AutoDeregister(Deregister);

    impl Deregister {
        /// Remove all previously registered handlers, and assure the default behaviour is reinstated, if this is the last available instance.
        ///
        /// Note that only the instantiation of the default behaviour can fail.
        pub fn deregister(self) -> std::io::Result<()> {
            let mut hooks = REGISTERED_HOOKS.lock();
            let count = DEREGISTER_COUNT.fetch_sub(1, Ordering::SeqCst);
            if count > 1 || hooks.is_empty() {
                return Ok(());
            }
            if self.do_reset {
                super::reset();
            }
            for (_, hook_id) in hooks.iter() {
                signal_hook::low_level::unregister(*hook_id);
            }

            let hooks = hooks.drain(..);
            let mut default_hooks = DEFAULT_BEHAVIOUR_HOOKS.lock();
            // Even if dropped, `drain(..)` clears the vec which is a must.
            for (sig, _) in hooks {
                // # SAFETY
                // * we only register a handler that is specifically designed to run in this environment.
                #[allow(unsafe_code)]
                unsafe {
                    default_hooks.push(signal_hook::low_level::register(sig, move || {
                        signal_hook::low_level::emulate_default_handler(sig).ok();
                    })?);
                }
            }
            Ok(())
        }

        /// If called with `toggle` being `true`, when actually deregistering, we will also reset the trigger by
        /// calling [`reset()`](super::reset()).
        pub fn with_reset(mut self, toggle: bool) -> Self {
            self.do_reset = toggle;
            self
        }

        /// Return a type that deregisters all installed signal handlers on drop.
        pub fn auto_deregister(self) -> AutoDeregister {
            AutoDeregister(self)
        }
    }

    impl Drop for AutoDeregister {
        fn drop(&mut self) {
            std::mem::take(&mut self.0).deregister().ok();
        }
    }

    /// Initialize a signal handler to listen to SIGINT and SIGTERM and trigger our [`trigger()`](super::trigger()) that way.
    /// Also trigger `interrupt()` which promises to never use a Mutex, allocate or deallocate, or do anything else that's blocking.
    /// Use `grace_count` to determine how often the termination signal can be received before it's terminal, e.g. 1 would only terminate
    /// the application the second time the signal is received.
    /// Note that only the `grace_count` and `interrupt` of the first call are effective, all others will be ignored.
    ///
    /// Use the returned `Deregister` type to explicitly deregister hooks, or to do so automatically.
    ///
    /// # Note
    ///
    /// It will abort the process on second press and won't inform the user about this behaviour either as we are unable to do so without
    /// deadlocking even when trying to write to stderr directly.
    ///
    /// SAFETY: `interrupt()` will be called from a signal handler. See [`signal_hook::low_level::register()`] for details about.
    #[allow(unsafe_code, clippy::missing_safety_doc)]
    pub unsafe fn init_handler(
        grace_count: usize,
        interrupt: impl Fn() + Send + Sync + Clone + 'static,
    ) -> io::Result<Deregister> {
        let prev_count = DEREGISTER_COUNT.fetch_add(1, Ordering::SeqCst);
        if prev_count != 0 {
            // Try to obtain the lock before we return just to wait for the signals to actually be registered.
            let _guard = REGISTERED_HOOKS.lock();
            return Ok(Deregister::default());
        }
        let mut guard = REGISTERED_HOOKS.lock();
        if !guard.is_empty() {
            return Ok(Deregister::default());
        }

        let mut hooks = Vec::with_capacity(signal_hook::consts::TERM_SIGNALS.len());
        for sig in signal_hook::consts::TERM_SIGNALS {
            // # SAFETY
            // * we only set atomics or call functions that do
            // * there is no use of the heap
            let interrupt = interrupt.clone();
            let action = move || {
                static INTERRUPT_COUNT: AtomicUsize = AtomicUsize::new(0);
                if !super::is_triggered() {
                    INTERRUPT_COUNT.store(0, Ordering::SeqCst);
                }
                let msg_idx = INTERRUPT_COUNT.fetch_add(1, Ordering::SeqCst);
                if msg_idx == grace_count {
                    gix_tempfile::registry::cleanup_tempfiles_signal_safe();
                    signal_hook::low_level::emulate_default_handler(*sig).ok();
                }
                interrupt();
                super::trigger();
            };
            #[allow(unsafe_code)]
            unsafe {
                let hook_id = signal_hook::low_level::register(*sig, action)?;
                hooks.push((*sig, hook_id));
            }
        }
        for hook_id in DEFAULT_BEHAVIOUR_HOOKS.lock().drain(..) {
            signal_hook::low_level::unregister(hook_id);
        }

        // This means that they won't setup a handler allowing us to call them right before we actually abort.
        gix_tempfile::signal::setup(gix_tempfile::signal::handler::Mode::None);

        *guard = hooks;
        Ok(Deregister::default())
    }
}
use std::{
    io,
    sync::atomic::{AtomicBool, Ordering},
};

#[cfg(feature = "interrupt")]
pub use init::{Deregister, init_handler};

/// A wrapper for an inner iterator which will check for interruptions on each iteration.
pub struct Iter<I, EFN> {
    /// The actual iterator to yield elements from.
    inner: gix_features::interrupt::IterWithErr<'static, I, EFN>,
}

impl<I, EFN, E> Iter<I, EFN>
where
    I: Iterator,
    EFN: FnOnce() -> E,
{
    /// Create a new iterator over `inner` which checks for interruptions on each iteration and calls `make_err()` to
    /// signal an interruption happened, causing no further items to be iterated from that point on.
    pub fn new(inner: I, make_err: EFN) -> Self {
        Iter {
            inner: gix_features::interrupt::IterWithErr::new(inner, make_err, &IS_INTERRUPTED),
        }
    }

    /// Return the inner iterator
    pub fn into_inner(self) -> I {
        self.inner.inner
    }

    /// Return the inner iterator as reference
    pub fn inner(&self) -> &I {
        &self.inner.inner
    }
}

impl<I, EFN, E> Iterator for Iter<I, EFN>
where
    I: Iterator,
    EFN: FnOnce() -> E,
{
    type Item = Result<I::Item, E>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

/// A wrapper for implementers of [`std::io::Read`] or [`std::io::BufRead`] with interrupt support.
///
/// It fails a [read][`std::io::Read::read`] while an interrupt was requested.
pub struct Read<R> {
    /// The actual implementor of [`std::io::Read`] to which interrupt support will be added.
    inner: gix_features::interrupt::Read<'static, R>,
}

impl<R> Read<R>
where
    R: io::Read,
{
    /// Create a new interruptible reader from `read`.
    pub fn new(read: R) -> Self {
        Read {
            inner: gix_features::interrupt::Read {
                inner: read,
                should_interrupt: &IS_INTERRUPTED,
            },
        }
    }

    /// Return the inner reader
    pub fn into_inner(self) -> R {
        self.inner.inner
    }
}

impl<R> io::Read for Read<R>
where
    R: io::Read,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<R> io::BufRead for Read<R>
where
    R: io::BufRead,
{
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.inner.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.inner.consume(amt);
    }
}

/// The flag behind all utility functions in this module.
pub static IS_INTERRUPTED: AtomicBool = AtomicBool::new(false);

/// Returns true if an interrupt is requested.
pub fn is_triggered() -> bool {
    IS_INTERRUPTED.load(Ordering::Relaxed)
}

/// Trigger an interrupt, signalling to those checking for [`is_triggered()`] to stop what they are doing.
pub fn trigger() {
    IS_INTERRUPTED.store(true, Ordering::SeqCst);
}

/// Sets the interrupt request to false, thus allowing those checking for [`is_triggered()`] to proceed.
pub fn reset() {
    IS_INTERRUPTED.store(false, Ordering::SeqCst);
}
