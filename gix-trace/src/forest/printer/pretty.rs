use crate::forest::{
    Formatter, Tag, Tree,
    tree::{Event, Shared, Span},
};
use std::fmt::{self, Write};
use tracing::metadata::LevelFilter;

type IndentVec = smallvec::SmallVec<[Indent; 32]>;

#[cfg(feature = "forest-ansi")]
use nu_ansi_term::Color;
#[cfg(feature = "forest-ansi")]
use tracing::Level;

/// Format logs for pretty printing.
///
/// # Interpreting span times
///
/// Spans have the following format:
/// ```txt
/// <NAME> [ <DURATION> | <BODY> / <ROOT> ]
/// ```
/// * DURATION represents elapsed time while the span has at least one active entry.
///   Overlapping entries into the same span count once. If the span instruments a
///   `Future`, time between polls does not count because the span is not entered then.
/// * BODY represents the percent time the span is entered relative to the root
///   span, *excluding* time that any child spans are entered.
/// * ROOT represents the percent time the span is entered relative to the root
///   span, *including* time that any child spans are entered.
///
/// As a mental model, look at `ROOT` to quickly narrow down which branches are
/// costly, and look at `BODY` to pinpoint exactly which spans are expensive.
///
/// Spans without any child spans would have the same `BODY` and `ROOT`, so the
/// redundancy is omitted.
/// Durations accumulate child work: overlapping children contribute separately,
/// and a parent's total is raised to at least their sum. Totals can therefore
/// exceed wall-clock time; see [`Span::total_duration`].
/// A root with zero duration displays `0.00%` for all percentages.
///
/// With `forest-cpu-time`, an additional `[ user: <TIME> | sys: <TIME> ]` shows
/// inclusive user and kernel CPU time. This is omitted on unsupported platforms
/// or when measurements are incomplete. CPU times exclude blocked time and
/// include parallel children; elapsed-time percentages are unchanged.
///
/// Use [`Pretty::with_max_level`] to omit less important nodes from the output
/// while retaining their more important descendants and the original durations.
///
/// # Examples
///
/// An arbitrarily complex example:
/// ```log
/// INFO     try_from_entry_ro [ 324µs | 8.47% / 100.00% ]
/// INFO     ┝━ server::internal_search [ 296µs | 19.02% / 91.53% ]
/// INFO     │  ┝━ ｉ [filter.info]: Some filter info...
/// INFO     │  ┝━ server::search [ 226µs | 10.11% / 70.01% ]
/// INFO     │  │  ┝━ be::search [ 181µs | 6.94% / 55.85% ]
/// INFO     │  │  │  ┕━ be::search -> filter2idl [ 158µs | 19.65% / 48.91% ]
/// INFO     │  │  │     ┝━ be::idl_arc_sqlite::get_idl [ 20.4µs | 6.30% ]
/// INFO     │  │  │     │  ┕━ ｉ [filter.info]: Some filter info...
/// INFO     │  │  │     ┕━ be::idl_arc_sqlite::get_idl [ 74.3µs | 22.96% ]
/// ERROR    │  │  │        ┝━ 🚨 [admin.error]: On no, an admin error occurred :(
/// DEBUG    │  │  │        ┝━ 🐛 [debug]: An untagged debug log
/// INFO     │  │  │        ┕━ ｉ [admin.info]: there's been a big mistake | alive: false | status: "very sad"
/// INFO     │  │  ┕━ be::idl_arc_sqlite::get_identry [ 13.1µs | 4.04% ]
/// ERROR    │  │     ┝━ 🔐 [security.critical]: A security critical log
/// INFO     │  │     ┕━ 🔓 [security.access]: A security access log
/// INFO     │  ┕━ server::search<filter_resolve> [ 8.08µs | 2.50% ]
/// WARN     │     ┕━ 🚧 [filter.warn]: Some filter warning
/// TRACE    ┕━ 📍 [trace]: Finished!
/// ```
#[derive(Debug)]
pub struct Pretty;

impl Formatter for Pretty {
    type Error = fmt::Error;

    fn fmt(&self, tree: &Tree) -> Result<String, fmt::Error> {
        Pretty.with_max_level(LevelFilter::TRACE).fmt(tree)
    }
}

/// Pretty-print only nodes at or above a selected importance level.
///
/// Construct this formatter with [`Pretty::with_max_level`]. Hidden spans' visible
/// descendants are promoted to their nearest visible ancestor, or to the top level
/// if there is none. Percentages retain the full, unfiltered root's duration.
#[derive(Debug)]
pub struct FilteredPretty {
    max_level: LevelFilter,
}

