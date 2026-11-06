#[cfg(not(any(target_os = "macos", windows)))]
#[forbid(unsafe_code)]
mod compat;
#[cfg(not(any(target_os = "macos", windows)))]
pub(crate) use compat::Backend;

#[cfg(target_os = "macos")]
mod fsevents;
#[cfg(target_os = "macos")]
pub(crate) use fsevents::Backend;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use windows::Backend;
