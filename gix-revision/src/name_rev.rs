use std::{
    borrow::Cow,
    fmt::{Display, Formatter},
};

use bstr::{BStr, BString, ByteVec};

/// A prepared reference tip from which names will be propagated through the commit graph.
#[derive(Debug, Clone)]
pub struct Tip<'name> {
    /// The peeled commit id the name points to.
    pub id: gix_hash::ObjectId,
    /// The display name associated with `id`.
    pub name: Cow<'name, BStr>,
    /// The timestamp used for ordering competing names.
    pub taggerdate: i64,
    /// If true, this tip came from a tag reference.
    pub from_tag: bool,
    /// If true, the display name names a tag object and needs `^0` when naming the tagged commit directly.
    pub deref: bool,
}

/// The positive result produced by [name_rev()][function::name_rev()].
#[derive(Debug, Clone)]
pub struct Outcome<'name> {
    /// The input commit object id that we name.
    pub id: gix_hash::ObjectId,
    /// The name found for `id`, or `None` if fallback formatting was requested and no name exists.
    pub name: Option<Cow<'name, BStr>>,
    /// The amount of commits traversed while propagating names.
    pub commits_seen: u32,
}

impl<'name> Outcome<'name> {
    /// Turn this outcome into a displayable `git name-rev`-like format.
    pub fn into_format(self, hex_len: usize) -> Format<'name> {
        Format {
            id: self.id,
            name: self.name,
            hex_len,
        }
    }
}

/// A structure implementing `Display`, producing a `git name-rev`-like string.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct Format<'name> {
    /// The object id to use if no symbolic name is available.
    pub id: gix_hash::ObjectId,
    /// The symbolic name to display, if one was found.
    pub name: Option<Cow<'name, BStr>>,
    /// The amount of hex characters to use for id fallback formatting.
    pub hex_len: usize,
}

impl Display for Format<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.name.as_deref() {
            Some(name) => name.fmt(f),
            None => self.id.to_hex_with_len(self.hex_len).fmt(f),
        }
    }
}

/// The options required to call [`name_rev()`][function::name_rev()].
#[derive(Clone, Debug, Default)]
pub struct Options<'name> {
    /// The prepared tips from which to propagate names through history.
    pub tips: Vec<Tip<'name>>,
    /// If no candidate for naming exists, always show the abbreviated hash.
    pub fallback_to_oid: bool,
}

/// The error returned by the [`name_rev()`](function::name_rev()) function.
pub type Error = gix_error::Message;

/// The per-commit state stored in the graph while running [`name_rev()`](function::name_rev()).
#[derive(Debug, Clone)]
pub struct State<'name> {
    tip_name: Cow<'name, BStr>,
    taggerdate: i64,
    generation: u32,
    distance: u32,
    from_tag: bool,
}

impl State<'_> {
    fn format(&self) -> BString {
        if self.generation == 0 {
            self.tip_name.as_ref().to_owned()
        } else {
            let base = self.tip_name.as_ref().strip_suffix(b"^0").unwrap_or(&self.tip_name);
            let mut out = BString::from(base);
            out.push_str(format!("~{}", self.generation));
            out
        }
    }
}

pub(crate) mod function {
    use std::{borrow::Cow, cmp::Ordering};

    use bstr::{BString, ByteVec};
    use gix_error::{Exn, ResultExt, message};
    use gix_hash::oid;

    use super::{Error, Outcome, State};
    use crate::{Graph, name_rev::Options};

    const MERGE_TRAVERSAL_WEIGHT: u32 = 65535;
    const CUTOFF_DATE_SLOP: i64 = 86400;