impl Formatter for FilteredPretty {
    type Error = fmt::Error;

    fn fmt(&self, tree: &Tree) -> Result<String, fmt::Error> {
        if self.max_level == LevelFilter::OFF {
            return Ok(String::new());
        }
        let mut writer = String::with_capacity(256);
        let root_duration = match tree {
            Tree::Span(span) => span.total_duration().as_nanos() as f64,
            Tree::Event(_) => 0.0,
        };
        let mut indent = IndentVec::new();
        for node in visible_nodes(std::slice::from_ref(tree), self.max_level) {
            Pretty::format_tree(node, root_duration, self.max_level, &mut indent, &mut writer)?;
        }
        Ok(writer)
    }
}

impl Pretty {
    /// Select the most verbose level to include in formatted output.
    ///
    /// For example, [`LevelFilter::WARN`] retains warnings and errors. Visible
    /// descendants of omitted spans are promoted without changing their order or
    /// the durations used for percentages. [`LevelFilter::OFF`] produces an empty
    /// string, and [`LevelFilter::TRACE`] produces the same output as `Pretty`.
    /// This filters formatting only; the collected tree is unchanged.
    ///
    /// ```
    /// use gix_trace::forest::{Printer, printer::Pretty};
    /// use tracing::metadata::LevelFilter;
    ///
    /// let printer = Printer::new().formatter(Pretty.with_max_level(LevelFilter::WARN));
    /// ```
    pub const fn with_max_level(self, max_level: LevelFilter) -> FilteredPretty {
        FilteredPretty { max_level }
    }

    fn format_tree(
        tree: &Tree,
        root_duration: f64,
        max_level: LevelFilter,
        indent: &mut IndentVec,
        writer: &mut String,
    ) -> fmt::Result {
        match tree {
            Tree::Event(event) => {
                Pretty::format_shared(&event.shared, writer)?;
                Pretty::format_indent(indent, writer)?;
                Pretty::format_event(event, writer)
            }
            Tree::Span(span) => {
                Pretty::format_shared(&span.shared, writer)?;
                Pretty::format_indent(indent, writer)?;
                Pretty::format_span(span, root_duration, max_level, indent, writer)
            }
        }
    }

    fn format_shared(shared: &Shared, writer: &mut String) -> fmt::Result {
        #[cfg(feature = "forest-ansi")]
        let level = ColorLevel(shared.level);
        #[cfg(not(feature = "forest-ansi"))]
        let level = shared.level;
        write!(writer, "{level:<8} ")
    }

    fn format_indent(indent: &[Indent], writer: &mut String) -> fmt::Result {
        for indent in indent {
            writer.write_str(indent.repr())?;
        }
        Ok(())
    }

    fn format_event(event: &Event, writer: &mut String) -> fmt::Result {
        let tag = event.tag().unwrap_or_else(|| Tag::from(event.level()));

        write!(writer, "{} [{tag}]: ", tag.icon())?;

        if let Some(message) = event.message() {
            writer.write_str(message)?;
        }

        for field in event.fields() {
            write!(writer, " | {}: {}", FieldKey(field.key()), field.value())?;
        }

        writeln!(writer)
    }

    fn format_span(
        span: &Span,
        root_duration: f64,
        max_level: LevelFilter,
        indent: &mut IndentVec,
        writer: &mut String,
    ) -> fmt::Result {
        let total_duration = span.total_duration().as_nanos() as f64;
        let inner_duration = span.inner_duration().as_nanos() as f64;
        let percentage = |duration| {
            if root_duration == 0.0 {
                0.0
            } else {
                100.0 * duration / root_duration
            }
        };
        let percent_total_of_root_duration = percentage(total_duration);

        write!(writer, "{} [ {} | ", span.name(), DurationDisplay(total_duration))?;

        if inner_duration > 0.0 {
            let base_duration = span.base_duration().as_nanos() as f64;
            let percent_base_of_root_duration = percentage(base_duration);
            write!(writer, "{percent_base_of_root_duration:.2}% / ")?;
        }

        write!(writer, "{percent_total_of_root_duration:.2}% ]")?;

        #[cfg(feature = "forest-cpu-time")]
        if let Some(cpu) = span.total_cpu_time() {
            write!(
                writer,
                " [ user: {} | sys: {} ]",
                DurationDisplay(cpu.user.as_nanos() as f64),
                DurationDisplay(cpu.system.as_nanos() as f64),
            )?;
        }

        for (n, field) in span.shared.fields.iter().enumerate() {
            write!(
                writer,
                "{} {}: {}",
                if n == 0 { "" } else { " |" },
                FieldKey(field.key()),
                field.value()
            )?;
        }
        writeln!(writer)?;

        let mut children = visible_nodes(span.nodes(), max_level).peekable();
        if children.peek().is_some() {
            match indent.last_mut() {
                Some(edge @ Indent::Turn) => *edge = Indent::Null,
                Some(edge @ Indent::Fork) => *edge = Indent::Line,
                _ => {}
            }

            indent.push(Indent::Fork);

            while let Some(tree) = children.next() {
                if let Some(edge) = indent.last_mut() {
                    *edge = if children.peek().is_some() {
                        Indent::Fork
                    } else {
                        Indent::Turn
                    };
                }
                Pretty::format_tree(tree, root_duration, max_level, indent, writer)?;
            }

            indent.pop();
        }

        Ok(())
    }
}

