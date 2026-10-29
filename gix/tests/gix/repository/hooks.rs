use std::{ffi::OsStr, path::Path, process::Stdio};

use gix_testtools::TestResult;

fn fixture() -> gix_testtools::Result<std::path::PathBuf> {
    Ok(gix_testtools::scripted_fixture_read_only("make_hook_repos.sh")?.canonicalize()?)
}

#[test]
fn discovery_uses_shared_hooks_and_configured_worktree_relative_paths() -> TestResult {
    let fixture = fixture()?;
    for (name, expected) in [
        ("repo", "repo/.git/hooks/pre-commit"),
        ("linked", "repo/.git/hooks/pre-commit"),
        ("bare.git", "bare.git/hooks/pre-commit"),
    ] {
        let repo = gix::open_opts(fixture.join(name), crate::restricted())?;
        assert_eq!(
            repo.find_hook("pre-commit")?
                .expect("the fixture installs the hook")
                .canonicalize()?,
            fixture.join(expected).canonicalize()?,
            "default hooks belong to the common repository, including from linked worktrees"
        );
        assert_eq!(
            repo.hooks_directory()?.canonicalize()?,
            fixture
                .join(expected)
                .parent()
                .expect("hook has a directory")
                .canonicalize()?,
            "directory access uses the same common directory as executable hook discovery"
        );
        assert!(repo.find_hook("missing")?.is_none(), "absent hooks are not configured");
    }
    for name in ["repo", "linked"] {
        for hooks_path in [
            "custom-hooks".to_owned(),
            fixture.join(name).join("custom-hooks").to_string_lossy().into_owned(),
        ] {
            let repo = gix::open_opts(
                fixture.join(name),
                crate::restricted().config_overrides([format!("core.hooksPath={hooks_path}")]),
            )?;
            assert_eq!(
                repo.find_hook("pre-commit")?
                    .expect("the configured hook exists")
                    .canonicalize()?,
                fixture.join(name).join("custom-hooks/pre-commit").canonicalize()?,
                "relative core.hooksPath starts at the hook's working directory"
            );
        }
    }
    let repo = gix::open_opts(
        fixture.join("repo"),
        crate::restricted().config_overrides(["core.hooksPath="]),
    )?;
    assert_eq!(
        repo.find_hook("pre-commit")?,
        Some(fixture.join("repo/pre-commit")),
        "an explicit empty path selects the worktree root"
    );
    let repo = gix::open_opts(
        fixture.join("repo"),
        crate::restricted().config_overrides(["core.hooksPath=absent"]),
    )?;
    assert!(
        repo.find_hook("pre-commit")?.is_none(),
        "a configured path does not fall back to common hooks"
    );
    assert_eq!(
        repo.hooks_directory()?,
        fixture.join("repo/absent"),
        "directory access also supports inspecting or installing hooks that are not present"
    );
    Ok(())
}

#[test]
fn discovery_rejects_path_components_and_ignores_non_executable_hooks() -> TestResult {
    let repo = gix::open_opts(fixture()?.join("repo"), crate::restricted())?;
    for name in [
        "",
        ".",
        "..",
        "../pre-commit",
        "hooks/pre-commit",
        "hooks\\pre-commit",
        "/pre-commit",
        "pre\0commit",
    ] {
        assert!(
            repo.find_hook(name)
                .expect_err("hook names cannot escape the hook directory")
                .is_validation(),
            "invalid names are classified as input errors"
        );
    }
    assert!(repo.find_hook("directory")?.is_none(), "directories cannot be hooks");
    #[cfg(unix)]
    assert!(
        repo.find_hook("post-commit")?.is_none(),
        "hooks without execute permission are ignored"
    );
    Ok(())
}

#[test]
fn hook_commands_preserve_arguments_and_match_git() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = fixture()?;
    for (name, expected) in [
        ("repo", "expected-output"),
        ("linked", "expected-linked-output"),
        ("bare.git", "expected-bare-output"),
    ] {
        let repo = gix::open_opts(fixture.join(name), crate::restricted())?;
        let hook = repo.find_hook("pre-commit")?.expect("the fixture installs the hook");
        let mut command = repo.prepare_hook(&hook)?;
        command.args(["one argument", "two"]).stderr(Stdio::piped());
        assert_eq!(
            command.get_current_dir(),
            Some(
                repo.current_dir()
                    .join(repo.workdir().unwrap_or(repo.git_dir()))
                    .as_path()
            ),
            "hooks run in the checkout or bare repository"
        );
        let output = command.output()?;
        assert!(
            output.status.success(),
            "the prepared hook runs successfully: {output:?}"
        );
        assert_eq!(
            output.stdout,
            std::fs::read(fixture.join(expected))?,
            "arguments are passed literally, as with git hook run"
        );
    }
    Ok(())
}

