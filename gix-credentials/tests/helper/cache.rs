use std::path::Path;

use bstr::BString;
use gix_credentials::{
    Program,
    helper::{self, Action},
    protocol::Context,
};
use gix_testtools::TestResult;

fn program(socket: &Path) -> gix_error::Result<Program> {
    let mut definition = BString::from("cache --socket ");
    definition.extend_from_slice(&gix_quote::single(gix_path::into_bstr(socket)?.as_ref()));
    Program::from_custom_definition(definition)
}

#[test]
fn absent_daemon_lookup_and_erase_need_no_git() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let dir = gix_testtools::tempfile::tempdir()?;
    let socket = dir.path().join("socket");
    let mut helper = program(&socket)?;
    let _environment = gix_testtools::Env::new().set("PATH", dir.path().display().to_string());
    let outcome =
        helper::invoke(&mut helper, &Action::Get(Context::default()))?.expect("Get returns the empty helper response");
    assert_eq!(outcome.username, None, "an absent daemon has no credentials");
    assert!(helper::invoke(&mut helper, &Action::Erase(BString::default()))?.is_none());
    assert!(!socket.exists(), "reads and erase do not start a daemon");
    Ok(())
}

use gix_credentials::program::cache::{self, Daemon};
use std::{
    io::{Read, Write},
    path::PathBuf,
    process::{Child, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
};

struct Running {
    socket: PathBuf,
    interrupt: Arc<AtomicBool>,
    thread: Option<JoinHandle<gix_error::Result<()>>>,
    child: Option<Child>,
}

impl Running {
    fn native(socket: PathBuf) -> TestResult<Self> {
        let daemon = Daemon::bind(&socket)?;
        let interrupt = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&interrupt);
        Ok(Running {
            socket,
            interrupt,
            thread: Some(std::thread::spawn(move || daemon.run(&flag))),
            child: None,
        })
    }

    fn git(socket: PathBuf) -> TestResult<Self> {
        let mut child = gix_testtools::git_command(socket.parent().expect("socket parent exists"))
            .arg("credential-cache--daemon")
            .arg(&socket)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let mut ready = Vec::new();
        child.stdout.take().expect("stdout is piped").read_to_end(&mut ready)?;
        assert_eq!(ready, b"ok\n", "Git daemon announces readiness and closes stdout");
        Ok(Running {
            socket,
            interrupt: Arc::new(AtomicBool::new(false)),
            thread: None,
            child: Some(child),
        })
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        cache::request(&self.socket, "exit", 900, b"", None).ok();
        self.interrupt.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            assert!(thread.join().expect("cache daemon did not panic").is_ok());
        }
        if let Some(mut child) = self.child.take() {
            child.kill().ok();
            child.wait().ok();
        }
    }
}

fn reference(socket: &Path, action: &str, timeout: i32, input: &[u8]) -> TestResult<Vec<u8>> {
    let mut child = gix_testtools::git_command(socket.parent().expect("socket parent exists"))
        .args(["credential-cache", "--socket"])
        .arg(socket)
        .arg(format!("--timeout={timeout}"))
        .arg(action)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child.stdin.take().expect("stdin is piped").write_all(input)?;
    let output = child.wait_with_output()?;
    assert!(output.status.success(), "Git cache client succeeds: {output:?}");
    Ok(output.stdout)
}

