//! Rename and copy detection following Git's `diffcore-rename.c`.
//!
//! Exact matches precede a unique-basename pass and a globally ranked, four-candidates-per-destination
//! similarity matrix. Directory relations remain an extension of this API; Git's merge-specific
//! directory heuristics and break detection are not part of this tracker.

use gix_error::{Result, message};
use std::ops::Range;

use bstr::{BStr, ByteSlice};
use gix_error::ResultExt;
use gix_object::tree::{EntryKind, EntryMode};

use crate::{
    Rewrites,
    blob::ResourceKind,
    rewrites::{
        CopySource, Outcome, Similarity, Tracker,
        similarity::{MAX_SCORE, Signature},
        tracker::visit::SourceKind,
    },
    tree::visit::{Action, ChangeId, Relation},
};

/// The kind of a change.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, PartialEq, Eq)]
pub enum ChangeKind {
    /// The change represents the *deletion* of an item.
    Deletion,
    /// The change represents the *modification* of an item.
    Modification,
    /// The change represents the *addition* of an item.
    Addition,
}

/// A trait providing all functionality to abstract over the concept of a change, as seen by the [`Tracker`].
pub trait Change: Clone {
    /// Return the hash of the object behind this change for identification.
    ///
    /// Note that this is the id of the object as stored in `git`, i.e. it must have gone through workspace
    /// conversions. What matters is that the IDs are comparable.
    /// Modifications return the previous id, as copies originate from the source state.
    fn id(&self) -> &gix_hash::oid;
    /// Return the relation that this change may have with other changes.
    ///
    /// It allows to associate a directory with its children that are added or removed at the same moment.
    /// Note that this is ignored for modifications.
    ///
    /// If rename-tracking should always be on leaf-level, this should be set to `None` consistently.
    /// Note that trees will never be looked up by their `id` as their children are assumed to be passed in
    /// with the respective relationship.
    ///
    /// Also note that the tracker only sees what's given to it, it will not lookup trees or match paths itself.
    fn relation(&self) -> Option<Relation>;
    /// Return the kind of this change.
    fn kind(&self) -> ChangeKind;
    /// Return the entry mode, using the previous mode for modifications.
    fn entry_mode(&self) -> EntryMode;
}

/// A set of tracked items allows to figure out their relations by figuring out their similarity.
pub(crate) struct Item<T> {
    /// The underlying raw change
    change: T,
    /// That slice into the backing for paths.
    path: Range<usize>,
    /// If true, this item was already emitted, i.e. seen by the caller.
    emitted: bool,
}

impl<T: Change> Item<T> {
    fn location<'a>(&self, backing: &'a [u8]) -> &'a BStr {
        backing[self.path.clone()].as_ref()
    }
    fn entry_mode_compatible(&self, other: EntryMode) -> bool {
        use EntryKind::*;
        matches!(
            (other.kind(), self.change.entry_mode().kind()),
            (Blob | BlobExecutable, Blob | BlobExecutable) | (Link, Link) | (Tree, Tree) | (Commit, Commit)
        )
    }
}

/// A module with types used in the user-callback in [Tracker::emit()](crate::rewrites::Tracker::emit()).
pub mod visit {
    use bstr::BStr;
    use gix_object::tree::EntryMode;

    use crate::rewrites::Similarity;

    /// The source of a rewrite, rename or copy.
    #[derive(Debug, Clone, PartialEq, PartialOrd)]
    pub struct Source<'a, T> {
        /// The kind of entry.
        pub entry_mode: EntryMode,
        /// The hash of the state of the source as seen in the object database.
        pub id: gix_hash::ObjectId,
        /// Further specify what kind of source this is.
        pub kind: SourceKind,
        /// The repository-relative location of this entry.
        pub location: &'a BStr,
        /// The change that was registered as source.
        pub change: &'a T,
        /// The byte-span score of an inexact match, including binary files.
        /// `None` denotes an exact identity match or an inferred directory rename.
        pub similarity: Option<Similarity>,
    }

    /// Further identify the kind of [Source].
    #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
    pub enum SourceKind {
        /// This is the source of an entry that was renamed, as `source` was renamed to `destination`.
        Rename,
        /// This is the source of a copy, as `source` was copied into `destination`.
        Copy,
    }

    /// A change along with a location.
    #[derive(Debug, Clone)]
    pub struct Destination<'a, T: Clone> {
        /// The change at the given `location`.
        pub change: T,
        /// The repository-relative location of this destination.
        pub location: &'a BStr,
    }
}

/// Lifecycle
impl<T: Change> Tracker<T> {
    /// Create a new instance with `rewrites` configuration.
    pub fn new(rewrites: Rewrites) -> Self {
        Tracker {
            items: vec![],
            path_backing: vec![],
            rewrites,
            child_renames: Default::default(),
        }
    }
}

/// build state and find matches.
impl<T: Change> Tracker<T> {
    /// We may refuse the push if that information isn't needed for what we have to track.
    pub fn try_push_change(&mut self, change: T, location: &BStr) -> Option<T> {
        let change_kind = change.kind();
        if let (None, ChangeKind::Modification) = (self.rewrites.copies, change_kind) {
            return Some(change);
        }

        let entry_kind = change.entry_mode().kind();
        let relation = change
            .relation()
            .filter(|_| matches!(change_kind, ChangeKind::Addition | ChangeKind::Deletion));
        if let (None, EntryKind::Tree) = (relation, entry_kind) {
            return Some(change);
        }

        let start = self.path_backing.len();
        self.path_backing.extend_from_slice(location);
        let path = start..self.path_backing.len();

        self.items.push(Item {
            path,
            change,
            emitted: false,
        });
        None
    }

