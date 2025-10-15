use bstr::{BString, ByteSlice, ByteVec};
use imara_diff::{intern, Sink};
use intern::{InternedInput, Interner, Token};
use std::fmt::Write;
use std::{hash::Hash, ops::Range};

use super::{ConsumeBinaryHunk, ConsumeBinaryHunkDelegate, ConsumeHunk, ContextSize, DiffLineKind, HunkHeader};

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

    buffer: Vec<(DiffLineKind, &'a [u8])>,

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
    pub fn new(input: &'a InternedInput<T>, consume_hunk: D, context_size: ContextSize) -> Self {
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

            buffer: Vec::with_capacity(8),
            delegate: consume_hunk,

            err: None,
        }
    }

    fn print_tokens(&mut self, tokens: &[Token], line_type: DiffLineKind) {
        for &token in tokens {
            let content = self.interner[token].as_ref();
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

        self.delegate.consume_hunk(header, &self.buffer)?;

        self.reset_hunks();
        Ok(())
    }

    fn print_context_and_update_pos(&mut self, print: Range<u32>, move_to: u32) {
        self.print_tokens(
            &self.before[print.start as usize..print.end as usize],
            DiffLineKind::Context,
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
            DiffLineKind::Remove,
        );
        self.print_tokens(&self.after[after.start as usize..after.end as usize], DiffLineKind::Add);
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
impl<D> ConsumeHunk for ConsumeBinaryHunk<'_, D>
where
    D: ConsumeBinaryHunkDelegate,
{
    type Out = D;

    fn consume_hunk(&mut self, header: HunkHeader, lines: &[(DiffLineKind, &[u8])]) -> std::io::Result<()> {
        self.header_buf.clear();
        self.header_buf
            .write_fmt(format_args!("{header}{nl}", nl = self.newline))
            .map_err(std::io::Error::other)?;

        let buf = &mut self.hunk_buf;
        buf.clear();
        for &(line_type, content) in lines {
            buf.push(line_type.to_prefix() as u8);
            buf.extend_from_slice(content);

            if !content.ends_with_str(self.newline) {
                buf.push_str(self.newline);
            }
        }

        self.delegate.consume_binary_hunk(header, &self.header_buf, buf)?;
        Ok(())
    }

    fn finish(self) -> Self::Out {
        self.delegate
    }
}

/// An implementation that fails if the input isn't UTF-8.
impl ConsumeBinaryHunkDelegate for String {
    fn consume_binary_hunk(&mut self, _header: HunkHeader, header_str: &str, hunk: &[u8]) -> std::io::Result<()> {
        self.push_str(header_str);
        self.push_str(hunk.to_str().map_err(std::io::Error::other)?);
        Ok(())
    }
}

/// An implementation that writes hunks into a byte buffer.
impl ConsumeBinaryHunkDelegate for Vec<u8> {
    fn consume_binary_hunk(&mut self, _header: HunkHeader, header_str: &str, hunk: &[u8]) -> std::io::Result<()> {
        self.push_str(header_str);
        self.extend_from_slice(hunk);
        Ok(())
    }
}

/// An implementation that writes hunks into a hunman-readable byte buffer.
impl ConsumeBinaryHunkDelegate for BString {
    fn consume_binary_hunk(&mut self, _header: HunkHeader, header_str: &str, hunk: &[u8]) -> std::io::Result<()> {
        self.push_str(header_str);
        self.extend_from_slice(hunk);
        Ok(())
    }
}

impl<'a, D> ConsumeBinaryHunk<'a, D>
where
    D: ConsumeBinaryHunkDelegate,
{
    /// Create a new instance that writes stringified hunks to `delegate`, which uses `newline` to separate header and hunk,
    /// as well as hunk lines that don't naturally end in a newline.
    pub fn new(delegate: D, newline: &'a str) -> ConsumeBinaryHunk<'a, D> {
        ConsumeBinaryHunk {
            newline,
            delegate,
            header_buf: String::new(),
            hunk_buf: Vec::with_capacity(128),
        }
    }
}

impl DiffLineKind {
    /// Returns a one-character representation for use in unified diffs.
    pub const fn to_prefix(self) -> char {
        match self {
            DiffLineKind::Context => ' ',
            DiffLineKind::Add => '+',
            DiffLineKind::Remove => '-',
        }
    }
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
