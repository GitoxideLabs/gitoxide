use gix_credentials::{Program, helper, program::Kind};
use gix_testtools::TestResult;
use std::sync::LazyLock;

static SH: LazyLock<&'static str> = LazyLock::new(|| {
    gix_path::env::shell()
        .to_str()
        .expect("some `from_custom_definition` tests must be run where 'sh' path is valid Unicode")
});

#[test]
fn empty() -> TestResult {
    let prog = Program::from_custom_definition("")?;
    assert!(matches!(&prog.kind, Kind::ExternalName { name_and_args } if name_and_args.is_empty()));
    let cmd = prog.to_command(&helper::Action::Store("egal".into()))?;
    if cfg!(windows) {
        assert_eq!(
            std::path::Path::new(cmd.get_program()),
            std::path::Path::new(
                std::process::Command::try_from(gix_command::prepare("git-credential-"))?.get_program()
            ),
            "the helper uses platform command resolution"
        );
        assert_eq!(
            cmd.get_args().collect::<Vec<_>>(),
            ["store"],
            "not useful, but allowed, would have to be caught elsewhere"
        );
    } else {
        assert_eq!(
            cmd.get_args().collect::<Vec<_>>(),
            ["-c", "command git-credential- store", "command git-credential- store"],
            "the native shell supports executable helpers without shebangs"
        );
    }
    Ok(())
}

#[test]
fn simple_script_in_path() -> TestResult {
    let prog = Program::from_custom_definition("!exe")?;
    assert!(matches!(&prog.kind, Kind::ExternalShellScript(script) if script == "exe"));
    let cmd = prog.to_command(&helper::Action::Store("egal".into()))?;
    if cfg!(windows) {
        assert_eq!(
            cmd.get_program(),
            std::process::Command::try_from(gix_command::prepare("exe"))?.get_program(),
            "simple Windows commands can bypass the shell"
        );
        assert_eq!(
            cmd.get_args().collect::<Vec<_>>(),
            ["store"],
            "the helper receives its action"
        );
    } else {
        assert_eq!(
            cmd.get_args().collect::<Vec<_>>(),
            ["-c", "exe store", "exe store"],
            "the fixed action is part of the shell command"
        );
    }
    Ok(())
}

#[test]
fn name_with_args() -> TestResult {
    let input = "name --arg --bar=\"a b\"";
    let prog = Program::from_custom_definition(input)?;
    assert!(matches!(&prog.kind, Kind::ExternalName{name_and_args} if name_and_args == input));
    let cmd = prog.to_command(&helper::Action::Store("egal".into()))?;
    if cfg!(windows) {
        assert_eq!(
            std::path::Path::new(cmd.get_program()),
            std::path::Path::new(
                std::process::Command::try_from(gix_command::prepare("git-credential-name"))?.get_program()
            ),
            "the helper uses platform command resolution"
        );
        assert_eq!(
            cmd.get_args().collect::<Vec<_>>(),
            ["--arg", "--bar=a b", "store"],
            "quoted arguments are split without losing their contents"
        );
    } else {
        assert_eq!(
            cmd.get_args().collect::<Vec<_>>(),
            [
                "-c",
                "command git-credential-name --arg --bar=\"a b\" store",
                "command git-credential-name --arg --bar=\"a b\" store"
            ],
            "the native shell supports executable helpers without shebangs"
        );
    }
    Ok(())
}

#[test]
fn name_with_special_args() -> TestResult {
    let input = "name --arg --bar=~/folder/in/home";
    let prog = Program::from_custom_definition(input)?;
    assert!(matches!(&prog.kind, Kind::ExternalName{name_and_args} if name_and_args == input));
    let cmd = prog.to_command(&helper::Action::Store("egal".into()))?;
    let args = cmd.get_args().collect::<Vec<_>>();
    let script = args.iter().position(|arg| *arg == "-c").expect("shell invocation") + 1;
    assert_eq!(
        args[script],
        if cfg!(windows) {
            "git-credential-name --arg --bar=~/folder/in/home store"
        } else {
            "command git-credential-name --arg --bar=~/folder/in/home store"
        },
        "shell expansions and the appended action remain in the native helper command"
    );
    Ok(())
}

