use std::{
    borrow::Cow,
    fmt::{Display, Formatter},
};

use bstr::BStr;
use gix_hashtable::HashMap;

/// The positive result produced by [describe()][function::describe()].
#[derive(Debug, Clone)]
pub struct Outcome<'name> {
    /// The name of the tag or branch that is closest to the commit `id`.
    ///
    /// If `None`, no name was found but it was requested to provide the `id` itself as fallback.
    pub name: Option<Cow<'name, BStr>>,
    /// The input commit object id that we describe.
    pub id: gix_hash::ObjectId,
    /// The number of commits that are between the tag or branch with `name` and `id`.
    /// These commits are all in the future of the named tag or branch.
    pub depth: u32,
    /// The mapping between object ids and their names initially provided by the describe call.
    pub name_by_oid: HashMap<gix_hash::ObjectId, Cow<'name, BStr>>,
    /// The amount of commits we traversed.
    pub commits_seen: u32,
}

impl<'a> Outcome<'a> {
    /// Turn this outcome into a structure that can display itself in the typical `git describe` format.
    pub fn into_format(self, hex_len: usize) -> Format<'a> {
        Format {
            name: self.name,
            id: self.id,
            hex_len,
            depth: self.depth,
            long: false,
            dirty_suffix: None,
        }
    }
}

/// A structure implementing `Display`, producing a `git describe` like string.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct Format<'a> {
    /// The name of the branch or tag to display, as is.
    ///
    /// If `None`, the `id` will be displayed as a fallback.
    pub name: Option<Cow<'a, BStr>>,
    /// The `id` of the commit to describe.
    pub id: gix_hash::ObjectId,
    /// The amount of hex characters to use to display `id`.
    pub hex_len: usize,
    /// The amount of commits between `name` and `id`, where `id` is in the future of `name`.
    pub depth: u32,
    /// If true, the long form of the describe string will be produced even if `id` lies directly on `name`,
    /// hence has a depth of 0.
    pub long: bool,
    /// If `Some(suffix)`, it will be appended to the describe string.
    /// This should be set if the working tree was determined to be dirty.
    pub dirty_suffix: Option<String>,
}

impl Format<'_> {
    /// Return true if the `name` is directly associated with `id`, i.e. there are no commits between them.
    pub fn is_exact_match(&self) -> bool {
        self.depth == 0
    }

    /// Set this instance to print in long mode, that is if `depth` is 0, it will still print the whole
    /// long form even though it's not quite necessary.
    ///
    /// Otherwise, it is allowed to shorten itself.
    pub fn long(&mut self, long: bool) -> &mut Self {
        self.long = long;
        self
    }
}

impl Display for Format<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = self.name.as_deref() {
            if !self.long && self.is_exact_match() {
                name.fmt(f)?;
            } else {
                write!(f, "{}-{}-g{}", name, self.depth, self.id.to_hex_with_len(self.hex_len))?;
            }
        } else {
            self.id.to_hex_with_len(self.hex_len).fmt(f)?;
        }

        if let Some(suffix) = &self.dirty_suffix {
            write!(f, "-{suffix}")?;
        }
        Ok(())
    }
}

/// A bit-field which keeps track of which commit is reachable by one of 32 candidates names.
pub type Flags = u32;
const MAX_CANDIDATES: usize = std::mem::size_of::<Flags>() * 8;

/// The options required to call [`describe()`][function::describe()].
#[derive(Clone, Debug)]
pub struct Options<'name> {
    /// The candidate names from which to determine the `name` to use for the describe string,
    /// as a mapping from a commit id and the name associated with it.
    pub name_by_oid: HashMap<gix_hash::ObjectId, Cow<'name, BStr>>,
    /// The amount of names we will keep track of. Defaults to the maximum of 32.
    ///
    /// If the number is exceeded, it will be capped at 32 and defaults to 10.
    pub max_candidates: usize,
    /// If no candidate for naming, always show the abbreviated hash. Default: false.
    pub fallback_to_oid: bool,
    /// Only follow the first parent during graph traversal. Default: false.
    ///
    /// This may speed up the traversal at the cost of accuracy.
    pub first_parent: bool,
}

