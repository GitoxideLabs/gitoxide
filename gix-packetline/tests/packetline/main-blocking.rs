pub mod bisync {
    pub use gix_macros::{discard as only_async, keep as only_sync, sync as bisync};
}

#[cfg(feature = "blocking-io")]
mod decode;
#[cfg(feature = "blocking-io")]
mod encode;
#[cfg(feature = "blocking-io")]
mod read;
#[cfg(feature = "blocking-io")]
mod write;
