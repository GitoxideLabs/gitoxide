//! `cargo run -p gix --features async-fsmonitor-server --example fsmonitor-async -- /path/to/worktree`
//! The opt-in feature supplies Tokio; blocking APIs remain available in the same build.
#[tokio::main(flavor = "current_thread")]
async fn main() -> gix::Result<()> {
    use gix::fsmonitor::server::{
        Options,
        ipc::{self, Request},
    };
    let path = std::env::args_os().nth(1).unwrap_or_else(|| ".".into());
    let server = gix::discover(path)?.fsmonitor_server_async(Options::default()).await?;
    let response = ipc::query_async(server.endpoint(), Request::Message(b"")).await?;
    println!("Endpoint: {}\nBaseline: {response:?}", server.endpoint().display());
    server.shutdown().await
}
