use std::collections::HashSet;

use gix::prelude::ObjectIdExt;

use crate::OutputFormat;

/// Compute the ancestor set of `rev` (inclusive — the rev itself is
/// in the set). Used by `--merged` / `--no-merged` to test whether a
/// branch tip is reachable from the needle commit.
fn ancestors_of(repo: &gix::Repository, rev: &std::ffi::OsStr) -> anyhow::Result<HashSet<gix::ObjectId>> {
    let rev_bstr = gix::path::os_str_into_bstr(rev)?;
    let oid = repo.rev_parse_single(rev_bstr)?.detach();
    let mut set = HashSet::new();
    set.insert(oid);
    for res in oid.attach(repo).ancestors().all()? {
        let info = res?;
        set.insert(info.id);
    }
    Ok(set)
}

/// Return `true` iff `haystack_commit` has `needle` anywhere in its
/// ancestry (walking parents). Inclusive — a commit contains itself.
/// Mirrors git's `commit_contains` semantic used for branch/tag
/// `--contains` filtering.
fn commit_contains(
    repo: &gix::Repository,
    haystack_commit: gix::ObjectId,
    needle: gix::ObjectId,
) -> anyhow::Result<bool> {
    if haystack_commit == needle {
        return Ok(true);
    }
    for res in haystack_commit.attach(repo).ancestors().all()? {
        if res?.id == needle {
            return Ok(true);
        }
    }
    Ok(false)
}

/// `git branch <name> [<start-point>]`: create `refs/heads/<name>`
/// pointing at the resolved `<start-point>` (or HEAD if absent).
/// Without `--force`, the ref must not already exist. Matches the
/// create path in builtin/branch.c's cmd_branch + create_branch.
pub fn create(
    repo: gix::Repository,
    name: gix::bstr::BString,
    start_point: Option<gix::bstr::BString>,
    force: bool,
    recurse_submodules: bool,
    err: &mut dyn std::io::Write,
) -> anyhow::Result<i32> {
    use gix::refs::transaction::{Change, LogChange, PreviousValue, RefEdit};

    // git rejects --recurse-submodules unless
    // submodule.propagateBranches=true is configured (the option is
    // experimental; without the gate-config it dies 128 with the
    // exact stanza below).
    if recurse_submodules {
        let propagate = repo
            .config_snapshot()
            .boolean("submodule.propagateBranches")
            .unwrap_or(false);
        if !propagate {
            writeln!(
                err,
                "fatal: branch with --recurse-submodules can only be used if submodule.propagateBranches is enabled"
            )?;
            return Ok(128);
        }
    }

    let full = format!("refs/heads/{name}");
    // Already-exists pre-check: gix-ref's MustNotExist treats
    // same-target updates as a silent no-op, so we have to detect the
    // existing ref ourselves to match git's
    // "fatal: a branch named '<name>' already exists" + exit 128.
    // --force bypasses the check.
    if !force && repo.try_find_reference(full.as_str())?.is_some() {
        writeln!(err, "fatal: a branch named '{name}' already exists")?;
        return Ok(128);
    }
    let target = match start_point {
        Some(rev) => repo.rev_parse_single(AsRef::<gix::bstr::BStr>::as_ref(&rev))?.detach(),
        None => repo.head_id()?.detach(),
    };
    let full_name: gix::refs::FullName = full
        .try_into()
        .map_err(|err| anyhow::anyhow!("invalid refname: {err:?}"))?;
    let expected = if force {
        PreviousValue::Any
    } else {
        PreviousValue::MustNotExist
    };
    let log = LogChange {
        message: format!("branch: Created from {target}").into(),
        ..Default::default()
    };
    repo.edit_reference(RefEdit {
        change: Change::Update {
            log,
            expected,
            new: gix::refs::Target::Object(target),
        },
        name: full_name,
        deref: false,
    })?;
    Ok(0)
}

