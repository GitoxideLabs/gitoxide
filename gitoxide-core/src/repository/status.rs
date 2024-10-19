use anyhow::bail;
use gix::bstr::{BStr, BString, ByteSlice};
use gix::status::index_worktree::iter::Item;
use gix_status::index_as_worktree::{Change, Conflict, EntryStatus};
use std::path::Path;

use crate::OutputFormat;

pub enum Submodules {
    /// display all information about submodules, including ref changes, modifications and untracked files.
    All,
    /// Compare only the configuration of the superprojects commit with the actually checked out `HEAD` commit.
    RefChange,
    /// See if there are worktree modifications compared to the index, but do not check for untracked files.
    Modifications,
    /// Ignore all submodule changes.
    None,
}

#[derive(Copy, Clone)]
pub enum Ignored {
    Collapsed,
    Matching,
}

#[derive(Copy, Clone)]
pub enum Format {
    Simplified,
    PorcelainV2,
}

pub struct Options {
    pub ignored: Option<Ignored>,
    pub format: Format,
    pub output_format: OutputFormat,
    pub submodules: Option<Submodules>,
    pub thread_limit: Option<usize>,
    pub statistics: bool,
    pub allow_write: bool,
    pub index_worktree_renames: Option<f32>,
}

pub fn show(
    repo: gix::Repository,
    pathspecs: Vec<BString>,
    mut out: impl std::io::Write,
    mut err: impl std::io::Write,
    mut progress: impl gix::NestedProgress + 'static,
    Options {
        ignored,
        format,
        output_format,
        submodules,
        thread_limit,
        allow_write,
        statistics,
        index_worktree_renames,
    }: Options,
) -> anyhow::Result<()> {
    if output_format != OutputFormat::Human {
        bail!("Only human format is supported right now");
    }
    if !matches!(format, Format::Simplified) {
        bail!("Only the simplified format is currently implemented");
    }

    let start = std::time::Instant::now();
    let prefix = repo.prefix()?.unwrap_or(Path::new(""));
    let index_progress = progress.add_child("traverse index");
    let mut iter = repo
        .status(index_progress)?
        .should_interrupt_shared(&gix::interrupt::IS_INTERRUPTED)
        .index_worktree_options_mut(|opts| {
            if let Some((opts, ignored)) = opts.dirwalk_options.as_mut().zip(ignored) {
                opts.set_emit_ignored(Some(match ignored {
                    Ignored::Collapsed => gix::dir::walk::EmissionMode::CollapseDirectory,
                    Ignored::Matching => gix::dir::walk::EmissionMode::Matching,
                }));
            }
            opts.rewrites = index_worktree_renames.map(|percentage| gix::diff::Rewrites {
                copies: None,
                percentage: Some(percentage),
                limit: 0,
            });
            if opts.rewrites.is_some() {
                if let Some(opts) = opts.dirwalk_options.as_mut() {
                    opts.set_emit_untracked(gix::dir::walk::EmissionMode::Matching);
                    if ignored.is_some() {
                        opts.set_emit_ignored(Some(gix::dir::walk::EmissionMode::Matching));
                    }
                }
            }
            opts.thread_limit = thread_limit;
            opts.sorting = Some(gix::status::plumbing::index_as_worktree_with_renames::Sorting::ByPathCaseSensitive);
        })
        .index_worktree_submodules(match submodules {
            Some(mode) => {
                let ignore = match mode {
                    Submodules::All => gix::submodule::config::Ignore::None,
                    Submodules::RefChange => gix::submodule::config::Ignore::Dirty,
                    Submodules::Modifications => gix::submodule::config::Ignore::Untracked,
                    Submodules::None => gix::submodule::config::Ignore::All,
                };
                gix::status::Submodule::Given {
                    ignore,
                    check_dirty: false,
                }
            }
            None => gix::status::Submodule::AsConfigured { check_dirty: false },
        })
        .into_index_worktree_iter(pathspecs)?;

    for item in iter.by_ref() {
        let item = item?;
        match item {
            Item::Modification {
                entry: _,
                entry_index: _,
                rela_path,
                status,
            } => print_index_entry_status(&mut out, prefix, rela_path.as_ref(), status)?,
            Item::DirectoryContents {
                entry,
                collapsed_directory_status,
            } => {
                if collapsed_directory_status.is_none() {
                    writeln!(
                        out,
                        "{status: >3} {rela_path}{slash}",
                        status = "?",
                        rela_path =
                            gix::path::relativize_with_prefix(&gix::path::from_bstr(entry.rela_path), prefix).display(),
                        slash = if entry.disk_kind.is_dir() { "/" } else { "" }
                    )?;
                }
            }
            Item::Rewrite {
                source,
                dirwalk_entry,
                copy: _, // TODO: how to visualize copies?
                ..
            } => {
                // TODO: handle multi-status characters, there can also be modifications at the same time as determined by their ID and potentially diffstats.
                writeln!(
                    out,
                    "{status: >3} {source_rela_path} → {dest_rela_path}",
                    status = "R",
                    source_rela_path =
                        gix::path::relativize_with_prefix(&gix::path::from_bstr(source.rela_path()), prefix).display(),
                    dest_rela_path = gix::path::relativize_with_prefix(
                        &gix::path::from_bstr(dirwalk_entry.rela_path.as_bstr()),
                        prefix
                    )
                    .display(),
                )?;
            }
        }
    }
    if gix::interrupt::is_triggered() {
        bail!("interrupted by user");
    }

    let out = iter.outcome_mut().expect("successful iteration has outcome");

    if out.has_changes() && allow_write {
        out.write_changes().transpose()?;
    }

    if statistics {
        writeln!(err, "{outcome:#?}", outcome = out.index_worktree).ok();
    }

    writeln!(err, "\nhead -> index isn't implemented yet")?;
    progress.init(Some(out.index.entries().len()), gix::progress::count("files"));
    progress.set(out.index.entries().len());
    progress.show_throughput(start);
    Ok(())
}

