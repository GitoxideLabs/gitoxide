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
/// A nonempty `reason = "..."` is required and appears in the enabled item's documentation.
/// Inherent implementations retain their public members as `pub(crate)` when disabled;
/// trait implementations are omitted instead, since Rust has no private trait implementations.
#[proc_macro_attribute]
pub fn unstable(attrs: TokenStream, input: TokenStream) -> TokenStream {
    unstable::item(attrs.into(), input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Gate a file-backed module on Rust versions which cannot apply attributes to `mod name;`.
///
/// Use `unstable_module! { reason = "..."; pub mod name; }`.
#[proc_macro]
pub fn unstable_module(input: TokenStream) -> TokenStream {
    unstable::module(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Gate selected public struct fields while retaining their storage and access within the crate.
///
/// Mark each field with `#[unstable(reason = "...")]`. With the consuming crate's `unstable`
/// feature enabled it remains public and gains that reason in its documentation; otherwise it
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
