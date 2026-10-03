use crate::ForestLayer;
use crate::forest::processor::Sink;
use crate::forest::tree::{self, FieldSet};
use crate::forest::{NoTag, PrettyPrinter, Processor, Tag, TagParser, TestCapturePrinter, Tree, fail};
use std::fmt;
use std::io::{self, Write};
use std::time::Instant;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
use tracing_subscriber::registry::{LookupSpan, Registry, SpanRef};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::util::TryInitError;

pub(crate) struct OpenedSpan {
    span: tree::Span,
    start: Instant,
    active_entries: usize,
    #[cfg(feature = "forest-cpu-time")]
    cpu: super::cpu::Span,
}

impl OpenedSpan {
    fn new(attrs: &Attributes<'_>) -> Self {
        let mut fields = FieldSet::default();
        attrs.record(&mut |field: &Field, value: &dyn fmt::Debug| {
            fields.push(tree::Field::new(field.name(), format!("{value:?}")));
        });
        OpenedSpan {
            span: tree::Span::new(
                tree::Shared {
                    level: *attrs.metadata().level(),
                    fields,
                },
                attrs.metadata().name(),
            ),
            start: Instant::now(),
            active_entries: 0,
            #[cfg(feature = "forest-cpu-time")]
            cpu: super::cpu::Span::default(),
        }
    }

    fn enter(&mut self, now: Instant) {
        self.active_entries = self
            .active_entries
            .checked_add(1)
            .expect("each active span entry requires a live guard, so the count fits in usize");
        if self.active_entries == 1 {
            self.start = now;
        }
        #[cfg(feature = "forest-cpu-time")]
        self.cpu.enter();
    }

    fn exit(&mut self, now: Instant) {
        #[cfg(feature = "forest-cpu-time")]
        self.cpu.exit();
        self.active_entries = self
            .active_entries
            .checked_sub(1)
            .expect("tracing exits a span only after entering it");
        if self.active_entries == 0 {
            self.span.total_duration += now.duration_since(self.start);
        }
    }

    fn close(self) -> tree::Span {
        #[cfg(feature = "forest-cpu-time")]
        {
            let mut span = self.span;
            span.base_cpu_time = self.cpu.time();
            span
        }
        #[cfg(not(feature = "forest-cpu-time"))]
        self.span
    }

    fn record_event(&mut self, event: tree::Event) {
        self.span.nodes.push(Tree::Event(event));
    }

    fn record_span(&mut self, span: tree::Span) {
        self.span.inner_duration += span.total_duration();
        #[cfg(feature = "forest-cpu-time")]
        {
            self.span.inner_cpu_time = self
                .span
                .inner_cpu_time
                .zip(span.total_cpu_time())
                .and_then(|(total, child)| total.checked_add(child));
        }
        self.span.nodes.push(Tree::Span(span));
    }
}

impl<P: Processor, T: TagParser> ForestLayer<P, T> {
    /// Create a new `ForestLayer` from a [`Processor`] and a [`TagParser`].
    pub fn new(processor: P, tag: T) -> Self {
        ForestLayer { processor, tag }
    }
}

impl<P: Processor> From<P> for ForestLayer<P, NoTag> {
    fn from(processor: P) -> Self {
        ForestLayer::new(processor, NoTag)
    }
}

impl ForestLayer<Sink, NoTag> {
    /// Create a new `ForestLayer` that does nothing with collected trace data.
    pub fn sink() -> Self {
        ForestLayer::from(Sink)
    }
}

impl Default for ForestLayer<PrettyPrinter, NoTag> {
    fn default() -> Self {
        ForestLayer {
            processor: PrettyPrinter::new(),
            tag: NoTag,
        }
    }
}

