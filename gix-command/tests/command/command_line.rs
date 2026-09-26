use std::ffi::OsString;

use gix_command::parse::{self, Outcome};
use gix_error::{Result, ResultExt};

#[test]
fn words_are_split_without_expansion() -> gix_testtools::Result {
    assert_eq!(
        command_line(
            r#"cmd 'single quoted' "double \"quoted\"" escaped\ word "kept\q" "" # ignored
next"#,
        )?,
        Outcome {
            env: Vec::new(),
            command: "cmd".into(),
            args: args(&[
                "single quoted",
                "double \"quoted\"",
                "escaped word",
                r"kept\q",
                "",
                "next"
            ]),
        }
    );
    assert_eq!(
        command_line("cmd one\\\ntwo")?.args,
        args(&["onetwo"]),
        "a backslash-newline continues the line without becoming part of the word"
    );
    Ok(())
}

#[test]
fn assignments_are_returned_separately() -> gix_testtools::Result {
    assert_eq!(
        command_line(r#" FIRST=one SECOND="two words" _THIRD='' command arg"#)?,
        Outcome {
            env: vec![
                ("FIRST".into(), "one".into()),
                ("SECOND".into(), "two words".into()),
                ("_THIRD".into(), OsString::new()),
            ],
            command: "command".into(),
            args: args(&["arg"]),
        }
    );
    Ok(())
}

#[test]
fn invalid_assignment_names_are_arguments() -> gix_testtools::Result {
    for (input, expected) in [
        ("tool-name=value arg", &["tool-name=value", "arg"]),
        (r#"'FOO'=bar command"#, &["FOO=bar", "command"]),
        (r#"F"OO"=bar command"#, &["FOO=bar", "command"]),
    ] {
        let outcome = command_line(input)?;
        assert_eq!(outcome.env, [], "{input:?} has no assignment prefix");
        assert_eq!(outcome.command, expected[0], "{input:?} is the command");
        assert_eq!(outcome.args, args(&expected[1..]));
    }
    Ok(())
}

#[test]
fn unterminated_quotes_are_rejected() {
    let mut error_snapshots = Vec::new();
    for input in ["cmd '", "cmd \"", "cmd \"\\"] {
        let err = parse::command_line(input.into()).expect_err("unterminated quote");
        assert_eq!(*err, parse::Error::MissingClosingQuote);
        error_snapshots.push((input, gix_testtools::redact_debug_snapshot(&err, &[])));
    }
    insta::assert_debug_snapshot!(error_snapshots, "unterminated quotes are rejected", @r#"
    [
        (
            "cmd '",
            missing closing quote,
        ),
        (
            "cmd \"",
            missing closing quote,
        ),
        (
            "cmd \"\\",
            missing closing quote,
        ),
    ]
    "#);
}

#[test]
fn dangling_unquoted_escape_is_rejected() {
    let err = parse::command_line("cmd arg\\".into()).expect_err("dangling escape");
    assert_eq!(*err, parse::Error::MissingEscapedByte);
    insta::assert_debug_snapshot!(err, "dangling unquoted escape is rejected", @"missing byte after escape");
}

#[test]
fn a_command_is_required() {
    let mut error_snapshots = Vec::new();
    for input in ["", " ", "\t\n", "# comment", "\\\n", "tool=name", "FOO=one BAR=two"] {
        let err = parse::command_line(input.into()).expect_err("no command");
        assert_eq!(*err, parse::Error::MissingCommand, "{input:?}");
        error_snapshots.push((input, gix_testtools::redact_debug_snapshot(&err, &[])));
    }
    insta::assert_debug_snapshot!(error_snapshots, "a command is required", @r##"
    [
        (
            "",
            missing command,
        ),
        (
            " ",
            missing command,
        ),
        (
            "\t\n",
            missing command,
        ),
        (
            "# comment",
            missing command,
        ),
        (
            "\\\n",
            missing command,
        ),
        (
            "tool=name",
            missing command,
        ),
        (
            "FOO=one BAR=two",
            missing command,
        ),
    ]
    "##);
}

#[test]
fn parse_errors_retain_their_classification() {
    let mut error_snapshots = Vec::new();
    for input in ["cmd '", "cmd arg\\", "FOO=one"] {
        let err = parse::command_line(input.into()).expect_err("the command line is invalid");
        let cause = *err;
        error_snapshots.push(gix_testtools::redact_debug_snapshot(&(err), &[]));
        assert!(err.is_validation(), "invalid commands classify as validation failures");
        assert_eq!(
            err.probable_cause().downcast_ref::<parse::Error>(),
            Some(&cause),
            "the parser error, not its classification marker, is the probable cause"
        );
        assert_eq!(
            err.downcast_any_ref::<parse::Error>(),
            Some(&cause),
            "the original parser error remains available"
        );
    }
    insta::assert_debug_snapshot!(error_snapshots, "parse errors retain their classification", @"
    [
        missing closing quote,
        missing byte after escape,
        missing command,
    ]
    ");
}

#[test]
#[cfg(unix)]
fn non_utf8_input_is_preserved() -> gix_testtools::Result {
    use bstr::ByteSlice;
    use std::os::unix::ffi::OsStringExt;

    assert_eq!(
        parse::command_line(b"FOO=\xff cmd \xfe".as_bstr())?,
        Outcome {
            env: vec![("FOO".into(), OsString::from_vec(vec![0xff]))],
            command: "cmd".into(),
            args: vec![OsString::from_vec(vec![0xfe])],
        }
    );
    Ok(())
}

fn command_line(input: &str) -> Result<Outcome> {
    parse::command_line(input.into()).or_error()
}

fn args(input: &[&str]) -> Vec<OsString> {
    input.iter().map(|arg| (*arg).into()).collect()
}
