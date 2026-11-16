use super::*;
use server::ipc::{self, Request};

#[cfg(windows)]
#[tokio::test(flavor = "current_thread")]
async fn ipc_helpers_wait_for_a_busy_pipe_without_blocking_the_executor() -> TestResult {
    for probe in [true, false] {
        let fixture = Fixture::new()?;
        let handle = server::spawn(fixture.context(), Default::default())?;
        let endpoint = handle.endpoint().to_owned();
        let held = connect_pipe(&endpoint)?;
        let error = connect_pipe(&endpoint).expect_err("the single pipe instance is occupied");
        assert_eq!(error.raw_os_error(), Some(231), "Windows reports ERROR_PIPE_BUSY");
        let request = async {
            if probe {
                assert!(ipc::is_running_async(&endpoint).await?, "the busy server is healthy");
            } else {
                assert!(
                    ipc::query_async(&endpoint, Request::Message(b""))
                        .await?
                        .ends_with(b"\0/\0"),
                    "the query succeeds after the other client releases its connection"
                );
            }
            Ok::<_, gix_error::Error>(())
        };
        tokio::pin!(request);
        assert!(
            tokio::time::timeout(Duration::from_millis(50), &mut request)
                .await
                .is_err(),
            "a busy pipe leaves the IPC future pending while executor timers keep running"
        );
        drop(held);
        tokio::time::timeout(Duration::from_secs(2), &mut request).await??;
        handle.shutdown()?;
    }
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn both_frontends_preserve_tokens_paths_and_native_client_interoperability() -> TestResult {
    for asynchronous in [false, true] {
        let fixture = Fixture::new()?;
        let context = fixture.context();
        let endpoint = context.endpoint.clone();
        let blocking = if asynchronous {
            None
        } else {
            Some(server::spawn(context.clone(), Default::default())?)
        };
        let async_handle = if asynchronous {
            Some(server::asynchronous::spawn(context, Default::default()).await?)
        } else {
            None
        };
        assert!(
            ipc::is_running_async(&endpoint).await?,
            "startup returns only after the endpoint is ready"
        );
        let baseline = ipc::query_async(&endpoint, Request::Message(b"")).await?;
        assert!(
            baseline.ends_with(b"\0/\0"),
            "unknown tokens request a full baseline on both frontends"
        );
        let previous = token(&baseline)?.to_vec();
        std::fs::write(fixture.worktree.join("changed"), b"data")?;
        let response = client::daemon::query_async(
            &endpoint,
            previous.as_bstr(),
            async || {
                panic!("ready servers never require startup");
            },
            Default::default(),
        )
        .await?;
        if cfg!(any(target_os = "macos", target_os = "linux", windows)) {
            assert!(
                response.split(|byte| *byte == 0).any(|path| path == b"changed"),
                "a same-stream fence publishes preceding worktree writes"
            );
        }
        let path = endpoint.clone();
        let reply = tokio::task::spawn_blocking(move || ipc::query(&path, Request::Message(b""))).await??;
        assert!(
            reply.ends_with(b"\0/\0"),
            "blocking clients also communicate with the async server"
        );
        let flushed = ipc::query_async(&endpoint, Request::Flush).await?;
        assert!(
            flushed.ends_with(b"\0/\0"),
            "flush invalidates journal history in both frontends"
        );
        assert!(
            ipc::query_async(&endpoint, Request::Quit).await?.is_empty(),
            "quit acknowledges cleanup"
        );
        if let Some(handle) = async_handle {
            handle.wait().await?;
        }
        if let Some(handle) = blocking {
            handle.wait()?;
        }
        assert!(!ipc::is_running_async(&endpoint).await?, "quit releases the endpoint");
    }
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn idle_and_requested_shutdown_release_resources_without_executor_waits() -> TestResult {
    let fixture = Fixture::new()?;
    let options = server::Options {
        idle_timeout: Some(Duration::from_millis(100)),
        ..Default::default()
    };
    let handle = server::asynchronous::spawn(fixture.context(), options).await?;
    let endpoint = handle.endpoint().to_owned();
    tokio::time::timeout(Duration::from_secs(3), handle.wait()).await??;
    assert!(
        !ipc::is_running_async(&endpoint).await?,
        "idle expiry releases the endpoint"
    );
    let handle = server::asynchronous::spawn(fixture.context(), Default::default()).await?;
    handle.request_shutdown();
    tokio::time::timeout(Duration::from_secs(3), handle.wait()).await??;
    assert!(
        !ipc::is_running_async(&endpoint).await?,
        "shutdown wakes an otherwise idle server promptly"
    );
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn startup_failure_and_dropped_handles_allow_replacement() -> TestResult {
    let fixture = Fixture::new()?;
    let mut invalid = fixture.context();
    invalid.worktree = PathBuf::from("relative");
    assert!(
        server::asynchronous::spawn(invalid, Default::default()).await.is_err(),
        "validation occurs before binding"
    );
    for options in [
        server::Options {
            fence_timeout: Duration::MAX,
            ..Default::default()
        },
        server::Options {
            retry_interval: Duration::MAX,
            ..Default::default()
        },
    ] {
        let error = server::asynchronous::spawn(fixture.context(), options)
            .await
            .err()
            .expect("overflowing scheduling durations must fail before async startup");
        assert!(
            error.to_string().contains("clock range"),
            "the startup error identifies clock overflow: {error}"
        );
        assert!(
            !fixture.context().endpoint.exists(),
            "invalid options must not acquire the endpoint"
        );
    }
    let handle = server::asynchronous::spawn(fixture.context(), Default::default()).await?;
    let endpoint = handle.endpoint().to_owned();
    assert!(
        server::asynchronous::spawn(fixture.context(), Default::default())
            .await
            .is_err(),
        "an occupied endpoint cannot be stolen"
    );
    drop(handle);
    tokio::time::timeout(Duration::from_secs(3), async {
        while ipc::is_running_async(&endpoint).await? {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Ok::<_, gix_error::Error>(())
    })
    .await??;
    server::asynchronous::spawn(fixture.context(), Default::default())
        .await?
        .shutdown()
        .await?;
    Ok(())
}

#[cfg(unix)]
#[tokio::test(flavor = "current_thread")]
async fn cancelling_startup_does_not_leave_an_endpoint_owned() -> TestResult {
    let fixture = Fixture::new()?;
    let endpoint = fixture.context().endpoint;
    let startup_lock = gix_lock::Marker::acquire_to_hold_resource(&endpoint, Default::default(), None, 0)?;
    let result = tokio::time::timeout(
        Duration::from_millis(10),
        server::asynchronous::spawn(fixture.context(), Default::default()),
    )
    .await;
    assert!(
        result.is_err(),
        "startup remains pending while another process holds the native lock"
    );
    drop(startup_lock);
    // Retry until the cancelled startup has released its own native resources.
    let replacement = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Ok(handle) = server::asynchronous::spawn(fixture.context(), Default::default()).await {
                break handle;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await?;
    replacement.shutdown().await?;
    assert!(
        !ipc::is_running_async(&endpoint).await?,
        "cancelled startup and replacement both release their endpoints"
    );
    Ok(())
}
