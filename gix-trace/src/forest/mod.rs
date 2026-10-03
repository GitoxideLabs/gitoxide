//! Collect related tracing spans and events into trees, then process each completed root.
//!
//! [`ForestLayer`](crate::ForestLayer) works with a [`tracing_subscriber::Registry`]. Its default processor
//! pretty-prints trees to stdout; [`processor::from_fn`] can instead collect or forward
//! them, and [`Printer`] combines a [`Formatter`] with a writer. Processing is synchronous
//! on the thread that closes a root span or emits an event without a parent.
//!
//! Install only one `ForestLayer` per registry: its per-span state occupies one shared
//! extension slot, regardless of the layer's processor and tag types. For multiple
//! output destinations, collect once and send the completed trees to the desired
//! formatting or storage sinks.
//!
//! This module ports the synchronous core of
//! [`tracing-forest` 0.2.0](https://github.com/QnnOkabayashi/tracing-forest), under its MIT
//! license. It does not require that crate's procedural macros or an asynchronous runtime.
//! Enable `forest` for collection and plain formatting, or `forest-ansi` for terminal colors.
//!
//! # Recording fields and filtering output
//!
//! Span fields can be populated or updated after creation with [`tracing::Span::record`]
//! or [`crate::Span::record`]. Declare fields whose values are not yet known with
//! [`tracing::field::Empty`]. A completed tree retains the most recently recorded value
//! for each field key.
//!
//! [`printer::Pretty::with_max_level`] limits the levels displayed by a printer while
//! retaining the complete collected tree for other uses:
//!
//! ```
//! use gix_trace::forest::{Printer, printer::Pretty};
//! use tracing_subscriber::filter::LevelFilter;
//!
//! let printer = Printer::new().formatter(Pretty.with_max_level(LevelFilter::INFO));
//! ```
//!
//! Filtering happens before rendering. Visible descendants of a hidden span are
//! promoted to their nearest visible ancestor. Durations and percentages still refer
//! to the original complete tree and its root, including time in hidden spans.
//!
//! # Connecting worker threads to their parent span
//!
//! Threads do not inherit the current span or a scoped default subscriber. Wrap the
//! thread's closure with [`crate::in_thread()`] before spawning to capture both and
//! restore them in the worker. Spans created in the worker then inherit the parent:
//!
//! ```
//! use gix_error::{message, ErrorExt};
//! use gix_trace::{ForestLayer, forest::processor};
//! use std::{sync::mpsc, thread};
//! use tracing_subscriber::{layer::SubscriberExt, Registry};
//!
//! let (sender, finished) = mpsc::channel();
//! let layer = ForestLayer::from(processor::from_fn(move |tree| {
//!     sender.send(tree).map_err(|err| {
//!         processor::error(err.0, message("tree receiver dropped").raise())
//!     })
//! }));
//! let subscriber = Registry::default().with(layer);
//!
//! tracing::subscriber::with_default(subscriber, || {
//!     let root = gix_trace::coarse!("operation");
//!     let worker = thread::spawn(gix_trace::in_thread(|| {
//!         let _worker = gix_trace::coarse!("worker");
//!         gix_trace::info!("work completed");
//!     }));
//!     worker.join().expect("worker must finish without panicking");
//!     drop(root);
//! });
//!
//! let tree = finished.try_recv()?;
//! let root = tree.span()?;
//! assert_eq!(root.name(), "operation");
//! assert_eq!(root.nodes()[0].span()?.name(), "worker");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`crate::Span`] is already an entered guard when tracing is enabled and must stay on
//! its creating thread. [`crate::in_thread()`] transfers an ordinary [`tracing::Span`]
//! handle and enters it on the worker. [`tracing::Span::follows_from`] records a causal relation;
//! it does not establish the parent relationship used to build these trees.
//!
//! A span closes only after all its handles and child references are dropped. Leaving
//! an entered scope alone may therefore leave a tree buffered, and a worker can keep
//! its parent open after the spawning thread drops its own handle. Drop worker and
//! parent handles before leaving the installed dispatch (as [`crate::in_thread()`] does), then join workers before
//! inspecting completed output. Child spans are attached in completion order, which
//! can differ from their creation order across threads.
//!
//! Each span measures time while at least one of its entries is active. Nested or
//! concurrent entries of the same span count overlapping intervals only once. Separate
//! child spans' durations are added together, and a parent's total is raised to at least
//! that sum. Overlapping workers can therefore produce a total larger than elapsed
//! wall-clock time. These durations describe aggregate tracing activity, not CPU time.
//! Giving each worker its own child span makes that worker's events and activity visible
//! separately.
//!
//! # CPU time
//!
//! Enable `forest-cpu-time` to also collect user and kernel CPU time on Linux, macOS,
//! FreeBSD, and OpenBSD. Each thread charges CPU time to its most recently entered
//! distinct forest span. Re-entering an active ancestor keeps its child current.
//! Concurrent entries contribute independently; suspended futures accumulate no CPU
//! time between polls. Time blocked on I/O or sleeping is excluded, as is CPU work in
//! uninstrumented worker threads and child processes.
//!
//! A completed `tree::Span` exposes `base_cpu_time()` for its own work,
//! `inner_cpu_time()` for its children's work, and `total_cpu_time()` for their sum.
//! The sum includes parallel children and children running while their parent is not
//! entered. CPU accounting is separate from the existing elapsed-time measurements.
//! The pretty printer appends inclusive times as `[ user: 12.00ms | sys: 3.00ms ]`.
//!
//! Sampling adds a system call on entry and exit, and tracing overhead contributes
//! to measured CPU time. OS accounting resolution can make short spans measure as
//! zero. Unsupported platforms and incomplete measurements return `None`, and the
//! pretty printer omits CPU times. Disabling the feature removes CPU sampling and its state.

