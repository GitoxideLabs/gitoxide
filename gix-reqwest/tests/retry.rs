#![cfg(not(target_arch = "wasm32"))]
#![cfg(not(feature = "rustls-no-provider"))]
mod support;
use support::server;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[tokio::test]
async fn retries_apply_in_scope() {
    let _ = env_logger::try_init();
    let cnt = Arc::new(AtomicUsize::new(0));
    let server = server::http(move |_req| {
        let cnt = cnt.clone();
        async move {
            if cnt.fetch_add(1, Ordering::Relaxed) == 0 {
                // first req is bad
                http::Response::builder()
                    .status(http::StatusCode::SERVICE_UNAVAILABLE)
                    .body(Default::default())
                    .unwrap()
            } else {
                http::Response::default()
            }
        }
    });

    let scope = server.addr().ip().to_string();
    let retries = reqwest::retry::for_host(scope).classify_fn(|req_rep| {
        if req_rep.status() == Some(http::StatusCode::SERVICE_UNAVAILABLE) {
            req_rep.retryable()
        } else {
            req_rep.success()
        }
    });

    let url = format!("http://{}", server.addr());
    let resp = reqwest::Client::builder()
        .retry(retries)
        .build()
        .unwrap()
        .get(url)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
}

#[cfg(feature = "http2")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn default_retries_have_a_limit() {
    let _ = env_logger::try_init();

    let server = server::http_with_config(
        move |req| async move {
            assert_eq!(req.version(), http::Version::HTTP_2);
            // refused forever
            Err(h2::Error::from(h2::Reason::REFUSED_STREAM))
        },
        |_| {},
    );

    let client = reqwest::Client::builder()
        .http2_prior_knowledge()
        .build()
        .unwrap();

    let url = format!("http://{}", server.addr());

    let _err = client.get(url).send().await.unwrap_err();
}

// NOTE: using the default "current_thread" runtime here would cause the test to
// fail, because the only thread would block until `panic_rx` receives a
// notification while the client needs to be driven to get the graceful shutdown
// done.
#[cfg(feature = "http2")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn highly_concurrent_requests_to_http2_server_with_low_max_concurrent_streams() {
    let client = reqwest::Client::builder()
        .http2_prior_knowledge()
        .build()
        .unwrap();

    let server = server::http_with_config(
        move |req| async move {
            assert_eq!(req.version(), http::Version::HTTP_2);
            Ok::<_, std::convert::Infallible>(http::Response::default())
        },
        |builder| {
            builder.http2().max_concurrent_streams(1);
        },
    );

    let url = format!("http://{}", server.addr());

    let futs = (0..100).map(|_| {
        let client = client.clone();
        let url = url.clone();
        async move {
            let res = client.get(&url).send().await.unwrap();
            assert_eq!(res.status(), reqwest::StatusCode::OK);
        }
    });
    futures_util::future::join_all(futs).await;
}

#[cfg(feature = "http2")]
#[tokio::test]
async fn highly_concurrent_requests_to_slow_http2_server_with_low_max_concurrent_streams() {
    use support::delay_server;

    let client = reqwest::Client::builder()
        .http2_prior_knowledge()
        .build()
        .unwrap();

    let server = delay_server::Server::new(
        move |req| async move {
            assert_eq!(req.version(), http::Version::HTTP_2);
            http::Response::default()
        },
        |http| {
            http.http2().max_concurrent_streams(1);
        },
        std::time::Duration::from_secs(2),
    )
    .await;

    let url = format!("http://{}", server.addr());

    let futs = (0..100).map(|_| {
        let client = client.clone();
        let url = url.clone();
        async move {
            let res = client.get(&url).send().await.unwrap();
            assert_eq!(res.status(), reqwest::StatusCode::OK);
        }
    });
    futures_util::future::join_all(futs).await;

    server.shutdown().await;
}
