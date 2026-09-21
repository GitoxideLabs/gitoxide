#[cfg(not(target_os = "macos"))]
#[forbid(unsafe_code)]
mod compat;
#[cfg(not(target_os = "macos"))]
pub(crate) use compat::Backend;

#[cfg(target_os = "macos")]
mod fsevents;
#[cfg(target_os = "macos")]
pub(crate) use fsevents::Backend;