#[test]
fn name() -> TestResult {
    let input = "name";
    let prog = Program::from_custom_definition(input)?;
    assert!(matches!(&prog.kind, Kind::ExternalName{name_and_args} if name_and_args == input));
    let cmd = prog.to_command(&helper::Action::Store("egal".into()))?;
    if cfg!(windows) {
        assert_eq!(
            std::path::Path::new(cmd.get_program()),
            std::path::Path::new(
                std::process::Command::try_from(gix_command::prepare("git-credential-name"))?.get_program()
            ),
            "we detect that this can run without shell, which is also more portable on windows"
        );
        assert_eq!(
            cmd.get_args().collect::<Vec<_>>(),
            ["store"],
            "the helper receives its action directly"
        );
    } else {
        assert_eq!(
            cmd.get_args().collect::<Vec<_>>(),
            [
                "-c",
                "command git-credential-name store",
                "command git-credential-name store"
            ],
            "the native shell supports executable helpers without shebangs"
        );
    }
    Ok(())
}

#[test]
fn path_with_args_that_definitely_need_shell() -> TestResult {
    let input = "/abs/name --arg --bar=\"a b\"";
    let prog = Program::from_custom_definition(input)?;
    assert!(matches!(&prog.kind, Kind::ExternalPath{path_and_args} if path_and_args == input));
    let sh = *SH;
    assert_eq!(
        format!("{:?}", prog.to_command(&helper::Action::Store("egal".into()))?),
        if cfg!(windows) {
            r#""/abs/name" "--arg" "--bar=a b" "store""#.to_owned()
        } else {
            format!(r#""{sh}" "-c" "/abs/name --arg --bar=\"a b\" store" "/abs/name --arg --bar=\"a b\" store""#)
        }
    );
    Ok(())
}

#[test]
fn path_without_args() -> TestResult {
    let input = "/abs/name";
    let prog = Program::from_custom_definition(input)?;
    assert!(matches!(&prog.kind, Kind::ExternalPath{path_and_args} if path_and_args == input));
    assert_eq!(
        format!("{:?}", prog.to_command(&helper::Action::Store("egal".into()))?),
        if cfg!(windows) {
            r#""/abs/name" "store""#.to_owned()
        } else {
            format!(r#""{}" "-c" "/abs/name store" "/abs/name store""#, *SH)
        },
        "the action is part of the command text, and Windows can split this simple command"
    );
    Ok(())
}

#[test]
fn path_with_simple_args() -> TestResult {
    let input = "/abs/name a b";
    let prog = Program::from_custom_definition(input)?;
    assert!(matches!(&prog.kind, Kind::ExternalPath{path_and_args} if path_and_args == input));
    let sh = *SH;
    assert_eq!(
        format!("{:?}", prog.to_command(&helper::Action::Store("egal".into()))?),
        if cfg!(windows) {
            r#""/abs/name" "a" "b" "store""#.to_owned()
        } else {
            format!(r#""{sh}" "-c" "/abs/name a b store" "/abs/name a b store""#)
        },
        "a shell is used as there are arguments, and it's generally more flexible, but on windows we split ourselves"
    );
    Ok(())
}

#[test]
fn shell_helper_actions_match_git() -> TestResult {
    use std::{io::Write, process::Stdio};

    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let root = gix_testtools::scripted_fixture_read_only("shell-arguments.sh")?.join("repo");
    let context = b"protocol=https\nhost=example.com\n\n";
    let with_credentials = b"protocol=https\nhost=example.com\nusername=user\npassword=pass\n\n";
    for script in [
        r#"printf 'username=user\npassword=pass\n'; printf '<%s>\n' "$0" "$@" >&2"#,
        r#"f() { printf 'username=user\npassword=pass\n'; printf '<%s>\n' "$0" "$@" >&2; }; f"#,
    ] {
        for (action, git_action, input) in [
            (helper::Action::Get(Default::default()), "fill", context.as_slice()),
            (
                helper::Action::Store(with_credentials.as_slice().into()),
                "approve",
                with_credentials.as_slice(),
            ),
            (
                helper::Action::Erase(with_credentials.as_slice().into()),
                "reject",
                with_credentials.as_slice(),
            ),
        ] {
            let mut child = gix_testtools::git_command(&root)
                .args([
                    "-c",
                    "credential.helper=",
                    "-c",
                    &format!("credential.helper=!{script}"),
                    "credential",
                    git_action,
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;
            child.stdin.take().expect("Git input is piped").write_all(input)?;
            let reference = child.wait_with_output()?;
            assert!(reference.status.success(), "Git runs the helper: {reference:?}");
            let output = Program::from_custom_definition(format!("!{script}"))?
                .to_command(&action)?
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()?;
            assert!(output.status.success(), "the helper completes");
            assert_eq!(
                output.stderr, reference.stderr,
                "helper actions and shell parameters match Git for {git_action} with {script:?}"
            );
        }
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn standalone_named_helpers_match_git_without_a_git_dispatcher() -> TestResult {
    use std::process::Stdio;

    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let root = gix_testtools::scripted_fixture_read_only("named-helpers.sh")?.canonicalize()?;
    let exec_path = root.join("exec path");
    let helper_path = std::env::join_paths([root.join("not executable"), root.join("path helpers")])?;
    for (name, origin) in [("probe", "exec"), ("path-only", "path"), ("no-shebang", "plain")] {
        let reference = gix_testtools::git_command(root.join("repo"))
            .env("GIT_EXEC_PATH", &exec_path)
            .env(
                "PATH",
                std::env::join_paths(
                    [root.join("not executable"), root.join("path helpers")]
                        .into_iter()
                        .chain(std::env::split_paths(
                            &std::env::var_os("PATH").expect("test tools need PATH"),
                        )),
                )?,
            )
            .args([&format!("credential-{name}"), "two words", "quote's", "get"])
            .output()?;
        assert!(reference.status.success(), "Git dispatches {name}: {reference:?}");
        let _environment = gix_testtools::Env::new()
            .set("GIT_EXEC_PATH", exec_path.display().to_string())
            .set("PATH", helper_path.to_str().expect("fixture paths are Unicode"));
        let output = Program::from_custom_definition(format!(r#"{name} 'two words' "quote's""#))?
            .to_command(&helper::Action::Get(Default::default()))?
            .stderr(Stdio::piped())
            .output()?;
        assert!(output.status.success(), "{name} runs without Git: {output:?}");
        assert_eq!(
            output.stdout, reference.stdout,
            "standalone helpers preserve Git's search order and quoted arguments for {name}"
        );
        assert!(
            output.stdout.starts_with(format!("origin={origin}\n").as_bytes()),
            "the expected search location is selected"
        );
    }
    let _environment = gix_testtools::Env::new()
        .set("GIT_EXEC_PATH", exec_path.display().to_string())
        .set("PATH", helper_path.to_str().expect("fixture paths are Unicode"));
    for name in ["store", "cache", "cache--daemon"] {
        let command = Program::from_custom_definition(name)?.to_command(&helper::Action::Get(Default::default()))?;
        assert_eq!(
            command.get_program(),
            gix_path::env::exe_invocation(),
            "a standalone file cannot override Git's {name} builtin"
        );
    }
    let command =
        Program::from_custom_definition(r#"probe "$0""#)?.to_command(&helper::Action::Get(Default::default()))?;
    assert_eq!(
        command.get_args().nth(1),
        Some(std::ffi::OsStr::new(r#"command git-credential-probe "$0" get"#)),
        "shell-dependent definitions use the native helper command as argument zero"
    );
    Ok(())
}
