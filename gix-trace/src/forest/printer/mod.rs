//! Utilities for formatting and writing trace trees.
use crate::forest::{Formatter, PrettyPrinter, Printer, Processor, TestCapturePrinter, Tree, processor};
use gix_error::ResultExt;
use std::error::Error;
use std::io::{self, Write};
use tracing_subscriber::fmt::MakeWriter;

mod pretty;
pub use pretty::{FilteredPretty, Pretty};

impl<F, E> Formatter for F
where
    F: Fn(&Tree) -> Result<String, E>,
    E: Error + Send + Sync,
{
    type Error = E;

    #[inline]
    fn fmt(&self, tree: &Tree) -> Result<String, E> {
        self(tree)
    }
}

/// A [`MakeWriter`] that writes to stdout.
///
/// This is functionally the same as using [`std::io::stdout`] as a `MakeWriter`,
/// except it has a named type and can therefore be used in type signatures.
#[derive(Debug)]
pub struct MakeStdout;

/// A [`MakeWriter`] that writes to stderr.
///
/// This is functionally the same as using [`std::io::stderr`] as a `MakeWriter`,
/// except it has a named type and can therefore be used in type signatures.
#[derive(Debug)]
pub struct MakeStderr;

impl MakeWriter<'_> for MakeStdout {
    type Writer = io::Stdout;

    fn make_writer(&self) -> Self::Writer {
        io::stdout()
    }
}

impl MakeWriter<'_> for MakeStderr {
    type Writer = io::Stderr;

    fn make_writer(&self) -> Self::Writer {
        io::stderr()
    }
}

impl PrettyPrinter {
    /// Returns a new [`PrettyPrinter`] that pretty-prints to stdout.
    ///
    /// Use [`Printer::formatter`] and [`Printer::writer`] for custom configuration.
    pub const fn new() -> Self {
        Printer {
            formatter: Pretty,
            make_writer: MakeStdout,
        }
    }
}

impl<F, W> Printer<F, W>
where
    F: 'static + Formatter,
    W: 'static + for<'a> MakeWriter<'a>,
{
    /// Set the formatter.
    ///
    /// See the [`Formatter`] trait for details on possible inputs.
    pub fn formatter<F2>(self, formatter: F2) -> Printer<F2, W>
    where
        F2: 'static + Formatter,
    {
        Printer {
            formatter,
            make_writer: self.make_writer,
        }
    }

    /// Set the writer.
    pub fn writer<W2>(self, make_writer: W2) -> Printer<F, W2>
    where
        W2: 'static + for<'a> MakeWriter<'a>,
    {
        Printer {
            formatter: self.formatter,
            make_writer,
        }
    }
}

impl Default for PrettyPrinter {
    fn default() -> Self {
        PrettyPrinter::new()
    }
}

impl<F, W> Processor for Printer<F, W>
where
    F: 'static + Formatter,
    W: 'static + for<'a> MakeWriter<'a>,
{
    fn process(&self, tree: Tree) -> processor::Result {
        let string = match self.formatter.fmt(&tree).or_error() {
            Ok(s) => s,
            Err(e) => return Err(processor::error(tree, e)),
        };

        match self.make_writer.make_writer().write_all(string.as_bytes()).or_error() {
            Ok(()) => Ok(()),
            Err(e) => Err(processor::error(tree, e)),
        }
    }
}

impl TestCapturePrinter<Pretty> {
    /// Construct a new test capturing printer with the default `Pretty` formatter. This printer
    /// is intended for use in tests only as it works with Rust's default stdout capture mechanism.
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        TestCapturePrinter { formatter: Pretty }
    }
}

impl<F> Processor for TestCapturePrinter<F>
where
    F: 'static + Formatter,
{
    fn process(&self, tree: Tree) -> processor::Result {
        let string = self
            .formatter
            .fmt(&tree)
            .or_error()
            .map_err(|e| processor::error(tree, e))?;

        print!("{string}");
        Ok(())
    }
}
