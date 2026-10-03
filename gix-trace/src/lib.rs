//! A crate providing macros for creating spans in various detail levels. `coarse!` should be used for top-level operations, whereas
//! `detail!` should be used in plumbing crates unless their operations are likely to cost a lot of time.
//!
//! The application is supposed to explicitly turn on tracing via `gix-features`.
//! Crates that use `gix-features` should use `gix_features::trace`, and those who don't can use `gix_trace` directly.
//! Enable `forest` to collect and print trace trees; its module documentation explains connecting worker spans to their parent.
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(missing_docs, unsafe_code)]

#[cfg(feature = "forest")]
pub mod forest;

/// A [`tracing_subscriber::Layer`] that collects and processes trace data while preserving
/// contextual coherence.
///
/// Completed root spans and events without parents are passed to the configured
/// [`forest::Processor`]. Processing failures panic unless handled by a fallback from
/// [`forest::Processor::or`].
/// Span fields retain their latest recorded values in the order they were first
/// recorded. Overlapping or nested entries into the same span count elapsed time
/// once, from the first entry until the last exit.
///
/// See [`forest`] for configuration and connecting worker threads to their parent span.
#[cfg(feature = "forest")]
#[derive(Clone, Debug)]
pub struct ForestLayer<P, T> {
    processor: P,
    tag: T,
}

/// The level at which the tracing item should be created.
///
/// It's used to filter items early.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub enum Level {
    /// A coarse-grained trace level, one that should span entire operations with low frequency.
    Coarse = 1,
    /// Finer grained trace level that further subdivides coarse-level traces.
    ///
    /// Note that these should only be created for areas of the code which have significant cost.
    Detail = 2,
}

/// The maximum allowed level for tracing items, as compiled in.
#[cfg(feature = "tracing-detail")]
pub const MAX_LEVEL: Level = Level::Detail;
/// The maximum allowed level for tracing items, as compiled in.
#[cfg(not(feature = "tracing-detail"))]
pub const MAX_LEVEL: Level = Level::Coarse;

#[cfg(feature = "tracing")]
mod enabled;

#[cfg(feature = "tracing")]
pub use enabled::{__tracing, Span, field};

impl Span {
    /// Execute `f` in with this span active, consuming it.
    pub fn into_scope<T>(self, f: impl FnOnce() -> T) -> T {
        f()
    }
}

#[cfg(not(feature = "tracing"))]
mod disabled;
#[cfg(not(feature = "tracing"))]
pub use disabled::Span;

/// Capture the current span and subscriber, restoring both while the returned closure runs.
///
/// Wrap a thread's closure before spawning it so spans and events in the thread retain
/// their parent, including when a subscriber was installed only on the calling thread.
/// With tracing disabled this returns `f` unchanged.
///
/// ```
/// let _span = gix_trace::coarse!("operation");
/// std::thread::scope(|scope| {
///     scope.spawn(gix_trace::in_thread(|| {
///         let _span = gix_trace::coarse!("worker");
///     }));
/// });
/// ```
pub fn in_thread<T>(f: impl FnOnce() -> T) -> impl FnOnce() -> T {
    #[cfg(feature = "tracing")]
    {
        let dispatch = tracing::dispatcher::get_default(Clone::clone);
        let parent = tracing::Span::current();
        move || {
            tracing::dispatcher::with_default(&dispatch, move || {
                let _parent = parent.entered();
                f()
            })
        }
    }
    #[cfg(not(feature = "tracing"))]
    f
}

///
pub mod event {
    #[cfg(feature = "tracing")]
    pub use tracing::Level;

    /// All available tracing levels for use in `event!()` macro.
    #[cfg(not(feature = "tracing"))]
    #[repr(usize)]
    #[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
    pub enum Level {
        /// The "trace" level.
        ///
        /// Designates very low priority, often extremely verbose, information.
        TRACE = 0,
        /// The "debug" level.
        ///
        /// Designates lower priority information.
        DEBUG = 1,
        /// The "info" level.
        ///
        /// Designates useful information.
        INFO = 2,
        /// The "warn" level.
        ///
        /// Designates hazardous situations.
        WARN = 3,
        /// The "error" level.
        ///
        /// Designates very serious errors.
        ERROR = 4,
    }
}

