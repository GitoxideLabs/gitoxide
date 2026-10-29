use std::sync::atomic::{AtomicBool, Ordering};

use gix_dir::{
    Entry, EntryRef,
    entry::{Kind, Property, Status},
    walk::{self, CollapsedEntriesEmissionMode, Delegate, EmissionMode, ForDeletionMode},
};

use crate::{
    Result,
    bstr::{BStr, BString, ByteSlice},
    error::{OptionExt, ResultExt, ensure, message, unsupported},
    worktree::clean::Options,
};

impl crate::Repository {
    /// Remove untracked worktree files, symlinks and optionally directories, or preview their paths.
    ///
    /// Pathspecs are relative to the repository's current-directory prefix; no patterns select that
    /// prefix. Explicit patterns enable directory cleaning, like `git clean`. Results are sorted by
    /// worktree-relative path, with wholly removable directories collapsed into single entries.
    ///
    /// Tracked paths, precious entries, nested repositories (including bare ones and
    /// repositories inside ignored directories), the worktree root and the current directory are
    /// retained. Ignored expendable entries are included only with `options.ignored`. Symlink targets
    /// are never traversed or removed. Bare repositories cannot be cleaned.
    ///
    /// Execution locks the index before reading it and holds the lock through deletion, without
    /// writing the index. Callers must prevent concurrent worktree modifications. Interruption and
    /// I/O failures are errors; an error during deletion can leave some candidates already removed.
    pub fn clean_worktree(
        &self,
        patterns: impl IntoIterator<Item = impl AsRef<BStr>>,
        should_interrupt: &AtomicBool,
        options: Options,
    ) -> Result<Vec<Entry>> {
        check_interrupt(should_interrupt)?;
        let repo = self.with_absolute_paths()?;
        let workdir = repo
            .workdir()
            .ok_or_raise(|| unsupported("A working tree is required to clean"))?;
        let _lock = (!options.dry_run)
            .then(|| {
                gix_lock::File::acquire(
                    repo.index_path(),
                    gix_lock::acquire::Fail::Immediately,
                    None,
                    0,
                    Some(&gix_lock::acquire::resolve_symlink),
                )
            })
            .transpose()
            .or_raise(|| message("Could not lock the index before cleaning the worktree"))?;
        let mut index = if repo.index_path().try_exists().or_error()? {
            repo.open_index()?
        } else {
            gix_index::File::from_state(gix_index::State::new(repo.object_hash()), repo.index_path())
        };
        // Cached type information must not turn a directory replaced by a symlink into a traversal.
        for entry in index.entries_mut() {
            entry
                .flags
                .remove(gix_index::entry::Flags::UPTODATE | gix_index::entry::Flags::ASSUME_VALID);
        }
        let patterns: Vec<BString> = patterns
            .into_iter()
            .map(|pattern| pattern.as_ref().to_owned())
            .collect();
        let mut candidates = Candidates {
            inner: Default::default(),
            directories: options.directories || !patterns.is_empty(),
        };
        repo.dirwalk(
            &index,
            patterns,
            should_interrupt,
            repo.dirwalk_options()?
                .for_deletion(Some(ForDeletionMode::FindRepositoriesInIgnoredDirectories))
                .classify_untracked_bare_repositories(true)
                .emit_untracked(EmissionMode::CollapseDirectory)
                .emit_ignored(options.ignored.then_some(EmissionMode::CollapseDirectory))
                .emit_collapsed(Some(CollapsedEntriesEmissionMode::All))
                .emit_empty_directories(true)
                .empty_patterns_match_prefix(true),
            &mut candidates,
        )?;
        check_interrupt(should_interrupt)?;
        // Group a directory with its descendants even when a sibling sorts before its slash.
        candidates.inner.unorded_entries.sort_by(|a, b| {
            a.0.rela_path
                .split(|byte| *byte == b'/')
                .cmp(b.0.rela_path.split(|byte| *byte == b'/'))
        });
        let mut selected: Vec<Entry> = Vec::new();
        for (entry, _) in candidates.inner.unorded_entries {
            if entry.rela_path.is_empty()
                || entry.index_kind.is_some()
                || entry.property == Some(Property::EmptyDirectoryAndCWD)
                || selected.last().is_some_and(|parent| {
                    entry
                        .rela_path
                        .strip_prefix(parent.rela_path.as_slice())
                        .is_some_and(|suffix| suffix.starts_with(b"/"))
                })
            {
                continue;
            }
            match entry.status {
                Status::Untracked => {}
                Status::Ignored(gix_ignore::Kind::Expendable) if options.ignored => {}
                Status::Ignored(_) | Status::Tracked | Status::Pruned => continue,
            }
            match entry.disk_kind {
                Some(Kind::File | Kind::Symlink) => {}
                Some(Kind::Directory) if candidates.directories => {}
                Some(Kind::Directory | Kind::Repository | Kind::Untrackable) | None => continue,
            }
            selected.push(entry);
        }
        selected.sort_by(|a, b| a.rela_path.cmp(&b.rela_path));
        if !options.dry_run {
            let mut paths = gix_status::SymlinkCheck::new(workdir.to_owned());
            for entry in &selected {
                check_interrupt(should_interrupt)?;
                let path = paths
                    .verified_path(gix_path::from_bstr(entry.rela_path.as_bstr())?.as_ref())
                    .or_raise(|| message!("Could not safely reach clean candidate {:?}", entry.rela_path))?;
                if entry.disk_kind == Some(Kind::Directory) {
                    std::fs::remove_dir_all(path)
                } else {
                    std::fs::remove_file(path)
                }
                .or_raise(|| message!("Could not remove clean candidate {:?}", entry.rela_path))?;
            }
        }
        Ok(selected)
    }
}

struct Candidates {
    inner: walk::delegate::Collect,
    directories: bool,
}

impl Delegate for Candidates {
    fn emit(&mut self, entry: EntryRef<'_>, collapsed_directory_status: Option<Status>) -> walk::Action {
        self.inner.emit(entry, collapsed_directory_status)
    }

    fn can_recurse(&mut self, entry: EntryRef<'_>, for_deletion: Option<ForDeletionMode>, root: bool) -> bool {
        if entry.index_kind.is_some_and(|kind| kind != Kind::Directory)
            || !self.directories && !entry.rela_path.is_empty() && entry.status != Status::Tracked
        {
            return false;
        }
        entry
            .status
            .can_recurse(entry.disk_kind, entry.pathspec_match, for_deletion, root)
    }
}

fn check_interrupt(should_interrupt: &AtomicBool) -> Result<()> {
    ensure!(
        !should_interrupt.load(Ordering::Relaxed),
        std::io::Error::new(std::io::ErrorKind::Interrupted, "Worktree cleaning interrupted")
    );
    Ok(())
}
