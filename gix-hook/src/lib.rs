//! Discover and execute git hooks the way `git` itself does.
//!
//! This is an early prototype covering only the client-side basics: locating a hook script
//! below `$GIT_DIR/hooks` (or a configured `core.hooksPath`), reading that override from
//! `gix-config`, and preparing the hook for execution with [`gix-command`].
//!
//! Missing, by design, until validated further:
//! - receive-side / `reference-transaction` hooks and quarantine-aware execution
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

#[cfg(test)]
mod tests {
    use super::*;

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
