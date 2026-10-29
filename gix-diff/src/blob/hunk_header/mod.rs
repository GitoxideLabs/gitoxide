mod builtin;

use bstr::BString;
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
    regex: Engine,
    negative: bool,
}

#[derive(Clone, Debug)]
enum Engine {
    Rust(Regex),
    Posix(gix_regex::PosixRegex<'static>),
}

/// The POSIX syntax used by a Git hunk-header configuration key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Syntax {
    /// Basic syntax, used by the funcname configuration key.
    #[default]
    Basic,
    /// Extended syntax, used by the xfuncname configuration key.
    Extended,
}

/// Options for compiling POSIX hunk-header patterns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Options {
    /// Whether patterns use basic or extended POSIX syntax.
    pub syntax: Syntax,
    /// Fold ASCII case, as some built-in language drivers do.
    pub case_insensitive: bool,
}

/// An uncompiled hunk-header definition suitable for storing in a diff driver.
///
/// Keeping definitions uncompiled allows invalid configuration to be diagnosed only when
/// its driver is selected for a path, as Git does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Definition {
    /// Newline-separated expressions, with optional leading exclusion markers.
    pub pattern: BString,
    /// Pattern syntax and case folding.
    pub options: Options,
}

impl Definition {
    /// Compile this definition into a reusable matcher.
    pub fn to_matcher(&self) -> Result<Matcher> {
        Matcher::from_posix(&self.pattern, self.options)
    }
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
            expressions.push(Expression {
                regex: Engine::Rust(regex),
                negative,
            });
        }
        Ok(Self { expressions })
    }

    /// Compile Git's newline-separated POSIX hunk-header expressions.
    ///
    /// A leading exclamation mark excludes matching lines. Expressions retain their order,
    /// and the final expression must be positive. Matches use POSIX leftmost-longest
    /// semantics, support backreferences, and operate on arbitrary bytes with ASCII classes.
    /// Parsing and matching enforce the resource limits of the underlying regex engine.
    pub fn from_posix(pattern: &[u8], options: Options) -> Result<Self> {
        let lines: Vec<_> = pattern.split(|byte| *byte == b'\n').collect();
        let mut expressions = Vec::with_capacity(lines.len());
        for (index, line) in lines.iter().enumerate() {
            let (negative, pattern) = line
                .strip_prefix(b"!")
                .map_or((false, *line), |pattern| (true, pattern));
            if negative && index + 1 == lines.len() {
                bail!(message("The last hunk-header expression must not be negative"));
            }
            let regex = gix_regex::PosixRegexBuilder::new(pattern)
                .with_default_classes()
                .extended(options.syntax == Syntax::Extended)
                .compile()
                .or_raise(|| message!("Invalid POSIX hunk-header expression on line {}", index + 1))?
                .case_insensitive(options.case_insensitive);
            expressions.push(Expression {
                regex: Engine::Posix(regex),
                negative,
            });
        }
        Ok(Self { expressions })
    }

    /// Return the header matched in `line`, borrowing its bytes, without trailing spaces, tabs, CR, or LF.
    ///
    /// The terminating LF, and a CR immediately preceding it, are excluded from regex matching.
    /// Other whitespace participates in matching, and is removed only from the matched header.
    /// Exhausting a POSIX matching resource limit returns an error.
    /// A line without a match yields `None`; a successful empty match yields `Some(b"")`.
    pub fn find<'a>(&self, line: &'a [u8]) -> Result<Option<&'a [u8]>> {
        if self.expressions.is_empty() {
            return Ok(line
                .first()
                .is_some_and(|byte| byte.is_ascii_alphabetic() || matches!(byte, b'_' | b'$'))
                .then(|| trim_whitespace(line)));
        }
        let line = line
            .strip_suffix(b"\n")
            .map_or(line, |line| line.strip_suffix(b"\r").unwrap_or(line));
        for expression in &self.expressions {
            let matched = match &expression.regex {
                Engine::Rust(regex) => regex.captures(line).and_then(|captures| {
                    captures
                        .get(1)
                        .or_else(|| captures.get(0))
                        .map(|matched| matched.as_bytes())
                }),
                Engine::Posix(regex) => {
                    let captures = regex
                        .matches(line, Some(1))
                        .or_raise(|| message("Could not match a POSIX hunk-header expression"))?;
                    captures
                        .first()
                        .and_then(|groups| groups.get(1).copied().flatten().or(groups[0]))
                        .map(|(start, end)| &line[start..end])
                }
            };
            let Some(matched) = matched else {
                continue;
            };
            return Ok((!expression.negative).then(|| trim_whitespace(matched)));
        }
        Ok(None)
    }

    /// Iterate over the matching lines in `source`, yielding their zero-based line numbers and headers.
    ///
    /// Lines are split at LF bytes. Both terminated and unterminated final lines are supported.
    /// The matched bytes are borrowed from `source` and outlive the matcher if necessary.
    pub fn lines<'matcher, 'source: 'matcher>(
        &'matcher self,
        source: &'source [u8],
    ) -> impl Iterator<Item = Result<(usize, &'source [u8])>> + 'matcher {
        source
            .split_inclusive(|byte| *byte == b'\n')
            .enumerate()
            .filter_map(|(number, line)| {
                self.find(line)
                    .transpose()
                    .map(|result| result.map(|header| (number, header)))
            })
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
