//! Shared trace collection and buffered presentation for CLI commands.

use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
};

use gix::{Result, error::bail};
use gix_trace::forest::{Printer, Processor, Tree, printer::Pretty, processor};
use tracing_subscriber::{Layer, filter::LevelFilter, layer::SubscriberExt};

pub type Output = Arc<Mutex<Vec<u8>>>;

/// Inspect complete trees before presentation, independently of the display level.
pub type ProcessTree = dyn Fn(&Tree) -> Result<()> + Send + Sync;

/// Build a subscriber without installing it or flushing its output.
///
/// A tree processor enables unfiltered collection, even when display is disabled.
/// Otherwise, only the requested display levels are collected.
pub fn subscriber(trace: u8, output: Output, process_tree: Option<Box<ProcessTree>>) -> Result<tracing::Dispatch> {
    let (forest_level, flat_level) = trace_settings(trace)?;
    let collection_level = if process_tree.is_some() {
        LevelFilter::TRACE
    } else {
        forest_level
    };
    if collection_level == LevelFilter::OFF && flat_level == LevelFilter::OFF {
        return Ok(tracing::Dispatch::none());
    }
    let writer = move || Writer(output.clone());
    let forest = (collection_level != LevelFilter::OFF).then(|| {
        let printer = Printer::new()
            .writer(writer.clone())
            .formatter(Pretty.with_max_level(forest_level));
        gix_trace::ForestLayer::from(processor::from_fn(move |tree| {
            if let Some(process_tree) = &process_tree
                && let Err(err) = process_tree(&tree)
            {
                return Err(processor::error(tree, err));
            }
            if forest_level == LevelFilter::OFF {
                Ok(())
            } else {
                printer.process(tree)
            }
        }))
        .with_filter(collection_level)
    });
    let flat = (flat_level != LevelFilter::OFF).then(|| {
        tracing_subscriber::fmt::layer()
            .with_ansi(true)
            .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
            .with_writer(writer)
            .with_filter(flat_level)
    });
    Ok(tracing::Dispatch::new(
        tracing_subscriber::registry().with(forest).with(flat),
    ))
}

fn trace_settings(trace: u8) -> Result<(LevelFilter, LevelFilter)> {
    Ok(match trace {
        0 => (LevelFilter::OFF, LevelFilter::OFF),
        1 => (LevelFilter::INFO, LevelFilter::OFF),
        2 => (LevelFilter::DEBUG, LevelFilter::OFF),
        3 => (LevelFilter::OFF, LevelFilter::DEBUG),
        4 => (LevelFilter::OFF, LevelFilter::TRACE),
        _ => bail!(gix::error::validation("trace level must be between zero and four")),
    })
}

// Both formatters write each completed tree or event as one buffer.
struct Writer(Output);

impl Write for Writer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use gix::error::TestResult;

    use super::{Output, subscriber, trace_settings};
    use tracing_subscriber::filter::LevelFilter;

    #[test]
    fn trace_repetitions_choose_format_and_level() -> TestResult<()> {
        for (trace, expected) in [
            (0, (LevelFilter::OFF, LevelFilter::OFF)),
            (1, (LevelFilter::INFO, LevelFilter::OFF)),
            (2, (LevelFilter::DEBUG, LevelFilter::OFF)),
            (3, (LevelFilter::OFF, LevelFilter::DEBUG)),
            (4, (LevelFilter::OFF, LevelFilter::TRACE)),
        ] {
            assert_eq!(
                trace_settings(trace)?,
                expected,
                "each repetition selects its display mode"
            );
        }
        let error = trace_settings(5).expect_err("only four trace display levels exist");
        assert_eq!(
            error.error().to_string(),
            "trace level must be between zero and four",
            "invalid levels report the supported range independently of source locations"
        );
        Ok(())
    }

    #[test]
    fn disabled_display_without_a_processor_is_a_noop() -> TestResult<()> {
        let output = Output::default();
        let dispatch = subscriber(0, output.clone(), None)?;
        tracing::dispatcher::with_default(&dispatch, || {
            assert!(
                !tracing::enabled!(tracing::Level::ERROR),
                "disabled tracing collects nothing"
            );
            tracing::info!("not displayed");
        });
        assert!(output.lock().expect("trace output lock is not poisoned").is_empty());
        Ok(())
    }

    #[test]
    fn flat_traces_include_closed_spans_in_the_deferred_output() -> TestResult<()> {
        let output = Output::default();
        let dispatch = subscriber(3, output.clone(), None)?;
        tracing::dispatcher::with_default(&dispatch, || {
            let span = tracing::debug_span!("operation");
            let _entered = span.enter();
            tracing::debug!("visible event");
            tracing::trace!("filtered event");
        });
        let output = output.lock().expect("trace output lock is not poisoned");
        let output = String::from_utf8_lossy(&output);
        assert_eq!(
            output.matches("visible event").count(),
            1,
            "debug events are retained: {output}"
        );
        assert!(output.contains("close"), "span completion is retained: {output}");
        assert!(output.contains("\x1b["), "flat terminal traces contain ANSI styling");
        assert!(
            !output.contains("filtered event"),
            "the selected level still filters: {output}"
        );
        Ok(())
    }
}
