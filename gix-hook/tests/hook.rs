use bstr::{BString, ByteSlice};
use gix_hook::{Context, Options};
use gix_testtools::TestResult;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

fn fixture() -> TestResult<PathBuf> {
    Ok(gix_testtools::scripted_fixture_read_only("make_hooks.sh")?.canonicalize()?)
}

fn context<'a>(workdir: &'a Path, common_dir: &'a Path) -> Context<'a> {
    Context {
        working_dir: workdir,
        common_dir,
        command: gix_command::Context {
            git_dir: Some(common_dir.into()),
            worktree_dir: Some(workdir.into()),
            ..Default::default()
        },
    }
}

fn config(input: &[u8]) -> gix_error::Result<gix_config::File> {
    gix_config::File::from_bytes_no_includes(input, gix_config::file::Metadata::default(), Default::default())
}

fn configured(input: &[u8], event: &str) -> gix_error::Result<Option<Vec<(BString, BString)>>> {
    gix_hook::configured_commands(&config(input)?, |_| true, event)
}

#[test]
fn discovery_validates_names_and_execute_permissions() -> TestResult {
    let fixture = fixture()?;
    let directory = fixture.join("repo/.git/hooks");
    assert_eq!(
        gix_hook::find("pre-commit", &directory)?,
        Some(directory.join("pre-commit"))
    );
    assert!(gix_hook::find("missing", &directory)?.is_none());
    assert!(gix_hook::find("directory", &directory)?.is_none());
    for name in [
        "",
        ".",
        "..",
        "../pre-commit",
        "hooks/pre-commit",
        "hooks\\pre-commit",
        "C:hook",
        "pre\0commit",
    ] {
        assert!(
            gix_hook::find(name, &directory)
                .expect_err("invalid hook name")
                .is_validation()
        );
    }
    #[cfg(unix)]
    assert!(gix_hook::find("post-commit", &directory)?.is_none());
    assert_eq!(
        gix_hook::find("applypatch-msg", &directory)?.is_some(),
        fixture.join("mode-hook-executable").exists(),
        "execute permission agrees with Git"
    );
    assert!(gix_hook::is_known_event(b"pre-commit".as_bstr()));
    assert!(!gix_hook::is_known_event(b"custom-\xff".as_bstr()));
    for event in [
        "pre-receive",
        "update",
        "post-receive",
        "post-update",
        "push-to-checkout",
        "proc-receive",
    ] {
        assert!(
            gix_hook::is_receive_event(event),
            "receive event {event} runs in the Git directory"
        );
    }
    assert!(!gix_hook::is_receive_event("pre-push"));
    assert!(!gix_hook::is_receive_event("post-checkout"));
    Ok(())
}

#[test]
fn preparation_and_execution_preserve_literal_values() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = fixture()?;
    let workdir = fixture.join("repo");
    let gitdir = workdir.join(".git");
    let context = context(&workdir, &gitdir);
    let options = Options {
        arguments: &[OsStr::new("one argument"), OsStr::new("$(touch unexpected)")],
        environment: &[(OsStr::new("HOOK_VALUE"), OsStr::new("literal $value"))],
        stdin: Some(b"hook input\n"),
    };
    let command = gix_hook::prepare(Path::new(".git/hooks/pre-commit"), &context, options)?;
    assert_eq!(command.get_current_dir(), Some(workdir.as_path()));
    let output = gix_hook::run(Path::new(".git/hooks/pre-commit"), &context, options)?;
    assert!(output.status.success());
    assert_eq!(
        output.stdout,
        b"literal $value\none argument\n$(touch unexpected)\nhook input\n"
    );
    assert_eq!(output.stderr, b"hook-stderr\n");
    let expected = gix_testtools::git_command(&workdir)
        .env("HOOK_VALUE", "literal $value")
        .args(["hook", "run", "--to-stdin"])
        .arg(fixture.join("input"))
        .args(["pre-commit", "--", "one argument", "$(touch unexpected)"])
        .output()?;
    assert!(expected.status.success());
    assert!(
        expected
            .stderr
            .contains_str(&output.stdout[..output.stdout.len() - b"hook input\n".len()])
    );
    assert!(expected.stderr.contains_str(b"hook input\n"));
    assert!(
        !workdir.join("unexpected").exists(),
        "argument syntax is never evaluated"
    );
    Ok(())
}

