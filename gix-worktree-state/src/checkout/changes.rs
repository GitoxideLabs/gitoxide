use std::{
    cmp::Ordering,
    collections::BTreeSet,
    io,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering as AtomicOrdering},
};

use bstr::{BStr, ByteSlice};
use gix_error::{ErrorExt, Result, ResultExt, ensure, message, validation};
use gix_index::entry::{Flags, Mode, Stage};

use super::{Change, ErrorRecord, Options, Outcome, function::checkout_inner};

/// Apply explicitly selected `changes` to `dir`, using `index` as the source of content and attributes.
///
/// All removals precede writes, irrespective of their order in `changes`. Only selected index entries
/// receive updated stat information; the index is not persisted. Writing a conflict stage is rejected:
/// callers must first materialize its desired worktree content as a stage-zero entry. Sparse-directory
/// entries must remain skipped or be expanded before writing.
///
/// This is an executor, not a safe checkout planner. Callers must check that replacing or removing each
/// selected path will not discard changes they want to preserve, and prevent concurrent worktree changes.
/// Existing regular files selected for writing are overwritten. Unselected paths are left alone, except
/// that empty parent directories of removed paths are pruned without removing `dir` itself.
///
/// Action indices, duplicate actions, invalid paths, and conflicting write paths are rejected before any
/// mutation. On case-insensitive filesystems, ASCII case aliases are also rejected. A removal followed by
/// a write at the same path, or a file/directory replacement, is allowed. Directory obstructions are never
/// removed recursively, even with [`Options::overwrite_existing`]: symlink and type replacements require
/// explicit removal actions. Selected sparse entries are skipped by the regular checkout machinery.
///
/// Paths below `dir` never traverse symlinked parents. As with [`crate::checkout()`], `dir` itself is
/// trusted and may contain symlink components, and other processes changing parents can race the checks.
/// Missing removal paths are already satisfied. Filesystem failures may leave partial changes; use
/// [`Options::keep_going`] to collect failures. Interruption returns an `Ok` outcome with partial work,
/// so callers must also check `should_interrupt`. Progress counts written files and bytes, not removals.
#[expect(clippy::too_many_arguments)]
pub fn apply_changes<Find>(
    index: &mut gix_index::State,
    changes: &[Change],
    dir: impl Into<PathBuf>,
    objects: Find,
    files: &dyn gix_utils::progress::Count,
    bytes: &dyn gix_utils::progress::Count,
    should_interrupt: &AtomicBool,
    mut options: Options,
) -> Result<Outcome>
where
    Find: gix_object::Find + Send + Clone,
{
    let (selected, removals) = validate_changes(index, changes, &options)?;
    let dir = dir.into();
    let mut stack = gix_fs::Stack::new(dir.clone());
    let mut parents = BTreeSet::new();
    let mut errors = Vec::new();
    for path in removals {
        if should_interrupt.load(AtomicOrdering::Relaxed) {
            return Ok(Outcome {
                errors,
                ..Default::default()
            });
        }
        let res = remove(path, &mut stack).map(|removed| {
            if removed && let Some(parent) = stack.current().parent() {
                parents.insert(parent.to_owned());
            }
        });
        if let Err(err) = res {
            let err = err.and_raise(message!("Could not remove worktree path {path:?}"));
            if !options.keep_going {
                return Err(err);
            }
            errors.push(ErrorRecord {
                path: path.into(),
                error: err,
            });
        }
    }
    // Prune after leaf removals so the path stack never caches a directory we just removed.
    for parent in parents.into_iter().rev() {
        if should_interrupt.load(AtomicOrdering::Relaxed) {
            return Ok(Outcome {
                errors,
                ..Default::default()
            });
        }
        if let Err(err) = gix_fs::dir::remove::empty_upward_until_boundary(&parent, &dir)
            && !matches!(err.kind(), io::ErrorKind::DirectoryNotEmpty | io::ErrorKind::NotFound)
        {
            let err = err.and_raise(message!(
                "Could not prune empty worktree directory {}",
                parent.display()
            ));
            if !options.keep_going {
                return Err(err);
            }
            errors.push(ErrorRecord {
                path: gix_path::into_bstr(parent.strip_prefix(&dir).expect("removal stays within worktree"))
                    .or_error()?
                    .into_owned(),
                error: err,
            });
        }
    }
    if selected.is_empty() || should_interrupt.load(AtomicOrdering::Relaxed) {
        return Ok(Outcome {
            errors,
            ..Default::default()
        });
    }
    // Only explicit removals may clear obstructions; the general writer's forced mode can recurse.
    options.overwrite_existing = false;
    options.destination_is_initially_empty = false;
    let paths = index.take_path_backing();
    let res = checkout_inner(
        index,
        &paths,
        Some(&selected),
        dir,
        objects,
        files,
        bytes,
        should_interrupt,
        options,
    );
    index.return_path_backing(paths);
    let mut out = res?;
    out.errors.splice(0..0, errors);
    Ok(out)
}

