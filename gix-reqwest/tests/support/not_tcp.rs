#![cfg(not(target_arch = "wasm32"))]
#![cfg(unix)]

use std::convert::Infallible;
use std::future::Future;
use std::sync::mpsc as std_mpsc;
use std::thread;
use std::time::Duration;

use tokio::runtime;
use tokio::sync::oneshot;

pub struct Server {
    path: std::path::PathBuf,
    panic_rx: std_mpsc::Receiver<()>,
    events_rx: std_mpsc::Receiver<Event>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

#[non_exhaustive]
pub enum Event {
    ConnectionClosed,
}

impl Server {
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub fn events(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        while let Ok(event) = self.events_rx.try_recv() {
            events.push(event);
        }
        events
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        if !::std::thread::panicking() {
            self.panic_rx
                .recv_timeout(Duration::from_secs(3))
                .expect("test server should not panic");
        }
    }
}

pub fn uds<F, Fut>(func: F) -> Server
where
    F: Fn(http::Request<hyper::body::Incoming>) -> Fut + Clone + Send + 'static,
    Fut: Future<Output = http::Response<reqwest::Body>> + Send + 'static,
{
    uds_with_config(func, |_builder| {})
}

type Builder = hyper_util::server::conn::auto::Builder<hyper_util::rt::TokioExecutor>;

pub fn uds_with_config<F1, Fut, F2, Bu>(func: F1, apply_config: F2) -> Server
where
    F1: Fn(http::Request<hyper::body::Incoming>) -> Fut + Clone + Send + 'static,
    Fut: Future<Output = http::Response<reqwest::Body>> + Send + 'static,
    F2: FnOnce(&mut Builder) -> Bu + Send + 'static,
{
    // Spawn new runtime in thread to prevent reactor execution context conflict
    let test_name = thread::current().name().unwrap_or("<unknown>").to_string();
    thread::spawn(move || {
        let rt = runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("new rt");
        let path = random_tmp_path();
        let listener = rt.block_on(async {
            tokio::net::UnixListener::bind(&path)
                .unwrap()
        });

        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        let (panic_tx, panic_rx) = std_mpsc::channel();
        let (events_tx, events_rx) = std_mpsc::channel();
        let tname = format!(
            "test({})-support-server",
            test_name,
        );

        let close_path = path.clone();

        thread::Builder::new()
            .name(tname)
            .spawn(move || {
                rt.block_on(async move {
                    let mut builder =
                        hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new());
                    apply_config(&mut builder);

                    loop {
                        tokio::select! {
                            _ = &mut shutdown_rx => {
                                break;
                            }
                            accepted = listener.accept() => {
                                let (io, _) = accepted.expect("accepted");
                                let func = func.clone();
                                let svc = hyper::service::service_fn(move |req| {
                                    let fut = func(req);
                                    async move { Ok::<_, Infallible>(fut.await) }
                                });
                                let builder = builder.clone();
                                let events_tx = events_tx.clone();
                                tokio::spawn(async move {
                                    let _ = builder.serve_connection_with_upgrades(hyper_util::rt::TokioIo::new(io), svc).await;
                                    let _ = events_tx.send(Event::ConnectionClosed);
                                });
                            }
                        }
                    }
                    let _ = std::fs::remove_file(close_path);
                    let _ = panic_tx.send(());
                });
            })
            .expect("thread spawn");
        Server {
            path,
            panic_rx,
            events_rx,
            shutdown_tx: Some(shutdown_tx),
        }
    })
    .join()
    .unwrap()
}

fn random_tmp_path() -> std::path::PathBuf {
    use std::hash::BuildHasher;

    let mut buf = std::env::temp_dir();

    // libstd uses system random to create each one
    let rng = std::collections::hash_map::RandomState::new();
    let n = rng.hash_one("reqwest-uds-sock");

    buf.push(format!("reqwest-test-uds-sock-{}", n));

    buf
}