#[test]
fn environment_overrides_win_and_bare_context_removes_inherited_worktree() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = fixture()?;
    let workdir = fixture.join("repo");
    let gitdir = workdir.join(".git");
    let mut context = context(&workdir, &gitdir);
    let _environment = gix_testtools::Env::new()
        .set("GIT_WORK_TREE", "inherited-wrong")
        .set("GIT_COMMON_DIR", "inherited-wrong");
    context.command.worktree_dir = None;
    let output = gix_hook::run(Path::new(".git/hooks/pre-auto-gc"), &context, Options::default())?;
    let lines: Vec<_> = output.stdout.lines().collect();
    assert_eq!(lines[0], gix_path::into_bstr(&gitdir)?.as_ref());
    assert_eq!(lines[1], gix_path::into_bstr(&gitdir)?.as_ref());
    assert_eq!(lines[2], b"unset");
    let output = gix_hook::run(
        Path::new(".git/hooks/pre-auto-gc"),
        &context,
        Options {
            environment: &[
                (OsStr::new("GIT_DIR"), OsStr::new("explicit-dir")),
                (OsStr::new("GIT_COMMON_DIR"), OsStr::new("explicit-common")),
                (OsStr::new("GIT_WORK_TREE"), OsStr::new("explicit-worktree")),
            ],
            ..Default::default()
        },
    )?;
    assert_eq!(output.stdout, b"explicit-dir\nexplicit-common\nexplicit-worktree\n");
    assert!(
        gix_hook::prepare(
            Path::new("hook"),
            &Context {
                working_dir: Path::new("relative"),
                ..context
            },
            Options::default()
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn shell_fallback_is_limited_to_executable_format_errors() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = fixture()?;
    let workdir = fixture.join("repo");
    let gitdir = workdir.join(".git");
    let context = context(&workdir, &gitdir);
    let options = Options {
        arguments: &[OsStr::new("one argument"), OsStr::new("$(touch unexpected)")],
        environment: &[(OsStr::new("HOOK_VALUE"), OsStr::new("literal $value"))],
        ..Default::default()
    };
    let result = gix_hook::run(Path::new("hook $@ space/pre-rebase"), &context, options);
    #[cfg(unix)]
    {
        let output = result?;
        assert!(output.status.success());
        assert_eq!(
            [output.stdout, output.stderr].concat(),
            std::fs::read(fixture.join("no-shebang-output"))?
        );
        use std::os::unix::ffi::OsStrExt;
        let options = Options {
            arguments: &[OsStr::from_bytes(b"non-utf8-\xff")],
            stdin: Some(b"input\n"),
            ..options
        };
        let output = gix_hook::run(Path::new("hook $@ space/pre-rebase"), &context, options)?;
        assert_eq!(output.stdout, b"literal $value\nnon-utf8-\xff\ninput\n");
        assert!(gix_hook::run(Path::new(".git/hooks/post-commit"), &context, options).is_err());
    }
    #[cfg(windows)]
    assert!(result.is_err(), "Windows requires a shebang");
    for path in ["missing", ".git/hooks/pre-receive"] {
        assert!(
            gix_hook::run(Path::new(path), &context, options).is_err(),
            "{path} is not retried through the shell"
        );
    }
    let output = gix_hook::run(
        &std::env::current_exe()?,
        &context,
        Options {
            arguments: &[OsStr::new("--list")],
            ..Default::default()
        },
    )?;
    assert!(output.status.success());
    assert!(
        output
            .stdout
            .contains_str("shell_fallback_is_limited_to_executable_format_errors"),
        "native executable stays native"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn input_and_output_do_not_deadlock_and_broken_pipe_keeps_status() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = fixture()?;
    let workdir = fixture.join("repo");
    let gitdir = workdir.join(".git");
    let context = context(&workdir, &gitdir);
    let input = vec![b'x'; 1024 * 1024];
    let options = Options {
        stdin: Some(&input),
        ..Default::default()
    };
    let output = gix_hook::run(Path::new(".git/hooks/pre-merge-commit"), &context, options)?;
    assert!(output.status.success());
    assert_eq!(output.stdout.len(), 2 * input.len());
    assert_eq!(output.stderr, vec![0; input.len()]);
    assert_eq!(&output.stdout[input.len()..], &input);
    let output = gix_hook::run(Path::new(".git/hooks/pre-applypatch"), &context, options)?;
    assert_eq!(output.status.code(), Some(23));
    assert_eq!(output.stdout, b"early stdout\n");
    assert_eq!(output.stderr, b"early stderr\n");
    Ok(())
}

#[test]
fn config_resolves_last_registration_order_resets_and_disabled_names() -> TestResult {
    let commands = configured(
        br#"
        [hook "first"]
            command = obsolete
            event = post-checkout
        [hook "second"]
            command = second
            event = post-checkout
            parallel = true
        [hook "cleared"]
            event = another
            event = post-checkout
            event =
        [hook "first"]
            command = first
            event = post-checkout
            enabled = false
            enabled = true
        [hook "disabled"]
            event = post-checkout
            enabled = false
    "#,
        "post-checkout",
    )?
    .expect("event enabled");
    assert_eq!(
        commands,
        vec![("second".into(), "second".into()), ("first".into(), "first".into())]
    );
    assert!(
        configured(
            br#"[hook "post-checkout"]
        enabled = false
        event =
    "#,
            "post-checkout"
        )?
        .is_none()
    );
    assert_eq!(
        configured(
            br#"[hook "custom"]
        enabled = false
        parallel = false
        [hook "other"]
        command = true
        event = custom
    "#,
            "custom"
        )?
        .expect("custom is a hook name, not an event disable")
        .len(),
        1
    );
    let bytes = b"[hook \"raw-\xff\"]\ncommand = bytes\nevent = post-checkout\n";
    assert_eq!(
        configured(bytes, "post-checkout")?.expect("event enabled")[0].0,
        b"raw-\xff".as_bstr()
    );
    Ok(())
}

#[test]
fn config_validation_precedes_execution_and_filters_sections() -> TestResult {
    for invalid in [
        "[hook \"missing\"]\nevent=other\n",
        "[hook \"post-checkout\"]\ncommand=true\n",
        "[hook \"post-checkout\"]\nparallel=false\n",
        "[hook \"implicit\"]\nevent\n",
        "[hook \"implicit\"]\ncommand\n",
        "[hook \"invalid\"]\nenabled=maybe\n",
    ] {
        let config = config(invalid.as_bytes())?;
        assert!(gix_hook::configured_commands(&config, |_| true, "post-checkout").is_err());
        assert_eq!(
            gix_hook::configured_commands(&config, |_| false, "post-checkout")?,
            Some(Vec::new()),
            "filtered invalid config is not interpreted"
        );
    }
    Ok(())
}

#[test]
fn serial_events_continue_after_exit_failure_and_replay_input() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = fixture()?;
    let workdir = fixture.join("repo");
    let gitdir = workdir.join(".git");
    let context = context(&workdir, &gitdir);
    let hooks_dir = gitdir.join("hooks");
    let commands = configured(
        br#"
        [hook "first"]
            command = "printf 'first\\n'; exit 7"
            event = post-checkout
            parallel = true
        [hook "second"]
            command = "printf 'second\\n'"
            event = post-checkout
            parallel = true
        [hook]
            jobs = 8
    "#,
        "post-checkout",
    )?;
    let outputs = gix_hook::run_event_serially(
        "post-checkout",
        commands.as_deref(),
        &hooks_dir,
        &context,
        Options::default(),
    )?;
    assert_eq!(
        outputs.iter().map(|output| output.status.code()).collect::<Vec<_>>(),
        [Some(7), Some(0), Some(0)]
    );
    let output = outputs
        .iter()
        .flat_map(|output| output.stdout.iter())
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(output, b"first\nsecond\ntraditional\n");
    if *gix_testtools::GIT_VERSION >= (2, 55, 0) {
        let expected = gix_testtools::git_command(&workdir)
            .args([
                "-c",
                "hook.first.command=printf 'first\\n'; exit 7",
                "-c",
                "hook.first.event=post-checkout",
                "-c",
                "hook.second.command=printf 'second\\n'",
                "-c",
                "hook.second.event=post-checkout",
                "hook",
                "run",
                "-j",
                "1",
                "post-checkout",
            ])
            .output()?;
        assert!(!expected.status.success());
        assert_eq!(expected.stderr, output, "serial ordering agrees with Git");
    }
    assert!(
        gix_hook::run_event_serially("post-checkout", None, &hooks_dir, &context, Options::default())?.is_empty(),
        "event disable includes traditional hooks"
    );
    let commands = vec![
        ("one".into(), "./.git/hooks/pre-commit".into()),
        ("two".into(), "./.git/hooks/pre-commit".into()),
    ];
    let options = Options {
        arguments: &[OsStr::new("literal $arg")],
        environment: &[(OsStr::new("HOOK_VALUE"), OsStr::new("value"))],
        stdin: Some(b"input\n"),
    };
    let outputs = gix_hook::run_event_serially("post-merge", Some(&commands), &hooks_dir, &context, options)?;
    for output in outputs {
        assert_eq!(output.stdout, b"value\nliteral $arg\ninput\n");
    }
    Ok(())
}

#[test]
fn launch_failure_stops_later_hooks_but_shell_exit_does_not() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = fixture()?;
    let workdir = fixture.join("repo");
    let gitdir = workdir.join(".git");
    let context = context(&workdir, &gitdir);
    let hooks_dir = gitdir.join("hooks");
    let observed = gix_testtools::tempfile::tempdir()?;
    let marker = observed.path().join("later");
    let mut commands = vec![
        ("missing".into(), "./missing-hook".into()),
        ("later".into(), "printf later > \"$HOOK_MARKER\"".into()),
    ];
    let options = Options {
        environment: &[(OsStr::new("HOOK_MARKER"), marker.as_os_str())],
        ..Default::default()
    };
    assert!(gix_hook::run_event_serially("post-merge", Some(&commands), &hooks_dir, &context, options).is_err());
    assert!(!marker.exists());
    commands[0].1 = "./missing-hook argument".into();
    let output = gix_hook::run_event_serially("post-merge", Some(&commands), &hooks_dir, &context, options)?;
    assert_eq!(
        output[0].status.code(),
        Some(127),
        "shell itself launches, then reports the missing command"
    );
    assert_eq!(std::fs::read(marker)?, b"later");
    Ok(())
}

