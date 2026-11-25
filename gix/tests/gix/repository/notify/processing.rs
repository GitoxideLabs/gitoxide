use std::{thread::ThreadId, time::Duration};

use gix::error::{ErrorExt, message};

use super::*;

#[tokio::test(flavor = "current_thread")]
async fn processing_uses_the_service_worker_and_retains_simultaneous_changes_and_errors() -> TestResult {
    let repo = crate::basic_repo()?;
    let monitor = repo.monitor(Options::default())?;
    let executor = std::thread::current().id();
    let (opened, opener_thread) = std::sync::mpsc::channel();
    let (monitor, (outcome, processor)) = monitor
        .service_async_with(
            Instant::now(),
            move || {
                opened
                    .send(std::thread::current().id())
                    .expect("the processor awaits the opener");
                Err(message("deliberate opener failure").raise())
            },
            move |monitor, mut outcome| {
                let worker = std::thread::current().id();
                assert_ne!(worker, executor, "outcome processing stays off the executor");
                assert_eq!(
                    opener_thread
                        .try_recv()
                        .expect("service calls the opener before processing"),
                    worker,
                    "servicing and processing use the same blocking dispatch"
                );
                assert_eq!(outcome.errors.len(), 1, "servicing errors reach the processor");
                assert!(
                    !outcome.changes.is_empty(),
                    "failed coverage retains conservative invalidations"
                );
                assert!(
                    matches!(outcome.coverage, CoverageStatus::Recovering { .. }),
                    "the processor receives coverage recovery state"
                );
                monitor
                    .update_subscriptions(
                        None,
                        vec![InvalidationPath {
                            path: monitor.layout().git_dir.join("application-state"),
                            changes: Changes {
                                operations: true,
                                ..Changes::default()
                            },
                        }],
                    )
                    .expect("the worker can resolve an application dependency");
                outcome.errors.push(message("processing diagnostic").raise());
                (outcome, worker)
            },
        )
        .await?;
    assert_ne!(processor, executor, "the returned processor identity is a worker");
    assert_eq!(outcome.errors.len(), 2, "the returned value preserves both diagnostics");
    assert_eq!(
        monitor.options().additional_paths.len(),
        1,
        "subscription updates remain on the returned monitor"
    );
    assert!(
        !outcome.changes.is_empty(),
        "the returned value preserves simultaneous changes"
    );
    monitor.shutdown_async().await?;
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn processing_failures_can_return_the_monitor_and_panics_report_task_errors() -> TestResult {
    let repo = crate::basic_repo()?;
    let inactive = Options {
        metadata: false,
        worktree_coverage: None,
        ..Options::default()
    };
    let (monitor, processed) = repo
        .monitor(inactive.clone())?
        .service_async_with(
            Instant::now(),
            || panic!("inactive monitoring never opens a repository"),
            |_, outcome| {
                assert!(outcome.errors.is_empty(), "inactive servicing has no native errors");
                assert_eq!(
                    outcome.coverage,
                    CoverageStatus::Inactive,
                    "processing failures do not imply failed coverage"
                );
                Err::<(), _>(message("processing failed").raise())
            },
        )
        .await?;
    assert!(
        processed.is_err(),
        "processing failure is separate from service task failure"
    );
    monitor.shutdown_async().await?;
    let error = repo
        .monitor(inactive)?
        .service_async_with(
            Instant::now(),
            || panic!("inactive monitoring never opens a repository"),
            |_, _| -> () { panic!("deliberate processing panic") },
        )
        .await
        .err()
        .expect("a processing panic is returned as a task error");
    assert!(
        error.to_string().contains("servicing repository notifications"),
        "the task failure preserves service context: {error}"
    );
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn cancelling_processing_keeps_active_work_and_unclaimed_results_on_workers() -> TestResult {
    struct RecordDrop(Option<tokio::sync::oneshot::Sender<ThreadId>>);
    impl Drop for RecordDrop {
        fn drop(&mut self) {
            if let Some(record) = self.0.take() {
                let _ = record.send(std::thread::current().id());
            }
        }
    }
    let repo = crate::basic_repo()?;
    let executor = std::thread::current().id();
    let monitor = repo.monitor(Options {
        metadata: false,
        worktree_coverage: None,
        ..Options::default()
    })?;
    let (started, processing) = tokio::sync::oneshot::channel();
    let (release, released) = std::sync::mpsc::channel();
    let (record, dropped) = tokio::sync::oneshot::channel();
    let reply = monitor.service_async_with(
        Instant::now(),
        || panic!("inactive monitoring never opens a repository"),
        move |_, _| {
            let _ = started.send(());
            released.recv().expect("the test releases the active processor");
            RecordDrop(Some(record))
        },
    );
    tokio::time::timeout(Duration::from_secs(2), processing).await??;
    drop(reply);
    release.send(())?;
    assert_ne!(
        tokio::time::timeout(Duration::from_secs(2), dropped).await??,
        executor,
        "an abandoned processed result is dropped on a worker after active processing finishes"
    );
    Ok(())
}