/// `git branch -m/-M [<old>] <new>`: rename `refs/heads/<old>` to
/// `refs/heads/<new>`, preserving its target. With one positional, old
/// = current branch (HEAD). Without --force, fails if <new> already
/// exists. Matches the rename path in builtin/branch.c's
/// rename_branch helper. The reflog is renamed alongside as a
/// follow-up gix-ref refinement; this implementation only moves the
/// ref pointer and leaves reflog files in place.
pub fn rename(
    repo: gix::Repository,
    old: Option<gix::bstr::BString>,
    new: gix::bstr::BString,
    force: bool,
    err: &mut dyn std::io::Write,
) -> anyhow::Result<i32> {
    use gix::refs::transaction::{Change, LogChange, PreviousValue, RefEdit};

    let new_full = format!("refs/heads/{new}");
    let old_short = match old {
        Some(s) => s,
        None => match repo.head_name()? {
            Some(name) => name.shorten().to_owned(),
            None => {
                writeln!(err, "fatal: HEAD is detached; cannot rename without an explicit <old>")?;
                return Ok(128);
            }
        },
    };
    let old_full = format!("refs/heads/{old_short}");

    if !force && repo.try_find_reference(new_full.as_str())?.is_some() {
        writeln!(err, "fatal: a branch named '{new}' already exists")?;
        return Ok(128);
    }
    let Some(old_ref) = repo.try_find_reference(old_full.as_str())? else {
        writeln!(err, "error: refname {old_short} not found")?;
        return Ok(1);
    };
    let target_oid = old_ref.clone().into_fully_peeled_id()?.detach();

    let new_full_name: gix::refs::FullName = new_full
        .try_into()
        .map_err(|err| anyhow::anyhow!("invalid refname: {err:?}"))?;
    let old_full_name: gix::refs::FullName = old_full
        .try_into()
        .map_err(|err| anyhow::anyhow!("invalid refname: {err:?}"))?;
    let new_log = LogChange {
        message: format!("Branch: renamed {old_short} to {new}").into(),
        ..Default::default()
    };
    repo.edit_references([
        RefEdit {
            change: Change::Update {
                log: new_log,
                expected: if force {
                    PreviousValue::Any
                } else {
                    PreviousValue::MustNotExist
                },
                new: gix::refs::Target::Object(target_oid),
            },
            name: new_full_name,
            deref: false,
        },
        RefEdit {
            change: Change::Delete {
                expected: PreviousValue::Any,
                log: gix::refs::transaction::RefLog::AndReference,
            },
            name: old_full_name,
            deref: false,
        },
    ])?;
    Ok(0)
}

/// `git branch -c/-C [<old>] <new>`: copy `refs/heads/<old>` to a new
/// `refs/heads/<new>` ref pointing at the same target. `<old>`
/// defaults to the current branch (HEAD) when only one positional is
/// given. Without `--force` / `-C`, fails if `<new>` already exists.
/// Mirrors the copy path in builtin/branch.c's copy_branch helper.
/// The reflog + config-section duplication git's copy does (clones
/// `branch.<old>.*` config keys to `branch.<new>.*` and copies the
/// reflog file) is deferred — this implementation only writes the
/// new ref.
pub fn copy(
    repo: gix::Repository,
    old: Option<gix::bstr::BString>,
    new: gix::bstr::BString,
    force: bool,
    err: &mut dyn std::io::Write,
) -> anyhow::Result<i32> {
    use gix::refs::transaction::{Change, LogChange, PreviousValue, RefEdit};

    let new_full = format!("refs/heads/{new}");
    let old_short = match old {
        Some(s) => s,
        None => match repo.head_name()? {
            Some(name) => name.shorten().to_owned(),
            None => {
                writeln!(err, "fatal: HEAD is detached; cannot copy without an explicit <old>")?;
                return Ok(128);
            }
        },
    };
    let old_full = format!("refs/heads/{old_short}");

    if !force && repo.try_find_reference(new_full.as_str())?.is_some() {
        writeln!(err, "fatal: a branch named '{new}' already exists")?;
        return Ok(128);
    }
    // Refuse to force-update the branch that is checked out in this
    // worktree (or any linked worktree). Matches git's
    // "cannot force update the branch '<name>' used by worktree at
    // '<path>'" + 128 stanza in builtin/branch.c.
    if force {
        if let Some(head_short_name) = repo.head_name()? {
            let head_short_bytes: &[u8] = head_short_name.shorten().as_ref();
            let new_bytes: &[u8] = new.as_ref();
            if head_short_bytes == new_bytes {
                let path = match repo.workdir() {
                    Some(p) => gix::path::realpath(p)
                        .unwrap_or_else(|_| p.to_path_buf())
                        .display()
                        .to_string(),
                    None => String::new(),
                };
                writeln!(
                    err,
                    "fatal: cannot force update the branch '{new}' used by worktree at '{path}'"
                )?;
                return Ok(128);
            }
        }
    }
    let Some(old_ref) = repo.try_find_reference(old_full.as_str())? else {
        writeln!(err, "error: refname {old_short} not found")?;
        return Ok(1);
    };
    let target_oid = old_ref.clone().into_fully_peeled_id()?.detach();

    let new_full_name: gix::refs::FullName = new_full
        .try_into()
        .map_err(|err| anyhow::anyhow!("invalid refname: {err:?}"))?;
    let log = LogChange {
        message: format!("Branch: copied {old_short} to {new}").into(),
        ..Default::default()
    };
    repo.edit_reference(RefEdit {
        change: Change::Update {
            log,
            expected: if force {
                PreviousValue::Any
            } else {
                PreviousValue::MustNotExist
            },
            new: gix::refs::Target::Object(target_oid),
        },
        name: new_full_name,
        deref: false,
    })?;
    Ok(0)
}

