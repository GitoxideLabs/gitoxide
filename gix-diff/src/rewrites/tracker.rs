//! ### Deviation
//!
//! Note that the algorithm implemented here is in many ways different from what `git` does.
//!
//! - it's less sophisticated and doesn't use any ranking of candidates. Instead, it picks the first possible match.
//! - the set used for copy-detection is probably smaller by default.

use std::ops::Range;

use bstr::{BStr, ByteSlice};
use gix_object::tree::{EntryKind, EntryMode};

use crate::rewrites::tracker::visit::SourceKind;
use crate::tree::visit::{Action, ChangeId, Relation};
use crate::{
    blob::{platform::prepare_diff::Operation, DiffLineStats, ResourceKind},
    rewrites::{CopySource, Outcome, Tracker},
    Rewrites,
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
    /// Return more information about the kind of entry affected by this change.
    fn entry_mode(&self) -> EntryMode;
    /// Return the id of the change along with its mode.
    fn id_and_entry_mode(&self) -> (&gix_hash::oid, EntryMode);
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
            (Blob | BlobExecutable, Blob | BlobExecutable) | (Link, Link) | (Tree, Tree)
        )
    }

    fn is_source_for_destination_of(&self, kind: visit::SourceKind, dest_item_mode: EntryMode) -> bool {
        self.entry_mode_compatible(dest_item_mode)
            && match kind {
                visit::SourceKind::Rename => !self.emitted && matches!(self.change.kind(), ChangeKind::Deletion),
                visit::SourceKind::Copy => {
                    matches!(self.change.kind(), ChangeKind::Modification)
                }
            }
    }
}

/// A module with types used in the user-callback in [Tracker::emit()](crate::rewrites::Tracker::emit()).
pub mod visit {
    use bstr::BStr;
    use gix_object::tree::EntryMode;