#[test]
fn native_and_git_daemons_interoperate_with_both_clients() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let directory = gix_testtools::tempfile::tempdir()?;
    let native = Running::native(directory.path().join("native/socket"))?;
    std::fs::create_dir(directory.path().join("reference"))?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(
        directory.path().join("reference"),
        std::fs::Permissions::from_mode(0o700),
    )?;
    let git = Running::git(directory.path().join("reference/socket"))?;
    let actions: &[(&str, &[u8])] = &[
        ("get", b""),
        ("store", b"protocol=https\nhost=example.com\nusername=alice\npassword=first\npassword_expiry_utc=9999999999\noauth_refresh_token=refresh\n"),
        ("get", b"protocol=https\nhost=example.com\npassword=ignored\n"),
        ("get", b"protocol=http\nhost=example.com\n"),
        ("get", b"protocol=https\nhost=other\n"),
        ("get", b"username=bob\n"),
        ("erase", b"host=example.com\npassword=wrong\n"),
        ("get", b"host=example.com\n"),
        ("store", b"protocol=https\nhost=example.com\nusername=alice\npassword=second\n"),
        ("get", b"host=example.com\n"),
        ("store", b"protocol=https\nhost=example.com\nusername=bob\npassword=other-user\npath=repo\n"),
        ("get", b"username=bob\npath=other\n"),
        ("get", b"username=bob\npath=repo\n"),
        ("erase", b"username=alice\npassword=second\n"),
        ("get", b"username=bob\n"),
        ("store", b"protocol=https\nhost=empty\nusername=\npassword=\n"),
        ("get", b"host=empty\n"),
        ("store", b"protocol=https\nhost=partial\nusername=alice\n"),
        ("get", b"host=partial\n"),
        ("store", b"protocol=https\nhost=ephemeral\nusername=alice\npassword=secret\nephemeral=true\n"),
        ("get", b"host=ephemeral\n"),
        ("store", b"protocol=https\nhost=opaque\nauthtype=Bearer\ncredential=opaque-token\n"),
        ("get", b"host=opaque\n"),
        ("get", b"capability[]=authtype\nhost=opaque\n"),
        ("erase", b"host=opaque\ncredential=wrong\n"),
        ("get", b"capability[]=authtype\nhost=opaque\n"),
        ("erase", b"host=opaque\ncredential=opaque-token\n"),
        ("get", b"capability[]=authtype\nhost=opaque\n"),
        ("store", b"username=discarded\nurl=https://url%ff:pass%ff@Case.Example:443/a%ff/b///\npassword=later\n"),
        ("get", b"host=Case.Example:443\npath=a\xff/b\n"),
        ("erase", b""),
        ("get", b""),
    ];
    for (index, (action, input)) in actions.iter().enumerate() {
        let expected = reference(&git.socket, action, 900, input)?;
        let actual = if index % 2 == 0 {
            let _environment = gix_testtools::Env::new().set("PATH", directory.path().display().to_string());
            cache::request(&native.socket, action, 900, input, None)?
        } else {
            reference(&native.socket, action, 900, input)?
        };
        assert_eq!(actual, expected, "action {index} {action} agrees with Git");
        let native_client_to_git = cache::request(&git.socket, "get", 900, b"", None)?;
        let native_client_to_native = cache::request(&native.socket, "get", 900, b"", None)?;
        assert_eq!(
            native_client_to_native, native_client_to_git,
            "cache state after action {index}"
        );
    }
    Ok(())
}

#[test]
fn socket_permissions_stale_files_and_exit_cleanup() -> TestResult {
    // Concurrent process creation can temporarily inherit a listener after this process closes it.
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    use std::os::unix::fs::PermissionsExt;
    let directory = gix_testtools::tempfile::tempdir()?;
    let socket = directory.path().join("private/socket");
    let native = Running::native(socket.clone())?;
    assert_eq!(
        std::fs::metadata(socket.parent().expect("socket parent"))?
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert!(
        Daemon::bind(&socket).is_err(),
        "a second daemon must not replace a live socket"
    );
    cache::request(&socket, "exit", 900, b"", None)?;
    assert!(!socket.exists(), "exit removes the socket before reporting completion");
    drop(native);
    let listener = std::os::unix::net::UnixListener::bind(&socket)?;
    drop(listener);
    drop(Daemon::bind(&socket)?);
    assert!(
        !socket.exists(),
        "a stale socket can be replaced and is removed on drop"
    );
    std::fs::write(&socket, b"keep")?;
    assert!(Daemon::bind(&socket).is_err(), "unrelated files must survive");
    assert_eq!(std::fs::read(&socket)?, b"keep");
    std::fs::remove_file(&socket)?;
    std::fs::set_permissions(
        socket.parent().expect("socket parent"),
        std::fs::Permissions::from_mode(0o755),
    )?;
    assert!(Daemon::bind(&socket).is_err(), "a public socket directory is rejected");
    assert!(Daemon::bind(Path::new("relative/socket")).is_err());
    Ok(())
}

#[test]
fn timeout_and_missing_launcher() -> TestResult {
    let directory = gix_testtools::tempfile::tempdir()?;
    let socket = directory.path().join("private/socket");
    let error = cache::request(&socket, "store", 1, b"username=user\npassword=secret\n", None)
        .expect_err("an absent daemon needs an explicit launcher");
    assert!(format!("{error:?}").contains("no daemon launcher"));
    assert!(!format!("{error:?}").contains("secret"));
    let _native = Running::native(socket.clone())?;
    cache::request(&socket, "store", 1, b"username=user\npassword=secret\n", None)?;
    assert!(!cache::request(&socket, "get", 900, b"", None)?.is_empty());
    std::thread::sleep(std::time::Duration::from_millis(1100));
    assert!(
        cache::request(&socket, "get", 900, b"", None)?.is_empty(),
        "expired credentials are not returned"
    );
    assert!(socket.exists(), "empty cache retains its grace period");
    Ok(())
}

#[test]
fn explicit_launcher_can_start_git_daemon_and_survives_client() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let directory = gix_testtools::tempfile::tempdir()?;
    let socket = directory.path().join("private/socket");
    let git = gix_testtools::git_command(directory.path());
    let mut helper = program(&socket)?;
    helper.cache_daemon = Some(cache::DaemonCommand {
        executable: git.get_program().into(),
        args: vec!["credential-cache--daemon".into()],
    });
    helper::invoke(
        &mut helper,
        &Action::Store(b"username=user\npassword=secret\n".as_slice().into()),
    )?;
    drop(helper);
    let cleanup = Running {
        socket: socket.clone(),
        interrupt: Arc::new(AtomicBool::new(false)),
        thread: None,
        child: None,
    };
    let _environment = gix_testtools::Env::new().set("PATH", directory.path().display().to_string());
    let outcome = helper::invoke(&mut program(&socket)?, &Action::Get(Context::default()))?.expect("response present");
    assert_eq!(
        outcome.password.as_deref(),
        Some("secret"),
        "a fresh client reaches the persistent daemon without Git"
    );
    drop(cleanup);
    assert!(!socket.exists());
    Ok(())
}

