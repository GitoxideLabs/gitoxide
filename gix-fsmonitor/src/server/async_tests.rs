use super::*;

struct Source {
    housekeeping: Housekeeping,
    batch: Option<Batch>,
    fence: std::result::Result<Fence, SynchronizeError>,
}

#[allow(clippy::unused_async_trait_impl)]
impl EventSource for Source {
    fn housekeeping(&self) -> &Housekeeping {
        &self.housekeeping
    }
    async fn register(&mut self, _watches: &[Watch]) -> Result {
        Ok(())
    }
    fn drain(&mut self, _budget: Budget) -> Batch {
        self.batch.take().unwrap_or_else(|| Batch {
            events: Vec::new(),
            loss: None,
            error: None,
            generation: 7,
            sequence: 42,
            more: false,
        })
    }
    async fn synchronize_at(
        &mut self,
        _directory: &Path,
        _deadline: Instant,
    ) -> std::result::Result<Fence, SynchronizeError> {
        self.fence
    }
    fn supports_cookie_synchronization(&self) -> bool {
        false
    }
    async fn ready(&self) {
        std::future::pending::<()>().await;
    }
    async fn close(self) -> Result {
        Ok(())
    }
}

#[tokio::test(flavor = "current_thread")]
async fn cancelled_housekeeping_keeps_its_worker_budget_until_native_work_finishes() -> gix_error::TestResult {
    let budget = OwnShared::new(Semaphore::new(1));
    let (started, receive_started) = oneshot::channel();
    let (release, receive_release) = std::sync::mpsc::sync_channel(1);
    let mut operation = Box::pin(run_io(&budget, move || {
        let _ = started.send(());
        receive_release.recv().map_err(io::Error::other)?;
        Ok(())
    }));
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut operation)
            .await
            .is_err(),
        "native housekeeping yields to executor deadlines"
    );
    receive_started.await?;
    drop(operation);
    assert!(
        budget.try_acquire().is_err(),
        "abandoning a reply does not release the outstanding native operation's budget"
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(10), run_io(&budget, || Ok(())))
            .await
            .is_err(),
        "successors cannot start additional workers while abandoned work remains active"
    );
    release.send(())?;
    let _released = tokio::time::timeout(Duration::from_secs(2), budget.acquire()).await??;
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn cookie_cleanup_holds_ownership_until_the_marker_is_removed() -> gix_error::TestResult {
    let directory = gix_testtools::tempfile::tempdir()?;
    let source = Source {
        housekeeping: Housekeeping::default(),
        batch: None,
        fence: Err(SynchronizeError::Unsupported),
    };
    let cookie = create_cookie(&source, directory.path()).await?;
    let path = cookie.path.clone();
    assert!(
        path.exists(),
        "the marker stays present while its observation is pending"
    );
    drop(cookie);
    let _cleaned = tokio::time::timeout(Duration::from_secs(2), source.housekeeping.cookies.acquire()).await??;
    assert!(
        !path.exists(),
        "the cookie budget is released only after native cleanup completes"
    );
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn cancelling_a_root_check_preserves_the_ingested_batch_and_recovery_state() -> gix_error::TestResult {
    let directory = gix_testtools::tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    let git_dir = root.join(".git");
    std::fs::create_dir(&git_dir)?;
    let layout = Layout::from_context(Context {
        worktree: root,
        git_dir: git_dir.clone(),
        common_dir: git_dir.clone(),
        endpoint: git_dir.join("socket"),
        precompose_unicode: false,
        ignore_case: false,
    });
    let source = Source {
        housekeeping: Housekeeping::default(),
        batch: None,
        fence: Ok(Fence {
            generation: 7,
            sequence: 42,
        }),
    };
    let mut monitor = Monitor::new(layout, source, Default::default()).await?;
    let io = monitor.watcher.housekeeping.io.clone();
    let occupied = io.acquire_owned().await?;
    let batch = Batch {
        events: Vec::new(),
        loss: Some(gix_notify::Loss::Overflow),
        error: None,
        generation: 8,
        sequence: 43,
        more: false,
    };
    assert!(
        tokio::time::timeout(Duration::from_millis(10), monitor.ingest(batch, Instant::now()))
            .await
            .is_err(),
        "a blocked root check can be abandoned without blocking the executor"
    );
    assert_eq!(
        monitor.generation,
        Some(8),
        "coverage invalidation is ingested before any filesystem wait"
    );
    assert!(
        !monitor.healthy && monitor.retry.is_some(),
        "cancellation leaves conservative recovery pending"
    );
    drop(occupied);
    monitor.close().await?;
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn async_responses_use_the_shared_loss_generation_and_deadline_rules() -> gix_error::TestResult {
    for (loss, fence) in [
        (
            None,
            Ok(Fence {
                generation: 7,
                sequence: 42,
            }),
        ),
        (
            Some(gix_notify::Loss::Overflow),
            Ok(Fence {
                generation: 7,
                sequence: 42,
            }),
        ),
        (
            None,
            Ok(Fence {
                generation: 8,
                sequence: 42,
            }),
        ),
        (None, Err(SynchronizeError::TimedOut)),
        (None, Err(SynchronizeError::CoverageLost)),
    ] {
        let directory = gix_testtools::tempfile::tempdir()?;
        let worktree = directory.path().canonicalize()?;
        let git_dir = worktree.join(".git");
        std::fs::create_dir(&git_dir)?;
        let layout = Layout::from_context(Context {
            worktree: worktree.clone(),
            git_dir: git_dir.clone(),
            common_dir: git_dir.clone(),
            endpoint: git_dir.join("socket"),
            precompose_unicode: false,
            ignore_case: false,
        });
        let source = Source {
            housekeeping: Housekeeping::default(),
            batch: None,
            fence,
        };
        let mut monitor = Monitor::new(layout, source, Default::default()).await?;
        monitor.generation = Some(7);
        let now = Instant::now();
        let baseline = monitor.journal.response(b"", true, now);
        let previous = baseline
            .split(|byte| *byte == 0)
            .next()
            .expect("every response begins with a token");
        monitor.watcher.batch = Some(Batch {
            events: vec![gix_notify::Event {
                paths: vec![worktree.join("changed")],
                kind: gix_notify::EventKind::Modify,
                path_kind: gix_notify::PathKind::File,
            }],
            loss,
            error: None,
            generation: 7,
            sequence: 42,
            more: false,
        });
        let response = monitor.response(previous, now).await;
        if loss.is_none()
            && fence
                == Ok(Fence {
                    generation: 7,
                    sequence: 42,
                })
        {
            assert!(
                response.ends_with(b"\0changed\0"),
                "certified same-generation delivery returns incremental paths"
            );
        } else {
            assert!(
                response.ends_with(b"\0/\0"),
                "loss, generation conflicts and incomplete fences require a full baseline"
            );
        }
        if fence.is_err() {
            assert!(
                !monitor.healthy && monitor.retry.is_some(),
                "failed synchronization schedules recovery"
            );
        }
        monitor.close().await?;
    }
    Ok(())
}
