pub use ::bisync::asynchronous as bisync;

use std::path::PathBuf;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T = ()> = std::result::Result<T, Error>;

pub fn fixture_bytes(path: &str) -> Vec<u8> {
    fn fixture_path(path: &str) -> PathBuf {
        PathBuf::from("tests").join("fixtures").join(path)
    }
    std::fs::read(fixture_path(path)).expect("fixture to be present and readable")
}

#[cfg(not(any(feature = "blocking-client", feature = "http-client-curl")))]
mod client;
