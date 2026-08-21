//! Discover and execute git hooks the way `git` itself does.
//!
//! This is an early prototype covering only the client-side basics: locating a hook script
//! below `$GIT_DIR/hooks` (or a configured `core.hooksPath`), reading that override from
//! `gix-config`, and preparing the hook for execution with [`gix_command`].
//!
//! Receive-side hooks get env-var helpers ([`push_option_env()`], [`quarantine_env()`],
//! [`push_cert_env()`]), stdin formatting ([`receive_stdin()`]) for `pre-receive`/`post-receive`,
//! and argument formatting ([`update_args()`]) for `update`.
//!
//! Windows hook resolution matches git's own `find_hook()` (`hook.c`): try the exact name
//! first, and only fall back to appending `.exe` if that's missing (the reverse order of
//! `gix-command`'s general-purpose `PATH` lookup, which prefers `.exe` first - a different
//! algorithm for a different problem, not reused here). `.bat`/`.cmd` aren't tried, matching
//! git upstream: its own fallback is hardcoded to `STRIP_EXTENSION`, defined as `".exe"` for
//! both the MinGW and MSVC Windows builds.
//!
//! `reference-transaction` gets its own state handling ([`TransactionState`],
//! [`reference_transaction_args()`], [`reference_transaction_stdin()`]) rather than reusing the
//! receive-side helpers, since its stdin values may be `ref:`-prefixed symbolic targets instead
//! of always being object ids.
//!
//! [`no_editor_env()`] sets `GIT_EDITOR=:` for `pre-commit`/`pre-merge-commit`, when the
//! caller's own commit flow won't show an editor.
//!
//! [`HookArgs`] covers every other named hook's own positional-argument contract in one place
//! (`commit-msg`, `prepare-commit-msg`, `pre-rebase`, `post-checkout`, `post-merge`, `pre-push`,
//! `push-to-checkout`, `post-update`, `post-rewrite`, `sendemail-validate`,
//! `post-index-change`, `applypatch-msg`) - everything except `update` and
//! `reference-transaction`, which stay on their own dedicated, fallible functions.
#![deny(missing_docs, rust_2018_idioms)]
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

mod error;
pub use error::Error;

/// The default directory name, relative to `$GIT_DIR`, that holds hook scripts.
pub const HOOKS_DIR: &str = "hooks";

/// Find the hook named `name`, returning its path if it exists and is executable.
///
/// `git_dir` is the repository's `.git` directory. `hooks_path`, if set, overrides the
/// default `$GIT_DIR/hooks` location, mirroring `core.hooksPath`.
///
/// Like `git`, a hook that exists but isn't executable is treated as absent - on Unix that
/// means the executable permission bit; on Windows, where that bit doesn't exist, it means
/// resolving `name` the way git's own `find_hook()` does (see `windows_find()` below).
pub fn find(name: &str, git_dir: &Path, hooks_path: Option<&Path>) -> Option<PathBuf> {
    let base = hooks_path.map_or_else(|| git_dir.join(HOOKS_DIR), Path::to_owned);
    if cfg!(windows) {
        windows_find(&base, name)
    } else {
        let candidate = base.join(name);
        is_executable_file(&candidate).then_some(candidate)
    }
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

/// Whether `s` contains a byte in the same control-byte range `gix_validate::reference::name()`
/// rejects (`\0..=\x1F` or `\x7F`), for fields this crate can't run full ref-name validation on
/// but still shouldn't hand a hook process control bytes in.
fn contains_control_byte(s: &str) -> bool {
    s.bytes().any(|b| matches!(b, 0x00..=0x1F | 0x7F))
}

/// Resolve a hook named `name` in directory `base` the way git's own `find_hook()` (`hook.c`)
/// does on Windows: try `base/<name>` first, and only if that's missing, retry with `.exe`
/// appended - the reverse order of `gix-command`'s general-purpose `PATH` lookup, which favors
/// `.exe` first. `STRIP_EXTENSION`, git's name for the appended suffix, is defined as `".exe"`
/// for both the MinGW and MSVC Windows builds; `.bat`/`.cmd` aren't tried, matching upstream.
///
/// Verified against git's own source at tag `v2.47.0` (`hook.c`, `config.mak.uname`) - the same
/// tag `tests/fixtures/git-hook-names.txt` is pinned to. `find_hook()`'s order has been stable
/// for a long time, but if a much newer git changes it, that's the tag to re-check against.
///
/// Not gated to `#[cfg(windows)]` so it stays compilable and testable on every platform;
/// [`find()`] only calls it behind a `cfg!(windows)` runtime check.
fn windows_find(base: &Path, name: &str) -> Option<PathBuf> {
    let candidate = base.join(name);
    if candidate.is_file() {
        return Some(candidate);
    }
    let mut with_exe = candidate.into_os_string();
    with_exe.push(".exe");
    let with_exe = PathBuf::from(with_exe);
    with_exe.is_file().then_some(with_exe)
}

/// Prepare `hook` for execution the way `git` does for client-side hooks: run it directly,
/// never through a shell, in `cwd` (typically the worktree root, or `git_dir` itself for bare
/// repositories), with `GIT_DIR` set via [`gix_command::Context`].
///
/// The caller is responsible for adding hook-specific arguments and further environment
/// variables (for example `GIT_INDEX_FILE`) before calling
/// [`spawn()`](gix_command::Prepare::spawn()).
pub fn command(hook: &Path, git_dir: &Path, cwd: &Path) -> gix_command::Prepare {
    gix_command::prepare(hook)
        .without_shell()
        .current_dir(cwd)
        .with_context(gix_command::Context {
            git_dir: Some(git_dir.to_owned()),
            ..Default::default()
        })
}

/// Resolve `core.hooksPath` from `config`, expanding `~` and `%(prefix)` using `ctx`.
///
/// Returns `Ok(None)` if the key isn't set. A relative path is returned as-is, matching `git`:
/// it's resolved against the current working directory at hook-invocation time, not `git_dir`.
pub fn hooks_path_from_config(
    config: &gix_config::File,
    ctx: gix_config::path::interpolate::Context<'_>,
) -> Result<Option<PathBuf>, Error> {
    Ok(config
        .path("core.hooksPath")
        .map(|path| path.interpolate(ctx))
        .transpose()?)
}

/// The result of a hook process running to completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The hook exited with status `0`.
    Success,
    /// The hook exited with a non-zero status, or was terminated by a signal.
    Rejected {
        /// The process's exit code, or `None` if it was terminated by a signal (Unix only).
        code: Option<i32>,
    },
}

impl From<std::process::ExitStatus> for Outcome {
    fn from(status: std::process::ExitStatus) -> Self {
        if status.success() {
            Outcome::Success
        } else {
            Outcome::Rejected { code: status.code() }
        }
    }
}

/// Spawn `prepared` and wait for it to exit, classifying the result as an [`Outcome`].
pub fn run(prepared: gix_command::Prepare) -> Result<Outcome, Error> {
    let status = prepared.spawn()?.wait()?;
    Ok(status.into())
}

