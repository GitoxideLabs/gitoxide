use std::path::PathBuf;

pub fn fixture(path: &str) -> PathBuf {
    PathBuf::from("tests/fixtures").join(path)
}

mod identity;
mod signature;
