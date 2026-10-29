// Derived from posix-regex 0.1.4, upstream revision
// 063882aa6051b5075062d20d00b41ddc0fb8cd89.
// Copyright (c) 2018 jD91mZM2. Licensed under the MIT license; see LICENSE-MIT.

//! The regex "compiler", which parses the regex itself.
//! Produces a matcher ready to match input.

// Keep unchanged upstream formatting and option handling while compatibility fixes are developed.
#![allow(
    clippy::uninlined_format_args,
    clippy::cloned_instead_of_copied,
    clippy::map_unwrap_or
)]

#[cfg(feature = "no_std")]
use alloc::collections::BTreeMap as HashMap;

#[cfg(not(feature = "no_std"))]
use std::collections::HashMap;

use alloc::{borrow::Cow, vec, vec::Vec};
use core::fmt;

use crate::{PosixRegex, ctype, tree::*};

/// Repetition bounds, for example + is (1, None), and ? is (0, Some(1))
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Range(pub u32, pub Option<u32>);
impl fmt::Debug for Range {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Range(start, None) => write!(f, "{}..", start),
            Range(start, Some(end)) => write!(f, "{}..{}", start, end),
        }
    }
}

/// An item inside square brackets, like `[abc]` or `[[:digit:]]`
#[derive(Clone)]
pub enum Collation {
    Char(u8),
    Class(fn(u8) -> bool),
}
impl PartialEq for Collation {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Char(left), Self::Char(right)) => left == right,
            (Self::Class(left), Self::Class(right)) => core::ptr::fn_addr_eq(*left, *right),
            _ => false,
        }
    }
}
impl Eq for Collation {}

impl Collation {
    /// Compare this collation to a character
    pub fn matches(&self, other: u8, insensitive: bool) -> bool {
        match *self {
            Collation::Char(me) if insensitive => {
                if ctype::is_alpha(me) && ctype::is_alpha(other) {
                    me | 32 == other | 32
                } else {
                    me == other
                }
            }
            Collation::Char(me) => me == other,
            Collation::Class(f) if insensitive => f(other.to_ascii_lowercase()) || f(other.to_ascii_uppercase()),
            Collation::Class(f) => f(other),
        }
    }
}
impl fmt::Debug for Collation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Collation::Char(c) => write!(f, "{:?}", c as char),
            Collation::Class(c) => write!(f, "{:p}", c),
        }
    }
}

/// A single "compiled" token, such as a `.` or a character literal
#[derive(Clone, PartialEq, Eq)]
pub enum Token {
    /// Internal token used to find matches that might be anywhere in the text
    InternalStart,

