//! Rename and copy detection following Git's `diffcore-rename.c`.
//!
//! Exact matches precede a unique-basename pass and a globally ranked, four-candidates-per-destination
//! similarity matrix. Only leaf entries participate; tree diffs infer directory renames from
//! the resulting file matches. Git's merge-specific directory heuristics and break detection
//! are not part of this tracker.

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
    tree::visit::Action,
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
    /// conversions. `None` means the hash is unavailable, and never establishes identity.
    /// Such blobs can still match by similarity if their worktree content is available through the diff platform.
    /// Implementations should return `None` for null IDs; the tracker also guards against `Some(null)`.
    /// Modifications return the previous id, as copies originate from the source state.
    fn id(&self) -> Option<&gix_hash::oid>;
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
            (Blob | BlobExecutable, Blob | BlobExecutable) | (Link, Link) | (Commit, Commit)
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
        /// The hash of the state of the source as seen in the object database, or `None` if unavailable.
        pub id: Option<gix_hash::ObjectId>,
        /// Further specify what kind of source this is.
        pub kind: SourceKind,
        /// The repository-relative location of this entry.
        pub location: &'a BStr,
        /// The change that was registered as source.
        pub change: &'a T,
        /// The byte-span score of an inexact match, including binary files.
        /// `None` denotes an exact identity match.
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

        if change.entry_mode().is_tree() {
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
    /// Tree entries are returned by [`Self::try_push_change()`] for the caller to handle.
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
                            item.change.id().filter(|id| !id.is_null()).map(ToOwned::to_owned),
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
                    if sources.insert((
                        location.to_vec(),
                        change.id().filter(|id| !id.is_null()).map(ToOwned::to_owned),
                        change.entry_mode(),
                    )) && self.try_push_change(change, location).is_none()
                    {
                        self.items.last_mut().expect("just pushed").emitted = true;
                    }
                })
                .or_raise(|| {
                    message("Could not obtain exhaustive item set to use as possible sources for copy detection")
                })?;
            }
            self.sort_items_by_id_and_location();
            let pairs = self.find_file_renames(&mut out, diff_cache, objects)?;
            if self.emit_pairs(&pairs, &mut cb).is_break() {
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
    /// and entry-mode - a total order over each item's matching inputs - so that
    /// matching is deterministic and independent of the order in which changes were pushed, which
    /// the parallel dirwalk and index-traversal threads leave nondeterministic
    /// (see <https://github.com/GitoxideLabs/gitoxide/issues/1832>). Items that compare equal
    /// have the same matching inputs.
    fn sort_items_by_id_and_location(&mut self) {
        self.items.sort_by(|a, b| {
            a.change
                .id()
                .cmp(&b.change.id())
                .then_with(|| a.location(&self.path_backing).cmp(b.location(&self.path_backing)))
                .then_with(|| a.change.kind().cmp(&b.change.kind()))
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
            || item
                .change
                .id()
                .is_none_or(|id| id != gix_hash::ObjectId::empty_blob(id.kind()))
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
        let object_id = item.change.id().filter(|id| !id.is_null())?;
        let start = sources_by_id.partition_point(|&idx| self.items[idx].change.id() < Some(object_id));
        let end = sources_by_id.partition_point(|&idx| self.items[idx].change.id() <= Some(object_id));
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
            self.path_order(&mut sources);
            let mut signatures: Vec<_> = (0..self.items.len()).map(|_| SimilarityData::default()).collect();
            if !copies {
                sources.retain(|&src| !used[src]);
                // Git reserves unique basename pairs at a stricter threshold before checking the matrix limit.
                let source_names = self.unique_basenames(sources.iter().copied());
                let destination_names =
                    self.unique_basenames(destinations.iter().copied().filter(|&dst| matches[dst].is_none()));
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
                .filter(|&dst| matches[dst].is_none())
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
        if destination_size == 0 && !self.rewrites.track_empty {
            return Ok(None);
        }
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
        Ok(Some(if source.size() == 0 && destination.size() == 0 {
            MAX_SCORE
        } else {
            source.similarity(destination)
        }))
    }

    fn emit_pairs(
        &mut self,
        pairs: &[Candidate],
        cb: &mut impl FnMut(visit::Destination<'_, T>, Option<visit::Source<'_, T>>) -> Action,
    ) -> Action {
        if pairs.is_empty() {
            return std::ops::ControlFlow::Continue(());
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
            self.emit_pair(pair.src, pair.dst, kind, similarity, cb)?;
        }
        std::ops::ControlFlow::Continue(())
    }

    fn emit_pair(
        &mut self,
        src: usize,
        dst: usize,
        kind: SourceKind,
        similarity: Option<Similarity>,
        cb: &mut impl FnMut(visit::Destination<'_, T>, Option<visit::Source<'_, T>>) -> Action,
    ) -> Action {
        let source = &self.items[src];
        let destination = &self.items[dst];
        let action = cb(
            visit::Destination {
                change: destination.change.clone(),
                location: destination.location(&self.path_backing),
            },
            Some(visit::Source {
                entry_mode: source.change.entry_mode(),
                id: source.change.id().filter(|id| !id.is_null()).map(ToOwned::to_owned),
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
        action
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

fn candidate_rank(candidate: Option<Candidate>) -> Option<(bool, u16, bool)> {
    candidate.map(|candidate| (candidate.eligible, candidate.score, candidate.name_score))
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