    use crate::blob::DiffLineStats;

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
        /// If this is a rewrite, indicate how many lines would need to change to turn this source into the destination.
        pub diff: Option<DiffLineStats>,
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

///
pub mod emit {
    /// The error returned by [Tracker::emit()](super::Tracker::emit()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Could not find blob for similarity checking")]
        FindExistingBlob(#[from] gix_object::find::existing_object::Error),
        #[error("Could not obtain exhaustive item set to use as possible sources for copy detection")]
        GetItemsForExhaustiveCopyDetection(#[source] Box<dyn std::error::Error + Send + Sync>),
        #[error(transparent)]
        SetResource(#[from] crate::blob::platform::set_resource::Error),
        #[error(transparent)]
        PrepareDiff(#[from] crate::blob::platform::prepare_diff::Error),
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
        if entry_kind == EntryKind::Commit {
            return Some(change);
        }
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
    /// Note that directories with [relation](Relation) will be emitted if there is a match, along with all their matching
    /// child-items which are similarly bundled as rename.
    ///
    /// `objects` is used to access blob data for similarity checks if required and is taken directly from the object database.
    /// Worktree filters and text conversions will be applied afterwards automatically. Note that object-caching *should not*
    /// be enabled as caching is implemented by `diff_cache`, after all, the blob that's actually diffed is going
    /// through conversion steps.
    ///
    /// `diff_cache` is a way to retain a cache of resources that are prepared for rapid diffing, and it also controls
    /// the diff-algorithm (provided no user-algorithm is set).
    /// Note that we control a few options of `diff_cache` to assure it will ignore external commands.
    /// Note that we do not control how the `diff_cache` converts resources, it's left to the caller to decide
    /// if it should look at what's stored in `git`, or in the working tree, along with all diff-specific conversions.
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
    ) -> Result<Outcome, emit::Error>
    where
        PushSourceTreeFn: FnMut(&mut dyn FnMut(T, &BStr)) -> Result<(), E>,
        E: std::error::Error + Send + Sync + 'static,
    {
        fn is_parent(change: &impl Change) -> bool {
            matches!(change.relation(), Some(Relation::Parent(_)))
        }
        diff_cache.options.skip_internal_diff_if_external_is_configured = false;

        // The point of this is to optimize for identity-based lookups, which should be easy to find
        // by partitioning.
        fn by_id_and_location<T: Change>(a: &Item<T>, b: &Item<T>) -> std::cmp::Ordering {
            a.change
                .id()
                .cmp(b.change.id())
                .then_with(|| a.path.start.cmp(&b.path.start).then(a.path.end.cmp(&b.path.end)))
        }

        let mut out = Outcome {
            options: self.rewrites,
            ..Default::default()
        };
        self.items.sort_by(by_id_and_location);

        // Rewrites by directory (without local changes) can be pruned out quickly,
        // by finding only parents, their counterpart, and then all children can be matched by
        // relationship ID.
        self.match_pairs_of_kind(
            visit::SourceKind::Rename,
            &mut cb,
            None, /* by identity for parents */
            &mut out,
            diff_cache,
            objects,
            Some(is_parent),
        )?;

        self.match_pairs_of_kind(
            visit::SourceKind::Rename,
            &mut cb,
            self.rewrites.percentage,
            &mut out,
            diff_cache,
            objects,
            None,
        )?;

        self.match_renamed_directories(&mut cb)?;

        if let Some(copies) = self.rewrites.copies {
            self.match_pairs_of_kind(
                visit::SourceKind::Copy,
                &mut cb,
                copies.percentage,
                &mut out,
                diff_cache,
                objects,
                None,
            )?;

            match copies.source {
                CopySource::FromSetOfModifiedFiles => {}
                CopySource::FromSetOfModifiedFilesAndAllSources => {
                    push_source_tree(&mut |change, location| {
                        if self.try_push_change(change, location).is_none() {
                            // make sure these aren't viable to be emitted anymore.
                            self.items.last_mut().expect("just pushed").emitted = true;
                        }
                    })
                    .map_err(|err| emit::Error::GetItemsForExhaustiveCopyDetection(Box::new(err)))?;
                    self.items.sort_by(by_id_and_location);

                    self.match_pairs_of_kind(
                        visit::SourceKind::Copy,
                        &mut cb,
                        copies.percentage,
                        &mut out,
                        diff_cache,
                        objects,
                        None,
                    )?;
                }
            }
        }

        self.items
            .sort_by(|a, b| a.location(&self.path_backing).cmp(b.location(&self.path_backing)));
        for item in self.items.drain(..).filter(|item| !item.emitted) {
            if cb(
                visit::Destination {
                    location: item.location(&self.path_backing),
                    change: item.change,
                },
                None,
            ) == Action::Cancel
            {
                break;
            }
        }
        Ok(out)
    }
}

impl<T: Change> Tracker<T> {
    #[allow(clippy::too_many_arguments)]
    fn match_pairs_of_kind(
        &mut self,
        kind: visit::SourceKind,
        cb: &mut impl FnMut(visit::Destination<'_, T>, Option<visit::Source<'_, T>>) -> Action,
        percentage: Option<f32>,
        out: &mut Outcome,
        diff_cache: &mut crate::blob::Platform,
        objects: &impl gix_object::FindObjectOrHeader,
        filter: Option<fn(&T) -> bool>,
    ) -> Result<(), emit::Error> {
        // we try to cheaply reduce the set of possibilities first, before possibly looking more exhaustively.
        let needs_second_pass = !needs_exact_match(percentage);

        // https://github.com/git/git/blob/cc01bad4a9f566cf4453c7edd6b433851b0835e2/diffcore-rename.c#L350-L369
        // We would need a hashmap to be OK to not use the limit here, otherwise the performance is too bad.
        // This also means we don't find all renames if we hit the rename limit.
        if self.match_pairs(cb, None /* by identity */, kind, out, diff_cache, objects, filter)? == Action::Cancel {
            return Ok(());
        }
        if needs_second_pass {
            let is_limited = if self.rewrites.limit == 0 {
                false
            } else {
                let (num_src, num_dst) =
                    estimate_involved_items(self.items.iter().map(|item| (item.emitted, item.change.kind())), kind);
                let permutations = num_src * num_dst;
                if permutations > self.rewrites.limit {
                    match kind {
                        visit::SourceKind::Rename => {
                            out.num_similarity_checks_skipped_for_rename_tracking_due_to_limit = permutations;
                        }
                        visit::SourceKind::Copy => {
                            out.num_similarity_checks_skipped_for_copy_tracking_due_to_limit = permutations;
                        }
                    }
                    true
                } else {
                    false
                }
            };
            if !is_limited {
                self.match_pairs(cb, percentage, kind, out, diff_cache, objects, None)?;
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn match_pairs(
        &mut self,
        cb: &mut impl FnMut(visit::Destination<'_, T>, Option<visit::Source<'_, T>>) -> Action,
        percentage: Option<f32>,
        kind: visit::SourceKind,
        stats: &mut Outcome,
        diff_cache: &mut crate::blob::Platform,
        objects: &impl gix_object::FindObjectOrHeader,
        filter: Option<fn(&T) -> bool>,
    ) -> Result<Action, emit::Error> {
        let mut dest_ofs = 0;
        let mut num_checks = 0;
        let max_checks = {
            let limit = self.rewrites.limit.saturating_pow(2);
            // There can be trees with a lot of entries and pathological search behaviour, as they can be repeated
            // and then have a lot of similar hashes. This also means we have to search a lot of candidates which
            // can be too slow despite best attempts. So play it save and detect such cases 'roughly' by amount of items.
            if self.items.len() < 100_000 {
                0
            } else {
                limit
            }
        };

        while let Some((mut dest_idx, dest)) = self.items[dest_ofs..].iter().enumerate().find_map(|(idx, item)| {
            (!item.emitted
                && matches!(item.change.kind(), ChangeKind::Addition)
                && filter.map_or_else(
                    || {
                        self.rewrites.track_empty
                            // We always want to keep track of entries that are involved of a directory rename.
                            // Note that this may still match them up arbitrarily if empty, but empty is empty.
                            || matches!(item.change.relation(), Some(Relation::ChildOfParent(_)))
                            || {
                                let id = item.change.id();
                                id != gix_hash::ObjectId::empty_blob(id.kind())
                            }
                    },
                    |f| f(&item.change),
                ))
            .then_some((idx, item))
        }) {
            dest_idx += dest_ofs;
            dest_ofs = dest_idx + 1;
            self.items[dest_idx].location(&self.path_backing);
            let src = find_match(
                &self.items,
                dest,
                dest_idx,
                percentage,
                kind,
                stats,
                objects,
                diff_cache,
                &self.path_backing,
                &mut num_checks,
            )?
            .map(|(src_idx, src, diff)| {
                let (id, entry_mode) = src.change.id_and_entry_mode();
                let id = id.to_owned();
                let location = src.location(&self.path_backing);
                (
                    visit::Source {
                        entry_mode,
                        id,
                        kind,
                        location,
                        change: &src.change,
                        diff,
                    },
                    src_idx,
                )
            });
            if max_checks != 0 && num_checks > max_checks {
                gix_trace::warn!(
                    "Cancelled rename matching as there were too many iterations ({num_checks} > {max_checks})"
                );
                return Ok(Action::Cancel);
            }
            let Some((src, src_idx)) = src else {
                continue;
            };
            let location = dest.location(&self.path_backing);
            let change = dest.change.clone();
            let dest = visit::Destination { change, location };
            let relations = if percentage.is_none() {
                src.change.relation().zip(dest.change.relation())
            } else {
                None
            };
            let res = cb(dest, Some(src));

            self.items[dest_idx].emitted = true;
            self.items[src_idx].emitted = true;

            if res == Action::Cancel {
                return Ok(Action::Cancel);
            }

            match relations {
                Some((Relation::Parent(src), Relation::Parent(dst))) => {
                    let res = self.emit_child_renames_matching_identity(cb, kind, src, dst)?;
                    if res == Action::Cancel {
                        return Ok(Action::Cancel);
                    }
                }
                Some((Relation::ChildOfParent(src), Relation::ChildOfParent(dst))) => {
                    self.child_renames.insert((src, dst));
                }
                _ => {}
            }
        }
        Ok(Action::Continue)
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
    ) -> Result<Action, emit::Error> {
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
                    diff: None,
                };
                let location = dst_item.location(&self.path_backing);
                let change = dst_item.change.clone();
                let dst = visit::Destination { change, location };
                let res = cb(dst, Some(src));

                src_item.emitted = true;
                dst_item.emitted = true;

                if res == Action::Cancel {
                    return Ok(res);
                }
            } else {
                gix_trace::warn!("Children of parents with change-id {src_parent_id} and {dst_parent_id} were not equal, even though their parents claimed to be");
                break;
            }
        }
        Ok(Action::Continue)
    }

    /// Find directories with relation id that haven't been emitted yet and store them for lookup.
    /// Then use the previously stored emitted renames with relation id to learn which directories they 'link'
    /// and emit them, too.
    /// Note that this works whenever top-level directories are renamed because they are always added and deleted,
    /// and we only match those. Thus, one rewrite inside the directory is enough.
    fn match_renamed_directories(
        &mut self,
        cb: &mut impl FnMut(visit::Destination<'_, T>, Option<visit::Source<'_, T>>) -> Action,
    ) -> Result<(), emit::Error> {
        fn unemitted_directory_matching_relation_id<T: Change>(items: &[Item<T>], child_id: ChangeId) -> Option<usize> {
            items.iter().position(|i| {
                !i.emitted && matches!(i.change.relation(), Some(Relation::Parent(pid)) if pid == child_id)
            })
        }
        for (deleted_child_id, added_child_id) in &self.child_renames {
            let Some(src_idx) = unemitted_directory_matching_relation_id(&self.items, *deleted_child_id) else {
                continue;
            };
            let Some(dst_idx) = unemitted_directory_matching_relation_id(&self.items, *added_child_id) else {
                // This could go wrong in case there are mismatches, so be defensive here.
                // But generally, we'd expect the destination item to exist.
                continue;
            };

            let (src_item, dst_item) = (&self.items[src_idx], &self.items[dst_idx]);
            let entry_mode = src_item.change.entry_mode();
            let location = src_item.location(&self.path_backing);
            let src = visit::Source {
                entry_mode,
                id: src_item.change.id().to_owned(),
                kind: SourceKind::Rename,
                location,
                change: &src_item.change,
                diff: None,
            };
            let location = dst_item.location(&self.path_backing);
            let change = dst_item.change.clone();
            let dst = visit::Destination { change, location };
            let res = cb(dst, Some(src));

            self.items[src_idx].emitted = true;
            self.items[dst_idx].emitted = true;

            if res == Action::Cancel {
                return Ok(());
            }
        }
        Ok(())
    }
}

fn filename(path: &BStr) -> &BStr {
    path.rfind_byte(b'/').map_or(path, |idx| path[idx + 1..].as_bstr())
}

/// Returns the amount of viable sources and destinations for `items` as eligible for the given `kind` of operation.
fn estimate_involved_items(
    items: impl IntoIterator<Item = (bool, ChangeKind)>,
    kind: visit::SourceKind,
) -> (usize, usize) {
    items
        .into_iter()
        .filter(|(emitted, _)| match kind {
            visit::SourceKind::Rename => !*emitted,
            visit::SourceKind::Copy => true,
        })
        .fold((0, 0), |(mut src, mut dest), (emitted, change_kind)| {
            match change_kind {
                ChangeKind::Addition => {
                    if kind == visit::SourceKind::Rename || !emitted {
                        dest += 1;
                    }
                }
                ChangeKind::Deletion => {
                    if kind == visit::SourceKind::Rename {
                        src += 1;
                    }
                }
                ChangeKind::Modification => {
                    if kind == visit::SourceKind::Copy {
                        src += 1;
                    }
                }
            }
            (src, dest)
        })
}

fn needs_exact_match(percentage: Option<f32>) -> bool {
    percentage.map_or(true, |p| p >= 1.0)
}

/// <`src_idx`, src, possibly diff stat>
type SourceTuple<'a, T> = (usize, &'a Item<T>, Option<DiffLineStats>);

/// Find `item` in our set of items ignoring `item_idx` to avoid finding ourselves, by similarity indicated by `percentage`.
/// The latter can be `None` or `Some(x)` where `x>=1` for identity, and anything else for similarity.
/// We also ignore emitted items entirely.
/// Use `kind` to indicate what kind of match we are looking for, which might be deletions matching an `item` addition, or
/// any non-deletion otherwise.
/// Note that we always try to find by identity first even if a percentage is given as it's much faster and may reduce the set
/// of items to be searched.
#[allow(clippy::too_many_arguments)]
fn find_match<'a, T: Change>(
    items: &'a [Item<T>],
    item: &Item<T>,
    item_idx: usize,
    percentage: Option<f32>,
    kind: visit::SourceKind,
    stats: &mut Outcome,
    objects: &impl gix_object::FindObjectOrHeader,
    diff_cache: &mut crate::blob::Platform,
    path_backing: &[u8],
    num_checks: &mut usize,
) -> Result<Option<SourceTuple<'a, T>>, emit::Error> {
    let (item_id, item_mode) = item.change.id_and_entry_mode();
    if needs_exact_match(percentage) || item_mode.is_link() {
        let first_idx = items.partition_point(|a| a.change.id() < item_id);
        let range = items.get(first_idx..).map(|slice| {
            let end = slice
                .iter()
                .position(|a| a.change.id() != item_id)
                .map_or(items.len(), |idx| first_idx + idx);
            first_idx..end
        });
        let range = match range {
            Some(range) => range,
            None => return Ok(None),
        };
        if range.is_empty() {
            return Ok(None);
        }
        let res = items[range.clone()].iter().enumerate().find_map(|(mut src_idx, src)| {
            src_idx += range.start;
            *num_checks += 1;
            (src_idx != item_idx && src.is_source_for_destination_of(kind, item_mode)).then_some((src_idx, src, None))
        });
        if let Some(src) = res {
            return Ok(Some(src));
        }
    } else if item_mode.is_blob() {
        let mut has_new = false;
        let percentage = percentage.expect("it's set to something below 1.0 and we assured this");

        for (can_idx, src) in items
            .iter()
            .enumerate()
            .filter(|(src_idx, src)| *src_idx != item_idx && src.is_source_for_destination_of(kind, item_mode))
        {
            if !has_new {
                diff_cache.set_resource(
                    item_id.to_owned(),
                    item_mode.kind(),
                    item.location(path_backing),
                    ResourceKind::NewOrDestination,
                    objects,
                )?;
                has_new = true;
            }
            let (src_id, src_mode) = src.change.id_and_entry_mode();
            diff_cache.set_resource(
                src_id.to_owned(),
                src_mode.kind(),
                src.location(path_backing),
                ResourceKind::OldOrSource,
                objects,
            )?;
            let prep = diff_cache.prepare_diff()?;
            stats.num_similarity_checks += 1;
            *num_checks += 1;
            match prep.operation {
                Operation::InternalDiff { algorithm } => {
                    let tokens =
                        crate::blob::intern::InternedInput::new(prep.old.intern_source(), prep.new.intern_source());
                    let counts = crate::blob::diff(
                        algorithm,
                        &tokens,
                        crate::blob::sink::Counter::new(diff::Statistics {
                            removed_bytes: 0,
                            input: &tokens,
                        }),
                    );
                    let old_data_len = prep.old.data.as_slice().unwrap_or_default().len();
                    let new_data_len = prep.new.data.as_slice().unwrap_or_default().len();
                    let similarity = (old_data_len - counts.wrapped) as f32 / old_data_len.max(new_data_len) as f32;
                    if similarity >= percentage {
                        return Ok(Some((
                            can_idx,
                            src,
                            DiffLineStats {
                                removals: counts.removals,
                                insertions: counts.insertions,
                                before: tokens.before.len().try_into().expect("interner handles only u32"),
                                after: tokens.after.len().try_into().expect("interner handles only u32"),
                                similarity,
                            }
                            .into(),
                        )));
                    }
                }
                Operation::ExternalCommand { .. } => {
                    unreachable!("we have disabled this possibility with an option")
                }
                Operation::SourceOrDestinationIsBinary => {
                    // TODO: figure out if git does more here
                }
            }
        }
    }
    Ok(None)
}

mod diff {
    use std::ops::Range;

