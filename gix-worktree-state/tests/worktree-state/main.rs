mod checkout;

use std::path::{Path, PathBuf};

pub type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

pub use gix_testtools::scripted_fixture_read_only;

pub fn fixture_path(name: &str) -> PathBuf {
    crate::scripted_fixture_read_only(Path::new(name).with_extension("sh")).expect("script works")
}