/// Hook names whose exit status git ignores, per [`githooks(5)`](https://git-scm.com/docs/githooks).
/// Every other hook name, including ones not listed here at all, is treated as gating by
/// [`is_gating()`].
///
/// `reference-transaction` is deliberately absent: it only gates during its `prepared` state,
/// while `committed` and `aborted` calls are always advisory - a single name can't capture
/// that. Use [`TransactionState::is_gating()`] for that hook instead.
///
/// `proc-receive` is also absent, but for a different reason: it gates unconditionally, just
/// not the whole push. Per `githooks(5)`, "the exit status of the `proc-receive` hook only
/// determines the success or failure of the group of commands sent to it, unless atomic push is
/// in use" - so `is_gating("proc-receive")` is `true`, but the caller must additionally know
/// whether the push is atomic to judge what a non-zero exit takes down with it.
pub const ADVISORY_HOOKS: &[&str] = &[
    "post-applypatch",
    "post-checkout",
    "post-commit",
    "post-merge",
    "post-receive",
    "post-rewrite",
    "post-update",
];

/// Whether a non-zero exit from the hook named `name` aborts the operation it guards, per
/// [`ADVISORY_HOOKS`].
///
/// Returns `true` for `name`s not listed there, since git only ever invokes hooks it knows
/// about, and an unrecognized name is safest treated as gating.
pub fn is_gating(name: &str) -> bool {
    !ADVISORY_HOOKS.contains(&name)
}

/// Set `GIT_EDITOR=:` for the `pre-commit`/`pre-merge-commit` hooks, telling them no editor
/// will be launched to modify the commit message for the pending commit.
///
/// Per `githooks(5)`: "All the git commit hooks are invoked with the environment variable
/// `GIT_EDITOR=:` if the command will not bring up an editor to modify the commit message." So
/// call this only when the caller's own commit flow won't show one (for example, `git commit
/// -m` never does) - the same "caller decides whether it applies" shape as
/// [`push_option_env()`].
///
/// Git itself hardcodes `":"` unconditionally, with no platform-specific branching (see
/// `commit.c`'s `run_commit_hook()`: `if (!editor_is_used) strvec_push(&opt.env,
/// "GIT_EDITOR=:");`) - even on Windows, where `:` isn't a native `cmd.exe` no-op, because Git
/// for Windows' own hook execution runs through its bundled MSYS2 `sh`, where it behaves the
/// same as on Unix. This function matches that: the same value on every platform.
pub fn no_editor_env(prepared: gix_command::Prepare) -> gix_command::Prepare {
    prepared.env("GIT_EDITOR", ":")
}

/// Add the push options git passes to `pre-receive` and `post-receive` when invoked via
/// `git push --push-option=<option>`: `GIT_PUSH_OPTION_COUNT` and `GIT_PUSH_OPTION_<n>` for each
/// option, in the order given.
///
/// Per `githooks(5)`: "If it is negotiated to not use the push options phase, the environment
/// variables will not be set" - so omit this call entirely rather than passing an empty slice
/// when push options were never negotiated for the push.
///
/// Option values are passed through unvalidated, unlike [`receive_stdin()`]'s ref names: git
/// itself treats push options as free-form text with no format to check against, and each one
/// becomes a single, discrete environment variable via [`Prepare::env()`](gix_command::Prepare::env())
/// rather than a field in a wire format this crate constructs - so there's no delimiter for a
/// crafted value to inject through. The one real OS-level hazard, an embedded NUL byte, is
/// already handled: `std::process::Command` reports that as an `io::Error` at `spawn()` rather
/// than silently corrupting or truncating anything.
pub fn push_option_env(prepared: gix_command::Prepare, options: &[impl AsRef<str>]) -> gix_command::Prepare {
    let mut prepared = prepared.env("GIT_PUSH_OPTION_COUNT", options.len().to_string());
    for (index, option) in options.iter().enumerate() {
        prepared = prepared.env(format!("GIT_PUSH_OPTION_{index}"), option.as_ref());
    }
    prepared
}

/// Add the quarantine environment git sets for `pre-receive` and `update`: new objects are
/// written to `quarantine_object_dir` rather than the real object store, so a rejecting hook
/// leaves no trace. `real_object_dir` is added as an alternate so the hook can still read
/// pre-existing objects.
///
/// Mirrors `tmp_objdir_create()`/`tmp_objdir_env()` in git's own `tmp-objdir.c`, which sets
/// `GIT_QUARANTINE_PATH` and `GIT_OBJECT_DIRECTORY` to the quarantine directory and appends
/// the real one to `GIT_ALTERNATE_OBJECT_DIRECTORIES`. Both paths are used as given - the
/// caller is responsible for making them absolute, matching every other path this crate takes.
pub fn quarantine_env(
    prepared: gix_command::Prepare,
    quarantine_object_dir: &Path,
    real_object_dir: &Path,
) -> gix_command::Prepare {
    prepared
        .env("GIT_QUARANTINE_PATH", quarantine_object_dir)
        .env("GIT_OBJECT_DIRECTORY", quarantine_object_dir)
        .env("GIT_ALTERNATE_OBJECT_DIRECTORIES", real_object_dir)
}

/// The fields of a signed push's certificate, as git exposes them via `GIT_PUSH_CERT*`
/// environment variables to hooks handling the push.
#[derive(Debug, Clone, Copy)]
pub struct PushCert<'a> {
    /// The object name of the blob holding the push certificate (`GIT_PUSH_CERT`).
    pub blob: &'a gix_hash::oid,
    /// The signer's name and email as recorded in the certificate (`GIT_PUSH_CERT_SIGNER`).
    pub signer: &'a str,
    /// The GPG key id used to sign the certificate (`GIT_PUSH_CERT_KEY`).
    pub key: &'a str,
    /// The GPG verification result for the signature (`GIT_PUSH_CERT_STATUS`).
    pub status: &'a str,
    /// The nonce string included in the certificate (`GIT_PUSH_CERT_NONCE`).
    pub nonce: &'a str,
    /// Whether the received nonce matched what was expected (`GIT_PUSH_CERT_NONCE_STATUS`).
    pub nonce_status: &'a str,
    /// The time difference in seconds used for nonce replay detection
    /// (`GIT_PUSH_CERT_NONCE_SLOP`), set only when relevant.
    pub nonce_slop: Option<&'a str>,
}

/// Add the `GIT_PUSH_CERT*` environment variables git sets for `pre-receive`, `update`, and
/// `post-receive` when the push was signed (`git push --signed`).
pub fn push_cert_env(prepared: gix_command::Prepare, cert: &PushCert<'_>) -> gix_command::Prepare {
    let prepared = prepared
        .env("GIT_PUSH_CERT", cert.blob.to_hex().to_string())
        .env("GIT_PUSH_CERT_SIGNER", cert.signer)
        .env("GIT_PUSH_CERT_KEY", cert.key)
        .env("GIT_PUSH_CERT_STATUS", cert.status)
        .env("GIT_PUSH_CERT_NONCE", cert.nonce)
        .env("GIT_PUSH_CERT_NONCE_STATUS", cert.nonce_status);
    match cert.nonce_slop {
        Some(slop) => prepared.env("GIT_PUSH_CERT_NONCE_SLOP", slop),
        None => prepared,
    }
}

/// Add the three positional arguments git passes to the `update` hook, per `githooks(5)`:
/// the ref being updated, then its old object name, then its new object name.
///
/// Rejects `ref_name` if it fails [`gix_validate::reference::name()`], rather than handing a
/// hook process a ref name that couldn't have come from git itself.
pub fn update_args(
    prepared: gix_command::Prepare,
    ref_name: &str,
    old_oid: &gix_hash::oid,
    new_oid: &gix_hash::oid,
) -> Result<gix_command::Prepare, Error> {
    gix_validate::reference::name(ref_name.into())?;
    Ok(prepared
        .arg(ref_name)
        .arg(old_oid.to_hex().to_string())
        .arg(new_oid.to_hex().to_string()))
}