/// Create a new [coarse][Level::Coarse] span.
#[macro_export]
macro_rules! coarse {
    (target: $target:expr, $name:expr, $($field:tt)*) => {
        $crate::span!(
            target: $target,
            $crate::Level::Coarse,
            $name,
            $($field)*
        )
    };
    (target: $target:expr, $name:expr) => {
        $crate::coarse!(target: $target, $name,)
    };
    ($name:expr, $($field:tt)*) => {
        $crate::span!(
            target: module_path!(),
            $crate::Level::Coarse,
            $name,
            $($field)*
        )
    };
    ($name:expr) => {$crate::coarse!($name,)};
}

/// Create a new [detail][Level::Detail] span.
#[macro_export]
macro_rules! detail {
    (target: $target:expr, $name:expr, $($field:tt)*) => {
        $crate::span!(
            target: $target,
            $crate::Level::Detail,
            $name,
            $($field)*
        )
    };
    (target: $target:expr, $name:expr) => {
        $crate::detail!(target: $target, $name,)
    };
    ($name:expr, $($field:tt)*) => {
        $crate::span!(
            target: module_path!(),
            $crate::Level::Detail,
            $name,
            $($field)*
        )
    };
    ($name:expr) => {$crate::coarse!($name,)};
}

/// Emit an error event.
#[macro_export]
macro_rules! error {
    (target: $target:expr, { $($field:tt)* }, $($arg:tt)* ) => (
        $crate::event!(target: $target, $crate::event::Level::ERROR, { $($field)* }, $($arg)*)
    );
    (target: $target:expr, $($k:ident).+ $($field:tt)* ) => (
        $crate::event!(target: $target, $crate::event::Level::ERROR, { $($k).+ $($field)* })
    );
    (target: $target:expr, ?$($k:ident).+ $($field:tt)* ) => (
        $crate::event!(target: $target, $crate::event::Level::ERROR, { ?$($k).+ $($field)* })
    );
    (target: $target:expr, %$($k:ident).+ $($field:tt)* ) => (
        $crate::event!(target: $target, $crate::event::Level::ERROR, { %$($k).+ $($field)* })
    );
    (target: $target:expr, $($arg:tt)+ ) => (
        $crate::event!(target: $target, $crate::event::Level::ERROR, {}, $($arg)+)
    );
    ({ $($field:tt)+ }, $($arg:tt)+ ) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::ERROR,
            { $($field)+ },
            $($arg)+
        )
    );
    ($($k:ident).+ = $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::ERROR,
            { $($k).+ = $($field)*}
        )
    );
    (?$($k:ident).+ = $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::ERROR,
            { ?$($k).+ = $($field)*}
        )
    );
    (%$($k:ident).+ = $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::ERROR,
            { %$($k).+ = $($field)*}
        )
    );
    ($($k:ident).+, $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::ERROR,
            { $($k).+, $($field)*}
        )
    );
    (?$($k:ident).+, $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::ERROR,
            { ?$($k).+, $($field)*}
        )
    );
    (%$($k:ident).+, $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::ERROR,
            { %$($k).+, $($field)*}
        )
    );
    (?$($k:ident).+) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::ERROR,
            { ?$($k).+ }
        )
    );
    (%$($k:ident).+) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::ERROR,
            { %$($k).+ }
        )
    );
    ($($k:ident).+) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::ERROR,
            { $($k).+ }
        )
    );
    ($($arg:tt)+) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::ERROR,
            {},
            $($arg)+
        )
    );
}

