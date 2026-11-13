use bstr::ByteSlice;
use gix_fsmonitor::client;
use gix_testtools::TestResult;

#[cfg(unix)]
fn spawn_mock_daemon(
    listener: std::os::unix::net::UnixListener,
) -> std::io::Result<std::thread::JoinHandle<std::io::Result<()>>> {
    use std::{
        io::{self, Read, Write},
        time::{Duration, Instant},
    };

    listener.set_nonblocking(true)?;
    Ok(std::thread::spawn(move || -> io::Result<()> {
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
    }))
}

#[test]
#[cfg(unix)]
fn running_daemon_exchanges_fragmented_packets_without_startup_context() -> TestResult {
    for long_path in [false, true] {
        let directory = gix_testtools::tempfile::tempdir_in("/tmp")?;
        let parent = directory.path().join(if long_path {
            "x".repeat(120)
        } else {
            "repository".into()
        });
        std::fs::create_dir(&parent)?;
        let alias = directory.path().join("alias");
        std::os::unix::fs::symlink(&parent, &alias)?;
        // Bind through a short parent alias so the mock can expose an overlong endpoint pathname.
        let server = spawn_mock_daemon(std::os::unix::net::UnixListener::bind(alias.join("fsmonitor.ipc"))?)?;
        let response = client::daemon::query(
            &parent.join("fsmonitor.ipc"),
            b"old-token".as_bstr(),
            || panic!("an existing daemon must not invoke the startup callback"),
            client::daemon::Options::default(),
        );
        server.join().expect("the mock daemon must finish without panicking")?;
        assert_eq!(
            response?, b"token\x00a\x00b",
            "short and overlong endpoints preserve the binary reply across fragmented packets"
        );
    }
    Ok(())
}

#[cfg(unix)]
fn query_with_delayed_io(
    timeout: Option<std::time::Duration>,
    fragmented: bool,
    unread_request: bool,
) -> TestResult<(gix_error::Result<Vec<u8>>, std::time::Duration)> {
    use std::{
        io::{self, Read, Write},
        os::unix::net::UnixListener,
        sync::mpsc,
        time::{Duration, Instant},
    };

    let directory = gix_testtools::tempfile::tempdir_in("/tmp")?;
    let socket = directory.path().join("fsmonitor.ipc");
    let listener = UnixListener::bind(&socket)?;
    listener.set_nonblocking(true)?;
    let (stop, receiver) = mpsc::channel();
    let server = std::thread::spawn(move || -> io::Result<()> {
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    if receiver.recv_timeout(Duration::from_millis(2)).is_ok() || Instant::now() >= deadline {
                        return Ok(());
                    }
                }
                Err(error) => return Err(error),
            }
        };
        stream.set_nonblocking(false)?;
        stream.set_read_timeout(Some(Duration::from_secs(3)))?;
        stream.set_write_timeout(Some(Duration::from_secs(3)))?;
        if unread_request {
            // The client must time out in write once its large token fills the socket buffer.
            let _ = receiver.recv_timeout(Duration::from_secs(3));
            return Ok(());
        }
        let mut request = [0; 13];
        if let Err(error) = stream.read_exact(&mut request) {
            return match error.kind() {
                io::ErrorKind::UnexpectedEof | io::ErrorKind::ConnectionReset => Ok(()),
                _ => Err(error),
            };
        }
        assert_eq!(
            request, *b"0009token0000",
            "the mock receives a complete packetized request"
        );
        if fragmented {
            // Every fragment arrives sooner than the timeout, but the complete reply takes longer.
            for byte in b"001c0123456789012345678901230000" {
                if receiver.recv_timeout(Duration::from_millis(30)).is_ok() {
                    return Ok(());
                }
                if let Err(error) = stream.write_all(&[*byte]) {
                    return match error.kind() {
                        io::ErrorKind::BrokenPipe | io::ErrorKind::ConnectionReset => Ok(()),
                        _ => Err(error),
                    };
                }
            }
        } else {
            let _ = receiver.recv_timeout(Duration::from_secs(3));
        }
        Ok(())
    });
    let token = if unread_request {
        vec![b'x'; 1024 * 1024]
    } else {
        b"token".to_vec()
    };
    let started = Instant::now();
    let result = client::daemon::query(
        &socket,
        token.as_bstr(),
        || panic!("an existing mock daemon must not trigger startup"),
        client::daemon::Options {
            socket_timeout: timeout,
            connect_retries: 0,
            ..Default::default()
        },
    );
    let elapsed = started.elapsed();
    let _ = stop.send(());
    server
        .join()
        .expect("the bounded mock daemon must finish without panicking")?;
    Ok((result, elapsed))
}

