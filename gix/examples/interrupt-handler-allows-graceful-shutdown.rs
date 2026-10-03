use gix::Result;

#[cfg(not(feature = "interrupt"))]
fn main() -> Result<()> {
    gix::error::bail!("Needs 'interrupt' feature toggle to be enabled");
}

#[cfg(feature = "interrupt")]
fn main() -> Result<()> {
    use gix::error::ResultExt;
    use gix_tempfile::{AutoRemove, ContainingDirectory};
    // SAFETY: The closure doesn't use mutexes or memory allocation, so it should be safe to call from a signal handler.
    unsafe {
        gix::interrupt::init_handler(1, || {})?;
    }
    eprintln!("About to emit the first term signal");
    let tempfile_path = std::path::Path::new("example-file.tmp");
    let _keep_tempfile =
        gix_tempfile::mark_at(tempfile_path, ContainingDirectory::Exists, AutoRemove::Tempfile).or_error()?;

    signal_hook::low_level::raise(signal_hook::consts::SIGTERM).or_error()?;
    eprintln!(
        "Still here to showdown gracefully, our handler was triggered to kick that off. Tempfiles are still present."
    );
    assert!(tempfile_path.is_file());
    eprintln!("The next signal will abort this process but leave no tempfile nonetheless");
    signal_hook::low_level::raise(signal_hook::consts::SIGTERM).or_error()?;
    unreachable!("the above aborts");
}
