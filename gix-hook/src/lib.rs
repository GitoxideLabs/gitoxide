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
}
