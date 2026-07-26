use crate::config;

// TODO(review): kept concrete, blocked three independent ways:
//                1. `impl gix_protocol::transport::IsSpuriousError for Error` below matches specific
//                   variants (`Fetch`, `Client`); as a `gix_error::Error` alias this becomes an orphan
//                   impl (neither the trait nor `gix_error::Error` are local to this crate) — E0117.
//                2. `clone::fetch::Error` (`gix/src/clone/fetch/mod.rs`) embeds this type via
//                   `Fetch(#[from] crate::remote::fetch::Error)`, but has already used its one erased
//                   slot via `ParseConfig(#[from] crate::config::overrides::Error)` — E0119.
//                3. Callers match on specific variants in `gix/src/env.rs`'s `is_corrupted()`:
//                   `PackThreads`, `PackIndexVersion`, `RemovePackKeepFile { .. }`,
//                   `Fetch(gix_protocol::fetch::Error::Negotiate(_))`, plus `Error::Fetch` in the
//                   `IsSpuriousError` impl on `env::collate::fetch::Error<E>` (that enum has no
//                   `Client` variant — `Error::Client` is matched only in this type's own
//                   `IsSpuriousError` impl below, already covered by point 1).
/// The error returned by [`receive()`](super::Prepare::receive()).
// TODO: remove unused variants
#[derive(Debug, thiserror::Error)]
#[expect(missing_docs)]
pub enum Error {
    #[error(transparent)]
    Fetch(#[from] gix_protocol::fetch::Error),
    #[error("The value to configure pack threads should be 0 to auto-configure or the amount of threads to use")]
    PackThreads(#[from] config::unsigned_integer::Error),
    #[error("The value to configure the pack index version should be 1 or 2")]
    PackIndexVersion(#[from] config::key::GenericError),
    #[error("Cannot fetch from a remote that uses {remote} while local repository uses {local} for object hashes")]
    IncompatibleObjectHash {
        local: gix_hash::Kind,
        remote: gix_hash::Kind,
    },
    #[error(transparent)]
    LoadAlternates(#[from] gix_odb::store::load_index::Error),
    #[error(transparent)]
    Client(#[from] gix_protocol::transport::client::Error),
    #[error(transparent)]
    UpdateRefs(#[from] super::refs::update::Error),
    #[error("Failed to remove .keep file at \"{}\"", path.display())]
    RemovePackKeepFile {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("None of the refspec(s) {} matched any of the {num_remote_refs} refs on the remote", refspecs.iter().map(|r| r.to_ref().instruction().to_bstring().to_string()).collect::<Vec<_>>().join(", "))]
    NoMapping {
        refspecs: Vec<gix_refspec::RefSpec>,
        num_remote_refs: usize,
    },
    #[error("Could not obtain configuration to learn if shallow remotes should be rejected")]
    RejectShallowRemoteConfig(#[from] config::boolean::Error),
    #[error(transparent)]
    NegotiationAlgorithmConfig(#[from] config::key::GenericErrorWithValue),
}

impl gix_protocol::transport::IsSpuriousError for Error {
    fn is_spurious(&self) -> bool {
        match self {
            Error::Fetch(err) => err.is_spurious(),
            Error::Client(err) => err.is_spurious(),
            _ => false,
        }
    }
}