fn print_index_entry_status(
    out: &mut dyn std::io::Write,
    prefix: &Path,
    rela_path: &BStr,
    status: EntryStatus<(), gix::submodule::Status>,
) -> std::io::Result<()> {
    let char_storage;
    let status = match status {
        EntryStatus::Conflict(conflict) => as_str(conflict),
        EntryStatus::Change(change) => {
            char_storage = change_to_char(&change);
            std::str::from_utf8(std::slice::from_ref(&char_storage)).expect("valid ASCII")
        }
        EntryStatus::NeedsUpdate(_stat) => {
            return Ok(());
        }
        EntryStatus::IntentToAdd => "A",
    };

    let rela_path = gix::path::from_bstr(rela_path);
    let display_path = gix::path::relativize_with_prefix(&rela_path, prefix);
    writeln!(out, "{status: >3} {}", display_path.display())
}

fn as_str(c: Conflict) -> &'static str {
    match c {
        Conflict::BothDeleted => "DD",
        Conflict::AddedByUs => "AU",
        Conflict::DeletedByThem => "UD",
        Conflict::AddedByThem => "UA",
        Conflict::DeletedByUs => "DU",
        Conflict::BothAdded => "AA",
        Conflict::BothModified => "UU",
    }
}

fn change_to_char(change: &Change<(), gix::submodule::Status>) -> u8 {
    // Known status letters: https://github.com/git/git/blob/6807fcfedab84bc8cd0fbf721bc13c4e68cda9ae/diff.h#L613
    match change {
        Change::Removed => b'D',
        Change::Type => b'T',
        Change::SubmoduleModification(_) => b'M',
        Change::Modification {
            executable_bit_changed, ..
        } => {
            if *executable_bit_changed {
                b'X'
            } else {
                b'M'
            }
        }
    }
}
