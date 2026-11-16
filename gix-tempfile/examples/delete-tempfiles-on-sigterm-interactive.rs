#[cfg(not(feature = "signals"))]
fn main() {
    panic!("The `signals` feature needs to be set to compile this example");
}

#[cfg(feature = "signals")]
fn main() -> std::io::Result<()> {
    use std::path::PathBuf;

    use gix_tempfile::{AutoRemove, ContainingDirectory};

    gix_tempfile::signal::setup(Default::default());
    let filepath = PathBuf::new().join("writable-tempfile.ext");
    let markerpath = PathBuf::new().join("marker.ext");
    let _tempfile = gix_tempfile::writable_at(&filepath, ContainingDirectory::Exists, AutoRemove::Tempfile)?;
    let _markerfile = gix_tempfile::mark_at(&markerpath, ContainingDirectory::Exists, AutoRemove::Tempfile)?;
    eprintln!(
        "Observe the tempfiles at {} and {} and hit Ctrl+C to see it vanish. I will go to sleep now…",
        filepath.display(),
        markerpath.display()
    );
    std::thread::sleep(std::time::Duration::from_hours(8));
    Ok(())
}
