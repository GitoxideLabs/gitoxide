use gix_testtools::Result;
use std::path::Path;

#[test]
fn extract_interpreter() -> gix_testtools::Result {
    let root = gix_testtools::scripted_fixture_read_only("win_path_lookup.sh")?;
    assert_eq!(
        gix_command::extract_interpreter(&root.join("b").join("exe")),
        Some(gix_command::shebang::Data {
            interpreter: Path::new("/b/exe").into(),
            args: vec![]
        })
    );
    Ok(())
}

mod shebang {
    mod parse {
        use gix_command::shebang;

        fn parse(input: &str) -> Option<shebang::Data> {
            shebang::parse(input.into())
        }

        fn exe(name: &str) -> Option<shebang::Data> {
            shebang::Data {
                interpreter: name.into(),
                args: Vec::new(),
            }
            .into()
        }

        fn exe_arg(name: &str, arg: &str) -> Option<shebang::Data> {
            shebang::Data {
                interpreter: name.into(),
                args: shell_words::split(arg)
                    .expect("can parse")
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            }
            .into()
        }

        #[test]
        fn valid() {
            assert_eq!(parse("#!/bin/sh"), exe("/bin/sh"));
            assert_eq!(parse("#!/bin/sh   "), exe("/bin/sh"), "trim trailing whitespace");
            assert_eq!(
                parse("#!/bin/sh\t\nother"),
                exe("/bin/sh"),
                "trimming works for tabs as well"
            );
            assert_eq!(
                parse("#!\\bin\\sh"),
                exe("\\bin\\sh"),
                "backslashes are recognized as path separator"
            );
            assert_eq!(
                parse("#!C:\\Program Files\\shell.exe\r\nsome stuff"),
                exe("C:\\Program Files\\shell.exe"),
                "absolute windows paths are fine"
            );
            assert_eq!(
                parse("#!/bin/sh -i -o -u\nunrelated content"),
                exe_arg("/bin/sh", "-i -o -u"),
                "argument splitting happens as well"
            );
            assert_eq!(
                parse("#!/bin/sh  -o\nunrelated content"),
                exe_arg("/bin/sh", "-o"),
                "single arguments are OK too"
            );
            assert_eq!(
                parse("#!/bin/exe anything goes\nunrelated content"),
                exe_arg("/bin/exe", "anything goes"),
                "any argument works"
            );

            #[cfg(unix)]
            {
                use bstr::ByteSlice;
                use std::os::unix::ffi::OsStrExt;
                assert_eq!(
                    shebang::parse(b"#!/bin/sh   -x \xC3\x28\x41 -y  ".as_bstr()),
                    Some(shebang::Data {
                        interpreter: "/bin/sh".into(),
                        args: vec![std::ffi::OsStr::from_bytes(b"-x \xC3\x28\x41 -y").to_owned()]
                    }),
                    "illformed UTF8 in the arguments leads to them not being split - useful in case it's just one path or so"
                );

                assert_eq!(
                    shebang::parse(b"#!/bin/\xC3\x28\x41 ".as_bstr()),
                    Some(shebang::Data {
                        interpreter: std::ffi::OsStr::from_bytes(b"/bin/\xC3\x28\x41").to_owned().into(),
                        args: vec![]
                    }),
                    "illformed UTF8 in the executable path is fine as well"
                );
            }
        }

        #[test]
        fn invalid() {
            assert_eq!(parse(""), None);
            assert_eq!(parse("missing shebang"), None);
            assert_eq!(parse("#!missing-slash"), None);
            assert_eq!(
                parse("/bin/sh"),
                None,
                "shebang missing, even though a valid path is given"
            );
        }
    }
}

mod context {
    use gix_command::Context;

    fn winfix(expected: impl Into<String>) -> String {
        // Unclear why it's not debug-printing the env on windows.
        if cfg!(windows) {
            "\"\"".into()
        } else {
            expected.into()
        }
    }

