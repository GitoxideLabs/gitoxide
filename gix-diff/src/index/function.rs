use std::{borrow::Cow, cell::RefCell, cmp::Ordering};

use bstr::BStr;
use gix_filter::attributes::glob::pattern::Case;

use super::{Action, ChangeRef, Error, RewriteOptions};
use crate::rewrites;

/// Produce an entry-by-entry diff between `lhs` and `rhs`, sending changes to `cb(change) -> Action` for consumption,
/// which would turn `lhs` into `rhs` if applied.
/// Use `pathspec` to reduce the set of entries to look at, and `pathspec_attributes` may be used by pathspecs that perform
/// attribute lookups.
///
/// If `cb` indicated that the operation should be cancelled, no error is triggered as this isn't supposed to
/// occur through user-interaction - this diff is typically too fast.
///
/// Note that rewrites will be emitted at the end, so no ordering can be assumed. They will only be tracked if
/// `rewrite_options` is `Some`. Note that the set of entries participating in rename tracking is affected by `pathspec`.
///
/// Return the outcome of the rewrite tracker if it was enabled.
///
/// Note that only `rhs` may contain unmerged entries, as `rhs` is expected to be the index read from `.git/index`.
/// Unmerged entries are skipped entirely.
///
/// Conceptually, `rhs` is *ours*, and `lhs` is *theirs*.
/// The entries in `lhs` and `rhs` are both expected to be sorted like index entries are typically sorted.
///
/// Note that sparse indices aren't supported, they must be "unsparsed" before.
pub fn diff<'rhs, 'lhs: 'rhs, E, Find>(
    lhs: &'lhs gix_index::State,
    rhs: &'rhs gix_index::State,
    mut cb: impl FnMut(ChangeRef<'lhs, 'rhs>) -> Result<Action, E>,
    rewrite_options: Option<RewriteOptions<'_, Find>>,
    pathspec: &mut gix_pathspec::Search,
    pathspec_attributes: &mut dyn FnMut(&BStr, Case, bool, &mut gix_attributes::search::Outcome) -> bool,
) -> Result<Option<rewrites::Outcome>, Error>
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
    Find: gix_object::FindObjectOrHeader,
{
    if lhs.is_sparse() || rhs.is_sparse() {
        return Err(Error::IsSparse);
    }
    if lhs
        .entries()
        .iter()
        .any(|e| e.stage() != gix_index::entry::Stage::Unconflicted)
    {
        return Err(Error::LhsHasUnmerged);
    }

    let lhs_range = lhs
        .prefixed_entries_range(pathspec.common_prefix())
        .unwrap_or_else(|| 0..lhs.entries().len());
    let rhs_range = rhs
        .prefixed_entries_range(pathspec.common_prefix())
        .unwrap_or_else(|| 0..rhs.entries().len());

    let pattern_matches = RefCell::new(|relative_path, entry: &gix_index::Entry| {
        pathspec
            .pattern_matching_relative_path(relative_path, Some(entry.mode.is_submodule()), pathspec_attributes)
            .is_some_and(|m| !m.is_excluded())
    });

    let (mut lhs_iter, mut rhs_iter) = (
        lhs.entries()[lhs_range.clone()]
            .iter()
            .enumerate()
            .map(|(idx, e)| (idx + lhs_range.start, e.path(lhs), e))
            .filter(|(_, path, e)| pattern_matches.borrow_mut()(path, e)),
        rhs.entries()[rhs_range.clone()]
            .iter()
            .enumerate()
            .map(|(idx, e)| (idx + rhs_range.start, e.path(rhs), e))
            .filter(|(_, path, e)| pattern_matches.borrow_mut()(path, e)),
    );

    let mut resource_cache_storage = None;
    let mut tracker = rewrite_options.map(
        |RewriteOptions {
             resource_cache,
             rewrites,
             find,
         }| {
            resource_cache_storage = Some((resource_cache, find));
            rewrites::Tracker::<ChangeRef<'lhs, 'rhs>>::new(rewrites)
        },
    );

    let (mut lhs_storage, mut rhs_storage) = (lhs_iter.next(), rhs_iter.next());
    loop {
        match (lhs_storage, rhs_storage) {
            (Some(lhs), Some(rhs)) => {
                let (lhs_idx, lhs_path, lhs_entry) = lhs;
                let (rhs_idx, rhs_path, rhs_entry) = rhs;
                match lhs_path.cmp(rhs_path) {
                    Ordering::Less => match emit_deletion(lhs, &mut cb, tracker.as_mut())? {
                        std::ops::ControlFlow::Continue(()) => {
                            lhs_storage = lhs_iter.next();
                        }
                        std::ops::ControlFlow::Break(()) => return Ok(None),
                    },
                    Ordering::Equal => {
                        if ignore_unmerged_and_intent_to_add(rhs) {
                            rhs_storage = rhs_iter.next();
                            lhs_storage = lhs_iter.next();
                            continue;
                        }
                        if lhs_entry.id != rhs_entry.id || lhs_entry.mode != rhs_entry.mode {
                            let change = ChangeRef::Modification {
                                location: Cow::Borrowed(rhs_path),
                                previous_index: lhs_idx,
                                previous_entry_mode: lhs_entry.mode,
                                previous_id: Cow::Borrowed(lhs_entry.id.as_ref()),
                                index: rhs_idx,
                                entry_mode: rhs_entry.mode,
                                id: Cow::Borrowed(rhs_entry.id.as_ref()),
                            };

                            let change = match tracker.as_mut() {
                                None => Some(change),
                                Some(tracker) => tracker.try_push_change(change, rhs_path),
                            };
                            if let Some(change) = change {
                                match cb(change).map_err(|err| Error::Callback(err.into()))? {
                                    std::ops::ControlFlow::Continue(()) => {}
                                    std::ops::ControlFlow::Break(()) => return Ok(None),
                                }
                            }
                        }
                        lhs_storage = lhs_iter.next();
                        rhs_storage = rhs_iter.next();
                    }
                    Ordering::Greater => match emit_addition(rhs, &mut cb, tracker.as_mut())? {
                        std::ops::ControlFlow::Continue(()) => {
                            rhs_storage = rhs_iter.next();
                        }
                        std::ops::ControlFlow::Break(()) => return Ok(None),
                    },
                }
            }
            (Some(lhs), None) => match emit_deletion(lhs, &mut cb, tracker.as_mut())? {
                std::ops::ControlFlow::Break(()) => return Ok(None),
                std::ops::ControlFlow::Continue(()) => {
                    lhs_storage = lhs_iter.next();
                }
            },
            (None, Some(rhs)) => match emit_addition(rhs, &mut cb, tracker.as_mut())? {
                std::ops::ControlFlow::Break(()) => return Ok(None),
                std::ops::ControlFlow::Continue(()) => {
                    rhs_storage = rhs_iter.next();
                }
            },
            (None, None) => break,
        }
    }

    if let Some((mut tracker, (resource_cache, find))) = tracker.zip(resource_cache_storage) {
        let mut cb_err = None;
        let out = tracker.emit(
            |dst, src| {
                let change = if let Some(src) = src {
                    let (lhs_path, lhs_index, lhs_mode, lhs_id) = src.change.fields();
                    let (rhs_path, rhs_index, rhs_mode, rhs_id) = dst.change.fields();
                    ChangeRef::Rewrite {
                        source_location: Cow::Owned(lhs_path.into()),
                        source_index: lhs_index,
                        source_entry_mode: lhs_mode,
                        source_id: Cow::Owned(lhs_id.into()),
                        location: Cow::Owned(rhs_path.into()),
                        index: rhs_index,
                        entry_mode: rhs_mode,
                        id: Cow::Owned(rhs_id.into()),
                        copy: match src.kind {
                            rewrites::tracker::visit::SourceKind::Rename => false,
                            rewrites::tracker::visit::SourceKind::Copy => true,
                        },
                    }
                } else {
                    dst.change
                };
                match cb(change) {
                    Ok(std::ops::ControlFlow::Continue(())) => std::ops::ControlFlow::Continue(()),
                    Ok(std::ops::ControlFlow::Break(())) => std::ops::ControlFlow::Break(()),
                    Err(err) => {
                        cb_err = Some(Error::Callback(err.into()));
                        std::ops::ControlFlow::Break(())
                    }
                }
            },
            resource_cache,
            find,
            |push| {
                for (index, entry) in lhs.entries().iter().enumerate() {
                    let path = entry.path(rhs);
                    push(
                        ChangeRef::Modification {
                            location: Cow::Borrowed(path),
                            previous_index: 0, /* does not matter */
                            previous_entry_mode: entry.mode,
                            previous_id: Cow::Owned(entry.id.kind().null()),
                            index,
                            entry_mode: entry.mode,
                            id: Cow::Borrowed(entry.id.as_ref()),
                        },
                        path,
                    );
                }
                Ok::<_, std::convert::Infallible>(())
            },
        )?;

        if let Some(err) = cb_err {
            Err(err)
        } else {
            Ok(Some(out))
        }
    } else {
        Ok(None)
    }
}