/// Format the `pre-receive`/`post-receive` stdin payload: one line per ref update, as
/// `<old-oid> SP <new-oid> SP <ref-name> LF`, per `githooks(5)`.
///
/// `post-receive` receives a line only for refs that were actually updated, unlike
/// `pre-receive`, which receives one for every ref requested - filtering `updates` down to
/// the successful ones for `post-receive` is the caller's responsibility.
///
/// Rejects any ref name that fails [`gix_validate::reference::name()`]: since this function
/// owns the line-oriented wire format it writes, a ref name that isn't actually valid (for
/// example, one containing a newline) could otherwise inject a bogus extra line.
pub fn receive_stdin<'a>(
    updates: impl IntoIterator<Item = (&'a gix_hash::oid, &'a gix_hash::oid, &'a str)>,
) -> Result<Vec<u8>, Error> {
    use std::io::Write;

    let mut buf = Vec::new();
    for (old_oid, new_oid, ref_name) in updates {
        gix_validate::reference::name(ref_name.into())?;
        writeln!(buf, "{} {} {}", old_oid.to_hex(), new_oid.to_hex(), ref_name)
            .expect("writing to a Vec<u8> never fails");
    }
    Ok(buf)
}

/// The three states git calls the `reference-transaction` hook with, per `githooks(5)`.
///
/// Verified against git's own source at tag `v2.47.0`, the same tag `windows_find()`'s docs
/// and `tests/fixtures/git-hook-names.txt` are pinned to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionState {
    /// All updates are queued to the transaction and references are locked on disk.
    ///
    /// A non-zero exit here aborts the transaction, and git does *not* then call the hook
    /// again with [`Aborted`](Self::Aborted) - that state is only used for transactions that
    /// were aborted for some other reason.
    Prepared,
    /// The transaction was committed; every reference now has its new value.
    Committed,
    /// The transaction was aborted; no changes were made and the locks were released.
    Aborted,
}

impl TransactionState {
    /// The literal string git passes as the hook's single positional argument for this state.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Prepared => "prepared",
            Self::Committed => "committed",
            Self::Aborted => "aborted",
        }
    }

    /// Whether a non-zero exit in this state aborts the transaction - true only for
    /// [`Prepared`](Self::Prepared); git ignores the exit status in the other two states.
    pub fn is_gating(self) -> bool {
        matches!(self, Self::Prepared)
    }
}

/// Add the single positional state argument git passes to the `reference-transaction` hook.
pub fn reference_transaction_args(prepared: gix_command::Prepare, state: TransactionState) -> gix_command::Prepare {
    prepared.arg(state.as_str())
}

/// One side (old or new) of a `reference-transaction` update: either an object id, or a
/// symbolic reference's target, per `githooks(5)`: "the value fields may use a `ref:` prefix
/// instead of object names" for symbolic reference updates.
#[derive(Debug, Clone, Copy)]
pub enum TransactionValue<'a> {
    /// An object id.
    Oid(&'a gix_hash::oid),
    /// A symbolic reference's target name, without the `ref:` prefix - added when formatting.
    SymbolicTarget(&'a str),
}

/// Format the `reference-transaction` stdin payload: one line per update, as
/// `<old-value> SP <new-value> SP <ref-name> LF`, per `githooks(5)` - the same shape as
/// [`receive_stdin()`], except each value may be an object id or a `ref:`-prefixed symbolic
/// target instead of always being an object id.
///
/// Rejects any ref name that fails [`gix_validate::reference::name()`], for the same reason
/// [`receive_stdin()`] does. A [`TransactionValue::SymbolicTarget`] is itself a reference name,
/// so it's validated the same way as `ref_name` - not just the object-id case.
pub fn reference_transaction_stdin<'a>(
    updates: impl IntoIterator<Item = (TransactionValue<'a>, TransactionValue<'a>, &'a str)>,
) -> Result<Vec<u8>, Error> {
    use std::io::Write;

    fn validate_and_write(
        buf: &mut Vec<u8>,
        value: TransactionValue<'_>,
    ) -> Result<(), gix_validate::reference::name::Error> {
        match value {
            TransactionValue::Oid(oid) => write!(buf, "{}", oid.to_hex()).expect("writing to a Vec<u8> never fails"),
            TransactionValue::SymbolicTarget(target) => {
                gix_validate::reference::name(target.into())?;
                write!(buf, "ref:{target}").expect("writing to a Vec<u8> never fails");
            }
        }
        Ok(())
    }

    let mut buf = Vec::new();
    for (old, new, ref_name) in updates {
        gix_validate::reference::name(ref_name.into())?;
        validate_and_write(&mut buf, old)?;
        buf.push(b' ');
        validate_and_write(&mut buf, new)?;
        writeln!(buf, " {ref_name}").expect("writing to a Vec<u8> never fails");
    }
    Ok(buf)
}

/// Format the `pre-push` stdin payload: one line per ref being considered for push, as
/// `<local-ref> SP <local-oid> SP <remote-ref> SP <remote-oid> LF`, per `githooks(5)`.
///
/// Unlike `remote_ref`, `local_ref` is *not* validated as a ref name: git documents that it may
/// be the literal `"(delete)"` when a ref is being deleted, or "supplied as it was originally
/// given" when the push source wasn't an expandable ref name (for example `HEAD~`, or a bare
/// object id) - both legitimately violate `check-ref-format`. It's still rejected if it contains
/// a control byte (`\0..=\x1F` or `\x7F`, the same range [`gix_validate::reference::name()`]
/// itself rejects) - a raw newline would corrupt this function's own line framing regardless of
/// whether the value is a "valid ref name", and a raw NUL has no legitimate reason to appear
/// either. Non-ASCII Unicode, including combining marks, is left alone: git's own ref-name rules
/// allow it, so restricting it here would just be incompatible with real git, not safer.
pub fn pre_push_stdin<'a>(
    updates: impl IntoIterator<Item = (&'a str, &'a gix_hash::oid, &'a str, &'a gix_hash::oid)>,
) -> Result<Vec<u8>, Error> {
    use std::io::Write;

    let mut buf = Vec::new();
    for (local_ref, local_oid, remote_ref, remote_oid) in updates {
        if contains_control_byte(local_ref) {
            return Err(Error::ControlByteInField(local_ref.to_owned()));
        }
        gix_validate::reference::name(remote_ref.into())?;
        writeln!(
            buf,
            "{local_ref} {} {remote_ref} {}",
            local_oid.to_hex(),
            remote_oid.to_hex()
        )
        .expect("writing to a Vec<u8> never fails");
    }
    Ok(buf)
}

/// Hook names git always executes in `$GIT_DIR`, regardless of whether the repository is bare -
/// the push-triggered hooks, per `githooks(5)`: "An exception are hooks triggered during a push
/// ('pre-receive', 'update', 'post-receive', 'post-update', 'push-to-checkout') which are always
/// executed in `$GIT_DIR`."
pub const ALWAYS_GIT_DIR_HOOKS: &[&str] = &[
    "pre-receive",
    "update",
    "post-receive",
    "post-update",
    "push-to-checkout",
];

