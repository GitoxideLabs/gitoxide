use std::{ops::DerefMut, path::PathBuf, sync::atomic::AtomicBool};

use gix_odb::store::RefreshMode;
use gix_protocol::fetch::{negotiate, refmap, Arguments, Shallow};
#[cfg(feature = "async-network-client")]
use gix_transport::client::async_io::Transport;
#[cfg(feature = "blocking-network-client")]
use gix_transport::client::blocking_io::Transport;

use crate::{
    config::{
        cache::util::ApplyLeniency,
        tree::{Clone, Fetch},
    },
    remote,
    remote::{
        connection::fetch::config,
        fetch,
        fetch::{negotiate::Algorithm, outcome, refs, Error, Outcome, Prepare, RefLogMessage, Status},
    },
};

impl<T> Prepare<'_, '_, T>
where
    T: Transport,
{
    /// Receive the pack and perform the operation as configured by git via `git-config` or overridden by various builder methods.
    /// Return `Ok(Outcome)` with an [`Outcome::status`] indicating if a change was made or not.
    ///
    /// Note that when in dry-run mode, we don't read the pack the server prepared, which leads the server to be hung up on unexpectedly.
    ///
    /// ### Negotiation
    ///
    /// "fetch.negotiationAlgorithm" describes algorithms `git` uses currently, with the default being `consecutive` and `skipping` being
    /// experimented with.
    ///
    /// ### Pack `.keep` files
    ///
    /// That packs that are freshly written to the object database are vulnerable to garbage collection for the brief time that
    /// it takes between them being placed and the respective references to be written to disk which binds their objects to the
    /// commit graph, making them reachable.
    ///
    /// To circumvent this issue, a `.keep` file is created before any pack related file (i.e. `.pack` or `.idx`) is written,
    /// which indicates the garbage collector (like `git maintenance`, `git gc`) to leave the corresponding pack file alone.
    ///
    /// If there were any ref updates or the received pack was empty, the `.keep` file will be deleted automatically leaving
    /// in its place at `write_pack_bundle.keep_path` a `None`.
    /// However, if no ref-update happened the path will still be present in `write_pack_bundle.keep_path` and is expected to be handled by the caller.
    /// A known application for this behaviour is in `remote-helper` implementations which should send this path via `lock <path>` to stdout
    /// to inform git about the file that it will remove once it updated the refs accordingly.
    ///
    /// ### Deviation
    ///
    /// When **updating refs**, the `git-fetch` docs state the following:
    ///
    /// > Unlike when pushing with git-push, any updates outside of refs/{tags,heads}/* will be accepted without + in the refspec (or --force),
    /// whether that’s swapping e.g. a tree object for a blob, or a commit for another commit that’s doesn’t have the previous commit
    /// as an ancestor etc.
    ///
    /// We explicitly don't special case those refs and expect the caller to take control. Note that by its nature,
    /// force only applies to refs pointing to commits and if they don't, they will be updated either way in our
    /// implementation as well.
    ///
    /// ### Async Mode Shortcoming
    ///
    /// Currently, the entire process of resolving a pack is blocking the executor. This can be fixed using the `blocking` crate, but it
    /// didn't seem worth the tradeoff of having more complex code.
    ///
    /// ### Configuration
    ///
    /// - `gitoxide.userAgent` is read to obtain the application user agent for git servers and for HTTP servers as well.
    ///
    #[gix_protocol::maybe_async::maybe_async]
    pub async fn receive<P>(mut self, progress: P, should_interrupt: &AtomicBool) -> Result<Outcome, Error>
    where
        P: gix_features::progress::NestedProgress,
        P::SubProgress: 'static,
    {
        let ref_map = &self.ref_map;
        if ref_map.mappings.is_empty() && !ref_map.remote_refs.is_empty() && self.additional_wants.is_empty() {
            let mut specs = ref_map.refspecs.clone();
            specs.extend(ref_map.extra_refspecs.clone());
            return Err(Error::NoMapping {
                refspecs: specs,
                num_remote_refs: ref_map.remote_refs.len(),
            });
        }

        let mut con = self.con.take().expect("receive() can only be called once");
        let mut handshake = con.handshake.take().expect("receive() can only be called once");
        let repo = con.remote.repo;

        let expected_object_hash = repo.object_hash();
        if ref_map.object_hash != expected_object_hash {
            return Err(Error::IncompatibleObjectHash {
                local: expected_object_hash,
                remote: ref_map.object_hash,
            });
        }

        let filter_argument = self
            .filter
            .as_ref()
            .map(crate::remote::fetch::ObjectFilter::to_argument_string)
            .filter(|_| self.additional_wants.is_empty());

        let fetch_options = gix_protocol::fetch::Options {
            shallow_file: repo.shallow_file(),
            shallow: &self.shallow,
            tags: con.remote.fetch_tags,
            reject_shallow_remote: repo
                .config
                .resolved
                .boolean_filter("clone.rejectShallow", &mut repo.filter_config_section())
                .map(|val| Clone::REJECT_SHALLOW.enrich_error(val))
                .transpose()?
                .unwrap_or(false),
            filter: filter_argument.as_deref(),
        };
        let context = gix_protocol::fetch::Context {
            handshake: &mut handshake,
            transport: &mut con.transport.inner,
            user_agent: repo.config.user_agent_tuple(),
            trace_packetlines: con.trace,
        };

        let negotiator = repo
            .config
            .resolved
            .string(Fetch::NEGOTIATION_ALGORITHM)
            .map(|n| Fetch::NEGOTIATION_ALGORITHM.try_into_negotiation_algorithm(n))
            .transpose()
            .with_leniency(repo.config.lenient_config)?
            .unwrap_or(Algorithm::Consecutive)
            .into_negotiator();
        let graph_repo = {
            let mut r = repo.clone();
            // assure that checking for unknown server refs doesn't trigger ODB refreshes.
            r.objects.refresh = RefreshMode::Never;
            // we cache everything of importance in the graph and thus don't need an object cache.
            r.objects.unset_object_cache();
            r
        };
        let cache = graph_repo.commit_graph_if_enabled().ok().flatten();
        let mut graph = graph_repo.revision_graph(cache.as_ref());
        let alternates = repo.objects.store_ref().alternate_db_paths()?;
        let mut negotiate = Negotiate {
            objects: &graph_repo.objects,
            refs: &graph_repo.refs,
            graph: &mut graph,
            alternates,
            ref_map,
            shallow: &self.shallow,
            tags: con.remote.fetch_tags,
            negotiator,
            open_options: repo.options.clone(),
            additional_wants: &self.additional_wants,
        };

        let write_pack_options = gix_pack::bundle::write::Options {
            thread_limit: config::index_threads(repo)?,
            index_version: config::pack_index_version(repo)?,
            iteration_mode: gix_pack::data::input::Mode::Verify,
            object_hash: con.remote.repo.object_hash(),
        };
        let mut write_pack_bundle = None;

        let res = gix_protocol::fetch(
            &mut negotiate,
            |reader, progress, should_interrupt| -> Result<bool, gix_pack::bundle::write::Error> {
                let mut may_read_to_end = false;
                write_pack_bundle = if matches!(self.dry_run, fetch::DryRun::No) {
                    let res = gix_pack::Bundle::write_to_directory(
                        reader,
                        Some(&repo.objects.store_ref().path().join("pack")),
                        progress,
                        should_interrupt,
                        Some(Box::new({
                            let repo = repo.clone();
                            repo.objects
                        })),
                        write_pack_options,
                    )?;
                    may_read_to_end = true;
                    Some(res)
                } else {
                    None
                };
                Ok(may_read_to_end)
            },
            progress,
            should_interrupt,
            context,
            fetch_options,
        )
        .await?;
        let negotiate = res.map(|v| outcome::Negotiate {
            graph: graph.detach(),
            rounds: v.negotiate.rounds,
        });

        if matches!(handshake.server_protocol_version, gix_protocol::transport::Protocol::V2) {
            gix_protocol::indicate_end_of_interaction(&mut con.transport.inner, con.trace)
                .await
                .ok();
        }

        let update_refs = refs::update(
            repo,
            self.reflog_message
                .take()
                .unwrap_or_else(|| RefLogMessage::Prefixed { action: "fetch".into() }),
            &self.ref_map.mappings,
            con.remote.refspecs(remote::Direction::Fetch),
            &self.ref_map.extra_refspecs,
            con.remote.fetch_tags,
            self.dry_run,
            self.write_packed_refs,
        )?;

        if let Some(bundle) = write_pack_bundle.as_mut() {
            if !update_refs.edits.is_empty() || bundle.index.num_objects == 0 {
                if let Some(path) = bundle.keep_path.take() {
                    std::fs::remove_file(&path).map_err(|err| Error::RemovePackKeepFile { path, source: err })?;
                }
            }
        }

        let out = Outcome {
            handshake,
            ref_map: std::mem::take(&mut self.ref_map),
            status: match write_pack_bundle {
                Some(write_pack_bundle) => Status::Change {
                    write_pack_bundle,
                    update_refs,
                    negotiate: negotiate.expect("if we have a pack, we always negotiated it"),
                },
                None => Status::NoPackReceived {
                    dry_run: matches!(self.dry_run, fetch::DryRun::Yes),
                    negotiate,
                    update_refs,
                },
            },
        };
        Ok(out)
    }
}

