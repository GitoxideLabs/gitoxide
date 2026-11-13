use std::path::PathBuf;

pub use gix_testtools::Result;
use gix_testtools::bstr::BString;

const LAYOUTS: [&str; 3] = ["files", "packed", "reftable"];

fn fixture() -> Result<PathBuf> {
    gix_testtools::scripted_fixture_read_only_needs_archive("make_ref_store.sh")
}

fn writable_fixture(layout: &str) -> Result<gix_testtools::tempfile::TempDir> {
    let dir = gix_testtools::tempfile::TempDir::new()?;
    gix_testtools::copy_recursively_into_existing_dir(fixture()?.join(layout), dir.path())?;
    Ok(dir)
}

fn baseline(layout: &str, name: &str) -> Result<BString> {
    Ok(std::fs::read(fixture()?.join(layout).join(name))?.into())
}

fn git_layouts() -> impl Iterator<Item = &'static str> {
    LAYOUTS
        .into_iter()
        .filter(|layout| *layout != "reftable" || *gix_testtools::GIT_VERSION >= (2, 45, 0))
}

mod oracle;
