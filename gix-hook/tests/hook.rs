use gix_hook::{Context, Options};
use gix_testtools::TestResult;
use gix_testtools::bstr::ByteSlice;
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
    Ok(())
}

#[test]
fn preparation_keeps_resolved_context_and_literal_arguments() -> TestResult {
    let fixture = fixture()?;
    let workdir = fixture.join("repo");
    let common_dir = workdir.join(".git");
    let executable = std::env::current_exe()?;
    let options = Options {
        arguments: &[OsStr::new("one $literal argument")],
        environment: &[(OsStr::new("GIT_COMMON_DIR"), OsStr::new("explicit-common"))],
        ..Default::default()
    };
    let command = gix_hook::prepare(&executable, &context(&workdir, &common_dir), options)?;
    assert_eq!(
        command.get_program(),
        executable.as_os_str(),
        "native paths stay literal"
    );
    assert_eq!(
        command.get_current_dir(),
        Some(workdir.as_path()),
        "the caller supplies the repository directory"
    );
    assert_eq!(
        command.get_args().collect::<Vec<_>>(),
        options.arguments,
        "arguments are passed without shell interpretation"
    );
    assert!(
        command
            .get_envs()
            .any(|(name, value)| name == "GIT_COMMON_DIR" && value == Some(OsStr::new("explicit-common"))),
        "explicit environment wins over repository context"
    );
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
