//! Facilities to produce the unified diff format.
//!
//! Originally based on <https://github.com/pascalkuthe/imara-diff/pull/14>.

/// Defines the size of the context printed before and after each change.
///
/// Similar to the `-U` option in git diff or gnu-diff. If the context overlaps
/// with previous or next change, the context gets reduced accordingly.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Ord, PartialOrd)]
pub struct ContextSize {
    /// Defines the size of the context printed before and after each change.
    symmetrical: u32,
}

impl Default for ContextSize {
    fn default() -> Self {
        ContextSize::symmetrical(3)
    }
}

/// Instantiation
impl ContextSize {
    /// Create a symmetrical context with `n` lines before and after a changed hunk.
    pub fn symmetrical(n: u32) -> Self {
        ContextSize { symmetrical: n }
    }
}

/// Represents the type of a line in a unified diff.
#[doc(alias = "git2")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineType {
    /// A line that exists in both the old and the new version is called a context line.
    Context,
    /// A line that was added in the new version.
    Add,
    /// A line that was removed from the old version.
    Remove,
}

impl DiffLineType {
    const fn to_prefix(self) -> char {
        match self {
            DiffLineType::Context => ' ',
            DiffLineType::Add => '+',
            DiffLineType::Remove => '-',
        }
    }
}

/// Specify where to put a newline.
#[derive(Debug, Copy, Clone)]
pub enum NewlineSeparator<'a> {
    /// Place the given newline separator, like `\n`, after each patch header as well as after each line.
    /// This is the right choice if tokens don't include newlines.
    AfterHeaderAndLine(&'a str),
    /// Place the given newline separator, like `\n`, only after each patch header or if a line doesn't contain a newline.
    /// This is the right choice if tokens do include newlines.
    /// Note that diff-tokens *with* newlines may diff strangely at the end of files when lines have been appended,
    /// as it will make the last line look like it changed just because the whitespace at the end 'changed'.
    AfterHeaderAndWhenNeeded(&'a str),
}

/// Holds information about a unified diff hunk, specifically with respect to line numbers.
pub struct HunkHeader {
    /// The 1-based start position in the 'before' lines.
    pub before_hunk_start: u32,
    /// The size of the 'before' hunk in lines.
    pub before_hunk_len: u32,
    /// The 1-based start position in the 'after' lines.
    pub after_hunk_start: u32,
    /// The size of the 'after' hunk in lines.
    pub after_hunk_len: u32,
}

impl std::fmt::Display for HunkHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "@@ -{},{} +{},{} @@",
            self.before_hunk_start, self.before_hunk_len, self.after_hunk_start, self.after_hunk_len
        )
    }
}

/// A utility trait for use in [`UnifiedDiff`](super::UnifiedDiff).
pub trait ConsumeHunk {
    /// The item this instance produces after consuming all hunks.
    type Out;

    /// Consume a single hunk. Note that it is the implementation's responsibility to add newlines
    /// where requested by `newline`.
    ///
    /// Note that the [`UnifiedDiff`](super::UnifiedDiff) sink will wrap its output in an [`std::io::Result`].
    /// After this method returned its first error, it will not be called anymore.
    fn consume_hunk(
        &mut self,
        header: HunkHeader,
        lines: &[(DiffLineType, &[u8])],
        newline: NewlineSeparator<'_>,
    ) -> std::io::Result<()>;

    /// Called after the last hunk is consumed to produce an output.
    fn finish(self) -> Self::Out;
}

pub(super) mod _impl {
    use std::{hash::Hash, io::ErrorKind, ops::Range};

    use bstr::{ByteSlice, ByteVec};
    use imara_diff::{intern, Sink};
    use intern::{InternedInput, Interner, Token};

    use super::{ConsumeHunk, ContextSize, DiffLineType, HunkHeader, NewlineSeparator};