struct Negotiate<'a, 'b, 'c> {
    objects: &'a crate::OdbHandle,
    refs: &'a gix_ref::file::Store,
    graph: &'a mut gix_negotiate::Graph<'b, 'c>,
    alternates: Vec<PathBuf>,
    ref_map: &'a gix_protocol::fetch::RefMap,
    shallow: &'a gix_protocol::fetch::Shallow,
    tags: gix_protocol::fetch::Tags,
    negotiator: Box<dyn gix_negotiate::Negotiator>,
    open_options: crate::open::Options,
    additional_wants: &'a [gix_hash::ObjectId],
}

impl gix_protocol::fetch::Negotiate for Negotiate<'_, '_, '_> {
    fn mark_complete_and_common_ref(&mut self) -> Result<negotiate::Action, negotiate::Error> {
        let action = negotiate::mark_complete_and_common_ref(
            &self.objects,
            self.refs,
            {
                let alternates = std::mem::take(&mut self.alternates);
                let open_options = self.open_options.clone();
                move || -> Result<_, std::convert::Infallible> {
                    Ok(alternates
                        .into_iter()
                        .filter_map(move |path| {
                            path.ancestors()
                                .nth(1)
                                .and_then(|git_dir| crate::open_opts(git_dir, open_options.clone()).ok())
                        })
                        .map(|repo| (repo.refs, repo.objects)))
                }
            },
            self.negotiator.deref_mut(),
            &mut *self.graph,
            self.ref_map,
            self.shallow,
            negotiate::make_refmapping_ignore_predicate(self.tags, self.ref_map),
        )?;

        if self.additional_wants.is_empty() {
            return Ok(action);
        }

        Ok(match action {
            negotiate::Action::MustNegotiate {
                remote_ref_target_known,
            } => negotiate::Action::MustNegotiate {
                remote_ref_target_known,
            },
            negotiate::Action::NoChange | negotiate::Action::SkipToRefUpdate => negotiate::Action::MustNegotiate {
                remote_ref_target_known: std::iter::repeat_n(true, self.ref_map.mappings.len()).collect(),
            },
        })
    }

    fn add_wants(&mut self, arguments: &mut Arguments, remote_ref_target_known: &[bool]) -> bool {
        if self.additional_wants.is_empty() {
            let mut has_wants = negotiate::add_wants(
                self.objects,
                arguments,
                self.ref_map,
                remote_ref_target_known,
                self.shallow,
                negotiate::make_refmapping_ignore_predicate(self.tags, self.ref_map),
            );

            for want in self.additional_wants {
                arguments.want(want);
                has_wants = true;
            }

            return has_wants;
        }

        let mut has_wants = false;
        let mapping_is_ignored = negotiate::make_refmapping_ignore_predicate(self.tags, self.ref_map);
        let is_shallow = !matches!(self.shallow, Shallow::NoChange);
        for (mapping, known) in self.ref_map.mappings.iter().zip(remote_ref_target_known) {
            if !is_shallow && *known {
                continue;
            }
            if mapping_is_ignored(mapping) {
                continue;
            }

            if !arguments.can_use_ref_in_want() || matches!(mapping.remote, refmap::Source::ObjectId(_)) {
                if let Some(id) = mapping.remote.as_id() {
                    arguments.want(id);
                    has_wants = true;
                }
            } else if let Some(name) = mapping.remote.as_name() {
                arguments.want_ref(name);
                has_wants = true;
            }
        }

        for want in self.additional_wants {
            arguments.want(want);
            has_wants = true;
        }

        has_wants
    }

    fn one_round(
        &mut self,
        state: &mut negotiate::one_round::State,
        arguments: &mut Arguments,
        previous_response: Option<&gix_protocol::fetch::Response>,
    ) -> Result<(negotiate::Round, bool), negotiate::Error> {
        if self.additional_wants.is_empty() {
            return negotiate::one_round(
                self.negotiator.deref_mut(),
                &mut *self.graph,
                state,
                arguments,
                previous_response,
            );
        }

        Ok((
            negotiate::Round {
                haves_sent: 0,
                in_vain: 0,
                haves_to_send: 0,
                previous_response_had_at_least_one_in_common: false,
            },
            true,
        ))
    }
}
