use gix_error::{Result, ResultExt, bail, message};
use regex::bytes::{Regex, RegexBuilder};

/// A matcher for source declarations that provide context for a diff hunk.
///
/// The default recognizes lines starting with an ASCII letter, `_`, or `$`, like Git's default
/// hunk-header matcher. It does not infer a programming language from a file extension.
/// A matcher borrows matched bytes directly from the source and does not impose a display width.
#[derive(Clone, Debug, Default)]
pub struct Matcher {
    expressions: Vec<Expression>,
}

#[derive(Clone, Debug)]
struct Expression {
    regex: Regex,
    negative: bool,
}

impl Matcher {
    /// Compile a newline-separated list of Rust byte regular expressions.
    ///
    /// Expressions are tried in order. A matching expression prefixed with `!` rejects the line;
    /// otherwise, capture group 1, or the entire match if that group did not participate, becomes the
    /// header. The last expression must not be negative.
    /// Matching is case sensitive and uses ASCII character classes.
    ///
    /// These expressions use the syntax and leftmost-first semantics of the Rust `regex` crate.
    /// They are not a parser for Git's `diff.<driver>.funcname` or `xfuncname` configuration,
    /// which uses POSIX basic or extended regular expressions, respectively.
    pub fn from_regex(pattern: &str) -> Result<Self> {
        let lines: Vec<_> = pattern.split('\n').collect();
        let mut expressions = Vec::with_capacity(lines.len());
        for (index, line) in lines.iter().enumerate() {
            let (negative, pattern) = line.strip_prefix('!').map_or((false, *line), |pattern| (true, pattern));
            if negative && index + 1 == lines.len() {
                bail!(message("The last hunk-header expression must not be negative"));
            }
            let regex = RegexBuilder::new(pattern)
                .unicode(false)
                .build()
                .or_raise(|| message!("Invalid hunk-header expression on line {}", index + 1))?;
            expressions.push(Expression { regex, negative });
        }
        Ok(Self { expressions })
    }

    /// Return the header matched in `line`, borrowing its bytes, without trailing spaces, tabs, CR, or LF.
    ///
    /// The terminating LF, and a CR immediately preceding it, are excluded from regex matching.
    /// Other whitespace participates in matching, and is removed only from the matched header.
    /// A line without a match yields `None`; a successful empty match yields `Some(b"")`.
    pub fn find<'a>(&self, line: &'a [u8]) -> Option<&'a [u8]> {
        if self.expressions.is_empty() {
            return line
                .first()
                .is_some_and(|byte| byte.is_ascii_alphabetic() || matches!(byte, b'_' | b'$'))
                .then(|| trim_whitespace(line));
        }
        let line = line
            .strip_suffix(b"\n")
            .map_or(line, |line| line.strip_suffix(b"\r").unwrap_or(line));
        for expression in &self.expressions {
            let Some(captures) = expression.regex.captures(line) else {
                continue;
            };
            if expression.negative {
                return None;
            }
            let matched = captures.get(1).or_else(|| captures.get(0))?;
            return Some(trim_whitespace(matched.as_bytes()));
        }
        None
    }

    /// Iterate over the matching lines in `source`, yielding their zero-based line numbers and headers.
    ///
    /// Lines are split at LF bytes. Both terminated and unterminated final lines are supported.
    /// The matched bytes are borrowed from `source` and outlive the matcher if necessary.
    pub fn lines<'matcher, 'source: 'matcher>(
        &'matcher self,
        source: &'source [u8],
    ) -> impl Iterator<Item = (usize, &'source [u8])> + 'matcher {
        source
            .split_inclusive(|byte| *byte == b'\n')
            .enumerate()
            .filter_map(|(number, line)| self.find(line).map(|header| (number, header)))
    }
}

fn trim_whitespace(line: &[u8]) -> &[u8] {
    // Git's locale-independent whitespace class excludes vertical tab and form feed.
    let end = line
        .iter()
        .rposition(|byte| !matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
        .map_or(0, |position| position + 1);
    &line[..end]
}
