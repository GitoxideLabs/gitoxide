use std::{io, sync::atomic::AtomicBool};

use crate::{
    Progress, Remote, Result,
    bstr::ByteSlice,
    push::{Options, Outcome, Update},
    remote::Direction,
};

impl Remote<'_> {
    /// Push explicit updates to the first push URL, using native repository operations for local paths.
    ///
    /// URL rewriting and protocol permissions are the same as [`Self::connect()`]. Network URLs
    /// use [`crate::remote::Connection::push()`] with configured credentials. Local paths do not
    /// launch Git and protect advertised values with reference transactions.
    ///
    /// Like the connection API, this does not expand push refspecs, run local pre-push hooks, or
    /// update local tracking refs. Use [`Self::urls()`] to handle additional configured push URLs.
    /// Local receive operations run receiver hooks and quarantine incoming objects. They preserve receiver
    /// shallow boundaries and honor `receive.shallowUpdate` when incoming history needs additional boundaries.
    /// `receive.denyCurrentBranch=updateInstead` delegates deployment to a `push-to-checkout` hook;
    /// its built-in worktree update is currently unsupported when no executable hook is present.
    /// Namespaces and `receive.hideRefs` rules are applied to advertisements, transactions, and hooks.
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
        if url.scheme != gix_url::Scheme::File {
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
        let namespace = self
            .command_environment
            .iter()
            .rev()
            .find(|(key, _)| key == "GIT_NAMESPACE")
            .map(|(_, value)| value.clone())
            .or_else(|| {
                self.repo
                    .options
                    .permissions
                    .env
                    .git_prefix
                    .check_opt("GIT_NAMESPACE")
                    .and_then(std::env::var_os)
            });
        if let Some(namespace) = namespace {
            if namespace.is_empty() {
                destination.clear_namespace();
            } else {
                destination.set_namespace(gix_path::os_string_into_bstring(namespace)?.as_bstr())?;
            }
        }
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