    /// Can only be called once effectively as it alters its own state to assure each item is only emitted once.
    ///
    /// `cb(destination, source)` is called for each item, either with `Some(source)` if it's
    /// the destination of a copy or rename, or with `None` for source if no relation to other
    /// items in the tracked set exist, which is like saying 'no rename or rewrite or copy' happened.
    /// Directories with [relation](Relation) are matched without changing Git's file choices. Unique exact file pairs
    /// can be emitted together with their identical directory; otherwise file renames determine the parent matches.
    ///
    /// `objects` provides blob data for similarity checks. Worktree resources are converted to their Git form
    /// using the filters in `diff_cache`. Similarity uses Git's byte-span fingerprints, including for binary files;
    /// external diff commands, text conversions and the display-oriented large-file threshold are ignored.
    ///
    /// `diff_cache` supplies attributes, worktree roots, filters and reusable buffers. Fingerprints are cached for
    /// this operation. Line diffs are not computed; use the blob diff APIs if line statistics are needed.
    ///
    /// `push_source_tree(push_fn: push(change, location))` is a function that is called when the entire tree of the source
    /// should be added as modifications by calling `push` repeatedly to use for perfect copy tracking. Note that `push`
    /// will panic if `change` is not a modification, and it's valid to not call `push` at all.
    pub fn emit<PushSourceTreeFn, E>(
        &mut self,
        mut cb: impl FnMut(visit::Destination<'_, T>, Option<visit::Source<'_, T>>) -> Action,
        diff_cache: &mut crate::blob::Platform,
        objects: &impl gix_object::FindObjectOrHeader,
        mut push_source_tree: PushSourceTreeFn,
    ) -> Result<Outcome>
    where
        PushSourceTreeFn: FnMut(&mut dyn FnMut(T, &BStr)) -> std::result::Result<(), E>,
        E: std::error::Error + Send + Sync + 'static,
    {
        self.child_renames.clear();
        let mut out = Outcome {
            options: self.rewrites,
            ..Default::default()
        };
        let has_destinations = self.items.iter().any(|item| item.change.kind() == ChangeKind::Addition);
        if has_destinations {
            if self
                .rewrites
                .copies
                .is_some_and(|copies| copies.source == CopySource::FromSetOfModifiedFilesAndAllSources)
            {
                let mut sources: std::collections::HashSet<_> = self
                    .items
                    .iter()
                    .filter(|item| item.change.kind() != ChangeKind::Addition)
                    .map(|item| {
                        (
                            item.location(&self.path_backing).to_vec(),
                            item.change.id().to_owned(),
                            item.change.entry_mode(),
                        )
                    })
                    .collect();
                push_source_tree(&mut |change, location| {
                    assert_eq!(
                        change.kind(),
                        ChangeKind::Modification,
                        "copy sources must be modifications"
                    );
                    if sources.insert((location.to_vec(), change.id().to_owned(), change.entry_mode()))
                        && self.try_push_change(change, location).is_none()
                    {
                        self.items.last_mut().expect("just pushed").emitted = true;
                    }
                })
                .or_raise(|| {
                    message("Could not obtain exhaustive item set to use as possible sources for copy detection")
                })?;
            }
            self.sort_items_by_id_and_location();
            let mut cancelled =
                self.can_match_directories_first() && self.match_identical_directories(&mut cb)?.is_break();
            if !cancelled {
                let mut pairs = self.find_file_renames(&mut out, diff_cache, objects)?;
                let mut directories = Vec::new();
                pairs.retain(|pair| {
                    if self.items[pair.dst].change.entry_mode().is_tree() {
                        directories.push(*pair);
                        false
                    } else {
                        true
                    }
                });
                cancelled = self.emit_pairs(&pairs, &mut cb)?.is_break();
                if !cancelled && !directories.is_empty() {
                    // File choices determine their parents before identical trees can claim another parent.
                    self.match_renamed_directories(&mut |destination, source| {
                        let action = cb(destination, source);
                        cancelled |= action.is_break();
                        action
                    })?;
                    if !cancelled {
                        let mut file_parents = std::collections::HashMap::new();
                        for &(src, dst) in self.child_renames.keys() {
                            for (parent, other) in [(src, dst), (dst, src)] {
                                file_parents
                                    .entry(parent)
                                    .and_modify(|current| {
                                        if *current != Some(other) {
                                            *current = None;
                                        }
                                    })
                                    .or_insert(Some(other));
                            }
                        }
                        directories.retain(|pair| {
                            let (src, dst) = (&self.items[pair.src], &self.items[pair.dst]);
                            !src.emitted
                                && !dst.emitted
                                && src
                                    .change
                                    .relation()
                                    .zip(dst.change.relation())
                                    .is_none_or(|(src, dst)| {
                                        let relation_id = |relation| match relation {
                                            Relation::Parent(id) | Relation::ChildOfParent(id) => id,
                                        };
                                        let (src, dst) = (relation_id(src), relation_id(dst));
                                        // An exact tree must not contradict the parent choices of renamed files.
                                        file_parents.get(&src).is_none_or(|other| *other == Some(dst))
                                            && file_parents.get(&dst).is_none_or(|other| *other == Some(src))
                                    })
                        });
                        cancelled = self.emit_pairs(&directories, &mut cb)?.is_break();
                    }
                }
            }
            if !cancelled {
                self.match_renamed_directories(&mut |destination, source| {
                    let action = cb(destination, source);
                    cancelled |= action.is_break();
                    action
                })?;
            }
            if cancelled {
                self.items.clear();
                self.path_backing.clear();
                return Ok(out);
            }
        }

        self.items.retain(|item| !item.emitted);
        self.items
            .sort_by(|a, b| a.location(&self.path_backing).cmp(b.location(&self.path_backing)));
        for item in self.items.drain(..) {
            if cb(
                visit::Destination {
                    location: item.location(&self.path_backing),
                    change: item.change,
                },
                None,
            )
            .is_break()
            {
                break;
            }
        }
        self.path_backing.clear();
        Ok(out)
    }
}

