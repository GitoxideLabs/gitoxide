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
        // TODO(review): kept concrete. Generic over the caller-supplied error `E` (default
        //                `std::convert::Infallible`); a `pub type Error = gix_error::Error` alias
        //                can't carry that parameter, and doing so would erase the caller's own error
        //                carried by `Other(E)`, which is the entire point of this type being generic —
        //                exercised at `gix/tests/gix/remote/fetch.rs:228` (`Error<std::io::Error>`)
        //                and `:245` (`Error::Other`). The `impl<E> crate::protocol::transport::
        //                IsSpuriousError for Error<E>` below would also become an orphan impl (that
        //                trait is `gix_transport`'s (`gix-transport/src/lib.rs:67`), merely re-exported
        //                through `gix_protocol::transport` and then as `crate::protocol`) on a foreign
        //                type if forced into a non-generic alias — E0117. `is_corrupted()` below also
        //                matches specific variants directly, e.g. `Open(open::Error::
        //                NotARepository { .. } | ...::Config(_))`.
        /// An error which combines all possible errors when opening a repository, finding remotes and using them to fetch.
        ///
        /// It can be used to detect if the repository is likely be corrupted in some way, or if the fetch failed spuriously
        /// and thus can be retried.
        #[derive(Debug, thiserror::Error)]
        #[expect(missing_docs)]
        pub enum Error<E: std::error::Error + Send + Sync + 'static = std::convert::Infallible> {
            #[error(transparent)]
            Open(#[from] crate::open::Error),
            #[error(transparent)]
            FindExistingReference(#[from] crate::reference::find::existing::Error),
            #[error(transparent)]
            RemoteInit(#[from] crate::remote::init::Error),
            #[error(transparent)]
            FindExistingRemote(#[from] crate::remote::find::existing::Error),
            #[error(transparent)]
            #[cfg(feature = "credentials")]
            CredentialHelperConfig(#[from] crate::config::credential_helpers::Error),
            #[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
            #[error(transparent)]
            Connect(#[from] crate::remote::connect::Error),
            #[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
            #[error(transparent)]
            PrepareFetch(#[from] crate::remote::fetch::prepare::Error),
            #[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
            #[error(transparent)]
            Fetch(#[from] crate::remote::fetch::Error),
            #[error(transparent)]
            Other(E),
        }

        #[cfg(any(feature = "async-network-client", feature = "blocking-network-client"))]
        impl<E> crate::protocol::transport::IsSpuriousError for Error<E>
        where
            E: std::error::Error + Send + Sync + 'static,
        {
            fn is_spurious(&self) -> bool {
                match self {
                    Error::Open(_)
                    | Error::CredentialHelperConfig(_)
                    | Error::RemoteInit(_)
                    | Error::FindExistingReference(_)
                    | Error::FindExistingRemote(_)
                    | Error::Other(_) => false,
                    Error::Connect(err) => err.is_spurious(),
                    Error::PrepareFetch(err) => err.is_spurious(),
                    Error::Fetch(err) => err.is_spurious(),
                }
            }
        }

        /// Queries
        impl<E> Error<E>
        where
            E: std::error::Error + Send + Sync + 'static,
        {
            /// Return true if repository corruption caused the failure.
            pub fn is_corrupted(&self) -> bool {
                match self {
                    Error::Open(crate::open::Error::NotARepository { .. } | crate::open::Error::Config(_)) => true,
                    #[cfg(any(feature = "async-network-client", feature = "blocking-network-client"))]
                    Error::PrepareFetch(crate::remote::fetch::prepare::Error::RefMap(
                        // Configuration couldn't be accessed or was incomplete.
                        crate::remote::ref_map::Error::GatherTransportConfig { .. }
                        | crate::remote::ref_map::Error::ConfigureCredentials(_),
                    )) => true,
                    // Maybe the value of the configuration was corrupted, or a file couldn't be removed.
                    #[cfg(any(feature = "async-network-client", feature = "blocking-network-client"))]
                    Error::Fetch(
                        crate::remote::fetch::Error::PackThreads(_)
                        | crate::remote::fetch::Error::PackIndexVersion(_)
                        | crate::remote::fetch::Error::RemovePackKeepFile { .. }
                        | crate::remote::fetch::Error::Fetch(gix_protocol::fetch::Error::Negotiate(_)),
                    ) => true,
                    _ => false,
                }
            }
        }
    }
}
