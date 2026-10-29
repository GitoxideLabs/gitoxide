// Derived from posix-regex 0.1.4, upstream revision
// 063882aa6051b5075062d20d00b41ddc0fb8cd89.
// Copyright (c) 2018 jD91mZM2. Licensed under the MIT license; see LICENSE-MIT.

//! Bounded, byte-oriented matching with captures and backreferences.

use alloc::{borrow::Cow, boxed::Box, rc::Rc, vec, vec::Vec};
use core::{cell::Cell, fmt, mem};

use crate::{
    compile::Token,
    ctype,
    tree::{NodeId, Tree},
};

/// Captured byte ranges, with the whole match at index zero.
/// A group that did not participate has no range.
pub type Captures = Box<[Option<(usize, usize)>]>;

/// Work and memory limits shared by all matches in one search.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Maximum number of matching steps, including byte comparisons.
    pub max_steps: usize,
    /// Maximum bytes charged to live capture states and retained results.
    ///
    /// Charges include each state's fixed storage and captures. Allocator bookkeeping,
    /// action/vector bookkeeping and spare vector capacity are excluded, so this is not
    /// an exact heap allocation bound.
    pub max_memory: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_steps: 1_000_000,
            max_memory: 16 * 1024 * 1024,
        }
    }
}

/// A matching resource limit was reached. No partial search result is returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// More matching work would exceed the configured step count.
    StepLimit,
    /// Another capture state would exceed the configured memory charge.
    MemoryLimit,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::StepLimit => "Regular expression matching step limit exceeded",
            Self::MemoryLimit => "Regular expression matching memory limit exceeded",
        })
    }
}
impl core::error::Error for Error {}

/// A compiled POSIX byte regular expression.
///
/// Matching selects the earliest starting position and the longest complete match there.
/// When complete matches have equal lengths, alternatives retain their pattern order and
/// repetitions are greedy. Matching is fallible and always subject to resource limits.
#[derive(Clone, Debug)]
pub struct PosixRegex<'a> {
    tree: Cow<'a, Tree>,
    case_insensitive: bool,
    newline: bool,
    no_start: bool,
    no_end: bool,
    limits: Limits,
    groups: usize,
}

impl<'a> PosixRegex<'a> {
    /// Only the parser can construct a matcher, preserving validated tree and depth invariants.
    pub(crate) fn new(tree: Cow<'a, Tree>) -> Self {
        let groups = 1 + tree
            .arena
            .iter()
            .filter(|node| matches!(node.token, Token::Group(_)))
            .count();
        Self {
            tree,
            case_insensitive: false,
            newline: false,
            no_start: false,
            no_end: false,
            limits: Limits::default(),
            groups,
        }
    }

    /// Set limits applied afresh to each call to a matching method.
    pub fn limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    /// Enable ASCII case folding for literals, classes and backreferences.
    pub fn case_insensitive(mut self, value: bool) -> Self {
        self.case_insensitive = value;
        self
    }

    /// Let line anchors match at LF boundaries, and exclude LF from dot and inverted classes.
    pub fn newline(mut self, value: bool) -> Self {
        self.newline = value;
        self
    }

    /// Prevent a line-start anchor from matching the start of the input.
    pub fn no_start(mut self, value: bool) -> Self {
        self.no_start = value;
        self
    }

    /// Prevent a line-end anchor from matching the end of the input.
    pub fn no_end(mut self, value: bool) -> Self {
        self.no_end = value;
        self
    }

    /// The number of capture slots, including the whole-match slot at index zero.
    pub fn count_groups(&self) -> usize {
        self.groups
    }

    /// Match a prefix of input, returning the longest match starting at byte zero.
    ///
    /// Despite the historical name, the match need not consume the entire input.
    pub fn matches_exact(&self, input: &[u8]) -> Result<Option<Captures>, Error> {
        let mut search = Search::new(self, input);
        Ok(search.at(0)?.map(State::into_captures))
    }