impl<T: Change> Tracker<T> {
    /// Sort `items` primarily by their id, so identity-based lookups can be found quickly by
    /// partitioning (see `exact_source`). Same-id items are then ordered by location, change-kind,
    /// relation and entry-mode - a total order over each item's observable identity - so that
    /// matching is deterministic and independent of the order in which changes were pushed, which
    /// the parallel dirwalk and index-traversal threads leave nondeterministic
    /// (see <https://github.com/GitoxideLabs/gitoxide/issues/1832>). Any items that still compare
    /// equal are identical in every observable field and are thus interchangeable for matching.
    fn sort_items_by_id_and_location(&mut self) {
        self.items.sort_by(|a, b| {
            a.change
                .id()
                .cmp(b.change.id())
                .then_with(|| a.location(&self.path_backing).cmp(b.location(&self.path_backing)))
                .then_with(|| a.change.kind().cmp(&b.change.kind()))
                .then_with(|| a.change.relation().cmp(&b.change.relation()))
                .then_with(|| a.change.entry_mode().cmp(&b.change.entry_mode()))
        });
    }

    fn path_order(&self, indices: &mut [usize]) {
        indices.sort_by(|&a, &b| {
            self.items[a]
                .location(&self.path_backing)
                .cmp(self.items[b].location(&self.path_backing))
        });
    }

    fn track_item(&self, item: &Item<T>) -> bool {
        self.rewrites.track_empty
            || matches!(item.change.relation(), Some(Relation::ChildOfParent(_)))
            || item.change.id() != gix_hash::ObjectId::empty_blob(item.change.id().kind())
    }

    fn can_match_directories_first(&self) -> bool {
        if self.rewrites.copies.is_some() {
            return false;
        }
        // Directory children can bypass Git's matching only when every exact file pair is unambiguous.
        // Copy detection must also consider later destinations before deciding which use is a rename.
        let mut previous_object_id = None;
        let (mut sources, mut destinations) = (0, 0);
        let mut has_parent = false;
        for item in &self.items {
            if item.change.entry_mode().is_tree() {
                has_parent |= matches!(item.change.relation(), Some(Relation::Parent(_)));
                continue;
            }
            let object_id = item.change.id();
            if previous_object_id != Some(object_id) {
                previous_object_id = Some(object_id);
                (sources, destinations) = (0, 0);
            }
            match item.change.kind() {
                ChangeKind::Deletion => sources += 1,
                ChangeKind::Addition => destinations += 1,
                ChangeKind::Modification => {}
            }
            if sources > 1 || destinations > 1 {
                return false;
            }
        }
        has_parent
    }

