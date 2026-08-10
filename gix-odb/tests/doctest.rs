// Shared setup for crate-level doctests in `src/lib.rs`.
// This file is included there, and not part of the test harness.

/// Return an empty object database that lives as long as its temporary directory.
pub fn empty_store() -> Result<(tempfile::TempDir, gix_odb::Handle), std::io::Error> {
    let dir = tempfile::tempdir()?;
    let store = gix_odb::at_opts(
        dir.path(),
        None,
        gix_odb::store::init::Options {
            object_hash: gix_testtools::object_hash(),
            ..Default::default()
        },
    )?;
    Ok((dir, store))
}