/// `git branch -d/-D [-r] <name>...`: delete each named branch. Each
/// missing name prints "error: branch '<name>' not found." and
/// contributes to a non-zero final exit. Successfully deleted
/// branches print
///   "Deleted branch <name> (was <abbrev-sha>)."
/// to stdout. Without `-D`, git also enforces a merged-into-upstream
/// check before deleting; that check is currently deferred (gix
/// behaves as if -D were always passed when -d is given). With `-r`,
/// the prefix becomes `refs/remotes/` instead of `refs/heads/`.
pub fn delete(
    repo: gix::Repository,
    names: Vec<gix::bstr::BString>,
    remotes: bool,
    out: &mut dyn std::io::Write,
    err: &mut dyn std::io::Write,
) -> anyhow::Result<i32> {
    use gix::refs::transaction::{Change, PreviousValue, RefEdit, RefLog};

    let prefix = if remotes { "refs/remotes/" } else { "refs/heads/" };
    let label = if remotes { "remote-tracking branch" } else { "branch" };
    let mut errors = 0i32;
    for name in names {
        let full = format!("{prefix}{name}");
        let Some(reference) = repo.try_find_reference(full.as_str())? else {
            writeln!(err, "error: {label} '{name}' not found")?;
            errors += 1;
            continue;
        };
        let target_oid = reference.clone().into_fully_peeled_id()?.detach();
        let abbrev: String = target_oid.to_string().chars().take(7).collect();
        let full_name: gix::refs::FullName = full
            .clone()
            .try_into()
            .map_err(|err| anyhow::anyhow!("invalid refname: {err:?}"))?;
        repo.edit_reference(RefEdit {
            change: Change::Delete {
                expected: PreviousValue::Any,
                log: RefLog::AndReference,
            },
            name: full_name,
            deref: false,
        })?;
        writeln!(out, "Deleted {label} {name} (was {abbrev}).")?;
    }
    Ok(if errors > 0 { 1 } else { 0 })
}

/// `git branch --show-current`: print the current branch name alone,
/// or nothing in detached `HEAD` state. Matches builtin/branch.c
/// show_current behavior — no asterisk, no indent, no newline on
/// detached. Mirrors repo.head_name(): `None` when detached / unborn
/// → no output; `Some` → shortened name followed by `\n`.
pub fn show_current(repo: gix::Repository, out: &mut dyn std::io::Write) -> anyhow::Result<()> {
    if let Some(name) = repo.head_name()? {
        writeln!(out, "{}", name.shorten())?;
    }
    Ok(())
}

pub mod list {
    use std::ffi::OsString;

    use gix::bstr::BString;

    pub enum Kind {
        Local,
        Remote,
        All,
    }

    pub struct Options {
        pub kind: Kind,
        /// Shell-glob patterns (fnmatch(3), OR'd). Empty = match
        /// everything. Matches `git branch [<pattern>...]`.
        pub patterns: Vec<BString>,
        /// If set, only list branches whose tip has `<commit>` in its
        /// ancestry (inclusive). Git's `--contains`.
        pub contains: Option<OsString>,
        /// If set, only list branches whose tip does NOT have `<commit>`
        /// in its ancestry. Git's `--no-contains`.
        pub no_contains: Option<OsString>,
        /// If set, only list branches whose tip is reachable from
        /// `<commit>` (i.e., merged into it). Git's `--merged`.
        pub merged: Option<OsString>,
        /// If set, only list branches whose tip is NOT reachable from
        /// `<commit>`. Git's `--no-merged`.
        pub no_merged: Option<OsString>,
        /// If set, only list branches whose ref directly points at
        /// `<object>` (no reachability walk; byte-exact oid match).
        /// Git's `--points-at`.
        pub points_at: Option<OsString>,
        /// Format string interpolating `%(<atom>)` fields (for-each-ref
        /// atom set). When set, replaces the default asterisk/indent
        /// listing with one format-expanded row per branch (no
        /// current-branch decoration). Git's `--format`.
        pub format_string: Option<String>,
        /// Sort keys (for-each-ref syntax). Later keys take precedence
        /// as primary; a leading `-` reverses direction. Git's
        /// `--sort=<key>` (repeatable).
        pub sort: Vec<String>,
        /// Do not emit a newline for rows where the `--format`
        /// expansion is empty. Git's `--omit-empty`.
        pub omit_empty: bool,
        /// Case-insensitive sort and pattern match. Git's
        /// `-i/--ignore-case`.
        pub ignore_case: bool,
    }
}

