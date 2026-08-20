//! Utilities to handle program arguments and other values of interest.
use std::ffi::{OsStr, OsString};

use crate::bstr::{BString, ByteVec};

/// Returns the name of the agent for identification towards a remote server as statically known when compiling the crate.
/// Suitable for both `git` servers and HTTP servers, and used unless configured otherwise.
///
/// Note that it's meant to be used in conjunction with [`protocol::agent()`][crate::protocol::agent()] which
/// prepends `git/`.
pub fn agent() -> &'static str {
    concat!("oxide-", env!("CARGO_PKG_VERSION"))
}

/// Equivalent to `std::env::args_os()`, but with precomposed unicode on MacOS and other apple platforms.
/// It does not change the input arguments on any other platform.
///
/// Note that this ignores `core.precomposeUnicode` as git-config isn't available yet. It's default enabled in modern git though,
/// and generally decomposed unicode is nothing one would want in a git repository.
pub fn args_os() -> impl Iterator<Item = OsString> {
    args_os_opt(cfg!(target_vendor = "apple"))
}

/// Like [`args_os()`], but with the `precompose_unicode` parameter akin to `core.precomposeUnicode` in the Git configuration.
pub fn args_os_opt(precompose_unicode: bool) -> impl Iterator<Item = OsString> {
    std::env::args_os().map(move |arg| {
        if precompose_unicode {
            gix_utils::str::precompose_os_string(arg.into()).into_owned()
        } else {
            arg
        }
    })
}

/// Convert the given `input` into a `BString`, useful for usage in `clap`.
pub fn os_str_to_bstring(input: &OsStr) -> Option<BString> {
    Vec::from_os_string(input.into()).map(Into::into).ok()
}

/// Utilities to collate errors of common operations into one error type.
///
/// This is useful as this type can present an API to answer common questions, like whether a network request seems to have failed
/// spuriously or if the underlying repository seems to be corrupted.
/// Error collation supports all operations, including opening the repository.
///
/// ### Usage
///
/// The caller may define a function that specifies the result type as `Result<T, gix::env::collate::{operation}::Error>` to collect
/// errors into a well-known error type which provides an API for simple queries.
pub mod collate {

    ///
    pub mod fetch {
        /// An error which combines all possible errors when opening a repository, finding remotes and using them to fetch.
        pub type Error = gix_error::Error;
    }
}
