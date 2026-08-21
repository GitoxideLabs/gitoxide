//! Discover and execute git hooks the way `git` itself does.
//!
//! This is an early prototype covering only the client-side basics: locating a hook script
//! below `$GIT_DIR/hooks` (or a configured `core.hooksPath`), reading that override from
//! `gix-config`, and preparing the hook for execution with [`gix-command`].
//!
//! Receive-side hooks get env-var helpers ([`push_option_env()`], [`quarantine_env()`],
//! [`push_cert_env()`]), stdin formatting ([`receive_stdin()`]) for `pre-receive`/`post-receive`,
//! and argument formatting ([`update_args()`]) for `update`.
//!
//! Missing, by design, until validated further:
//! - `reference-transaction` state handling
//! - Windows executable-extension resolution (`.exe`, `.bat`, `.cmd`, …)
#![deny(missing_docs, rust_2018_idioms)]
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

/// The default directory name, relative to `$GIT_DIR`, that holds hook scripts.
pub const HOOKS_DIR: &str = "hooks";

/// Find the hook named `name`, returning its path if it exists and is executable.
///
/// `git_dir` is the repository's `.git` directory. `hooks_path`, if set, overrides the
/// default `$GIT_DIR/hooks` location, mirroring `core.hooksPath`.
///
/// Like `git`, a hook that exists but isn't executable is treated as absent.
pub fn find(name: &str, git_dir: &Path, hooks_path: Option<&Path>) -> Option<PathBuf> {
    let base = hooks_path.map_or_else(|| git_dir.join(HOOKS_DIR), Path::to_owned);
    let candidate = base.join(name);
    is_executable_file(&candidate).then_some(candidate)
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    // Executability isn't governed by a permission bit on these platforms;
    // git relies on file extensions and its own launcher logic instead.
    path.is_file()
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
) -> Result<Option<PathBuf>, gix_config::path::interpolate::Error> {
    config
        .path("core.hooksPath")
        .map(|path| path.interpolate(ctx))
        .transpose()
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
pub fn run(prepared: gix_command::Prepare) -> std::io::Result<Outcome> {
    let status = prepared.spawn()?.wait()?;
    Ok(status.into())
}

/// Hook names whose exit status git ignores, per [`githooks(5)`](https://git-scm.com/docs/githooks).
/// Every other hook name, including ones not listed here at all, is treated as gating by
/// [`is_gating()`].
///
/// `reference-transaction` is deliberately absent: it only gates during its `prepared` state,
/// while `committed` and `aborted` calls are always advisory - a single name can't capture that,
/// so callers driving that hook must check its state argument themselves.
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

/// Add the push options git passes to `pre-receive` and `post-receive` when invoked via
/// `git push --push-option=<option>`: `GIT_PUSH_OPTION_COUNT` and `GIT_PUSH_OPTION_<n>` for each
/// option, in the order given.
///
/// Per `githooks(5)`: "If it is negotiated to not use the push options phase, the environment
/// variables will not be set" - so omit this call entirely rather than passing an empty slice
/// when push options were never negotiated for the push.
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
) -> Result<gix_command::Prepare, gix_validate::reference::name::Error> {
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
) -> Result<Vec<u8>, gix_validate::reference::name::Error> {
    use std::io::Write;

    let mut buf = Vec::new();
    for (old_oid, new_oid, ref_name) in updates {
        gix_validate::reference::name(ref_name.into())?;
        writeln!(buf, "{} {} {}", old_oid.to_hex(), new_oid.to_hex(), ref_name)
            .expect("writing to a Vec<u8> never fails");
    }
    Ok(buf)
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
}