    #[test]
    fn git_dir_sets_git_dir_env_and_cwd() {
        let ctx = Context {
            git_dir: Some(".".into()),
            ..Default::default()
        };
        let cmd = std::process::Command::from(gix_command::prepare("").with_context(ctx));
        assert_eq!(format!("{cmd:?}"), winfix(r#"GIT_DIR="." """#));
    }

    #[test]
    fn worktree_dir_sets_env_only() {
        let ctx = Context {
            worktree_dir: Some(".".into()),
            ..Default::default()
        };
        let cmd = std::process::Command::from(gix_command::prepare("").with_context(ctx));
        assert_eq!(format!("{cmd:?}"), winfix(r#"GIT_WORK_TREE="." """#));
    }

    #[test]
    fn no_replace_objects_sets_env_only() {
        for value in [false, true] {
            let expected = usize::from(value);
            let ctx = Context {
                no_replace_objects: Some(value),
                ..Default::default()
            };
            let cmd = std::process::Command::from(gix_command::prepare("").with_context(ctx));
            assert_eq!(
                format!("{cmd:?}"),
                winfix(format!(r#"GIT_NO_REPLACE_OBJECTS="{expected}" """#))
            );
        }
    }

    #[test]
    fn ref_namespace_sets_env_only() {
        let ctx = Context {
            ref_namespace: Some("namespace".into()),
            ..Default::default()
        };
        let cmd = std::process::Command::from(gix_command::prepare("").with_context(ctx));
        assert_eq!(format!("{cmd:?}"), winfix(r#"GIT_NAMESPACE="namespace" """#));
    }

    #[test]
    fn literal_pathspecs_sets_env_only() {
        for value in [false, true] {
            let expected = usize::from(value);
            let ctx = Context {
                literal_pathspecs: Some(value),
                ..Default::default()
            };
            let cmd = std::process::Command::from(gix_command::prepare("").with_context(ctx));
            assert_eq!(
                format!("{cmd:?}"),
                winfix(format!(r#"GIT_LITERAL_PATHSPECS="{expected}" """#))
            );
        }
    }

    #[test]
    fn glob_pathspecs_sets_env_only() {
        for (value, expected) in [
            (false, "GIT_NOGLOB_PATHSPECS=\"1\""),
            (true, "GIT_GLOB_PATHSPECS=\"1\""),
        ] {
            let ctx = Context {
                glob_pathspecs: Some(value),
                ..Default::default()
            };
            let cmd = std::process::Command::from(gix_command::prepare("").with_context(ctx));
            assert_eq!(format!("{cmd:?}"), winfix(format!(r#"{expected} """#)));
        }
    }

    #[test]
    fn icase_pathspecs_sets_env_only() {
        for value in [false, true] {
            let expected = usize::from(value);
            let ctx = Context {
                icase_pathspecs: Some(value),
                ..Default::default()
            };
            let cmd = std::process::Command::from(gix_command::prepare("").with_context(ctx));
            assert_eq!(
                format!("{cmd:?}"),
                winfix(format!(r#"GIT_ICASE_PATHSPECS="{expected}" """#))
            );
        }
    }
}

mod prepare {
    use once_cell::sync::Lazy;

    static SH: Lazy<&'static str> = Lazy::new(|| {
        gix_path::env::shell()
            .to_str()
            .expect("`prepare` tests must be run where 'sh' path is valid Unicode")
    });

    fn quoted(input: &[&str]) -> String {
        input.iter().map(|s| format!("\"{s}\"")).collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn empty() {
        let cmd = std::process::Command::from(gix_command::prepare(""));
        assert_eq!(format!("{cmd:?}"), "\"\"");
    }

    #[test]
    fn single_and_multiple_arguments() {
        let cmd = std::process::Command::from(gix_command::prepare("ls").arg("first").args(["second", "third"]));
        assert_eq!(format!("{cmd:?}"), quoted(&["ls", "first", "second", "third"]));
    }

    #[test]
    fn multiple_arguments_in_one_line_with_auto_split() {
        let cmd = std::process::Command::from(
            gix_command::prepare("echo first second third")
                .command_may_be_shell_script_allow_manual_argument_splitting(),
        );
        assert_eq!(
            format!("{cmd:?}"),
            quoted(&["echo", "first", "second", "third"]),
            "we split by hand which works unless one tries to rely on shell-builtins (which we can't detect)"
        );
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
        let cmd =
            std::process::Command::from(gix_command::prepare("ls first second third").command_may_be_shell_script());
        assert_eq!(
            format!("{cmd:?}"),
            if cfg!(windows) {
                quoted(&["ls", "first", "second", "third"])
            } else {
                quoted(&[*SH, "-c", "ls first second third", "--"])
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
                quoted(&["/somepath/to/bash", "-c", "ls first second third", "--"])
            },
            "with shell, this works as it performs word splitting on Windows, but on linux (or without splitting) it uses the given shell"
        );
    }

    #[test]
    fn single_and_complex_arguments_as_part_of_command_with_shell() {
        let cmd = std::process::Command::from(
            gix_command::prepare("ls --foo \"a b\"")
                .arg("additional")
                .command_may_be_shell_script(),
        );
        assert_eq!(
            format!("{cmd:?}"),
            if cfg!(windows) {
                quoted(&["ls", "--foo", "a b", "additional"])
            } else {
                let sh = *SH;
                format!(r#""{sh}" "-c" "ls --foo \"a b\" \"$@\"" "--" "additional""#)
            },
            "with shell, this works as it performs word splitting"
        );
    }

    #[test]
    fn single_and_complex_arguments_with_auto_split() {
        let cmd = std::process::Command::from(
            gix_command::prepare("ls --foo=\"a b\"").command_may_be_shell_script_allow_manual_argument_splitting(),
        );
        assert_eq!(
            format!("{cmd:?}"),
            format!(r#""ls" "--foo=a b""#),
            "splitting can also handle quotes"
        );
    }

    #[test]
    fn single_and_complex_arguments_without_auto_split() {
        let cmd = std::process::Command::from(
            gix_command::prepare("ls --foo=\"a b\"").command_may_be_shell_script_disallow_manual_argument_splitting(),
        );
        assert_eq!(format!("{cmd:?}"), quoted(&[*SH, "-c", r#"ls --foo=\"a b\""#, "--"]));
    }

    #[test]
    fn single_and_simple_arguments_without_auto_split_with_shell() {
        let cmd = std::process::Command::from(
            gix_command::prepare("ls")
                .arg("--foo=a b")
                .command_may_be_shell_script_disallow_manual_argument_splitting()
                .with_shell(),
        );
        assert_eq!(
            format!("{cmd:?}"),
            quoted(&[*SH, "-c", "ls \\\"$@\\\"", "--", "--foo=a b"])
        );
    }

    #[test]
    fn quoted_command_without_argument_splitting() {
        let cmd = std::process::Command::from(
            gix_command::prepare("ls")
                .arg("--foo=a b")
                .command_may_be_shell_script_disallow_manual_argument_splitting()
                .with_shell()
                .with_quoted_command(),
        );
        assert_eq!(
            format!("{cmd:?}"),
            quoted(&[*SH, "-c", "\\'ls\\' \\\"$@\\\"", "--", "--foo=a b"]),
            "looks strange thanks to debug printing, but is the right amount of quotes actually"
        );
    }

    #[test]
    fn quoted_windows_command_without_argument_splitting() {
        let cmd = std::process::Command::from(
            gix_command::prepare("C:\\Users\\O'Shaughnessy\\with space.exe")
                .arg("--foo='a b'")
                .command_may_be_shell_script_disallow_manual_argument_splitting()
                .with_shell()
                .with_quoted_command(),
        );
        assert_eq!(
            format!("{cmd:?}"),
            quoted(&[
                *SH,
                "-c",
                "\\'C:\\\\Users\\\\O\\'\\\\\\'\\'Shaughnessy\\\\with space.exe\\' \\\"$@\\\"",
                "--",
                "--foo=\\'a b\\'"
            ]),
            "again, a lot of extra backslashes, but it's correct outside of the debug formatting"
        );
    }

    #[test]
    fn single_and_complex_arguments_will_not_auto_split_on_special_characters() {
        let cmd = std::process::Command::from(
            gix_command::prepare("ls --foo=~/path").command_may_be_shell_script_allow_manual_argument_splitting(),
        );
        let sh = *SH;
        assert_eq!(
            format!("{cmd:?}"),
            format!(r#""{sh}" "-c" "ls --foo=~/path" "--""#),
            "splitting can also handle quotes"
        );
    }

    #[test]
    fn tilde_path_and_multiple_arguments_as_part_of_command_with_shell() {
        let cmd =
            std::process::Command::from(gix_command::prepare("~/bin/exe --foo \"a b\"").command_may_be_shell_script());
        let sh = *SH;
        assert_eq!(
            format!("{cmd:?}"),
            format!(r#""{sh}" "-c" "~/bin/exe --foo \"a b\"" "--""#),
            "this always needs a shell as we need tilde expansion"
        );
    }

    #[test]
    fn script_with_dollar_at() {
        let cmd = std::process::Command::from(
            gix_command::prepare("echo \"$@\" >&2")
                .command_may_be_shell_script()
                .arg("store"),
        );
        let sh = *SH;
        assert_eq!(
            format!("{cmd:?}"),
            format!(r#""{sh}" "-c" "echo \"$@\" >&2" "--" "store""#),
            "this is how credential helpers have to work as for some reason they don't get '$@' added in Git.\
            We deal with it by not doubling the '$@' argument, which seems more flexible."
        );
    }

    #[test]
    fn script_with_dollar_at_has_no_quoting() {
        let cmd = std::process::Command::from(
            gix_command::prepare("echo \"$@\" >&2")
                .command_may_be_shell_script()
                .with_quoted_command()
                .arg("store"),
        );
        let sh = *SH;
        assert_eq!(
            format!("{cmd:?}"),
            format!(r#""{sh}" "-c" "echo \"$@\" >&2" "--" "store""#)
        );
    }
}

mod spawn {
    use bstr::ByteSlice;

    #[test]
    fn environment_variables_are_passed_one_by_one() -> crate::Result {
        let out = gix_command::prepare("echo $FIRST $SECOND")
            .env("FIRST", "first")
            .env("SECOND", "second")
            .command_may_be_shell_script()
            .spawn()?
            .wait_with_output()?;
        assert_eq!(out.stdout.as_bstr(), "first second\n");
        Ok(())
    }

    #[test]
    fn disallow_shell() -> crate::Result {
        let out = gix_command::prepare("PATH= echo hi")
            .command_may_be_shell_script_disallow_manual_argument_splitting()
            .spawn()?
            .wait_with_output()?;
        assert_eq!(out.stdout.as_bstr(), "hi\n");

        let mut cmd: std::process::Command = gix_command::prepare("echo hi")
            .command_may_be_shell_script()
            .without_shell()
            .into();
        assert!(
            cmd.env_remove("PATH").spawn().is_err(),
            "no command named 'echo hi' exists"
        );
        Ok(())
    }

    #[test]
    fn script_with_dollar_at() -> crate::Result {
        let out = std::process::Command::from(
            gix_command::prepare("echo \"$@\"")
                .command_may_be_shell_script()
                .arg("arg"),
        )
        .spawn()?
        .wait_with_output()?;
        assert_eq!(
            out.stdout.to_str_lossy().trim(),
            "arg",
            "the argument is just mentioned once"
        );
        Ok(())
    }

    #[test]
    fn direct_command_execution_searches_in_path() -> crate::Result {
        assert!(gix_command::prepare(if cfg!(unix) { "ls" } else { "dir.exe" })
            .spawn()?
            .wait()?
            .success());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn direct_command_with_absolute_command_path() -> crate::Result {
        assert!(gix_command::prepare("/usr/bin/env").spawn()?.wait()?.success());
        Ok(())
    }

    mod with_shell {
        use gix_testtools::bstr::ByteSlice;

        #[test]
        fn command_in_path_with_args() -> crate::Result {
            assert!(gix_command::prepare(if cfg!(unix) { "ls -l" } else { "dir.exe -a" })
                .command_may_be_shell_script()
                .spawn()?
                .wait()?
                .success());
            Ok(())
        }

        #[test]
        fn sh_shell_specific_script_code() -> crate::Result {
            assert!(gix_command::prepare(":;:;:")
                .command_may_be_shell_script()
                .spawn()?
                .wait()?
                .success());
            Ok(())
        }

        #[test]
        fn sh_shell_specific_script_code_with_single_extra_arg() -> crate::Result {
            let out = gix_command::prepare("printf")
                .command_may_be_shell_script()
                .arg("1")
                .spawn()?
                .wait_with_output()?;
            assert!(out.status.success());
            assert_eq!(out.stdout.as_bstr(), "1");
            Ok(())
        }

        #[test]
        fn sh_shell_specific_script_code_with_multiple_extra_args() -> crate::Result {
            let out = gix_command::prepare("printf")
                .command_may_be_shell_script()
                .arg("%s")
                .arg("arg")
                .spawn()?
                .wait_with_output()?;
            assert!(out.status.success());
            assert_eq!(out.stdout.as_bstr(), "arg");
            Ok(())
        }
    }
}