    /// Find non-overlapping matches, optionally limited to a maximum count.
    ///
    /// An empty match advances by one byte, including one final empty match at end of input.
    /// All matching work and retained captures share this call's resource budget.
    pub fn matches(&self, input: &[u8], max: Option<usize>) -> Result<Vec<Captures>, Error> {
        let mut search = Search::new(self, input);
        let mut matches = Vec::new();
        let mut offset = 0;
        while max.is_none_or(|max| matches.len() < max) && offset <= input.len() {
            let Some(state) = search.at(offset)? else {
                offset += 1;
                continue;
            };
            let end = state.offset;
            matches.push(state.into_captures());
            if end == offset {
                if offset == input.len() {
                    break;
                }
                offset += 1;
            } else {
                offset = end;
            }
        }
        Ok(matches)
    }
}

struct Budget {
    steps: usize,
    memory: Rc<Cell<usize>>,
    max_memory: usize,
}

impl Budget {
    fn step(&mut self, amount: usize) -> Result<(), Error> {
        self.steps = self.steps.checked_sub(amount).ok_or(Error::StepLimit)?;
        Ok(())
    }

    fn claim(&mut self, groups: usize) -> Result<usize, Error> {
        // Initializing or copying captures performs work proportional to the group count.
        self.step(groups.max(1))?;
        let bytes = groups
            .checked_mul(mem::size_of::<Option<(usize, usize)>>())
            .and_then(|bytes| bytes.checked_add(mem::size_of::<State>()))
            .ok_or(Error::MemoryLimit)?;
        let memory = self.memory.get().checked_add(bytes).ok_or(Error::MemoryLimit)?;
        if memory > self.max_memory {
            return Err(Error::MemoryLimit);
        }
        self.memory.set(memory);
        Ok(bytes)
    }

    fn state(&mut self, offset: usize, captures: &[Option<(usize, usize)>]) -> Result<State, Error> {
        let bytes = self.claim(captures.len())?;
        Ok(State {
            offset,
            captures: captures.to_vec(),
            memory: Rc::clone(&self.memory),
            bytes,
        })
    }
}

struct State {
    offset: usize,
    captures: Vec<Option<(usize, usize)>>,
    memory: Rc<Cell<usize>>,
    bytes: usize,
}

impl State {
    fn copy(&self, budget: &mut Budget) -> Result<Self, Error> {
        budget.state(self.offset, &self.captures)
    }

    fn into_captures(mut self) -> Captures {
        // Retain the charge for outputs until the entire search finishes.
        self.bytes = 0;
        mem::take(&mut self.captures).into_boxed_slice()
    }
}

impl Drop for State {
    fn drop(&mut self) {
        self.memory.set(self.memory.get() - self.bytes);
    }
}

struct Search<'a, 'pattern> {
    regex: &'a PosixRegex<'pattern>,
    input: &'a [u8],
    budget: Budget,
}

impl<'a, 'pattern> Search<'a, 'pattern> {
    fn new(regex: &'a PosixRegex<'pattern>, input: &'a [u8]) -> Self {
        Self {
            regex,
            input,
            budget: Budget {
                steps: regex.limits.max_steps,
                memory: Rc::new(Cell::new(0)),
                max_memory: regex.limits.max_memory,
            },
        }
    }

    fn at(&mut self, offset: usize) -> Result<Option<State>, Error> {
        let groups = self.regex.count_groups();
        let bytes = self.budget.claim(groups)?;
        let initial = State {
            offset,
            captures: vec![None; groups],
            memory: Rc::clone(&self.budget.memory),
            bytes,
        };
        let mut best: Option<State> = None;
        for mut state in self.alternatives(self.regex.tree.root, &initial)? {
            if best.as_ref().is_none_or(|best| state.offset > best.offset) {
                state.captures[0] = Some((offset, state.offset));
                best = Some(state);
            }
        }
        Ok(best)
    }

    fn alternatives(&mut self, node: NodeId, state: &State) -> Result<Vec<State>, Error> {
        let mut result = Vec::new();
        let mut alternative = self.regex.tree[node].child;
        while let Some(id) = alternative {
            let mut states = vec![state.copy(&mut self.budget)?];
            let mut cursor = self.regex.tree[id].child;
            while let Some(node) = cursor {
                let mut next = Vec::new();
                for state in states {
                    next.extend(self.repeated(node, state)?);
                }
                states = next;
                if states.is_empty() {
                    break;
                }
                cursor = self.regex.tree[node].next_sibling;
            }
            result.extend(states);
            alternative = self.regex.tree[id].next_sibling;
        }
        Ok(result)
    }