impl<P, T, S> Layer<S> for ForestLayer<P, T>
where
    P: Processor,
    T: TagParser,
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect(fail::SPAN_NOT_IN_CONTEXT);
        let opened = OpenedSpan::new(attrs);

        let mut extensions = span.extensions_mut();
        extensions.insert(opened);
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        let mut recorded = FieldSet::default();
        // User-provided Debug implementations can trace into this span again.
        values.record(&mut |field: &Field, value: &dyn fmt::Debug| {
            recorded.push(tree::Field::new(field.name(), format!("{value:?}")));
        });
        let span = ctx.span(id).expect(fail::SPAN_NOT_IN_CONTEXT);
        let mut extensions = span.extensions_mut();
        let fields = &mut extensions
            .get_mut::<OpenedSpan>()
            .expect(fail::OPENED_SPAN_NOT_IN_EXTENSIONS)
            .span
            .shared
            .fields;
        for field in recorded {
            if let Some(existing) = fields.iter_mut().find(|existing| existing.key() == field.key()) {
                *existing = field;
            } else {
                fields.push(field);
            }
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        struct Visitor {
            message: Option<String>,
            fields: FieldSet,
            immediate: bool,
        }

        impl Visit for Visitor {
            fn record_bool(&mut self, field: &Field, value: bool) {
                match field.name() {
                    "immediate" => self.immediate |= value,
                    _ => self.record_debug(field, &value),
                }
            }

            fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
                let value = format!("{value:?}");
                match field.name() {
                    "message" if self.message.is_none() => self.message = Some(value),
                    key => self.fields.push(tree::Field::new(key, value)),
                }
            }
        }

        let mut visitor = Visitor {
            message: None,
            fields: FieldSet::default(),
            immediate: false,
        };

        event.record(&mut visitor);

        let shared = tree::Shared {
            level: *event.metadata().level(),
            fields: visitor.fields,
        };

        let tree_event = tree::Event {
            shared,
            message: visitor.message,
            tag: self.tag.parse(event),
        };

        let current_span = ctx.event_span(event);

        if visitor.immediate {
            write_immediate(&tree_event, current_span.as_ref())
                .expect("writing an immediate trace event to stderr failed");
        }

        match current_span.as_ref() {
            Some(parent) => parent
                .extensions_mut()
                .get_mut::<OpenedSpan>()
                .expect(fail::OPENED_SPAN_NOT_IN_EXTENSIONS)
                .record_event(tree_event),
            None => self
                .processor
                .process(Tree::Event(tree_event))
                .expect(fail::PROCESSING_ERROR),
        }
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        ctx.span(id)
            .expect(fail::SPAN_NOT_IN_CONTEXT)
            .extensions_mut()
            .get_mut::<OpenedSpan>()
            .expect(fail::OPENED_SPAN_NOT_IN_EXTENSIONS)
            .enter(Instant::now());
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        ctx.span(id)
            .expect(fail::SPAN_NOT_IN_CONTEXT)
            .extensions_mut()
            .get_mut::<OpenedSpan>()
            .expect(fail::OPENED_SPAN_NOT_IN_EXTENSIONS)
            .exit(Instant::now());
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let span_ref = ctx.span(&id).expect(fail::SPAN_NOT_IN_CONTEXT);

        let mut span = span_ref
            .extensions_mut()
            .remove::<OpenedSpan>()
            .expect(fail::OPENED_SPAN_NOT_IN_EXTENSIONS)
            .close();

        // Ensure that the total duration is at least as much as the inner
        // duration. This is caused by when a child span is manually passed
        // a parent span and then enters without entering the parent span. Also
        // when a child span is created within a parent, and then stored and
        // entered again when the parent isn't opened.
        //
        // Issue: https://github.com/QnnOkabayashi/tracing-forest/issues/11
        if span.total_duration < span.inner_duration {
            span.total_duration = span.inner_duration;
        }

        match span_ref.parent() {
            Some(parent) => parent
                .extensions_mut()
                .get_mut::<OpenedSpan>()
                .expect(fail::OPENED_SPAN_NOT_IN_EXTENSIONS)
                .record_span(span),
            None => self.processor.process(Tree::Span(span)).expect(fail::PROCESSING_ERROR),
        }
    }
}

