mod checkout;

use std::path::{Path, PathBuf};

pub type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

pub use gix_testtools::scripted_fixture_read_only;

pub fn fixture_path(name: &str) -> PathBuf {
    crate::scripted_fixture_read_only(Path::new(name).with_extension("sh")).expect("script works")
}

fn odb_at(objects_dir: impl Into<std::path::PathBuf>) -> Result<gix_odb::Handle> {
    Ok(gix_odb::at_opts(
        objects_dir,
        Vec::new(),
        gix_odb::store::init::Options {
            object_hash: gix_testtools::object_hash(),
            ..Default::default()
        },
    )?)
}
