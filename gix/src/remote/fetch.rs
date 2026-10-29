#[cfg(feature = "blocking-network-client")]
use crate::Result;

///
pub mod negotiate {
    #[cfg(feature = "credentials")]
    pub use gix_negotiate::Algorithm;
}

#[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
pub use super::connection::fetch::{Outcome, Prepare, ProgressId, RefLogMessage, Status, outcome, prune, refs};

#[cfg(any(feature = "blocking-network-client", feature = "async-network-client-async-std"))]
pub(crate) use super::connection::fetch::PrepareDetached;
#[cfg(feature = "blocking-network-client")]
pub(crate) use super::connection::fetch::PrepareLocal;

/// If `Yes`, don't really make changes but do as much as possible to get an idea of what would be done.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
pub(crate) enum DryRun {
    /// Enable dry-run mode and don't actually change the underlying repository in any way.
    Yes,
    /// Run the operation like normal, making changes to the underlying repository.
    No,
}

/// How to deal with refs when cloning or fetching.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
pub(crate) enum WritePackedRefs {
    /// Normal operation, i.e. don't use packed-refs at all for writing.
    Never,
    /// Put ref updates straight into the `packed-refs` file, without creating loose refs first or dealing with them in any way.
    Only,
}

pub use gix_protocol::fetch::{Filter, Shallow, Tags};
#[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
pub use gix_protocol::fetch::{RefMap, refmap};

/// Options for [`crate::Remote::fetch()`].
#[cfg(feature = "blocking-network-client")]
#[derive(Default)]
pub struct Options {
    /// Additional ref mapping and handshake options.
    pub ref_map: crate::remote::ref_map::Options,
    /// Report proposed changes without writing objects, refs, or shallow state.
    pub dry_run: bool,
    /// Delete stale destinations covered by explicit refspecs before updating fetched refs.
    pub prune: bool,
    /// Add `refs/tags/*:refs/tags/*` to the explicit fetch refspecs.
    /// This fetches all tags even when automatic tag following is disabled, and permits
    /// pruning stale tags when `prune` is also enabled. It does not force tag updates.
    pub prune_tags: bool,
    /// Replace the worktree's `FETCH_HEAD` with the refs received by this fetch.
    ///
    /// Explicit refspecs are merge candidates, as with `git fetch <remote> <refspec>…`;
    /// implicit tags and non-commit targets are not for merging. The file is locked before
    /// fetching and atomically replaced afterwards. Unlike Git, errors leave the old file intact.
    /// Disabled by default and always ignored during a dry run.
    pub write_fetch_head: bool,
    /// Allow a selection whose exact source refs no longer exist.
    pub missing_refs_allowed: bool,
    /// Write reference updates directly to packed-refs.
    pub write_packed_refs_only: bool,
    /// Override the reflog message; by default it describes a fetch.
    pub reflog_message: Option<RefLogMessage>,
    /// How to adjust the shallow boundary.
    pub shallow: Shallow,
    /// Select objects for a partial fetch. Received packs are marked as promisor packs.
    pub filter: Option<Filter>,
}

#[cfg(feature = "blocking-network-client")]
impl crate::Remote<'_> {
    /// Return fetch options with the configured pruning policy for this remote.
    ///
    /// `remote.<name>.prune` and `remote.<name>.pruneTags` override `fetch.prune` and
    /// `fetch.pruneTags`, respectively. Unspecified values are false. Other options retain
    /// their defaults; tag following is already configured on the remote itself.
    /// Callers issuing explicit command-line refspecs may disable `prune_tags`, as Git does.
    pub fn fetch_options(&self) -> Result<Options> {
        use crate::config::tree::{Fetch, Key, Remote, keys};

        let config = &self.repo.config.resolved;
        let configured = |remote_key: &'static keys::Boolean, global_key: &'static keys::Boolean| -> Result<bool> {
            let remote = self
                .name()
                .map(|name| {
                    remote_key.enrich_error(config.boolean_filter_by(
                        "remote",
                        Some(name.as_bstr()),
                        remote_key.name(),
                        self.repo.filter_config_section(),
                    ))
                })
                .transpose()?
                .flatten();
            let global =
                global_key.enrich_error(config.boolean_filter(global_key, self.repo.filter_config_section()))?;
            Ok(remote.or(global).unwrap_or(false))
        };
        Ok(Options {
            prune: configured(&Remote::PRUNE, &Fetch::PRUNE)?,
            prune_tags: configured(&Remote::PRUNE_TAGS, &Fetch::PRUNE_TAGS)?,
            ..Default::default()
        })
    }
}