impl Default for Options<'_> {
    fn default() -> Self {
        Options {
            max_candidates: 10, // the same number as git uses, otherwise we perform worse by default on big repos
            name_by_oid: Default::default(),
            fallback_to_oid: false,
            first_parent: false,
        }
    }
}

/// The error returned by the [`describe()`][function::describe()] function.
pub type Error = Simple;

/// A simple error type for describe operations.
#[derive(Debug)]
pub struct Simple(pub String);

impl std::fmt::Display for Simple {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Simple {}

pub(crate) mod function {
    use std::{borrow::Cow, cmp::Ordering};

    use bstr::BStr;
    use gix_hash::oid;

    use super::{Error, Simple, Outcome};
    use crate::{
        describe::{CommitTime, Flags, Options, MAX_CANDIDATES},
        Graph, PriorityQueue,
    };

    /// Given a `commit` id, traverse the commit `graph` and collect candidate names from the `name_by_oid` mapping to produce
    /// an `Outcome`, which converted [`into_format()`][Outcome::into_format()] will produce a typical `git describe` string.
    ///
    /// Note that the `name_by_oid` map is returned in the [`Outcome`], which can be forcefully returned even if there was no matching
    /// candidate by setting `fallback_to_oid` to true.
    pub fn describe<'name>(
        commit: &oid,
        graph: &mut Graph<'_, '_, Flags>,
        Options {
            name_by_oid,
            mut max_candidates,
            fallback_to_oid,
            first_parent,
        }: Options<'name>,
    ) -> Result<Option<Outcome<'name>>, Error> {
        let _span = gix_trace::coarse!(
            "gix_revision::describe()",
            commit = %commit,
            name_count = name_by_oid.len(),
            max_candidates,
            first_parent
        );
        max_candidates = max_candidates.min(MAX_CANDIDATES);
        if let Some(name) = name_by_oid.get(commit) {
            return Ok(Some(Outcome {
                name: name.clone().into(),
                id: commit.to_owned(),
                depth: 0,
                name_by_oid,
                commits_seen: 0,
            }));
        }

        if max_candidates == 0 || name_by_oid.is_empty() {
            return if fallback_to_oid {
                Ok(Some(Outcome {
                    id: commit.to_owned(),
                    name: None,
                    name_by_oid,
                    depth: 0,
                    commits_seen: 0,
                }))
            } else {
                Ok(None)
            };
        }

        let mut queue = PriorityQueue::from_iter(Some((u32::MAX, commit.to_owned())));
        let mut candidates = Vec::new();
        let mut commits_seen = 0;
        let mut gave_up_on_commit = None;
        graph.clear();
        graph.insert(commit.to_owned(), 0u32);

        while let Some(commit) = queue.pop_value() {
            commits_seen += 1;
            let flags = if let Some(name) = name_by_oid.get(&commit) {
                if candidates.len() < max_candidates {
                    let identity_bit = 1 << candidates.len();
                    candidates.push(Candidate {
                        name: name.clone(),
                        commits_in_its_future: commits_seen - 1,
                        identity_bit,
                        order: candidates.len(),
                    });
                    let flags = graph.get_mut(&commit).expect("inserted");
                    *flags |= identity_bit;
                    *flags
                } else {
                    gave_up_on_commit = Some(commit);
                    break;
                }
            } else {
                graph[&commit]
            };

            for candidate in candidates
                .iter_mut()
                .filter(|c| (flags & c.identity_bit) != c.identity_bit)
            {
                candidate.commits_in_its_future += 1;
            }

            if queue.is_empty() && !candidates.is_empty() {
                // single-trunk history that waits to be replenished.
                // Abort early if the best-candidate is in the current commits past.
                let mut shortest_depth = Flags::MAX;
                let mut best_candidates_at_same_depth = 0_u32;
                for candidate in &candidates {
                    match candidate.commits_in_its_future.cmp(&shortest_depth) {
                        Ordering::Less => {
                            shortest_depth = candidate.commits_in_its_future;
                            best_candidates_at_same_depth = candidate.identity_bit;
                        }
                        Ordering::Equal => {
                            best_candidates_at_same_depth |= candidate.identity_bit;
                        }
                        Ordering::Greater => {}
                    }
                }

                if (flags & best_candidates_at_same_depth) == best_candidates_at_same_depth {
                    break;
                }
            }

            parents_by_date_onto_queue_and_track_names(graph, &mut queue, commit, flags, first_parent)?;
        }