    fn match_identical_directories(
        &mut self,
        cb: &mut impl FnMut(visit::Destination<'_, T>, Option<visit::Source<'_, T>>) -> Action,
    ) -> Result<Action> {
        let sources: Vec<_> = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| {
                (item.change.kind() == ChangeKind::Deletion
                    && matches!(item.change.relation(), Some(Relation::Parent(_))))
                .then_some(idx)
            })
            .collect();
        let mut destinations: Vec<_> = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| {
                (item.change.kind() == ChangeKind::Addition
                    && matches!(item.change.relation(), Some(Relation::Parent(_))))
                .then_some(idx)
            })
            .collect();
        self.path_order(&mut destinations);
        if sources.is_empty() || destinations.is_empty() {
            return Ok(std::ops::ControlFlow::Continue(()));
        }
        let mut used: Vec<_> = self.items.iter().map(|item| item.emitted).collect();
        for dst in destinations {
            if self.items[dst].emitted {
                continue;
            }
            if let Some(src) = self.exact_source(&sources, dst, &used, false, false) {
                if self.emit_pair(src, dst, SourceKind::Rename, None, true, cb)?.is_break() {
                    return Ok(std::ops::ControlFlow::Break(()));
                }
                used[src] = true;
            }
        }
        Ok(std::ops::ControlFlow::Continue(()))
    }

    fn exact_source(
        &self,
        sources_by_id: &[usize],
        dst: usize,
        used: &[bool],
        copies: bool,
        all_sources_used: bool,
    ) -> Option<usize> {
        let item = &self.items[dst];
        let start = sources_by_id.partition_point(|&idx| self.items[idx].change.id() < item.change.id());
        let end = sources_by_id.partition_point(|&idx| self.items[idx].change.id() <= item.change.id());
        let mut best = None;
        let mut checked = 0;
        for &src in &sources_by_id[start..end] {
            let candidate = &self.items[src];
            if (used[src] && !copies) || !candidate.entry_mode_compatible(item.change.entry_mode()) {
                continue;
            }
            let score = u8::from(!used[src])
                + u8::from(
                    filename(candidate.location(&self.path_backing)) == filename(item.location(&self.path_backing)),
                );
            if best.is_none_or(|(_, best_score)| score > best_score) {
                best = Some((src, score));
                if score == if all_sources_used { 1 } else { 2 } {
                    break;
                }
            }
            checked += 1;
            if checked == 100 {
                break;
            }
        }
        best.map(|(src, _)| src)
    }

    fn find_file_renames(
        &self,
        out: &mut Outcome,
        diff_cache: &mut crate::blob::Platform,
        objects: &impl gix_object::FindObjectOrHeader,
    ) -> Result<Vec<Candidate>> {
        let copies = self.rewrites.copies.is_some();
        let mut sources: Vec<_> = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| {
                (self.track_item(item)
                    && match item.change.kind() {
                        ChangeKind::Deletion => !item.emitted || copies,
                        ChangeKind::Modification => copies,
                        ChangeKind::Addition => false,
                    })
                .then_some(idx)
            })
            .collect();
        let mut destinations: Vec<_> = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| {
                (!item.emitted && item.change.kind() == ChangeKind::Addition && self.track_item(item)).then_some(idx)
            })
            .collect();
        self.path_order(&mut destinations);
        let mut used: Vec<_> = self
            .items
            .iter()
            .map(|item| item.emitted || item.change.kind() == ChangeKind::Modification)
            .collect();
        let mut matches = vec![None; self.items.len()];
        let mut unused_sources = sources.iter().filter(|&&src| !used[src]).count();
        for &dst in &destinations {
            if let Some(src) = self.exact_source(&sources, dst, &used, copies, unused_sources == 0) {
                matches[dst] = Some(Candidate {
                    src,
                    dst,
                    score: MAX_SCORE,
                    name_score: false,
                    exact: true,
                    eligible: true,
                });
                if !used[src] {
                    unused_sources -= 1;
                    used[src] = true;
                }
            }
        }
        let rename_score = minimum_score(self.rewrites.percentage);
        let copy_score = self
            .rewrites
            .copies
            .map_or(MAX_SCORE, |copies| minimum_score(copies.percentage));
        if rename_score < MAX_SCORE || copy_score < MAX_SCORE {
            sources.retain(|&src| !self.items[src].change.entry_mode().is_tree());
            self.path_order(&mut sources);
            let mut signatures: Vec<_> = (0..self.items.len()).map(|_| SimilarityData::default()).collect();
            if !copies {
                sources.retain(|&src| !used[src]);
                // Git reserves unique basename pairs at a stricter threshold before checking the matrix limit.
                let source_names = self.unique_basenames(sources.iter().copied());
                let destination_names = self.unique_basenames(
                    destinations
                        .iter()
                        .copied()
                        .filter(|&dst| matches[dst].is_none() && !self.items[dst].change.entry_mode().is_tree()),
                );
                let basename_score = rename_score + (MAX_SCORE - rename_score) / 2;
                for &src in &sources {
                    let name = filename(self.items[src].location(&self.path_backing));
                    if source_names.get(name) != Some(&Some(src)) {
                        continue;
                    }
                    if let Some(&Some(dst)) = destination_names.get(name)
                        && let Some(score) = self.estimate_similarity(
                            src,
                            dst,
                            basename_score,
                            &mut signatures,
                            out,
                            diff_cache,
                            objects,
                        )?
                        && score >= basename_score
                    {
                        matches[dst] = Some(Candidate {
                            src,
                            dst,
                            score,
                            name_score: true,
                            exact: false,
                            eligible: true,
                        });
                        used[src] = true;
                    }
                }
                sources.retain(|&src| !used[src]);
            }
            sources.retain(|&src| {
                if used[src] {
                    copy_score < MAX_SCORE
                } else {
                    rename_score < MAX_SCORE
                }
            });
            let destinations: Vec<_> = destinations
                .iter()
                .copied()
                .filter(|&dst| matches[dst].is_none() && !self.items[dst].change.entry_mode().is_tree())
                .collect();
            let mut limited =
                self.rewrites.limit != 0 && limit_is_exceeded(sources.len(), destinations.len(), self.rewrites.limit);
            if limited {
                let permutations = sources.len().saturating_mul(destinations.len());
                if copies {
                    out.num_similarity_checks_skipped_for_copy_tracking_due_to_limit = permutations;
                    if self
                        .rewrites
                        .copies
                        .is_some_and(|copies| copies.source == CopySource::FromSetOfModifiedFilesAndAllSources)
                    {
                        let modified: Vec<_> = sources
                            .iter()
                            .copied()
                            .filter(|&src| {
                                !(self.items[src].emitted && self.items[src].change.kind() == ChangeKind::Modification)
                            })
                            .collect();
                        if !limit_is_exceeded(modified.len(), destinations.len(), self.rewrites.limit) {
                            sources = modified;
                            limited = false;
                        }
                    }
                } else {
                    out.num_similarity_checks_skipped_for_rename_tracking_due_to_limit = permutations;
                }
            }
            if !limited {
                let mut matrix = Vec::with_capacity(destinations.len().saturating_mul(4));
                for &dst in &destinations {
                    let mut best = [None; 4];
                    for &src in &sources {
                        let threshold = if used[src] {
                            copy_score
                        } else if copies {
                            rename_score.min(copy_score)
                        } else {
                            rename_score
                        };
                        let score =
                            self.estimate_similarity(src, dst, threshold, &mut signatures, out, diff_cache, objects)?;
                        // Git inserts even rejected (zero-score) and below-threshold comparisons. They affect
                        // which slots later candidates replace, and thus the stable ordering of equal scores.
                        let candidate = Candidate {
                            src,
                            dst,
                            score: score.unwrap_or_default(),
                            exact: false,
                            eligible: score.is_some_and(|score| score >= threshold),
                            name_score: filename(self.items[src].location(&self.path_backing))
                                == filename(self.items[dst].location(&self.path_backing)),
                        };
                        let mut worst = 0;
                        for idx in 1..best.len() {
                            if candidate_rank(best[idx]) < candidate_rank(best[worst]) {
                                worst = idx;
                            }
                        }
                        if candidate_rank(Some(candidate)) > candidate_rank(best[worst]) {
                            best[worst] = Some(candidate);
                        }
                    }
                    matrix.extend(best.into_iter().flatten().filter(|candidate| candidate.eligible));
                }
                matrix.sort_by_key(|candidate| std::cmp::Reverse((candidate.score, candidate.name_score)));
                for candidate in &matrix {
                    if candidate.score >= rename_score && !used[candidate.src] && matches[candidate.dst].is_none() {
                        matches[candidate.dst] = Some(*candidate);
                        used[candidate.src] = true;
                    }
                }
                if copies {
                    for candidate in matrix {
                        if candidate.score >= copy_score
                            && (used[candidate.src] || candidate.score >= rename_score)
                            && matches[candidate.dst].is_none()
                        {
                            matches[candidate.dst] = Some(candidate);
                            used[candidate.src] = true;
                        }
                    }
                }
            }
        }
        Ok(destinations.into_iter().filter_map(|dst| matches[dst]).collect())
    }

    fn unique_basenames(
        &self,
        indices: impl Iterator<Item = usize>,
    ) -> std::collections::HashMap<&BStr, Option<usize>> {
        let mut names = std::collections::HashMap::new();
        for idx in indices {
            names
                .entry(filename(self.items[idx].location(&self.path_backing)))
                .and_modify(|index| *index = None)
                .or_insert(Some(idx));
        }
        names
    }

    #[expect(clippy::too_many_arguments)]
    fn estimate_similarity(
        &self,
        src: usize,
        dst: usize,
        minimum_score: u16,
        signatures: &mut [SimilarityData],
        out: &mut Outcome,
        diff_cache: &mut crate::blob::Platform,
        objects: &impl gix_object::FindObjectOrHeader,
    ) -> Result<Option<u16>> {
        if !self.items[src].change.entry_mode().is_blob() || !self.items[dst].change.entry_mode().is_blob() {
            return Ok(None);
        }
        out.num_similarity_checks += 1;
        for (idx, kind) in [(src, ResourceKind::OldOrSource), (dst, ResourceKind::NewOrDestination)] {
            let cached = &mut signatures[idx];
            if cached.size.is_none() && cached.signature.is_none() {
                let item = &self.items[idx];
                cached.size = diff_cache.similarity_size(item.change.id(), kind, objects)?;
                // Worktree sizes are only comparable after applying clean filters.
                if cached.size.is_none() {
                    cached.signature = Some(diff_cache.similarity_signature(
                        item.change.id(),
                        item.change.entry_mode().kind(),
                        item.location(&self.path_backing),
                        kind,
                        objects,
                    )?);
                    cached.size = cached
                        .signature
                        .as_ref()
                        .and_then(Option::as_ref)
                        .map(|s| s.size() as u64);
                }
            }
        }
        let (Some(source_size), Some(destination_size)) = (signatures[src].size, signatures[dst].size) else {
            return Ok(None);
        };
        let max_size = source_size.max(destination_size) as u128;
        let delta = source_size.abs_diff(destination_size) as u128;
        if max_size * u128::from(MAX_SCORE - minimum_score) < delta * u128::from(MAX_SCORE) {
            return Ok(None);
        }
        for (idx, kind) in [(src, ResourceKind::OldOrSource), (dst, ResourceKind::NewOrDestination)] {
            if signatures[idx].signature.is_none() {
                let item = &self.items[idx];
                signatures[idx].signature = Some(diff_cache.similarity_signature(
                    item.change.id(),
                    item.change.entry_mode().kind(),
                    item.location(&self.path_backing),
                    kind,
                    objects,
                )?);
            }
        }
        let (Some(source), Some(destination)) = (
            signatures[src].signature.as_ref().and_then(Option::as_ref),
            signatures[dst].signature.as_ref().and_then(Option::as_ref),
        ) else {
            return Ok(None);
        };
        Ok(Some(source.similarity(destination)))
    }

    fn emit_pairs(
        &mut self,
        pairs: &[Candidate],
        cb: &mut impl FnMut(visit::Destination<'_, T>, Option<visit::Source<'_, T>>) -> Action,
    ) -> Result<Action> {
        if pairs.is_empty() {
            return Ok(std::ops::ControlFlow::Continue(()));
        }
        // Git labels the last destination of a removed source a rename, and earlier uses copies.
        let mut last_destination = vec![None; self.items.len()];
        for pair in pairs {
            if !self.items[pair.src].emitted && pair.score >= minimum_score(self.rewrites.percentage) {
                last_destination[pair.src] = Some(pair.dst);
            }
        }
        for pair in pairs {
            let kind = if self.items[pair.src].change.kind() == ChangeKind::Deletion
                && last_destination[pair.src] == Some(pair.dst)
            {
                SourceKind::Rename
            } else {
                SourceKind::Copy
            };
            let similarity = (!pair.exact).then_some(Similarity(pair.score));
            if self
                .emit_pair(pair.src, pair.dst, kind, similarity, false, cb)?
                .is_break()
            {
                return Ok(std::ops::ControlFlow::Break(()));
            }
        }
        Ok(std::ops::ControlFlow::Continue(()))
    }

    fn emit_pair(
        &mut self,
        src: usize,
        dst: usize,
        kind: SourceKind,
        similarity: Option<Similarity>,
        match_children: bool,
        cb: &mut impl FnMut(visit::Destination<'_, T>, Option<visit::Source<'_, T>>) -> Action,
    ) -> Result<Action> {
        let source = &self.items[src];
        let destination = &self.items[dst];
        let relations = (kind == SourceKind::Rename)
            .then(|| source.change.relation().zip(destination.change.relation()))
            .flatten();
        let action = cb(
            visit::Destination {
                change: destination.change.clone(),
                location: destination.location(&self.path_backing),
            },
            Some(visit::Source {
                entry_mode: source.change.entry_mode(),
                id: source.change.id().to_owned(),
                kind,
                location: source.location(&self.path_backing),
                change: &source.change,
                similarity,
            }),
        );
        self.items[dst].emitted = true;
        if self.items[src].change.kind() == ChangeKind::Deletion {
            self.items[src].emitted = true;
        }
        if action.is_break() {
            return Ok(action);
        }
        match relations {
            Some((Relation::Parent(src), Relation::Parent(dst))) if match_children && similarity.is_none() => {
                return self.emit_child_renames_matching_identity(cb, kind, src, dst);
            }
            Some((Relation::ChildOfParent(src_parent), Relation::ChildOfParent(dst_parent))) => {
                self.child_renames
                    .entry((src_parent, dst_parent))
                    .or_default()
                    .push((src, dst));
            }
            _ => {}
        }
        Ok(action)
    }

    /// Emit the children of `src_parent_id` and `dst_parent_id` as pairs of exact matches, which are assumed
    /// as `src` and `dst` were an exact match (so all children have to match exactly).
    /// Note that we intentionally do not record them as their parents will be emitted, too.
    fn emit_child_renames_matching_identity(
        &mut self,
        cb: &mut impl FnMut(visit::Destination<'_, T>, Option<visit::Source<'_, T>>) -> Action,
        kind: visit::SourceKind,
        src_parent_id: ChangeId,
        dst_parent_id: ChangeId,
    ) -> Result<Action> {
        debug_assert_ne!(
            src_parent_id, dst_parent_id,
            "src and destination directories must be distinct"
        );
        let (mut src_items, mut dst_items) = (Vec::with_capacity(1), Vec::with_capacity(1));
        for item in self.items.iter_mut().filter(|item| !item.emitted) {
            match item.change.relation() {
                Some(Relation::ChildOfParent(id)) if id == src_parent_id => {
                    src_items.push((item.change.id().to_owned(), item));
                }
                Some(Relation::ChildOfParent(id)) if id == dst_parent_id => {
                    dst_items.push((item.change.id().to_owned(), item));
                }
                _ => continue,
            }
        }

        for ((src_id, src_item), (dst_id, dst_item)) in src_items.into_iter().zip(dst_items) {
            // Since the parent items are already identical by ID, we know that the children will also match, we just
            // double-check to still have a chance to be correct in case some of that goes wrong.
            if src_id == dst_id
                && filename(src_item.location(&self.path_backing)) == filename(dst_item.location(&self.path_backing))
            {
                let entry_mode = src_item.change.entry_mode();
                let location = src_item.location(&self.path_backing);
                let src = visit::Source {
                    entry_mode,
                    id: src_id,
                    kind,
                    location,
                    change: &src_item.change,
                    similarity: None,
                };
                let location = dst_item.location(&self.path_backing);
                let change = dst_item.change.clone();
                let dst = visit::Destination { change, location };
                let res = cb(dst, Some(src));

                src_item.emitted = true;
                dst_item.emitted = true;

                if res.is_break() {
                    return Ok(res);
                }
            } else {
                gix_trace::warn!(
                    "Children of parents with change-id {src_parent_id} and {dst_parent_id} were not equal, even though their parents claimed to be"
                );
                break;
            }
        }
        Ok(std::ops::ControlFlow::Continue(()))
    }

    /// Find directories with relation id that haven't been emitted yet and store them for lookup.
    /// Then use the previously stored emitted renames with relation id to learn which directories they 'link'
    /// and emit them, too.
    /// Note that this works whenever top-level directories are renamed because they are always added and deleted,
    /// and we only match those. One rewrite preserving the relative directory path is enough.
    fn match_renamed_directories(
        &mut self,
        cb: &mut impl FnMut(visit::Destination<'_, T>, Option<visit::Source<'_, T>>) -> Action,
    ) -> Result {
        /// Per looked-up relation id, a cursor past the emitted directories and the indices of all its directories,
        /// in item order.
        type DirectoriesByRelationId = std::collections::HashMap<ChangeId, (usize, Vec<usize>)>;
        fn unemitted_directory_matching_relation_id<T: Change>(
            items: &[Item<T>],
            directories: &mut DirectoriesByRelationId,
            child_id: ChangeId,
        ) -> Option<usize> {
            let (cursor, indices) = directories.get_mut(&child_id)?;
            while let Some(&idx) = indices.get(*cursor) {
                if !items[idx].emitted {
                    return Some(idx);
                }
                *cursor += 1;
            }
            None
        }

        if self.child_renames.is_empty() {
            return Ok(());
        }
        // Scanning all items for each lookup would take time proportional to the number of child renames
        // times the number of items, so index the directories once instead, but only those that are looked up.
        // Items are only ever marked as emitted, never unmarked, so a cursor that skips emitted directories
        // for good still finds the first unemitted one in item order, which is what a scan finds.
        let mut directories: DirectoriesByRelationId = self
            .child_renames
            .keys()
            .flat_map(|&(deleted_child_id, added_child_id)| [deleted_child_id, added_child_id])
            .map(|id| (id, Default::default()))
            .collect();
        for (idx, item) in self.items.iter().enumerate() {
            if let Some(Relation::Parent(id)) = item.change.relation()
                && let Some((_, indices)) = directories.get_mut(&id)
            {
                indices.push(idx);
            }
        }
        for ((deleted_child_id, added_child_id), children) in &self.child_renames {
            let Some(src_idx) =
                unemitted_directory_matching_relation_id(&self.items, &mut directories, *deleted_child_id)
            else {
                continue;
            };
            let Some(dst_idx) =
                unemitted_directory_matching_relation_id(&self.items, &mut directories, *added_child_id)
            else {
                // This could go wrong in case there are mismatches, so be defensive here.
                // But generally, we'd expect the destination item to exist.
                continue;
            };

            let (src_item, dst_item) = (&self.items[src_idx], &self.items[dst_idx]);
            if !children.iter().any(|&(src_child_idx, dst_child_idx)| {
                let relative_directory = |child_idx: usize, parent: &Item<T>| {
                    let child = &self.items[child_idx];
                    child
                        .location(&self.path_backing)
                        .strip_prefix(parent.location(&self.path_backing).as_bytes())
                        .and_then(|path| path.strip_prefix(b"/"))
                        .map(|path| {
                            if child.change.entry_mode().is_tree() {
                                path
                            } else {
                                // File names may change without renaming their containing directory.
                                &path[..path.rfind_byte(b'/').unwrap_or(0)]
                            }
                        })
                };
                let src_relative = relative_directory(src_child_idx, src_item);
                let dst_relative = relative_directory(dst_child_idx, dst_item);
                src_relative.is_some() && src_relative == dst_relative
            }) {
                continue;
            }
            let entry_mode = src_item.change.entry_mode();
            let location = src_item.location(&self.path_backing);
            let src = visit::Source {
                entry_mode,
                id: src_item.change.id().to_owned(),
                kind: SourceKind::Rename,
                location,
                change: &src_item.change,
                similarity: None,
            };
            let location = dst_item.location(&self.path_backing);
            let change = dst_item.change.clone();
            let dst = visit::Destination { change, location };
            let res = cb(dst, Some(src));

            self.items[src_idx].emitted = true;
            self.items[dst_idx].emitted = true;

            if res.is_break() {
                return Ok(());
            }
        }
        Ok(())
    }
}

