use crate::{Result, bstr::BStr, clone::PrepareFetch};
use gix_error::ResultExt;
use gix_ref::Category;

use crate::config::tree::Key;

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
    /// Local repositories are read directly without starting Git when using blocking I/O and
    /// either the default shallow policy or an absolute depth. A custom connection callback,
    /// handshake parameters, or another shallow policy selects the protocol transport instead.
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
    ) -> Result<(crate::Repository, crate::remote::fetch::Outcome)>
    where
        P: crate::NestedProgress,
        P::SubProgress: 'static,
    {
        use crate::{bstr::ByteVec, remote};

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
        let branch_prefix = if repo.is_bare() {
            "refs/heads/".to_owned()
        } else {
            format!("refs/remotes/{remote_name}/")
        };

        // For shallow clones without custom configuration, we'll use a single-branch refspec
        // to match git's behavior (matching git's single-branch behavior for shallow clones).
        let use_single_branch_for_shallow = self.shallow != remote::fetch::Shallow::NoChange
            && remote.fetch_specs.is_empty()
            && self.fetch_options.extra_refspecs.is_empty()
            && self.revision.is_none()
            && self.configure_remote.is_none();

        let target_ref: Option<gix_ref::FullName> = if use_single_branch_for_shallow {
            // Determine target branch from user-specified ref_name or default branch
            if let Some(ref_name) = &self.ref_name {
                let prev_tags = std::mem::replace(&mut remote.fetch_tags, remote::fetch::Tags::None);
                let refmap = prepare::ref_map(
                    &remote,
                    &mut self.configure_connection,
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
                Some(gix_ref::FullName::try_from(full_ref_name).or_error()?)
            } else {
                // For shallow clones without a specified ref, we need to determine the ref to clone.
                // Advertise branches too so a detached HEAD can be matched to a branch.
                let prev_tags = std::mem::replace(&mut remote.fetch_tags, remote::fetch::Tags::None);
                let refmap = prepare::ref_map(
                    &remote,
                    &mut self.configure_connection,
                    &mut progress,
                    remote::ref_map::Options {
                        extra_refspecs: vec![
                            gix_refspec::parse(
                                "refs/heads/*:refs/heads/*".into(),
                                gix_refspec::parse::Operation::Fetch,
                            )
                            .expect("valid")
                            .to_owned(),
                            gix_refspec::parse("HEAD".into(), gix_refspec::parse::Operation::Fetch)
                                .expect("valid")
                                .to_owned(),
                        ],
                        ..Default::default()
                    },
                )
                .await?;

                let target = util::guess_remote_head(&repo, &refmap)?
                    .and_then(|mapping| mapping.remote.as_name())
                    .map(gix_ref::FullName::try_from)
                    .transpose()
                    .or_error()?;
                remote.fetch_tags = prev_tags;
                target
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
                        format!("{branch_prefix}{short_name}")
                    }
                    _ => target_ref.to_string(),
                };
                remote = remote
                    .with_refspecs(
                        Some(format!("+{target_ref}:{destination}").as_str()),
                        remote::Direction::Fetch,
                    )
                    .expect("valid refspec");
            } else if !use_single_branch_for_shallow {
                // Wildcard refspec for non-shallow clones or when target couldn't be determined
                remote = remote
                    .with_refspecs(
                        Some(format!("+refs/heads/*:{branch_prefix}*").as_str()),
                        remote::Direction::Fetch,
                    )
                    .expect("valid static spec");
            }
        }

        // Bare clone defaults are used for the initial fetch, but are not remote configuration.
        let bare_fetch_specs = repo.is_bare().then(|| std::mem::take(&mut remote.fetch_specs));
        let mut clone_fetch_tags = None;
        if let Some(f) = self.configure_remote.as_mut() {
            remote =
                f(remote).or_raise(|| gix_error::message("Custom configuration of remote to clone from failed"))?;
        } else if self.revision.is_none() {
            clone_fetch_tags = Some(if use_single_branch_for_shallow {
                remote::fetch::Tags::Included
            } else {
                remote::fetch::Tags::All
            });
        }
        if self.revision.is_some() {
            remote
                .replace_refspecs(std::iter::empty::<&BStr>(), remote::Direction::Fetch)
                .expect("an empty refspec list is always valid");
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
        if remote.fetch_specs.is_empty()
            && self.revision.is_none()
            && let Some(specs) = bare_fetch_specs
        {
            remote.fetch_specs = specs;
        }
        if self.revision.is_some() {
            remote = remote.with_fetch_tags(remote::fetch::Tags::None);
        }
        if let Some(fetch_tags) = clone_fetch_tags {
            remote = remote.with_fetch_tags(fetch_tags);
        }

        // Fetch HEAD's object without writing a tracking ref; clone creates its symref from the selected peer.
        let head_refspec = gix_refspec::parse("HEAD".into(), gix_refspec::parse::Operation::Fetch)
            .expect("valid")
            .to_owned();
        let fetch_opts = {
            let mut opts = self.fetch_options.clone();
            if let Some(revision) = &self.revision {
                opts.extra_refspecs.clear();
                opts.extra_refspecs.push(revision.clone());
            } else {
                if !(opts.extra_refspecs.contains(&head_refspec)
                    || use_single_branch_for_shallow && self.ref_name.is_some())
                {
                    opts.extra_refspecs.push(head_refspec.clone());
                }
                if let Some(ref_name) = self.ref_name.as_ref().filter(|_| !use_single_branch_for_shallow) {
                    opts.extra_refspecs.push(
                        gix_refspec::parse(ref_name.as_ref().as_bstr(), gix_refspec::parse::Operation::Fetch)
                            .expect("partial names are valid refspecs")
                            .to_owned(),
                    );
                }
            }
            opts
        };
        let pending_pack = prepare::Prepare::new(
            &remote,
            &mut self.configure_connection,
            fetch_opts,
            &self.shallow,
            &mut progress,
        )
        .await?;
        drop(remote);

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
                repo.config
                    .resolved
                    .write_to_filter(&mut in_memory_config, |section| {
                        section.meta().source == gix_config::Source::Api
                    })
                    .or_error()?;
                // Reopen the still-empty repo with the remote's format; on error the original is kept for a retry.
                repo = util::reinitialize_with_object_hash(&repo, remote_object_hash)?;
                let mut resolved_config = repo.config.resolved.as_ref().clone();
                // The reopened repo has the rewritten local config. Reapply the
                // old API-only layer and then the remote config written during
                // clone setup, matching the normal in-memory config order.
                // TODO: make this much easier - we go from parsed-to-buffer-to-parsed.
                //       Maybe make API changes available as overlay, just as utility over
                //       Api sections.
                resolved_config
                    .append(gix_config::File::from_bytes_owned(
                        &mut in_memory_config,
                        gix_config::file::Metadata::api(),
                        Default::default(),
                    )?)
                    .or_raise(|| {
                        gix_error::message(
                            "Failed to transfer in-memory configuration after adopting the remote's object format",
                        )
                    })?;
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
        // Git clone writes its initial packed refs without creating reflogs.
        let mut receiving_repo = repo.clone();
        receiving_repo.refs.write_reflog = gix_ref::store::WriteReflog::Disable;
        let outcome = pending_pack
            .receive(
                &receiving_repo,
                &mut progress,
                reflog_message.clone(),
                self.shallow.clone(),
                should_interrupt,
            )
            .await?;

        // Before finalisation, the current repo handle still needs to
        // learn about the remote config written after it was opened.
        if let Some(config) = config {
            util::append_config_to_repo_config(&mut repo, config)?;
        }
        util::update_remote_head(&repo, &outcome.ref_map, reflog_message.as_ref(), remote_name.as_ref())?;
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
    ) -> Result<(crate::clone::PrepareCheckout, crate::remote::fetch::Outcome)>
    where
        P: crate::NestedProgress,
        P::SubProgress: 'static,
    {
        let (repo, fetch_outcome) = self.fetch_only(progress, should_interrupt)?;
        Ok((
            crate::clone::PrepareCheckout {
                repo: repo.into(),
                remove_worktree_on_drop: self.remove_worktree_on_drop,
                command_environment: Vec::new(),
            },
            fetch_outcome,
        ))
    }
}

mod prepare;
mod util;