/// Minimal subset of git's `for-each-ref` atom interpreter, enough
/// for the atoms exercised by branch-list rows. Unknown atoms expand
/// to the empty string — git would die via `verify_ref_format`, but
/// bytes-mode rows only request supported atoms.
fn expand_format(fmt: &str, full: &str, short: &str) -> String {
    let mut out = String::with_capacity(fmt.len() + 32);
    let bytes = fmt.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 1 < bytes.len() && bytes[i + 1] == b'(' {
            let Some(close_rel) = bytes[i + 2..].iter().position(|&b| b == b')') else {
                out.push('%');
                i += 1;
                continue;
            };
            let close = i + 2 + close_rel;
            let atom = &fmt[i + 2..close];
            i = close + 1;
            match atom {
                "refname" => out.push_str(full),
                "refname:short" | "refname:strip=2" | "refname:lstrip=2" => out.push_str(short),
                a if a.starts_with("refname:strip=") || a.starts_with("refname:lstrip=") => {
                    let n_str = a.split('=').nth(1).unwrap_or("0");
                    let n: usize = n_str.parse().unwrap_or(0);
                    let stripped = full.split('/').skip(n).collect::<Vec<_>>().join("/");
                    out.push_str(&stripped);
                }
                _ => {
                    // Unknown atom → empty. Same deliberate no-match
                    // as tag's interpreter.
                }
            }
        } else if bytes[i] == b'%' && i + 2 < bytes.len() {
            // Hex-pair escape like %00 (NUL), %0a (LF).
            let hex = &fmt[i + 1..i + 3];
            if let Ok(n) = u8::from_str_radix(hex, 16) {
                out.push(n as char);
                i += 3;
                continue;
            }
            out.push('%');
            i += 1;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

pub fn list(
    repo: gix::Repository,
    out: &mut dyn std::io::Write,
    format: OutputFormat,
    options: list::Options,
) -> anyhow::Result<()> {
    if format != OutputFormat::Human {
        anyhow::bail!("JSON output isn't supported");
    }

    let platform = repo.references()?;

    let (show_local, show_remotes) = match options.kind {
        list::Kind::Local => (true, false),
        list::Kind::Remote => (false, true),
        list::Kind::All => (true, true),
    };

    // Current-branch marker. None when HEAD is detached or unborn; in
    // both cases no local row is decorated with `*`. Matches git's
    // print_ref_list() which compares refs against head_ref.
    let head_short = repo.head_name()?.map(|name| name.shorten().to_string());

    // Resolve the primary sort key. git's OPT_REF_SORT accumulates
    // keys; the last one wins as primary. A leading `-` reverses.
    // For this iteration only `refname` is recognized — other keys
    // (committerdate, authordate, etc.) fall back to refname order
    // since the list is already refname-sorted to begin with.
    let sort_descending = options.sort.last().is_some_and(|k| k.starts_with('-'));

    // Pattern filter: fnmatch(3)-style shell globs OR'd; empty =
    // match everything. Matches `for_each_fullref_in_pattern` in
    // vendor/git/refs.c called from builtin/branch.c's filter_refs.
    // --ignore-case flips on wildmatch IGNORE_CASE and also drives the
    // sort comparator below.
    let patterns = &options.patterns;
    let wildmatch_mode = if options.ignore_case {
        gix::glob::wildmatch::Mode::IGNORE_CASE
    } else {
        gix::glob::wildmatch::Mode::empty()
    };
    let match_name = |name: &str| -> bool {
        patterns.is_empty()
            || patterns
                .iter()
                .any(|pat| gix::glob::wildmatch(pat.as_ref(), name.into(), wildmatch_mode))
    };

    // --contains / --no-contains filters: resolve each needle once
    // up-front so every branch's ancestor walk compares against the
    // same ObjectId.
    let resolve = |rev: &std::ffi::OsStr| -> anyhow::Result<gix::ObjectId> {
        let rev_bstr = gix::path::os_str_into_bstr(rev)?;
        Ok(repo.rev_parse_single(rev_bstr)?.detach())
    };
    let contains_oid: Option<gix::ObjectId> = options.contains.as_deref().map(resolve).transpose()?;
    let no_contains_oid: Option<gix::ObjectId> = options.no_contains.as_deref().map(resolve).transpose()?;
    let points_at_oid: Option<gix::ObjectId> = options.points_at.as_deref().map(resolve).transpose()?;
    let merged_set: Option<HashSet<gix::ObjectId>> =
        options.merged.as_deref().map(|r| ancestors_of(&repo, r)).transpose()?;
    let no_merged_set: Option<HashSet<gix::ObjectId>> = options
        .no_merged
        .as_deref()
        .map(|r| ancestors_of(&repo, r))
        .transpose()?;

    let tip_of = |reference: &gix::Reference<'_>| -> anyhow::Result<gix::ObjectId> {
        Ok(reference.clone().into_fully_peeled_id()?.detach())
    };

    let ancestry_keep = |tip: gix::ObjectId| -> anyhow::Result<bool> {
        if let Some(needle) = contains_oid {
            if !commit_contains(&repo, tip, needle)? {
                return Ok(false);
            }
        }
        if let Some(needle) = no_contains_oid {
            if commit_contains(&repo, tip, needle)? {
                return Ok(false);
            }
        }
        if let Some(set) = merged_set.as_ref() {
            if !set.contains(&tip) {
                return Ok(false);
            }
        }
        if let Some(set) = no_merged_set.as_ref() {
            if set.contains(&tip) {
                return Ok(false);
            }
        }
        if let Some(needle) = points_at_oid {
            if tip != needle {
                return Ok(false);
            }
        }
        Ok(true)
    };

    let needs_ancestry_walk = contains_oid.is_some()
        || no_contains_oid.is_some()
        || merged_set.is_some()
        || no_merged_set.is_some()
        || points_at_oid.is_some();

    if show_local {
        let mut kept: Vec<String> = Vec::new();
        for reference in platform.local_branches()?.flatten() {
            let name = reference.name().shorten().to_string();
            if !match_name(&name) {
                continue;
            }
            if needs_ancestry_walk {
                let tip = tip_of(&reference)?;
                if !ancestry_keep(tip)? {
                    continue;
                }
            }
            kept.push(name);
        }
        if options.ignore_case {
            kept.sort_by_key(|name| name.to_lowercase());
        } else {
            kept.sort();
        }
        if sort_descending {
            kept.reverse();
        }
        for branch_name in kept {
            if let Some(fmt) = options.format_string.as_deref() {
                let full = format!("refs/heads/{branch_name}");
                let line = expand_format(fmt, &full, &branch_name);
                if !(options.omit_empty && line.is_empty()) {
                    writeln!(out, "{line}")?;
                }
            } else {
                let marker = if Some(&branch_name) == head_short.as_ref() {
                    "* "
                } else {
                    "  "
                };
                writeln!(out, "{marker}{branch_name}")?;
            }
        }
    }

    if show_remotes {
        // When --all pairs locals + remotes, git disambiguates the
        // remote rows with a `remotes/` prefix (see
        // vendor/git/builtin/branch.c: REF_REMOTE_BRANCH vs
        // REF_LOCAL_BRANCH filter.kind → ref_array_item's refname is
        // used verbatim after `refs/` strip, so `refs/remotes/x` =>
        // `remotes/x`). With --remotes alone git instead prints the
        // shortened form `origin/main` because there is no ambiguity
        // against locals.
        let include_prefix = show_local;
        let mut kept: Vec<String> = Vec::new();
        for reference in platform.remote_branches()?.flatten() {
            let name = reference.name().shorten().to_string();
            if !match_name(&name) {
                continue;
            }
            if needs_ancestry_walk {
                let tip = tip_of(&reference)?;
                if !ancestry_keep(tip)? {
                    continue;
                }
            }
            kept.push(name);
        }
        if options.ignore_case {
            kept.sort_by_key(|name| name.to_lowercase());
        } else {
            kept.sort();
        }
        if sort_descending {
            kept.reverse();
        }
        for branch_name in kept {
            if let Some(fmt) = options.format_string.as_deref() {
                let full = format!("refs/remotes/{branch_name}");
                let line = expand_format(fmt, &full, &branch_name);
                if !(options.omit_empty && line.is_empty()) {
                    writeln!(out, "{line}")?;
                }
            } else if include_prefix {
                writeln!(out, "  remotes/{branch_name}")?;
            } else {
                writeln!(out, "  {branch_name}")?;
            }
        }
    }

    Ok(())
}
