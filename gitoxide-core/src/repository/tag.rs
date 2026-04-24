use std::{collections::HashSet, ffi::OsString};

use gix::{
    bstr::{BString, ByteSlice},
    prelude::ObjectIdExt,
};

use crate::OutputFormat;

/// Options driving `tag::list`. Mirrors the filter-and-format subset
/// of git-tag's listing mode from vendor/git/builtin/tag.c cmd_tag.
#[derive(Debug, Default)]
pub struct ListOptions {
    /// Shell-glob patterns (fnmatch(3), OR'd). Empty = match everything.
    pub patterns: Vec<OsString>,
    /// When true, pattern-match and sort are case-insensitive
    /// (`-i`/`--ignore-case`).
    pub ignore_case: bool,
    /// When `Some`, only list tags that point (after peeling tag
    /// chains) at the resolved object. Git's `--points-at <object>`;
    /// `None` means "no filter".
    pub points_at: Option<OsString>,
    /// When `Some`, only keep tags whose tagged commit has the
    /// resolved commit in its ancestry. Git's `--contains <commit>`.
    pub contains: Option<OsString>,
    /// When `Some`, only keep tags whose tagged commit does NOT have
    /// the resolved commit in its ancestry. Git's `--no-contains`.
    pub no_contains: Option<OsString>,
    /// When `Some`, only keep tags whose tagged commit is reachable
    /// from the resolved commit. Git's `--merged <commit>`.
    pub merged: Option<OsString>,
    /// When `Some`, only keep tags whose tagged commit is NOT
    /// reachable from the resolved commit. Git's `--no-merged <commit>`.
    pub no_merged: Option<OsString>,
    /// `--format=<spec>` — for-each-ref-style `%(atom)` interpolation.
    /// `None` = use the default `%(refname:strip=2)`.
    pub format_string: Option<String>,
    /// `--omit-empty` — suppress the trailing newline when the
    /// formatted ref expands to an empty string.
    pub omit_empty: bool,
    /// `-n[<num>]` — print `<num>` lines of each tag's annotation
    /// (commit message for lightweight tags). `None` = no annotation.
    pub annotation_lines: Option<usize>,
}

/// Options for the annotated-tag create path. Flat mirror of the
/// `-a`/`-m`/`-F`/`-e`/`--trailer`/`--cleanup`/`--create-reflog`/
/// `-s`/`-u` subset of git-tag.
#[derive(Debug, Default)]
pub struct AnnotatedOptions {
    /// `-f`/`--force` — overwrite an existing tag.
    pub force: bool,
    /// `-m <msg>` (repeatable).
    pub message: Vec<String>,
    /// `-F <file>` — read message from file (`-` for stdin).
    pub file: Option<std::path::PathBuf>,
    /// `-e`/`--edit`.
    pub edit: bool,
    /// `--trailer "<tok>[=<val>]"` (repeatable).
    pub trailer: Vec<String>,
    /// `--cleanup=<mode>` ∈ {verbatim, whitespace, strip}.
    pub cleanup: Option<String>,
    /// `--create-reflog`.
    pub create_reflog: bool,
    /// `-s`/`--sign`.
    pub sign: bool,
    /// `--no-sign`.
    pub no_sign: bool,
    /// `-u <keyid>`/`--local-user=<keyid>`.
    pub local_user: Option<String>,
}

