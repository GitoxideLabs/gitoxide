use gix_testtools::TestResult;
use std::sync::LazyLock;

#[test]
#[cfg(any(unix, windows))]
fn native_program_paths_are_preserved_without_utf8_conversion() -> TestResult {
    #[cfg(unix)]
    let program = {
        use std::os::unix::ffi::OsStringExt;
        std::ffi::OsString::from_vec(b"./native-\xff".to_vec())
    };
    #[cfg(windows)]
    let program = {
        use std::os::windows::ffi::OsStringExt;
        std::ffi::OsString::from_wide(&[b'.' as u16, b'/' as u16, 0xd800])
    };
    // An explicit PATH avoids reading the process environment during Windows preparation.
    let command = std::process::Command::try_from(gix_command::prepare(&program).env("PATH", ""))?;
    assert_eq!(
        command.get_program(),
        program,
        "direct commands preserve native path encoding"
    );
    Ok(())
}

#[test]
#[cfg(windows)]
fn native_shell_inspection_preserves_bytes_and_quoting_reports_encoding_errors() -> TestResult {
    use std::os::windows::ffi::OsStringExt;

    let native = std::ffi::OsString::from_wide(&[0xd800]);
    let mut script = native.clone();
    script.push(" $@");
    let prepare = gix_command::prepare(&script).command_may_be_shell_script();
    assert!(
        prepare.use_shell,
        "ASCII shell syntax is recognized beside an unpaired surrogate"
    );
    let command = std::process::Command::try_from(prepare.with_shell_program("unused-shell").arg("arg"))?;
    script.push(r#" "$@""#);
    assert_eq!(
        command.get_args().nth(1),
        Some(script.as_os_str()),
        "automatic argument forwarding preserves native script encoding"
    );
    let err = std::process::Command::try_from(
        gix_command::prepare(native)
            .with_shell()
            .with_shell_program("unused-shell")
            .with_quoted_command()
            .arg("arg"),
    )
    .expect_err("shell quoting needs representable bytes");
    assert!(err.is_validation(), "unrepresentable shell input is a validation error");
    Ok(())
}

#[test]
#[cfg(windows)]
fn unrepresentable_namespace_is_reported_before_spawning() -> TestResult {
    let prepare = || {
        gix_command::prepare("./unused-command")
            .env("PATH", "")
            .with_context(gix_command::Context {
                ref_namespace: Some(vec![0xff].into()),
                ..Default::default()
            })
    };
    let err = std::process::Command::try_from(prepare()).expect_err("a namespace must fit the process environment");
    assert!(
        err.is_validation(),
        "invalid namespace bytes retain their validation classification"
    );
    let err = prepare().spawn().expect_err("invalid input prevents process creation");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert!(
        err.get_ref().is_some(),
        "the encoding failure remains the I/O error's source"
    );
    Ok(())
}

fn default_shell() -> &'static str {
    static SH: LazyLock<std::ffi::OsString> = LazyLock::new(|| gix_path::env::shell_command().get_program().to_owned());
    SH.to_str()
        .expect("`prepare` tests must be run where 'sh' path is valid Unicode")
}

fn quoted(input: &[&str]) -> String {
    // These assertions cover argument parsing. Windows resolves the program before spawning, so
    // compare against the same program prepared without any argument splitting.
    let (program, args) = input.split_first().expect("a command always includes its program");
    let cmd = std::process::Command::try_from(gix_command::prepare(program))
        .expect("command fixture can be represented by the platform");
    std::iter::once(format!("{cmd:?}"))
        .chain(args.iter().map(|s| format!("\"{s}\"")))
        .collect::<Vec<_>>()
        .join(" ")
}

fn quoted_default_shell(input: &[&str]) -> String {
    let shell = gix_path::env::shell_command();
    let mut args = vec![
        shell
            .get_program()
            .to_str()
            .expect("the default shell path must be valid Unicode in these tests"),
    ];
    args.extend(
        shell
            .get_args()
            .map(|arg| arg.to_str().expect("default shell arguments must be valid Unicode")),
    );
    args.extend(input.iter().copied());
    quoted(&args)
}

#[test]
fn empty() -> TestResult {
    let cmd = std::process::Command::try_from(gix_command::prepare(""))?;
    assert_eq!(format!("{cmd:?}"), "\"\"");
    Ok(())
}

#[test]
fn whitespace_only_without_shell() -> TestResult {
    let cmd = std::process::Command::try_from(gix_command::prepare("   "))?;
    assert_eq!(format!("{cmd:?}"), quoted(&["   "]));
    Ok(())
}