    Alternative,
    Any,
    BackRef(u32),
    Char(u8),
    End,
    Group(usize),
    OneOf {
        invert: bool,
        list: Vec<Collation>,
    },
    Root,
    /// A non-capturing wrapper for another repetition.
    Repetition,
    Start,
    WordEnd,
    WordStart,
    WordBoundary,
    NotWordBoundary,
    AbsoluteStart,
    AbsoluteEnd,
}
impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Token::InternalStart => write!(f, "<START>"),

            Token::Alternative => write!(f, "Alternative"),
            Token::Any => write!(f, "."),
            Token::BackRef(id) => write!(f, "\\{}", id),
            Token::Char(c) => write!(f, "{:?}", c as char),
            Token::End => write!(f, "$"),
            Token::Group(id) => write!(f, "Group({})", id),
            Token::OneOf { invert, ref list } => write!(f, "{{invert: {}, {:?}}}", invert, list),
            Token::Root => write!(f, "Root"),
            Token::Repetition => write!(f, "Repetition"),
            Token::Start => write!(f, "^"),
            Token::WordEnd => write!(f, ">"),
            Token::WordStart => write!(f, "<"),
            Token::WordBoundary => write!(f, "\\b"),
            Token::NotWordBoundary => write!(f, "\\B"),
            Token::AbsoluteStart => write!(f, "\\`"),
            Token::AbsoluteEnd => write!(f, "\\'"),
        }
    }
}
/// An error that occurred while compiling the regex
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    EOF,
    EmptyRepetition,
    Expected(u8, Option<u8>),
    IllegalRange,
    IntegerOverflow,
    InvalidBackRef(u32),
    LeadingRepetition,
    UnclosedRepetition,
    UnexpectedToken(u8),
    UnknownClass(Vec<u8>),
    UnknownCollation,
    /// The pattern exceeds the configured byte limit.
    PatternTooLarge {
        limit: usize,
    },
    /// Group nesting exceeds the configured depth limit.
    NestingTooDeep {
        limit: usize,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EOF => f.write_str("Unexpected end of regular expression"),
            Self::EmptyRepetition => f.write_str("A repetition requires a lower bound"),
            Self::Expected(expected, actual) => write!(
                f,
                "Expected {:?}, found {:?}",
                char::from(*expected),
                actual.map(char::from)
            ),
            Self::IllegalRange => f.write_str("Invalid character or repetition range"),
            Self::IntegerOverflow => f.write_str("Regular expression repetition bound is too large"),
            Self::InvalidBackRef(id) => write!(f, "Backreference {id} does not name a preceding closed group"),
            Self::LeadingRepetition => f.write_str("A repetition must follow an expression"),
            Self::UnclosedRepetition => f.write_str("Unclosed repetition range"),
            Self::UnexpectedToken(token) => write!(f, "Unexpected regular expression token {:?}", char::from(*token)),
            Self::UnknownClass(name) => write!(f, "Unknown regular expression character class {name:?}"),
            Self::UnknownCollation => f.write_str("Unknown regular expression collation"),
            Self::PatternTooLarge { limit } => write!(f, "Regular expression exceeds its {limit}-byte pattern limit"),
            Self::NestingTooDeep { limit } => write!(f, "Regular expression exceeds its {limit}-level nesting limit"),
        }
    }
}
impl core::error::Error for Error {}

