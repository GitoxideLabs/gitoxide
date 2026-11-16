#![cfg(not(target_arch = "wasm32"))]
#![cfg(not(feature = "rustls-no-provider"))]
#![cfg(unix)]

mod support;

#[tokio::test]
async fn unix_socket_works() {
    let server = support::not_tcp::uds(move |_| async move { http::Response::default() });

    let res = reqwest::Client::builder()
        .unix_socket(server.path())
        .build()
        .unwrap()
        .get("http://yolo.local/foo")
        .send()
        .await
        .expect("send request");

    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn unix_socket_ignores_proxies() {
    let server = support::not_tcp::uds(move |_| async move { http::Response::default() });

    let res = reqwest::Client::builder()
        .unix_socket(server.path())
        .proxy(reqwest::Proxy::http("http://dont.use.me.local").unwrap())
        .build()
        .unwrap()
        .get("http://yolo.local/foo")
        .send()
        .await
        .expect("send request");

    assert_eq!(res.status(), 200);
}

// TODO: enable when test server supports TLS
#[ignore]
#[tokio::test]
async fn unix_socket_uses_tls() {
    let server = support::not_tcp::uds(move |_| async move { http::Response::default() });

    let res = reqwest::Client::builder()
        .unix_socket(server.path())
        .build()
        .unwrap()
        .get("https://yolo.local/foo")
        .send()
        .await
        .expect("send request");

    assert_eq!(res.status(), 200);
}