#[test]
fn whitespace_only_commands_with_auto_split_fall_back_to_shell() -> TestResult {
    let cmd = std::process::Command::try_from(
        gix_command::prepare("   ").command_may_be_shell_script_allow_manual_argument_splitting(),
    )?;
    assert_eq!(format!("{cmd:?}"), quoted_default_shell(&["-c", "   ", "   "]));
    Ok(())
}

#[test]
fn single_and_multiple_arguments() -> TestResult {
    let cmd = std::process::Command::try_from(gix_command::prepare("ls").arg("first").args(["second", "third"]))?;
    assert_eq!(format!("{cmd:?}"), quoted(&["ls", "first", "second", "third"]));
    Ok(())
}

#[test]
fn multiple_arguments_in_one_line_with_auto_split() -> TestResult {
    let cmd = std::process::Command::try_from(
        gix_command::prepare("echo first second third").command_may_be_shell_script_allow_manual_argument_splitting(),
    )?;
    assert_eq!(
        format!("{cmd:?}"),
        quoted(&["echo", "first", "second", "third"]),
        "we split by hand which works unless one tries to rely on shell-builtins (which we can't detect)"
    );
    Ok(())
}

#[test]
fn shell_assignments_are_applied_during_manual_splitting() -> TestResult {
    let dir = gix_testtools::tempfile::TempDir::new()?;
    let cmd = std::process::Command::try_from(
        gix_command::prepare(r#"  FOO=bar BAR="two words" GIT_DIR=inline command.exe arg"#)
            .env("FOO", "overridden")
            .with_context(gix_command::Context {
                git_dir: Some(gix_path::realpath(dir.path())?),
                ..Default::default()
            })
            .command_may_be_shell_script_allow_manual_argument_splitting(),
    )?;
    assert_eq!(
        cmd.get_program(),
        std::process::Command::try_from(gix_command::prepare("command.exe"))?.get_program(),
        "non-PATH assignments don't prevent manual splitting"
    );
    assert_eq!(cmd.get_args().collect::<Vec<_>>(), ["arg"], "arguments are retained");
    assert_eq!(
        cmd.get_envs()
            .find(|(name, _)| *name == "BAR")
            .and_then(|(_, value)| value),
        Some(std::ffi::OsStr::new("two words"))
    );
    assert_eq!(
        cmd.get_envs()
            .find(|(name, _)| *name == "FOO")
            .and_then(|(_, value)| value),
        Some(std::ffi::OsStr::new("bar")),
        "the inline assignment overrides the inherited builder environment"
    );
    assert_eq!(
        cmd.get_envs()
            .find(|(name, _)| *name == "GIT_DIR")
            .and_then(|(_, value)| value),
        Some(std::ffi::OsStr::new("inline")),
        "inline assignments have shell precedence over context values"
    );
    Ok(())
}

#[test]
#[cfg(windows)]
fn inline_path_assignment_controls_lookup() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("win_path_lookup.sh")?;
    let joined_paths = root.join("a").to_string_lossy().replace('\\', "/");
    let program = format!("{joined_paths}/x.exe");
    let input = format!(r#"PATH="{joined_paths}" x.exe arg"#);
    let cmd = std::process::Command::try_from(
        gix_command::prepare(input)
            .env("PATH", "builder path must be overridden")
            .command_may_be_shell_script_allow_manual_argument_splitting(),
    )?;
    assert_eq!(cmd.get_program(), std::path::Path::new(&program));
    assert_eq!(
        cmd.get_args().collect::<Vec<_>>(),
        [std::ffi::OsStr::new("arg")],
        "the resolved executable receives its manually split arguments"
    );
    assert_eq!(
        cmd.get_envs()
            .find(|(name, _)| *name == "PATH")
            .and_then(|(_, value)| value),
        Some(std::ffi::OsStr::new(&joined_paths)),
        "inline PATH has shell precedence over the builder environment"
    );
    Ok(())
}

#[test]
#[cfg(windows)]
fn builder_path_controls_lookup() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("windows_shebang.sh")?;
    let joined_paths = std::env::join_paths([root.join("first"), root.join("second")])?;
    let script = root.join("first").join("hook");
    let cmd = std::process::Command::try_from(gix_command::prepare("hook").env("Path", joined_paths))?;
    assert_eq!(
        std::path::Path::new(cmd.get_program()),
        root.join("second").join("sh.exe"),
        "builder PATH selects the .exe interpreter as well as the script"
    );
    assert_eq!(
        cmd.get_args().collect::<Vec<_>>(),
        [script.as_os_str()],
        "the interpreter receives the resolved script found through the builder PATH"
    );
    Ok(())
}

