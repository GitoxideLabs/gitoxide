// Modified for gitoxide from the upstream imara-diff crate.
// Upstream source: git cat-file -p 32d1e45d3df061e6ccba6db7fdce92db29e345d8:src/intern.rs

use std::hash::{BuildHasher as _, Hash};
use std::ops::Index;

use hashbrown::DefaultHashBuilder as RandomState;
use hashbrown::hash_table::{Entry, HashTable};

/// A token represented as an interned integer.
///
/// A token represents the smallest possible unit of change during a diff.
/// For text this is usually a line, a word or a single character.
/// All [algorithms](crate::Algorithm) operate on interned tokens instead
/// of using the token data directly.
/// This allows for much better performance by amortizing the cost of hashing/equality.
///
/// While you can intern tokens yourself it is strongly recommended to use [`InternedInput`] module.
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
#[repr(transparent)]
pub struct Token(pub u32);

impl From<u32> for Token {
    fn from(token: u32) -> Self {
        Token(token)
    }
}

impl From<Token> for u32 {
    fn from(token: Token) -> Self {
        token.0
    }
}

/// A trait for types that can be split into tokens for diffing.
///
/// Implementing this trait allows a type to be used with [`InternedInput`] to create
/// interned token sequences for computing diffs. For example, `&str` implements this trait
/// by default to split text into lines.
pub trait TokenSource {
    /// The type of token this source produces. Must be hashable and comparable for equality.
    type Token: Hash + Eq;
    /// An iterator that yields tokens from this source.
    type Tokenizer: Iterator<Item = Self::Token>;
    /// Creates an iterator that yields all tokens from this source.
    fn tokenize(&self) -> Self::Tokenizer;
    /// Provides an estimate of the number of tokens this source will produce.
    ///
    /// This is used to pre-allocate memory for better performance. The estimate
    /// does not need to be exact.
    fn estimate_tokens(&self) -> u32;
}

/// Two lists of interned [tokens](Token) that a [`Diff`](crate::Diff) can be computed from.
///
/// A token represents the smallest possible unit of change during a diff.
/// For text this is usually a line, a word or a single character.
/// All [algorithms](crate::Algorithm) operate on interned tokens instead
/// of using the token data directly.
/// This allows for much better performance by amortizing the cost of hashing/equality.
///
/// While you can intern tokens yourself it is strongly recommended to use [`InternedInput`] module.
#[derive(Default)]
pub struct InternedInput<T> {
    /// The list of interned tokens from the first sequence (before changes).
    pub before: Vec<Token>,
    /// The list of interned tokens from the second sequence (after changes).
    pub after: Vec<Token>,
    /// The interner that stores the actual token data and maps tokens to their interned IDs.
    pub interner: Interner<T>,
}

impl<T> InternedInput<T> {
    /// Clears all token sequences and the interner.
    ///
    /// This removes all tokens from both the before and after sequences, as well as
    /// clearing the interner's storage.
    ///
    /// Note that this will not free the allocated memory.
    pub fn clear(&mut self) {
        self.before.clear();
        self.after.clear();
        self.interner.clear();
    }
}

impl<T: Eq + Hash> InternedInput<T> {
    /// Creates a new `InternedInput` by tokenizing and interning two token sources.
    ///
    /// # Parameters
    ///
    /// * `before` - The token source for the first sequence
    /// * `after` - The token source for the second sequence
    ///
    /// # Returns
    ///
    /// An `InternedInput` containing interned token sequences ready for diffing
    pub fn new<I: TokenSource<Token = T>>(before: I, after: I) -> Self {
        let token_estimate_before = before.estimate_tokens() as usize;
        let token_estimate_after = after.estimate_tokens() as usize;
        let mut res = Self {
            before: Vec::with_capacity(token_estimate_before),
            after: Vec::with_capacity(token_estimate_after),
            interner: Interner::new(token_estimate_before + token_estimate_after),
        };
        res.update_before(before.tokenize());
        res.update_after(after.tokenize());
        res
    }

    /// Reserve capacity so that `before` and `after` would not need
    /// to allocate if their [`estimate_tokens`](TokenSource::estimate_tokens)
    /// would represent an exact match of their actual tokens.
    ///
    /// Useful for minimization of allocation before calls to
    /// [`update_before`](InternedInput::update_before) and
    /// [`update_after`](InternedInput::update_after).
    pub fn reserve_for_token_source<S: TokenSource<Token = T> + ?Sized>(&mut self, before: &S, after: &S) {
        self.reserve(before.estimate_tokens(), after.estimate_tokens())
    }

    /// Reserves capacity for the specified number of tokens in each sequence.
    ///
    /// # Parameters
    ///
    /// * `capacity_before` - The number of tokens to reserve for the "before" sequence
    /// * `capacity_after` - The number of tokens to reserve for the "after" sequence
    pub fn reserve(&mut self, capacity_before: u32, capacity_after: u32) {
        self.before.reserve(capacity_before as usize);
        self.after.reserve(capacity_after as usize);
        self.interner
            .reserve(capacity_before as usize + capacity_after as usize);
    }

