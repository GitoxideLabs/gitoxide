use std::ffi::OsString;

use gix_command::parse::{self, Outcome};

#[test]
fn words_are_split_without_expansion() -> gix_testtools::Result {
    assert_eq!(
        command_line(
            r#"cmd 'single quoted' "double \"quoted\"" escaped\ word "kept\q" "" # ignored
next"#,
        ),
        Ok(Outcome {
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
        })
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
    assert_eq!(command_line("cmd '"), Err(parse::Error::MissingClosingQuote));
    assert_eq!(command_line("cmd \""), Err(parse::Error::MissingClosingQuote));
    assert_eq!(command_line("cmd \"\\"), Err(parse::Error::MissingClosingQuote));
}

#[test]
fn dangling_unquoted_escape_is_rejected() {
    assert_eq!(command_line("cmd arg\\"), Err(parse::Error::MissingEscapedByte));
}

#[test]
fn a_command_is_required() {
    for input in ["", " ", "\t\n", "# comment", "\\\n", "tool=name", "FOO=one BAR=two"] {
        assert_eq!(command_line(input), Err(parse::Error::MissingCommand), "{input:?}");
    }
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

fn command_line(input: &str) -> Result<Outcome, parse::Error> {
    parse::command_line(input.into())
}

fn args(input: &[&str]) -> Vec<OsString> {
    input.iter().map(|arg| (*arg).into()).collect()
}
