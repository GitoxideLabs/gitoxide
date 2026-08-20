pub use ::bisync::synchronous as bisync;

pub type Result = std::result::Result<(), Box<dyn std::error::Error + Send + Sync>>;

pub fn assert_err_display<T: std::fmt::Debug, E: std::error::Error>(
    res: std::result::Result<T, E>,
    expected: impl AsRef<str>,
) {
    match res {
        Ok(v) => panic!("Expected error '{}', got value {:?}", expected.as_ref(), v),
        Err(err) => assert_eq!(err.to_string(), expected.as_ref()),
    }
}

#[cfg(feature = "blocking-io")]
mod decode;
#[cfg(feature = "blocking-io")]
mod encode;
#[cfg(feature = "blocking-io")]
mod read;
#[cfg(feature = "blocking-io")]
mod write;
