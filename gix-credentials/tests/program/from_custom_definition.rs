use gix_credentials::{Program, helper, program::Kind};
use gix_testtools::TestResult;
use std::sync::LazyLock;

static SH: LazyLock<&'static str> = LazyLock::new(|| {
    gix_path::env::shell()
        .to_str()
        .expect("some `from_custom_definition` tests must be run where 'sh' path is valid Unicode")
});

// The basename of the default shell, used as the `command_name` operand after
// `-c <script>` and observable inside the shell as `$0`. The default shell is
// `gix_path::env::shell()`, documented as `/bin/sh` on Unix and a path ending
// in `sh.exe` on Windows.
const SH_BASENAME: &str = if cfg!(windows) { "sh.exe" } else { "sh" };

#[test]
fn empty() -> TestResult {
    let prog = Program::from_custom_definition("")?;
    assert!(matches!(&prog.kind, Kind::ExternalName { name_and_args } if name_and_args.is_empty()));
    let cmd = prog.to_command(&helper::Action::Store("egal".into()))?;
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
    Ok(())
}

#[test]
fn simple_script_in_path() -> TestResult {
    let prog = Program::from_custom_definition("!exe")?;
    assert!(matches!(&prog.kind, Kind::ExternalShellScript(script) if script == "exe"));
    let cmd = prog.to_command(&helper::Action::Store("egal".into()))?;
    assert_eq!(
        cmd.get_program(),
        std::process::Command::try_from(gix_command::prepare("exe"))?.get_program(),
        "it didn't detect anything shell-scripty, and thus doesn't use a shell"
    );
    assert_eq!(
        cmd.get_args().collect::<Vec<_>>(),
        ["store"],
        "the helper receives its action"
    );
    Ok(())
}

#[test]
fn name_with_args() -> TestResult {
    let input = "name --arg --bar=\"a b\"";
    let prog = Program::from_custom_definition(input)?;
    assert!(matches!(&prog.kind, Kind::ExternalName{name_and_args} if name_and_args == input));
    let cmd = prog.to_command(&helper::Action::Store("egal".into()))?;
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
    assert_eq!(args[script], r#"git-credential-name --arg --bar=~/folder/in/home "$@""#);
    Ok(())
}

#[test]
fn name() -> TestResult {
    let input = "name";
    let prog = Program::from_custom_definition(input)?;
    assert!(matches!(&prog.kind, Kind::ExternalName{name_and_args} if name_and_args == input));
    let cmd = prog.to_command(&helper::Action::Store("egal".into()))?;
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
            format!(r#""{sh}" "-c" "/abs/name --arg --bar=\"a b\" \"$@\"" "{SH_BASENAME}" "store""#)
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
        r#""/abs/name" "store""#,
        "no shell is used"
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
            format!(r#""{sh}" "-c" "/abs/name a b \"$@\"" "{SH_BASENAME}" "store""#)
        },
        "a shell is used as there are arguments, and it's generally more flexible, but on windows we split ourselves"
    );
    Ok(())
}
