use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use gix_features::progress::DynNestedProgress;

use crate::fetch::{
    negotiate, Arguments, Context, Error, Negotiate, NegotiateOutcome, Options, Outcome, ProgressId, Shallow, Tags,
};
#[cfg(feature = "async-client")]
use crate::transport::client::async_io::{ExtendedBufRead, HandleProgress, Transport};
#[cfg(feature = "blocking-client")]
use crate::transport::client::blocking_io::{ExtendedBufRead, HandleProgress, Transport};

/// Perform one fetch operation, relying on a `transport`.
/// `negotiate` is used to run the negotiation of objects that should be contained in the pack, *if* one is to be received.
/// `progress` and `should_interrupt` is passed to all potentially long-running parts of the operation.
///
/// `consume_pack(pack_read, progress, interrupt) -> bool` is always called to consume all bytes that are sent by the server, returning `true` if we should assure the pack is read to the end,
/// or `false` to do nothing. Dropping the reader without reading to EOF (i.e. returning `false`) is an offense to the server, and
/// `transport` won't be in the correct state to perform additional operations, or indicate the end of operation.
/// Note that the passed reader blocking as the pack-writing is blocking as well.
///
/// The `Context` and `Options` further define parts of this `fetch` operation.
///
/// As opposed to a full `git fetch`, this operation does *not*…
///
/// * …update local refs
/// * …end the interaction after the fetch
///
/// **Note that the interaction will never be ended**, even on error or failure, leaving it up to the caller to do that, maybe
/// with the help of [`SendFlushOnDrop`](crate::SendFlushOnDrop) which can wrap `transport`.
/// Generally, the `transport` is left in a state that allows for more commands to be run.
///
/// Return `Ok(None)` if there was nothing to do because all remote refs are at the same state as they are locally,
/// or there was nothing wanted, or `Ok(Some(outcome))` to inform about all the changes that were made.
#[maybe_async::maybe_async]
pub async fn fetch<P, T, E>(
    negotiate: &mut impl Negotiate,
    consume_pack: impl FnOnce(&mut dyn std::io::BufRead, &mut dyn DynNestedProgress, &AtomicBool) -> Result<bool, E>,
    mut progress: P,
    should_interrupt: &AtomicBool,
    Context {
        handshake,
        transport,
        user_agent,
        trace_packetlines,
    }: Context<'_, T>,
    Options {
        shallow_file,
        shallow,
        tags,
        reject_shallow_remote,
    }: Options<'_>,
) -> Result<Option<Outcome>, Error>
where
    P: gix_features::progress::NestedProgress,
    P::SubProgress: 'static,
    T: Transport,
    E: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
{
    let _span = gix_trace::coarse!("gix_protocol::fetch()");
    let v1_shallow_updates = handshake.v1_shallow_updates.take();
    let protocol_version = handshake.server_protocol_version;

    let fetch = crate::Command::Fetch;
    let fetch_features = {
        let mut f = fetch.default_features(protocol_version, &handshake.capabilities);
        f.push(user_agent);
        f
    };

    crate::fetch::Response::check_required_features(protocol_version, &fetch_features)?;
    let sideband_all = fetch_features.iter().any(|(n, _)| *n == "sideband-all");
    let mut arguments = Arguments::new(protocol_version, fetch_features, trace_packetlines);
    if matches!(tags, Tags::Included) {
        if !arguments.can_use_include_tag() {
            return Err(Error::MissingServerFeature {
                    feature: "include-tag",
                    description:
                    // NOTE: if this is an issue, we could probably do what's proposed here.
                    "To make this work we would have to implement another pass to fetch attached tags separately",
                });
        }
        arguments.use_include_tag();
    }
    let (shallow_commits, mut shallow_lock) = add_shallow_args(&mut arguments, shallow, &shallow_file)?;

    let negotiate_span = gix_trace::detail!(
        "negotiate",
        protocol_version = handshake.server_protocol_version as usize
    );
    let action = negotiate.mark_complete_and_common_ref()?;
    let mut previous_response = None::<crate::fetch::Response>;
    match &action {
        negotiate::Action::NoChange | negotiate::Action::SkipToRefUpdate => Ok(None),
        negotiate::Action::MustNegotiate {
            remote_ref_target_known,
        } => {
            if !negotiate.add_wants(&mut arguments, remote_ref_target_known) {
                return Ok(None);
            }
            let mut rounds = Vec::new();
            let is_stateless = arguments.is_stateless(!transport.connection_persists_across_multiple_requests());
            let mut state = negotiate::one_round::State::new(is_stateless);
            let mut reader = 'negotiation: loop {
                let _round = gix_trace::detail!("negotiate round", round = rounds.len() + 1);
                progress.step();
                progress.set_name(format!("negotiate (round {})", rounds.len() + 1));
                if should_interrupt.load(Ordering::Relaxed) {
                    return Err(Error::Negotiate(negotiate::Error::NegotiationFailed {
                        rounds: rounds.len(),
                    }));
                }

                let is_done = match negotiate.one_round(&mut state, &mut arguments, previous_response.as_ref()) {
                    Ok((round, is_done)) => {
                        rounds.push(round);
                        is_done
                    }
                    Err(err) => {
                        return Err(err.into());
                    }
                };
                let mut reader = arguments.send(transport, is_done).await?;
                if sideband_all {
                    setup_remote_progress(&mut progress, &mut reader, should_interrupt);
                }
                let response =
                    crate::fetch::Response::from_line_reader(protocol_version, &mut reader, is_done, !is_done).await?;
                let has_pack = response.has_pack();
                previous_response = Some(response);
                if has_pack {
                    progress.step();
                    progress.set_name("receiving pack".into());
                    if !sideband_all {
                        setup_remote_progress(&mut progress, &mut reader, should_interrupt);
                    }
                    break 'negotiation reader;
                }
            };
            drop(negotiate_span);

            let mut previous_response = previous_response.expect("knowledge of a pack means a response was received");
            previous_response.append_v1_shallow_updates(v1_shallow_updates);
            if !previous_response.shallow_updates().is_empty() && shallow_lock.is_none() {
                if reject_shallow_remote {
                    return Err(Error::RejectShallowRemote);
                }
                shallow_lock = acquire_shallow_lock(&shallow_file).map(Some)?;
            }

            #[cfg(feature = "async-client")]
            let mut rd = crate::futures_lite::io::BlockOn::new(reader);
            #[cfg(not(feature = "async-client"))]
            let mut rd = reader;
            let may_read_to_end =
                consume_pack(&mut rd, &mut progress, should_interrupt).map_err(|err| Error::ConsumePack(err.into()))?;
            #[cfg(feature = "async-client")]
            {
                reader = rd.into_inner();
            }
            #[cfg(not(feature = "async-client"))]
            {
                reader = rd;
            }

            if may_read_to_end {
                // Assure the final flush packet is consumed.
                let has_read_to_end = reader.stopped_at().is_some();
                #[cfg(feature = "async-client")]
                {
                    if !has_read_to_end {
                        futures_lite::io::copy(&mut reader, &mut futures_lite::io::sink())
                            .await
                            .map_err(Error::ReadRemainingBytes)?;
                    }
                }
                #[cfg(not(feature = "async-client"))]
                {
                    if !has_read_to_end {
                        std::io::copy(&mut reader, &mut std::io::sink()).map_err(Error::ReadRemainingBytes)?;
                    }
                }
            }
            drop(reader);

            if let Some(shallow_lock) = shallow_lock {
                if !previous_response.shallow_updates().is_empty() {
                    gix_shallow::write(shallow_lock, shallow_commits, previous_response.shallow_updates())?;
                }
            }
            Ok(Some(Outcome {
                last_response: previous_response,
                negotiate: NegotiateOutcome { action, rounds },
            }))
        }
    }
}

