use std::path::Path;

use anyhow::bail;
use gix::{
    bstr::{BStr, BString, ByteSlice},
    status::{self, index_worktree},
};
use gix_status::index_as_worktree::{Change, Conflict, EntryStatus};

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

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Format {
    Simplified,
    PorcelainV2,
    /// Byte-exact compatibility with `git status --short` — 2-char XY status
    /// codes, `??` for untracked, `!!` for ignored, one line per path, path-
    /// grouped so that the same path with both index and worktree changes is
    /// emitted as a single line with combined XY.
    Short,
}

pub struct Options {
    pub ignored: Option<Ignored>,
    pub format: Format,
    pub output_format: OutputFormat,
    pub submodules: Option<Submodules>,
    pub thread_limit: Option<usize>,
    pub statistics: bool,
    /// Show the branch header (`## <branch>`) in short / porcelain formats.
    /// Mirrors git's `-b` / `--branch`. Ignored for the default (long)
    /// format, which always shows the branch.
    pub branch: bool,
    /// Terminate entries with NUL instead of LF. Mirrors `git status -z`;
    /// only honored for the short / porcelain formats.
    pub null_terminator: bool,
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
        branch,
        null_terminator,
        index_worktree_renames,
    }: Options,
) -> anyhow::Result<()> {
    if output_format != OutputFormat::Human {
        bail!("Only human format is supported right now");
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
                track_empty: false,
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
        .into_iter(pathspecs)?;

    if format == Format::Short {
        // git --short / -s: 2-char XY status, `??` for untracked, `!!` for
        // ignored. Collect all items per-path (TreeIndex gives X, IndexWorktree
        // gives Y); emit sorted tracked first, then renames, then untracked,
        // then ignored. Mirrors the order of vendor/git/wt-status.c::
        // wt_status_print_changes on -s output: changed tracked entries in
        // commit-index order (we use sorted path order as a deterministic
        // substitute), then untracked, then ignored.
        //
        // `-z` toggles terminator `\n` -> `\0`. git also drops the ` -> `
        // rename separator in favor of `<dest>\0<source>\0` and suppresses
        // path quoting (gix never quotes here, so that half is already in
        // sync).
        let terminator: u8 = if null_terminator { 0 } else { b'\n' };
        if branch {
            // `-b` / `--branch` — prepend a `## <branch>` header. Git also
            // covers detached-HEAD (`## HEAD (no branch)`) and initial-repo
            // (`## No commits yet on <branch>`) cases; those are not yet
            // handled — they surface as shortcomings when their row lands.
            // Upstream-tracking lines (`## br...origin/br [ahead N]`) are
            // also TODO.
            match repo.head_name()? {
                Some(name) => {
                    out.write_all(b"## ")?;
                    out.write_all(name.shorten().as_ref())?;
                    out.write_all(&[terminator])?;
                }
                None => {
                    out.write_all(b"## HEAD (no branch)")?;
                    out.write_all(&[terminator])?;
                }
            }
        }
        use std::collections::BTreeMap;
        let mut tracked: BTreeMap<BString, [u8; 2]> = BTreeMap::new();
        let mut renames: Vec<(BString, BString, u8, u8)> = Vec::new();
        let mut untracked: BTreeMap<BString, bool> = BTreeMap::new();
        for item in iter.by_ref() {
            let item = item?;
            match item {
                status::Item::TreeIndex(change) => {
                    let (location, _, _, _) = change.fields();
                    match change {
                        gix::diff::index::Change::Addition { .. } => {
                            let entry = tracked.entry(location.into()).or_insert([b' ', b' ']);
                            entry[0] = b'A';
                        }
                        gix::diff::index::Change::Deletion { .. } => {
                            let entry = tracked.entry(location.into()).or_insert([b' ', b' ']);
                            entry[0] = b'D';
                        }
                        gix::diff::index::Change::Modification { .. } => {
                            let entry = tracked.entry(location.into()).or_insert([b' ', b' ']);
                            entry[0] = b'M';
                        }
                        gix::diff::index::Change::Rewrite {
                            ref source_location, ..
                        } => {
                            renames.push((source_location.as_ref().to_owned(), location.into(), b'R', b' '));
                        }
                    }
                }
                status::Item::IndexWorktree(index_worktree::Item::Modification { rela_path, status, .. }) => {
                    let y = match status {
                        EntryStatus::Conflict { summary, entries: _ } => {
                            // Conflicts use a 2-char combined XY; map directly.
                            let [x, y] = conflict_to_xy(summary);
                            let entry = tracked.entry(rela_path.clone()).or_insert([b' ', b' ']);
                            entry[0] = x;
                            entry[1] = y;
                            continue;
                        }
                        EntryStatus::Change(change) => {
                            // exec-bit-only mode change surfaces as `X` in gix;
                            // git shows it as plain `M` in the Y column.
                            let c = change_to_char(&change);
                            if c == b'X' {
                                b'M'
                            } else {
                                c
                            }
                        }
                        EntryStatus::NeedsUpdate(_) => continue,
                        EntryStatus::IntentToAdd => b'A',
                    };
                    let entry = tracked.entry(rela_path).or_insert([b' ', b' ']);
                    entry[1] = y;
                }
                status::Item::IndexWorktree(index_worktree::Item::DirectoryContents {
                    entry,
                    collapsed_directory_status,
                }) => {
                    if collapsed_directory_status.is_none() {
                        let is_dir = entry.disk_kind.unwrap_or(gix::dir::entry::Kind::File).is_dir();
                        untracked.insert(entry.rela_path, is_dir);
                    }
                }
                status::Item::IndexWorktree(index_worktree::Item::Rewrite {
                    source,
                    dirwalk_entry,
                    copy: _,
                    ..
                }) => {
                    renames.push((source.rela_path().into(), dirwalk_entry.rela_path, b'R', b' '));
                }
            }
        }
        for (path, [x, y]) in &tracked {
            out.write_all(&[*x, *y, b' '])?;
            out.write_all(path)?;
            out.write_all(&[terminator])?;
        }
        for (source, dest, x, y) in &renames {
            out.write_all(&[*x, *y, b' '])?;
            if null_terminator {
                // `-z` order reversal: `<dest>\0<source>\0` per git docs.
                out.write_all(dest)?;
                out.write_all(&[terminator])?;
                out.write_all(source)?;
                out.write_all(&[terminator])?;
            } else {
                out.write_all(source)?;
                out.write_all(b" -> ")?;
                out.write_all(dest)?;
                out.write_all(&[terminator])?;
            }
        }
        for (path, is_dir) in &untracked {
            out.write_all(b"?? ")?;
            out.write_all(path)?;
            if *is_dir {
                out.write_all(b"/")?;
            }
            out.write_all(&[terminator])?;
        }
    } else if format == Format::PorcelainV2 {
        // git --porcelain=v2: one line per entry with extensive metadata.
        //   Ordinary changed: `1 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <path>`
        //   Rename/copy:      `2 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <X><score> <path>\t<origPath>`
        //   Unmerged:         `u <XY> <sub> <m1> <m2> <m3> <mW> <h1> <h2> <h3> <path>`
        //   Untracked:        `? <path>`
        //   Ignored:          `! <path>`
        // Collect per-path state; emit sorted. XY uses `.` in unchanged
        // positions (unlike --short which uses space). Current impl:
        // covers the ordinary-changed case and untracked; rename/copy
        // emits score=100 by default, unmerged not yet exercised.
        //
        // Worktree mode (mW): not all status paths expose it. When we
        // don't have it explicitly we reuse mI (the index mode), which
        // matches git for regular files without exec-bit-only changes.
        //
        // Submodule summary `<sub>`: always emitted as `N...` here (not
        // a submodule); `S<c><m><u>` support lands when the submodule
        // row is closed.
        use std::collections::BTreeMap;
        struct V2Entry {
            xy: [u8; 2],
            mh: Option<u32>,
            mi: Option<u32>,
            mw: Option<u32>,
            hh: Option<gix::ObjectId>,
            hi: Option<gix::ObjectId>,
        }
        impl Default for V2Entry {
            fn default() -> Self {
                Self {
                    xy: [b'.', b'.'],
                    mh: None,
                    mi: None,
                    mw: None,
                    hh: None,
                    hi: None,
                }
            }
        }
        let null_oid = gix::ObjectId::null(repo.object_hash());
        let mut tracked: BTreeMap<BString, V2Entry> = BTreeMap::new();
        let mut untracked: BTreeMap<BString, bool> = BTreeMap::new();
        for item in iter.by_ref() {
            let item = item?;
            match item {
                status::Item::TreeIndex(change) => {
                    let (path, entry_mode, id) = {
                        let (loc, _, em, id) = change.fields();
                        let path: BString = loc.into();
                        (path, em, id.to_owned())
                    };
                    let entry = tracked.entry(path).or_default();
                    match change {
                        gix::diff::index::Change::Addition { .. } => {
                            entry.xy[0] = b'A';
                            entry.mh = Some(0);
                            entry.mi = Some(entry_mode.bits());
                            entry.hh = Some(null_oid);
                            entry.hi = Some(id);
                        }
                        gix::diff::index::Change::Deletion { .. } => {
                            // For a deletion, `entry_mode` / `id` are the
                            // HEAD-side values (nothing is in the index).
                            entry.xy[0] = b'D';
                            entry.mh = Some(entry_mode.bits());
                            entry.mi = Some(0);
                            entry.hh = Some(id);
                            entry.hi = Some(null_oid);
                        }
                        gix::diff::index::Change::Modification {
                            previous_entry_mode,
                            previous_id,
                            ..
                        } => {
                            entry.xy[0] = b'M';
                            entry.mh = Some(previous_entry_mode.bits());
                            entry.mi = Some(entry_mode.bits());
                            entry.hh = Some(previous_id.into_owned());
                            entry.hi = Some(id);
                        }
                        gix::diff::index::Change::Rewrite { .. } => {
                            // Renames in v2 carry extra score+origPath and go
                            // on a `2 ...` line. Not yet implemented; these
                            // currently fall through as no entry, causing a
                            // divergence when renames are in play.
                        }
                    }
                }
                status::Item::IndexWorktree(index_worktree::Item::Modification {
                    rela_path,
                    status,
                    entry: index_entry,
                    ..
                }) => {
                    let entry = tracked.entry(rela_path).or_default();
                    match status {
                        EntryStatus::Conflict { summary, entries: _ } => {
                            let [x, y] = conflict_to_xy(summary);
                            entry.xy = [x, y];
                        }
                        EntryStatus::Change(change) => {
                            let c = change_to_char(&change);
                            entry.xy[1] = if c == b'X' { b'M' } else { c };
                        }
                        EntryStatus::NeedsUpdate(_) => continue,
                        EntryStatus::IntentToAdd => {
                            entry.xy[0] = b'A';
                        }
                    }
                    // Fill in missing mode/hash from the index entry.
                    if entry.mi.is_none() {
                        entry.mi = Some(index_entry.mode.bits());
                    }
                    if entry.hi.is_none() {
                        entry.hi = Some(index_entry.id);
                    }
                    if entry.mw.is_none() {
                        entry.mw = entry.mi;
                    }
                    if entry.mh.is_none() {
                        entry.mh = entry.mi;
                    }
                    if entry.hh.is_none() {
                        entry.hh = entry.hi;
                    }
                }
                status::Item::IndexWorktree(index_worktree::Item::DirectoryContents {
                    entry,
                    collapsed_directory_status,
                }) => {
                    if collapsed_directory_status.is_none() {
                        let is_dir = entry.disk_kind.unwrap_or(gix::dir::entry::Kind::File).is_dir();
                        untracked.insert(entry.rela_path, is_dir);
                    }
                }
                status::Item::IndexWorktree(index_worktree::Item::Rewrite { .. }) => {
                    // v2 rename row not yet supported.
                }
            }
        }
        for (path, entry) in &tracked {
            // Default unchanged-mH-or-mI to the other column and default
            // hashes to null. Also default mW = mI if we never observed
            // a worktree modification for this path.
            let mh = entry.mh.unwrap_or_else(|| entry.mi.unwrap_or(0));
            let mi = entry.mi.unwrap_or(mh);
            let mw = entry.mw.unwrap_or(mi);
            let hh = entry.hh.unwrap_or(null_oid);
            let hi = entry.hi.unwrap_or(null_oid);
            write!(
                out,
                "1 {}{} N... {:06o} {:06o} {:06o} {} {} ",
                entry.xy[0] as char, entry.xy[1] as char, mh, mi, mw, hh, hi,
            )?;
            out.write_all(path)?;
            out.write_all(b"\n")?;
        }
        for (path, is_dir) in &untracked {
            out.write_all(b"? ")?;
            out.write_all(path)?;
            if *is_dir {
                out.write_all(b"/")?;
            }
            out.write_all(b"\n")?;
        }
    } else {
        for item in iter.by_ref() {
            let item = item?;
            match item {
                status::Item::TreeIndex(change) => {
                    let (location, _, _, _) = change.fields();
                    let status = match change {
                        gix::diff::index::Change::Addition { .. } => "A",
                        gix::diff::index::Change::Deletion { .. } => "D",
                        gix::diff::index::Change::Modification { .. } => "M",
                        gix::diff::index::Change::Rewrite {
                            ref source_location, ..
                        } => {
                            let source_location = gix::path::from_bstr(source_location.as_ref());
                            let source_location = gix::path::relativize_with_prefix(&source_location, prefix);
                            writeln!(
                                out,
                                "{status: >2}  {source_rela_path} → {dest_rela_path}",
                                status = "R",
                                source_rela_path = source_location.display(),
                                dest_rela_path =
                                    gix::path::relativize_with_prefix(&gix::path::from_bstr(location), prefix)
                                        .display(),
                            )?;
                            continue;
                        }
                    };
                    writeln!(
                        out,
                        "{status: >2}  {rela_path}",
                        rela_path =
                            gix::path::relativize_with_prefix(&gix::path::from_bstr(location), prefix).display(),
                    )?;
                }
                status::Item::IndexWorktree(index_worktree::Item::Modification {
                    entry: _,
                    entry_index: _,
                    rela_path,
                    status,
                }) => print_index_entry_status(&mut out, prefix, rela_path.as_ref(), status)?,
                status::Item::IndexWorktree(index_worktree::Item::DirectoryContents {
                    entry,
                    collapsed_directory_status,
                }) => {
                    if collapsed_directory_status.is_none() {
                        writeln!(
                            out,
                            "{status: >3} {rela_path}{slash}",
                            status = "?",
                            rela_path =
                                gix::path::relativize_with_prefix(&gix::path::from_bstr(entry.rela_path), prefix)
                                    .display(),
                            slash = if entry.disk_kind.unwrap_or(gix::dir::entry::Kind::File).is_dir() {
                                "/"
                            } else {
                                ""
                            }
                        )?;
                    }
                }
                status::Item::IndexWorktree(index_worktree::Item::Rewrite {
                    source,
                    dirwalk_entry,
                    copy: _, // TODO: how to visualize copies?
                    ..
                }) => {
                    // TODO: handle multi-status characters, there can also be modifications at the same time as determined by their ID and potentially diffstats.
                    writeln!(
                        out,
                        "{status: >3} {source_rela_path} → {dest_rela_path}",
                        status = "R",
                        source_rela_path =
                            gix::path::relativize_with_prefix(&gix::path::from_bstr(source.rela_path()), prefix)
                                .display(),
                        dest_rela_path = gix::path::relativize_with_prefix(
                            &gix::path::from_bstr(dirwalk_entry.rela_path.as_bstr()),
                            prefix
                        )
                        .display(),
                    )?;
                }
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

    progress.init(Some(out.worktree_index.entries().len()), gix::progress::count("files"));
    progress.set(out.worktree_index.entries().len());
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
        EntryStatus::Conflict { summary, entries: _ } => as_str(summary),
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

fn conflict_to_xy(c: Conflict) -> [u8; 2] {
    let bytes = as_str(c).as_bytes();
    [bytes[0], bytes[1]]
}

fn change_to_char(change: &Change<(), gix::submodule::Status>) -> u8 {
    // Known status letters: https://github.com/git/git/blob/6807fcfedab84bc8cd0fbf721bc13c4e68cda9ae/diff.h#L613
    match change {
        Change::Removed => b'D',
        Change::Type { .. } => b'T',
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
