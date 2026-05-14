// Modified for gitoxide from the upstream imara-diff crate.
// Upstream source: git cat-file -p 32d1e45d3df061e6ccba6db7fdce92db29e345d8:src/unified_diff.rs

use std::fmt::{self, Display};
use std::hash::Hash;

use crate::Diff;
use crate::intern::{InternedInput, Interner, Token};

impl Diff {
    /// Creates a unified diff output that can be formatted as a string.
    ///
    /// This is a convenience method that extracts the token sequences from the `input`.
    ///
    /// # Parameters
    ///
    /// * `printer` - A printer implementation that controls how tokens are displayed
    /// * `config` - Configuration options for the unified diff format
    /// * `input` - The interned input containing the token sequences
    pub fn unified_diff<'a, P: UnifiedDiffPrinter, T: Hash + Eq>(
        &'a self,
        printer: &'a P,
        config: UnifiedDiffConfig,
        input: &'a InternedInput<T>,
    ) -> UnifiedDiff<'a, P> {
        self.unified_diff_with(printer, config, &input.before, &input.after)
    }

    /// Creates a unified diff output with explicit token sequences.
    ///
    /// # Parameters
    ///
    /// * `printer` - A printer implementation that controls how tokens are displayed
    /// * `config` - Configuration options for the unified diff format
    /// * `before` - The token sequence from the first file, before changes.
    /// * `after` - The token sequence from the second file, after changes.
    pub fn unified_diff_with<'a, P: UnifiedDiffPrinter>(
        &'a self,
        printer: &'a P,
        config: UnifiedDiffConfig,
        before: &'a [Token],
        after: &'a [Token],
    ) -> UnifiedDiff<'a, P> {
        UnifiedDiff {
            printer,
            diff: self,
            config,
            before,
            after,
        }
    }
}

/// A trait for customizing the output format of unified diffs.
///
/// Implementations of this trait control how different parts of a unified diff are displayed,
/// including headers, context lines, and changed hunks.
pub trait UnifiedDiffPrinter {
    /// Displays the header for a hunk in the unified diff format.
    ///
    /// The header typically includes the line numbers and lengths for both files.
    ///
    /// # Parameters
    ///
    /// * `f` - The formatter to write to
    /// * `start_before` - The starting line number in the first file (0-indexed)
    /// * `start_after` - The starting line number in the second file (0-indexed)
    /// * `len_before` - The number of lines from the first file in this hunk
    /// * `len_after` - The number of lines from the second file in this hunk
    fn display_header(
        &self,
        f: impl fmt::Write,
        start_before: u32,
        start_after: u32,
        len_before: u32,
        len_after: u32,
    ) -> fmt::Result;
    /// Displays a context token (an unchanged line) in the unified diff.
    ///
    /// # Parameters
    ///
    /// * `f` - The formatter to write to
    /// * `token` - The token to display
    fn display_context_token(&self, f: impl fmt::Write, token: Token) -> fmt::Result;
    /// Displays a hunk showing the changes between before and after tokens.
    ///
    /// # Parameters
    ///
    /// * `f` - The formatter to write to
    /// * `before` - The tokens from the first file that were removed
    /// * `after` - The tokens from the second file that were added
    fn display_hunk(&self, f: impl fmt::Write, before: &[Token], after: &[Token]) -> fmt::Result;
}

/// Configuration options for unified diff output.
///
/// Controls aspects of the unified diff format such as the number of context lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnifiedDiffConfig {
    /// The number of unchanged lines to show around each hunk for context.
    context_len: u32,
}

impl Default for UnifiedDiffConfig {
    fn default() -> Self {
        UnifiedDiffConfig { context_len: 3 }
    }
}

impl UnifiedDiffConfig {
    /// Sets the number of context lines to display around each hunk.
    ///
    /// # Parameters
    ///
    /// * `len` - The number of unchanged lines to show before and after each change
    ///
    /// # Returns
    ///
    /// A mutable reference to self for method chaining
    pub fn context_len(&mut self, len: u32) -> &mut Self {
        self.context_len = len;
        self
    }
}

/// A helper trait for determining if a token ends with a newline.
///
/// This is used by the unified diff printer to decide whether to add newlines
/// when displaying tokens.
pub trait EndsWithNewline {
    /// Returns `true` if the token ends with a newline character.
    fn ends_with_newline(&self) -> bool;
}

impl<T: AsRef<[u8]> + ?Sized> EndsWithNewline for T {
    fn ends_with_newline(&self) -> bool {
        self.as_ref().ends_with(b"\n")
    }
}

