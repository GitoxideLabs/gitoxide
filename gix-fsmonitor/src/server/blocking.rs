use super::*;
use gix_macros::sync as maybe_async;

fn prepare_cookies<S>(_source: &S, layout: &Layout) -> io::Result<()> {
    layout.prepare_cookies()
}
fn create_cookie<S>(_source: &S, directory: &Path) -> io::Result<cookie::Cookie> {
    cookie::Cookie::create(directory)
}
fn roots_lost<S>(_source: &S, layout: &Layout) -> io::Result<bool> {
    Ok(layout.roots().any(|root| !root.exists()))
}
fn wait_for_cookie<S>(_source: &S, remaining: Duration, interval: Duration) {
    std::thread::park_timeout(remaining.min(interval));
}
include!("monitor.rs");
