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

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use windows::Backend;
