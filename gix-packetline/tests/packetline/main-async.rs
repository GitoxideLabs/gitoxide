pub mod bisync {
    pub use gix_macros::{discard as only_sync, keep as bisync, keep as only_async};
}

#[cfg(all(feature = "async-io", not(feature = "blocking-io")))]
mod decode;
#[cfg(all(feature = "async-io", not(feature = "blocking-io")))]
mod encode;
#[cfg(all(feature = "async-io", not(feature = "blocking-io")))]
mod read;
#[cfg(all(feature = "async-io", not(feature = "blocking-io")))]
mod write;