#[cfg(feature = "forest-cpu-time")]
mod cpu;
mod fail;
mod layer;
pub mod printer;
pub mod processor;
pub mod tag;
pub mod tree;

pub use layer::{init, test_init};

/// A node in the log tree, consisting of either a [`Span`](tree::Span) or an [`Event`](tree::Event).
///
/// The inner types can be extracted through a `match` statement. Alternatively,
/// the [`event`] and [`span`] methods provide access to a node of an expected kind.
///
/// [`event`]: Tree::event
/// [`span`]: Tree::span
#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)] // https://github.com/rust-lang/rust-clippy/issues/9798
pub enum Tree {
    /// An [`Event`](tree::Event) leaf node.
    Event(tree::Event),

    /// A [`Span`](tree::Span) inner node.
    Span(tree::Span),
}

/// A trait for processing completed [`Tree`]s.
///
/// `Processor`s are responsible for both formatting and writing logs to their
/// intended destinations. This is typically implemented using
/// [`Formatter`], [`MakeWriter`], and [`io::Write`].
///
/// While this trait may be implemented on downstream types, [`processor::from_fn`]
/// provides a convenient interface for creating `Processor`s without having to
/// explicitly define new types.
///
/// [trace trees]: crate::forest::Tree
/// [`Formatter`]: crate::forest::Formatter
/// [`MakeWriter`]: tracing_subscriber::fmt::MakeWriter
/// [`io::Write`]: std::io::Write
pub trait Processor: 'static + Sized {
    /// Process a [`Tree`]. This can mean many things, such as writing to
    /// stdout or a file, sending over a network, storing in memory, ignoring,
    /// or anything else.
    ///
    /// # Errors
    ///
    /// If the `Tree` cannot be processed, then it is returned along with a
    /// [`gix_error::Error`]. If the processor is configured with a
    /// fallback processor from [`Processor::or`], then the `Tree` is deferred
    /// to that processor.
    #[allow(clippy::result_large_err)]
    fn process(&self, tree: Tree) -> processor::Result;

    /// Returns a `Processor` that first attempts processing with `self`, and
    /// resorts to processing with `fallback` on failure.
    ///
    /// Note that [`or_stdout`], [`or_stderr`], and [`or_none`] can be used as
    /// shortcuts for pretty printing or dropping the `Tree` entirely.
    ///
    /// [`or_stdout`]: Processor::or_stdout
    /// [`or_stderr`]: Processor::or_stderr
    /// [`or_none`]: Processor::or_none
    fn or<P: Processor>(self, processor: P) -> processor::WithFallback<Self, P> {
        processor::WithFallback {
            primary: self,
            fallback: processor,
        }
    }

    /// Returns a `Processor` that first attempts processing with `self`, and
    /// resorts to pretty-printing to stdout on failure.
    fn or_stdout(self) -> processor::WithFallback<Self, Printer<printer::Pretty, printer::MakeStdout>> {
        self.or(Printer::new().writer(printer::MakeStdout))
    }

    /// Returns a `Processor` that first attempts processing with `self`, and
    /// resorts to pretty-printing to stderr on failure.
    fn or_stderr(self) -> processor::WithFallback<Self, Printer<printer::Pretty, printer::MakeStderr>> {
        self.or(Printer::new().writer(printer::MakeStderr))
    }

    /// Returns a `Processor` that first attempts processing with `self`, otherwise
    /// silently fails.
    fn or_none(self) -> processor::WithFallback<Self, processor::Sink> {
        self.or(processor::Sink)
    }
}

