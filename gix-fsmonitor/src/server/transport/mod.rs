#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(super) use unix::{Listener, Wake, connect};
#[cfg(windows)]
mod windows;
#[cfg(windows)]
mod windows_security;

#[cfg(windows)]
pub(super) use windows::{Listener, Wake, connect};
#[cfg(all(windows, feature = "async-tokio"))]
pub(super) use windows_security::create_async_pipe;