#[test]
#[cfg(windows)]
fn manually_split_commands_retain_shebang_dispatch() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("windows_shebang.sh")?;
    let script = root.join("hook").to_string_lossy().replace('\\', "/");
    let paths = root.join("second").to_string_lossy().replace('\\', "/");
    let input = format!(r#"PATH="{paths}" FOO=bar "{script}" arg"#);
    let cmd = std::process::Command::try_from(
        gix_command::prepare(input).command_may_be_shell_script_allow_manual_argument_splitting(),
    )?;
    assert_eq!(
        std::path::Path::new(cmd.get_program()),
        root.join("second").join("sh.exe"),
        "inline PATH also selects the interpreter"
    );
    assert_eq!(
        cmd.get_args().collect::<Vec<_>>(),
        [std::ffi::OsStr::new(&script), std::ffi::OsStr::new("arg")],
        "the interpreter receives the script before its manually split arguments"
    );
    assert_eq!(
        cmd.get_envs()
            .find(|(name, _)| *name == "FOO")
            .and_then(|(_, value)| value),
        Some(std::ffi::OsStr::new("bar")),
        "inline assignments are retained during shebang dispatch"
    );
    Ok(())
}

#[test]
#[cfg(windows)]
fn shebang_execution_matches_git_for_windows() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let root = gix_testtools::scripted_fixture_read_only("windows_shebang.sh")?;
    let shell_directory = std::path::Path::new(gix_path::env::shell())
        .parent()
        .expect("Git for Windows provides an absolute shell path");
    let mut command = std::process::Command::try_from(
        gix_command::prepare(root.join("reference.git/hooks/pre-commit"))
            .env("PATH", shell_directory)
            .args(["one argument", "literal $value"]),
    )?;
    let output = command.output()?;
    assert!(
        output.status.success(),
        "the interpreter found in PATH executes the hook"
    );
    assert_eq!(
        output.stdout,
        std::fs::read(root.join("reference-output"))?,
        "Git and gitoxide ignore the shebang directory and options while retaining arguments"
    );
    Ok(())
}

