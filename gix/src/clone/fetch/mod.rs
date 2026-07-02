use crate::{
    bstr::{BString, ByteSlice},
    clone::PrepareFetch,
};
use gix_ref::Category;

/// The error returned by [`PrepareFetch::fetch_only()`].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
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
    #[error(transparent)]
    CommitterOrFallback(#[from] crate::config::time::Error),
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
}

/// Modification
impl PrepareFetch {
    /// Fetch a pack and update local branches according to refspecs, providing `progress` and checking `should_interrupt` to stop
    /// the operation.
    /// On success, the persisted repository is returned, and this method must not be called again to avoid a **panic**.
    /// On error, the method may be called again to retry as often as needed.
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
    #[gix_protocol::maybe_async::maybe_async]
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
                .unwrap_or_else(|| "origin".into()),
        };

        let mut remote = repo.remote_at(self.url.clone())?;

        // For shallow clones without custom configuration, we'll use a single-branch refspec
        // to match git's behavior (matching git's single-branch behavior for shallow clones).
        let use_single_branch_for_shallow = self.shallow != remote::fetch::Shallow::NoChange
            && remote.fetch_specs.is_empty()
            && self.fetch_options.extra_refspecs.is_empty();

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
        if remote.fetch_specs.is_empty() {
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
        } else {
            clone_fetch_tags = remote::fetch::Tags::All.into();
        }

        // The remote section just written to `.git/config`, kept around so we can
        // mirror it into the repository's in-memory config once we know which
        // repo handle survives.
        let config = Some(util::append_remote_to_local_config_file(
            &mut remote,
            remote_name.clone(),
        )?);
        #[cfg(feature = "sha256")]
        let mut config = config;

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

        // Assure problems with custom branch names fail early, not after getting the pack or during negotiation.
        if let Some(ref_name) = &self.ref_name {
            util::find_custom_refname(pending_pack.ref_map(), ref_name)?;
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
                // Reopen the still-empty repo with the remote's format; on error the original is kept for a retry.
                repo = util::reinitialize_with_object_hash(&repo, remote_object_hash)?;
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
                )?);
                repo.config
                    .reread_values_and_clear_caches_replacing_config(resolved_config.into())?;
                config = None;
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

        // Before finalisation, the current repo handle still needs to
        // learn about the remote config written after it was opened.
        if let Some(config) = config {
            util::append_config_to_repo_config(&mut repo, config);
        }
        util::update_head(
            &mut repo,
            &outcome.ref_map,
            reflog_message.as_ref(),
            remote_name.as_ref(),
            self.ref_name.as_ref(),
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
