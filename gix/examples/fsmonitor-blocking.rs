//! `cargo run -p gix --features fsmonitor-server --example fsmonitor-blocking -- /path/to/worktree`
fn main() -> gix::Result<()> {
    use gix::fsmonitor::server::{
        Options,
        ipc::{self, Request},
    };
    let path = std::env::args_os().nth(1).unwrap_or_else(|| ".".into());
    let server = gix::discover(path)?.fsmonitor_server(Options::default())?;
    let response = ipc::query(server.endpoint(), Request::Message(b""))?;
    println!("Endpoint: {}\nBaseline: {response:?}", server.endpoint().display());
    server.shutdown()
}