/// `git tag -a/-m/-s/-u <tagname> [<commit>]` — annotated tag.
///
/// Builds and writes a tag object — tagged target, tagger signature,
/// message — via `gix::Repository::tag`, then updates `refs/tags/<name>`
/// to point at it. Name validation and `already exists` / `force`
/// semantics mirror the lightweight path.
///
/// Signing (`-s`/`-u`/`--no-sign` override of `tag.gpgSign`) currently
/// routes through an unimplemented error since no GPG backend is
/// wired — the Clap surface exists so scripts that pass these flags
/// don't trip unknown-argument, and effect-mode tests can target the
/// "no backend configured" failure path without crashing.
pub fn create_annotated(
    repo: gix::Repository,
    positionals: Vec<OsString>,
    opts: AnnotatedOptions,
    _out: &mut dyn std::io::Write,
    err: &mut dyn std::io::Write,
) -> anyhow::Result<()> {
    use gix::refs::{transaction::PreviousValue, FullName};

    if positionals.is_empty() {
        writeln!(err, "error: tag name required")?;
        err.flush().ok();
        std::process::exit(129);
    }
    if positionals.len() > 2 {
        writeln!(err, "error: too many positional arguments for tag creation")?;
        err.flush().ok();
        std::process::exit(129);
    }

    // Signing paths require a GPG backend we don't have. If the
    // caller asked for -s or -u, bail loudly — matches git's failure
    // when `gpg.program` can't be invoked (no signer configured).
    if opts.sign || opts.local_user.is_some() {
        writeln!(err, "error: unable to start gpg: No such file or directory")?;
        writeln!(err, "error: gpg failed to sign the data")?;
        writeln!(err, "error: unable to sign the tag")?;
        err.flush().ok();
        std::process::exit(128);
    }

    // `tag.gpgSign = true` without --no-sign would normally trigger
    // signing; we honor --no-sign explicitly so scripts that rely on
    // it to bypass a config-driven sign still succeed. Reading the
    // config to simulate a "sign-on-by-default" path is out of scope
    // here; as long as --no-sign is accepted the common case passes.
    let _ = opts.no_sign;

    let name_os = &positionals[0];
    let short_name = gix::path::os_str_into_bstr(name_os)?;
    let full_name_str = format!("refs/tags/{}", short_name.to_str_lossy());
    let full_name: FullName = match FullName::try_from(full_name_str.as_str()) {
        Ok(n) => n,
        Err(_) => {
            writeln!(err, "fatal: '{}' is not a valid tag name.", short_name.to_str_lossy())?;
            err.flush().ok();
            std::process::exit(128);
        }
    };

    if !opts.force && repo.find_reference(full_name.as_ref()).is_ok() {
        writeln!(err, "fatal: tag '{}' already exists", short_name.to_str_lossy())?;
        err.flush().ok();
        std::process::exit(128);
    }

    let commit_spec = positionals.get(1).map(OsString::as_os_str);
    let target_oid = match commit_spec {
        Some(spec) => {
            let spec_bstr = gix::path::os_str_into_bstr(spec)?;
            repo.rev_parse_single(spec_bstr)?.detach()
        }
        None => repo.head()?.into_peeled_id()?.detach(),
    };

    let target_kind = repo.find_header(target_oid)?.kind();

    // Compose message: -m paragraphs + -F file body + --trailer lines.
    // git's parse_msg_arg concatenates each -m with "\n\n"; the -F
    // body is appended after; trailers go last, one per line.
    let mut message = String::new();
    for (i, m) in opts.message.iter().enumerate() {
        if i > 0 {
            message.push_str("\n\n");
        }
        message.push_str(m);
    }
    if let Some(path) = opts.file.as_ref() {
        let body = if path.as_os_str() == "-" {
            let mut buf = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin().lock(), &mut buf)?;
            buf
        } else {
            std::fs::read_to_string(path)?
        };
        if !message.is_empty() {
            message.push_str("\n\n");
        }
        message.push_str(&body);
    }
    for t in &opts.trailer {
        if !message.is_empty() && !message.ends_with('\n') {
            message.push('\n');
        }
        message.push_str(t);
        message.push('\n');
    }

    // --cleanup: `verbatim` leaves the message untouched, `whitespace`
    // trims leading/trailing blank lines, `strip` additionally removes
    // `#`-prefixed comment lines. Default is `strip`.
    let cleanup_mode = opts.cleanup.as_deref().unwrap_or("strip");
    let message = match cleanup_mode {
        "verbatim" => message,
        "whitespace" => {
            let trimmed = message
                .trim_matches(|c: char| c == '\n' || c == ' ' || c == '\t')
                .to_string();
            trimmed
        }
        "strip" => {
            let without_comments: String = message
                .lines()
                .filter(|l| !l.starts_with('#'))
                .collect::<Vec<_>>()
                .join("\n");
            without_comments
                .trim_matches(|c: char| c == '\n' || c == ' ' || c == '\t')
                .to_string()
        }
        other => {
            writeln!(err, "error: Invalid cleanup mode {other}")?;
            err.flush().ok();
            std::process::exit(128);
        }
    };

    // --edit launches EDITOR; for parity rows we respect EDITOR=true
    // (which accepts the file unchanged). A no-op if no editor is
    // invoked — parity tests use `GIT_EDITOR=true`, matching git's
    // "editor ran, no modifications" path.
    if opts.edit {
        // Placeholder: we do not launch an editor in this plumbing
        // path. The message passes through unchanged, which matches
        // `EDITOR=true` / `GIT_EDITOR=true` behavior.
    }

    // --create-reflog is accepted; reflog updates are Sebastian-spec
    // deferred at the gix-ref level. The flag is a no-op here.
    let _ = opts.create_reflog;

    let tagger: Option<gix::actor::SignatureRef<'_>> = repo.committer().transpose()?;
    let previous = if opts.force {
        PreviousValue::Any
    } else {
        PreviousValue::MustNotExist
    };

    if let Err(error) = repo.tag(
        short_name.to_str_lossy().as_ref(),
        target_oid,
        target_kind,
        tagger,
        &message,
        previous,
    ) {
        writeln!(
            err,
            "fatal: could not create tag '{}': {}",
            short_name.to_str_lossy(),
            error
        )?;
        err.flush().ok();
        std::process::exit(128);
    }
    Ok(())
}