    pub struct Statistics<'a, 'data> {
        pub removed_bytes: usize,
        pub input: &'a crate::blob::intern::InternedInput<&'data [u8]>,
    }

    impl crate::blob::Sink for Statistics<'_, '_> {
        type Out = usize;

        fn process_change(&mut self, before: Range<u32>, _after: Range<u32>) {
            self.removed_bytes += self.input.before[before.start as usize..before.end as usize]
                .iter()
                .map(|token| self.input.interner[*token].len())
                .sum::<usize>();
        }

        fn finish(self) -> Self::Out {
            self.removed_bytes
        }
    }
}

#[cfg(test)]
mod estimate_involved_items {
    use super::estimate_involved_items;
    use crate::rewrites::tracker::{visit::SourceKind, ChangeKind};

    #[test]
    fn renames_count_unemitted_as_sources_and_destinations() {
        let items = [
            (false, ChangeKind::Addition),
            (true, ChangeKind::Deletion),
            (true, ChangeKind::Deletion),
        ];
        assert_eq!(
            estimate_involved_items(items, SourceKind::Rename),
            (0, 1),
            "here we only have one eligible source, hence nothing to do"
        );
        assert_eq!(
            estimate_involved_items(items.into_iter().map(|t| (false, t.1)), SourceKind::Rename),
            (2, 1),
            "now we have more possibilities as renames count un-emitted deletions as source"
        );
    }

    #[test]
    fn copies_do_not_count_additions_as_sources() {
        let items = [
            (false, ChangeKind::Addition),
            (true, ChangeKind::Addition),
            (true, ChangeKind::Deletion),
        ];
        assert_eq!(
            estimate_involved_items(items, SourceKind::Copy),
            (0, 1),
            "one addition as source, the other isn't counted as it's emitted, nor is it considered a copy-source.\
            deletions don't count"
        );
    }

    #[test]
    fn copies_count_modifications_as_sources() {
        let items = [
            (false, ChangeKind::Addition),
            (true, ChangeKind::Modification),
            (false, ChangeKind::Modification),
        ];
        assert_eq!(
            estimate_involved_items(items, SourceKind::Copy),
            (2, 1),
            "any modifications is a valid source, emitted or not"
        );
    }
}
