//! Validation for various kinds of git related items.
//!
//! ## Examples
//!
//! ```
//! use bstr::ByteSlice;
//!
//! assert!(gix_validate::reference::name(b"refs/heads/main".as_bstr()).is_ok());
//! assert!(gix_validate::tag::name(b"v1.2.3".as_bstr()).is_ok());
//! assert!(gix_validate::submodule::name(b"vendor/package".as_bstr()).is_ok());
//!
//! assert!(gix_validate::path::component(b"src".as_bstr(), None, Default::default()).is_ok());
//! assert!(gix_validate::path::component(b".git".as_bstr(), None, Default::default()).is_err());
//! ```
#![deny(missing_docs)]
#![forbid(unsafe_code)]

///
pub mod reference;

///
pub mod tag;

///
pub mod submodule;

///
pub mod path;