fn filename(path: &BStr) -> &BStr {
    path.rfind_byte(b'/').map_or(path, |idx| path[idx + 1..].as_bstr())
}

fn limit_is_exceeded(num_src: usize, num_dst: usize, limit: usize) -> bool {
    // Saturating both products can hide an exceeded limit; u128 holds them exactly on supported targets.
    let limit = limit as u128;
    (num_src as u128) * (num_dst as u128) > limit * limit
}

fn minimum_score(percentage: Option<f32>) -> u16 {
    (percentage.unwrap_or(1.0).clamp(0.0, 1.0) * f32::from(MAX_SCORE)) as u16
}

#[derive(Default)]
struct SimilarityData {
    size: Option<u64>,
    signature: Option<Option<Signature>>,
}

#[derive(Clone, Copy)]
struct Candidate {
    src: usize,
    dst: usize,
    score: u16,
    name_score: bool,
    exact: bool,
    /// Rejected and below-threshold comparisons occupy matrix slots without becoming matches.
    eligible: bool,
}

fn candidate_rank(candidate: Option<Candidate>) -> Option<(u16, bool)> {
    candidate.map(|candidate| (candidate.score, candidate.name_score))
}

#[cfg(test)]
mod limit_is_exceeded {
    use super::limit_is_exceeded;

