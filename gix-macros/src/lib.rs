//! A crate of useful macros used in `gix` primarily.
//!
//! Note that within `gix-*` crates, monomorphization should never be used for convenience, but only for performance
//! reasons. And in the latter case, manual denomophization should be considered if the trait in questions isn't called
//! often enough or measurements indicate that `&dyn Trait` is increasing the runtime. Thus, `gix-*` crates should probably
//! by default prefer using `&dyn` unless measurements indicate otherwise.
use proc_macro::TokenStream;

/// Turn async-shaped Rust code into its blocking equivalent by removing `async` and `.await` tokens.
#[proc_macro_attribute]
pub fn sync(_attrs: TokenStream, input: TokenStream) -> TokenStream {
    bisync::sync(input.into()).into()
}

/// Keep an item unchanged when selecting one side of a shared blocking and async implementation.
#[proc_macro_attribute]
pub fn keep(_attrs: TokenStream, input: TokenStream) -> TokenStream {
    input
}

/// Remove an item when selecting one side of a shared blocking and async implementation.
#[proc_macro_attribute]
pub fn discard(_attrs: TokenStream, _input: TokenStream) -> TokenStream {
    TokenStream::new()
}

/// Expose an item only within its crate unless the consuming crate enables `unstable`.
///
/// Supply `#[gix_macros::unstable(since = "YYYY-MM-DD")]` with the date the item was introduced.
/// The date must be a valid calendar date and is included in the enabled item's documentation.
/// Usually the date alone is enough; add an optional, nonempty `reason = "..."` only to explain
/// a non-obvious rationale. The gate also applies directly to file-backed modules
/// (`pub mod name;`), requiring Rust 1.99 or newer.
/// Inherent implementations retain their public members as `pub(crate)` when disabled;
/// trait implementations are omitted instead, since Rust has no private trait implementations.
#[proc_macro_attribute]
pub fn unstable(attrs: TokenStream, input: TokenStream) -> TokenStream {
    unstable::item(attrs.into(), input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Gate selected public struct fields while retaining their storage and access within the crate.
///
/// Mark each field with `#[unstable(since = "YYYY-MM-DD")]`, dating its introduction.
/// Add an optional, nonempty `reason = "..."` only when the rationale is not obvious.
/// With the consuming crate's `unstable` feature enabled it remains public and gains an instability
/// marker, the date and any supplied reason in its documentation; otherwise it
/// becomes `pub(crate)`. All unmarked fields are unchanged.
#[proc_macro_attribute]
pub fn unstable_fields(attrs: TokenStream, input: TokenStream) -> TokenStream {
    unstable::fields(attrs.into(), input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// When applied to functions or methods, it will turn it into a wrapper that will immediately call
/// a de-monomorphized implementation (i.e. one that uses `&dyn Trait`).
///
/// That way, the landing-pads for convenience will be as small as possible which then delegate to a single
/// function or method for implementation.
///
/// The parameters using the following traits can be de-monomorphized:
///
/// * `Into`
/// * `AsRef`
/// * `AsMut`
#[proc_macro_attribute]
pub fn momo(_attrs: TokenStream, input: TokenStream) -> TokenStream {
    momo::inner(input.into()).into()
}

mod bisync;
mod momo;
mod unstable;
