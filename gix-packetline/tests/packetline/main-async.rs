pub mod bisync {
    pub use gix_macros::{discard as only_sync, keep as bisync, keep as only_async};
}

pub type Result = gix_error::TestResult;

pub fn assert_err_display<T: std::fmt::Debug, E: std::error::Error>(
    res: std::result::Result<T, E>,
    expected: impl AsRef<str>,
) {
    match res {
        Ok(v) => panic!("Expected error '{}', got value {:?}", expected.as_ref(), v),
        Err(err) => assert_eq!(err.to_string(), expected.as_ref()),
    }
}

#[cfg(all(feature = "async-io", not(feature = "blocking-io")))]
mod decode;
#[cfg(all(feature = "async-io", not(feature = "blocking-io")))]
mod encode;
#[cfg(all(feature = "async-io", not(feature = "blocking-io")))]
mod read;
#[cfg(all(feature = "async-io", not(feature = "blocking-io")))]
mod write;
