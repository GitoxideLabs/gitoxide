// Derived from posix-regex 0.1.4, upstream revision
// 063882aa6051b5075062d20d00b41ddc0fb8cd89.
// Copyright (c) 2018 jD91mZM2. Licensed under the MIT license; see LICENSE-MIT.

//! Byte-oriented POSIX basic and extended regular expressions, derived from Redox posix-regex.
//!
//! This crate provides the imported parsing and matching foundation. Git-specific compatibility
//! fixes are developed here before the engine is used to interpret repository configuration.

#![forbid(unsafe_code)]
#![cfg_attr(feature = "no_std", no_std)]

extern crate alloc;

pub mod compile;
pub mod ctype;
pub mod immut_vec;
pub mod matcher;
pub mod tree;

pub use compile::PosixRegexBuilder;
pub use matcher::PosixRegex;