fn validate_changes<'a>(
    index: &gix_index::State,
    changes: &'a [Change],
    options: &Options,
) -> Result<(Vec<usize>, Vec<&'a BStr>)> {
    let mut selected = Vec::new();
    let mut writes = Vec::new();
    let mut removals = Vec::new();
    for change in changes {
        let (path, mode) = match change {
            Change::Write { entry_index } => {
                let entry = index.entries().get(*entry_index).ok_or_else(|| {
                    validation(format!("Checkout entry index {entry_index} is out of bounds")).raise()
                })?;
                ensure!(
                    entry.stage() == Stage::Unconflicted,
                    validation("Checkout requires stage-zero worktree content")
                );
                ensure!(
                    entry.mode != Mode::DIR || entry.flags.contains(Flags::SKIP_WORKTREE),
                    validation("Checkout requires sparse directories to be expanded or skipped")
                );
                selected.push(*entry_index);
                let path = entry.path(index);
                writes.push(path);
                (path, Some(entry.mode))
            }
            Change::Remove { path } => {
                removals.push(path.as_bstr());
                (path.as_bstr(), None)
            }
        };
        let validated_path = if mode == Some(Mode::DIR) {
            path.strip_suffix(b"/").unwrap_or(path).as_bstr()
        } else {
            path
        };
        let mut components = validated_path.split(|b| *b == b'/').peekable();
        while let Some(component) = components.next() {
            gix_worktree::validate::path::component(
                component.as_bstr(),
                (components.peek().is_none() && mode == Some(Mode::SYMLINK))
                    .then_some(gix_worktree::validate::path::component::Mode::Symlink),
                options.validate,
            )
            .or_raise(|| validation(format!("Invalid checkout path {path:?}")))?;
        }
        // Conversion must also succeed before a preceding removal can mutate the worktree.
        gix_path::from_bstr(path).or_raise(|| validation(format!("Invalid checkout path {path:?}")))?;
    }
    validate_paths(&mut writes, options.fs.ignore_case, "write")?;
    validate_paths(&mut removals, options.fs.ignore_case, "remove")?;
    selected.sort_unstable();
    Ok((selected, removals))
}

fn validate_paths(paths: &mut [&BStr], ignore_case: bool, operation: &str) -> Result {
    let compare = |a: &BStr, b: &BStr| -> Ordering {
        if ignore_case {
            a.iter()
                .map(u8::to_ascii_lowercase)
                .cmp(b.iter().map(u8::to_ascii_lowercase))
        } else {
            a.cmp(b)
        }
    };
    paths.sort_by(|a, b| compare(a, b));
    for pair in paths.windows(2) {
        ensure!(
            compare(pair[0], pair[1]) != Ordering::Equal,
            validation(format!("Duplicate checkout {operation} path {:?}", pair[1]))
        );
    }
    for path in paths.iter() {
        for (pos, byte) in path.iter().enumerate() {
            if *byte == b'/' {
                let prefix = path[..pos].as_bstr();
                ensure!(
                    paths.binary_search_by(|path| compare(path, prefix)).is_err(),
                    validation(format!(
                        "Conflicting checkout {operation} paths {prefix:?} and {path:?}"
                    ))
                );
            }
        }
    }
    Ok(())
}

fn remove(path: &BStr, stack: &mut gix_fs::Stack) -> io::Result<bool> {
    let res = stack.make_relative_path_current(path, &mut NoFollow).and_then(|()| {
        let path = stack.current();
        let metadata = path.symlink_metadata()?;
        if metadata.file_type().is_symlink() {
            gix_fs::symlink::remove(path)
        } else if metadata.is_dir() {
            std::fs::remove_dir(path)
        } else {
            std::fs::remove_file(path)
        }
    });
    match res {
        Ok(()) => Ok(true),
        Err(err) if matches!(err.kind(), io::ErrorKind::NotFound | io::ErrorKind::NotADirectory) => Ok(false),
        Err(err) => Err(err),
    }
}

struct NoFollow;

impl gix_fs::stack::Delegate for NoFollow {
    fn push_directory(&mut self, _stack: &gix_fs::Stack) -> io::Result<()> {
        Ok(())
    }

    fn push(&mut self, is_last_component: bool, stack: &gix_fs::Stack) -> io::Result<()> {
        if !is_last_component {
            let metadata = stack.current().symlink_metadata()?;
            if metadata.file_type().is_symlink() {
                return Err(io::Error::other(validation(
                    "Cannot remove a path through a symlinked parent",
                )));
            }
            if !metadata.is_dir() {
                return Err(io::ErrorKind::NotADirectory.into());
            }
        }
        Ok(())
    }

    fn pop_directory(&mut self) {}
}