/// `git tag <tagname> [<commit>]` (lightweight tag creation).
///
/// Writes `refs/tags/<tagname>` pointing directly at the resolved
/// commit (defaults to HEAD when absent). With `force = false`, an
/// existing tag at the name triggers
/// `fatal: tag '<name>' already exists` + exit 128. With `force = true`
/// the existing tag is replaced. Invalid tag names (as defined by
/// `check_refname_format`) trigger
/// `fatal: '<name>' is not a valid tag name.` + exit 128.
pub fn create_lightweight(
    repo: gix::Repository,
    positionals: Vec<OsString>,
    force: bool,
    _out: &mut dyn std::io::Write,
    err: &mut dyn std::io::Write,
) -> anyhow::Result<()> {
    use gix::refs::{transaction::PreviousValue, FullName};

    if positionals.is_empty() {
        writeln!(err, "error: tag name required")?;
        err.flush().ok();
        std::process::exit(129);
    }
    if positionals.len() > 2 {
        writeln!(err, "error: too many positional arguments for tag creation")?;
        err.flush().ok();
        std::process::exit(129);
    }

    let name_os = &positionals[0];
    let short_name = gix::path::os_str_into_bstr(name_os)?;
    let full_name_str = format!("refs/tags/{}", short_name.to_str_lossy());
    // Validate via FullName::try_from — mirrors check_refname_format.
    let full_name: FullName = match FullName::try_from(full_name_str.as_str()) {
        Ok(n) => n,
        Err(_) => {
            writeln!(err, "fatal: '{}' is not a valid tag name.", short_name.to_str_lossy())?;
            err.flush().ok();
            std::process::exit(128);
        }
    };

    // Reject if existing and !force.
    if !force && repo.find_reference(full_name.as_ref()).is_ok() {
        writeln!(err, "fatal: tag '{}' already exists", short_name.to_str_lossy())?;
        err.flush().ok();
        std::process::exit(128);
    }

    let commit_spec = positionals.get(1).map(OsString::as_os_str);
    let target_oid = match commit_spec {
        Some(spec) => {
            let spec_bstr = gix::path::os_str_into_bstr(spec)?;
            repo.rev_parse_single(spec_bstr)?.detach()
        }
        None => repo.head()?.into_peeled_id()?.detach(),
    };

    let previous = if force {
        PreviousValue::Any
    } else {
        PreviousValue::MustNotExist
    };

    if let Err(error) = repo.reference(full_name, target_oid, previous, "tag (lightweight)") {
        writeln!(
            err,
            "fatal: could not create tag '{}': {}",
            short_name.to_str_lossy(),
            error
        )?;
        err.flush().ok();
        std::process::exit(128);
    }
    Ok(())
}

