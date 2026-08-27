use std::sync::LazyLock;

fn default_shell() -> &'static str {
    static SH: LazyLock<std::ffi::OsString> = LazyLock::new(|| gix_path::env::shell_command().get_program().to_owned());
    SH.to_str()
        .expect("`prepare` tests must be run where 'sh' path is valid Unicode")
}

// The basename of the default shell, used as the `command_name` operand
// after `-c <script>` and observable inside the shell as `$0`. The default
// shell command uses `/bin/sh` on Unix and a path ending in `sh.exe` on
// Windows.
const SH_BASENAME: &str = if cfg!(windows) { "sh.exe" } else { "sh" };

fn quoted(input: &[&str]) -> String {
    input.iter().map(|s| format!("\"{s}\"")).collect::<Vec<_>>().join(" ")
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
fn empty() {
    let cmd = std::process::Command::from(gix_command::prepare(""));
    assert_eq!(format!("{cmd:?}"), "\"\"");
}

#[test]
fn whitespace_only_without_shell() {
    let cmd = std::process::Command::from(gix_command::prepare("   "));
    assert_eq!(format!("{cmd:?}"), "\"   \"");
}

#[test]
fn whitespace_only_commands_with_auto_split_fall_back_to_shell() {
    let cmd = std::process::Command::from(
        gix_command::prepare("   ").command_may_be_shell_script_allow_manual_argument_splitting(),
    );
    assert_eq!(format!("{cmd:?}"), quoted_default_shell(&["-c", "   ", SH_BASENAME]));
}

#[test]
fn single_and_multiple_arguments() {
    let cmd = std::process::Command::from(gix_command::prepare("ls").arg("first").args(["second", "third"]));
    assert_eq!(format!("{cmd:?}"), quoted(&["ls", "first", "second", "third"]));
}

#[test]
fn multiple_arguments_in_one_line_with_auto_split() {
    let cmd = std::process::Command::from(
        gix_command::prepare("echo first second third").command_may_be_shell_script_allow_manual_argument_splitting(),
    );
    assert_eq!(
        format!("{cmd:?}"),
        quoted(&["echo", "first", "second", "third"]),
        "we split by hand which works unless one tries to rely on shell-builtins (which we can't detect)"
    );
}

#[test]
fn shell_assignments_are_applied_during_manual_splitting() {
    let cmd = std::process::Command::from(
        gix_command::prepare(r#"  FOO=bar BAR="two words" GIT_DIR=inline command.exe arg"#)
            .env("FOO", "overridden")
            .with_context(gix_command::Context {
                git_dir: Some("context".into()),
                ..Default::default()
            })
            .command_may_be_shell_script_allow_manual_argument_splitting(),
    );
    assert_eq!(
        cmd.get_program(),
        "command.exe",
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
}

#[test]
#[cfg(windows)]
fn inline_path_assignment_controls_lookup() -> gix_testtools::Result {
    let root = gix_testtools::scripted_fixture_read_only("win_path_lookup.sh")?;
    let joined_paths = root.join("a").to_string_lossy().replace('\\', "/");
    let program = format!("{joined_paths}/x.exe");
    let input = format!(r#"PATH="{joined_paths}" x.exe arg"#);
    let cmd = std::process::Command::from(
        gix_command::prepare(input)
            .env("PATH", "builder path must be overridden")
            .command_may_be_shell_script_allow_manual_argument_splitting(),
    );
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
fn builder_path_controls_lookup() -> gix_testtools::Result {
    let root = gix_testtools::scripted_fixture_read_only("win_path_lookup.sh")?;
    let joined_paths = root.join("b").to_string_lossy().replace('\\', "/");
    let script = std::path::PathBuf::from(&joined_paths).join("exe");
    let cmd = std::process::Command::from(gix_command::prepare("exe").env("Path", joined_paths.as_str()));
    assert_eq!(cmd.get_program(), std::path::Path::new("/b/exe"));
    assert_eq!(
        cmd.get_args().collect::<Vec<_>>(),
        [script.as_os_str()],
        "the interpreter receives the resolved script found through the builder PATH"
    );
    Ok(())
}

#[test]
#[cfg(windows)]
fn manually_split_commands_retain_shebang_dispatch() -> gix_testtools::Result {
    let root = gix_testtools::scripted_fixture_read_only("win_path_lookup.sh")?;
    let script = root.join("b").join("exe").to_string_lossy().replace('\\', "/");
    let input = format!(r#"FOO=bar "{script}" arg"#);
    let cmd = std::process::Command::from(
        gix_command::prepare(input).command_may_be_shell_script_allow_manual_argument_splitting(),
    );
    assert_eq!(cmd.get_program(), std::path::Path::new("/b/exe"));
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
fn only_unambiguous_shell_assignments_are_applied() {
    for (input, program, args) in [
        ("tool-name=value arg", "tool-name=value", &["arg"][..]),
        (r#"'FOO'=bar command"#, "FOO=bar", &["command"][..]),
    ] {
        let cmd = std::process::Command::from(
            gix_command::prepare(input).command_may_be_shell_script_allow_manual_argument_splitting(),
        );
        assert_eq!(cmd.get_program(), program, "{input:?} is not an assignment prefix");
        assert_eq!(cmd.get_args().collect::<Vec<_>>(), args, "arguments are retained");
        assert_eq!(cmd.get_envs().count(), 0, "the environment is unchanged");
    }
}

#[test]
fn assignment_only_input_is_left_to_the_shell() {
    let cmd = std::process::Command::from(
        gix_command::prepare("tool=name").command_may_be_shell_script_allow_manual_argument_splitting(),
    );
    assert_eq!(
        format!("{cmd:?}"),
        quoted_default_shell(&["-c", "tool=name", SH_BASENAME])
    );
}

#[test]
#[cfg(unix)]
fn invalid_utf8_commands_are_checked_for_shell_syntax() {
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

    let cmd = std::process::Command::from(
        gix_command::prepare(std::ffi::OsString::from_vec(vec![0xff, b' ', 0xfe]))
            .command_may_be_shell_script_allow_manual_argument_splitting(),
    );
    assert_eq!(cmd.get_program().as_bytes(), [0xff]);
    assert_eq!(
        cmd.get_args().map(OsStrExt::as_bytes).collect::<Vec<_>>(),
        [&[0xfe][..]],
        "manual splitting preserves invalid UTF-8"
    );
}

#[test]
fn relative_existing_paths_with_shell_syntax_still_use_the_shell() -> crate::Result {
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
fn single_and_multiple_arguments_as_part_of_command() {
    let cmd = std::process::Command::from(gix_command::prepare("ls first second third"));
    assert_eq!(
        format!("{cmd:?}"),
        quoted(&["ls first second third"]),
        "without shell, this is an invalid command"
    );
}

#[test]
fn single_and_multiple_arguments_as_part_of_command_with_shell() {
    let cmd = std::process::Command::from(gix_command::prepare("ls first second third").command_may_be_shell_script());
    assert_eq!(
        format!("{cmd:?}"),
        if cfg!(windows) {
            quoted(&["ls", "first", "second", "third"])
        } else {
            quoted(&[default_shell(), "-c", "ls first second third", SH_BASENAME])
        },
        "with shell, this works as it performs word splitting"
    );
}

#[test]
fn single_and_multiple_arguments_as_part_of_command_with_given_shell() {
    let cmd = std::process::Command::from(
        gix_command::prepare("ls first second third")
            .command_may_be_shell_script()
            .with_shell_program("/somepath/to/bash"),
    );
    assert_eq!(
        format!("{cmd:?}"),
        if cfg!(windows) {
            quoted(&["ls", "first", "second", "third"])
        } else {
            quoted(&["/somepath/to/bash", "-c", "ls first second third", "bash"])
        },
        "with shell, this works as it performs word splitting on Windows, but on linux (or without splitting) it uses the given shell"
    );
}

#[test]
fn single_and_complex_arguments_as_part_of_command_with_shell() {
    let cmd = std::process::Command::from(
        gix_command::prepare(r#"ls --foo "a b""#)
            .arg("additional")
            .command_may_be_shell_script(),
    );
    assert_eq!(
        format!("{cmd:?}"),
        if cfg!(windows) {
            quoted(&["ls", "--foo", "a b", "additional"])
        } else {
            let sh = default_shell();
            format!(r#""{sh}" "-c" "ls --foo \"a b\" \"$@\"" "{SH_BASENAME}" "additional""#)
        },
        "with shell, this works as it performs word splitting, on windows we can avoid the shell"
    );
}

#[test]
fn single_and_complex_arguments_with_auto_split() {
    let cmd = std::process::Command::from(
        gix_command::prepare(r#"ls --foo="a b""#).command_may_be_shell_script_allow_manual_argument_splitting(),
    );
    assert_eq!(
        format!("{cmd:?}"),
        r#""ls" "--foo=a b""#,
        "splitting can also handle quotes"
    );
}

#[test]
fn single_and_complex_arguments_without_auto_split() {
    let cmd = std::process::Command::from(
        gix_command::prepare(r#"ls --foo="a b""#).command_may_be_shell_script_disallow_manual_argument_splitting(),
    );
    assert_eq!(
        format!("{cmd:?}"),
        quoted_default_shell(&["-c", r#"ls --foo=\"a b\""#, SH_BASENAME])
    );
}

#[test]
fn single_and_simple_arguments_without_auto_split_with_shell() {
    let cmd = std::process::Command::from(gix_command::prepare("ls").arg("--foo=a b").with_shell());
    assert_eq!(
        format!("{cmd:?}"),
        quoted_default_shell(&["-c", r#"ls \"$@\""#, SH_BASENAME, "--foo=a b"])
    );
}

#[test]
fn quoted_command_without_argument_splitting() {
    let cmd = std::process::Command::from(
        gix_command::prepare("ls")
            .arg("--foo=a b")
            .with_shell()
            .with_quoted_command(),
    );
    assert_eq!(
        format!("{cmd:?}"),
        quoted_default_shell(&["-c", r#"'ls' \"$@\""#, SH_BASENAME, "--foo=a b"]),
        "looks strange thanks to debug printing, but is the right amount of quotes actually"
    );
}

#[test]
fn quoted_windows_command_without_argument_splitting() {
    let cmd = std::process::Command::from(
        gix_command::prepare(r"C:\Users\O'Shaughnessy\with space.exe")
            .arg("--foo='a b'")
            .with_shell()
            .with_quoted_command(),
    );
    assert_eq!(
        format!("{cmd:?}"),
        quoted_default_shell(&[
            "-c",
            r#"'C:\\Users\\O'\\''Shaughnessy\\with space.exe' \"$@\""#,
            SH_BASENAME,
            r"--foo='a b'"
        ]),
        "again, a lot of extra backslashes, but it's correct outside of the debug formatting"
    );
}

#[test]
fn single_and_complex_arguments_will_not_auto_split_on_special_characters() {
    let cmd = std::process::Command::from(
        gix_command::prepare("ls --foo=~/path").command_may_be_shell_script_allow_manual_argument_splitting(),
    );
    assert_eq!(
        format!("{cmd:?}"),
        quoted_default_shell(&["-c", "ls --foo=~/path", SH_BASENAME]),
        "splitting can also handle quotes"
    );
}

#[test]
fn tilde_path_and_multiple_arguments_as_part_of_command_with_shell() {
    let cmd =
        std::process::Command::from(gix_command::prepare(r#"~/bin/exe --foo "a b""#).command_may_be_shell_script());
    assert_eq!(
        format!("{cmd:?}"),
        quoted_default_shell(&["-c", r#"~/bin/exe --foo \"a b\""#, SH_BASENAME]),
        "this always needs a shell as we need tilde expansion"
    );
}

#[test]
fn script_with_dollar_at() {
    let cmd = std::process::Command::from(
        gix_command::prepare(r#"echo "$@" >&2"#)
            .command_may_be_shell_script()
            .arg("store"),
    );
    assert_eq!(
        format!("{cmd:?}"),
        quoted_default_shell(&["-c", r#"echo \"$@\" >&2"#, SH_BASENAME, "store"]),
        "this is how credential helpers have to work as for some reason they don't get '$@' added in Git.\
            We deal with it by not doubling the '$@' argument, which seems more flexible."
    );
}

#[test]
#[cfg(unix)]
fn non_utf8_script_with_dollar_at_does_not_duplicate_arguments() {
    use bstr::ByteSlice;
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    let script = std::ffi::OsString::from_vec(b"echo \xff \"$@\"".to_vec());
    let cmd = std::process::Command::from(
        gix_command::prepare(script.clone())
            .command_may_be_shell_script()
            .arg("argument"),
    );
    assert_eq!(
        cmd.get_args()
            .nth(1)
            .expect("the script follows -c")
            .as_bytes()
            .as_bstr(),
        script.as_bytes().as_bstr(),
        "the existing byte-encoded $@ is retained without appending another one"
    );
}

#[test]
fn script_with_dollar_at_has_no_quoting() {
    let cmd = std::process::Command::from(
        gix_command::prepare(r#"echo "$@" >&2"#)
            .command_may_be_shell_script()
            .with_quoted_command()
            .arg("store"),
    );
    assert_eq!(
        format!("{cmd:?}"),
        quoted_default_shell(&["-c", r#"echo \"$@\" >&2"#, SH_BASENAME, "store"])
    );
}

#[test]
fn shell_program_with_no_basename_uses_underscore_placeholder() {
    // Defensive fallback for degenerate input that should not occur in
    // practice. If a caller passes a shell path whose `file_name()` is
    // `None` (empty string, `/`, etc.), the `command_name` operand falls
    // back to `_`, the conventional placeholder for an unused `$0` used
    // in shell one-liners. Such a "shell" would not produce a runnable
    // command — the fallback only keeps the construction total in the
    // face of bad input, without making a false claim about the shell.
    let cmd = std::process::Command::from(
        gix_command::prepare("echo hi")
            .command_may_be_shell_script_disallow_manual_argument_splitting()
            .with_shell_program(""),
    );
    assert_eq!(
        format!("{cmd:?}"),
        quoted(&["", "-c", "echo hi", "_"]),
        "with no basename available, the command_name operand is '_', not a guessed shell name"
    );
}
