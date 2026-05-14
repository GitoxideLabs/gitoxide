//! Parse git ref-specs and represent them.
//!
//! ## Examples
//!
//! ```
//! use bstr::ByteSlice;
//! use gix_refspec::parse::Operation;
//!
//! let spec = gix_refspec::parse(
//!     "refs/heads/*:refs/remotes/origin/*".into(),
//!     Operation::Fetch,
//! )
//! .unwrap();
//!
//! assert_eq!(spec.remote().unwrap(), "refs/heads/*");
//! assert_eq!(spec.local().unwrap(), "refs/remotes/origin/*");
//! assert_eq!(spec.prefix().unwrap(), "refs/heads/");
//!
//! let mut prefixes = Vec::new();
//! spec.expand_prefixes(&mut prefixes);
//! assert_eq!(prefixes.len(), 1);
//! assert_eq!(prefixes[0].as_bstr(), "refs/heads/");
//!
//! assert_eq!(spec.to_bstring(), "refs/heads/*:refs/remotes/origin/*");
//! ```
#![deny(missing_docs)]
#![forbid(unsafe_code)]

///
pub mod parse;
pub use parse::function::parse;

///
pub mod instruction;

/// A refspec with references to the memory it was parsed from.
#[derive(Eq, Copy, Clone, Debug)]
pub struct RefSpecRef<'a> {
    mode: types::Mode,
    op: parse::Operation,
    src: Option<&'a bstr::BStr>,
    dst: Option<&'a bstr::BStr>,
}

/// An owned refspec.
#[derive(Eq, Clone, Debug)]
pub struct RefSpec {
    mode: types::Mode,
    op: parse::Operation,
    src: Option<bstr::BString>,
    dst: Option<bstr::BString>,
}

mod spec;

mod write;

///
pub mod match_group;
pub use match_group::types::MatchGroup;

mod types;
pub use types::Instruction;