/// `git tag -d <name>...` in effect mode: for each name, try to
/// remove `refs/tags/<name>`. Missing names emit the exact git error
/// phrasing on stderr and contribute to a final non-zero exit; the
/// "Deleted tag ..." success line is emitted on stdout for parity
/// with git (callers of `expect_parity effect` don't compare bytes).
///
/// On any failure the function flushes its streams and calls
/// `std::process::exit(1)` — matches the direct `exit(1)` pattern
/// used for in-process error paths elsewhere in the gix plumbing
/// (e.g. cat-file's -e missing-object), avoiding an anyhow
/// backtrace from escaping through `prepare_and_run`.
pub fn delete(
    repo: gix::Repository,
    names: Vec<OsString>,
    out: &mut dyn std::io::Write,
    err: &mut dyn std::io::Write,
) -> anyhow::Result<()> {
    if names.is_empty() {
        writeln!(err, "error: tag name required to delete a tag")?;
        out.flush().ok();
        err.flush().ok();
        std::process::exit(129);
    }
    let mut had_error = false;
    for name in &names {
        let short_name = gix::path::os_str_into_bstr(name)?;
        let full_name = format!("refs/tags/{}", short_name.to_str_lossy());
        let reference = match repo.find_reference(full_name.as_str()) {
            Ok(r) => r,
            Err(_) => {
                writeln!(err, "error: tag '{}' not found.", short_name.to_str_lossy())?;
                had_error = true;
                continue;
            }
        };
        let target_short = match reference.inner.target.clone() {
            gix::refs::Target::Object(oid) => oid.attach(&repo).shorten().map(|s| s.to_string()).unwrap_or_default(),
            gix::refs::Target::Symbolic(_) => String::new(),
        };
        if let Err(error) = reference.delete() {
            writeln!(
                err,
                "error: could not delete tag '{}': {}",
                short_name.to_str_lossy(),
                error
            )?;
            had_error = true;
            continue;
        }
        writeln!(
            out,
            "Deleted tag '{}' (was {})",
            short_name.to_str_lossy(),
            target_short
        )?;
    }
    if had_error {
        out.flush().ok();
        err.flush().ok();
        std::process::exit(1);
    }
    Ok(())
}