    /// Given a `commit` id, propagate all prepared names through `graph` and produce an `Outcome`.
    pub fn name_rev<'name>(
        commit: &oid,
        graph: &mut Graph<'_, '_, State<'name>>,
        Options {
            mut tips,
            fallback_to_oid,
        }: Options<'name>,
    ) -> Result<Option<Outcome<'name>>, Exn<Error>> {
        let _span = gix_trace::coarse!(
            "gix_revision::name_rev()",
            commit = %commit,
            name_count = tips.len()
        );

        if tips.is_empty() {
            return if fallback_to_oid {
                Ok(Some(Outcome {
                    id: commit.to_owned(),
                    name: None,
                    commits_seen: 0,
                }))
            } else {
                Ok(None)
            };
        }

        tips.sort_by(|a, b| {
            b.from_tag
                .cmp(&a.from_tag)
                .then_with(|| a.taggerdate.cmp(&b.taggerdate))
        });

        let mut commits_seen = 0;
        graph.clear();
        let cutoff = cutoff_timestamp(
            graph
                .lookup(commit)
                .or_raise(|| message!("could not lookup commit {}", commit.to_hex()))?
                .committer_timestamp()
                .or_raise(|| message!("could not read commit timestamp {}", commit.to_hex()))?,
        );

        for tip in tips {
            if graph
                .lookup(&tip.id)
                .or_raise(|| message!("could not lookup tip commit {}", tip.id.to_hex()))?
                .committer_timestamp()
                .or_raise(|| message!("could not read tip commit timestamp {}", tip.id.to_hex()))?
                < cutoff
            {
                continue;
            }

            let mut name = tip.name.into_owned();
            if tip.deref {
                name.push_str("^0");
            }
            let name = State {
                tip_name: Cow::Owned(name),
                taggerdate: tip.taggerdate,
                generation: 0,
                distance: 0,
                from_tag: tip.from_tag,
            };

            if create_or_update_name(graph, tip.id, name) {
                propagate_names(tip.id, cutoff, graph, &mut commits_seen)?;
            }
        }

        match graph.get(commit) {
            Some(name) => Ok(Some(Outcome {
                id: commit.to_owned(),
                name: Some(Cow::Owned(name.format())),
                commits_seen,
            })),
            None if fallback_to_oid => Ok(Some(Outcome {
                id: commit.to_owned(),
                name: None,
                commits_seen,
            })),
            None => Ok(None),
        }
    }

    fn propagate_names(
        start: gix_hash::ObjectId,
        cutoff: i64,
        graph: &mut Graph<'_, '_, State<'_>>,
        commits_seen: &mut u32,
    ) -> Result<(), Exn<Error>> {
        let mut stack = vec![start];
        let mut parents_to_queue = Vec::new();

        while let Some(commit) = stack.pop() {
            *commits_seen += 1;
            let name = graph.get(&commit).expect("all queued commits have names").clone();

            parents_to_queue.clear();
            let parents = {
                let commit_object = graph
                    .lookup(&commit)
                    .or_raise(|| message!("could not lookup commit {}", commit.to_hex()))?;
                commit_object
                    .iter_parents()
                    .collect::<Result<Vec<_>, _>>()
                    .or_raise(|| message!("could not read parents of commit {}", commit.to_hex()))?
            };

            for (parent_index, parent_id) in parents.into_iter().enumerate() {
                let Some(parent_commit) = graph
                    .try_lookup(&parent_id)
                    .or_raise(|| message!("could not lookup parent commit {}", parent_id.to_hex()))?
                else {
                    continue;
                };
                if parent_commit
                    .committer_timestamp()
                    .or_raise(|| message!("could not read parent commit timestamp {}", parent_id.to_hex()))?
                    < cutoff
                {
                    continue;
                }

                let parent_number = parent_index as u32 + 1;
                let (tip_name, generation, distance) = if parent_number > 1 {
                    (
                        Cow::Owned(parent_name(&name, parent_number)),
                        0,
                        name.distance.saturating_add(MERGE_TRAVERSAL_WEIGHT),
                    )
                } else {
                    (
                        name.tip_name.clone(),
                        name.generation.saturating_add(1),
                        name.distance.saturating_add(1),
                    )
                };

                let parent_name = State {
                    tip_name,
                    taggerdate: name.taggerdate,
                    generation,
                    distance,
                    from_tag: name.from_tag,
                };

                if create_or_update_name(graph, parent_id, parent_name) {
                    parents_to_queue.push(parent_id);
                }
            }

            while let Some(parent) = parents_to_queue.pop() {
                stack.push(parent);
            }
        }

        Ok(())
    }

    fn cutoff_timestamp(timestamp: i64) -> i64 {
        timestamp.saturating_sub(CUTOFF_DATE_SLOP)
    }

    fn create_or_update_name<'name>(
        graph: &mut Graph<'_, '_, State<'name>>,
        id: gix_hash::ObjectId,
        new_name: State<'name>,
    ) -> bool {
        match graph.get_mut(&id) {
            Some(name) => {
                if !is_better_name(name, &new_name) {
                    return false;
                }
                *name = new_name;
            }
            None => {
                graph.insert(id, new_name);
            }
        }
        true
    }

    fn is_better_name(current: &State<'_>, new: &State<'_>) -> bool {
        let current_distance = effective_distance(current.distance, current.generation);
        let new_distance = effective_distance(new.distance, new.generation);

        if new.from_tag && current.from_tag {
            return current_distance > new_distance;
        }

        if current.from_tag != new.from_tag {
            return new.from_tag;
        }

        match current_distance.cmp(&new_distance) {
            Ordering::Greater => return true,
            Ordering::Less => return false,
            Ordering::Equal => {}
        }

        if current.taggerdate != new.taggerdate {
            return current.taggerdate > new.taggerdate;
        }

        false
    }

    fn effective_distance(distance: u32, generation: u32) -> u32 {
        distance.saturating_add(if generation > 0 { MERGE_TRAVERSAL_WEIGHT } else { 0 })
    }

    fn parent_name(name: &State<'_>, parent_number: u32) -> BString {
        let base = name.tip_name.as_ref().strip_suffix(b"^0").unwrap_or(&name.tip_name);
        let mut out = BString::from(base);
        if name.generation > 0 {
            out.push_str(format!("~{}^{}", name.generation, parent_number));
        } else {
            out.push_str(format!("^{parent_number}"));
        }
        out
    }
}