    /// A [`Sink`] that creates a unified diff. It can be used to create a textual diff in the
    /// format typically output by `git` or `gnu-diff` if the `-u` option is used.
    pub struct UnifiedDiff<'a, T, D>
    where
        T: Hash + Eq + AsRef<[u8]>,
        D: ConsumeHunk,
    {
        before: &'a [Token],
        after: &'a [Token],
        interner: &'a Interner<T>,

        /// The 0-based start position in the 'before' tokens for the accumulated hunk for display in the header.
        before_hunk_start: u32,
        /// The size of the accumulated 'before' hunk in lines for display in the header.
        before_hunk_len: u32,
        /// The 0-based start position in the 'after' tokens for the accumulated hunk for display in the header.
        after_hunk_start: u32,
        /// The size of the accumulated 'after' hunk in lines.
        after_hunk_len: u32,
        // An index into `before` and the context line to print next,
        // or `None` if this value was never computed to be the correct starting point for an accumulated hunk.
        ctx_pos: Option<u32>,

        /// Symmetrical context before and after the changed hunk.
        ctx_size: u32,
        newline: NewlineSeparator<'a>,

        buffer: Vec<(DiffLineType, Vec<u8>)>,

        delegate: D,

        err: Option<std::io::Error>,
    }

    impl<'a, T, D> UnifiedDiff<'a, T, D>
    where
        T: Hash + Eq + AsRef<[u8]>,
        D: ConsumeHunk,
    {
        /// Create a new instance to create a unified diff using the lines in `input`,
        /// which also must be used when running the diff algorithm.
        /// `context_size` is the amount of lines around each hunk which will be passed
        /// to `consume_hunk`.
        ///
        /// `consume_hunk` is called for each hunk with all the information required to create a
        /// unified diff.
        pub fn new(
            input: &'a InternedInput<T>,
            consume_hunk: D,
            newline_separator: NewlineSeparator<'a>,
            context_size: ContextSize,
        ) -> Self {
            Self {
                interner: &input.interner,
                before: &input.before,
                after: &input.after,

                before_hunk_start: 0,
                before_hunk_len: 0,
                after_hunk_len: 0,
                after_hunk_start: 0,
                ctx_pos: None,

                ctx_size: context_size.symmetrical,
                newline: newline_separator,

                buffer: Vec::with_capacity(8),
                delegate: consume_hunk,

                err: None,
            }
        }

        fn print_tokens(&mut self, tokens: &[Token], line_type: DiffLineType) {
            for &token in tokens {
                let content = self.interner[token].as_ref().to_vec();
                self.buffer.push((line_type, content));
            }
        }

        fn flush_accumulated_hunk(&mut self) -> std::io::Result<()> {
            if self.nothing_to_flush() {
                return Ok(());
            }

            let ctx_pos = self.ctx_pos.expect("has been set if we started a hunk");
            let end = (ctx_pos + self.ctx_size).min(self.before.len() as u32);
            self.print_context_and_update_pos(ctx_pos..end, end);

            let hunk_start = self.before_hunk_start + 1;
            let hunk_end = self.after_hunk_start + 1;

            let header = HunkHeader {
                before_hunk_start: hunk_start,
                before_hunk_len: self.before_hunk_len,
                after_hunk_start: hunk_end,
                after_hunk_len: self.after_hunk_len,
            };

            // TODO:
            // If I remember correctly, we wanted this buffer to be reused between calls, but I ran
            // into lifetime issues when I tried to make it a field on `UnifiedDiff`.
            let buffer2: Vec<(DiffLineType, &[u8])> = self
                .buffer
                .iter()
                .map(|(line_type, content)| (*line_type, content.as_slice()))
                .collect();

            self.delegate.consume_hunk(header, &buffer2, self.newline)?;

            self.reset_hunks();
            Ok(())
        }

        fn print_context_and_update_pos(&mut self, print: Range<u32>, move_to: u32) {
            self.print_tokens(
                &self.before[print.start as usize..print.end as usize],
                DiffLineType::Context,
            );

            let len = print.end - print.start;
            self.ctx_pos = Some(move_to);
            self.before_hunk_len += len;
            self.after_hunk_len += len;
        }

        fn reset_hunks(&mut self) {
            self.buffer.clear();
            self.before_hunk_len = 0;
            self.after_hunk_len = 0;
        }

        fn nothing_to_flush(&self) -> bool {
            self.before_hunk_len == 0 && self.after_hunk_len == 0
        }
    }