fn visible_nodes(nodes: &[Tree], max_level: LevelFilter) -> impl Iterator<Item = &Tree> {
    let mut stack = smallvec::SmallVec::<[std::slice::Iter<'_, Tree>; 8]>::new();
    stack.push(nodes.iter());
    std::iter::from_fn(move || {
        loop {
            let nodes = stack.last_mut()?;
            let Some(tree) = nodes.next() else {
                stack.pop();
                continue;
            };
            match tree {
                Tree::Event(event) if event.level() <= max_level => return Some(tree),
                Tree::Span(span) if span.level() <= max_level => return Some(tree),
                Tree::Span(span) => stack.push(span.nodes().iter()),
                Tree::Event(_) => {}
            }
        }
    })
}

enum Indent {
    Null,
    Line,
    Fork,
    Turn,
}

impl Indent {
    fn repr(&self) -> &'static str {
        match self {
            Self::Null => "   ",
            Self::Line => "│  ",
            Self::Fork => "┝━ ",
            Self::Turn => "┕━ ",
        }
    }
}

struct DurationDisplay(f64);

// Taken from chrono
impl fmt::Display for DurationDisplay {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut t = self.0;
        for unit in ["ns", "µs", "ms", "s"] {
            if t < 10.0 {
                return write!(f, "{t:.2}{unit}");
            } else if t < 100.0 {
                return write!(f, "{t:.1}{unit}");
            } else if t < 1000.0 {
                return write!(f, "{t:.0}{unit}");
            }
            t /= 1000.0;
        }
        write!(f, "{:.0}s", t * 1000.0)
    }
}

/// Implements colored formatting for a field if enabled
struct FieldKey<'a>(&'a str);

impl fmt::Display for FieldKey<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(feature = "forest-ansi")]
        {
            let color = Color::White.dimmed();

            write!(f, "{}{}{}", color.prefix(), self.0, color.suffix())
        }
        #[cfg(not(feature = "forest-ansi"))]
        {
            f.write_str(self.0)
        }
    }
}

// From tracing-tree
#[cfg(feature = "forest-ansi")]
struct ColorLevel(Level);

#[cfg(feature = "forest-ansi")]
impl fmt::Display for ColorLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let color = match self.0 {
            Level::TRACE => Color::Purple,
            Level::DEBUG => Color::Blue,
            Level::INFO => Color::Green,
            Level::WARN => Color::Rgb(252, 234, 160), // orange
            Level::ERROR => Color::Red,
        };
        let style = color.bold();
        write!(f, "{}", style.prefix())?;
        f.pad(self.0.as_str())?;
        write!(f, "{}", style.suffix())
    }
}

#[cfg(all(test, feature = "forest-cpu-time"))]
mod tests {
    use super::*;

    #[test]
    fn unavailable_cpu_time_is_omitted() -> gix_error::TestResult {
        let zero = Some(crate::forest::tree::CpuTime::default());
        for (base, inner) in [(None, None), (None, zero), (zero, None)] {
            let mut span = Span::new(
                Shared {
                    level: tracing::Level::INFO,
                    fields: Default::default(),
                },
                "operation",
            );
            span.base_cpu_time = base;
            span.inner_cpu_time = inner;
            let rendered = Pretty.fmt(&Tree::Span(span))?;
            assert!(
                rendered.ends_with("operation [ 0.00ns | 0.00% ]\n"),
                "missing own or child CPU measurements leave only elapsed timing: {rendered:?}"
            );
        }
        Ok(())
    }
}
