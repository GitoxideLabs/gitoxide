#[cfg(feature = "async")]
mod asynchronous;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
mod watcher;
#[cfg(windows)]
mod windows;