    #[test]
    fn overflowing_products_are_compared_without_saturation() {
        for limit in [70_000, usize::MAX / 2] {
            assert!(
                limit_is_exceeded(limit + 1, limit + 1, limit),
                "candidate counts just above the limit must be rejected even when both products overflow usize"
            );
            assert!(
                !limit_is_exceeded(limit, limit, limit),
                "a product equal to the squared limit is allowed"
            );
            assert!(
                !limit_is_exceeded(limit - 1, limit, limit),
                "a product below the squared limit is allowed even when both products overflow usize"
            );
        }
    }

    #[test]
    fn maximum_counts_and_unbalanced_candidates() {
        assert!(
            !limit_is_exceeded(usize::MAX, usize::MAX, usize::MAX),
            "squaring the largest supported count must not overflow the comparison"
        );
        assert!(
            !limit_is_exceeded(0, usize::MAX, 1),
            "without sources there are no comparisons to limit"
        );
        assert!(
            limit_is_exceeded(usize::MAX, 1, 1),
            "the product, not the smaller candidate count, determines whether the limit is exceeded"
        );
    }
}

#[cfg(test)]
mod match_renamed_directories {
    use bstr::{BString, ByteSlice};
    use gix_error::TestResult;
    use gix_hash::ObjectId;
    use gix_object::tree::{EntryKind, EntryMode};