#[test]
fn preparation_anchors_paths_and_repository_context() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = fixture()?;
    let elsewhere = gix_testtools::tempfile::tempdir()?;
    let _cwd = gix_testtools::set_current_dir(fixture.join("repo"))?;
    let repo = gix::open_opts(".", crate::restricted())?;
    let _moved = gix_testtools::set_current_dir(elsewhere.path())?;
    let command = repo.prepare_hook(Path::new("custom-hooks/pre-commit"))?;
    assert_eq!(
        command.get_current_dir(),
        Some(fixture.join("repo").as_path()),
        "preparation uses the repository's captured current directory"
    );
    let context = repo.command_context()?;
    for (key, expected) in [("GIT_DIR", context.git_dir), ("GIT_WORK_TREE", context.worktree_dir)] {
        assert_eq!(
            command
                .get_envs()
                .find(|(name, _)| *name == OsStr::new(key))
                .and_then(|(_, value)| value),
            expected.as_deref().map(Path::as_os_str),
            "hook environment keeps {key} attached to this repository"
        );
    }
    #[cfg(unix)]
    assert_eq!(
        command.get_program(),
        fixture.join("repo/custom-hooks/pre-commit"),
        "ordinary executables are prepared directly without forcing a shell"
    );
    Ok(())
}

#[test]
fn reduced_trust_does_not_offer_repository_hooks_for_execution() -> TestResult {
    let repo = gix::open_opts(
        fixture()?.join("repo"),
        crate::restricted().with(gix::sec::Trust::Reduced),
    )?;
    assert!(
        repo.find_hook("pre-commit")
            .expect_err("repository hooks require full trust")
            .is_permission_denied(),
        "untrusted hook discovery is rejected"
    );
    assert!(
        repo.prepare_hook("pre-commit")
            .expect_err("repository hook commands require full trust")
            .is_permission_denied(),
        "preparation cannot bypass repository trust"
    );
    assert!(
        repo.run_hook("pre-commit", Default::default())
            .expect_err("hook execution requires full trust")
            .is_permission_denied(),
        "execution cannot bypass repository trust"
    );
    Ok(())
}

#[test]
fn configured_paths_keep_empty_optional_and_implicit_values_distinct() -> TestResult {
    let fixture = fixture()?;
    let workdir = fixture.join("repo");
    for (value, expected) in [
        (":(optional)missing", ".git/hooks"),
        (":(optional)", ".git/hooks"),
        (":(optional).git/hooks", ".git/hooks"),
    ] {
        let overrides = [
            "core.hooksPath=custom-hooks".to_owned(),
            format!("core.hooksPath={value}"),
        ];
        let repo = gix::open_opts(&workdir, crate::restricted().config_overrides(overrides.clone()))?;
        assert_eq!(
            repo.find_hook("pre-commit")?,
            Some(workdir.join(expected).join("pre-commit")),
            "missing optional paths select the default hook directory"
        );
        if *gix_testtools::GIT_VERSION >= (2, 53, 0) {
            let output = gix_testtools::git_command(&workdir)
                .args([
                    "-c",
                    &overrides[0],
                    "-c",
                    &overrides[1],
                    "rev-parse",
                    "--path-format=absolute",
                    "--git-path",
                    "hooks/pre-commit",
                ])
                .output()?;
            assert!(output.status.success(), "Git accepts optional hook paths");
            assert_eq!(
                Path::new(std::str::from_utf8(&output.stdout)?.trim()).canonicalize()?,
                repo.find_hook("pre-commit")?.expect("hook exists").canonicalize()?,
                "Git and gitoxide select the same hook after optional-path filtering"
            );
        }
    }
    let repo = gix::open_opts(&workdir, crate::restricted().config_overrides(["core.hooksPath"]))?;
    assert!(
        repo.find_hook("pre-commit")
            .expect_err("an implicit path is not an explicit empty path")
            .is_validation(),
        "hooksPath needs a value"
    );
    let output = gix_testtools::git_command(&workdir)
        .args(["-c", "core.hooksPath", "hook", "run", "pre-commit"])
        .output()?;
    assert!(!output.status.success(), "Git rejects an implicit hook path as well");
    Ok(())
}