    impl<T, D> Sink for UnifiedDiff<'_, T, D>
    where
        T: Hash + Eq + AsRef<[u8]>,
        D: ConsumeHunk,
    {
        type Out = std::io::Result<D::Out>;

        fn process_change(&mut self, before: Range<u32>, after: Range<u32>) {
            if self.err.is_some() {
                return;
            }
            let start_next_hunk = self
                .ctx_pos
                .is_some_and(|ctx_pos| before.start - ctx_pos > 2 * self.ctx_size);
            if start_next_hunk {
                if let Err(err) = self.flush_accumulated_hunk() {
                    self.err = Some(err);
                    return;
                }
                let ctx_pos = before.start - self.ctx_size;
                self.ctx_pos = Some(ctx_pos);
                self.before_hunk_start = ctx_pos;
                self.after_hunk_start = after.start - self.ctx_size;
            }
            let ctx_pos = match self.ctx_pos {
                None => {
                    // TODO: can this be made so the code above does the job?
                    let ctx_pos = before.start.saturating_sub(self.ctx_size);
                    self.before_hunk_start = ctx_pos;
                    self.after_hunk_start = after.start.saturating_sub(self.ctx_size);
                    ctx_pos
                }
                Some(pos) => pos,
            };
            self.print_context_and_update_pos(ctx_pos..before.start, before.end);
            self.before_hunk_len += before.end - before.start;
            self.after_hunk_len += after.end - after.start;

            self.print_tokens(
                &self.before[before.start as usize..before.end as usize],
                DiffLineType::Remove,
            );
            self.print_tokens(&self.after[after.start as usize..after.end as usize], DiffLineType::Add);
        }

        fn finish(mut self) -> Self::Out {
            if let Err(err) = self.flush_accumulated_hunk() {
                self.err = Some(err);
            }
            if let Some(err) = self.err {
                return Err(err);
            }
            Ok(self.delegate.finish())
        }
    }

    /// An implementation that fails if the input isn't UTF-8.
    impl ConsumeHunk for String {
        type Out = Self;

        fn consume_hunk(
            &mut self,
            header: HunkHeader,
            lines: &[(DiffLineType, &[u8])],
            newline: NewlineSeparator<'_>,
        ) -> std::io::Result<()> {
            self.push_str(&header.to_string());
            self.push_str(match newline {
                NewlineSeparator::AfterHeaderAndLine(nl) | NewlineSeparator::AfterHeaderAndWhenNeeded(nl) => nl,
            });

            for &(line_type, content) in lines {
                self.push(line_type.to_prefix());
                self.push_str(std::str::from_utf8(content).map_err(|e| std::io::Error::new(ErrorKind::Other, e))?);

                match newline {
                    NewlineSeparator::AfterHeaderAndLine(nl) => {
                        self.push_str(nl);
                    }
                    NewlineSeparator::AfterHeaderAndWhenNeeded(nl) => {
                        if !content.ends_with_str(nl) {
                            self.push_str(nl);
                        }
                    }
                }
            }
            Ok(())
        }

        fn finish(self) -> Self::Out {
            self
        }
    }

    /// An implementation that writes hunks into a byte buffer.
    impl ConsumeHunk for Vec<u8> {
        type Out = Self;

        fn consume_hunk(
            &mut self,
            header: HunkHeader,
            lines: &[(DiffLineType, &[u8])],
            newline: NewlineSeparator<'_>,
        ) -> std::io::Result<()> {
            self.push_str(header.to_string());
            self.push_str(match newline {
                NewlineSeparator::AfterHeaderAndLine(nl) | NewlineSeparator::AfterHeaderAndWhenNeeded(nl) => nl,
            });

            for &(line_type, content) in lines {
                self.push(line_type.to_prefix() as u8);
                self.extend_from_slice(content);

                match newline {
                    NewlineSeparator::AfterHeaderAndLine(nl) => {
                        self.push_str(nl);
                    }
                    NewlineSeparator::AfterHeaderAndWhenNeeded(nl) => {
                        if !content.ends_with_str(nl) {
                            self.push_str(nl);
                        }
                    }
                }
            }
            Ok(())
        }

        fn finish(self) -> Self::Out {
            self
        }
    }
}
