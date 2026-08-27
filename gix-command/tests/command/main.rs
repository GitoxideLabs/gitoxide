use std::path::Path;

use gix_testtools::Result;

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

        fn exe_args(name: &str, args: &[&str]) -> Option<shebang::Data> {
            shebang::Data {
                interpreter: name.into(),
                args: args.iter().map(Into::into).collect(),
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
                parse(r"#!\bin\sh"),
                exe(r"\bin\sh"),
                "backslashes are recognized as path separator"
            );
            assert_eq!(
                parse("#!C:\\Program Files\\shell.exe\r\nsome stuff"),
                exe(r"C:\Program Files\shell.exe"),
                "absolute windows paths are fine"
            );
            assert_eq!(
                parse("#!/bin/sh -i -o -u\nunrelated content"),
                exe_args("/bin/sh", &["-i", "-o", "-u"]),
                "all interpreter options are returned"
            );
            assert_eq!(
                parse("#!/bin/sh  -o\nunrelated content"),
                exe_args("/bin/sh", &["-o"]),
                "single interpreter options are returned too"
            );
            assert_eq!(
                parse("#!/bin/exe anything goes\nunrelated content"),
                exe_args("/bin/exe", &["anything", "goes"]),
                "the entire shebang suffix is returned"
            );
            assert_eq!(
                parse("#!/usr/bin/env -S FOO=bar command"),
                exe_args("/usr/bin/env", &["-S", "FOO=bar", "command"]),
                "assignment-looking arguments are retained"
            );
            assert_eq!(
                parse(r#"#!/usr/bin/env -S "command name" plain"#),
                exe_args("/usr/bin/env", &["-S", "command name", "plain"]),
                "interpreter arguments use command-line quoting"
            );
            assert_eq!(
                parse("#!/bin/sh O'Brien"),
                exe("/bin/sh"),
                "malformed interpreter options do not hide a valid interpreter"
            );

            use bstr::ByteSlice;

            #[cfg(unix)]
            {
                use std::os::unix::ffi::OsStrExt;

                assert_eq!(
                    shebang::parse(b"#!/bin/sh   -x \xC3\x28\x41 -y  ".as_bstr()),
                    Some(shebang::Data {
                        interpreter: "/bin/sh".into(),
                        args: vec![
                            "-x".into(),
                            std::ffi::OsStr::from_bytes(b"\xC3\x28\x41").to_owned(),
                            "-y".into(),
                        ]
                    }),
                    "non-UTF-8 interpreter arguments are returned losslessly"
                );
            }

            #[cfg(not(unix))]
            assert_eq!(
                shebang::parse(b"#!/bin/sh   -x \xC3\x28\x41 -y  ".as_bstr()),
                exe("/bin/sh"),
                "unrepresentable interpreter arguments do not hide a valid interpreter"
            );

            #[cfg(unix)]
            {
                use std::os::unix::ffi::OsStrExt;

                assert_eq!(
                    shebang::parse(b"#!/bin/\xC3\x28\x41 ".as_bstr()),
                    Some(shebang::Data {
                        interpreter: std::ffi::OsStr::from_bytes(b"/bin/\xC3\x28\x41").to_owned().into(),
                        args: vec![]
                    }),
                    "illformed UTF8 in the executable path is fine as well"
                );
            }

            #[cfg(not(unix))]
            {
                assert_eq!(
                    shebang::parse(b"#!/bin/\xC3\x28\x41 ".as_bstr()),
                    None,
                    "an unrepresentable interpreter invalidates the shebang"
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

mod command_line;
mod context;
mod prepare;
mod spawn;