#[cfg(unix)]
#[test]
fn hook_paths_retain_non_utf8_bytes_and_native_executables() -> TestResult {
    use std::os::unix::ffi::OsStrExt;
    let fixture = fixture()?;
    let repo = gix::open_opts(fixture.join("repo"), crate::restricted())?;
    let path = Path::new(OsStr::from_bytes(b"custom-\xff/pre-commit"));
    assert_eq!(
        repo.prepare_hook(path)?.get_program(),
        fixture.join("repo").join(path),
        "preparation retains non-UTF-8 path bytes even on filesystems that cannot store them"
    );
    let native = std::env::current_exe()?;
    let command = repo.prepare_hook(&native)?;
    assert_eq!(command.get_program(), native, "native binaries are executed directly");
    assert!(
        command.get_args().next().is_none(),
        "preparation does not add shell or interpreter arguments to native binaries"
    );
    Ok(())
}

#[test]
fn inherited_repository_paths_cannot_redirect_prepared_hooks() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = fixture()?;
    let _environment = gix_testtools::Env::new()
        .set(
            "GIT_COMMON_DIR",
            fixture.join("bare.git").to_str().expect("fixture path is UTF-8"),
        )
        .set(
            "GIT_WORK_TREE",
            fixture.join("linked").to_str().expect("fixture path is UTF-8"),
        );
    for name in ["repo", "bare.git"] {
        let repo = gix::open_opts(fixture.join(name), crate::restricted())?;
        let hook = repo
            .find_hook("pre-auto-gc")?
            .expect("the fixture installs the environment hook");
        let output = repo.prepare_hook(&hook)?.output()?;
        assert!(output.status.success(), "the environment hook completes");
        let expected = format!(
            "{}\n{}\n",
            repo.current_dir().join(repo.common_dir()).display(),
            repo.workdir().map_or_else(
                || "unset".into(),
                |path| repo.current_dir().join(path).display().to_string()
            )
        );
        assert_eq!(
            output.stdout,
            expected.as_bytes(),
            "the hook belongs to its repository, even if inherited Git paths name another repository"
        );
        let output = repo.run_hook(
            &hook,
            gix::hook::Options {
                environment: &[
                    (OsStr::new("GIT_COMMON_DIR"), OsStr::new("literal common directory")),
                    (OsStr::new("GIT_WORK_TREE"), OsStr::new("literal worktree directory")),
                ],
                ..Default::default()
            },
        )?;
        assert_eq!(
            output.stdout, b"literal common directory\nliteral worktree directory\n",
            "explicit hook options still override repository context, including bare worktree removal"
        );
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn discovery_checks_the_callers_execute_permission() -> TestResult {
    let fixture = fixture()?;
    let repo = gix::open_opts(fixture.join("repo"), crate::restricted())?;
    assert_eq!(
        repo.find_hook("applypatch-msg")?.is_some(),
        fixture.join("mode-hook-executable").exists(),
        "Git and gitoxide honor the caller's permissions, not merely any executable bit"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn execution_falls_back_for_scripts_and_preserves_native_arguments() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = fixture()?;
    let repo = gix::open_opts(fixture.join("repo"), crate::restricted())?;
    assert!(
        fixture.join("no-shebang-supported").exists(),
        "Git supports shell fallback on Unix"
    );
    let environment = [(OsStr::new("HOOK_VALUE"), OsStr::new("literal $value"))];
    let output = repo.run_hook(
        "hook $@ space/pre-rebase",
        gix::hook::Options {
            arguments: &[OsStr::new("one argument"), OsStr::new("$(touch unexpected)")],
            environment: &environment,
            ..Default::default()
        },
    )?;
    assert!(
        output.status.success(),
        "an executable no-shebang script runs through the shell"
    );
    assert_eq!(
        output.stdout, b"literal $value\none argument\n$(touch unexpected)\n",
        "shell metacharacters remain literal"
    );
    assert_eq!(output.stderr, b"hook-stderr\n", "stderr is captured separately");
    assert_eq!(
        [output.stdout, output.stderr].concat(),
        std::fs::read(fixture.join("no-shebang-output"))?,
        "Git uses the same shell fallback and arguments"
    );
    let output = repo.run_hook(
        "hook $@ space/pre-rebase",
        gix::hook::Options {
            environment: &environment,
            ..Default::default()
        },
    )?;
    assert_eq!(
        output.stdout, b"literal $value\n",
        "a literal hook path also works without arguments"
    );
    assert!(output.status.success(), "missing input supplies EOF");

    {
        use std::os::unix::ffi::OsStrExt;
        let output = repo.run_hook(
            "hook $@ space/pre-rebase",
            gix::hook::Options {
                arguments: &[OsStr::from_bytes(b"non-utf8-\xff")],
                environment: &environment,
                stdin: Some(b"hook input\n"),
            },
        )?;
        assert!(output.status.success(), "the fallback retains piped input");
        assert_eq!(
            output.stdout, b"literal $value\nnon-utf8-\xff\nhook input\n",
            "fallback input and native argument bytes are retained"
        );
    }

    Ok(())
}

#[cfg(windows)]
#[test]
fn execution_rejects_scripts_without_a_shebang_on_windows() -> TestResult {
    let fixture = fixture()?;
    let repo = gix::open_opts(fixture.join("repo"), crate::restricted())?;
    assert!(
        !fixture.join("no-shebang-supported").exists(),
        "Git requires a shebang on Windows"
    );
    assert!(
        repo.run_hook("hook $@ space/pre-rebase", Default::default()).is_err(),
        "gitoxide also rejects scripts without a shebang on Windows"
    );
    Ok(())
}

#[test]
fn execution_preserves_native_executables() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let repo = gix::open_opts(fixture()?.join("repo"), crate::restricted())?;
    let output = repo.run_hook(
        std::env::current_exe()?,
        gix::hook::Options {
            arguments: &[OsStr::new("--list")],
            ..Default::default()
        },
    )?;
    assert!(
        output.status.success(),
        "native executables are not interpreted as shell scripts"
    );
    assert!(
        String::from_utf8(output.stdout)?.contains("repository::hooks::execution_preserves_native_executables"),
        "the native executable received its arguments"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn execution_drains_output_while_feeding_input() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let repo = gix::open_opts(fixture()?.join("repo"), crate::restricted())?;
    let input = vec![b'x'; 1024 * 1024];
    let output = repo.run_hook(
        ".git/hooks/pre-merge-commit",
        gix::hook::Options {
            stdin: Some(&input),
            ..Default::default()
        },
    )?;
    assert!(
        output.status.success(),
        "large simultaneous input and output complete without blocking"
    );
    assert_eq!(output.stdout.len(), 2 * input.len(), "all stdout is captured");
    assert_eq!(output.stderr, vec![0; input.len()], "all stderr is captured");
    assert_eq!(
        &output.stdout[..input.len()],
        vec![0; input.len()],
        "the hook writes before reading"
    );
    assert_eq!(&output.stdout[input.len()..], &input, "all input reaches the hook");
    Ok(())
}

#[test]
fn execution_retains_hook_failures_and_ignores_early_stdin_close() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let repo = gix::open_opts(fixture()?.join("repo"), crate::restricted())?;
    let input = vec![b'x'; 1024 * 1024];
    let output = repo.run_hook(
        ".git/hooks/pre-applypatch",
        gix::hook::Options {
            stdin: Some(&input),
            ..Default::default()
        },
    )?;
    assert_eq!(
        output.status.code(),
        Some(23),
        "a broken input pipe does not hide the hook's status"
    );
    assert_eq!(output.stdout, b"early stdout\n", "failed hooks retain stdout");
    assert_eq!(output.stderr, b"early stderr\n", "failed hooks retain stderr");
    assert!(
        repo.run_hook("missing", Default::default()).is_err(),
        "missing executables are not retried as shell scripts"
    );
    assert!(
        repo.run_hook(".git/hooks/pre-receive", Default::default()).is_err(),
        "a missing interpreter is not bypassed with a shell"
    );
    #[cfg(unix)]
    assert!(
        repo.run_hook(".git/hooks/post-commit", Default::default()).is_err(),
        "permission failures are not bypassed with a shell"
    );
    Ok(())
}

#[cfg(windows)]
#[test]
fn interpreter_lookup_uses_the_hook_directory_and_environment() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = fixture()?;
    let repo = gix::open_opts(fixture.join("repo"), crate::restricted())?;
    let shell_directory = Path::new(gix_path::env::shell())
        .parent()
        .expect("Git for Windows provides an absolute shell path");
    let _environment = gix_testtools::Env::new().set("PATH", "hook-interpreter");
    let command = repo.prepare_hook(".git/hooks/pre-commit")?;
    assert_eq!(
        command.get_program(),
        fixture.join("repo/hook-interpreter/sh.exe"),
        "relative interpreter PATH entries start at the hook's working directory"
    );
    let output = repo.run_hook(
        ".git/hooks/pre-commit",
        gix::hook::Options {
            arguments: &[OsStr::new("one argument"), OsStr::new("two")],
            environment: &[(OsStr::new("PATH"), shell_directory.as_os_str())],
            ..Default::default()
        },
    )?;
    assert!(
        output.status.success(),
        "execution resolves the real interpreter from options"
    );
    assert_eq!(
        output.stdout,
        std::fs::read(fixture.join("expected-output"))?,
        "the interpreter selected before launch receives the literal arguments"
    );
    Ok(())
}

