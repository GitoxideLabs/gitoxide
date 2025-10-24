use crate::{
    bstr::{BString, ByteSlice},
    clone::PrepareFetch,
};

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

        let repo = self
            .repo
            .as_mut()
            .expect("user error: multiple calls are allowed only until it succeeds");

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
            && self.configure_remote.is_none()
            && remote.fetch_specs.is_empty();

        let target_ref = if use_single_branch_for_shallow {
            // Determine target branch from user-specified ref_name or default branch
            if let Some(ref_name) = &self.ref_name {
                // User specified a branch, use that
                Some(format!("refs/heads/{}", ref_name.as_ref().as_bstr()))
            } else {
                // For shallow clones without a specified ref, we need to determine the default branch.
                // We'll connect to get HEAD information. For Protocol V2, we need to explicitly list refs.
                let mut connection = remote.connect(remote::Direction::Fetch).await?;

                // Perform handshake and try to get HEAD from it (works for Protocol V1)
                let _ = connection.ref_map_by_ref(&mut progress, Default::default()).await?;

                let target = if let Some(handshake) = &connection.handshake {
                    // Protocol V1: refs are in handshake
                    handshake.refs.as_ref().and_then(|refs| {
                        refs.iter().find_map(|r| match r {
                            gix_protocol::handshake::Ref::Symbolic {
                                full_ref_name, target, ..
                            } if full_ref_name == "HEAD" => Some(target.to_string()),
                            _ => None,
                        })
                    })
                } else {
                    None
                };

                // For Protocol V2 or if we couldn't determine HEAD, use the configured default branch
                let fallback_branch = target
                    .or_else(|| {
                        repo.config
                            .resolved
                            .string(crate::config::tree::Init::DEFAULT_BRANCH)
                            .and_then(|name| name.to_str().ok().map(|s| format!("refs/heads/{}", s)))
                    })
                    .unwrap_or_else(|| "refs/heads/main".to_string());

                // Drop the connection explicitly to release the borrow on remote
                drop(connection);

                Some(fallback_branch)
            }
        } else {
            None
        };

        // Set up refspec based on whether we're doing a single-branch shallow clone
        if remote.fetch_specs.is_empty() {
            if let Some(target_ref) = &target_ref {
                // Single-branch refspec for shallow clones
                let short_name = target_ref.strip_prefix("refs/heads/").unwrap_or(target_ref.as_str());
                remote = remote
                    .with_refspecs(
                        Some(format!("+{target_ref}:refs/remotes/{remote_name}/{short_name}").as_str()),
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

        let config = util::write_remote_to_local_config_file(&mut remote, remote_name.clone())?;

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
        let pending_pack: remote::fetch::Prepare<'_, '_, _> = {
            // For shallow clones, we already connected once, so we need to connect again
            let mut connection = remote.connect(remote::Direction::Fetch).await?;
            if let Some(f) = self.configure_connection.as_mut() {
                f(&mut connection).map_err(Error::RemoteConnection)?;
            }
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
            match connection.prepare_fetch(&mut progress, fetch_opts.clone()).await {
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
                    fetch_opts.extra_refspecs.remove(head_refspec_idx);
                    connection.prepare_fetch(&mut progress, fetch_opts).await?
                }
                Err(err) => return Err(err.into()),
            }
        };

        // Assure problems with custom branch names fail early, not after getting the pack or during negotiation.
        if let Some(ref_name) = &self.ref_name {
            util::find_custom_refname(pending_pack.ref_map(), ref_name)?;
        }
        if pending_pack.ref_map().object_hash != repo.object_hash() {
            unimplemented!("configure repository to expect a different object hash as advertised by the server")
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
            .receive(&mut progress, should_interrupt)
            .await?;

        util::append_config_to_repo_config(repo, config);
        util::update_head(
            repo,
            &outcome.ref_map,
            reflog_message.as_ref(),
            remote_name.as_ref(),
            self.ref_name.as_ref(),
        )?;

        Ok((self.repo.take().expect("still present"), outcome))
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
            },
            fetch_outcome,
        ))
    }
}

mod util;
