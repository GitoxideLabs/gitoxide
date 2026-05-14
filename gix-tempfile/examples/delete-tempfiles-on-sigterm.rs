#[cfg(not(feature = "signals"))]
fn main() {
    panic!("The `signals` feature needs to be set to compile this example");
}

#[cfg(feature = "signals")]
fn main() -> std::io::Result<()> {
    use std::{
        io::{Write, stdout},
        path::PathBuf,
    };

    use gix_tempfile::{AutoRemove, ContainingDirectory};

    gix_tempfile::signal::setup(Default::default());
    let filepath = PathBuf::new().join("tempfile.ext");
    let _tempfile = gix_tempfile::mark_at(&filepath, ContainingDirectory::Exists, AutoRemove::Tempfile)?;
    assert!(filepath.is_file(), "a tempfile was created");

    writeln!(stdout(), "{}", filepath.display())?;
    stdout().flush()?;

    signal_hook::low_level::raise(signal_hook::consts::SIGTERM)?;
    unreachable!(
        "the above line aborts the process, and prevents destructors from running. The tempfile will go away nonetheless"
    );
}
