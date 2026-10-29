// Derived from posix-regex 0.1.4, upstream revision
// 063882aa6051b5075062d20d00b41ddc0fb8cd89.
// Copyright (c) 2018 jD91mZM2. Licensed under the MIT license; see LICENSE-MIT.

//! Byte-oriented POSIX basic and extended regular expressions, derived from Redox posix-regex.
//!
//! Expressions operate on arbitrary bytes with ASCII character classes, independently of the
//! process locale. The default GNU dialect supports captures, backreferences and GNU word
//! boundaries. An explicit Darwin dialect follows Git's libc ERE and enhanced BRE conventions.
//! Parsing limits pattern size and nesting; matching returns errors on exhausted
//! work or capture-state memory budgets instead of returning a partial result.

#![forbid(unsafe_code)]
#![cfg_attr(feature = "no_std", no_std)]

extern crate alloc;

pub mod compile;
pub mod ctype;
pub mod immut_vec;
pub mod matcher;
pub mod tree;

pub use compile::{Dialect, PosixRegexBuilder};
pub use matcher::PosixRegex;
