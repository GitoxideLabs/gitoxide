//! Provides functions to quote and possibly unquote strings with different quoting styles.
//!
//! ## Examples
//!
//! ```
//! use bstr::ByteSlice;
//!
//! let shell_argument = gix_quote::single("hello it's git!".into());
//! assert_eq!(shell_argument, "'hello it'\\''s git'\\!''");
//!
//! let input = br#""line\nbreak""#.as_bstr();
//! let (unquoted, consumed) = gix_quote::ansi_c::undo(input).unwrap();
//! assert_eq!(unquoted.as_ref(), b"line\nbreak".as_bstr());
//! assert_eq!(consumed, input.len());
//! ```
#![deny(missing_docs)]
#![forbid(unsafe_code)]

///
pub mod ansi_c;

mod single;
pub use single::single;