/// Emit a warn event.
#[macro_export]
macro_rules! warn {
    (target: $target:expr, { $($field:tt)* }, $($arg:tt)* ) => (
        $crate::event!(target: $target, $crate::event::Level::WARN, { $($field)* }, $($arg)*)
    );
    (target: $target:expr, $($k:ident).+ $($field:tt)* ) => (
        $crate::event!(target: $target, $crate::event::Level::WARN, { $($k).+ $($field)* })
    );
    (target: $target:expr, ?$($k:ident).+ $($field:tt)* ) => (
        $crate::event!(target: $target, $crate::event::Level::WARN, { ?$($k).+ $($field)* })
    );
    (target: $target:expr, %$($k:ident).+ $($field:tt)* ) => (
        $crate::event!(target: $target, $crate::event::Level::WARN, { %$($k).+ $($field)* })
    );
    (target: $target:expr, $($arg:tt)+ ) => (
        $crate::event!(target: $target, $crate::event::Level::WARN, {}, $($arg)+)
    );
    ({ $($field:tt)+ }, $($arg:tt)+ ) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::WARN,
            { $($field)+ },
            $($arg)+
        )
    );
    ($($k:ident).+ = $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::WARN,
            { $($k).+ = $($field)*}
        )
    );
    (?$($k:ident).+ = $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::WARN,
            { ?$($k).+ = $($field)*}
        )
    );
    (%$($k:ident).+ = $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::WARN,
            { %$($k).+ = $($field)*}
        )
    );
    ($($k:ident).+, $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::WARN,
            { $($k).+, $($field)*}
        )
    );
    (?$($k:ident).+, $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::WARN,
            { ?$($k).+, $($field)*}
        )
    );
    (%$($k:ident).+, $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::WARN,
            { %$($k).+, $($field)*}
        )
    );
    (?$($k:ident).+) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::WARN,
            { ?$($k).+ }
        )
    );
    (%$($k:ident).+) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::WARN,
            { %$($k).+ }
        )
    );
    ($($k:ident).+) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::WARN,
            { $($k).+ }
        )
    );
    ($($arg:tt)+) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::WARN,
            {},
            $($arg)+
        )
    );
}

/// Emit an info event.
#[macro_export]
macro_rules! info {
    (target: $target:expr, { $($field:tt)* }, $($arg:tt)* ) => (
        $crate::event!(target: $target, $crate::event::Level::INFO, { $($field)* }, $($arg)*)
    );
    (target: $target:expr, $($k:ident).+ $($field:tt)* ) => (
        $crate::event!(target: $target, $crate::event::Level::INFO, { $($k).+ $($field)* })
    );
    (target: $target:expr, ?$($k:ident).+ $($field:tt)* ) => (
        $crate::event!(target: $target, $crate::event::Level::INFO, { ?$($k).+ $($field)* })
    );
    (target: $target:expr, %$($k:ident).+ $($field:tt)* ) => (
        $crate::event!(target: $target, $crate::event::Level::INFO, { %$($k).+ $($field)* })
    );
    (target: $target:expr, $($arg:tt)+ ) => (
        $crate::event!(target: $target, $crate::event::Level::INFO, {}, $($arg)+)
    );
    ({ $($field:tt)+ }, $($arg:tt)+ ) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::INFO,
            { $($field)+ },
            $($arg)+
        )
    );
    ($($k:ident).+ = $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::INFO,
            { $($k).+ = $($field)*}
        )
    );
    (?$($k:ident).+ = $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::INFO,
            { ?$($k).+ = $($field)*}
        )
    );
    (%$($k:ident).+ = $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::INFO,
            { %$($k).+ = $($field)*}
        )
    );
    ($($k:ident).+, $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::INFO,
            { $($k).+, $($field)*}
        )
    );
    (?$($k:ident).+, $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::INFO,
            { ?$($k).+, $($field)*}
        )
    );
    (%$($k:ident).+, $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::INFO,
            { %$($k).+, $($field)*}
        )
    );
    (?$($k:ident).+) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::INFO,
            { ?$($k).+ }
        )
    );
    (%$($k:ident).+) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::INFO,
            { %$($k).+ }
        )
    );
    ($($k:ident).+) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::INFO,
            { $($k).+ }
        )
    );
    ($($arg:tt)+) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::INFO,
            {},
            $($arg)+
        )
    );
}

