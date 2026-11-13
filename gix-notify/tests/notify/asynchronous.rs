use std::time::{Duration, Instant};

use gix_notify::{Budget, Loss, Watch, asynchronous::Watcher};

#[test]
fn registration_readiness_fencing_and_shutdown() -> gix_error::TestResult {
    futures_lite::future::block_on(async {
        let directory = tempfile::tempdir()?;
        let root = directory.path().canonicalize()?;
        let mut watcher = Watcher::new(Default::default())?;
        watcher
            .replace([Watch {
                path: root.clone(),
                recursive: true,
            }])?
            .await?;
        watcher.ready().await;
        let baseline = watcher.drain(Budget::default());
        assert_eq!(
            baseline.loss,
            Some(Loss::WatchSetChanged),
            "registration requires a baseline"
        );
        watcher
            .replace([Watch {
                path: root.clone(),
                recursive: true,
            }])?
            .wait()?;
        assert_eq!(
            watcher.drain(Budget::default()).generation,
            baseline.generation,
            "healthy registration preserves coverage"
        );
        std::fs::write(root.join("changed"), b"data")?;
        let fence = watcher.synchronize_at(&root, Instant::now() + Duration::from_secs(5))?;
        if cfg!(any(target_os = "macos", target_os = "linux")) {
            let fence = fence.await?;
            let mut seen = false;
            loop {
                let batch = watcher.drain(Budget::default());
                assert_eq!(
                    batch.generation, fence.generation,
                    "delivery stays in the certified generation"
                );
                seen |= batch
                    .events
                    .iter()
                    .any(|event| event.paths.contains(&root.join("changed")));
                if batch.sequence >= fence.sequence {
                    break;
                }
            }
            assert!(seen, "preceding writes are published before the same-stream fence");
        } else {
            assert_eq!(fence.await, Err(gix_notify::SynchronizeError::Unsupported));
        }
        watcher.shutdown().await?;
        Ok(())
    })
}

#[test]
fn dropping_registration_preserves_loss_and_diagnostics() -> gix_error::TestResult {
    futures_lite::future::block_on(async {
        let directory = tempfile::tempdir()?;
        let root = directory.path().canonicalize()?;
        let mut watcher = Watcher::new(Default::default())?;
        drop(watcher.replace([Watch {
            path: root.join("missing"),
            recursive: true,
        }])?);
        // Wait for the owner to finish, without assuming dropping a future cancelled native work.
        watcher.idle().await;
        watcher.ready().await;
        let batch = watcher.drain(Budget::default());
        assert!(
            batch.loss.is_some(),
            "cancelled callers cannot hide coverage invalidation"
        );
        assert!(
            batch.error.is_some(),
            "unobserved registration errors remain in the event stream"
        );
        watcher
            .replace([Watch {
                path: root,
                recursive: true,
            }])?
            .await?;
        watcher.shutdown().await?;
        Ok(())
    })
}