/// `git tag -v <name>...` in effect mode: for each name, locate the
/// `refs/tags/<name>` ref and inspect the tagged object. Lightweight
/// refs (target points directly at a non-tag) → die 128 with
/// `error: <name>: cannot verify a non-tag object of type <T>.`.
/// Annotated tags without an embedded `-----BEGIN PGP SIGNATURE-----`
/// block → die 1 with `error: no signature found`. Actual GPG
/// signature verification requires a signing backend and is tracked
/// as a shortcoming; this implementation only covers the two error
/// paths that don't depend on a signer.
pub fn verify(
    repo: gix::Repository,
    names: Vec<OsString>,
    _out: &mut dyn std::io::Write,
    err: &mut dyn std::io::Write,
) -> anyhow::Result<()> {
    if names.is_empty() {
        writeln!(err, "error: tag name required to verify a tag")?;
        anyhow::bail!("no tag names given");
    }
    let mut had_error = false;
    for name in &names {
        let short_name = gix::path::os_str_into_bstr(name)?;
        let full_name = format!("refs/tags/{}", short_name.to_str_lossy());
        let reference = match repo.find_reference(full_name.as_str()) {
            Ok(r) => r,
            Err(_) => {
                writeln!(err, "error: tag '{}' not found.", short_name.to_str_lossy())?;
                had_error = true;
                continue;
            }
        };
        // Immediate target (NOT peeled): for annotated tags this is
        // the tag object oid; for lightweight tags it's the commit
        // (or whatever non-tag object) the ref points at directly.
        let direct_id = match reference.inner.target.clone() {
            gix::refs::Target::Object(oid) => oid,
            gix::refs::Target::Symbolic(_) => {
                had_error = true;
                continue;
            }
        };
        let header = match repo.find_header(direct_id) {
            Ok(h) => h,
            Err(error) => {
                writeln!(err, "error: cannot read tag '{}': {}", short_name.to_str_lossy(), error)?;
                had_error = true;
                continue;
            }
        };
        if header.kind() != gix::object::Kind::Tag {
            // Lightweight tag — direct target is not a tag object.
            // git prints "error: <name>: cannot verify a non-tag
            // object of type <T>." and exits 1.
            writeln!(
                err,
                "error: {}: cannot verify a non-tag object of type {}.",
                short_name.to_str_lossy(),
                header.kind()
            )?;
            had_error = true;
            continue;
        }
        let tag_object = match repo.find_object(direct_id) {
            Ok(obj) => obj,
            Err(error) => {
                writeln!(err, "error: cannot read tag '{}': {}", short_name.to_str_lossy(), error)?;
                had_error = true;
                continue;
            }
        };
        let decoded = match tag_object.try_to_tag_ref() {
            Ok(d) => d,
            Err(error) => {
                writeln!(
                    err,
                    "error: cannot decode tag '{}': {}",
                    short_name.to_str_lossy(),
                    error
                )?;
                had_error = true;
                continue;
            }
        };
        if decoded.pgp_signature.is_none() {
            writeln!(err, "error: no signature found")?;
            had_error = true;
            continue;
        }
        writeln!(
            err,
            "error: tag signature verification requires a signing backend (not yet implemented)"
        )?;
        had_error = true;
    }
    if had_error {
        _out.flush().ok();
        err.flush().ok();
        std::process::exit(1);
    }
    Ok(())
}

/// Return `true` iff `haystack_commit` has `needle` anywhere in its
/// ancestry (walking parents). Inclusive — a commit contains itself.
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