fn event_fixture() -> gix_testtools::Result<std::path::PathBuf> {
    Ok(gix_testtools::scripted_fixture_read_only("make_configured_hook_repos.sh")?.canonicalize()?)
}

#[test]
fn configured_events_preserve_last_registration_order_and_run_after_failures() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = event_fixture()?;
    let overrides = [
        "hook.first.command=printf obsolete",
        "hook.first.event=post-checkout",
        "hook.second.command=printf 'second\\n'; exit 7",
        "hook.second.event=post-checkout",
        "hook.cleared.event=post-checkout",
        "hook.cleared.event=another-event",
        "hook.cleared.event=",
        "hook.first.command=printf 'first\\n'",
        "hook.first.event=post-checkout",
        "hook.disabled.event=post-checkout",
        "hook.disabled.enabled=false",
        "hook.first.enabled=false",
        "hook.first.enabled=true",
        "hook.first.parallel=true",
        "hook.second.parallel=true",
        "hook.jobs=8",
    ];
    let workdir = fixture.join("repo");
    let repo = gix::open_opts(&workdir, crate::restricted().config_overrides(overrides))?;
    let outputs = repo.run_hook_event_serially("post-checkout", Default::default())?;
    assert_eq!(
        outputs.iter().map(|output| output.status.code()).collect::<Vec<_>>(),
        [Some(7), Some(0), Some(0)],
        "a failed configured hook does not stop later configured or traditional hooks"
    );
    let stdout = outputs
        .iter()
        .flat_map(|output| &output.stdout)
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(
        stdout, b"second\nfirst\ntraditional\n",
        "last event registration orders hooks, last commands win, and empty events clear registrations"
    );
    if *gix_testtools::GIT_VERSION >= (2, 55, 0) {
        let mut git = gix_testtools::git_command(&workdir);
        for config in overrides {
            git.args(["-c", config]);
        }
        let expected = git.args(["hook", "run", "-j", "1", "post-checkout"]).output()?;
        assert!(!expected.status.success(), "Git also reports the failed hook");
        // Git warns about a disabled hook without a command; omit that harmless diagnostic.
        let expected: Vec<_> = expected
            .stderr
            .split_inclusive(|byte| *byte == b'\n')
            .filter(|line| !line.starts_with(b"warning:"))
            .flatten()
            .copied()
            .collect();
        assert_eq!(stdout, expected, "serial output ordering matches Git");
    }
    Ok(())
}

