//! End-to-end tests against a real `git` binary, verifying this crate's documented hook
//! contracts (cwd, argv, env, stdin) match what git actually does - not just what its docs or
//! source say. Unlike the rest of this crate's tests, there's no new library behavior being
//! driven here; these are a verification harness, not a red/green implementation cycle.
//!
//! Skips (rather than fails) if `git` isn't on `PATH`, since CI environments without it
//! shouldn't break the rest of the suite over a test whose whole point is checking against a
//! real git installation.
use std::{path::Path, process::Command};

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|out| out.status.success())
}

fn git(repo: &Path, args: &[&str]) {
    let output = Command::new("git").args(args).current_dir(repo).output().unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed in {}:\n{}",
        repo.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn init_repo(repo: &Path) {
    git(repo, &["init", "--quiet"]);
    git(repo, &["config", "user.email", "test@example.com"]);
    git(repo, &["config", "user.name", "Test"]);
}

/// Install a hook that records its cwd, argv, a chosen env var, and stdin to `capture_file`.
///
/// `hooks_dir` is `<repo>/.git/hooks` for a non-bare repo, or `<repo>/hooks` for a bare one -
/// hooks live directly under the bare repo root, not under a nonexistent `.git` subdirectory.
fn install_capture_hook(hooks_dir: &Path, hook_name: &str, capture_file: &Path, env_var_to_capture: &str) {
    std::fs::create_dir_all(hooks_dir).unwrap();
    let script = format!(
        "#!/bin/sh\n\
         {{\n\
         echo \"CWD:$(pwd)\"\n\
         echo \"ARGS:$*\"\n\
         echo \"{env_var_to_capture}:${{{env_var_to_capture}}}\"\n\
         echo STDIN_START\n\
         cat\n\
         echo STDIN_END\n\
         }} > {}\n",
        capture_file.display()
    );
    let hook_path = hooks_dir.join(hook_name);
    std::fs::write(&hook_path, script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook_path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn field<'a>(capture: &'a str, label: &str) -> &'a str {
    capture
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{label}:")))
        .unwrap_or_else(|| panic!("no {label:?} line in capture:\n{capture}"))
}

#[test]
fn pre_commit_runs_in_worktree_root_with_git_editor_unset_for_dash_m() {
    if !git_available() {
        eprintln!("skipping: git not found on PATH");
        return;
    }
    let repo = tempfile::tempdir().unwrap();
    let repo_path = repo.path().canonicalize().unwrap();
    init_repo(&repo_path);

    let capture = repo_path.join("pre-commit.capture");
    install_capture_hook(&repo_path.join(".git/hooks"), "pre-commit", &capture, "GIT_EDITOR");

    std::fs::write(repo_path.join("file.txt"), b"hello\n").unwrap();
    git(&repo_path, &["add", "file.txt"]);
    git(&repo_path, &["commit", "-m", "a message"]);

    let output = std::fs::read_to_string(&capture).unwrap();
    assert_eq!(
        field(&output, "CWD"),
        repo_path.to_str().unwrap(),
        "pre-commit's cwd should be the worktree root, matching gix_hook::cwd_for(\"pre-commit\", ..)"
    );
    assert_eq!(
        field(&output, "GIT_EDITOR"),
        ":",
        "git commit -m never shows an editor, so GIT_EDITOR should be \":\" per commit.c's \
         run_commit_hook - what gix_hook::no_editor_env() sets"
    );
}

#[test]
fn commit_msg_receives_exactly_one_arg_the_message_file() {
    if !git_available() {
        eprintln!("skipping: git not found on PATH");
        return;
    }
    let repo = tempfile::tempdir().unwrap();
    let repo_path = repo.path().canonicalize().unwrap();
    init_repo(&repo_path);

    let capture = repo_path.join("commit-msg.capture");
    install_capture_hook(&repo_path.join(".git/hooks"), "commit-msg", &capture, "GIT_EDITOR");

    std::fs::write(repo_path.join("file.txt"), b"hello\n").unwrap();
    git(&repo_path, &["add", "file.txt"]);
    git(&repo_path, &["commit", "-m", "a message"]);

    let output = std::fs::read_to_string(&capture).unwrap();
    let args = field(&output, "ARGS");
    let message_file = args.trim();
    assert!(
        !message_file.is_empty() && !message_file.contains(' '),
        "commit-msg takes exactly one positional argument, the message file path: {args:?}"
    );
    // The hook observed message_file relative to its own cwd (the worktree root), not ours.
    let message = std::fs::read_to_string(repo_path.join(message_file)).unwrap();
    assert!(message.starts_with("a message"));
}

#[test]
fn receive_side_hooks_run_in_git_dir_with_the_documented_stdin_and_argv() {
    if !git_available() {
        eprintln!("skipping: git not found on PATH");
        return;
    }
    let remote = tempfile::tempdir().unwrap();
    let remote_path = remote.path().canonicalize().unwrap();
    git(&remote_path, &["init", "--quiet", "--bare"]);

    let hooks_dir = remote_path.join("hooks");
    let pre_receive_capture = remote_path.join("pre-receive.capture");
    install_capture_hook(&hooks_dir, "pre-receive", &pre_receive_capture, "GIT_PUSH_OPTION_COUNT");
    let update_capture = remote_path.join("update.capture");
    install_capture_hook(&hooks_dir, "update", &update_capture, "GIT_DIR");

    let local = tempfile::tempdir().unwrap();
    let local_path = local.path().canonicalize().unwrap();
    init_repo(&local_path);
    std::fs::write(local_path.join("file.txt"), b"hello\n").unwrap();
    git(&local_path, &["add", "file.txt"]);
    git(&local_path, &["commit", "-m", "a message"]);
    git(&local_path, &["remote", "add", "origin", remote_path.to_str().unwrap()]);
    git(&local_path, &["push", "origin", "HEAD:refs/heads/main"]);

    let pre_receive_output = std::fs::read_to_string(&pre_receive_capture).unwrap();
    assert_eq!(
        field(&pre_receive_output, "CWD"),
        remote_path.to_str().unwrap(),
        "pre-receive must always run in $GIT_DIR per githooks(5), matching gix_hook::cwd_for"
    );
    let stdin_line = pre_receive_output
        .lines()
        .skip_while(|l| *l != "STDIN_START")
        .nth(1)
        .expect("one stdin line for the pushed ref");
    let fields: Vec<_> = stdin_line.split(' ').collect();
    assert_eq!(
        fields.len(),
        3,
        "receive_stdin()'s format is <old-oid> SP <new-oid> SP <ref-name>: {stdin_line:?}"
    );
    assert_eq!(
        fields[0],
        "0".repeat(40),
        "old-oid is all-zeroes for a newly created ref"
    );
    assert_eq!(fields[2], "refs/heads/main");
    let parsed = gix_hook::parse_receive_stdin(stdin_line.as_bytes()).expect("must parse with our own reader");
    assert_eq!(parsed[0].ref_name, "refs/heads/main");

    let update_output = std::fs::read_to_string(&update_capture).unwrap();
    let update_args: Vec<_> = field(&update_output, "ARGS").split(' ').collect();
    assert_eq!(
        update_args.len(),
        3,
        "update's argv is <ref-name> <old-oid> <new-oid>: {update_args:?}"
    );
    assert_eq!(update_args[0], "refs/heads/main");
    assert_eq!(update_args[1], "0".repeat(40));
}