fn emit_deletion<'rhs, 'lhs: 'rhs, E>(
    (idx, path, entry): (usize, &'lhs BStr, &'lhs gix_index::Entry),
    mut cb: impl FnMut(ChangeRef<'lhs, 'rhs>) -> Result<Action, E>,
    tracker: Option<&mut rewrites::Tracker<ChangeRef<'lhs, 'rhs>>>,
) -> Result<Action, Error>
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    let change = ChangeRef::Deletion {
        location: Cow::Borrowed(path),
        index: idx,
        entry_mode: entry.mode,
        id: Cow::Borrowed(entry.id.as_ref()),
    };

    let change = match tracker {
        None => change,
        Some(tracker) => match tracker.try_push_change(change, path) {
            Some(change) => change,
            None => return Ok(std::ops::ControlFlow::Continue(())),
        },
    };

    cb(change).map_err(|err| Error::Callback(err.into()))
}

fn emit_addition<'rhs, 'lhs: 'rhs, E>(
    (idx, path, entry): (usize, &'rhs BStr, &'rhs gix_index::Entry),
    mut cb: impl FnMut(ChangeRef<'lhs, 'rhs>) -> Result<Action, E>,
    tracker: Option<&mut rewrites::Tracker<ChangeRef<'lhs, 'rhs>>>,
) -> Result<Action, Error>
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    if ignore_unmerged_and_intent_to_add((idx, path, entry)) {
        return Ok(std::ops::ControlFlow::Continue(()));
    }

    let change = ChangeRef::Addition {
        location: Cow::Borrowed(path),
        index: idx,
        entry_mode: entry.mode,
        id: Cow::Borrowed(entry.id.as_ref()),
    };

    let change = match tracker {
        None => change,
        Some(tracker) => match tracker.try_push_change(change, path) {
            Some(change) => change,
            None => return Ok(std::ops::ControlFlow::Continue(())),
        },
    };

    cb(change).map_err(|err| Error::Callback(err.into()))
}

fn ignore_unmerged_and_intent_to_add<'rhs, 'lhs: 'rhs>(
    (_idx, _path, entry): (usize, &'rhs BStr, &'rhs gix_index::Entry),
) -> bool {
    let stage = entry.stage();
    entry.flags.contains(gix_index::entry::Flags::INTENT_TO_ADD) || stage != gix_index::entry::Stage::Unconflicted
}
