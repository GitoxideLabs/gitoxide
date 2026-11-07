use bstr::ByteSlice;
use gix_fsmonitor::client;
use gix_testtools::TestResult;

#[test]
#[cfg(unix)]
fn running_daemon_exchanges_fragmented_packets_without_startup_context() -> TestResult {
    use std::{
        io::{self, Read, Write},
        os::unix::net::UnixListener,
        time::{Duration, Instant},
    };

    let directory = gix_testtools::tempfile::tempdir_in("/tmp")?;
    let socket = directory.path().join("fsmonitor.ipc");
    let listener = UnixListener::bind(&socket)?;
    listener.set_nonblocking(true)?;
    let server = std::thread::spawn(move || -> io::Result<()> {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(err) if err.kind() == io::ErrorKind::WouldBlock && Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(err) => return Err(err),
            }
        };
        stream.set_nonblocking(false)?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        let mut request = [0; 17];
        stream.read_exact(&mut request)?;
        assert_eq!(request, *b"000dold-token0000", "the request ends in a flush, not a NUL");
        for byte in b"0009token0008\x00a\x00b0000" {
            stream.write_all(&[*byte])?;
        }
        Ok(())
    });
    let response = client::daemon::query(
        &socket,
        b"old-token".as_bstr(),
        || panic!("an existing daemon must not need repository command context"),
        &client::daemon::Options::default(),
    );
    server.join().expect("the mock daemon must finish without panicking")?;
    assert_eq!(
        response?, b"token\x00a\x00b",
        "packet boundaries preserve the binary reply"
    );
    Ok(())
}

#[test]
fn hook_negotiates_versions_and_preserves_arguments_context_and_binary_output() -> TestResult {
    use client::hook::Version;

    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let directory = gix_testtools::tempfile::tempdir()?;
    let worktree = directory.path().join("fsmonitor-hook");
    std::fs::create_dir(&worktree)?;
    let context = gix_command::Context {
        worktree_dir: Some(worktree),
        ref_namespace: Some("client-test".into()),
        stderr: Some(false),
        ..Default::default()
    };
    let command = br#"query() {
        test "${PWD##*/}" = fsmonitor-hook || exit 3
        test "$GIT_NAMESPACE" = client-test || exit 4
        test "$1" = 1 || exit 5
        printf '%s\0%s\0' "$1" "$2"
    }
    query "$@""#
        .as_bstr();
    for worktree_dir in [None, Some("relative".into())] {
        assert!(
            client::hook::query(
                command,
                b"0".as_bstr(),
                None,
                gix_command::Context {
                    worktree_dir,
                    ..Default::default()
                },
            )
            .is_err(),
            "hooks require an explicit absolute worktree before executing commands"
        );
    }
    let previous = b"previous token '$;".as_bstr();
    assert_eq!(
        client::hook::query(command, previous, None, context.clone())?,
        Some((Version::One, b"1\x00previous token '$;\x00".to_vec())),
        "automatic negotiation retries version one in the worktree with literal token arguments"
    );
    assert_eq!(
        client::hook::query(command, previous, Some(Version::Two), context.clone())?,
        None,
        "an explicitly selected version must not fall back"
    );
    assert_eq!(
        client::hook::query(
            br#"printf 'next-token\0path\0'"#.as_bstr(),
            previous,
            Some(Version::Two),
            context,
        )?,
        Some((Version::Two, b"next-token\x00path\x00".to_vec())),
        "the client returns the raw version-two response for its caller to interpret"
    );
    Ok(())
}