/// A regex builder struct
pub struct PosixRegexBuilder<'a> {
    input: &'a [u8],
    classes: HashMap<&'a [u8], fn(u8) -> bool>,
    group_id: usize,
    closed_groups: Vec<bool>,
    builder: TreeBuilder,
    extended: bool,
    size_limit: usize,
    nesting_limit: usize,
    nesting: usize,
}
impl<'a> PosixRegexBuilder<'a> {
    /// Create a new instance that is ready to parse the regex `input`
    pub fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            classes: HashMap::new(),
            group_id: 1,
            closed_groups: vec![false],
            builder: TreeBuilder::default(),
            extended: false,
            size_limit: 64 * 1024,
            nesting_limit: 64,
            nesting: 0,
        }
    }
    /// Add a custom collation class, for use within square brackets (such as `[[:digit:]]`)
    pub fn with_class(mut self, name: &'a [u8], callback: fn(u8) -> bool) -> Self {
        self.classes.insert(name, callback);
        self
    }
    /// Add all the default collation classes, like `[[:digit:]]` and `[[:alnum:]]`
    pub fn with_default_classes(mut self) -> Self {
        #[cfg(not(feature = "no_std"))]
        self.classes.reserve(12);

        self.classes.insert(b"alnum", ctype::is_alnum);
        self.classes.insert(b"alpha", ctype::is_alpha);
        self.classes.insert(b"blank", ctype::is_blank);
        self.classes.insert(b"cntrl", ctype::is_cntrl);
        self.classes.insert(b"digit", ctype::is_digit);
        self.classes.insert(b"graph", ctype::is_graph);
        self.classes.insert(b"lower", ctype::is_lower);
        self.classes.insert(b"print", ctype::is_print);
        self.classes.insert(b"punct", ctype::is_punct);
        self.classes.insert(b"space", ctype::is_space);
        self.classes.insert(b"upper", ctype::is_upper);
        self.classes.insert(b"xdigit", ctype::is_xdigit);

        self
    }
    /// Use POSIX extended regex
    pub fn extended(mut self, extended: bool) -> Self {
        self.extended = extended;
        self
    }
    /// Limit pattern length in bytes. The default is 64 KiB.
    pub fn size_limit(mut self, bytes: usize) -> Self {
        self.size_limit = bytes;
        self
    }
    /// Limit group nesting depth. The default is 64; values above 128 are capped
    /// to protect the parser and matcher stacks.
    pub fn nesting_limit(mut self, depth: usize) -> Self {
        self.nesting_limit = depth.min(128);
        self
    }
    /// "Compile" this regex to a struct ready to match input
    pub fn compile(self) -> Result<PosixRegex<'static>, Error> {
        let tree = self.compile_tokens()?;
        Ok(PosixRegex::new(Cow::Owned(tree)))
    }
    pub fn compile_tokens(mut self) -> Result<Tree, Error> {
        if self.input.len() > self.size_limit {
            return Err(Error::PatternTooLarge { limit: self.size_limit });
        }
        self.builder.start_internal(Token::Root, Range(1, Some(1)));
        self.parse(false)?;
        self.builder.finish_internal();
        let tree = self.builder.finish();
        // Repetition wrappers can add depth outside a deeply nested parsed group.
        // Check the final tree iteratively so their combined depth stays bounded.
        let mut pending = vec![(tree.root, 0)];
        while let Some((id, mut depth)) = pending.pop() {
            if matches!(tree[id].token, Token::Group(_) | Token::Repetition) {
                depth += 1;
                if depth > self.nesting_limit {
                    return Err(Error::NestingTooDeep {
                        limit: self.nesting_limit,
                    });
                }
            }
            pending.extend(tree[id].children(&tree).map(|child| (child, depth)));
        }
        Ok(tree)
    }

    fn consume(&mut self, amount: usize) {
        self.input = &self.input[amount..];
    }
    fn take_int(&mut self) -> Result<Option<u32>, Error> {
        let mut out: Option<u32> = None;
        while let Some(&c @ b'0'..=b'9') = self.input.first() {
            self.consume(1);
            out = Some(
                out.unwrap_or(0)
                    .checked_mul(10)
                    .and_then(|out| out.checked_add((c - b'0') as u32))
                    .ok_or(Error::IntegerOverflow)?,
            );
        }
        Ok(out)
    }
    fn next(&mut self) -> Result<u8, Error> {
        self.input
            .first()
            .map(|&c| {
                self.consume(1);
                c
            })
            .ok_or(Error::EOF)
    }
    fn expect(&mut self, c: u8) -> Result<(), Error> {
        if self.input.first() != Some(&c) {
            return Err(Error::Expected(c, self.input.first().cloned()));
        }
        self.consume(1);
        Ok(())
    }
    fn parse_range(&mut self) -> Result<Range, Error> {
        let (consumed, range) = match self.input {
            [b'*', ..] => (1, Range(0, None)),
            [b'?', ..] if self.extended => (1, Range(0, Some(1))),
            [b'+', ..] if self.extended => (1, Range(1, None)),
            [b'\\', b'?', ..] if !self.extended => (2, Range(0, Some(1))),
            [b'\\', b'+', ..] if !self.extended => (2, Range(1, None)),
            [b'{', ..] if self.extended => {
                self.consume(1);
                return self.parse_interval();
            }
            [b'\\', b'{', ..] if !self.extended => {
                self.consume(2);
                return self.parse_interval();
            }
            _ => return Ok(Range(1, Some(1))),
        };
        self.consume(consumed);
        Ok(range)
    }

    fn has_repetition(&self) -> bool {
        matches!(self.input.first(), Some(b'*'))
            || if self.extended {
                matches!(self.input.first(), Some(b'+' | b'?' | b'{'))
            } else {
                matches!(self.input, [b'\\', b'+' | b'?' | b'{', ..])
            }
    }

    fn wrap_repetitions(&mut self, checkpoint: Checkpoint) -> Result<(), Error> {
        let mut wrappers = 0;
        while self.has_repetition() {
            if !self.extended && (self.input.starts_with(b"*") || self.input.starts_with(br"\{")) {
                return Err(Error::LeadingRepetition);
            }
            wrappers += 1;
            if self.nesting + wrappers > self.nesting_limit {
                return Err(Error::NestingTooDeep {
                    limit: self.nesting_limit,
                });
            }
            let range = self.parse_range()?;
            self.builder.start_internal_at(checkpoint, Token::Repetition, range);
            self.builder.finish_internal();
        }
        Ok(())
    }

    fn parse_interval(&mut self) -> Result<Range, Error> {
        let first = self.take_int()?;
        let (min, max) = if self.input.first() == Some(&b',') {
            self.consume(1);
            (first.unwrap_or(0), self.take_int()?)
        } else {
            let first = first.ok_or(Error::EmptyRepetition)?;
            (first, Some(first))
        };
        let closing = if self.extended { b"}".as_slice() } else { br"\}" };
        if !self.input.starts_with(closing) {
            return Err(Error::UnclosedRepetition);
        }
        self.consume(closing.len());
        if max.is_some_and(|max| min > max) {
            return Err(Error::IllegalRange);
        }
        // GNU regex and Git support the same finite interval limit in BRE and ERE.
        if min > 32767 || max.is_some_and(|max| max > 32767) {
            return Err(Error::IntegerOverflow);
        }
        Ok(Range(min, max))
    }
    fn bracket_item(&mut self) -> Result<Collation, Error> {
        let first = self.next()?;
        if first != b'[' {
            return Ok(Collation::Char(first));
        }
        match self.input.first() {
            Some(b'.' | b'=') => {
                let delimiter = self.next()?;
                let value = self.next()?;
                self.expect(delimiter)?;
                self.expect(b']')?;
                Ok(Collation::Char(value))
            }
            Some(b':') => {
                self.consume(1);
                let end = self.input.iter().position(|&byte| byte == b':').ok_or(Error::EOF)?;
                let name = &self.input[..end];
                let class = *self
                    .classes
                    .get(name)
                    .ok_or_else(|| Error::UnknownClass(name.to_vec()))?;
                self.consume(end + 1);
                self.expect(b']')?;
                Ok(Collation::Class(class))
            }
            _ => Ok(Collation::Char(b'[')),
        }
    }

    fn parse_bracket(&mut self) -> Result<Token, Error> {
        let invert = self.input.first() == Some(&b'^');
        if invert {
            self.consume(1);
        }
        let mut list = Vec::new();
        loop {
            // A closing bracket is literal in the first position of the set.
            let item = self.bracket_item()?;
            if self.input.first() == Some(&b'-') && self.input.get(1) != Some(&b']') {
                self.consume(1);
                let end = self.bracket_item()?;
                let (Collation::Char(start), Collation::Char(end)) = (item, end) else {
                    return Err(Error::IllegalRange);
                };
                if start > end {
                    return Err(Error::IllegalRange);
                }
                list.extend((start..=end).map(Collation::Char));
                if self.input.first() == Some(&b'-') && self.input.get(1) != Some(&b']') {
                    return Err(Error::IllegalRange);
                }
            } else {
                list.push(item);
            }
            if self.input.first() == Some(&b']') {
                self.consume(1);
                return Ok(Token::OneOf { invert, list });
            }
        }
    }

    fn parse(&mut self, in_group: bool) -> Result<(), Error> {
        if in_group {
            self.nesting += 1;
            if self.nesting > self.nesting_limit {
                return Err(Error::NestingTooDeep {
                    limit: self.nesting_limit,
                });
            }
        }
        let mut closed = !in_group;
        let mut at_start = true;
        self.builder.start_internal(Token::Alternative, Range(1, Some(1)));
        while let Ok(c) = self.next() {
            let token = match c {
                b'*' | b'+' | b'?' | b'{' if self.extended => return Err(Error::LeadingRepetition),
                b'^' if self.extended || at_start => Token::Start,
                b'$' if self.extended
                    || self.input.is_empty()
                    || self.input.starts_with(br"\|")
                    || (in_group && self.input.starts_with(br"\)")) =>
                {
                    Token::End
                }
                b'.' => Token::Any,
                b'[' => self.parse_bracket()?,
                b'\\' if self.input.first().is_some_and(|c| matches!(c, b'1'..=b'9')) => {
                    let id = u32::from(self.next()? - b'0');
                    if !self.closed_groups.get(id as usize).copied().unwrap_or(false) {
                        return Err(Error::InvalidBackRef(id));
                    }
                    Token::BackRef(id)
                }
                b'\\' => match self.next()? {
                    b'(' if !self.extended => {
                        let id = self.group_id;
                        self.group_id += 1;
                        self.closed_groups.push(false);
                        let checkpoint = self.builder.checkpoint();
                        self.parse(true)?;
                        self.closed_groups[id] = true;
                        let range = self.parse_range()?;
                        self.builder.start_internal_at(checkpoint, Token::Group(id), range);
                        self.builder.finish_internal();
                        self.wrap_repetitions(checkpoint)?;
                        at_start = false;
                        continue;
                    }
                    b')' if !self.extended => {
                        if !in_group {
                            return Err(Error::UnexpectedToken(b')'));
                        }
                        closed = true;
                        break;
                    }
                    b'{' if !self.extended => return Err(Error::LeadingRepetition),
                    b'|' if !self.extended => {
                        self.builder.finish_internal();
                        self.builder.start_internal(Token::Alternative, Range(1, Some(1)));
                        at_start = true;
                        continue;
                    }
                    b'<' => Token::WordStart,
                    b'>' => Token::WordEnd,
                    b'b' => Token::WordBoundary,
                    b'B' => Token::NotWordBoundary,
                    b'`' => Token::AbsoluteStart,
                    b'\'' => Token::AbsoluteEnd,
                    c @ (b'w' | b'W') => Token::OneOf {
                        invert: c == b'W',
                        list: vec![Collation::Class(ctype::is_word)],
                    },
                    b's' => Token::OneOf {
                        invert: false,
                        list: vec![Collation::Class(ctype::is_space)],
                    },
                    b'S' => Token::OneOf {
                        invert: true,
                        list: vec![Collation::Class(ctype::is_space)],
                    },
                    c => Token::Char(c),
                },
                b'(' if self.extended => {
                    let id = self.group_id;
                    self.group_id += 1;
                    self.closed_groups.push(false);
                    let checkpoint = self.builder.checkpoint();
                    self.parse(true)?;
                    self.closed_groups[id] = true;
                    let range = self.parse_range()?;
                    self.builder.start_internal_at(checkpoint, Token::Group(id), range);
                    self.builder.finish_internal();
                    self.wrap_repetitions(checkpoint)?;
                    at_start = false;
                    continue;
                }
                b')' if self.extended && in_group => {
                    closed = true;
                    break;
                }
                b'|' if self.extended => {
                    self.builder.finish_internal();
                    self.builder.start_internal(Token::Alternative, Range(1, Some(1)));
                    at_start = true;
                    continue;
                }
                c => Token::Char(c),
            };
            let assertion = matches!(
                token,
                Token::Start
                    | Token::End
                    | Token::WordStart
                    | Token::WordEnd
                    | Token::WordBoundary
                    | Token::NotWordBoundary
                    | Token::AbsoluteStart
                    | Token::AbsoluteEnd
            );
            let checkpoint = self.builder.checkpoint();
            let range = if assertion {
                if self.extended && self.has_repetition() {
                    return Err(Error::LeadingRepetition);
                }
                Range(1, Some(1))
            } else {
                self.parse_range()?
            };
            self.builder.leaf(token, range);
            if !assertion {
                self.wrap_repetitions(checkpoint)?;
            }
            at_start = false;
        }
        if !closed {
            return Err(Error::Expected(b')', None));
        }
        self.builder.finish_internal();
        if in_group {
            self.nesting -= 1;
        }
        Ok(())
    }
}