fn write_immediate<S>(event: &tree::Event, current: Option<&SpanRef<'_, S>>) -> io::Result<()>
where
    S: for<'a> LookupSpan<'a>,
{
    // LEVEL root > inner > leaf > my message here | key: val
    let mut writer = smallvec::SmallVec::<[u8; 256]>::new();

    write!(writer, "{:<8} ", event.level())?;

    let tag = event.tag().unwrap_or_else(|| Tag::from(event.level()));

    write!(writer, "{icon} IMMEDIATE {icon} ", icon = tag.icon())?;

    if let Some(span) = current {
        for ancestor in span.scope().from_root() {
            write!(writer, "{} > ", ancestor.name())?;
        }
    }

    if let Some(message) = event.message() {
        write!(writer, "{message}")?;
    }

    for field in event.fields() {
        write!(writer, " | {}: {}", field.key(), field.value())?;
    }

    writeln!(writer)?;

    io::stderr().write_all(&writer)
}

/// Initializes a global subscriber with a [`ForestLayer`] using the default configuration.
///
/// Completed trees are formatted and written synchronously on the thread that closes
/// the root span or emits an event outside any span. Configure a [`ForestLayer`]
/// manually to customize processing and output.
///
/// Returns an error if a global subscriber or logger was already installed.
///
/// # Examples
/// ```
/// use tracing::{info, info_span};
///
/// gix_trace::forest::init()?;
///
/// info!("Hello, world!");
/// info_span!("my_span").in_scope(|| {
///     info!("Relevant information");
/// });
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
/// Produces output like:
/// ```log
/// INFO     ｉ [info]: Hello, world!
/// INFO     my_span [ 26.0µs | 100.000% ]
/// INFO     ┕━ ｉ [info]: Relevant information
/// ```
pub fn init() -> Result<(), TryInitError> {
    Registry::default().with(ForestLayer::default()).try_init()
}

/// Initializes a global subscriber for cargo tests with a [`ForestLayer`] using the default
/// configuration.
///
/// Completed trees are formatted synchronously and printed through the standard
/// test output capture. Calling this repeatedly is safe; initialization errors are
/// returned if a global subscriber was already installed.
///
/// # Examples
/// ```
/// use tracing::{info, info_span};
///
/// let _ = gix_trace::forest::test_init();
///
/// info!("Hello, world!");
/// info_span!("my_span").in_scope(|| {
///     info!("Relevant information");
/// });
/// ```
pub fn test_init() -> Result<(), TryInitError> {
    Registry::default()
        .with(ForestLayer::new(TestCapturePrinter::new(), NoTag))
        .try_init()
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{OpenedSpan, tree};

    fn span() -> OpenedSpan {
        OpenedSpan {
            span: tree::Span::new(
                tree::Shared {
                    level: tracing::Level::INFO,
                    fields: Default::default(),
                },
                "operation",
            ),
            start: Instant::now(),
            active_entries: 0,
            #[cfg(feature = "forest-cpu-time")]
            cpu: super::super::cpu::Span::default(),
        }
    }

    #[test]
    fn overlapping_and_reentrant_entries_count_elapsed_time_once() {
        let mut span = span();
        let start = Instant::now();
        span.enter(start);
        span.enter(start + Duration::from_nanos(5));
        span.enter(start + Duration::from_nanos(9));
        span.exit(start + Duration::from_nanos(13));
        span.exit(start + Duration::from_nanos(17));
        span.exit(start + Duration::from_nanos(23));
        assert_eq!(
            span.close().total_duration(),
            Duration::from_nanos(23),
            "overlapping entries keep the first start and count until the last exit"
        );
    }

    #[test]
    fn disjoint_entries_exclude_idle_time() {
        let mut span = span();
        let start = Instant::now();
        span.enter(start);
        span.exit(start + Duration::from_nanos(5));
        span.enter(start + Duration::from_nanos(20));
        span.exit(start + Duration::from_nanos(29));
        assert_eq!(
            span.close().total_duration(),
            Duration::from_nanos(14),
            "separate entry intervals accumulate without the idle gap between them"
        );
    }
}