        if candidates.is_empty() {
            return if fallback_to_oid {
                Ok(Some(Outcome {
                    id: commit.to_owned(),
                    name: None,
                    name_by_oid,
                    depth: 0,
                    commits_seen,
                }))
            } else {
                Ok(None)
            };
        }

        candidates.sort_by(|a, b| {
            a.commits_in_its_future
                .cmp(&b.commits_in_its_future)
                .then_with(|| a.order.cmp(&b.order))
        });

        if let Some(commit_id) = gave_up_on_commit {
            queue.insert(u32::MAX, commit_id);
            commits_seen -= 1;
        }

        commits_seen += finish_depth_computation(
            queue,
            graph,
            candidates.first_mut().expect("at least one candidate"),
            first_parent,
        )?;

        Ok(candidates.into_iter().next().map(|c| Outcome {
            name: c.name.into(),
            id: commit.to_owned(),
            depth: c.commits_in_its_future,
            name_by_oid,
            commits_seen,
        }))
    }

    fn parents_by_date_onto_queue_and_track_names(
        graph: &mut Graph<'_, '_, Flags>,
        queue: &mut PriorityQueue<CommitTime, gix_hash::ObjectId>,
        commit: gix_hash::ObjectId,
        commit_flags: Flags,
        first_parent: bool,
    ) -> Result<(), Error> {
        graph
            .insert_parents(
                &commit,
                &mut |parent_id, parent_commit_date| {
                    queue.insert(parent_commit_date as u32, parent_id);
                    commit_flags
                },
                &mut |_parent_id, flags| *flags |= commit_flags,
                first_parent,
            )
            .map_err(|_| {
                Simple(format!(
                    "could not insert parents of commit {} into graph",
                    commit.to_hex()
                ))
            })?;
        Ok(())
    }

    fn finish_depth_computation(
        mut queue: PriorityQueue<CommitTime, gix_hash::ObjectId>,
        graph: &mut Graph<'_, '_, Flags>,
        best_candidate: &mut Candidate<'_>,
        first_parent: bool,
    ) -> Result<u32, Error> {
        let mut commits_seen = 0;
        while let Some(commit) = queue.pop_value() {
            commits_seen += 1;
            let flags = graph[&commit];
            if (flags & best_candidate.identity_bit) == best_candidate.identity_bit {
                if queue
                    .iter_unordered()
                    .all(|id| (graph[id] & best_candidate.identity_bit) == best_candidate.identity_bit)
                {
                    break;
                }
            } else {
                best_candidate.commits_in_its_future += 1;
            }

            parents_by_date_onto_queue_and_track_names(graph, &mut queue, commit, flags, first_parent)?;
        }
        Ok(commits_seen)
    }

    #[derive(Debug)]
    struct Candidate<'a> {
        name: Cow<'a, BStr>,
        commits_in_its_future: Flags,
        /// A single bit identifying this candidate uniquely in a bitset
        identity_bit: Flags,
        /// The order at which we found the candidate, first one has order = 0
        order: usize,
    }
}

/// The timestamp for the creation date of a commit in seconds since unix epoch.
type CommitTime = u32;
