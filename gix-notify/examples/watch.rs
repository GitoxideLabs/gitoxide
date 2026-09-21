//! Run with `cargo run -p gix-notify --example watch -- <directory>`.
use std::{sync::mpsc, time::Duration};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os().nth(1).unwrap_or_else(|| ".".into());
    let path = std::fs::canonicalize(path)?;
    let (send, receive) = mpsc::sync_channel(1);
    let mut watcher = gix_notify::Watcher::new(Default::default()).map_err(gix_error::Exn::into_error)?;
    watcher.set_waker(move || {
        let _ = send.try_send(());
    });
    watcher
        .replace([gix_notify::Watch { path, recursive: true }])
        .map_err(gix_error::Exn::into_error)?;
    loop {
        let batch = watcher.drain(Default::default());
        if let Some(reason) = batch.loss {
            println!("Rescan required: {reason:?}");
        }
        for error in batch.errors {
            eprintln!("{error}");
        }
        for event in batch.events {
            println!("{:?}: {:?}", event.kind, event.paths);
        }
        if !batch.more {
            let _ = receive.recv_timeout(Duration::from_secs(60));
        }
    }
}
