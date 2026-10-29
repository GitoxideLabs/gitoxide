use std::{path::Path, process::Stdio};

use bstr::{BString, ByteSlice};
use gix_credentials::{
    Program,
    helper::{self, Action},
    protocol::Context,
};
use gix_testtools::TestResult;

fn program(path: &Path) -> gix_error::Result<Program> {
    let path = gix_path::to_unix_separators_on_windows(gix_path::into_bstr(path)?);
    let mut definition = BString::from("store --file ");
    definition.extend_from_slice(&gix_quote::single(path.as_ref()));
    Program::from_custom_definition(definition)
}

fn reference(path: &Path, action: &Action) -> TestResult<Vec<u8>> {
    let mut child = gix_testtools::git_command(path.parent().expect("file has a parent"))
        .args(["credential-store", "--file"])
        .arg(path)
        .arg(action.as_arg(true))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    action.send(&mut child.stdin.take().expect("stdin is piped"))?;
    let output = child.wait_with_output()?;
    assert!(output.status.success(), "reference helper succeeds: {output:?}");
    Ok(output.stdout)
}

#[test]
fn explicit_store_matches_git_without_executing_git() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let dir = gix_testtools::tempfile::tempdir()?;
    let actual = dir.path().join("our credentials 'with $syntax;");
    let expected = dir.path().join("git credentials");
    let initial = b"# keep malformed entries
https://old:password@example.com
https://wrong:no@dos.example\r
https://path:pass@dos.example/repo.git\r
https://:empty@example.org
no-final-newline";
    std::fs::write(&actual, initial)?;
    std::fs::write(&expected, initial)?;
    let mut helper = program(Path::new(actual.file_name().expect("file has a name")))?;
    helper.current_dir = Some(dir.path().to_owned());
    let context = Context {
        protocol: Some("https".into()),
        host: Some("example.com".into()),
        username: Some("new user".into()),
        password: Some("p@ss:word%".into()),
        ..Default::default()
    };
    for action in [
        Action::Get(Context::default()),
        Action::Erase(
            b"protocol=https
host=example.com
username=old
credential=separate-token
"
            .as_bstr()
            .into(),
        ),
        Action::Store(context.to_bstring()),
        Action::Get(Context {
            password: Some("ignored for lookup".into()),
            ..context.clone()
        }),
        Action::Get(Context {
            username: Some("no match".into()),
            ..context.clone()
        }),
        Action::Store(
            Context {
                password: None,
                ..context.clone()
            }
            .to_bstring(),
        ),
        Action::Store(
            Context {
                password: Some("replacement".into()),
                ..context.clone()
            }
            .to_bstring(),
        ),
        Action::Erase(context.to_bstring()),
        Action::Erase(
            Context {
                password: Some("replacement".into()),
                ..context.clone()
            }
            .to_bstring(),
        ),
        Action::Erase(Context::default().to_bstring()),
        Action::Erase(
            Context {
                password: Some("password alone cannot erase".into()),
                ..Default::default()
            }
            .to_bstring(),
        ),
        Action::Get(Context {
            host: Some("dos.example".into()),
            ..Default::default()
        }),
        Action::Get(Context {
            host: Some("dos.example".into()),
            path: Some("repo.git".into()),
            ..Default::default()
        }),
        Action::Get(Context {
            host: Some("example.org".into()),
            ..Default::default()
        }),
        Action::Store(
            b"protocol=discarded
url=custom://u:p@EXAMPLE%3a123/a%3Fb/
password=override
"
            .as_bstr()
            .into(),
        ),
        Action::get_for_url("custom://u@EXAMPLE%3a123/a%3Fb"),
        Action::Store(
            Context {
                host: Some("[::1]:443".into()),
                path: Some(b"byte-\xff/path?x=[]@!+$,%".as_bstr().into()),
                ..context.clone()
            }
            .to_bstring(),
        ),
        Action::Get(Context {
            host: Some("[::1]:443".into()),
            path: Some(b"byte-\xff/path?x=[]@!+$,%".as_bstr().into()),
            ..context.clone()
        }),
        Action::Store(
            Context {
                password: Some("embedded\rreturn".into()),
                options: gix_credentials::protocol::ContextOptions {
                    protect_protocol: false,
                },
                ..context.clone()
            }
            .to_bstring(),
        ),
        Action::Get(Context {
            options: gix_credentials::protocol::ContextOptions {
                protect_protocol: false,
            },
            ..context.clone()
        }),
        Action::Store(
            Context {
                username: Some("".into()),
                password: Some("".into()),
                ..context.clone()
            }
            .to_bstring(),
        ),
        Action::Get(Context {
            username: Some("".into()),
            ..context.clone()
        }),
    ] {
        let reference = reference(&expected, &action)?;
        let _environment = gix_testtools::Env::new().set("PATH", dir.path().display().to_string());
        let outcome = helper::invoke(&mut helper, &action)?;
        if let Some(outcome) = outcome {
            let expected =
                Context::from_bytes(&reference, action.context().map(|ctx| ctx.options).unwrap_or_default())?;
            assert_eq!(outcome.username, expected.username, "username matches Git");
            assert_eq!(outcome.password, expected.password, "password matches Git");
        }
        assert_eq!(
            std::fs::read(&actual)?,
            std::fs::read(&expected)?,
            "stored bytes match Git"
        );
    }
    Ok(())
}