    fn repeated(&mut self, node: NodeId, state: State) -> Result<Vec<State>, Error> {
        enum Action {
            Repeat(State, usize),
            Accept(State),
        }
        let range = self.regex.tree[node].range;
        let mut pending = vec![Action::Repeat(state, 0)];
        let mut result = Vec::new();
        while let Some(action) = pending.pop() {
            self.budget.step(1)?;
            let (state, count) = match action {
                Action::Accept(state) => {
                    result.push(state);
                    continue;
                }
                Action::Repeat(state, count) => (state, count),
            };
            if count >= range.0 as usize {
                pending.push(Action::Accept(state.copy(&mut self.budget)?));
            }
            if range.1.is_some_and(|max| count >= max as usize) {
                continue;
            }
            let offset = state.offset;
            // Reverse the stack insertion order so the first alternative is visited first.
            for next in self.atom(node, state)?.into_iter().rev() {
                if next.offset == offset && count >= range.0 as usize {
                    // One empty iteration records participating captures. Further optional empty
                    // iterations must not overwrite the capture of a preceding non-empty match.
                    if count == 0 {
                        pending.push(Action::Accept(next));
                    }
                } else {
                    pending.push(Action::Repeat(next, count + 1));
                }
            }
        }
        Ok(result)
    }

    fn atom(&mut self, node: NodeId, mut state: State) -> Result<Vec<State>, Error> {
        self.budget.step(1)?;
        let token = &self.regex.tree[node].token;
        if let Token::Group(id) = *token {
            let start = state.offset;
            let mut result = self.alternatives(node, &state)?;
            for state in &mut result {
                state.captures[id] = Some((start, state.offset));
            }
            return Ok(result);
        }
        let previous = state
            .offset
            .checked_sub(1)
            .and_then(|offset| self.input.get(offset))
            .copied();
        let next = self.input.get(state.offset).copied();
        let previous_word = previous.is_some_and(ctype::is_word);
        let next_word = next.is_some_and(ctype::is_word);
        let (accepts, length) = match token {
            Token::Any => (next.is_some_and(|byte| !self.regex.newline || byte != b'\n'), 1),
            Token::Char(expected) => (next.is_some_and(|next| self.equal(*expected, next)), 1),
            Token::OneOf { invert, list } => {
                self.budget.step(list.len())?;
                (
                    next.is_some_and(|next| {
                        (!invert || !self.regex.newline || next != b'\n')
                            && (list.iter().any(|item| item.matches(next, self.regex.case_insensitive)) != *invert)
                    }),
                    1,
                )
            }
            Token::BackRef(id) => {
                let Some((start, end)) = state.captures[*id as usize] else {
                    return Ok(Vec::new());
                };
                let length = end - start;
                self.budget.step(length)?;
                (
                    self.input
                        .get(state.offset..)
                        .and_then(|input| input.get(..length))
                        .is_some_and(|input| {
                            input
                                .iter()
                                .zip(&self.input[start..end])
                                .all(|(&actual, &expected)| self.equal(actual, expected))
                        }),
                    length,
                )
            }
            Token::Start => (
                (!self.regex.no_start && state.offset == 0) || (self.regex.newline && previous == Some(b'\n')),
                0,
            ),
            Token::End => (
                (!self.regex.no_end && next.is_none()) || (self.regex.newline && next == Some(b'\n')),
                0,
            ),
            Token::WordStart => (!previous_word && next_word, 0),
            Token::WordEnd => (previous_word && !next_word, 0),
            Token::WordBoundary => (previous_word != next_word, 0),
            Token::NotWordBoundary => (previous_word == next_word, 0),
            Token::AbsoluteStart => (state.offset == 0, 0),
            Token::AbsoluteEnd => (next.is_none(), 0),
            Token::Root | Token::Alternative | Token::Group(_) | Token::InternalStart => {
                unreachable!("only parsed expression atoms are matched")
            }
        };
        if accepts {
            state.offset += length;
            Ok(vec![state])
        } else {
            Ok(Vec::new())
        }
    }

    fn equal(&self, left: u8, right: u8) -> bool {
        if self.regex.case_insensitive {
            left.eq_ignore_ascii_case(&right)
        } else {
            left == right
        }
    }
}