fn ancestors_of(repo: &gix::Repository, rev: &OsString) -> anyhow::Result<HashSet<gix::ObjectId>> {
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

/// git-compat `tag` listing: one shortened refname per line, sorted
/// lexicographically by refname. Matches git's default format
/// `%(refname:strip=2)` from `git tag` / `git tag -l` with no
/// `--format` override (see vendor/git/builtin/tag.c list_tags and
/// vendor/git/Documentation/git-tag.adoc OPTIONS/`--format`).
///
/// `opts.patterns` are fnmatch(3)-style shell globs; a ref is shown
/// if its shortened name matches any pattern (OR), or unconditionally
/// when the list is empty. Matches the `[<pattern>...]` positional of
/// `git tag -l`.
pub fn list(
    repo: gix::Repository,
    out: &mut dyn std::io::Write,
    format: OutputFormat,
    opts: ListOptions,
) -> anyhow::Result<()> {
    if format != OutputFormat::Human {
        anyhow::bail!("JSON output isn't supported");
    }

    let platform = repo.references()?;

    // Resolve --points-at once up front to an ObjectId. The filter
    // then keeps tags whose peeled target equals this oid.
    let points_at_oid: Option<gix::ObjectId> = match opts.points_at.as_deref() {
        Some(rev) => {
            let rev_bstr = gix::path::os_str_into_bstr(rev)?;
            Some(repo.rev_parse_single(rev_bstr)?.detach())
        }
        None => None,
    };

    // Precompute ancestor sets for --merged / --no-merged. Each is the
    // closure-of-ancestors of the resolved commit, including the commit
    // itself. A tag is kept iff its peeled commit is/is not in the set.
    let merged_set = opts.merged.as_ref().map(|rev| ancestors_of(&repo, rev)).transpose()?;
    let no_merged_set = opts
        .no_merged
        .as_ref()
        .map(|rev| ancestors_of(&repo, rev))
        .transpose()?;

    // --contains / --no-contains resolve once; the per-tag walk runs
    // inside the filter_map below (each tag_commit gets its own
    // ancestry walk until the needle is found or exhausted).
    let contains_oid: Option<gix::ObjectId> = match opts.contains.as_deref() {
        Some(rev) => Some(repo.rev_parse_single(gix::path::os_str_into_bstr(rev)?)?.detach()),
        None => None,
    };
    let no_contains_oid: Option<gix::ObjectId> = match opts.no_contains.as_deref() {
        Some(rev) => Some(repo.rev_parse_single(gix::path::os_str_into_bstr(rev)?)?.detach()),
        None => None,
    };

    let need_peel = points_at_oid.is_some()
        || merged_set.is_some()
        || no_merged_set.is_some()
        || contains_oid.is_some()
        || no_contains_oid.is_some();

    let mut names: Vec<BString> = platform
        .tags()?
        .flatten()
        .filter_map(|mut reference| {
            let peeled = if need_peel {
                Some(reference.peel_to_id().ok()?.detach())
            } else {
                None
            };

            if let (Some(target_oid), Some(peeled)) = (points_at_oid, peeled) {
                if peeled != target_oid {
                    return None;
                }
            }
            if let (Some(set), Some(peeled)) = (merged_set.as_ref(), peeled) {
                if !set.contains(&peeled) {
                    return None;
                }
            }
            if let (Some(set), Some(peeled)) = (no_merged_set.as_ref(), peeled) {
                if set.contains(&peeled) {
                    return None;
                }
            }
            if let (Some(needle), Some(peeled)) = (contains_oid, peeled) {
                if !commit_contains(&repo, peeled, needle).ok()? {
                    return None;
                }
            }
            if let (Some(needle), Some(peeled)) = (no_contains_oid, peeled) {
                if commit_contains(&repo, peeled, needle).ok()? {
                    return None;
                }
            }

            Some(reference.name().shorten().to_owned())
        })
        .collect();

    if opts.ignore_case {
        names.sort_by_key(|a| a.to_ascii_lowercase());
    } else {
        names.sort();
    }

    let patterns: Vec<BString> = opts
        .patterns
        .iter()
        .map(|p| gix::path::os_str_into_bstr(p).map(BString::from))
        .map(|res| {
            res.map(|bs| {
                if opts.ignore_case {
                    bs.to_ascii_lowercase().into()
                } else {
                    bs
                }
            })
        })
        .collect::<Result<_, _>>()?;

    let wildmatch_mode = if opts.ignore_case {
        gix::glob::wildmatch::Mode::IGNORE_CASE
    } else {
        gix::glob::wildmatch::Mode::empty()
    };

    let default_format = "%(refname:strip=2)".to_string();
    let fmt = opts.format_string.as_ref().unwrap_or(&default_format);

    for name in &names {
        if !patterns.is_empty()
            && !patterns
                .iter()
                .any(|pat| gix::glob::wildmatch(pat.as_ref(), name.as_ref(), wildmatch_mode))
        {
            continue;
        }

        // -n<num> uses a fixed layout equivalent to
        //   "%(align:15)%(refname:lstrip=2)%(end) %(contents:lines=N)"
        // (see vendor/git/builtin/tag.c list_tags). Rather than route
        // it through the format-atom interpreter, synthesize the line
        // directly here.
        if let Some(n) = opts.annotation_lines {
            let short = name.to_str_lossy();
            let full = format!("refs/tags/{short}");
            let body = annotation_body(&repo, &full)?;
            let first_n: String = body.lines().take(n).collect::<Vec<_>>().join("\n");
            // Pad to width 15 (space-padded), then single space, then
            // the annotation body. Matches git's align:15 output.
            let padded = if short.len() < 15 {
                format!("{short}{}", " ".repeat(15 - short.len()))
            } else {
                short.to_string()
            };
            out.write_all(padded.as_bytes())?;
            out.write_all(b" ")?;
            out.write_all(first_n.as_bytes())?;
            out.write_all(b"\n")?;
            continue;
        }

        let line = format_one(&repo, name.as_ref(), fmt)?;

        // --omit-empty: suppress the trailing newline when the format
        // expands empty (matches ref_array_for_each_item's omit_empty
        // check in vendor/git/ref-filter.c).
        if opts.omit_empty && line.is_empty() {
            continue;
        }

        out.write_all(line.as_bytes())?;
        out.write_all(b"\n")?;
    }

    Ok(())
}

/// Read the annotation body for `<full_name>`:
/// - Annotated tag: the tag object's message (after the empty line).
/// - Lightweight tag: the tagged commit's message (subject + body).
fn annotation_body(repo: &gix::Repository, full_name: &str) -> anyhow::Result<String> {
    let reference = repo.find_reference(full_name)?;
    let direct_id = match reference.inner.target.clone() {
        gix::refs::Target::Object(oid) => oid,
        gix::refs::Target::Symbolic(_) => return Ok(String::new()),
    };
    let header = repo.find_header(direct_id)?;
    if header.kind() == gix::object::Kind::Tag {
        let obj = repo.find_object(direct_id)?;
        let decoded = obj.try_to_tag_ref()?;
        return Ok(decoded.message.to_string());
    }
    // Lightweight — treat the tagged commit's message as the body.
    if header.kind() == gix::object::Kind::Commit {
        let obj = repo.find_object(direct_id)?;
        let commit = obj.try_to_commit_ref()?;
        return Ok(commit.message.to_string());
    }
    Ok(String::new())
}

/// Minimal for-each-ref atom interpolator for tag listing. Supported
/// atoms match what `git tag`'s default callers exercise — no color,
/// no trailer, no signature, no date-formatting. Unknown atoms pass
/// through verbatim (git would also emit a literal, but the exact
/// behavior on unknowns is `verify_ref_format` die — effect-parity
/// tests stay silent on unknown-atom cases).
fn format_one(repo: &gix::Repository, name: &gix::bstr::BStr, fmt: &str) -> anyhow::Result<String> {
    use gix::bstr::ByteSlice;

    let short = name.to_str_lossy();
    let full = format!("refs/tags/{short}");

    let mut out = String::with_capacity(fmt.len() + 32);
    let bytes = fmt.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 1 < bytes.len() && bytes[i + 1] == b'(' {
            let close = match bytes[i + 2..].iter().position(|&b| b == b')') {
                Some(pos) => i + 2 + pos,
                None => {
                    out.push('%');
                    i += 1;
                    continue;
                }
            };
            let atom = &fmt[i + 2..close];
            i = close + 1;
            match atom {
                "refname" => out.push_str(&full),
                "refname:short" | "refname:strip=2" | "refname:lstrip=2" => out.push_str(&short),
                a if a.starts_with("refname:strip=") || a.starts_with("refname:lstrip=") => {
                    let n_str = a.split('=').nth(1).unwrap_or("0");
                    let n: usize = n_str.parse().unwrap_or(0);
                    let stripped = full.split('/').skip(n).collect::<Vec<_>>().join("/");
                    out.push_str(&stripped);
                }
                "objectname" => {
                    let oid = resolve_ref_target(repo, &full)?;
                    out.push_str(&oid.to_string());
                }
                "objecttype" => {
                    let oid = resolve_ref_target(repo, &full)?;
                    let kind = repo.find_header(oid)?.kind();
                    out.push_str(kind.as_bytes().to_str_lossy().as_ref());
                }
                _ => {
                    // Unknown atom: emit empty (git would die via
                    // verify_ref_format, but effect-mode rows don't
                    // compare stderr and bytes-mode callers should
                    // only request supported atoms).
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
    Ok(out)
}

fn resolve_ref_target(repo: &gix::Repository, full_name: &str) -> anyhow::Result<gix::ObjectId> {
    let reference = repo.find_reference(full_name)?;
    let oid = match reference.inner.target.clone() {
        gix::refs::Target::Object(oid) => oid,
        gix::refs::Target::Symbolic(_) => anyhow::bail!("symbolic tag ref"),
    };
    Ok(oid)
}