#[cfg(windows)]
#[test]
fn interpreter_lookup_uses_working_directory_and_explicit_path() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = fixture()?;
    let workdir = fixture.join("repo");
    let gitdir = workdir.join(".git");
    let context = context(&workdir, &gitdir);
    let _environment = gix_testtools::Env::new().set("PATH", "hook-interpreter");
    let command = gix_hook::prepare(Path::new(".git/hooks/pre-commit"), &context, Options::default())?;
    assert_eq!(command.get_program(), workdir.join("hook-interpreter/sh.exe"));
    let shell_dir = Path::new(gix_path::env::shell())
        .parent()
        .expect("absolute Windows shell path");
    let options = Options {
        environment: &[(OsStr::new("PATH"), shell_dir.as_os_str())],
        ..Default::default()
    };
    let output = gix_hook::run(Path::new(".git/hooks/pre-commit"), &context, options)?;
    assert!(
        output.status.success(),
        "explicit PATH reaches interpreter lookup before spawning"
    );
    Ok(())
}

#[test]
fn configured_commands_reject_invalid_event_names() -> TestResult {
    let config = config(b"")?;
    for event in ["", "..", "../post-checkout", "invalid:event", "post\0checkout"] {
        assert!(
            gix_hook::configured_commands(&config, |_| true, event)
                .expect_err("event names cannot escape the hook directory")
                .is_validation()
        );
    }
    Ok(())
}

#[test]
fn relative_hook_directories_start_at_the_execution_context() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = fixture()?;
    let workdir = fixture.join("repo");
    let gitdir = workdir.join(".git");
    let context = context(&workdir, &gitdir);
    let elsewhere = gix_testtools::tempfile::tempdir()?;
    let _directory = gix_testtools::set_current_dir(elsewhere.path())?;
    let outputs = gix_hook::run_event_serially(
        "post-checkout",
        Some(&[]),
        Path::new(".git/hooks"),
        &context,
        Options::default(),
    )?;
    assert_eq!(outputs.len(), 1, "discovery and execution use the same directory base");
    assert_eq!(outputs[0].stdout, b"traditional\n");
    Ok(())
}