#[test]
fn failed_startup_does_not_leak_child_output() -> TestResult {
    let directory = gix_testtools::tempfile::tempdir()?;
    let socket = directory.path().join("socket");
    for command in [
        cache::DaemonCommand {
            executable: directory.path().join("missing"),
            args: vec![],
        },
        cache::DaemonCommand {
            executable: gix_path::env::shell().into(),
            args: vec!["-c".into(), "printf 'sensitive startup output'".into()],
        },
    ] {
        let error = cache::request(
            &socket,
            "store",
            900,
            b"username=user\npassword=secret\n",
            Some(&command),
        )
        .expect_err("failed startup cannot store credentials");
        let text = format!("{error:?}");
        assert!(
            !text.contains("sensitive"),
            "unexpected daemon output is not included in errors"
        );
        assert!(!text.contains("secret"), "credential input is not included in errors");
    }
    Ok(())
}

#[test]
fn default_paths_and_static_helper_options() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let directory = gix_testtools::tempfile::tempdir()?;
    let home = directory.path().join("home");
    let xdg = directory.path().join("xdg");
    std::fs::create_dir(&home)?;
    let _environment = gix_testtools::Env::new()
        .set("HOME", home.display().to_string())
        .set("XDG_CACHE_HOME", "")
        .set("PATH", directory.path().display().to_string());
    assert_eq!(cache::default_socket()?, home.join(".cache/git/credential/socket"));
    {
        let _xdg = gix_testtools::Env::new().set("XDG_CACHE_HOME", xdg.display().to_string());
        assert_eq!(cache::default_socket()?, xdg.join("git/credential/socket"));
        std::fs::create_dir(home.join(".git-credential-cache"))?;
        assert_eq!(cache::default_socket()?, home.join(".git-credential-cache/socket"));
    }
    let mut helper = Program::from_custom_definition("cache --timeout=1 --no-timeout --no-socket")?;
    helper::invoke(&mut helper, &Action::Get(Context::default()))?;
    for definition in [
        "cache --timeout",
        "cache --timeout=invalid",
        "cache --socket",
        "cache --unknown",
        "cache --socket=relative",
    ] {
        assert!(
            helper::invoke(
                &mut Program::from_custom_definition(definition)?,
                &Action::Get(Context::default())
            )
            .is_err(),
            "invalid option {definition} fails"
        );
    }
    Ok(())
}

#[test]
fn malformed_request_does_not_stop_daemon_or_store_partial_input() -> TestResult {
    use std::net::Shutdown;
    use std::os::unix::net::UnixStream;
    let directory = gix_testtools::tempfile::tempdir()?;
    let native = Running::native(directory.path().join("private/socket"))?;
    for input in [
        b"bogus\ntimeout=1\n".as_slice(),
        b"action=store\ntimeout=invalid\nusername=user\npassword=secret\n",
        b"action=store\ntimeout=900\nusername=user\npassword=secret\ninvalid\n",
    ] {
        let mut stream = UnixStream::connect(&native.socket)?;
        stream.write_all(input)?;
        stream.shutdown(Shutdown::Write)?;
        let mut response = Vec::new();
        stream.read_to_end(&mut response)?;
        assert!(response.is_empty());
        assert!(cache::request(&native.socket, "get", 900, b"", None)?.is_empty());
    }
    Ok(())
}

#[test]
fn exit_preserves_a_replacement_at_the_original_socket_path() -> TestResult {
    let directory = gix_testtools::tempfile::tempdir()?;
    let socket = directory.path().join("private/socket");
    let native = Running::native(socket.clone())?;
    let moved = socket.with_file_name("moved");
    std::fs::rename(&socket, &moved)?;
    std::fs::write(&socket, b"unrelated replacement")?;
    cache::request(&moved, "exit", 900, b"", None)?;
    assert_eq!(
        std::fs::read(&socket)?,
        b"unrelated replacement",
        "exit must only remove the daemon's own socket"
    );
    drop(native);
    Ok(())
}

