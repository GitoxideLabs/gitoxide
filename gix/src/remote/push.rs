use std::{io, sync::atomic::AtomicBool};

use crate::{
    Progress, Remote, Result,
    bstr::BString,
    error::{bail, message},
    push::{Options, Outcome, Update},
    remote::Direction,
};

impl Remote<'_> {
    /// Push explicit updates to the first push URL, using native repository operations for local paths.
    ///
    /// URL rewriting and protocol permissions are the same as [`Self::connect()`]. Network URLs
    /// use [`crate::remote::Connection::push()`] with configured credentials. Local paths do not
    /// launch Git and protect advertised values with reference transactions, unless a receive-pack
    /// command is explicitly configured, in which case that command handles the receive operation.
    /// An empty `options.push_options` uses `push.pushOption` defaults, including empty-value resets.
    /// A nonempty slice, even one containing an empty option, overrides those defaults.
    /// Named remotes with `remote.<name>.mirror=true` are rejected, as mirroring conflicts with explicit updates.
    ///
    /// Like the connection API, this does not expand push refspecs, run local pre-push hooks, or
    /// update local tracking refs. Use [`Self::update_tracking_refs()`] for the latter and
    /// [`Self::urls()`] to handle additional configured push URLs.
    /// Local receive operations run configured and traditional receiver hooks serially and quarantine incoming objects.
    /// Event-wide disables suppress both hook forms. Receiver shallow boundaries are preserved, honoring
    /// `receive.shallowUpdate` when incoming history needs additional boundaries.
    /// `receive.denyCurrentBranch=updateInstead` delegates deployment to a `push-to-checkout` hook;
    /// its built-in worktree update is currently unsupported when no enabled hook is present.
    /// Incoming objects honor `receive.fsckObjects` and `receive.fsck` severity and skip-list policy.
    /// Namespaces and `receive.hideRefs` rules are applied to advertisements, transactions, and hooks.
    /// Reference-transaction hooks can reject during `preparing` before locks or `prepared` after locking.
    /// Advanced receive policies that cannot be enforced are rejected before modifying references.
    pub fn push(
        &self,
        updates: impl IntoIterator<Item = Update>,
        options: Options<'_>,
        progress: impl Progress,
        remote_progress: impl FnMut(bool, &[u8]) -> io::Result<()>,
        should_interrupt: &AtomicBool,
    ) -> Result<Outcome> {
        let config = self.repo.config_snapshot();
        if let Some(name) = self.name()
            && config
                .try_boolean(gix_config::KeyRef {
                    section_name: "remote",
                    subsection_name: Some(name.as_bstr()),
                    value_name: "mirror",
                })?
                .unwrap_or(false)
        {
            bail!(message("Mirroring cannot be combined with explicit push updates").validation());
        }
        let mut configured_options = Vec::<BString>::new();
        for section in config.plumbing().sections().filter(|section| {
            section.header().subsection_name().is_none() && section.header().name().eq_ignore_ascii_case(b"push")
        }) {
            for (_, value) in section
                .body()
                .into_iter_implicit()
                .filter(|(name, _)| name.eq_ignore_ascii_case("pushOption"))
            {
                let Some(value) = value else {
                    bail!(message("push.pushOption requires an explicit value").validation());
                };
                if value.is_empty() {
                    configured_options.clear();
                } else {
                    configured_options.push(value);
                }
            }
        }
        let options = Options {
            push_options: if options.push_options.is_empty() {
                &configured_options
            } else {
                options.push_options
            },
            ..options
        };
        let (url, _) = self.sanitized_url_and_version(Direction::Push)?;
        if url.scheme != gix_url::Scheme::File || self.service_command(Direction::Push)?.is_some() {
            return self
                .connect(Direction::Push)?
                .push(updates, options, progress, remote_progress, should_interrupt);
        }
        let mut open = self.repo.options.clone().without_repository_environment_overrides();
        // API and CLI overrides belong to the source repository; receiver configuration belongs
        // to the destination. Keep the caller's configuration-source permissions and pinned paths.
        open.api_config_overrides.clear();
        open.cli_config_overrides.clear();
        let mut destination = crate::open_opts(gix_path::from_bstr(url.path)?.into_owned(), open)?;
        super::local::apply_namespace(self, &mut destination)?;
        crate::push::local::push(
            self,
            &mut destination,
            updates,
            options,
            progress,
            remote_progress,
            should_interrupt,
        )
    }

    /// Update this remote's tracking references from a confirmed push result.
    ///
    /// Apply the first matching fetch refspec to each successful or up-to-date destination,
    /// honoring negative specs and any actual destination or value reported by proc-receive.
    /// Dry runs and rejected updates do nothing. Deletions remove the tracking reference and
    /// its log; other updates record `update by push`. Missing committer identity uses the same
    /// in-memory generic fallback as cloning.
    /// Reported objects must exist locally, and mappings into `refs/heads/` require commits.
    ///
    /// Reference edits use one transaction. An error affects only this local update step; the
    /// remote result remains available in `outcome`, so callers can report a local failure
    /// without losing confirmation that the push already succeeded.
    pub fn update_tracking_refs(&self, outcome: &Outcome) -> Result<Vec<gix_ref::transaction::RefEdit>> {
        crate::push::tracking::update_refs(self, outcome)
    }
}