#[test]
fn configured_events_replay_native_arguments_environment_and_input() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = event_fixture()?;
    let script = "f () { printf '%s\\n' \"$HOOK_VALUE\" \"$@\"; cat; }; f";
    let overrides = [
        format!("hook.first.command={script}"),
        "hook.first.event=post-merge".into(),
        "hook.second.command=./configured-hook".into(),
        "hook.second.event=post-merge".into(),
    ];
    let repo = gix::open_opts(
        fixture.join("repo"),
        crate::restricted().config_overrides(overrides.clone()),
    )?;
    let outputs = repo.run_hook_event_serially(
        "post-merge",
        gix::hook::Options {
            arguments: &[OsStr::new("one argument"), OsStr::new("$(touch unexpected)")],
            environment: &[(OsStr::new("HOOK_VALUE"), OsStr::new("literal $value"))],
            stdin: Some(b"hook input\n"),
        },
    )?;
    assert_eq!(outputs.len(), 2, "both configured hooks run without a traditional hook");
    let expected = b"literal $value\none argument\n$(touch unexpected)\nhook input\n";
    for output in outputs {
        assert!(output.status.success(), "each hook consumes its own input");
        assert_eq!(
            output.stdout, expected,
            "each hook receives the same literal arguments, environment and full stdin"
        );
    }
    if *gix_testtools::GIT_VERSION >= (2, 55, 0) {
        let mut git = gix_testtools::git_command(fixture.join("repo"));
        for config in overrides {
            git.arg("-c").arg(config);
        }
        let reference = git
            .env("HOOK_VALUE", "literal $value")
            .args(["hook", "run", "-j", "1", "--to-stdin"])
            .arg(fixture.join("input"))
            .args(["post-merge", "--", "one argument", "$(touch unexpected)"])
            .output()?;
        assert!(reference.status.success(), "Git also replays stdin for each hook");
        assert_eq!(
            reference.stderr,
            expected.repeat(2),
            "literal values and input match Git"
        );
    }
    let repo = gix::open_opts(
        fixture.join("repo"),
        crate::restricted().config_overrides([
            "hook.inline.command=HOOK_VALUE=inline ./configured-hook",
            "hook.inline.event=post-merge",
        ]),
    )?;
    let output = repo.run_hook_event_serially(
        "post-merge",
        gix::hook::Options {
            environment: &[(OsStr::new("HOOK_VALUE"), OsStr::new("outer"))],
            ..Default::default()
        },
    )?;
    assert_eq!(
        output[0].stdout, b"inline\n",
        "inline shell assignments override the supplied environment"
    );
    Ok(())
}