/// Emit a debug event.
#[macro_export]
macro_rules! debug {
    (target: $target:expr, { $($field:tt)* }, $($arg:tt)* ) => (
        $crate::event!(target: $target, $crate::event::Level::DEBUG, { $($field)* }, $($arg)*)
    );
    (target: $target:expr, $($k:ident).+ $($field:tt)* ) => (
        $crate::event!(target: $target, $crate::event::Level::DEBUG, { $($k).+ $($field)* })
    );
    (target: $target:expr, ?$($k:ident).+ $($field:tt)* ) => (
        $crate::event!(target: $target, $crate::event::Level::DEBUG, { ?$($k).+ $($field)* })
    );
    (target: $target:expr, %$($k:ident).+ $($field:tt)* ) => (
        $crate::event!(target: $target, $crate::event::Level::DEBUG, { %$($k).+ $($field)* })
    );
    (target: $target:expr, $($arg:tt)+ ) => (
        $crate::event!(target: $target, $crate::event::Level::DEBUG, {}, $($arg)+)
    );
    ({ $($field:tt)+ }, $($arg:tt)+ ) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::DEBUG,
            { $($field)+ },
            $($arg)+
        )
    );
    ($($k:ident).+ = $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::DEBUG,
            { $($k).+ = $($field)*}
        )
    );
    (?$($k:ident).+ = $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::DEBUG,
            { ?$($k).+ = $($field)*}
        )
    );
    (%$($k:ident).+ = $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::DEBUG,
            { %$($k).+ = $($field)*}
        )
    );
    ($($k:ident).+, $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::DEBUG,
            { $($k).+, $($field)*}
        )
    );
    (?$($k:ident).+, $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::DEBUG,
            { ?$($k).+, $($field)*}
        )
    );
    (%$($k:ident).+, $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::DEBUG,
            { %$($k).+, $($field)*}
        )
    );
    (?$($k:ident).+) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::DEBUG,
            { ?$($k).+ }
        )
    );
    (%$($k:ident).+) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::DEBUG,
            { %$($k).+ }
        )
    );
    ($($k:ident).+) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::DEBUG,
            { $($k).+ }
        )
    );
    ($($arg:tt)+) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::DEBUG,
            {},
            $($arg)+
        )
    );
}

/// Emit a trace event.
#[macro_export]
macro_rules! trace {
    (target: $target:expr, { $($field:tt)* }, $($arg:tt)* ) => (
        $crate::event!(target: $target, $crate::event::Level::TRACE, { $($field)* }, $($arg)*)
    );
    (target: $target:expr, $($k:ident).+ $($field:tt)* ) => (
        $crate::event!(target: $target, $crate::event::Level::TRACE, { $($k).+ $($field)* })
    );
    (target: $target:expr, ?$($k:ident).+ $($field:tt)* ) => (
        $crate::event!(target: $target, $crate::event::Level::TRACE, { ?$($k).+ $($field)* })
    );
    (target: $target:expr, %$($k:ident).+ $($field:tt)* ) => (
        $crate::event!(target: $target, $crate::event::Level::TRACE, { %$($k).+ $($field)* })
    );
    (target: $target:expr, $($arg:tt)+ ) => (
        $crate::event!(target: $target, $crate::event::Level::TRACE, {}, $($arg)+)
    );
    ({ $($field:tt)+ }, $($arg:tt)+ ) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::TRACE,
            { $($field)+ },
            $($arg)+
        )
    );
    ($($k:ident).+ = $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::TRACE,
            { $($k).+ = $($field)*}
        )
    );
    (?$($k:ident).+ = $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::TRACE,
            { ?$($k).+ = $($field)*}
        )
    );
    (%$($k:ident).+ = $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::TRACE,
            { %$($k).+ = $($field)*}
        )
    );
    ($($k:ident).+, $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::TRACE,
            { $($k).+, $($field)*}
        )
    );
    (?$($k:ident).+, $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::TRACE,
            { ?$($k).+, $($field)*}
        )
    );
    (%$($k:ident).+, $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::TRACE,
            { %$($k).+, $($field)*}
        )
    );
    (?$($k:ident).+) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::TRACE,
            { ?$($k).+ }
        )
    );
    (%$($k:ident).+) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::TRACE,
            { %$($k).+ }
        )
    );
    ($($k:ident).+) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::TRACE,
            { $($k).+ }
        )
    );
    ($($arg:tt)+) => (
        $crate::event!(
            target: module_path!(),
            $crate::event::Level::TRACE,
            {},
            $($arg)+
        )
    );
}