/// Format a [`Tree`] into a `String`.
///
/// This trait is implemented for all `Fn(&Tree) -> Result<String, E>` types, where `E: Error + Send + Sync`.
pub trait Formatter {
    /// The error type if the `Tree` cannot be stringified.
    type Error: std::error::Error + Send + Sync;

    /// Stringifies the `Tree`, or returns an error.
    ///
    /// # Errors
    ///
    /// If the `Tree` cannot be formatted to a string, an error is returned.
    fn fmt(&self, tree: &Tree) -> Result<String, Self::Error>;
}

/// A [`Processor`] that formats and writes logs.
#[derive(Clone, Debug)]
pub struct Printer<F, W> {
    formatter: F,
    make_writer: W,
}

/// A [`Processor`] that pretty-prints to stdout.
pub type PrettyPrinter = Printer<printer::Pretty, printer::MakeStdout>;

/// A [`Processor`] that captures logs during tests and allows them to be presented
/// when `--nocapture` is used.
#[derive(Clone, Debug)]
pub struct TestCapturePrinter<F> {
    formatter: F,
}

/// A basic `Copy` type containing information about where an event occurred.
///
/// See the [module-level documentation](mod@crate::forest::tag) for more details.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Tag {
    /// Optional prefix for the tag message
    prefix: Option<&'static str>,

    /// Level specifying the importance of the log.
    ///
    /// This value isn't necessarily "trace", "debug", "info", "warn", or "error",
    /// and can be customized.
    suffix: &'static str,

    /// An icon, typically emoji, that represents the tag.
    icon: char,
}

/// A type that can parse [`Tag`]s from Tracing events.
///
/// This trait is blanket-implemented for all `Fn(&tracing::Event) -> Option<Tag>`,
/// so top-level `fn`s can be used.
///
/// See the [module-level documentation](mod@crate::forest::tag) for more details.
pub trait TagParser: 'static {
    /// Parse a tag from a [`tracing::Event`]
    fn parse(&self, event: &tracing::Event) -> Option<Tag>;
}

/// A `TagParser` that always returns `None`.
#[derive(Clone, Debug)]
pub struct NoTag;

/// Bring forest, tracing, and subscriber extension traits into scope anonymously.
pub mod traits {
    pub use super::Processor as _;
    pub use tracing::Instrument as _;
    pub use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};
}