#[test]
#[cfg(unix)]
fn blocking_daemon_bounds_stalled_and_fragmented_reads_and_blocked_writes() -> TestResult {
    use std::{io, time::Duration};

    for (fragmented, unread_request) in [(false, false), (true, false), (false, true)] {
        let (result, elapsed) = query_with_delayed_io(Some(Duration::from_millis(200)), fragmented, unread_request)?;
        let error = result.expect_err("one deadline bounds the exchange even if individual fragments make progress");
        assert_eq!(
            error
                .downcast_any_ref::<io::Error>()
                .expect("the query preserves its I/O error")
                .kind(),
            io::ErrorKind::TimedOut,
            "stalled reads, fragmented replies, and undrained writes all report a timeout"
        );
        assert!(
            elapsed < Duration::from_secs(2),
            "the deadline, not mock cleanup, releases the client"
        );
    }
    Ok(())
}

#[test]
#[cfg(unix)]
fn blocking_daemon_can_disable_the_deadline_or_expire_it_immediately() -> TestResult {
    use std::{io, time::Duration};

    let (response, elapsed) = query_with_delayed_io(None, true, false)?;
    assert_eq!(
        response?, b"012345678901234567890123",
        "None permits a slow successful reply"
    );
    assert!(
        elapsed > Duration::from_millis(200),
        "the reply outlives the enabled test timeout"
    );
    let (result, elapsed) = query_with_delayed_io(Some(Duration::ZERO), false, false)?;
    assert_eq!(
        result
            .expect_err("zero must expire immediately")
            .downcast_any_ref::<io::Error>()
            .expect("the query preserves the timeout error")
            .kind(),
        io::ErrorKind::TimedOut,
        "zero disables waiting, not the deadline"
    );
    assert!(
        elapsed < Duration::from_secs(1),
        "zero never waits for the daemon's reply"
    );
    Ok(())
}

#[test]
#[cfg(unix)]
fn custom_startup_connects_to_missing_and_refused_endpoints() -> TestResult {
    use gix_error::ResultExt;
    use std::{os::unix::net::UnixListener, time::Duration};

    for refused in [false, true] {
        let directory = gix_testtools::tempfile::tempdir_in("/tmp")?;
        let socket = directory.path().join("fsmonitor.ipc");
        if refused {
            drop(UnixListener::bind(&socket)?);
        }
        let mut calls = 0;
        let mut server = None;
        let startup_socket = socket.clone();
        let start = || {
            calls += 1;
            if refused {
                std::fs::remove_file(&startup_socket).or_error()?;
            }
            server = Some(spawn_mock_daemon(UnixListener::bind(startup_socket).or_error()?).or_error()?);
            Ok(Ok(()))
        };
        let response = client::daemon::query(
            &socket,
            b"old-token".as_bstr(),
            start,
            client::daemon::Options {
                connect_retries: 0,
                socket_timeout: Some(Duration::from_secs(5)),
                max_response_size: 9,
                ..Default::default()
            },
        );
        server
            .expect("custom startup must spawn the mock daemon")
            .join()
            .expect("the mock daemon must finish without panicking")?;
        assert_eq!(calls, 1, "an unavailable endpoint invokes custom startup exactly once");
        assert_eq!(
            response?, b"token\x00a\x00b",
            "custom startup uses the normal IPC exchange"
        );
    }
    Ok(())
}

#[test]
#[cfg(unix)]
fn custom_startup_errors_and_unavailable_endpoints_do_not_fall_back_to_git() -> TestResult {
    use gix_error::{bail, message};
    use std::time::Duration;

    let directory = gix_testtools::tempfile::tempdir_in("/tmp")?;
    let socket = directory.path().join("fsmonitor.ipc");
    for fails in [true, false] {
        let mut calls = 0;
        let start = || {
            calls += 1;
            if fails {
                bail!(message("custom startup failed"));
            }
            Ok(Ok(()))
        };
        let error = client::daemon::query(
            &socket,
            b"old-token".as_bstr(),
            start,
            client::daemon::Options {
                connect_retries: 2,
                connect_retry_delay: Duration::ZERO,
                ..Default::default()
            },
        )
        .expect_err("startup failure or an endpoint that stays unavailable must fail the query");
        assert_eq!(calls, 1, "connection retries must not invoke startup again");
        assert!(
            error.to_string().contains(if fails {
                "Could not start the filesystem monitor"
            } else {
                "Could not connect to the filesystem monitor after starting it"
            }),
            "errors distinguish failed startup from failed connection after startup"
        );
    }
    Ok(())
}

