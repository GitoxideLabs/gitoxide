//! Collected trace data: [`Span`] branches and [`Event`] leaves in a [`Tree`].
//!
//! A [`Processor`](crate::forest::Processor) receives completed trees and can inspect
//! their fields, child nodes, and durations before storing or formatting them.
use crate::forest::{Tag, Tree};
use std::{fmt, time::Duration};
use tracing::Level;

mod field;

pub use field::Field;
pub(crate) use field::FieldSet;

/// A leaf node in the log tree carrying information about a Tracing event.
#[derive(Clone, Debug)]
pub struct Event {
    /// Shared fields between events and spans.
    pub(crate) shared: Shared,

    /// The message associated with the event.
    pub(crate) message: Option<String>,

    /// The tag that the event was collected with.
    pub(crate) tag: Option<Tag>,
}

/// An internal node in the log tree carrying information about a Tracing span.
#[derive(Clone, Debug)]
pub struct Span {
    /// Shared fields between events and spans.
    pub(crate) shared: Shared,

    /// The name of the span.
    pub(crate) name: &'static str,

    /// The duration with at least one active entry, raised to at least `inner_duration`.
    pub(crate) total_duration: Duration,

    /// The sum of child spans' total durations, including overlaps between children.
    pub(crate) inner_duration: Duration,

    #[cfg(feature = "forest-cpu-time")]
    pub(crate) base_cpu_time: Option<CpuTime>,

    #[cfg(feature = "forest-cpu-time")]
    pub(crate) inner_cpu_time: Option<CpuTime>,

    /// Events and spans collected while the span was open.
    pub(crate) nodes: Vec<Tree>,
}

/// CPU time consumed in user mode and kernel mode, excluding time blocked on I/O or sleeping.
///
/// Available with `forest-cpu-time`. Concurrent threads contribute separately, so
/// their accumulated CPU time can exceed elapsed wall-clock time. Counter resolution
/// depends on the operating system; short operations may measure as zero.
#[cfg(feature = "forest-cpu-time")]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CpuTime {
    /// CPU time executing user-space code.
    pub user: Duration,
    /// CPU time executing kernel code on behalf of the measured threads.
    pub system: Duration,
}

#[cfg(feature = "forest-cpu-time")]
impl CpuTime {
    pub(crate) fn checked_add(self, other: Self) -> Option<Self> {
        Some(Self {
            user: self.user.checked_add(other.user)?,
            system: self.system.checked_add(other.system)?,
        })
    }

    pub(crate) fn checked_sub(self, earlier: Self) -> Option<Self> {
        Some(Self {
            user: self.user.checked_sub(earlier.user)?,
            system: self.system.checked_sub(earlier.system)?,
        })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Shared {
    /// The level the event or span occurred at.
    pub(crate) level: Level,

    /// Key-value data.
    pub(crate) fields: FieldSet,
}

/// Error returned by [`Tree::event`][event].
///
/// [event]: crate::forest::Tree::event
#[derive(Debug)]
pub struct ExpectedEventError(());

impl fmt::Display for ExpectedEventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Expected an event, found a span")
    }
}

impl std::error::Error for ExpectedEventError {}

/// Error returned by [`Tree::span`][span].
///
/// [span]: crate::forest::Tree::span
#[derive(Debug)]
pub struct ExpectedSpanError(());

impl fmt::Display for ExpectedSpanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Expected a span, found an event")
    }
}

impl std::error::Error for ExpectedSpanError {}

impl Tree {
    /// Returns a reference to the inner [`Event`] if the tree is an event.
    ///
    /// # Errors
    ///
    /// This function returns an error if the `Tree` contains the `Span` variant.
    ///
    /// # Examples
    ///
    /// Collect a completed tree with a channel and inspect its event:
    /// ```
    /// use gix_trace::{ForestLayer, forest::{processor, tree::Event}};
    /// use tracing_subscriber::layer::SubscriberExt;
    ///
    /// let (sender, receiver) = std::sync::mpsc::channel();
    /// let processor = processor::from_fn(move |tree| {
    ///     sender.send(tree).expect("the receiver lives until the tree is inspected");
    ///     Ok(())
    /// });
    /// let subscriber = tracing_subscriber::Registry::default().with(ForestLayer::from(processor));
    /// tracing::subscriber::with_default(subscriber, || {
    ///     tracing::info!("some information");
    /// });
    /// let tree = receiver.recv()?;
    /// let event: &Event = tree.event()?;
    /// assert_eq!(event.message(), Some("some information"), "the event retains its message");
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn event(&self) -> Result<&Event, ExpectedEventError> {
        match self {
            Tree::Event(event) => Ok(event),
            Tree::Span(_) => Err(ExpectedEventError(())),
        }
    }

