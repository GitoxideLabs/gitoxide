//! Observe whether a child process was reaped, to pin the lifecycle of long-running `process` filters.
//!
//! On Unix a child that exits but is never `wait()`ed for stays in the process table as a zombie,
//! owned by its parent until the parent itself exits. Nothing in `std` can observe that: `try_wait()`
//! needs the `Child` handle, and holding one means the process wasn't abandoned in the first place.
//! Hence these helpers, which ask the kernel by process id instead.

use std::time::{Duration, Instant};

/// What the operating system knows about a process that was launched by this test binary.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum Child {
    /// It is still running as it didn't exit yet.
    Running,
    /// It exited, but nobody waited for it, so it still occupies a slot in the process table.
    ///
    /// Note that observing this state is also what collects the process, as `waitpid()` has no way
    /// to ask without reaping.
    Unreaped,
    /// It isn't a child of this process anymore because it was waited for.
    Reaped,
}

/// How long to give a child to notice that its input and output were closed.
///
/// Only a leaked child is ever waited for here, so this is a failure timeout, not a delay we pay
/// on the happy path.
const EXIT_TIMEOUT: Duration = Duration::from_secs(5);

/// Ask the kernel about `pid` without blocking.
pub(crate) fn observe(pid: u32) -> Child {
    let mut status = 0;
    // SAFETY: `waitpid()` writes to `status` only, and it is valid for the duration of the call.
    let res = unsafe { libc::waitpid(pid as libc::pid_t, &mut status, libc::WNOHANG) };
    match res {
        0 => Child::Running,
        res if res > 0 => Child::Unreaped,
        _ => {
            let err = std::io::Error::last_os_error();
            assert_eq!(
                err.raw_os_error(),
                Some(libc::ECHILD),
                "`waitpid({pid})` may only fail because the process isn't ours to wait for, but failed with {err}"
            );
            Child::Reaped
        }
    }
}

/// Ask the kernel about `pid` until it stops [running][Child::Running], and return what it settled on.
///
/// A child that the library waited for is observable right away, as reaping implies it had already
/// exited. A leaked one first needs a moment to notice that its input was closed, hence the polling.
pub(crate) fn settle(pid: u32) -> Child {
    let start = Instant::now();
    loop {
        match observe(pid) {
            Child::Running if start.elapsed() < EXIT_TIMEOUT => std::thread::sleep(Duration::from_millis(5)),
            state => return state,
        }
    }
}
