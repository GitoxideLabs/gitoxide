use std::{io, sync::atomic::AtomicBool};

use crate::{
    Progress, Remote, Result,
    bstr::{BString, ByteSlice},
    error::{ResultExt, bail, message},
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
    /// `options.follow_tags` overrides `push.followTags`; when enabled, missing annotated tags
    /// reachable from proposed or advertised tips are appended to the explicit updates.
    /// Named remotes with `remote.<name>.mirror=true` are rejected, as mirroring conflicts with explicit updates.
    ///
    /// Sender push certificates and recursive submodule publication are currently unsupported.
    /// Enabling `push.gpgSign` (including `if-asked`), `push.recurseSubmodules=check|on-demand|only`,
    /// or `submodule.recurse=true` is rejected before contacting the receiver. In particular,
    /// `if-asked` is refused even for servers that would allow an unsigned push. Later false values
    /// disable these policies; recursive settings follow configuration occurrence order.
    /// [`crate::remote::Connection::push()`] leaves sender policy to its caller.
    ///
    /// Like the connection API, this does not expand push refspecs or update local tracking refs.
    /// `options.pre_push_hook` lets the caller run sender hooks against the exact planned updates. Use [`Self::update_tracking_refs()`] for the latter and
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
        let (mut signing, mut recurse, mut follow_tags) = (false, false, false);
        let boolean = |value: Option<&BString>, key: &str| -> Result<bool> {
            value
                .map_or(Ok(true), |value| {
                    gix_config::Boolean::try_from(value.as_bstr()).map(|value| value.0)
                })
                .or_raise(|| message("Invalid boolean push configuration").with("key", key))
        };
        for section in config
            .plumbing()
            .sections()
            .filter(|section| section.header().subsection_name().is_none())
        {
            let is_push = section.header().name().eq_ignore_ascii_case(b"push");
            let is_submodule = section.header().name().eq_ignore_ascii_case(b"submodule");
            if !is_push && !is_submodule {
                continue;
            }
            for (name, value) in section.body().into_iter_implicit() {
                if is_push && name.eq_ignore_ascii_case("pushOption") {
                    let Some(value) = value else {
                        bail!(message("push.pushOption requires an explicit value").validation());
                    };
                    if value.is_empty() {
                        configured_options.clear();
                    } else {
                        configured_options.push(value);
                    }
                } else if is_push && name.eq_ignore_ascii_case("gpgSign") {
                    signing = value
                        .as_ref()
                        .is_some_and(|value| value.eq_ignore_ascii_case(b"if-asked"))
                        || boolean(value.as_ref(), "push.gpgSign")?;
                } else if is_push && name.eq_ignore_ascii_case("followTags") {
                    follow_tags = boolean(value.as_ref(), "push.followTags")?;
                } else if is_push && name.eq_ignore_ascii_case("recurseSubmodules") {
                    recurse = match value.as_ref().map(|value| value.as_slice()) {
                        Some(b"check" | b"on-demand" | b"only") => true,
                        _ => {
                            if boolean(value.as_ref(), "push.recurseSubmodules")? {
                                bail!(
                                    message("push.recurseSubmodules requires no, check, on-demand, or only")
                                        .validation()
                                );
                            }
                            false
                        }
                    };
                } else if is_submodule && name.eq_ignore_ascii_case("recurse") {
                    recurse = boolean(value.as_ref(), "submodule.recurse")?;
                }
            }
        }
        if signing {
            bail!(
                message("Configured push signing is not supported")
                    .with("key", "push.gpgSign")
                    .unsupported()
            );
        }
        if recurse {
            bail!(message("Configured recursive submodule pushes are not supported").unsupported());
        }
        let options = Options {
            follow_tags: Some(options.follow_tags.unwrap_or(follow_tags)),
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
