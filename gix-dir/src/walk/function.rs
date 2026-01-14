use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use bstr::{BStr, BString, ByteSlice};

use crate::{
    entry,
    walk::{classify, readdir, Action, Context, Delegate, Error, ForDeletionMode, Options, Outcome},
    EntryRef,
};

/// A function to perform a git-style, unsorted, directory walk.
///
/// * `worktree_root` - the top-most root of the worktree, which must be a prefix to `root`.
///     - If [`Options::precompose_unicode`] is enabled, this path must be precomposed.
///     - The starting point of the traversal (traversal root) is calculated from by doing `worktree_root + pathspec.common_prefix()`.
///     - Note that if the traversal root leading to this directory or it itself is excluded, it will be provided to [`Delegate::emit()`]
///       without further traversal.
///     - If [`Options::precompose_unicode`] is enabled, all involved paths must be precomposed.
///     - Must be contained in `worktree_root`.
/// * `ctx` - everything needed to classify the paths seen during the traversal.
/// * `delegate` - an implementation of [`Delegate`] to control details of the traversal and receive its results.
///
/// Returns `(outcome, traversal_root)`, with the `traversal_root` actually being used for the traversal,
/// useful to transform the paths returned for the user. It's always within the `worktree_root`, or the same,
/// but is hard to guess due to additional logic affecting it.
///
/// ### Performance Notes
///
/// In theory, parallel directory traversal can be significantly faster, and what's possible for our current
/// `gix_features::fs::WalkDir` implementation is to abstract a `filter_entry()` method so it works both for
/// the iterator from the `walkdir` crate as well as from `jwalk`. However, doing so as initial version
/// has the risk of not being significantly harder if not impossible to implement as flow-control is very
/// limited.
///
/// Thus the decision was made to start out with something akin to the Git implementation, get all tests and
/// baseline comparison to pass, and see if an iterator with just `filter_entry` would be capable of dealing with
/// it. Note that `filter_entry` is the only just-in-time traversal control that `walkdir` offers, even though
/// one could consider switching to `jwalk` and just use its single-threaded implementation if a unified interface
/// is necessary to make this work - `jwalk` has a more powerful API for this to work.
///
/// If that was the case, we are talking about 0.5s for single-threaded traversal (without doing any extra work)
/// or 0.25s for optimal multi-threaded performance, all in the WebKit directory with 388k items to traverse.
/// Thus, the speedup could easily be 2x or more and thus worth investigating in due time.
pub fn walk(
    worktree_root: &Path,
    mut ctx: Context<'_>,
    options: Options<'_>,
    delegate: &mut dyn Delegate,
) -> Result<(Outcome, PathBuf), Error> {
    let root = match ctx.explicit_traversal_root {
        Some(root) => root.to_owned(),
        None => ctx
            .pathspec
            .longest_common_directory()
            .and_then(|candidate| {
                let candidate = worktree_root.join(candidate);
                candidate.is_dir().then_some(candidate)
            })
            .unwrap_or_else(|| worktree_root.join(ctx.pathspec.prefix_directory())),
    };
    let _span = gix_trace::coarse!("walk", root = ?root, worktree_root = ?worktree_root, options = ?options);
    let (mut current, worktree_root_relative) = assure_no_symlink_in_root(worktree_root, &root)?;
    let mut out = Outcome::default();
    let mut buf = BString::default();
    let (root_info, worktree_root_is_repository) = classify::root(
        worktree_root,
        &mut buf,
        worktree_root_relative.as_ref(),
        options,
        &mut ctx,
    )?;

    let can_recurse = can_recurse(
        buf.as_bstr(),
        if root == worktree_root && root_info.disk_kind == Some(entry::Kind::Symlink) && current.is_dir() {
            classify::Outcome {
                disk_kind: Some(entry::Kind::Directory),
                ..root_info
            }
        } else {
            root_info
        },
        options.for_deletion,
        worktree_root_is_repository,
        delegate,
    );
    if !can_recurse {
        if buf.is_empty() && !root_info.disk_kind.is_some_and(|kind| kind.is_dir()) {
            return Err(Error::WorktreeRootIsFile { root: root.to_owned() });
        }
        if options.precompose_unicode {
            buf = gix_utils::str::precompose_bstr(buf.into()).into_owned();
        }
        let _ = emit_entry(
            Cow::Borrowed(buf.as_bstr()),
            root_info,
            None,
            options,
            &mut out,
            delegate,
        );
        return Ok((out, root.to_owned()));
    }

    let mut state = readdir::State::new(worktree_root, ctx.current_dir, options.for_deletion.is_some());
    let may_collapse = root != worktree_root && state.may_collapse(&current);
    let (action, _) = readdir::recursive(
        may_collapse,
        &mut current,
        &mut buf,
        root_info,
        &mut ctx,
        options,
        delegate,
        &mut out,
        &mut state,
    )?;
    if action.is_continue() {
        state.emit_remaining(may_collapse, options, &mut out, delegate);
        assert_eq!(state.on_hold.len(), 0, "BUG: after emission, on hold must be empty");
    }
    gix_trace::debug!(statistics = ?out);
    Ok((out, root.to_owned()))
}

