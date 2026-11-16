#![cfg(not(target_arch = "wasm32"))]
#![cfg(not(feature = "rustls-no-provider"))]
mod support;
use support::server;

#[tokio::test]
#[should_panic(expected = "test server should not panic")]
async fn server_panics_should_propagate() {
    let server = server::http(|_| async {
        panic!("kaboom");
    });

    let _ = reqwest::get(format!("http://{}/ci", server.addr())).await;
}
