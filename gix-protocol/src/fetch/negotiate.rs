//! A modules with primitives to perform negotiation as part of a fetch operation.
//!
//! The functions provided are called in a certain order:
//!
//! 1. [`mark_complete_and_common_ref()`] - initialize the [`negotiator`](gix_negotiate::Negotiator) with all state known on the remote.
//! 2. [`add_wants()`] is called if the call at 1) returned [`Action::MustNegotiate`].
//! 3. [`one_round()`] is called for each negotiation round, providing information if the negotiation is done.
use std::borrow::Cow;

use gix_date::SecondsSinceUnixEpoch;
use gix_negotiate::Flags;
use gix_ref::file::ReferenceExt;

use crate::fetch::{refmap, RefMap, Shallow, Tags};

type Queue = gix_revwalk::PriorityQueue<SecondsSinceUnixEpoch, gix_hash::ObjectId>;

/// The error returned during [`one_round()`] or [`mark_complete_and_common_ref()`].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("We were unable to figure out what objects the server should send after {rounds} round(s)")]
    NegotiationFailed { rounds: usize },
    #[error(transparent)]
    LookupCommitInGraph(#[from] gix_revwalk::graph::get_or_insert_default::Error),
    #[error(transparent)]
    OpenPackedRefsBuffer(#[from] gix_ref::packed::buffer::open::Error),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    InitRefIter(#[from] gix_ref::file::iter::loose_then_packed::Error),
    #[error(transparent)]
    PeelToId(#[from] gix_ref::peel::to_id::Error),
    #[error(transparent)]
    AlternateRefsAndObjects(Box<dyn std::error::Error + Send + Sync + 'static>),
}

/// Determines what should be done after [preparing the commit-graph for negotiation](mark_complete_and_common_ref).
#[must_use]
#[derive(Debug, Clone)]
pub enum Action {
    /// None of the remote refs moved compared to our last recorded state (via tracking refs), so there is nothing to do at all,
    /// not even a ref update.
    NoChange,
    /// Don't negotiate, don't fetch the pack, skip right to updating the references.
    ///
    /// This happens if we already have all local objects even though the server seems to have changed.
    SkipToRefUpdate,
    /// We can't know for sure if fetching *is not* needed, so we go ahead and negotiate.
    MustNegotiate {
        /// Each `ref_map.mapping` has a slot here which is `true` if we have the object the remote ref points to, locally.
        remote_ref_target_known: Vec<bool>,
    },
}

/// Key information about each round in the pack-negotiation, as produced by [`one_round()`].
#[derive(Debug, Clone, Copy)]
pub struct Round {
    /// The amount of `HAVE` lines sent this round.
    ///
    /// Each `HAVE` is an object that we tell the server about which would acknowledge each one it has as well.
    pub haves_sent: usize,
    /// A total counter, over all previous rounds, indicating how many `HAVE`s we sent without seeing a single acknowledgement,
    /// i.e. the indication of a common object.
    ///
    /// This number maybe zero or be lower compared to the previous round if we have received at least one acknowledgement.
    pub in_vain: usize,
    /// The amount of haves we should send in this round.
    ///
    /// If the value is lower than `haves_sent` (the `HAVE` lines actually sent), the negotiation algorithm has run out of options
    /// which typically indicates the end of the negotiation phase.
    pub haves_to_send: usize,
    /// If `true`, the server reported, as response to our previous `HAVE`s, that at least one of them is in common by acknowledging it.
    ///
    /// This may also lead to the server responding with a pack.
    pub previous_response_had_at_least_one_in_common: bool,
}

/// This function is modeled after the similarly named one in the git codebase to mark known refs in a commit-graph.
///
/// It to do the following:
///
/// * figure out all advertised refs on the remote *that we already have* and keep track of the oldest one as cutoff date.
/// * mark all of our own refs as tips for a traversal.
/// * mark all their parents, recursively, up to (and including) the cutoff date up to which we have seen the servers commit that we have.
/// * pass all known-to-be-common-with-remote commits to the negotiator as common commits.
///
/// This is done so that we already find the most recent common commits, even if we are ahead, which is *potentially* better than
/// what we would get if we would rely on tracking refs alone, particularly if one wouldn't trust the tracking refs for some reason.
///
/// Note that git doesn't trust its own tracking refs as the server *might* have changed completely, for instance by force-pushing, so
/// marking our local tracking refs as known is something that's actually not proven to be correct so it's not done.
///
/// Additionally, it does what's done in `transport.c` and we check if a fetch is actually needed as at least one advertised ref changed.
///
/// Finally, we also mark tips in the `negotiator` in one go to avoid traversing all refs twice, since we naturally encounter all tips during
/// our own walk.
///
/// Return whether we should negotiate, along with a queue for later use.
///
/// # Parameters
///
/// * `objects`
///     - Access to the object database. *Note* that the `exists()` calls must not trigger a refresh of the ODB packs as plenty of them might fail, i.e. find on object.
/// * `refs`
///     - Access to the git references database.
/// * `alternates`
///     - A function that returns an iterator over `(refs, objects)` for each alternate repository, to assure all known objects are added also according to their tips.
/// * `negotiator`
///     - The implementation that performs the negotiation later, i.e. prepare wants and haves.
/// * `graph`
///     - The commit-graph for use by the `negotiator` - we populate it with tips to initialize the graph traversal.
/// * `ref_map`
///     - The references known on the remote, as previously obtained with [`crate::Handshake::fetch_or_extract_refmap()`].
/// * `shallow`
///     - How to deal with shallow repositories. It does affect how negotiations are performed.
/// * `mapping_is_ignored`
///     - `f(mapping) -> bool` returns `true` if the given mapping should not participate in change tracking.
///     - [`make_refmapping_ignore_predicate()`] is a typical implementation for this.
#[allow(clippy::too_many_arguments)]
pub fn mark_complete_and_common_ref<Out, F, E>(
    objects: &(impl gix_object::Find + gix_object::FindHeader + gix_object::Exists),
    refs: &gix_ref::file::Store,
    alternates: impl FnOnce() -> Result<Out, E>,
    negotiator: &mut dyn gix_negotiate::Negotiator,
    graph: &mut gix_negotiate::Graph<'_, '_>,
    ref_map: &RefMap,
    shallow: &Shallow,
    mapping_is_ignored: impl Fn(&refmap::Mapping) -> bool,
) -> Result<Action, Error>
where
    E: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    Out: Iterator<Item = (gix_ref::file::Store, F)>,
    F: gix_object::Find,
{
    let _span = gix_trace::detail!("mark_complete_and_common_ref", mappings = ref_map.mappings.len());
    if ref_map.mappings.is_empty() {
        return Ok(Action::NoChange);
    }
    if let Shallow::Deepen(0) = shallow {
        // Avoid deepening (relative) with zero as it seems to upset the server. Git also doesn't actually
        // perform the negotiation for some reason (couldn't find it in code).
        return Ok(Action::NoChange);
    }
    if let Some(refmap::Mapping {
        remote: refmap::Source::Ref(crate::handshake::Ref::Unborn { .. }),
        ..
    }) = ref_map.mappings.last().filter(|_| ref_map.mappings.len() == 1)
    {
        // There is only an unborn branch, as the remote has an empty repository. This means there is nothing to do except for
        // possibly reproducing the unborn branch locally.
        return Ok(Action::SkipToRefUpdate);
    }

    // Compute the cut-off date by checking which of the refs advertised (and matched in refspecs) by the remote we have,
    // and keep the oldest one.
    let mut cutoff_date = None::<SecondsSinceUnixEpoch>;
    let mut num_mappings_with_change = 0;
    let mut remote_ref_target_known: Vec<bool> = std::iter::repeat_n(false, ref_map.mappings.len()).collect();
    let mut remote_ref_included: Vec<bool> = std::iter::repeat_n(false, ref_map.mappings.len()).collect();

    for (mapping_idx, mapping) in ref_map.mappings.iter().enumerate() {
        let want_id = mapping.remote.as_id();
        let have_id = mapping.local.as_ref().and_then(|name| {
            // this is the only time git uses the peer-id.
            let r = refs.find(name).ok()?;
            r.target.try_id().map(ToOwned::to_owned)
        });

        // Even for ignored mappings we want to know if the `want` is already present locally, so skip nothing else.
        if !mapping_is_ignored(mapping) {
            remote_ref_included[mapping_idx] = true;
            // Like git, we don't let known unchanged mappings participate in the tree traversal
            if want_id.zip(have_id).is_none_or(|(want, have)| want != have) {
                num_mappings_with_change += 1;
            }
        }

        if let Some(commit) = want_id
            .and_then(|id| graph.get_or_insert_commit(id.into(), |_| {}).transpose())
            .transpose()?
        {
            remote_ref_target_known[mapping_idx] = true;
            cutoff_date = cutoff_date.unwrap_or_default().max(commit.commit_time).into();
        } else if want_id.is_some_and(|maybe_annotated_tag| objects.exists(maybe_annotated_tag)) {
            remote_ref_target_known[mapping_idx] = true;
        }
    }

    if matches!(shallow, Shallow::NoChange) {
        if num_mappings_with_change == 0 {
            return Ok(Action::NoChange);
        } else if remote_ref_target_known
            .iter()
            .zip(remote_ref_included)
            .filter_map(|(known, included)| included.then_some(known))
            .all(|known| *known)
        {
            return Ok(Action::SkipToRefUpdate);
        }
    }

    // color our commits as complete as identified by references, unconditionally
    // (`git` is conditional here based on `deepen`, but it doesn't make sense and it's hard to extract from history when that happened).
    let mut queue = Queue::new();
    mark_all_refs_in_repo(refs, objects, graph, &mut queue, Flags::COMPLETE)?;
    for (alt_refs, alt_objs) in alternates().map_err(|err| Error::AlternateRefsAndObjects(err.into()))? {
        mark_all_refs_in_repo(&alt_refs, &alt_objs, graph, &mut queue, Flags::COMPLETE)?;
    }
    // Keep track of the tips, which happen to be on our queue right, before we traverse the graph with cutoff.
    let tips = if let Some(cutoff) = cutoff_date {
        let tips = Cow::Owned(queue.clone());
        // color all their parents up to the cutoff date, the oldest commit we know the server has.
        mark_recent_complete_commits(&mut queue, graph, cutoff)?;
        tips
    } else {
        Cow::Borrowed(&queue)
    };

    gix_trace::detail!("mark known_common").into_scope(|| -> Result<_, Error> {
        // mark all complete advertised refs as common refs.
        for mapping in ref_map
            .mappings
            .iter()
            .zip(remote_ref_target_known.iter().copied())
            // We need this filter as the graph wouldn't contain annotated tags.
            .filter_map(|(mapping, known)| (!known).then_some(mapping))
        {
            let want_id = mapping.remote.as_id();
            if let Some(common_id) = want_id
                .and_then(|id| graph.get(id).map(|c| (c, id)))
                .filter(|(c, _)| c.data.flags.contains(Flags::COMPLETE))
                .map(|(_, id)| id)
            {
                negotiator.known_common(common_id.into(), graph)?;
            }
        }
        Ok(())
    })?;

    // As negotiators currently may rely on getting `known_common` calls first and tips after, we adhere to that which is the only
    // reason we cached the set of tips.
    gix_trace::detail!("mark tips", num_tips = tips.len()).into_scope(|| -> Result<_, Error> {
        for tip in tips.iter_unordered() {
            negotiator.add_tip(*tip, graph)?;
        }
        Ok(())
    })?;

    Ok(Action::MustNegotiate {
        remote_ref_target_known,
    })
}

/// Create a predicate that checks if a refspec mapping should be ignored.
///
/// We want to ignore mappings during negotiation if they would be handled implicitly by the server, which is the case
/// when tags would be sent implicitly due to `Tags::Included`.
pub fn make_refmapping_ignore_predicate(fetch_tags: Tags, ref_map: &RefMap) -> impl Fn(&refmap::Mapping) -> bool + '_ {
    // With included tags, we have to keep mappings of tags to handle them later when updating refs, but we don't want to
    // explicitly `want` them as the server will determine by itself which tags are pointing to a commit it wants to send.
    // If we would not exclude implicit tag mappings like this, we would get too much of the graph.
    let tag_refspec_to_ignore = matches!(fetch_tags, Tags::Included)
        .then(|| fetch_tags.to_refspec())
        .flatten();
    move |mapping| {
        tag_refspec_to_ignore.is_some_and(|tag_spec| {
            mapping
                .spec_index
                .implicit_index()
                .and_then(|idx| ref_map.extra_refspecs.get(idx))
                .is_some_and(|spec| spec.to_ref() == tag_spec)
        })
    }
}

/// Add all 'wants' to `arguments` once it's known negotiation is necessary.
///
/// This is a call to be made when [`mark_complete_and_common_ref()`] returned [`Action::MustNegotiate`].
/// That variant also contains the `remote_ref_target_known` field which is supposed to be passed here.
///
/// `objects` are used to see if remote ids are known here and are tags, in which case they are also added as 'haves' as
/// [negotiators](gix_negotiate::Negotiator) don't see tags at all.
///
/// * `ref_map` is the state of refs as known on the remote.
/// * `shallow` defines if the history should be shallow.
/// * `mapping_is_ignored` is typically initialized with [`make_refmapping_ignore_predicate`].
///
/// Returns `true` if at least one [want](crate::fetch::Arguments::want()) was added, or `false` otherwise.
/// Note that not adding a single want can make the remote hang, so it's avoided on the client side by ending the fetch operation.
pub fn add_wants(
    objects: &impl gix_object::FindHeader,
    arguments: &mut crate::fetch::Arguments,
    ref_map: &RefMap,
    remote_ref_target_known: &[bool],
    shallow: &Shallow,
    mapping_is_ignored: impl Fn(&refmap::Mapping) -> bool,
) -> bool {
    // When using shallow, we can't exclude `wants` as the remote won't send anything then. Thus, we have to resend everything
    // we have as want instead to get exactly the same graph, but possibly deepened.
    let is_shallow = !matches!(shallow, Shallow::NoChange);
    let mut has_want = false;
    let wants = ref_map
        .mappings
        .iter()
        .zip(remote_ref_target_known)
        .filter_map(|(m, known)| (is_shallow || !*known).then_some(m))
        .filter(|m| !mapping_is_ignored(m));
    for want in wants {
        let id_on_remote = want.remote.as_id();
        if !arguments.can_use_ref_in_want() || matches!(want.remote, refmap::Source::ObjectId(_)) {
            if let Some(id) = id_on_remote {
                arguments.want(id);
                has_want = true;
            }
        } else {
            arguments.want_ref(
                want.remote
                    .as_name()
                    .expect("name available if this isn't an object id"),
            );
            has_want = true;
        }
        let id_is_annotated_tag_we_have = id_on_remote
            .and_then(|id| objects.try_header(id).ok().flatten().map(|h| (id, h)))
            .filter(|(_, h)| h.kind == gix_object::Kind::Tag)
            .map(|(id, _)| id);
        if let Some(tag_on_remote) = id_is_annotated_tag_we_have {
            // Annotated tags are not handled at all by negotiators in the commit-graph - they only see commits and thus won't
            // ever add `have`s for tags. To correct for that, we add these haves here to avoid getting them sent again.
            arguments.have(tag_on_remote);
        }
    }
    has_want
}

/// Remove all commits that are more recent than the cut-off, which is the commit time of the oldest common commit we have with the server.
fn mark_recent_complete_commits(
    queue: &mut Queue,
    graph: &mut gix_negotiate::Graph<'_, '_>,
    cutoff: SecondsSinceUnixEpoch,
) -> Result<(), Error> {
    let _span = gix_trace::detail!("mark_recent_complete", queue_len = queue.len());
    while let Some(id) = queue
        .peek()
        .and_then(|(commit_time, id)| (commit_time >= &cutoff).then_some(*id))
    {
        queue.pop_value();
        let commit = graph.get(&id).expect("definitely set when adding tips or parents");
        for parent_id in commit.parents.clone() {
            let mut was_complete = false;
            if let Some(parent) = graph
                .get_or_insert_commit(parent_id, |md| {
                    was_complete = md.flags.contains(Flags::COMPLETE);
                    md.flags |= Flags::COMPLETE;
                })?
                .filter(|_| !was_complete)
            {
                queue.insert(parent.commit_time, parent_id);
            }
        }
    }
    Ok(())
}

fn mark_all_refs_in_repo(
    store: &gix_ref::file::Store,
    objects: &impl gix_object::Find,
    graph: &mut gix_negotiate::Graph<'_, '_>,
    queue: &mut Queue,
    mark: Flags,
) -> Result<(), Error> {
    let _span = gix_trace::detail!("mark_all_refs");
    for local_ref in store.iter()?.all()? {
        let mut local_ref = local_ref?;
        let id = local_ref.peel_to_id_packed(store, objects, store.cached_packed_buffer()?.as_ref().map(|b| &***b))?;
        let mut is_complete = false;
        if let Some(commit) = graph
            .get_or_insert_commit(id, |md| {
                is_complete = md.flags.contains(Flags::COMPLETE);
                md.flags |= mark;
            })?
            .filter(|_| !is_complete)
        {
            queue.insert(commit.commit_time, id);
        };
    }
    Ok(())
}

///
pub mod one_round {
    /// State to keep between individual [rounds](super::one_round()).
    #[derive(Clone, Debug)]
    pub struct State {
        /// The amount of haves to send the next round.
        /// It's initialized with the standard window size for negotations.
        pub haves_to_send: usize,
        /// Is turned `true` if the remote as confirmed any common commit so far.
        pub(super) seen_ack: bool,
        /// The amount of haves we have sent that didn't have a match on the remote.
        ///
        /// The higher this number, the more time was wasted.
        pub(super) in_vain: usize,
        /// Commits we have in common.
        ///
        /// Only set when we are stateless as we have to resend known common commits each round.
        pub(super) common_commits: Option<Vec<gix_hash::ObjectId>>,
    }

    impl State {
        /// Create a new instance.
        ///
        /// setting `connection_is_stateless` accordingly which affects the amount of haves to send.
        pub fn new(connection_is_stateless: bool) -> Self {
            State {
                haves_to_send: gix_negotiate::window_size(connection_is_stateless, None),
                seen_ack: false,
                in_vain: 0,
                common_commits: connection_is_stateless.then(Vec::new),
            }
        }
    }

    impl State {
        /// Return `true` if the transports connection is stateless.
        fn connection_is_stateless(&self) -> bool {
            self.common_commits.is_some()
        }
        pub(super) fn adjust_window_size(&mut self) {
            self.haves_to_send = gix_negotiate::window_size(self.connection_is_stateless(), Some(self.haves_to_send));
        }
    }
}

/// Prepare to negotiate a single round in the process of letting the remote know what we have, and have in common.
///
/// Note that this function only configures `arguments`, no IO is performed.
///
/// The operation is performed with `negotiator` and `graph`, sending the amount of `haves_to_send` after possibly
/// making the common commits (as sent by the remote) known to `negotiator` using `previous_response`, if this isn't the first round.
/// All [commits we have](crate::fetch::Arguments::have()) are added to `arguments` accordingly.
///
/// Returns information about this round, and `true` if we are done and should stop negotiating *after* the `arguments` have
/// been sent to the remote one last time.
pub fn one_round(
    negotiator: &mut dyn gix_negotiate::Negotiator,
    graph: &mut gix_negotiate::Graph<'_, '_>,
    state: &mut one_round::State,
    arguments: &mut crate::fetch::Arguments,
    previous_response: Option<&crate::fetch::Response>,
) -> Result<(Round, bool), Error> {
    let mut seen_ack = false;
    if let Some(response) = previous_response {
        use crate::fetch::response::Acknowledgement;
        for ack in response.acknowledgements() {
            match ack {
                Acknowledgement::Common(id) => {
                    seen_ack = true;
                    negotiator.in_common_with_remote(*id, graph)?;
                    if let Some(common) = &mut state.common_commits {
                        common.push(*id);
                    }
                }
                Acknowledgement::Ready => {
                    // NOTE: In git, there is some logic dealing with whether to expect a DELIM or FLUSH package,
                    //       but we handle this with peeking.
                }
                Acknowledgement::Nak => {}
            }
        }
    }

    // `common` is set only if this is a stateless transport, and we repeat previously confirmed common commits as HAVE, because
    // we are not going to repeat them otherwise.
    if let Some(common) = &mut state.common_commits {
        for have_id in common {
            arguments.have(have_id);
        }
    }

    let mut haves_added = 0;
    for have_id in (0..state.haves_to_send).map_while(|_| negotiator.next_have(graph)) {
        arguments.have(have_id?);
        haves_added += 1;
    }
    // Note that we are differing from the git implementation, which does an extra-round of with no new haves sent at all.
    // For us, it seems better to just say we are done when we know we are done, as potentially additional acks won't affect the
    // queue of our implementation at all (so the negotiator won't come up with more haves next time either).
    if seen_ack {
        state.in_vain = 0;
    }
    state.seen_ack |= seen_ack;
    state.in_vain += haves_added;
    let round = Round {
        haves_sent: haves_added,
        in_vain: state.in_vain,
        haves_to_send: state.haves_to_send,
        previous_response_had_at_least_one_in_common: seen_ack,
    };
    let is_done = haves_added != state.haves_to_send || (state.seen_ack && state.in_vain >= 256);
    state.adjust_window_size();

    Ok((round, is_done))
}