/// The directory git itself would run the hook named `name` in, per `githooks(5)`: `$GIT_DIR`
/// for [`ALWAYS_GIT_DIR_HOOKS`] and for bare repositories (`worktree_dir` is `None`), otherwise
/// `worktree_dir`.
pub fn cwd_for<'a>(name: &str, git_dir: &'a Path, worktree_dir: Option<&'a Path>) -> &'a Path {
    match worktree_dir {
        Some(worktree_dir) if !ALWAYS_GIT_DIR_HOOKS.contains(&name) => worktree_dir,
        _ => git_dir,
    }
}

/// Where `git commit` got the default log message from, per `prepare-commit-msg`'s second
/// positional argument.
#[derive(Debug, Clone, Copy)]
pub enum CommitMessageSource<'a> {
    /// A `-m` or `-F` option was given.
    Message,
    /// A `-t` option was given, or `commit.template` is set.
    Template,
    /// The commit is a merge, or a `.git/MERGE_MSG` file exists.
    Merge,
    /// A `.git/SQUASH_MSG` file exists.
    Squash,
    /// A `-c`, `-C`, or `--amend` option named this commit.
    Commit(&'a gix_hash::oid),
}

impl CommitMessageSource<'_> {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Message => "message",
            Self::Template => "template",
            Self::Merge => "merge",
            Self::Squash => "squash",
            Self::Commit(_) => "commit",
        }
    }
}

/// Which command triggered a `post-rewrite` call - its first positional argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostRewriteCommand {
    /// `git commit --amend`.
    Amend,
    /// `git rebase`.
    Rebase,
}

impl PostRewriteCommand {
    fn as_str(self) -> &'static str {
        match self {
            Self::Amend => "amend",
            Self::Rebase => "rebase",
        }
    }
}

/// Positional arguments for hooks whose argv construction can't fail - every named hook with
/// its own argument contract except `update` and `reference-transaction`, whose own dedicated
/// functions ([`update_args()`], [`reference_transaction_args()`]) validate a ref name and so
/// return `Result`, unlike everything here.
///
/// Hooks with no positional arguments at all - `pre-applypatch`, `post-applypatch`,
/// `pre-commit`, `pre-merge-commit`, `post-commit`, `pre-receive`, `post-receive`,
/// `proc-receive`, `pre-auto-gc` - have no variant here either; there's nothing to build.
///
/// `p4-*` and `fsmonitor-watchman` are deliberately out of scope: the former are `git-p4`
/// (Perforce bridge)-specific, and the latter uses a NUL-delimited stdout protocol rather than
/// a plain argv/stdin contract - neither is relevant to a Gitaly-style git server.
///
/// Verified against git's own source at tag `v2.47.0`, the same tag every other hook-contract
/// claim in this crate is pinned to.
#[derive(Debug, Clone, Copy)]
pub enum HookArgs<'a> {
    /// `applypatch-msg`: the file holding the proposed commit log message.
    ApplypatchMsg {
        /// The file holding the proposed commit log message.
        message_file: &'a Path,
    },
    /// `commit-msg`: the file holding the proposed commit log message.
    CommitMsg {
        /// The file holding the proposed commit log message.
        message_file: &'a Path,
    },
    /// `prepare-commit-msg`: the message file, and where the default message came from.
    PrepareCommitMsg {
        /// The file holding the proposed commit log message.
        message_file: &'a Path,
        /// Where the default message came from.
        source: CommitMessageSource<'a>,
    },
    /// `pre-rebase`: the upstream the series was forked from, and the branch being rebased -
    /// `None` when rebasing the current branch.
    PreRebase {
        /// The upstream the series was forked from.
        upstream: &'a str,
        /// The branch being rebased, or `None` when rebasing the current branch.
        branch: Option<&'a str>,
    },
    /// `post-checkout`: the previous and new `HEAD`, and whether this was a branch checkout.
    /// For `git clone`/`git worktree add` (unless `--no-checkout`), `previous_head` is the
    /// all-zeroes id and this is always a branch checkout.
    PostCheckout {
        /// The ref of the previous `HEAD`, or the all-zeroes id for a fresh clone/worktree.
        previous_head: &'a gix_hash::oid,
        /// The ref of the new `HEAD`.
        new_head: &'a gix_hash::oid,
        /// `true` for a branch checkout (changing branches), `false` for a file checkout
        /// (retrieving a file from the index).
        is_branch_checkout: bool,
    },
    /// `post-merge`: whether the merge being done was a squash merge.
    PostMerge {
        /// Whether the merge being done was a squash merge.
        is_squash: bool,
    },
    /// `pre-push`: the destination remote's name and location (URL) - the same value for both
    /// if no named remote is used.
    PrePush {
        /// The destination remote's name.
        remote_name: &'a str,
        /// The destination remote's location (URL).
        remote_location: &'a str,
    },
    /// `push-to-checkout`: the commit the currently checked-out branch's tip is being updated to.
    PushToCheckout {
        /// The commit the currently checked-out branch's tip is being updated to.
        commit: &'a gix_hash::oid,
    },
    /// `post-update`: the refs that were actually updated. Unlike [`update_args()`], these
    /// aren't validated as ref names - git only ever calls this hook with names it produced
    /// itself, so there's no untrusted input to guard against here the way there is for values
    /// arriving over the wire from a pusher.
    PostUpdate {
        /// The refs that were actually updated.
        updated_refs: &'a [&'a str],
    },
    /// `post-rewrite`: which command triggered the rewrite.
    PostRewrite {
        /// Which command triggered the rewrite.
        command: PostRewriteCommand,
    },
    /// `sendemail-validate`: the file holding the email body, and the file holding its SMTP headers.
    SendemailValidate {
        /// The file holding the contents of the email to be sent.
        email_file: &'a Path,
        /// The file holding the SMTP headers of the email.
        headers_file: &'a Path,
    },
    /// `post-index-change`: whether the working directory was updated, and whether the index's
    /// skip-worktree bits may have changed. Git documents that both are never `true` together.
    PostIndexChange {
        /// Whether the working directory was updated.
        working_directory_updated: bool,
        /// Whether the index was updated and the skip-worktree bit could have changed.
        skip_worktree_bits_changed: bool,
    },
}

