#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
#[forbid(unsafe_code)]
mod compat;
#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
pub(crate) use compat::Backend;

#[cfg(target_os = "linux")]
#[forbid(unsafe_code)]
mod inotify;
#[cfg(target_os = "linux")]
pub(crate) use inotify::Backend;

#[cfg(target_os = "macos")]
mod fsevents;
#[cfg(target_os = "macos")]
pub(crate) use fsevents::Backend;

#[cfg(any(target_os = "macos", test))]
fn wait_for_fence(
    reply: &std::sync::mpsc::Receiver<Result<crate::Fence, crate::SynchronizeError>>,
    deadline: std::time::Instant,
) -> Result<crate::Fence, crate::SynchronizeError> {
    use std::{sync::mpsc::RecvTimeoutError, time::Instant};

    use crate::SynchronizeError;

    let fence = reply
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .map_err(|error| match error {
            RecvTimeoutError::Timeout => SynchronizeError::TimedOut,
            RecvTimeoutError::Disconnected => SynchronizeError::Stopped,
        })??;
    if Instant::now() >= deadline {
        return Err(SynchronizeError::TimedOut);
    }
    Ok(fence)
}

#[cfg(test)]
#[test]
fn the_marker_reply_is_sufficient_until_the_deadline() -> gix_error::TestResult {
    use std::{
        sync::mpsc,
        time::{Duration, Instant},
    };

    use crate::{Fence, SynchronizeError};

    let (reply, receive) = mpsc::sync_channel(1);
    let fence = Fence {
        generation: 7,
        sequence: 42,
    };
    assert_eq!(
        wait_for_fence(&receive, Instant::now()),
        Err(SynchronizeError::TimedOut),
        "a missing marker cannot satisfy an expired fence"
    );
    reply.send(Ok(fence))?;
    assert_eq!(
        wait_for_fence(&receive, Instant::now() + Duration::from_secs(1)),
        Ok(fence),
        "the callback reply is sufficient without a flush acknowledgement"
    );
    reply.send(Ok(fence))?;
    assert_eq!(
        wait_for_fence(&receive, Instant::now()),
        Err(SynchronizeError::TimedOut),
        "a queued marker result cannot certify a fence after its deadline"
    );
    Ok(())
}

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use windows::Backend;