fn acquire_shallow_lock(shallow_file: &Path) -> Result<gix_lock::File, Error> {
    gix_lock::File::acquire_to_update_resource(shallow_file, gix_lock::acquire::Fail::Immediately, None)
        .map_err(Into::into)
}

fn add_shallow_args(
    args: &mut Arguments,
    shallow: &Shallow,
    shallow_file: &std::path::Path,
) -> Result<(Option<Vec<gix_hash::ObjectId>>, Option<gix_lock::File>), Error> {
    let expect_change = *shallow != Shallow::NoChange;
    let shallow_lock = expect_change.then(|| acquire_shallow_lock(shallow_file)).transpose()?;

    let shallow_commits = gix_shallow::read(shallow_file)?;
    if (shallow_commits.is_some() || expect_change) && !args.can_use_shallow() {
        // NOTE: if this is an issue, we can always unshallow the repo ourselves.
        return Err(Error::MissingServerFeature {
            feature: "shallow",
            description: "shallow clones need server support to remain shallow, otherwise bigger than expected packs are sent effectively unshallowing the repository",
        });
    }
    if let Some(shallow_commits) = &shallow_commits {
        for commit in shallow_commits.iter() {
            args.shallow(commit);
        }
    }
    match shallow {
        Shallow::NoChange => {}
        Shallow::DepthAtRemote(commits) => args.deepen(commits.get() as usize),
        Shallow::Deepen(commits) => {
            args.deepen(*commits as usize);
            args.deepen_relative();
        }
        Shallow::Since { cutoff } => {
            args.deepen_since(cutoff.seconds);
        }
        Shallow::Exclude {
            remote_refs,
            since_cutoff,
        } => {
            if let Some(cutoff) = since_cutoff {
                args.deepen_since(cutoff.seconds);
            }
            for ref_ in remote_refs {
                args.deepen_not(ref_.as_ref().as_bstr());
            }
        }
    }
    Ok((shallow_commits, shallow_lock))
}

fn setup_remote_progress<'a>(
    progress: &mut dyn gix_features::progress::DynNestedProgress,
    reader: &mut Box<dyn ExtendedBufRead<'a> + Unpin + 'a>,
    should_interrupt: &'a AtomicBool,
) {
    reader.set_progress_handler(Some(Box::new({
        let mut remote_progress = progress.add_child_with_id("remote".to_string(), ProgressId::RemoteProgress.into());
        move |is_err: bool, data: &[u8]| {
            crate::RemoteProgress::translate_to_progress(is_err, data, &mut remote_progress);
            if should_interrupt.load(Ordering::Relaxed) {
                std::ops::ControlFlow::Break(())
            } else {
                std::ops::ControlFlow::Continue(())
            }
        }
    }) as HandleProgress<'a>));
}