impl HookArgs<'_> {
    /// Add this variant's positional arguments to `prepared`, in the order git documents them.
    pub fn apply(self, prepared: gix_command::Prepare) -> gix_command::Prepare {
        fn flag(value: bool) -> &'static str {
            if value { "1" } else { "0" }
        }

        match self {
            Self::ApplypatchMsg { message_file } | Self::CommitMsg { message_file } => prepared.arg(message_file),
            Self::PrepareCommitMsg { message_file, source } => {
                let prepared = prepared.arg(message_file).arg(source.as_str());
                match source {
                    CommitMessageSource::Commit(oid) => prepared.arg(oid.to_hex().to_string()),
                    _ => prepared,
                }
            }
            Self::PreRebase { upstream, branch } => {
                let prepared = prepared.arg(upstream);
                match branch {
                    Some(branch) => prepared.arg(branch),
                    None => prepared,
                }
            }
            Self::PostCheckout {
                previous_head,
                new_head,
                is_branch_checkout,
            } => prepared
                .arg(previous_head.to_hex().to_string())
                .arg(new_head.to_hex().to_string())
                .arg(flag(is_branch_checkout)),
            Self::PostMerge { is_squash } => prepared.arg(flag(is_squash)),
            Self::PrePush {
                remote_name,
                remote_location,
            } => prepared.arg(remote_name).arg(remote_location),
            Self::PushToCheckout { commit } => prepared.arg(commit.to_hex().to_string()),
            Self::PostUpdate { updated_refs } => updated_refs.iter().fold(prepared, |p, r| p.arg(*r)),
            Self::PostRewrite { command } => prepared.arg(command.as_str()),
            Self::SendemailValidate {
                email_file,
                headers_file,
            } => prepared.arg(email_file).arg(headers_file),
            Self::PostIndexChange {
                working_directory_updated,
                skip_worktree_bits_changed,
            } => prepared
                .arg(flag(working_directory_updated))
                .arg(flag(skip_worktree_bits_changed)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Collect a `Prepare`'s env vars as `(key, value)` string pairs, for asserting on.
    fn env_of(prepared: gix_command::Prepare) -> Vec<(String, String)> {
        let cmd = std::process::Command::from(prepared);
        cmd.get_envs()
            .map(|(k, v)| (k.to_str().unwrap().to_owned(), v.unwrap().to_str().unwrap().to_owned()))
            .collect()
    }

    mod push_option_env {
        use crate::{command, push_option_env};

        fn prepared() -> gix_command::Prepare {
            command("git".as_ref(), "/repo/.git".as_ref(), "/repo".as_ref())
        }

        #[test]
        fn no_options_sets_count_zero() {
            let env = super::env_of(push_option_env(prepared(), &[] as &[&str]));
            assert!(env.contains(&("GIT_PUSH_OPTION_COUNT".into(), "0".into())));
        }

        #[test]
        fn options_are_indexed_in_order() {
            let env = super::env_of(push_option_env(prepared(), &["ci.skip", "reviewer=jane"]));
            assert!(env.contains(&("GIT_PUSH_OPTION_COUNT".into(), "2".into())));
            assert!(env.contains(&("GIT_PUSH_OPTION_0".into(), "ci.skip".into())));
            assert!(env.contains(&("GIT_PUSH_OPTION_1".into(), "reviewer=jane".into())));
        }
    }

    mod no_editor_env {
        use crate::{command, no_editor_env};

        #[test]
        fn sets_git_editor_to_a_no_op() {
            let prepared = command("pre-commit".as_ref(), "/repo/.git".as_ref(), "/repo".as_ref());
            let env = super::env_of(no_editor_env(prepared));
            assert!(env.contains(&("GIT_EDITOR".into(), ":".into())));
        }
    }

    mod quarantine_env {
        use std::path::Path;

        use crate::{command, quarantine_env};

        fn prepared() -> gix_command::Prepare {
            command("git".as_ref(), "/repo/.git".as_ref(), "/repo".as_ref())
        }

        #[test]
        fn sets_the_three_object_directory_variables() {
            let env = super::env_of(quarantine_env(
                prepared(),
                Path::new("/repo/.git/objects/incoming-123"),
                Path::new("/repo/.git/objects"),
            ));
            assert!(env.contains(&("GIT_QUARANTINE_PATH".into(), "/repo/.git/objects/incoming-123".into())));
            assert!(env.contains(&("GIT_OBJECT_DIRECTORY".into(), "/repo/.git/objects/incoming-123".into())));
            assert!(env.contains(&("GIT_ALTERNATE_OBJECT_DIRECTORIES".into(), "/repo/.git/objects".into())));
        }
    }

    /// A valid, arbitrary SHA-1 `ObjectId` for use as test data - never parsed from untrusted input.
    fn oid(hex: &str) -> gix_hash::ObjectId {
        gix_hash::ObjectId::from_hex(hex.as_bytes()).unwrap()
    }

    mod push_cert_env {
        use crate::{PushCert, command, push_cert_env};

        fn prepared() -> gix_command::Prepare {
            command("git".as_ref(), "/repo/.git".as_ref(), "/repo".as_ref())
        }

        #[test]
        fn sets_all_required_fields() {
            let blob = super::oid("0000000000000000000000000000000000000000");
            let cert = PushCert {
                blob: &blob,
                signer: "Jane Doe <jane@example.com>",
                key: "0xDEADBEEF",
                status: "G",
                nonce: "some-nonce",
                nonce_status: "OK",
                nonce_slop: None,
            };
            let env = super::env_of(push_cert_env(prepared(), &cert));
            assert!(env.contains(&(
                "GIT_PUSH_CERT".into(),
                "0000000000000000000000000000000000000000".into()
            )));
            assert!(env.contains(&("GIT_PUSH_CERT_SIGNER".into(), "Jane Doe <jane@example.com>".into())));
            assert!(env.contains(&("GIT_PUSH_CERT_KEY".into(), "0xDEADBEEF".into())));
            assert!(env.contains(&("GIT_PUSH_CERT_STATUS".into(), "G".into())));
            assert!(env.contains(&("GIT_PUSH_CERT_NONCE".into(), "some-nonce".into())));
            assert!(env.contains(&("GIT_PUSH_CERT_NONCE_STATUS".into(), "OK".into())));
            assert!(!env.iter().any(|(k, _)| k == "GIT_PUSH_CERT_NONCE_SLOP"));
        }

        #[test]
        fn nonce_slop_is_set_only_when_present() {
            let blob = super::oid("0000000000000000000000000000000000000000");
            let cert = PushCert {
                blob: &blob,
                signer: "Jane Doe <jane@example.com>",
                key: "0xDEADBEEF",
                status: "G",
                nonce: "some-nonce",
                nonce_status: "SLOP",
                nonce_slop: Some("5"),
            };
            let env = super::env_of(push_cert_env(prepared(), &cert));
            assert!(env.contains(&("GIT_PUSH_CERT_NONCE_SLOP".into(), "5".into())));
        }
    }

    mod update_args {
        use crate::{command, update_args};

        #[test]
        fn args_are_ref_name_old_oid_new_oid_in_order() {
            let old = super::oid("0000000000000000000000000000000000000000");
            let new = super::oid("1111111111111111111111111111111111111111");
            let prepared = update_args(
                command("update".as_ref(), "/repo/.git".as_ref(), "/repo/.git".as_ref()),
                "refs/heads/main",
                &old,
                &new,
            )
            .unwrap();
            let cmd = std::process::Command::from(prepared);
            let args: Vec<_> = cmd.get_args().map(|a| a.to_str().unwrap()).collect();
            assert_eq!(
                args,
                [
                    "refs/heads/main",
                    "0000000000000000000000000000000000000000",
                    "1111111111111111111111111111111111111111",
                ]
            );
        }

        #[test]
        fn invalid_ref_name_is_rejected() {
            let old = super::oid("0000000000000000000000000000000000000000");
            let new = super::oid("1111111111111111111111111111111111111111");
            let result = update_args(
                command("update".as_ref(), "/repo/.git".as_ref(), "/repo/.git".as_ref()),
                "refs/heads/../escape",
                &old,
                &new,
            );
            assert!(result.is_err(), "a ref name with a repeated dot must be rejected");
        }
    }

    mod receive_stdin {
        use crate::receive_stdin;

        #[test]
        fn no_updates_is_empty() {
            assert_eq!(
                receive_stdin(std::iter::empty::<(&gix_hash::oid, &gix_hash::oid, &str)>()).unwrap(),
                b""
            );
        }

        #[test]
        fn one_line_per_update_old_new_ref() {
            let a = super::oid("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
            let b = super::oid("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
            let zero = super::oid("0000000000000000000000000000000000000000");
            let c = super::oid("cccccccccccccccccccccccccccccccccccccccc");
            let updates = [(&a, &b, "refs/heads/main"), (&zero, &c, "refs/heads/feature")];
            assert_eq!(
                receive_stdin(updates.map(|(old, new, name)| (old.as_ref(), new.as_ref(), name))).unwrap(),
                b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb refs/heads/main\n\
                  0000000000000000000000000000000000000000 cccccccccccccccccccccccccccccccccccccccc refs/heads/feature\n"
                    .as_slice()
            );
        }

        #[test]
        fn invalid_ref_name_is_rejected() {
            let old = super::oid("0000000000000000000000000000000000000000");
            let new = super::oid("1111111111111111111111111111111111111111");
            let result = receive_stdin([(old.as_ref(), new.as_ref(), "refs/heads/../escape")]);
            assert!(result.is_err(), "a ref name with a repeated dot must be rejected");
        }
    }

    fn write_hook(dir: &Path, name: &str, executable: bool) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, b"#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = if executable { 0o755 } else { 0o644 };
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
        }
        let _ = executable;
        path
    }

    #[test]
    fn missing_hook_is_none() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(find("pre-commit", dir.path(), None), None);
    }

    #[test]
    fn present_and_executable_hook_is_found() {
        let dir = tempfile::tempdir().unwrap();
        let hooks = dir.path().join(HOOKS_DIR);
        std::fs::create_dir(&hooks).unwrap();
        let hook = write_hook(&hooks, "pre-commit", true);
        assert_eq!(find("pre-commit", dir.path(), None), Some(hook));
    }

    #[cfg(unix)]
    #[test]
    fn present_but_non_executable_hook_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let hooks = dir.path().join(HOOKS_DIR);
        std::fs::create_dir(&hooks).unwrap();
        write_hook(&hooks, "pre-commit", false);
        assert_eq!(find("pre-commit", dir.path(), None), None);
    }

    #[test]
    fn hooks_path_override_takes_precedence() {
        let dir = tempfile::tempdir().unwrap();
        let custom = tempfile::tempdir().unwrap();
        let hook = write_hook(custom.path(), "pre-commit", true);
        assert_eq!(find("pre-commit", dir.path(), Some(custom.path())), Some(hook));
    }

    /// `find()`'s Windows branch (`cfg!(windows)`) is dead code when compiled and tested on
    /// Unix, so these test `windows_find()` directly - the same thing `find()` calls -
    /// mirroring how gix-command's own test suite exercises its Windows-only lookup logic
    /// unconditionally rather than gating it to Windows-only CI.
    mod windows_find {
        use crate::windows_find;

        #[test]
        fn exact_name_is_preferred_over_exe() {
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(dir.path().join("pre-commit"), b"").unwrap();
            std::fs::write(dir.path().join("pre-commit.exe"), b"").unwrap();
            assert_eq!(
                windows_find(dir.path(), "pre-commit"),
                Some(dir.path().join("pre-commit")),
                "matches git's own find_hook() in hook.c: it tries the exact name via access(X_OK) \
                 first, and only appends STRIP_EXTENSION (\".exe\") if that's missing"
            );
        }

        #[test]
        fn falls_back_to_exe_when_exact_name_is_missing() {
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(dir.path().join("pre-commit.exe"), b"").unwrap();
            assert_eq!(
                windows_find(dir.path(), "pre-commit"),
                Some(dir.path().join("pre-commit.exe"))
            );
        }

        #[test]
        fn missing_hook_is_none() {
            let dir = tempfile::tempdir().unwrap();
            assert_eq!(windows_find(dir.path(), "pre-commit"), None);
        }
    }

    mod hooks_path_from_config {
        use std::path::{Path, PathBuf};

        use crate::hooks_path_from_config;

        fn config(input: &str) -> gix_config::File {
            input.parse().unwrap()
        }

        #[test]
        fn unset_key_is_none() {
            let cfg = config("[core]\n\tbare = false\n");
            assert_eq!(hooks_path_from_config(&cfg, Default::default()).unwrap(), None);
        }

        #[test]
        fn plain_path_is_returned_unchanged() {
            let cfg = config("[core]\n\thooksPath = /custom/hooks\n");
            assert_eq!(
                hooks_path_from_config(&cfg, Default::default()).unwrap(),
                Some(PathBuf::from("/custom/hooks"))
            );
        }

        #[test]
        fn tilde_is_expanded_using_home_dir() {
            let cfg = config("[core]\n\thooksPath = ~/my-hooks\n");
            let home = Path::new("/home/tester");
            let ctx = gix_config::path::interpolate::Context {
                home_dir: Some(home),
                ..Default::default()
            };
            assert_eq!(hooks_path_from_config(&cfg, ctx).unwrap(), Some(home.join("my-hooks")));
        }

        #[test]
        fn tilde_without_home_dir_errors() {
            let cfg = config("[core]\n\thooksPath = ~/my-hooks\n");
            assert!(hooks_path_from_config(&cfg, Default::default()).is_err());
        }
    }

    mod outcome {
        use std::process::ExitStatus;

        use crate::Outcome;

        #[cfg(unix)]
        fn status(code: i32) -> ExitStatus {
            use std::os::unix::process::ExitStatusExt;
            ExitStatus::from_raw(code << 8)
        }

        #[cfg(unix)]
        #[test]
        fn zero_is_success() {
            assert_eq!(Outcome::from(status(0)), Outcome::Success);
        }

        #[cfg(unix)]
        #[test]
        fn non_zero_is_rejected_with_code() {
            assert_eq!(Outcome::from(status(1)), Outcome::Rejected { code: Some(1) });
        }
    }

    mod is_gating {
        use crate::is_gating;

        #[test]
        fn pre_hooks_gate() {
            for name in [
                "pre-commit",
                "pre-merge-commit",
                "prepare-commit-msg",
                "commit-msg",
                "pre-rebase",
            ] {
                assert!(is_gating(name), "{name} should gate per githooks(5)");
            }
        }

        #[test]
        fn post_hooks_are_advisory() {
            for name in [
                "post-applypatch",
                "post-commit",
                "post-merge",
                "post-receive",
                "post-update",
                "post-checkout",
                "post-rewrite",
            ] {
                assert!(!is_gating(name), "{name} should be advisory-only per githooks(5)");
            }
        }

        #[test]
        fn update_gates_its_own_ref() {
            assert!(is_gating("update"));
        }

        #[test]
        fn proc_receive_gates_the_reported_group_of_commands() {
            assert!(
                is_gating("proc-receive"),
                "non-zero exit fails the group of commands sent to it, unless atomic push is in use"
            );
        }

        #[test]
        fn unrecognized_hook_defaults_to_gating() {
            assert!(is_gating("some-future-hook-we-dont-know-about"));
        }
    }

    #[cfg(unix)]
    #[test]
    fn command_runs_in_configured_cwd() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let hooks = dir.path().join(HOOKS_DIR);
        std::fs::create_dir(&hooks).unwrap();
        let worktree = tempfile::tempdir().unwrap();
        let out_file = dir.path().join("pwd.txt");

        let hook = hooks.join("pre-commit");
        std::fs::write(&hook, format!("#!/bin/sh\npwd > {}\n", out_file.display())).unwrap();
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();

        let status = command(&hook, &dir.path().join(".git"), worktree.path())
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
        assert!(status.success());

        let recorded = std::fs::read_to_string(&out_file).unwrap();
        assert_eq!(
            std::fs::canonicalize(recorded.trim()).unwrap(),
            std::fs::canonicalize(worktree.path()).unwrap(),
            "the hook observed the cwd we configured, not the test process's own cwd"
        );
    }

    #[cfg(unix)]
    #[test]
    fn run_classifies_a_successful_hook() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let hook = dir.path().join("pre-commit");
        std::fs::write(&hook, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();

        let outcome = run(command(&hook, &dir.path().join(".git"), dir.path())).unwrap();
        assert_eq!(outcome, Outcome::Success);
    }

    #[cfg(unix)]
    #[test]
    fn run_classifies_a_rejecting_hook() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let hook = dir.path().join("pre-commit");
        std::fs::write(&hook, "#!/bin/sh\nexit 3\n").unwrap();
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();

        let outcome = run(command(&hook, &dir.path().join(".git"), dir.path())).unwrap();
        assert_eq!(outcome, Outcome::Rejected { code: Some(3) });
    }

    mod transaction_state {
        use crate::TransactionState;

        #[test]
        fn as_str_matches_what_git_passes_on_argv() {
            assert_eq!(TransactionState::Prepared.as_str(), "prepared");
            assert_eq!(TransactionState::Committed.as_str(), "committed");
            assert_eq!(TransactionState::Aborted.as_str(), "aborted");
        }

        #[test]
        fn only_prepared_is_gating() {
            assert!(TransactionState::Prepared.is_gating());
            assert!(!TransactionState::Committed.is_gating());
            assert!(!TransactionState::Aborted.is_gating());
        }
    }

    mod reference_transaction_args {
        use crate::{TransactionState, command, reference_transaction_args};

        #[test]
        fn adds_the_state_as_the_only_argument() {
            let prepared = reference_transaction_args(
                command(
                    "reference-transaction".as_ref(),
                    "/repo/.git".as_ref(),
                    "/repo/.git".as_ref(),
                ),
                TransactionState::Prepared,
            );
            let cmd = std::process::Command::from(prepared);
            let args: Vec<_> = cmd.get_args().map(|a| a.to_str().unwrap()).collect();
            assert_eq!(args, ["prepared"]);
        }
    }

    mod reference_transaction_stdin {
        use crate::{TransactionValue, reference_transaction_stdin};

        #[test]
        fn oid_update_matches_receive_stdin_format() {
            let old = super::oid("0000000000000000000000000000000000000000");
            let new = super::oid("1111111111111111111111111111111111111111");
            let updates = [(
                TransactionValue::Oid(old.as_ref()),
                TransactionValue::Oid(new.as_ref()),
                "refs/heads/main",
            )];
            assert_eq!(
                reference_transaction_stdin(updates).unwrap(),
                b"0000000000000000000000000000000000000000 1111111111111111111111111111111111111111 refs/heads/main\n"
                    .as_slice()
            );
        }

        #[test]
        fn symbolic_update_uses_ref_prefix() {
            let updates = [(
                TransactionValue::SymbolicTarget("refs/heads/old-default"),
                TransactionValue::SymbolicTarget("refs/heads/main"),
                "HEAD",
            )];
            assert_eq!(
                reference_transaction_stdin(updates).unwrap(),
                b"ref:refs/heads/old-default ref:refs/heads/main HEAD\n".as_slice(),
                "no space between \"ref:\" and the target - confirmed against refs.c's \
                 strbuf_addf(&buf, \"ref:%s \", ...)"
            );
        }

        #[test]
        fn invalid_ref_name_is_rejected() {
            let old = super::oid("0000000000000000000000000000000000000000");
            let new = super::oid("1111111111111111111111111111111111111111");
            let result = reference_transaction_stdin([(
                TransactionValue::Oid(old.as_ref()),
                TransactionValue::Oid(new.as_ref()),
                "refs/heads/../escape",
            )]);
            assert!(result.is_err());
        }

        #[test]
        fn invalid_symbolic_target_is_rejected() {
            let old = super::oid("0000000000000000000000000000000000000000");
            let updates = [(
                TransactionValue::Oid(old.as_ref()),
                TransactionValue::SymbolicTarget("refs/heads/../escape"),
                "HEAD",
            )];
            assert!(
                reference_transaction_stdin(updates).is_err(),
                "a symbolic target is itself a ref name and must be validated the same way"
            );
        }
    }

    mod hook_args {
        use std::path::Path;

        use crate::{CommitMessageSource, HookArgs, PostRewriteCommand, command};

        fn args(hook_args: HookArgs<'_>) -> Vec<String> {
            let prepared = hook_args.apply(command("hook".as_ref(), "/repo/.git".as_ref(), "/repo".as_ref()));
            std::process::Command::from(prepared)
                .get_args()
                .map(|a| a.to_str().unwrap().to_owned())
                .collect()
        }

        #[test]
        fn applypatch_msg_is_the_message_file() {
            assert_eq!(
                args(HookArgs::ApplypatchMsg {
                    message_file: Path::new("/repo/.git/COMMIT_EDITMSG")
                }),
                ["/repo/.git/COMMIT_EDITMSG"]
            );
        }

        #[test]
        fn commit_msg_is_the_message_file() {
            assert_eq!(
                args(HookArgs::CommitMsg {
                    message_file: Path::new("/repo/.git/COMMIT_EDITMSG")
                }),
                ["/repo/.git/COMMIT_EDITMSG"]
            );
        }

        #[test]
        fn prepare_commit_msg_without_commit_source_has_two_args() {
            assert_eq!(
                args(HookArgs::PrepareCommitMsg {
                    message_file: Path::new("/repo/.git/COMMIT_EDITMSG"),
                    source: CommitMessageSource::Message,
                }),
                ["/repo/.git/COMMIT_EDITMSG", "message"]
            );
        }

        #[test]
        fn prepare_commit_msg_with_commit_source_adds_the_oid() {
            let oid = super::oid("1111111111111111111111111111111111111111");
            assert_eq!(
                args(HookArgs::PrepareCommitMsg {
                    message_file: Path::new("/repo/.git/COMMIT_EDITMSG"),
                    source: CommitMessageSource::Commit(oid.as_ref()),
                }),
                [
                    "/repo/.git/COMMIT_EDITMSG",
                    "commit",
                    "1111111111111111111111111111111111111111"
                ]
            );
        }

        #[test]
        fn pre_rebase_without_branch_has_one_arg() {
            assert_eq!(
                args(HookArgs::PreRebase {
                    upstream: "origin/main",
                    branch: None
                }),
                ["origin/main"]
            );
        }

        #[test]
        fn pre_rebase_with_branch_has_two_args() {
            assert_eq!(
                args(HookArgs::PreRebase {
                    upstream: "origin/main",
                    branch: Some("topic")
                }),
                ["origin/main", "topic"]
            );
        }

        #[test]
        fn post_checkout_args_are_prev_new_flag_in_order() {
            let prev = super::oid("0000000000000000000000000000000000000000");
            let new = super::oid("1111111111111111111111111111111111111111");
            assert_eq!(
                args(HookArgs::PostCheckout {
                    previous_head: prev.as_ref(),
                    new_head: new.as_ref(),
                    is_branch_checkout: true,
                }),
                [
                    "0000000000000000000000000000000000000000",
                    "1111111111111111111111111111111111111111",
                    "1"
                ]
            );
        }

        #[test]
        fn post_merge_flag_is_0_or_1() {
            assert_eq!(args(HookArgs::PostMerge { is_squash: false }), ["0"]);
            assert_eq!(args(HookArgs::PostMerge { is_squash: true }), ["1"]);
        }

        #[test]
        fn pre_push_args_are_name_then_location() {
            assert_eq!(
                args(HookArgs::PrePush {
                    remote_name: "origin",
                    remote_location: "git@example.com:repo.git"
                }),
                ["origin", "git@example.com:repo.git"]
            );
        }

        #[test]
        fn push_to_checkout_is_the_target_commit() {
            let commit = super::oid("2222222222222222222222222222222222222222");
            assert_eq!(
                args(HookArgs::PushToCheckout {
                    commit: commit.as_ref()
                }),
                ["2222222222222222222222222222222222222222"]
            );
        }

        #[test]
        fn post_update_lists_every_updated_ref() {
            assert_eq!(
                args(HookArgs::PostUpdate {
                    updated_refs: &["refs/heads/main", "refs/heads/topic"]
                }),
                ["refs/heads/main", "refs/heads/topic"]
            );
        }

        #[test]
        fn post_rewrite_command_is_amend_or_rebase() {
            assert_eq!(
                args(HookArgs::PostRewrite {
                    command: PostRewriteCommand::Amend
                }),
                ["amend"]
            );
            assert_eq!(
                args(HookArgs::PostRewrite {
                    command: PostRewriteCommand::Rebase
                }),
                ["rebase"]
            );
        }

        #[test]
        fn sendemail_validate_args_are_email_then_headers() {
            assert_eq!(
                args(HookArgs::SendemailValidate {
                    email_file: Path::new("/tmp/0001-patch.eml"),
                    headers_file: Path::new("/tmp/headers"),
                }),
                ["/tmp/0001-patch.eml", "/tmp/headers"]
            );
        }

        #[test]
        fn post_index_change_flags_are_0_or_1_in_order() {
            assert_eq!(
                args(HookArgs::PostIndexChange {
                    working_directory_updated: true,
                    skip_worktree_bits_changed: false,
                }),
                ["1", "0"]
            );
            assert_eq!(
                args(HookArgs::PostIndexChange {
                    working_directory_updated: false,
                    skip_worktree_bits_changed: true,
                }),
                ["0", "1"]
            );
        }
    }

    mod pre_push_stdin {
        use crate::pre_push_stdin;

        #[test]
        fn normal_update_matches_the_documented_field_order() {
            let local = super::oid("1111111111111111111111111111111111111111");
            let remote = super::oid("2222222222222222222222222222222222222222");
            let updates = [(
                "refs/heads/master",
                local.as_ref(),
                "refs/heads/foreign",
                remote.as_ref(),
            )];
            assert_eq!(
                pre_push_stdin(updates).unwrap(),
                b"refs/heads/master 1111111111111111111111111111111111111111 \
                  refs/heads/foreign 2222222222222222222222222222222222222222\n"
                    .as_slice()
            );
        }

        #[test]
        fn delete_marker_is_not_validated_as_a_ref_name() {
            let zero = super::oid("0000000000000000000000000000000000000000");
            let remote = super::oid("2222222222222222222222222222222222222222");
            let updates = [("(delete)", zero.as_ref(), "refs/heads/foreign", remote.as_ref())];
            assert!(
                pre_push_stdin(updates).is_ok(),
                "\"(delete)\" is documented, not a ref name"
            );
        }

        #[test]
        fn arbitrary_local_source_text_is_not_validated_as_a_ref_name() {
            let local = super::oid("1111111111111111111111111111111111111111");
            let remote = super::oid("2222222222222222222222222222222222222222");
            let updates = [("HEAD~", local.as_ref(), "refs/heads/foreign", remote.as_ref())];
            assert!(
                pre_push_stdin(updates).is_ok(),
                "git documents local-ref may be supplied as originally given, e.g. HEAD~"
            );
        }

        #[test]
        fn local_ref_with_embedded_newline_is_rejected() {
            let local = super::oid("1111111111111111111111111111111111111111");
            let remote = super::oid("2222222222222222222222222222222222222222");
            let updates = [(
                "evil\nrefs/heads/injected",
                local.as_ref(),
                "refs/heads/foreign",
                remote.as_ref(),
            )];
            assert!(pre_push_stdin(updates).is_err());
        }

        #[test]
        fn local_ref_with_embedded_nul_is_rejected() {
            let local = super::oid("1111111111111111111111111111111111111111");
            let remote = super::oid("2222222222222222222222222222222222222222");
            let updates = [(
                "evil\0refs/heads/injected",
                local.as_ref(),
                "refs/heads/foreign",
                remote.as_ref(),
            )];
            assert!(
                pre_push_stdin(updates).is_err(),
                "a raw NUL is a control byte too, not just newline"
            );
        }

        #[test]
        fn local_ref_with_unicode_is_accepted() {
            let local = super::oid("1111111111111111111111111111111111111111");
            let remote = super::oid("2222222222222222222222222222222222222222");
            let updates = [(
                "HEAD~\u{0301}\u{0301}\u{0301}",
                local.as_ref(),
                "refs/heads/foreign",
                remote.as_ref(),
            )];
            assert!(
                pre_push_stdin(updates).is_ok(),
                "non-ASCII Unicode, even heavily combined, isn't a control byte and git allows it in ref-shaped text"
            );
        }

        #[test]
        fn invalid_remote_ref_is_rejected() {
            let local = super::oid("1111111111111111111111111111111111111111");
            let remote = super::oid("2222222222222222222222222222222222222222");
            let updates = [(
                "refs/heads/master",
                local.as_ref(),
                "refs/heads/../escape",
                remote.as_ref(),
            )];
            assert!(
                pre_push_stdin(updates).is_err(),
                "unlike local-ref, remote-ref is always a real destination ref and must validate"
            );
        }
    }

    mod cwd_for {
        use std::path::Path;

        use crate::cwd_for;

        #[test]
        fn ordinary_hooks_use_the_worktree_when_present() {
            let git_dir = Path::new("/repo/.git");
            let worktree = Path::new("/repo");
            assert_eq!(cwd_for("pre-commit", git_dir, Some(worktree)), worktree);
        }

        #[test]
        fn ordinary_hooks_use_git_dir_when_bare() {
            let git_dir = Path::new("/repo.git");
            assert_eq!(cwd_for("pre-commit", git_dir, None), git_dir);
        }

        #[test]
        fn push_triggered_hooks_always_use_git_dir() {
            let git_dir = Path::new("/repo/.git");
            let worktree = Path::new("/repo");
            for name in [
                "pre-receive",
                "update",
                "post-receive",
                "post-update",
                "push-to-checkout",
            ] {
                assert_eq!(
                    cwd_for(name, git_dir, Some(worktree)),
                    git_dir,
                    "{name} must always run in $GIT_DIR per githooks(5), even with a worktree present"
                );
            }
        }
    }
}
