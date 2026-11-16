use std::{
    io::Write,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

use gix::{
    Result,
    error::{ErrorExt, ResultExt, bail, message, validation},
    notify::plumbing::{Budget, Options, Watch, Watcher},
};

/// Print raw native notifications until interrupted, without repository discovery or recovery.
/// Loss requires a fresh baseline; this diagnostic tool only reports it and backend errors.
pub fn function(
    paths: &[PathBuf],
    recursive: bool,
    options: Options,
    budget: Budget,
    mut out: impl Write,
    mut err: impl Write,
    should_interrupt: &AtomicBool,
) -> Result<()> {
    if budget.max_events == 0 {
        bail!(validation("the notification batch event limit must be positive"));
    }
    if options
        .root_check_interval
        .is_some_and(|interval| Instant::now().checked_add(interval).is_none())
    {
        bail!(validation(
            "the filesystem root check interval exceeds the supported clock range"
        ));
    }
    let watches = paths
        .iter()
        .map(|path| {
            path.canonicalize()
                .or_raise(|| message!("resolving notification path {}", path.display()))
                .map(|path| Watch { path, recursive })
        })
        .collect::<Result<Vec<_>>>()?;
    let (send, receive) = mpsc::sync_channel(1);
    let mut watcher = Watcher::new(options)?;
    watcher.set_waker(move || {
        // A full channel already carries a wakeup; disconnection means the receiver is gone.
        let _ = send.try_send(());
    });
    watcher
        .replace(watches)
        .or_raise(|| message("registering filesystem notification paths"))?;

    while !should_interrupt.load(Ordering::Relaxed) {
        let batch = watcher.drain(budget);
        if !batch.events.is_empty() || batch.loss.is_some() || batch.error.is_some() {
            writeln!(
                out,
                "Batch: generation={} sequence={} more={}",
                batch.generation, batch.sequence, batch.more
            )
            .or_error()?;
        }
        if let Some(reason) = batch.loss {
            writeln!(out, "Rescan required: {reason:?}").or_error()?;
        }
        if let Some(error) = batch.error {
            writeln!(err, "Notification error: {error:?}").or_error()?;
        }
        for event in batch.events {
            writeln!(out, "{:?} ({:?}): {:?}", event.kind, event.path_kind, event.paths).or_error()?;
        }
        out.flush().or_error()?;
        err.flush().or_error()?;
        if !batch.more && !should_interrupt.load(Ordering::Relaxed) {
            // Native callbacks wake this wait. The ceiling only bounds signal response time.
            match receive.recv_timeout(Duration::from_millis(250)) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(error) => return Err(error.and_raise(message("waiting for filesystem notifications"))),
            }
        }
    }
    // Dropping the blocking watcher retains native worker ownership through shutdown.
    Ok(())
}
