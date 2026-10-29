use std::{io, sync::atomic::AtomicBool};

use crate::{
    Progress, Remote, Result,
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
    ///
    /// Like the connection API, this does not expand push refspecs, run local pre-push hooks, or
    /// update local tracking refs. Use [`Self::urls()`] to handle additional configured push URLs.
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
}