#[test]
fn event_disables_cover_configured_and_traditional_hooks() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = event_fixture()?;
    for enabled in [false, true] {
        let repo = gix::open_opts(
            fixture.join("repo"),
            crate::restricted().config_overrides([
                "hook.named.command=printf 'configured\\n'".to_owned(),
                "hook.named.event=post-checkout".into(),
                "hook.post-checkout.event=".into(),
                "hook.post-checkout.enabled=false".into(),
                format!("hook.post-checkout.enabled={enabled}"),
            ]),
        )?;
        assert_eq!(
            repo.run_hook_event_serially("post-checkout", Default::default())?.len(),
            if enabled { 2 } else { 0 },
            "event-wide disables include the traditional hook, and can be overridden"
        );
    }
    let repo = gix::open_opts(
        fixture.join("repo"),
        crate::restricted().config_overrides([
            "hook.custom.enabled=false",
            "hook.custom.parallel=false",
            "hook.other.command=printf 'other\\n'",
            "hook.other.event=custom",
        ]),
    )?;
    assert_eq!(
        repo.run_hook_event_serially("custom", Default::default())?.len(),
        1,
        "a friendly name sharing an unknown event name only disables that individual hook"
    );
    assert!(
        repo.run_hook_event_serially("absent", Default::default())?.is_empty(),
        "no enabled hooks produces an empty result"
    );
    Ok(())
}

#[test]
fn event_configuration_is_validated_before_hooks_run_and_respects_trust() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = event_fixture()?;
    for overrides in [
        vec!["hook.missing.event=another-event"],
        vec![
            "hook.post-checkout.event=post-commit",
            "hook.post-checkout.command=true",
        ],
        vec!["hook.post-checkout.command=printf unused"],
        vec!["hook.post-checkout.parallel=false"],
        vec!["hook.implicit.event"],
        vec!["hook.implicit.command"],
        vec!["hook.invalid.enabled=maybe"],
    ] {
        let repo = gix::open_opts(fixture.join("repo"), crate::restricted().config_overrides(overrides))?;
        assert!(
            repo.run_hook_event_serially("post-checkout", Default::default())
                .is_err(),
            "invalid hook configuration is rejected before executing an event"
        );
    }
    let repo = gix::open_opts(
        fixture.join("repo"),
        crate::restricted()
            .config_overrides(["hook.untrusted.event=post-checkout"])
            .filter_config_section(|_| false),
    )?;
    assert_eq!(
        repo.run_hook_event_serially("post-checkout", Default::default())?.len(),
        1,
        "excluded configuration cannot introduce commands or invalid registrations"
    );
    let repo = gix::open_opts(fixture.join("repo"), crate::restricted().with(gix::sec::Trust::Reduced))?;
    assert!(
        repo.run_hook_event_serially("post-checkout", Default::default())
            .expect_err("events require full repository trust")
            .is_permission_denied(),
        "configured events cannot bypass repository trust"
    );
    Ok(())
}