    use super::{Change as _, Item, visit};
    use crate::{
        Rewrites,
        rewrites::Tracker,
        tree::visit::{Action, Change, ChangeId, Relation},
    };

    /// A deterministic xorshift64* sequence, so the cases are reproducible without a dependency.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
        }

        fn below(&mut self, n: u64) -> u64 {
            self.next() % n.max(1)
        }
    }

    /// The lookup as it was before directories were indexed: a scan of all items per child rename.
    /// The rest is the same as in [`Tracker::match_renamed_directories()`], but directory pairs whose
    /// children don't agree on their relative directory are counted in `rejected`.
    fn scan_all_items(
        tracker: &mut Tracker<Change>,
        cb: &mut impl FnMut(visit::Destination<'_, Change>, Option<visit::Source<'_, Change>>) -> Action,
        rejected: &mut usize,
    ) {
        fn unemitted_directory_matching_relation_id(items: &[Item<Change>], child_id: ChangeId) -> Option<usize> {
            items.iter().position(|i| {
                !i.emitted && matches!(i.change.relation(), Some(Relation::Parent(pid)) if pid == child_id)
            })
        }
        for ((deleted_child_id, added_child_id), children) in &tracker.child_renames {
            let Some(src_idx) = unemitted_directory_matching_relation_id(&tracker.items, *deleted_child_id) else {
                continue;
            };
            let Some(dst_idx) = unemitted_directory_matching_relation_id(&tracker.items, *added_child_id) else {
                continue;
            };

            let (src_item, dst_item) = (&tracker.items[src_idx], &tracker.items[dst_idx]);
            if !children.iter().any(|&(src_child_idx, dst_child_idx)| {
                let relative_directory = |child_idx: usize, parent: &Item<Change>| {
                    let child = &tracker.items[child_idx];
                    child
                        .location(&tracker.path_backing)
                        .strip_prefix(parent.location(&tracker.path_backing).as_bytes())
                        .and_then(|path| path.strip_prefix(b"/"))
                        .map(|path| {
                            if child.change.entry_mode().is_tree() {
                                path
                            } else {
                                &path[..path.rfind_byte(b'/').unwrap_or(0)]
                            }
                        })
                };
                let src_relative = relative_directory(src_child_idx, src_item);
                let dst_relative = relative_directory(dst_child_idx, dst_item);
                src_relative.is_some() && src_relative == dst_relative
            }) {
                *rejected += 1;
                continue;
            }
            let src = visit::Source {
                entry_mode: src_item.change.entry_mode(),
                id: src_item.change.id().to_owned(),
                kind: visit::SourceKind::Rename,
                location: src_item.location(&tracker.path_backing),
                change: &src_item.change,
                similarity: None,
            };
            let dst = visit::Destination {
                change: dst_item.change.clone(),
                location: dst_item.location(&tracker.path_backing),
            };
            let res = cb(dst, Some(src));

            tracker.items[src_idx].emitted = true;
            tracker.items[dst_idx].emitted = true;

            if res.is_break() {
                return;
            }
        }
    }

    /// Path components are drawn from a small set, so that paths repeat, and children
    /// sometimes do and sometimes don't agree on their directory relative to their parent.
    const NAMES: &[&str] = &["a", "b"];

    /// Create `num_items` items whose relations use `num_ids` ids, so several directories can share one,
    /// with about `emitted_per_mille` of them emitted already.
    /// Then record up to `num_pairs` child renames, with random child pairs, whose ids may also name no directory
    /// or the same id for source and destination.
    fn random_tracker(
        rng: &mut Rng,
        num_items: usize,
        num_ids: u64,
        num_pairs: usize,
        emitted_per_mille: u64,
    ) -> Tracker<Change> {
        let mut tracker = Tracker::new(Rewrites::default());
        for _ in 0..num_items {
            let relation = match rng.below(3) {
                0 => Some(Relation::Parent(rng.below(num_ids) as ChangeId)),
                1 => Some(Relation::ChildOfParent(rng.below(num_ids) as ChangeId)),
                _ => None,
            };
            let is_tree = matches!(relation, Some(Relation::Parent(_))) || rng.below(4) == 0;
            let entry_mode: EntryMode = if is_tree { EntryKind::Tree } else { EntryKind::Blob }.into();
            let mut oid = [0u8; 20];
            oid[..8].copy_from_slice(&rng.next().to_be_bytes());
            let oid = ObjectId::from_bytes_or_panic(&oid);
            let change = if rng.below(2) == 0 {
                Change::Addition {
                    entry_mode,
                    oid,
                    relation,
                }
            } else {
                Change::Deletion {
                    entry_mode,
                    oid,
                    relation,
                }
            };
            // Directories are top-level, so other items are below them more often.
            let depth = if matches!(relation, Some(Relation::Parent(_))) {
                1
            } else {
                [1, 2, 2, 3][rng.below(4) as usize]
            };
            let path = (0..depth)
                .map(|_| NAMES[rng.below(NAMES.len() as u64) as usize])
                .collect::<Vec<_>>()
                .join("/");
            let start = tracker.path_backing.len();
            tracker.path_backing.extend_from_slice(path.as_bytes());
            tracker.items.push(Item {
                change,
                path: start..tracker.path_backing.len(),
                emitted: rng.below(1000) < emitted_per_mille,
            });
        }
        let id_range = num_ids + num_ids / 4 + 1;
        for _ in 0..num_pairs {
            let deleted = rng.below(id_range) as ChangeId;
            let added = if rng.below(20) == 0 {
                deleted
            } else {
                rng.below(id_range) as ChangeId
            };
            let num_children = 1 + rng.below(6);
            let children = tracker.child_renames.entry((deleted, added)).or_default();
            for _ in 0..num_children {
                children.push((
                    rng.below(num_items as u64) as usize,
                    rng.below(num_items as u64) as usize,
                ));
            }
        }
        tracker
    }

    fn clone_tracker(tracker: &Tracker<Change>) -> Tracker<Change> {
        Tracker {
            items: tracker
                .items
                .iter()
                .map(|item| Item {
                    change: item.change.clone(),
                    path: item.path.clone(),
                    emitted: item.emitted,
                })
                .collect(),
            path_backing: tracker.path_backing.clone(),
            rewrites: tracker.rewrites,
            child_renames: tracker.child_renames.clone(),
        }
    }

    type Emitted = Vec<(BString, ObjectId, EntryMode, Change, BString, Change)>;

    /// Emit renamed directories with the indexed lookup if `indexed` is `true`, or by scanning all items otherwise,
    /// and stop after `stop_after` emissions if set.
    /// Return all emitted pairs and the amount of directory pairs rejected by the scan.
    fn emit(tracker: &mut Tracker<Change>, indexed: bool, stop_after: Option<usize>) -> TestResult<(Emitted, usize)> {
        let mut out = Emitted::new();
        let mut cb = |dst: visit::Destination<'_, Change>, src: Option<visit::Source<'_, Change>>| {
            let src = src.expect("renamed directories always have a source");
            out.push((
                src.location.to_owned(),
                src.id,
                src.entry_mode,
                src.change.clone(),
                dst.location.to_owned(),
                dst.change,
            ));
            if stop_after.is_some_and(|n| out.len() >= n) {
                Action::Break(())
            } else {
                Action::Continue(())
            }
        };
        let mut rejected = 0;
        if indexed {
            tracker.match_renamed_directories(&mut cb)?;
        } else {
            scan_all_items(tracker, &mut cb, &mut rejected);
        }
        Ok((out, rejected))
    }

    #[test]
    fn indexed_lookups_emit_the_same_as_scanning_all_items() -> TestResult {
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        let (mut cases_with_renames, mut rejected_pairs) = (0, 0);
        for case in 0..3000 {
            let num_items = 1 + rng.below(if case % 10 == 0 { 2000 } else { 60 }) as usize;
            let num_ids = 1 + rng.below((num_items as u64 / 3).clamp(1, 40));
            let num_pairs = rng.below(80) as usize;
            let emitted_per_mille = [0, 100, 500, 1000][rng.below(4) as usize];
            let tracker = random_tracker(&mut rng, num_items, num_ids, num_pairs, emitted_per_mille);
            let stop_after = (rng.below(4) == 0).then(|| 1 + rng.below(5) as usize);

            let (mut scanned, mut indexed) = (clone_tracker(&tracker), clone_tracker(&tracker));
            let (expected, rejected) = emit(&mut scanned, false, stop_after)?;
            let (actual, _) = emit(&mut indexed, true, stop_after)?;
            assert_eq!(
                actual, expected,
                "case {case}: the same directory pairs are emitted in the same order"
            );
            let emitted = |t: &Tracker<Change>| t.items.iter().map(|item| item.emitted).collect::<Vec<_>>();
            assert_eq!(
                emitted(&indexed),
                emitted(&scanned),
                "case {case}: the same items are marked as emitted"
            );
            cases_with_renames += usize::from(!expected.is_empty());
            rejected_pairs += rejected;
        }
        assert!(
            cases_with_renames > 1000,
            "many cases emit renamed directories, got {cases_with_renames}"
        );
        assert!(
            rejected_pairs > 1000,
            "many directory pairs are rejected by their children and stay unemitted, got {rejected_pairs}"
        );
        Ok(())
    }
}