#[test]
#[cfg(unix)]
fn startup_callback_can_supply_context_for_default_git_startup() -> TestResult {
    let directory = gix_testtools::tempfile::tempdir_in("/tmp")?;
    let socket = directory.path().join("fsmonitor.ipc");
    let context = gix_command::Context::default();
    let mut start_called = false;
    let error = client::daemon::query(
        &socket,
        b"old-token".as_bstr(),
        || {
            start_called = true;
            Ok(Err(context))
        },
        client::daemon::Options::default(),
    )
    .expect_err("default startup must reject unsupported platforms or a context without a worktree");
    assert!(
        start_called,
        "the startup callback decides whether to use Git's fallback"
    );

    assert!(
        error.to_string().contains(if cfg!(target_os = "macos") {
            "Cannot start a filesystem monitor without a worktree"
        } else {
            "Git's filesystem monitor daemon is unavailable on this platform"
        }),
        "returning a context selects default startup without launching Git in this test"
    );
    assert_eq!(
        error.is_unsupported(),
        !cfg!(target_os = "macos"),
        "default startup on unsupported platforms is classified as unsupported"
    );
    if !cfg!(target_os = "macos") {
        assert_eq!(
            error
                .downcast_any_ref::<std::io::Error>()
                .expect("the original connection error is preserved")
                .kind(),
            std::io::ErrorKind::NotFound,
            "unsupported startup retains the missing endpoint as its cause"
        );
    }
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

#[cfg(feature = "async-tokio")]
#[tokio::test(flavor = "current_thread")]
async fn async_hooks_share_negotiation_and_binary_arguments() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let directory = gix_testtools::tempfile::tempdir()?;
    let command = br#"query() {
        test "$1" = 1 || exit 5
        printf '%s\0%s\0' "$1" "$2"
    }
    query "$@""#
        .as_bstr();
    let previous = b"previous token '$;".as_bstr();
    let context = gix_command::Context {
        worktree_dir: Some(directory.path().to_owned()),
        ..Default::default()
    };
    assert_eq!(
        client::hook::query_async(command, previous, None, context.clone()).await?,
        Some((client::hook::Version::One, b"1\0previous token '$;\0".to_vec())),
        "async negotiation preserves literal checkpoint arguments"
    );
    assert!(
        client::hook::query_async(command, previous, Some(client::hook::Version::Two), context)
            .await?
            .is_none(),
        "explicit protocols do not fall back"
    );
    Ok(())
}

#[cfg(all(feature = "async-tokio", unix))]
#[tokio::test(flavor = "current_thread")]
async fn async_daemon_exchange_deadline_does_not_block_the_executor() -> TestResult {
    use std::{
        io,
        time::{Duration, Instant},
    };
    let directory = gix_testtools::tempfile::tempdir_in("/tmp")?;
    let endpoint = directory.path().join("socket");
    let error = client::daemon::query_async(
        &endpoint,
        b"token".as_bstr(),
        async || panic!("an immediate deadline must not invoke startup even if the endpoint is missing"),
        client::daemon::Options {
            socket_timeout: Some(Duration::ZERO),
            ..Default::default()
        },
    )
    .await
    .expect_err("zero expires before a connection attempt");
    assert_eq!(
        error
            .downcast_any_ref::<io::Error>()
            .expect("the immediate timeout retains its I/O error")
            .kind(),
        io::ErrorKind::TimedOut,
        "zero has the same meaning in blocking and async clients"
    );
    let listener = tokio::net::UnixListener::bind(&endpoint)?;
    let server = tokio::spawn(async move {
        let (_stream, _) = listener.accept().await?;
        tokio::time::sleep(Duration::from_secs(5)).await;
        Ok::<_, io::Error>(())
    });
    let now = Instant::now();
    let result = client::daemon::query_async(
        &endpoint,
        b"token".as_bstr(),
        async || panic!("the endpoint is present"),
        client::daemon::Options {
            socket_timeout: Some(Duration::from_millis(20)),
            ..Default::default()
        },
    )
    .await;
    server.abort();
    let error = result.expect_err("a daemon that sends no response must time out");
    assert_eq!(
        error
            .downcast_any_ref::<io::Error>()
            .expect("the timeout retains its I/O error")
            .kind(),
        io::ErrorKind::TimedOut
    );
    assert!(
        now.elapsed() < Duration::from_secs(1),
        "the current-thread runtime runs the deadline timer while waiting for I/O"
    );
    Ok(())
}