fn default_reference(root: &Path, action: &Action) -> TestResult<Vec<u8>> {
    let mut child = gix_testtools::git_command(root)
        .env("HOME", root)
        .env("XDG_CONFIG_HOME", root.join("xdg"))
        .args(["credential-store", action.as_arg(true)])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    action.send(&mut child.stdin.take().expect("stdin is piped"))?;
    let output = child.wait_with_output()?;
    assert!(output.status.success(), "reference helper succeeds: {output:?}");
    Ok(output.stdout)
}

#[test]
fn default_files_follow_git_precedence_for_lookup_store_and_erase() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let temp = gix_testtools::tempfile::tempdir()?;
    let actual = temp.path().join("actual");
    let expected = temp.path().join("expected");
    std::fs::create_dir(&actual)?;
    std::fs::create_dir(&expected)?;
    let context = Context {
        protocol: Some("https".into()),
        host: Some("example.com".into()),
        username: Some("user".into()),
        password: Some("secret".into()),
        ..Default::default()
    };
    let mut helper = Program::from_custom_definition("store")?;
    for phase in 0..3 {
        for root in [&actual, &expected] {
            if phase == 1 {
                std::fs::remove_file(root.join(".git-credentials"))?;
                std::fs::create_dir_all(root.join("xdg/git"))?;
                std::fs::write(root.join("xdg/git/credentials"), b"https://xdg:pass@example.com\n")?;
            } else if phase == 2 {
                std::fs::write(root.join(".git-credentials"), b"https://home:pass@example.com\n")?;
            }
        }
        for action in [
            Action::Get(Context {
                host: context.host.clone(),
                ..Default::default()
            }),
            Action::Store(context.to_bstring()),
            Action::Get(Context {
                host: context.host.clone(),
                ..Default::default()
            }),
            Action::Erase(
                Context {
                    username: context.username.clone(),
                    ..Default::default()
                }
                .to_bstring(),
            ),
        ] {
            let reference = default_reference(&expected, &action)?;
            let _environment = gix_testtools::Env::new()
                .set("HOME", actual.display().to_string())
                .set("XDG_CONFIG_HOME", actual.join("xdg").display().to_string())
                .set("PATH", temp.path().display().to_string());
            let outcome = helper::invoke(&mut helper, &action)?;
            if let Some(outcome) = outcome {
                let expected =
                    Context::from_bytes(&reference, action.context().map(|ctx| ctx.options).unwrap_or_default())?;
                assert_eq!(outcome.username, expected.username, "lookup order matches Git");
                assert_eq!(outcome.password, expected.password, "lookup credentials match Git");
            }
            for file in [".git-credentials", "xdg/git/credentials"] {
                assert_eq!(
                    actual.join(file).exists(),
                    expected.join(file).exists(),
                    "store creates the same file as Git"
                );
                if actual.join(file).exists() {
                    assert_eq!(
                        std::fs::read(actual.join(file))?,
                        std::fs::read(expected.join(file))?,
                        "store and erase use Git's files"
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn store_locking_preserves_existing_secrets_and_errors_are_redacted() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let temp = gix_testtools::tempfile::tempdir()?;
    let path = temp.path().join("credentials");
    let original = b"https://user:original@example.com\n";
    std::fs::write(&path, original)?;
    std::fs::write(path.with_extension("lock"), b"held")?;
    let mut helper = program(&path)?;
    helper.store_lock_timeout = gix_lock::acquire::Fail::Immediately;
    let context = Context {
        protocol: Some("https".into()),
        host: Some("example.com".into()),
        username: Some("user".into()),
        password: Some("unprintable-secret".into()),
        ..Default::default()
    };
    let err = helper::invoke(&mut helper, &Action::Store(context.to_bstring()))
        .expect_err("an existing lock excludes writers");
    assert!(
        !format!("{err:?}").contains("unprintable-secret"),
        "errors never include credential contents"
    );
    assert_eq!(
        std::fs::read(&path)?,
        original,
        "failed acquisition cannot alter the store"
    );
    std::fs::remove_file(path.with_extension("lock"))?;
    helper::invoke(&mut helper, &Action::Store(context.to_bstring()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path)?.permissions().mode() & 0o777,
            0o600,
            "credential files are private"
        );
    }
    let invalid = Action::Get(Context {
        password: Some("unprintable-secret\ninvalid".into()),
        ..context
    });
    let err = helper::invoke(&mut helper, &invalid).expect_err("protocol injection is rejected");
    assert!(
        !format!("{err:?}").contains("unprintable-secret"),
        "input validation errors also redact values"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn store_follows_symlinks_and_failed_helpers_allow_fallback() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let temp = gix_testtools::tempfile::tempdir()?;
    let actual = temp.path().join("actual");
    let expected = temp.path().join("expected");
    for (link, target) in [(&actual, "actual-target"), (&expected, "expected-target")] {
        std::fs::write(temp.path().join(target), b"https://user:old@example.com\n")?;
        std::os::unix::fs::symlink(target, link)?;
    }
    let action = Action::Store(
        b"protocol=https
host=example.com
username=user
password=updated
"
        .as_bstr()
        .into(),
    );
    reference(&expected, &action)?;
    helper::invoke(&mut program(&actual)?, &action)?;
    assert!(
        actual.symlink_metadata()?.is_symlink(),
        "the credential-file link is retained"
    );
    assert_eq!(
        std::fs::read(actual)?,
        std::fs::read(expected)?,
        "the link target changes like Git"
    );
    let mut cascade = helper::Cascade::default().extend([
        program(temp.path())?,
        Program::from_custom_definition("!printf 'username=fallback\npassword=pass\n'")?,
    ]);
    let outcome = cascade
        .invoke(
            Action::get_for_url("https://example.com"),
            gix_prompt::Options {
                mode: gix_prompt::Mode::Disable,
                ..Default::default()
            },
        )?
        .expect("fallback helper completes credentials");
    assert_eq!(
        outcome.identity.username, "fallback",
        "a broken native store permits trying the next helper"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn write_only_credentials_are_preserved_when_the_store_cannot_read_them() -> TestResult {
    use std::os::unix::fs::PermissionsExt;

    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let temp = gix_testtools::tempfile::tempdir()?;
    let path = temp.path().join("credentials");
    let original = b"https://user:existing@example.com\n";
    let context = Context {
        protocol: Some("https".into()),
        host: Some("example.com".into()),
        username: Some("user".into()),
        password: Some("new-secret".into()),
        ..Default::default()
    };
    for action in [Action::Store(context.to_bstring()), Action::Erase(context.to_bstring())] {
        std::fs::write(&path, original)?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o200))?;
        if std::fs::read(&path).is_ok() {
            return Ok(()); // Privileged accounts can bypass Unix read permissions.
        }
        let result = helper::invoke(&mut program(&path)?, &action);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        assert_eq!(
            std::fs::read(&path)?,
            original,
            "an unreadable store must not lose existing credentials"
        );
        assert!(
            result.is_err(),
            "read failure aborts the update before committing its lock"
        );
        assert!(
            !path.with_extension("lock").exists(),
            "the failed update releases its lock"
        );
    }
    Ok(())
}

#[test]
fn operation_environment_selects_native_store_files() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let tmp = gix_testtools::tempfile::tempdir()?;
    let process_home_dir = tmp.path().join("process-home");
    let process_xdg_dir = tmp.path().join("process-xdg");
    std::fs::create_dir(&process_home_dir)?;
    std::fs::create_dir(&process_xdg_dir)?;
    let _process_environment = gix_testtools::Env::new()
        .set("HOME", process_home_dir.to_str().expect("temporary path is Unicode"))
        .set(
            "XDG_CONFIG_HOME",
            process_xdg_dir.to_str().expect("temporary path is Unicode"),
        );
    let home = tmp.path().join("home");
    let xdg = tmp.path().join("xdg");
    let xdg_file = xdg.join("git/credentials");
    std::fs::create_dir(&home)?;
    std::fs::create_dir_all(xdg_file.parent().expect("file has a parent"))?;
    std::fs::write(&xdg_file, b"")?;
    let process_home = std::env::var_os("HOME");
    let process_xdg = std::env::var_os("XDG_CONFIG_HOME");
    let environment = [
        ("HOME".into(), tmp.path().join("unused").into_os_string()),
        ("HOME".into(), "home".into()),
        ("XDG_CONFIG_HOME".into(), "xdg".into()),
        ("PATH".into(), tmp.path().join("no-programs").into_os_string()),
    ];
    let mut program = Program::from_custom_definition("store")?;
    program.current_dir = Some(tmp.path().to_owned());
    let mut cascade = helper::Cascade {
        programs: vec![program],
        ..Default::default()
    };
    let prompt = gix_prompt::Options {
        mode: gix_prompt::Mode::Disable,
        ..Default::default()
    };
    let context = Context {
        protocol: Some("https".into()),
        host: Some("example.com".into()),
        username: Some("user".into()),
        password: Some("secret".into()),
        ..Default::default()
    };
    for file in [&xdg_file, &home.join(".git-credentials")] {
        cascade.invoke_with_environment(Action::Store(context.to_bstring()), prompt.clone(), &environment)?;
        assert_eq!(
            std::fs::read(file)?,
            b"https://user:secret@example.com\n",
            "native storage uses the operation's paths"
        );
        let result = cascade
            .invoke_with_environment(
                Action::Get(Context {
                    password: None,
                    ..context.clone()
                }),
                prompt.clone(),
                &environment,
            )?
            .expect("stored credentials are available");
        assert_eq!(
            result.identity.password, "secret",
            "lookup uses the same operation environment"
        );
        cascade.invoke_with_environment(Action::Erase(context.to_bstring()), prompt.clone(), &environment)?;
        assert_eq!(std::fs::read(file)?, b"", "erase uses the same operation environment");
        std::fs::remove_file(file)?;
    }
    assert_eq!(
        std::env::var_os("HOME"),
        process_home,
        "operation environment does not escape the call"
    );
    assert_eq!(
        std::env::var_os("XDG_CONFIG_HOME"),
        process_xdg,
        "operation environment does not escape the call"
    );
    Ok(())
}