#[test]
fn only_unambiguous_shell_assignments_are_applied() -> TestResult {
    for (input, program, args) in [
        ("tool-name=value arg", "tool-name=value", &["arg"][..]),
        (r#"'FOO'=bar command"#, "FOO=bar", &["command"][..]),
    ] {
        let cmd = std::process::Command::try_from(
            gix_command::prepare(input).command_may_be_shell_script_allow_manual_argument_splitting(),
        )?;
        assert_eq!(
            cmd.get_program(),
            std::process::Command::try_from(gix_command::prepare(program))?.get_program(),
            "{input:?} is not an assignment prefix"
        );
        assert_eq!(cmd.get_args().collect::<Vec<_>>(), args, "arguments are retained");
        assert_eq!(cmd.get_envs().count(), 0, "the environment is unchanged");
    }
    Ok(())
}

#[test]
fn assignment_only_input_is_left_to_the_shell() -> TestResult {
    let cmd = std::process::Command::try_from(
        gix_command::prepare("tool=name").command_may_be_shell_script_allow_manual_argument_splitting(),
    )?;
    assert_eq!(
        format!("{cmd:?}"),
        quoted_default_shell(&["-c", "tool=name", "tool=name"])
    );
    Ok(())
}

#[test]
#[cfg(unix)]
fn invalid_utf8_commands_are_checked_for_shell_syntax() -> TestResult {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    assert!(
        !gix_command::prepare(std::ffi::OsString::from_vec(vec![0xff]))
            .command_may_be_shell_script()
            .use_shell,
        "invalid UTF-8 alone doesn't require a shell"
    );
    assert!(
        gix_command::prepare(std::ffi::OsString::from_vec(vec![0xff, b' ']))
            .command_may_be_shell_script()
            .use_shell,
        "shell syntax is detected without requiring UTF-8"
    );

    let cmd = std::process::Command::try_from(
        gix_command::prepare(std::ffi::OsString::from_vec(vec![0xff, b' ', 0xfe]))
            .command_may_be_shell_script_allow_manual_argument_splitting(),
    )?;
    assert_eq!(cmd.get_program().as_bytes(), [0xff]);
    assert_eq!(
        cmd.get_args().map(OsStrExt::as_bytes).collect::<Vec<_>>(),
        [&[0xfe][..]],
        "manual splitting preserves invalid UTF-8"
    );
    Ok(())
}

#[test]
fn relative_existing_paths_with_shell_syntax_still_use_the_shell() -> TestResult {
    let temp = gix_testtools::tempfile::Builder::new()
        .prefix("$HOME")
        .tempdir_in(".")?;
    let program =
        std::path::Path::new(temp.path().file_name().expect("a temporary directory has a file name")).join("editor");
    std::fs::File::create(temp.path().join("editor"))?;

    assert!(
        gix_command::prepare(&program).command_may_be_shell_script().use_shell,
        "filesystem state doesn't change shell-syntax detection"
    );
    Ok(())
}

#[test]
fn single_and_multiple_arguments_as_part_of_command() -> TestResult {
    let cmd = std::process::Command::try_from(gix_command::prepare("ls first second third"))?;
    assert_eq!(
        format!("{cmd:?}"),
        quoted(&["ls first second third"]),
        "without shell, this is an invalid command"
    );
    Ok(())
}

#[test]
fn single_and_multiple_arguments_as_part_of_command_with_shell() -> TestResult {
    let cmd =
        std::process::Command::try_from(gix_command::prepare("ls first second third").command_may_be_shell_script())?;
    assert_eq!(
        format!("{cmd:?}"),
        if cfg!(windows) {
            quoted(&["ls", "first", "second", "third"])
        } else {
            quoted(&[default_shell(), "-c", "ls first second third", "ls first second third"])
        },
        "with shell, this works as it performs word splitting"
    );
    Ok(())
}

#[test]
fn single_and_multiple_arguments_as_part_of_command_with_given_shell() -> TestResult {
    let cmd = std::process::Command::try_from(
        gix_command::prepare("ls first second third")
            .command_may_be_shell_script()
            .with_shell_program("/somepath/to/bash"),
    )?;
    assert_eq!(
        format!("{cmd:?}"),
        if cfg!(windows) {
            quoted(&["ls", "first", "second", "third"])
        } else {
            quoted(&[
                "/somepath/to/bash",
                "-c",
                "ls first second third",
                "ls first second third",
            ])
        },
        "with shell, this works as it performs word splitting on Windows, but on linux (or without splitting) it uses the given shell"
    );
    Ok(())
}

#[test]
fn single_and_complex_arguments_as_part_of_command_with_shell() -> TestResult {
    let cmd = std::process::Command::try_from(
        gix_command::prepare(r#"ls --foo "a b""#)
            .arg("additional")
            .command_may_be_shell_script(),
    )?;
    assert_eq!(
        format!("{cmd:?}"),
        if cfg!(windows) {
            quoted(&["ls", "--foo", "a b", "additional"])
        } else {
            let sh = default_shell();
            format!(r#""{sh}" "-c" "ls --foo \"a b\" \"$@\"" "ls --foo \"a b\"" "additional""#)
        },
        "with shell, this works as it performs word splitting, on windows we can avoid the shell"
    );
    Ok(())
}

#[test]
fn single_and_complex_arguments_with_auto_split() -> TestResult {
    let cmd = std::process::Command::try_from(
        gix_command::prepare(r#"ls --foo="a b""#).command_may_be_shell_script_allow_manual_argument_splitting(),
    )?;
    assert_eq!(
        format!("{cmd:?}"),
        quoted(&["ls", "--foo=a b"]),
        "splitting can also handle quotes"
    );
    Ok(())
}

#[test]
fn single_and_complex_arguments_without_auto_split() -> TestResult {
    let cmd = std::process::Command::try_from(
        gix_command::prepare(r#"ls --foo="a b""#).command_may_be_shell_script_disallow_manual_argument_splitting(),
    )?;
    assert_eq!(
        format!("{cmd:?}"),
        quoted_default_shell(&["-c", r#"ls --foo=\"a b\""#, r#"ls --foo=\"a b\""#])
    );
    Ok(())
}

#[test]
fn single_and_simple_arguments_without_auto_split_with_shell() -> TestResult {
    let cmd = std::process::Command::try_from(gix_command::prepare("ls").arg("--foo=a b").with_shell())?;
    assert_eq!(
        format!("{cmd:?}"),
        quoted_default_shell(&["-c", r#"ls \"$@\""#, "ls", "--foo=a b"])
    );
    Ok(())
}

#[test]
fn quoted_command_without_argument_splitting() -> TestResult {
    let cmd = std::process::Command::try_from(
        gix_command::prepare("ls")
            .arg("--foo=a b")
            .with_shell()
            .with_quoted_command(),
    )?;
    assert_eq!(
        format!("{cmd:?}"),
        quoted_default_shell(&["-c", r#"'ls' \"$@\""#, "ls", "--foo=a b"]),
        "looks strange thanks to debug printing, but is the right amount of quotes actually"
    );
    Ok(())
}

#[test]
fn quoted_windows_command_without_argument_splitting() -> TestResult {
    let cmd = std::process::Command::try_from(
        gix_command::prepare(r"C:\Users\O'Shaughnessy\with space.exe")
            .arg("--foo='a b'")
            .with_shell()
            .with_quoted_command(),
    )?;
    assert_eq!(
        format!("{cmd:?}"),
        quoted_default_shell(&[
            "-c",
            r#"'C:\\Users\\O'\\''Shaughnessy\\with space.exe' \"$@\""#,
            r"C:\\Users\\O'Shaughnessy\\with space.exe",
            r"--foo='a b'"
        ]),
        "again, a lot of extra backslashes, but it's correct outside of the debug formatting"
    );
    Ok(())
}

#[test]
fn single_and_complex_arguments_will_not_auto_split_on_special_characters() -> TestResult {
    let cmd = std::process::Command::try_from(
        gix_command::prepare("ls --foo=~/path").command_may_be_shell_script_allow_manual_argument_splitting(),
    )?;
    assert_eq!(
        format!("{cmd:?}"),
        quoted_default_shell(&["-c", "ls --foo=~/path", "ls --foo=~/path"]),
        "splitting can also handle quotes"
    );
    Ok(())
}

#[test]
fn tilde_path_and_multiple_arguments_as_part_of_command_with_shell() -> TestResult {
    let cmd = std::process::Command::try_from(
        gix_command::prepare(r#"~/bin/exe --foo "a b""#).command_may_be_shell_script(),
    )?;
    assert_eq!(
        format!("{cmd:?}"),
        quoted_default_shell(&["-c", r#"~/bin/exe --foo \"a b\""#, r#"~/bin/exe --foo \"a b\""#]),
        "this always needs a shell as we need tilde expansion"
    );
    Ok(())
}

#[test]
fn script_with_dollar_at() -> TestResult {
    let cmd = std::process::Command::try_from(
        gix_command::prepare(r#"echo "$@" >&2"#)
            .command_may_be_shell_script()
            .arg("store"),
    )?;
    assert_eq!(
        format!("{cmd:?}"),
        quoted_default_shell(&["-c", r#"echo \"$@\" >&2 \"$@\""#, r#"echo \"$@\" >&2"#, "store"]),
        "Git appends arguments even when the command already expands them"
    );
    Ok(())
}

#[test]
#[cfg(unix)]
fn non_utf8_shell_scripts_keep_both_argument_expansions() -> TestResult {
    use bstr::ByteSlice;
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    let script = std::ffi::OsString::from_vec(b"echo \xff \"$@\"".to_vec());
    let cmd = std::process::Command::try_from(
        gix_command::prepare(script.clone())
            .command_may_be_shell_script()
            .arg("argument"),
    )?;
    assert_eq!(
        cmd.get_args()
            .nth(1)
            .expect("the script follows -c")
            .as_bytes()
            .as_bstr(),
        [script.as_bytes(), br#" "$@""#].concat().as_bstr(),
        "automatic forwarding retains the original non-UTF-8 script"
    );
    Ok(())
}

#[test]
fn quoted_commands_keep_dollar_at_literal_with_or_without_arguments() -> TestResult {
    use std::ffi::OsStr;
    for arguments in [&[][..], &["argument"][..]] {
        let cmd = std::process::Command::try_from(
            gix_command::prepare("some $@ path")
                .with_shell()
                .with_quoted_command()
                .args(arguments.iter().copied()),
        )?;
        let script = if arguments.is_empty() {
            "'some $@ path'"
        } else {
            r#"'some $@ path' "$@""#
        };
        let mut expected = vec![OsStr::new("-c"), OsStr::new(script), OsStr::new("some $@ path")];
        expected.extend(arguments.iter().map(OsStr::new));
        assert_eq!(
            cmd.get_args().skip_while(|arg| *arg != "-c").collect::<Vec<_>>(),
            expected,
            "quoted program paths remain literal, including without additional arguments"
        );
    }
    Ok(())
}