    /// replaces `self.before` with the interned Tokens yielded by `input`
    /// Note that this does not erase any tokens from the interner and might therefore be considered
    /// a memory leak. If this function is called often over a long-running process
    /// consider clearing the interner with [`clear`](Interner::clear).
    pub fn update_before(&mut self, input: impl Iterator<Item = T>) {
        self.before.clear();
        self.before.extend(input.map(|token| self.interner.intern(token)));
    }

    /// replaces `self.before` with the interned Tokens yielded by `input`
    /// Note that this does not erase any tokens from the interner and might therefore be considered
    /// a memory leak. If this function is called often over a long-running process
    /// consider clearing the interner with [`clear`](Interner::clear) or
    /// [`erase_tokens_after`](Interner::erase_tokens_after).
    pub fn update_after(&mut self, input: impl Iterator<Item = T>) {
        self.after.clear();
        self.after.extend(input.map(|token| self.interner.intern(token)));
    }
}

/// An interner that allows for fast access of tokens produced by a [`TokenSource`].
#[derive(Default)]
pub struct Interner<T> {
    tokens: Vec<T>,
    table: HashTable<Token>,
    hasher: RandomState,
}

impl<T> Interner<T> {
    /// Create an Interner with an initial capacity calculated by summing the results of calling
    /// [`estimate_tokens`](TokenSource::estimate_tokens) methods of `before` and `after`.
    pub fn new_for_token_source<S: TokenSource<Token = T>>(before: &S, after: &S) -> Self {
        Self::new(before.estimate_tokens() as usize + after.estimate_tokens() as usize)
    }

    /// Create an Interner with initial capacity `capacity`.
    pub fn new(capacity: usize) -> Interner<T> {
        Interner {
            tokens: Vec::with_capacity(capacity),
            table: HashTable::with_capacity(capacity),
            hasher: RandomState::default(),
        }
    }

    /// Remove all interned tokens.
    pub fn clear(&mut self) {
        self.table.clear();
        self.tokens.clear();
    }

    /// Returns to total number of **distinct** tokens currently interned.
    pub fn num_tokens(&self) -> u32 {
        self.tokens.len() as u32
    }
}

impl<T: Hash + Eq> Interner<T> {
    /// Create an Interner with an initial capacity calculated by calling
    /// [`estimate_tokens`](TokenSource::estimate_tokens) methods of `before` and `after`
    pub fn reserve_for_token_source<S: TokenSource<Token = T>>(&mut self, before: &S, after: &S) {
        self.reserve(before.estimate_tokens() as usize + after.estimate_tokens() as usize)
    }

    /// Reserves capacity for at least the specified number of additional tokens.
    ///
    /// # Parameters
    ///
    /// * `capacity` - The number of additional tokens to reserve space for
    pub fn reserve(&mut self, capacity: usize) {
        self.table
            .reserve(capacity, |&token| self.hasher.hash_one(&self.tokens[token.0 as usize]));
        self.tokens.reserve(capacity);
    }

    /// Intern `token` and return the interned integer.
    pub fn intern(&mut self, token: T) -> Token {
        let hash = self.hasher.hash_one(&token);
        match self.table.entry(
            hash,
            |&it| self.tokens[it.0 as usize] == token,
            |&token| self.hasher.hash_one(&self.tokens[token.0 as usize]),
        ) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let interned = Token(self.tokens.len() as u32);
                entry.insert(interned);
                self.tokens.push(token);
                interned
            }
        }
    }

    /// Erases `first_erased_token` and any tokens interned afterward from the interner.
    pub fn erase_tokens_after(&mut self, first_erased_token: Token) {
        assert!(first_erased_token.0 <= self.tokens.len() as u32);
        let retained = first_erased_token.0 as usize;
        let erased = self.tokens.len() - retained;
        if retained <= erased {
            self.table.clear();
            for (i, token) in self.tokens[0..retained].iter().enumerate() {
                let hash = self.hasher.hash_one(token);
                self.table.insert_unique(hash, Token(i as u32), |&token| {
                    self.hasher.hash_one(&self.tokens[token.0 as usize])
                });
            }
        } else {
            for (i, token) in self.tokens[retained..].iter().enumerate() {
                let hash = self.hasher.hash_one(token);
                match self.table.find_entry(hash, |token| token.0 == (retained + i) as u32) {
                    Ok(occupied) => drop(occupied.remove()),
                    Err(_absent) => unreachable!(),
                }
            }
        }
        self.tokens.truncate(first_erased_token.0 as usize);
    }
}

impl<T> Index<Token> for Interner<T> {
    type Output = T;
    fn index(&self, index: Token) -> &Self::Output {
        &self.tokens[index.0 as usize]
    }
}
