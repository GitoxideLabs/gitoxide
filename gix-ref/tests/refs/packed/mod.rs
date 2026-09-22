use crate::Result;
use std::path::PathBuf;

pub fn write_packed_refs_with(input: &[u8]) -> Result<(gix_testtools::tempfile::TempDir, PathBuf)> {
    let dir = gix_testtools::tempfile::tempdir()?;
    let packed_refs_path = dir.path().join("packed-refs");
    std::fs::write(&packed_refs_path, input)?;
    Ok((dir, packed_refs_path))
}

mod find;
pub mod iter;
mod open;
