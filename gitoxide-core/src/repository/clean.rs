use crate::OutputFormat;

#[derive(Default, Copy, Clone)]
pub enum FindRepository {
    #[default]
    NonBare,
    All,
}

pub struct Options {
    pub debug: bool,
    pub format: OutputFormat,
    pub execute: bool,
    pub ignored: bool,
    pub precious: bool,
    pub directories: bool,
    pub repositories: bool,
    pub pathspec_matches_result: bool,
    pub skip_hidden_repositories: Option<FindRepository>,
    pub find_untracked_repositories: FindRepository,
}
pub(crate) mod function {
    use crate::repository::clean::{FindRepository, Options};
    use crate::OutputFormat;
    use anyhow::bail;
    use gix::bstr::BString;
    use gix::bstr::ByteSlice;
    use gix::dir::entry::{Kind, Status};
    use gix::dir::walk::EmissionMode::CollapseDirectory;
    use gix::dir::walk::ForDeletionMode::*;
    use gix::dir::{walk, EntryRef};
    use std::borrow::Cow;
    use std::path::Path;

    pub fn clean(
        repo: gix::Repository,
        out: &mut dyn std::io::Write,
        err: &mut dyn std::io::Write,
        patterns: Vec<BString>,
        Options {
            debug,
            format,
            mut execute,
            ignored,
            precious,
            directories,
            repositories,
            skip_hidden_repositories,
            find_untracked_repositories,
            pathspec_matches_result,
        }: Options,
    ) -> anyhow::Result<()> {
        if format != OutputFormat::Human {
            bail!("JSON output isn't implemented yet");
        }
        let Some(workdir) = repo.work_dir() else {
            bail!("Need a worktree to clean, this is a bare repository");
        };

        let index = repo.index_or_empty()?;
        let pathspec_for_dirwalk = !pathspec_matches_result;
        let has_patterns = !patterns.is_empty();
        let mut collect = InterruptibleCollect::default();
        let collapse_directories = CollapseDirectory;
        let options = repo
            .dirwalk_options()?
            .emit_pruned(true)
            .for_deletion(if (ignored || precious) && directories {
                match skip_hidden_repositories {
                    Some(FindRepository::NonBare) => Some(FindNonBareRepositoriesInIgnoredDirectories),
                    Some(FindRepository::All) => Some(FindRepositoriesInIgnoredDirectories),
                    None => Some(Default::default()),
                }
            } else {
                Some(Default::default())
            })
            .classify_untracked_bare_repositories(matches!(find_untracked_repositories, FindRepository::All))
            .emit_untracked(collapse_directories)
            .emit_ignored(Some(collapse_directories))
            .empty_patterns_match_prefix(true)
            .emit_empty_directories(true);
        repo.dirwalk(
            &index,
            if pathspec_for_dirwalk {
                patterns.clone()
            } else {
                Vec::new()
            },
            &gix::interrupt::IS_INTERRUPTED,
            options,
            &mut collect,
        )?;

        let mut pathspec = pathspec_matches_result
            .then(|| {
                repo.pathspec(
                    true,
                    patterns,
                    true,
                    &index,
                    gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
                )
            })
            .transpose()?;
        let prefix = repo.prefix()?.unwrap_or(Path::new(""));
        let entries = collect.inner.into_entries_by_path();
        let mut entries_to_clean = 0;
        let mut skipped_directories = 0;
        let mut skipped_ignored = 0;
        let mut skipped_precious = 0;
        let mut skipped_repositories = 0;
        let mut pruned_entries = 0;
        let mut saw_ignored_directory = false;
        let mut saw_untracked_directory = false;
        for (mut entry, dir_status) in entries.into_iter() {
            if dir_status.is_some() {
                if debug {
                    writeln!(
                        err,
                        "DBG: prune '{}' {:?} as parent dir is used instead",
                        entry.rela_path, entry.status
                    )
                    .ok();
                }
                continue;
            }

            let pathspec_includes_entry = match pathspec.as_mut() {
                None => entry
                    .pathspec_match
                    .is_some_and(|m| m != gix::dir::entry::PathspecMatch::Excluded),
                Some(pathspec) => pathspec
                    .pattern_matching_relative_path(entry.rela_path.as_bstr(), entry.disk_kind.map(|k| k.is_dir()))
                    .is_some_and(|m| !m.is_excluded()),
            };
            pruned_entries += usize::from(!pathspec_includes_entry);
            if !pathspec_includes_entry && debug {
                writeln!(err, "DBG: prune '{}'", entry.rela_path).ok();
            }
            if entry.status.is_pruned() || !pathspec_includes_entry {
                continue;
            }

            let keep = match entry.status {
                Status::Pruned => {
                    unreachable!("BUG: we skipped these above")
                }
                Status::Tracked => {
                    unreachable!("BUG: tracked aren't emitted")
                }
                Status::Ignored(gix::ignore::Kind::Expendable) => {
                    skipped_ignored += usize::from(!ignored);
                    ignored
                }
                Status::Ignored(gix::ignore::Kind::Precious) => {
                    skipped_precious += usize::from(!precious);
                    precious
                }
                Status::Untracked => true,
            };
            if entry.disk_kind.is_none() {
                entry.disk_kind = workdir
                    .join(gix::path::from_bstr(entry.rela_path.as_bstr()))
                    .symlink_metadata()
                    .ok()
                    .map(|e| e.file_type().into());
            }
            let Some(mut disk_kind) = entry.disk_kind else {
                if debug {
                    writeln!(err, "DBG: ignoring unreadable entry at '{}' ", entry.rela_path).ok();
                }
                continue;
            };
            if !keep {
                if debug {
                    writeln!(err, "DBG: prune '{}' as -x or -p is missing", entry.rela_path).ok();
                }
                continue;
            }

            if disk_kind == gix::dir::entry::Kind::Directory
                && gix::discover::is_git(&workdir.join(gix::path::from_bstr(entry.rela_path.as_bstr()))).is_ok()
            {
                if debug {
                    writeln!(err, "DBG: upgraded directory '{}' to bare repository", entry.rela_path).ok();
                }
                disk_kind = gix::dir::entry::Kind::Repository;
            }

            match disk_kind {
                Kind::Untrackable => {
                    if debug {
                        writeln!(err, "DBG: skipped untrackable entry at '{}'", entry.rela_path).ok();
                    }
                    continue;
                }
                Kind::File | Kind::Symlink => {}
                Kind::Directory => {
                    if !directories {
                        skipped_directories += 1;
                        if debug {
                            writeln!(err, "DBG: prune '{}' as -d is missing", entry.rela_path).ok();
                        }
                        continue;
                    }
                }
                Kind::Repository => {
                    if !repositories {
                        skipped_repositories += 1;
                        if debug {
                            writeln!(err, "DBG: skipped repository at '{}'", entry.rela_path)?;
                        }
                        continue;
                    }
                }
            }

            let is_ignored = matches!(entry.status, gix::dir::entry::Status::Ignored(_));
            let entry_path = gix::path::from_bstr(entry.rela_path);
            let display_path = gix::path::relativize_with_prefix(&entry_path, prefix);
            if disk_kind == gix::dir::entry::Kind::Directory {
                saw_ignored_directory |= is_ignored;
                saw_untracked_directory |= entry.status == gix::dir::entry::Status::Untracked;
            }

            if gix::interrupt::is_triggered() {
                execute = false;
            }
            let mut may_remove_this_entry = execute;
            writeln!(
                out,
                "{maybe}{suffix} {}{} {status}",
                display_path.display(),
                disk_kind.is_dir().then_some("/").unwrap_or_default(),
                status = match entry.status {
                    Status::Ignored(kind) => {
                        Cow::Owned(format!(
                            "({})",
                            match kind {
                                gix::ignore::Kind::Precious => "💲",
                                gix::ignore::Kind::Expendable => "🗑️",
                            }
                        ))
                    }
                    Status::Untracked => {
                        "".into()
                    }
                    status =>
                        if debug {
                            format!("(DBG: {status:?})").into()
                        } else {
                            "".into()
                        },
                },
                maybe = if entry.property == Some(gix::dir::entry::Property::EmptyDirectoryAndCWD) {
                    may_remove_this_entry = false;
                    if execute {
                        "Refusing to remove empty current working directory"
                    } else {
                        "Would refuse to remove empty current working directory"
                    }
                } else if execute {
                    "removing"
                } else {
                    "WOULD remove"
                },
                suffix = match disk_kind {
                    Kind::Untrackable => unreachable!("always skipped earlier"),
                    Kind::Directory if entry.property == Some(gix::dir::entry::Property::EmptyDirectory) => {
                        " empty"
                    }
                    Kind::Repository => {
                        " repository"
                    }
                    Kind::File | Kind::Symlink | Kind::Directory => {
                        ""
                    }
                },
            )?;

            if may_remove_this_entry {
                let path = workdir.join(entry_path);
                if disk_kind.is_dir() {
                    std::fs::remove_dir_all(path)?;
                } else {
                    std::fs::remove_file(path)?;
                }
            } else {
                entries_to_clean += 1;
            }
        }
        if !execute {
            let mut messages = Vec::new();
            messages.extend((skipped_directories > 0).then(|| {
                format!(
                    "Skipped {skipped_directories} {directories} - show with -d",
                    directories = plural("directory", "directories", skipped_directories)
                )
            }));
            messages.extend((skipped_repositories > 0).then(|| {
                format!(
                    "Skipped {skipped_repositories} {repositories} - show with -r",
                    repositories = plural("repository", "repositories", skipped_repositories)
                )
            }));
            messages.extend((skipped_ignored > 0).then(|| {
                format!(
                    "Skipped {skipped_ignored} expendable {entries} - show with -x",
                    entries = plural("entry", "entries", skipped_ignored)
                )
            }));
            messages.extend((skipped_precious > 0).then(|| {
                format!(
                    "Skipped {skipped_precious} precious {entries} - show with -p",
                    entries = plural("entry", "entries", skipped_precious)
                )
            }));
            messages.extend((pruned_entries > 0 && has_patterns).then(|| {
                format!(
                    "try to adjust your pathspec to reveal some of the {pruned_entries} pruned {entries} - show with --debug",
                    entries = plural("entry", "entries", pruned_entries)
                )
            }));
            let make_msg = || -> String {
                if messages.is_empty() {
                    return String::new();
                }
                messages.join("; ")
            };
            let wrap_in_parens = |msg: String| if msg.is_empty() { msg } else { format!(" ({msg})") };
            if entries_to_clean > 0 {
                let mut wrote_nl = false;
                let msg = make_msg();
                let mut msg = if msg.is_empty() { None } else { Some(msg) };
                if saw_ignored_directory && skip_hidden_repositories.is_none() {
                    writeln!(err).ok();
                    wrote_nl = true;
                    writeln!(
                        err,
                        "WARNING: would remove repositories hidden inside ignored directories - use --skip-hidden-repositories to skip{}",
                        wrap_in_parens(msg.take().unwrap_or_default())
                    )?;
                }
                if saw_untracked_directory && matches!(find_untracked_repositories, FindRepository::NonBare) {
                    if !wrote_nl {
                        writeln!(err).ok();
                        wrote_nl = true;
                    }
                    writeln!(
                        err,
                        "WARNING: would remove repositories hidden inside untracked directories - use --find-untracked-repositories to find{}",
                        wrap_in_parens(msg.take().unwrap_or_default())
                    )?;
                }
                if let Some(msg) = msg.take() {
                    if !wrote_nl {
                        writeln!(err).ok();
                    }
                    writeln!(err, "{msg}").ok();
                }
            } else {
                writeln!(err, "Nothing to clean{}", wrap_in_parens(make_msg()))?;
            }
            if gix::interrupt::is_triggered() {
                writeln!(err, "Result may be incomplete as it was interrupted")?;
            }
        }
        Ok(())
    }

    fn plural<'a>(one: &'a str, many: &'a str, number: usize) -> &'a str {
        if number == 1 {
            one
        } else {
            many
        }
    }

    #[derive(Default)]
    struct InterruptibleCollect {
        inner: gix::dir::walk::delegate::Collect,
    }

    impl gix::dir::walk::Delegate for InterruptibleCollect {
        fn emit(&mut self, entry: EntryRef<'_>, collapsed_directory_status: Option<Status>) -> walk::Action {
            let res = self.inner.emit(entry, collapsed_directory_status);
            if gix::interrupt::is_triggered() {
                return walk::Action::Cancel;
            }
            res
        }
    }
}