/// A basic implementation of [`UnifiedDiffPrinter`] for line-based diffs.
///
/// This printer formats diffs in the standard unified diff format commonly used by
/// tools like `git diff` and `diff -u`. It displays removed lines with a `-` prefix
/// and added lines with a `+` prefix.
pub struct BasicLineDiffPrinter<'a, T: EndsWithNewline + ?Sized + Hash + Eq + Display>(
    /// A reference to the interner containing the line data.
    pub &'a Interner<&'a T>,
);

impl<T: EndsWithNewline + Hash + Eq + Display + ?Sized> UnifiedDiffPrinter for BasicLineDiffPrinter<'_, T> {
    fn display_header(
        &self,
        mut f: impl fmt::Write,
        start_before: u32,
        start_after: u32,
        len_before: u32,
        len_after: u32,
    ) -> fmt::Result {
        writeln!(
            f,
            "@@ -{},{} +{},{} @@",
            start_before + 1,
            len_before,
            start_after + 1,
            len_after
        )
    }

    fn display_context_token(&self, mut f: impl fmt::Write, token: Token) -> fmt::Result {
        write!(f, " {}", &self.0[token])?;
        if !&self.0[token].ends_with_newline() {
            writeln!(f)?;
        }
        Ok(())
    }

    fn display_hunk(&self, mut f: impl fmt::Write, before: &[Token], after: &[Token]) -> fmt::Result {
        if let Some(&last) = before.last() {
            for &token in before {
                let token = self.0[token];
                write!(f, "-{token}")?;
            }
            if !self.0[last].ends_with_newline() {
                writeln!(f)?;
            }
        }
        if let Some(&last) = after.last() {
            for &token in after {
                let token = self.0[token];
                write!(f, "+{token}")?;
            }
            if !self.0[last].ends_with_newline() {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

/// A formatted unified diff that can be displayed as a string.
///
/// This structure is created by [`Diff::unified_diff`] or [`Diff::unified_diff_with`]
/// and implements [`Display`] to produce standard unified diff output.
pub struct UnifiedDiff<'a, P: UnifiedDiffPrinter> {
    /// The printer that controls output formatting.
    printer: &'a P,
    /// The computed diff to display.
    diff: &'a Diff,
    /// Configuration for the unified diff format.
    config: UnifiedDiffConfig,
    /// The token sequence from the first file, before changes.
    before: &'a [Token],
    /// The token sequence from the second file, after changes.
    after: &'a [Token],
}

impl<P: UnifiedDiffPrinter> Display for UnifiedDiff<'_, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let first_hunk = self.diff.hunks().next().unwrap_or_default();
        let context_len = self.config.context_len.min(1024 * 1024);
        let mut pos = first_hunk.before.start.saturating_sub(context_len);
        let mut before_context_start = pos;
        let mut after_context_start = first_hunk.after.start.saturating_sub(context_len);
        let mut before_context_len = 0;
        let mut after_context_len = 0;
        let mut buffer = String::new();
        for hunk in self.diff.hunks() {
            if hunk.before.start - pos > 2 * context_len {
                if !buffer.is_empty() {
                    let end = (pos + context_len).min(self.before.len() as u32);
                    self.printer.display_header(
                        &mut *f,
                        before_context_start,
                        after_context_start,
                        before_context_len + end - pos,
                        after_context_len + end - pos,
                    )?;
                    write!(f, "{buffer}")?;
                    for &token in &self.before[pos as usize..end as usize] {
                        self.printer.display_context_token(&mut *f, token)?;
                    }
                    buffer.clear();
                }
                pos = hunk.before.start - context_len;
                before_context_start = pos;
                after_context_start = hunk.after.start - context_len;
                before_context_len = 0;
                after_context_len = 0;
            }
            for &token in &self.before[pos as usize..hunk.before.start as usize] {
                self.printer.display_context_token(&mut buffer, token)?;
            }
            let context_len = hunk.before.start - pos;
            before_context_len += hunk.before.len() as u32 + context_len;
            after_context_len += hunk.after.len() as u32 + context_len;
            self.printer.display_hunk(
                &mut buffer,
                &self.before[hunk.before.start as usize..hunk.before.end as usize],
                &self.after[hunk.after.start as usize..hunk.after.end as usize],
            )?;
            pos = hunk.before.end;
        }
        if !buffer.is_empty() {
            let end = (pos + context_len).min(self.before.len() as u32);
            self.printer.display_header(
                &mut *f,
                before_context_start,
                after_context_start,
                before_context_len + end - pos,
                after_context_len + end - pos,
            )?;
            write!(f, "{buffer}")?;
            for &token in &self.before[pos as usize..end as usize] {
                self.printer.display_context_token(&mut *f, token)?;
            }
            buffer.clear();
        }
        Ok(())
    }
}