#[test]
fn operation_environment_selects_native_cache_socket() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let directory = gix_testtools::tempfile::tempdir()?;
    let process_home = directory.path().join("process-home");
    let process_xdg = directory.path().join("process-xdg");
    std::fs::create_dir(&process_home)?;
    std::fs::create_dir(&process_xdg)?;
    let _process_environment = gix_testtools::Env::new()
        .set("HOME", process_home.to_str().expect("temporary path is Unicode"))
        .set(
            "XDG_CACHE_HOME",
            process_xdg.to_str().expect("temporary path is Unicode"),
        );
    let home = directory.path().join("home");
    let xdg = directory.path().join("xdg");
    std::fs::create_dir(&home)?;
    for (socket, cache_home) in [
        (xdg.join("git/credential/socket"), xdg.clone().into_os_string()),
        (home.join(".cache/git/credential/socket"), "".into()),
        (home.join(".git-credential-cache/socket"), xdg.into_os_string()),
    ] {
        let native = Running::native(socket)?;
        cache::request(
            &native.socket,
            "store",
            900,
            b"protocol=https\nhost=example.com\nusername=user\npassword=secret\n",
            None,
        )?;
        let mut cascade = helper::Cascade::default().extend([Program::from_custom_definition("cache")?]);
        let result = cascade
            .invoke_with_environment(
                Action::get_for_url("https://example.com"),
                gix_prompt::Options {
                    mode: gix_prompt::Mode::Disable,
                    ..Default::default()
                },
                &[
                    ("HOME".into(), process_home.clone().into_os_string()),
                    ("HOME".into(), home.clone().into_os_string()),
                    ("XDG_CACHE_HOME".into(), cache_home),
                ],
            )?
            .expect("the operation's socket provides credentials");
        assert_eq!(
            result.identity.password, "secret",
            "HOME, XDG and legacy precedence use the operation environment"
        );
    }
    assert_eq!(
        std::env::var_os("HOME"),
        Some(process_home.into_os_string()),
        "HOME remains local to the operation"
    );
    assert_eq!(
        std::env::var_os("XDG_CACHE_HOME"),
        Some(process_xdg.into_os_string()),
        "XDG remains local to the operation"
    );
    Ok(())
}

#[test]
fn operation_environment_reaches_cache_daemon_launcher() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let directory = gix_testtools::tempfile::tempdir()?;
    let process_home = directory.path().join("process-home");
    let process_xdg = directory.path().join("process-xdg");
    std::fs::create_dir(&process_home)?;
    std::fs::create_dir(&process_xdg)?;
    let _process_environment = gix_testtools::Env::new()
        .set("HOME", process_home.to_str().expect("temporary path is Unicode"))
        .set(
            "XDG_CACHE_HOME",
            process_xdg.to_str().expect("temporary path is Unicode"),
        );
    let home = directory.path().join("home");
    let xdg = directory.path().join("xdg");
    std::fs::create_dir(&home)?;
    std::fs::create_dir(&xdg)?;
    let marker = directory.path().join("environment");
    let socket = directory.path().join("private/socket");
    let _cleanup = Running {
        socket: socket.clone(),
        interrupt: Arc::new(AtomicBool::new(false)),
        thread: None,
        child: None,
    };
    let mut helper = program(&socket)?;
    helper.cache_daemon = Some(cache::DaemonCommand {
        executable: gix_path::env::shell().into(),
        args: vec![
            "-c".into(),
            r#"printf '%s\n' "$HOME" "$XDG_CACHE_HOME" "$GIX_CACHE_VALUE" >"$GIX_CACHE_MARKER"; exec "$@""#.into(),
            "cache-launcher".into(),
            gix_testtools::git_command(directory.path()).get_program().into(),
            "credential-cache--daemon".into(),
        ],
    });
    helper::Cascade::default().extend([helper]).invoke_with_environment(
        Action::Store(b"username=user\npassword=secret\n".as_slice().into()),
        gix_prompt::Options {
            mode: gix_prompt::Mode::Disable,
            ..Default::default()
        },
        &[
            ("HOME".into(), home.clone().into_os_string()),
            ("XDG_CACHE_HOME".into(), xdg.clone().into_os_string()),
            ("GIX_CACHE_VALUE".into(), "operation-value".into()),
            ("GIX_CACHE_MARKER".into(), marker.clone().into_os_string()),
        ],
    )?;
    assert_eq!(
        std::fs::read_to_string(marker)?,
        format!("{}\n{}\noperation-value\n", home.display(), xdg.display()),
        "the daemon inherits operation-specific environment"
    );
    assert!(
        !cache::request(&socket, "get", 900, b"", None)?.is_empty(),
        "the launched daemon stored the credential"
    );
    Ok(())
}