/// Note that we only check symlinks on the way from `worktree_root` to `root`,
/// so `worktree_root` may go through a symlink.
/// Returns `(worktree_root, normalized_worktree_relative_root)`.
fn assure_no_symlink_in_root<'root>(
    worktree_root: &Path,
    root: &'root Path,
) -> Result<(PathBuf, Cow<'root, Path>), Error> {
    let mut current = worktree_root.to_owned();
    let worktree_relative = root
        .strip_prefix(worktree_root)
        .expect("BUG: root was created from worktree_root + prefix");
    let worktree_relative = gix_path::normalize(worktree_relative.into(), Path::new(""))
        .ok_or(Error::NormalizeRoot { root: root.to_owned() })?;

    for (idx, component) in worktree_relative.components().enumerate() {
        current.push(component);
        let meta = current.symlink_metadata().map_err(|err| Error::SymlinkMetadata {
            source: err,
            path: current.to_owned(),
        })?;
        if meta.is_symlink() {
            return Err(Error::SymlinkInRoot {
                root: root.to_owned(),
                worktree_root: worktree_root.to_owned(),
                component_index: idx,
            });
        }
    }
    Ok((current, worktree_relative))
}

pub(super) fn can_recurse(
    rela_path: &BStr,
    info: classify::Outcome,
    for_deletion: Option<ForDeletionMode>,
    worktree_root_is_repository: bool,
    delegate: &mut dyn Delegate,
) -> bool {
    let is_dir = info.disk_kind.is_some_and(|k| k.is_dir());
    if !is_dir {
        return false;
    }
    delegate.can_recurse(
        EntryRef::from_outcome(Cow::Borrowed(rela_path), info),
        for_deletion,
        worktree_root_is_repository,
    )
}

/// Possibly emit an entry to `for_each` in case the provided information makes that possible.
#[allow(clippy::too_many_arguments)]
pub(super) fn emit_entry(
    rela_path: Cow<'_, BStr>,
    info: classify::Outcome,
    dir_status: Option<entry::Status>,
    Options {
        emit_pruned,
        emit_tracked,
        emit_ignored,
        emit_empty_directories,
        ..
    }: Options<'_>,
    out: &mut Outcome,
    delegate: &mut dyn Delegate,
) -> Action {
    out.seen_entries += 1;

    if (!emit_empty_directories && info.property == Some(entry::Property::EmptyDirectory)
        || !emit_tracked && info.status == entry::Status::Tracked)
        || emit_ignored.is_none() && matches!(info.status, entry::Status::Ignored(_))
        || !emit_pruned
            && (info.status.is_pruned() || info.pathspec_match.is_none_or(|m| m == entry::PathspecMatch::Excluded))
    {
        return std::ops::ControlFlow::Continue(());
    }

    out.returned_entries += 1;
    delegate.emit(EntryRef::from_outcome(rela_path, info), dir_status)
}
