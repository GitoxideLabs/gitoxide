use crate::{
    bstr::{BStr, BString, ByteSlice},
    clone::PrepareFetch,
};
use gix_ref::Category;

use crate::config::tree::Key;

/// The error returned by [`PrepareFetch::fetch_only()`].
#[derive(Debug, thiserror::Error)]
#[expect(missing_docs)]
pub enum Error {
    #[error(transparent)]
    Connect(#[from] crate::remote::connect::Error),
    #[error(transparent)]
    PrepareFetch(#[from] crate::remote::fetch::prepare::Error),
    #[error(transparent)]
    Fetch(#[from] crate::remote::fetch::Error),
    #[error(transparent)]
    RemoteInit(#[from] crate::remote::init::Error),
    #[error("Custom configuration of remote to clone from failed")]
    RemoteConfiguration(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("Custom configuration of connection to use when cloning failed")]
    RemoteConnection(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error(transparent)]
    RemoteName(#[from] crate::config::remote::symbolic_name::Error),
    #[error(transparent)]
    ParseConfig(#[from] crate::config::overrides::Error),
    #[error(transparent)]
    ApplyConfig(#[from] crate::config::Error),
    #[error(transparent)]
    Span(#[from] gix_config::parse::span::Error),
    #[error(transparent)]
    ConfigValue(#[from] gix_config::file::section::value::Error),
    #[error("Failed to load repo-local git configuration before writing")]
    LoadConfig(#[from] gix_config::file::init::from_paths::Error),
    #[error("Failed to store configured remote in memory")]
    SaveConfig(#[from] crate::remote::save::AsError),
    #[error("Failed to write repository configuration to disk")]
    SaveConfigIo(#[from] std::io::Error),
    #[error("Failed to acquire lock to write repository configuration to disk")]
    SaveConfigLockAcquire(#[from] gix_lock::acquire::Error),
    #[error("Failed to commit lock after writing repository configuration to disk")]
    SaveConfigLockCommit(#[from] gix_lock::commit::Error<gix_lock::File>),
    #[error("The remote HEAD points to a reference named {head_ref_name:?} which is invalid.")]
    InvalidHeadRef {
        source: gix_validate::reference::name::Error,
        head_ref_name: crate::bstr::BString,
    },
    #[error("Failed to update HEAD with values from remote")]
    HeadUpdate(#[from] crate::reference::edit::Error),
    #[error("The remote didn't have any ref that matched '{}'", wanted.as_ref().as_bstr())]
    RefNameMissing { wanted: gix_ref::PartialName },
    #[error("The remote has {} refs for '{}', try to use a specific name: {}", candidates.len(), wanted.as_ref().as_bstr(), candidates.iter().filter_map(|n| n.to_str().ok()).collect::<Vec<_>>().join(", "))]
    RefNameAmbiguous {
        wanted: gix_ref::PartialName,
        candidates: Vec<BString>,
    },
    #[error("The remote didn't have the requested revision {wanted:?}")]
    RevisionMissing { wanted: BString },
    #[error("The requested revision could not be read")]
    FindRevision(#[from] crate::object::find::existing::Error),
    #[error("The requested revision did not peel to a commit")]
    PeelRevision(#[from] crate::object::peel::to_kind::Error),
    #[error(transparent)]
    CommitterOrFallback(#[from] crate::config::commit_signature::Error),
    #[error(transparent)]
    RefMap(#[from] crate::remote::ref_map::Error),
    #[error(transparent)]
    ReferenceName(#[from] gix_validate::reference::name::Error),
    #[error(
        "Remote now uses {remote} even though the local repository was reconfigured to {local} \
        to match the remote's previously advertised object format. The server is advertising \
        inconsistent formats; if the correct one is known, pass `create::Options {{ \
        object_hash: Some(gix_hash::Kind::{remote:?}), .. }}` to `PrepareFetch::new` to pin it."
    )]
    IncompatibleObjectHash {
        local: gix_hash::Kind,
        remote: gix_hash::Kind,
    },
    #[cfg(feature = "sha256")]
    #[error("Failed to reopen the local repository after adopting the remote's object format")]
    ReopenWithObjectHash(#[from] crate::open::Error),
    #[cfg(feature = "sha256")]
    #[error("Failed to transfer in-memory configuration after adopting the remote's object format")]
    TransferInMemoryConfig(#[from] gix_config::file::init::Error),
    #[cfg(feature = "sha256")]
    #[error("Failed to inspect the pristine reftable HEAD before adopting the remote object format")]
    InspectReftableHead(#[from] gix_ref::store::find::existing::Error),
    #[cfg(feature = "sha256")]
    #[error("The pristine reftable HEAD is not symbolic")]
    ReftableHeadNotSymbolic,
    #[cfg(feature = "sha256")]
    #[error("Failed to inspect whether the reftable stack is still pristine")]
    InspectReftablePristine {
        #[source]
        source: gix_ref::store::BackendError,
    },
    #[cfg(feature = "sha256")]
    #[error("Refusing to replace a reftable stack after clone initialization wrote references")]
    ReftableNotPristine,
    #[cfg(feature = "sha256")]
    #[error("Failed to initialize the negotiated reftable object format")]
    ReinitializeReferenceStorage(#[from] gix_ref::store::BackendError),
    #[cfg(feature = "sha256")]
    #[error("Could not {operation} during the pristine reftable object-format handoff at '{}'", path.display())]
    ReftableHandoffIo {
        source: std::io::Error,
        operation: &'static str,
        path: std::path::PathBuf,
    },
    #[cfg(feature = "sha256")]
    #[error("Could not lock '{}' to {operation} during the pristine reftable object-format handoff", path.display())]
    ReftableHandoffLock {
        source: gix_lock::acquire::Error,
        operation: &'static str,
        path: std::path::PathBuf,
    },
    #[cfg(feature = "sha256")]
    #[error("The pristine reftable object-format handoff failed ({original}); rollback also failed")]
    ReftableHandoffRollback {
        original: Box<Error>,
        #[source]
        rollback: Box<Error>,
    },
    #[cfg(feature = "sha256")]
    #[error(
        "The previous pristine reftable object-format handoff had an incomplete rollback; this clone preparation cannot be retried safely"
    )]
    ReftableHandoffRetryBlocked,
}

/// Modification
impl PrepareFetch {
    /// Fetch a pack and update local branches according to refspecs, providing `progress` and checking `should_interrupt` to stop
    /// the operation.
    /// On success, the persisted repository is returned, and this method must not be called again to avoid a **panic**.
    /// On error, the method may be called again to retry as often as needed, except after
    /// [`Error::ReftableHandoffRollback`]. That error means repository rollback itself was
    /// incomplete, so later calls fail with [`Error::ReftableHandoffRetryBlocked`] without
    /// using a potentially stale repository handle.
    ///
    /// If the remote repository was empty, that is newly initialized, the returned repository will also be empty and like
    /// it was newly initialized.
    ///
    /// Note that all data we created will be removed once this instance drops if the operation wasn't successful.
    ///
    /// ### Note for users of `async`
    ///
    /// Even though `async` is technically supported, it will still be blocking in nature as it uses a lot of non-async writes
    /// and computation under the hood. Thus it should be spawned into a runtime which can handle blocking futures.
    #[gix_protocol::bisync::bisync]
    pub async fn fetch_only<P>(
        &mut self,
        mut progress: P,
        should_interrupt: &std::sync::atomic::AtomicBool,
    ) -> Result<(crate::Repository, crate::remote::fetch::Outcome), Error>
    where
        P: crate::NestedProgress,
        P::SubProgress: 'static,
    {
        use crate::{bstr::ByteVec, remote, remote::fetch::RefLogMessage};

        #[cfg(feature = "sha256")]
        if self.retry_blocked_by_handoff_rollback {
            return Err(Error::ReftableHandoffRetryBlocked);
        }

        let mut repo = self
            .repo
            .as_ref()
            .expect("user error: multiple calls are allowed only until it succeeds")
            .clone();

        repo.committer_or_set_generic_fallback()?;

        if !self.config_overrides.is_empty() {
            let mut snapshot = repo.config_snapshot_mut();
            snapshot.append_config(&self.config_overrides, gix_config::Source::Api)?;
        }

        let remote_name = match self.remote_name.as_ref() {
            Some(name) => name.to_owned(),
            None => repo
                .config
                .resolved
                .string(crate::config::tree::Clone::DEFAULT_REMOTE_NAME)
                .map(|n| crate::config::tree::Clone::DEFAULT_REMOTE_NAME.try_into_symbolic_name(n))
                .transpose()?
                .unwrap_or_else(|| {
                    crate::config::tree::Clone::DEFAULT_REMOTE_NAME
                        .default_value_or_panic()
                        .into()
                }),
        };

        let mut remote = repo.remote_at(self.url.clone())?;

        // For shallow clones without custom configuration, we'll use a single-branch refspec
        // to match git's behavior (matching git's single-branch behavior for shallow clones).
        let use_single_branch_for_shallow = self.shallow != remote::fetch::Shallow::NoChange
            && remote.fetch_specs.is_empty()
            && self.fetch_options.extra_refspecs.is_empty()
            && self.revision.is_none();

        let target_ref = if use_single_branch_for_shallow {
            // Determine target branch from user-specified ref_name or default branch
            if let Some(ref_name) = &self.ref_name {
                let prev_tags = std::mem::replace(&mut remote.fetch_tags, remote::fetch::Tags::None);
                let mut connection = remote.connect(remote::Direction::Fetch).await?;
                if let Some(f) = self.configure_connection.as_mut() {
                    f(&mut connection).map_err(Error::RemoteConnection)?;
                }
                let (refmap, _) = connection
                    .ref_map(
                        &mut progress,
                        remote::ref_map::Options {
                            extra_refspecs: vec![
                                gix_refspec::parse(ref_name.as_ref().as_bstr(), gix_refspec::parse::Operation::Fetch)
                                    .expect("partial names are valid refspecs")
                                    .to_owned(),
                            ],
                            ..Default::default()
                        },
                    )
                    .await?;
                let (_target, full_ref_name) = util::find_custom_refname(&refmap, ref_name)?;
                remote.fetch_tags = prev_tags;
                Some(full_ref_name.try_into()?)
            } else {
                // For shallow clones without a specified ref, we need to determine the ref to clone.
                // Just fetch HEAD for that.
                let prev_tags = std::mem::replace(&mut remote.fetch_tags, remote::fetch::Tags::None);
                let mut connection = remote.connect(remote::Direction::Fetch).await?;
                if let Some(f) = self.configure_connection.as_mut() {
                    f(&mut connection).map_err(Error::RemoteConnection)?;
                }
                let (refmap, _) = connection
                    .ref_map(
                        &mut progress,
                        remote::ref_map::Options {
                            extra_refspecs: vec![
                                gix_refspec::parse("HEAD".into(), gix_refspec::parse::Operation::Fetch)
                                    .expect("valid")
                                    .to_owned(),
                            ],
                            ..Default::default()
                        },
                    )
                    .await?;

                // Find HEAD in the remote refs (works for both Protocol V1 and V2)
                let target = refmap
                    .remote_refs
                    .iter()
                    .find_map(|r| match r {
                        gix_protocol::handshake::Ref::Symbolic {
                            full_ref_name, target, ..
                        }
                        | gix_protocol::handshake::Ref::Unborn {
                            full_ref_name, target, ..
                        } if full_ref_name == "HEAD" => gix_ref::FullName::try_from(target)
                            .map_err(|err| Error::InvalidHeadRef {
                                head_ref_name: target.clone(),
                                source: err,
                            })
                            .into(),
                        _ => None,
                    })
                    .transpose()?;

                let target = target.ok_or_else(|| Error::RefNameMissing {
                    wanted: "HEAD".try_into().expect("valid partial name"),
                })?;

                remote.fetch_tags = prev_tags;

                Some(target)
            }
        } else {
            None
        };

        // Set up refspec based on whether we're doing a single-branch shallow clone,
        // which requires a single ref to match Git unless it's overridden.
        if remote.fetch_specs.is_empty() && self.revision.is_none() {
            if let Some(target_ref) = &target_ref {
                // Single-branch refspec for shallow clones
                let destination = match target_ref.category_and_short_name() {
                    Some((Category::LocalBranch, short_name)) => {
                        format!("refs/remotes/{remote_name}/{short_name}")
                    }
                    _ => target_ref.to_string(),
                };
                remote = remote
                    .with_refspecs(
                        Some(format!("+{target_ref}:{destination}").as_str()),
                        remote::Direction::Fetch,
                    )
                    .expect("valid refspec");
            } else {
                // Wildcard refspec for non-shallow clones or when target couldn't be determined
                remote = remote
                    .with_refspecs(
                        Some(format!("+refs/heads/*:refs/remotes/{remote_name}/*").as_str()),
                        remote::Direction::Fetch,
                    )
                    .expect("valid static spec");
            }
        }

        let mut clone_fetch_tags = None;
        if let Some(f) = self.configure_remote.as_mut() {
            remote = f(remote).map_err(Error::RemoteConfiguration)?;
        } else if self.revision.is_none() {
            clone_fetch_tags = remote::fetch::Tags::All.into();
        }
        if self.revision.is_some() {
            remote
                .replace_refspecs(std::iter::empty::<&BStr>(), remote::Direction::Fetch)
                .expect("an empty refspec list is always valid");
            remote = remote.with_fetch_tags(remote::fetch::Tags::None);
        }

        // The configurator is stateful and runs on every attempt, so persist its
        // complete result on every attempt as an idempotent replacement. Retain
        // the same resolved view for a later retry before performing any I/O.
        let resolved_config = util::upsert_remote_in_local_config(&mut remote, remote_name.clone())?;
        self.repo
            .as_mut()
            .expect("the repository is retained until success")
            .reread_values_and_clear_caches_replacing_config(resolved_config.clone().into())?;
        // These overrides now live in the retained repository configuration. Keeping
        // them here as well would append multi-valued settings again on every retry.
        self.config_overrides.clear();

        // Now we are free to apply remote configuration we don't want to be written to disk.
        if let Some(fetch_tags) = clone_fetch_tags {
            remote = remote.with_fetch_tags(fetch_tags);
        }

        // Add HEAD after the remote was written to config, we need it to know what to check out later, and assure
        // the ref that HEAD points to is present no matter what.
        let head_local_tracking_branch = format!("refs/remotes/{remote_name}/HEAD");
        let head_refspec = gix_refspec::parse(
            format!("HEAD:{head_local_tracking_branch}").as_str().into(),
            gix_refspec::parse::Operation::Fetch,
        )
        .expect("valid")
        .to_owned();
        let pending_pack = {
            // For shallow clones, we already connected once, so we need to connect again
            let mut connection = remote.connect(remote::Direction::Fetch).await?;
            if let Some(f) = self.configure_connection.as_mut() {
                f(&mut connection).map_err(Error::RemoteConnection)?;
            }
            let connection = connection.into_detached();
            let mut fetch_opts = {
                let mut opts = self.fetch_options.clone();
                if let Some(revision) = &self.revision {
                    opts.extra_refspecs.clear();
                    opts.extra_refspecs.push(revision.clone());
                } else {
                    if !opts.extra_refspecs.contains(&head_refspec) {
                        opts.extra_refspecs.push(head_refspec.clone());
                    }
                    if let Some(ref_name) = &self.ref_name {
                        opts.extra_refspecs.push(
                            gix_refspec::parse(ref_name.as_ref().as_bstr(), gix_refspec::parse::Operation::Fetch)
                                .expect("partial names are valid refspecs")
                                .to_owned(),
                        );
                    }
                }
                opts
            };
            match connection.prepare_fetch(&repo, &mut progress, fetch_opts.clone()).await {
                Ok(prepare) => prepare,
                Err(remote::fetch::prepare::Error::RefMap(remote::ref_map::Error::InitRefMap(
                    gix_protocol::fetch::refmap::init::Error::MappingValidation(err),
                ))) if err.issues.len() == 1
                    && fetch_opts.extra_refspecs.contains(&head_refspec)
                    && matches!(
                        err.issues.first(),
                        Some(gix_refspec::match_group::validate::Issue::Conflict {
                            destination_full_ref_name,
                            ..
                        }) if *destination_full_ref_name == head_local_tracking_branch
                    ) =>
                {
                    let head_refspec_idx = fetch_opts
                        .extra_refspecs
                        .iter()
                        .enumerate()
                        .find_map(|(idx, spec)| (*spec == head_refspec).then_some(idx))
                        .expect("it's contained");
                    // On the very special occasion that we fail as there is a remote `refs/heads/HEAD` reference that clashes
                    // with our implicit refspec, retry without it. Maybe this tells us that we shouldn't have that implicit
                    // refspec, as git can do this without connecting twice.
                    let connection = remote.connect(remote::Direction::Fetch).await?;
                    let connection = connection.into_detached();
                    fetch_opts.extra_refspecs.remove(head_refspec_idx);
                    connection.prepare_fetch(&repo, &mut progress, fetch_opts).await?
                }
                Err(err) => return Err(err.into()),
            }
        };
        drop(remote);
        repo.reread_values_and_clear_caches_replacing_config(resolved_config.into())?;

        // Assure problems with custom branch names fail early, not after getting the pack or during negotiation.
        if let Some(ref_name) = &self.ref_name {
            util::find_custom_refname(pending_pack.ref_map(), ref_name)?;
        }
        if let Some(revision) = &self.revision {
            util::find_revision(pending_pack.ref_map(), revision)?;
        }
        // On an object-format mismatch: adopt the remote's format before receiving the pack.
        // Only reachable with sha256, otherwise `gix_hash::Kind` has a single variant, so
        // local and remote hashes can never differ.
        #[cfg(feature = "sha256")]
        {
            let remote_object_hash = pending_pack.ref_map().object_hash;
            if remote_object_hash != repo.object_hash() {
                let mut in_memory_config = Vec::new();
                repo.config.resolved.write_to_filter(&mut in_memory_config, |section| {
                    section.meta().source == gix_config::Source::Api
                })?;
                // Reopen the still-empty repo with the remote's format. Retain the reopened
                // handle immediately: the on-disk handoff has committed, so every later
                // error must retry or persist through the matching reference store.
                repo = match util::reinitialize_with_object_hash(&repo, remote_object_hash) {
                    Ok(repo) => repo,
                    Err(error @ Error::ReftableHandoffRollback { .. }) => {
                        self.retry_blocked_by_handoff_rollback = true;
                        return Err(error);
                    }
                    Err(error) => return Err(error),
                };
                *self.repo.as_mut().expect("the repository is retained until success") = repo.clone();
                let mut resolved_config = repo.config.resolved.as_ref().clone();
                // The reopened repo has the rewritten local config. Reapply the
                // old API-only layer and then the remote config written during
                // clone setup, matching the normal in-memory config order.
                // TODO: make this much easier - we go from parsed-to-buffer-to-parsed.
                //       Maybe make API changes available as overlay, just as utility over
                //       Api sections.
                resolved_config.append(gix_config::File::from_bytes_owned(
                    &mut in_memory_config,
                    gix_config::file::Metadata::api(),
                    Default::default(),
                )?)?;
                repo.config
                    .reread_values_and_clear_caches_replacing_config(resolved_config.into())?;
                *self.repo.as_mut().expect("the repository is retained until success") = repo.clone();
            }
        }
        let reflog_message = {
            let mut b = self.url.to_bstring();
            b.insert_str(0, "clone: from ");
            b
        };
        let outcome = pending_pack
            .with_write_packed_refs_only(true)
            .with_reflog_message(RefLogMessage::Override {
                message: reflog_message.clone(),
            })
            .with_shallow(self.shallow.clone())
            .receive(&repo, &mut progress, should_interrupt)
            .await?;

        util::update_head(
            &mut repo,
            &outcome.ref_map,
            reflog_message.as_ref(),
            remote_name.as_ref(),
            self.ref_name.as_ref(),
            self.revision.as_ref(),
        )?;

        drop(self.repo.take().expect("still present"));
        Ok((repo, outcome))
    }

    /// Similar to [`fetch_only()`][Self::fetch_only()`], but passes ownership to a utility type to configure a checkout operation.
    #[cfg(all(feature = "worktree-mutation", feature = "blocking-network-client"))]
    pub fn fetch_then_checkout<P>(
        &mut self,
        progress: P,
        should_interrupt: &std::sync::atomic::AtomicBool,
    ) -> Result<(crate::clone::PrepareCheckout, crate::remote::fetch::Outcome), Error>
    where
        P: crate::NestedProgress,
        P::SubProgress: 'static,
    {
        let (repo, fetch_outcome) = self.fetch_only(progress, should_interrupt)?;
        Ok((
            crate::clone::PrepareCheckout {
                repo: repo.into(),
                ref_name: self.ref_name.clone(),
                remove_worktree_on_drop: self.remove_worktree_on_drop,
            },
            fetch_outcome,
        ))
    }
}

mod util;

#[cfg(all(test, feature = "sha256", feature = "blocking-network-client"))]
mod tests {
    use super::util;
    use gix_testtools::tempfile;

    #[test]
    fn incomplete_handoff_rollback_blocks_retry_before_using_the_retained_repository() -> gix_testtools::Result {
        let temp = tempfile::tempdir()?;
        let remote = gix_testtools::scripted_fixture_read_only("make_sha256_remote.sh")?.join("remote");
        let mut prepare = crate::clone::PrepareFetch::new(
            remote,
            temp.path().join("clone"),
            crate::create::Kind::Bare,
            crate::create::Options {
                reference_storage: crate::create::ReferenceStorage::Reftable,
                ..Default::default()
            },
            crate::open::Options::isolated(),
        )?;
        let _injection = util::inject_incomplete_handoff_rollback_once();

        let handoff_err = prepare
            .fetch_only(crate::progress::Discard, &std::sync::atomic::AtomicBool::default())
            .expect_err("the injected handoff and rollback failure must be reported");
        assert!(
            matches!(handoff_err, super::Error::ReftableHandoffRollback { .. }),
            "the first call reports that handoff and rollback both failed: {handoff_err}"
        );

        let retry_err = prepare
            .fetch_only(crate::progress::Discard, &std::sync::atomic::AtomicBool::default())
            .expect_err("an incomplete rollback makes the builder unsafe to retry");
        assert!(
            matches!(retry_err, super::Error::ReftableHandoffRetryBlocked),
            "the retry is rejected before the stale retained repository can be used: {retry_err}"
        );
        Ok(())
    }
}