#[test]
fn receive_events_use_gitdir_for_execution_and_relative_hook_paths() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = event_fixture()?;
    let repo = gix::open_opts(
        fixture.join("receiver"),
        crate::restricted().config_overrides([
            "hook.configured.command=test -f config && printf 'configured-gitdir\\n'",
            "hook.configured.event=pre-receive",
        ]),
    )?;
    let outputs = repo.run_hook_event_serially("pre-receive", Default::default())?;
    assert_eq!(outputs.len(), 2, "both kinds of hook use the receive context");
    assert_eq!(
        outputs[0].stdout, b"configured-gitdir\n",
        "configured commands run in gitdir"
    );
    assert_eq!(
        outputs[1].stdout, b"gitdir\n",
        "relative hooksPath also starts in gitdir"
    );
    assert!(
        std::fs::read(fixture.join("receive-output"))?
            .windows(b"remote: gitdir".len())
            .any(|part| part == b"remote: gitdir"),
        "Git's non-bare receive operation found and ran the same hook"
    );
    Ok(())
}

#[test]
fn configured_launch_failures_stop_later_hooks_like_git() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = event_fixture()?;
    let observed = gix_testtools::tempfile::tempdir()?;
    let marker = observed.path().join("later-hook-ran");
    let overrides = [
        "hook.missing.command=./missing-configured-hook",
        "hook.missing.event=post-checkout",
        "hook.later.command=printf later > \"$HOOK_MARKER\"",
        "hook.later.event=post-checkout",
    ];
    let workdir = fixture.join("repo");
    let repo = gix::open_opts(&workdir, crate::restricted().config_overrides(overrides))?;
    let error = repo
        .run_hook_event_serially(
            "post-checkout",
            gix::hook::Options {
                environment: &[(OsStr::new("HOOK_MARKER"), marker.as_os_str())],
                ..Default::default()
            },
        )
        .expect_err("a configured executable that cannot launch is an error");
    assert!(
        error.to_string().contains("missing"),
        "the error identifies the configured hook"
    );
    assert!(!marker.exists(), "launch failures stop later hooks");
    if *gix_testtools::GIT_VERSION >= (2, 55, 0) {
        let mut git = gix_testtools::git_command(&workdir);
        for config in overrides {
            git.args(["-c", config]);
        }
        let reference = git
            .env("HOOK_MARKER", &marker)
            .args(["hook", "run", "-j", "1", "post-checkout"])
            .output()?;
        assert!(
            !reference.status.success(),
            "Git also fails to launch the missing executable"
        );
        assert!(
            !marker.exists(),
            "Git also stops later configured hooks after launch failure"
        );
        assert!(
            !reference
                .stderr
                .windows(b"traditional".len())
                .any(|part| part == b"traditional"),
            "Git also skips the traditional hook"
        );
    }
    Ok(())
}

#[test]
fn absent_events_do_not_evaluate_command_only_configuration() -> TestResult {
    let repo = gix::open_opts(
        fixture()?.join("repo"),
        crate::restricted()
            .strict_config(true)
            .config_overrides(["gitoxide.core.externalCommandStderr=invalid"]),
    )?;
    assert!(
        repo.command_context().is_err(),
        "the configured command context is invalid"
    );
    assert!(
        repo.run_hook_event_serially("missing-event", Default::default())?
            .is_empty(),
        "an event without hooks does not construct a command context"
    );
    assert!(
        repo.run_hook_event_serially("pre-commit", Default::default()).is_err(),
        "an existing hook still requires valid command configuration"
    );
    Ok(())
}