    /// Returns a reference to the inner [`Span`] if the tree is a span.
    ///
    /// # Errors
    ///
    /// This function returns an error if the `Tree` contains the `Event` variant.
    ///
    /// # Examples
    ///
    /// Collect a completed tree with a channel and inspect its span:
    /// ```
    /// use gix_trace::{ForestLayer, forest::{processor, tree::Span}};
    /// use tracing_subscriber::layer::SubscriberExt;
    ///
    /// let (sender, receiver) = std::sync::mpsc::channel();
    /// let processor = processor::from_fn(move |tree| {
    ///     sender.send(tree).expect("the receiver lives until the tree is inspected");
    ///     Ok(())
    /// });
    /// let subscriber = tracing_subscriber::Registry::default().with(ForestLayer::from(processor));
    /// tracing::subscriber::with_default(subscriber, || {
    ///     tracing::info_span!("my_span").in_scope(|| tracing::info!("inside the span"));
    /// });
    /// let tree = receiver.recv()?;
    /// let span: &Span = tree.span()?;
    /// assert_eq!(span.name(), "my_span", "the completed span retains its name");
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn span(&self) -> Result<&Span, ExpectedSpanError> {
        match self {
            Tree::Event(_) => Err(ExpectedSpanError(())),
            Tree::Span(span) => Ok(span),
        }
    }
}

impl Event {
    /// Returns the event's [`Level`].
    pub fn level(&self) -> Level {
        self.shared.level
    }

    /// Returns the event's message, if there is one.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// Returns the event's [`Tag`], if there is one.
    pub fn tag(&self) -> Option<Tag> {
        self.tag
    }

    /// Returns the event's fields.
    pub fn fields(&self) -> &[Field] {
        &self.shared.fields
    }
}

impl Span {
    pub(crate) fn new(shared: Shared, name: &'static str) -> Self {
        Span {
            shared,
            name,
            total_duration: Duration::ZERO,
            inner_duration: Duration::ZERO,
            #[cfg(feature = "forest-cpu-time")]
            base_cpu_time: crate::forest::cpu::initial_time(),
            #[cfg(feature = "forest-cpu-time")]
            inner_cpu_time: crate::forest::cpu::initial_time(),
            nodes: Vec::new(),
        }
    }

    /// Returns the span's [`Level`].
    pub fn level(&self) -> Level {
        self.shared.level
    }

    /// Returns the span's name.
    pub fn name(&self) -> &str {
        self.name
    }

    /// Returns the span's fields, including values recorded after its creation.
    ///
    /// Each key occurs once and retains its most recently recorded value.
    pub fn fields(&self) -> &[Field] {
        &self.shared.fields
    }

    /// Returns the span's child trees.
    pub fn nodes(&self) -> &[Tree] {
        &self.nodes
    }

    /// Returns the accumulated duration measured while the span had at least one active entry.
    ///
    /// Nested or concurrent entries of this same span count overlapping intervals
    /// only once: the first entry starts timing and the last matching exit stops it.
    ///
    /// When the span closes, this is raised to at least the sum of its child
    /// spans' durations. Children can run concurrently or while their parent is
    /// not entered, so this can exceed elapsed wall-clock time.
    ///
    /// For an instrumented `Future`, only time spent polling is measured.
    pub fn total_duration(&self) -> Duration {
        self.total_duration
    }

    /// Returns the sum of the child spans' total durations.
    ///
    /// Concurrent children contribute separately even when their execution overlaps.
    pub fn inner_duration(&self) -> Duration {
        self.inner_duration
    }

    /// Returns the total duration minus the sum of the child spans' durations.
    pub fn base_duration(&self) -> Duration {
        self.total_duration
            .checked_sub(self.inner_duration)
            .expect("the forest layer raises the total duration to at least the sum of its children")
    }

    /// Returns this span's own CPU time plus its children's total CPU time.
    ///
    /// Children contribute even when they run on other threads or while their
    /// parent is not entered. Returns `None` on unsupported platforms or if any
    /// contributing measurement failed, went backwards, or overflowed.
    #[cfg(feature = "forest-cpu-time")]
    pub fn total_cpu_time(&self) -> Option<CpuTime> {
        self.base_cpu_time?.checked_add(self.inner_cpu_time?)
    }

    /// Returns CPU time charged directly to this span, excluding other active spans.
    ///
    /// Each thread charges its most recently entered distinct forest span. Re-entering
    /// an already active span leaves the current span unchanged, as in the tracing
    /// registry. Nested spans with a different explicit parent or subscriber also
    /// receive their own CPU time. Concurrent entries contribute separately, and
    /// instrumented futures accumulate time only while they are polled or dropped.
    ///
    /// Returns `None` on unsupported platforms or if this span's own measurement
    /// failed, went backwards, or overflowed.
    #[cfg(feature = "forest-cpu-time")]
    pub fn base_cpu_time(&self) -> Option<CpuTime> {
        self.base_cpu_time
    }

    /// Returns the sum of the child spans' total CPU times, including parallel work.
    ///
    /// Returns `None` on unsupported platforms or if any child measurement is
    /// unavailable or their sum overflowed.
    #[cfg(feature = "forest-cpu-time")]
    pub fn inner_cpu_time(&self) -> Option<CpuTime> {
        self.inner_cpu_time
    }
}
