#![cfg(not(target_arch = "wasm32"))]
#![cfg(not(feature = "rustls-no-provider"))]
mod support;
use support::server;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[tokio::test]
async fn retries_apply_in_scope() -> Result<(), gix_reqwest::Error> {
    let _ = env_logger::try_init();
    let origin = server::http(|req| async move {
        assert_eq!(
            req.headers()["x-shared"],
            "origin",
            "origin headers survive proxy retries"
        );
        assert!(
            !req.headers().contains_key("x-proxy-only"),
            "proxy headers do not reach a bypassed origin"
        );
        assert!(
            !req.headers().contains_key("proxy-authorization"),
            "proxy credentials do not reach a bypassed origin"
        );
        http::Response::default()
    });
    let destination = format!("http://{}/final", origin.addr());
    let cnt = Arc::new(AtomicUsize::new(0));
    let requests = cnt.clone();
    let server = server::http(move |req| {
        let cnt = cnt.clone();
        let destination = destination.clone();
        async move {
            assert_eq!(
                req.headers()["x-shared"],
                "proxy",
                "each retry keeps the proxy overlay"
            );
            assert_eq!(
                req.headers()["x-proxy-only"],
                "proxy",
                "each retry keeps proxy-only headers"
            );
            assert_eq!(
                req.headers()["proxy-authorization"],
                "Basic dXNlcjpwYXNz",
                "each retry keeps the selected proxy's credentials"
            );
            if cnt.fetch_add(1, Ordering::Relaxed) == 0 {
                // first req is bad
                http::Response::builder()
                    .status(http::StatusCode::SERVICE_UNAVAILABLE)
                    .body(Default::default())
                    .expect("valid retryable response")
            } else {
                http::Response::builder()
                    .status(http::StatusCode::FOUND)
                    .header(http::header::LOCATION, destination)
                    .body(Default::default())
                    .expect("valid redirect to the bypassed origin")
            }
        }
    });

    let retries = gix_reqwest::retry::for_host("origin.local").classify_fn(|req_rep| {
        if req_rep.status() == Some(http::StatusCode::SERVICE_UNAVAILABLE) {
            req_rep.retryable()
        } else {
            req_rep.success()
        }
    });

    let mut headers = gix_reqwest::header::HeaderMap::new();
    headers.insert(
        "x-shared",
        gix_reqwest::header::HeaderValue::from_static("proxy"),
    );
    headers.insert(
        "x-proxy-only",
        gix_reqwest::header::HeaderValue::from_static("proxy"),
    );
    let proxy = gix_reqwest::Proxy::http(format!("http://{}", server.addr()))?
        .basic_auth("user", "pass")
        .headers(headers)
        .no_proxy(gix_reqwest::NoProxy::from_string("127.0.0.1"));
    let resp = gix_reqwest::Client::builder()
        .retry(retries)
        .proxy(proxy)
        .build()?
        .get("http://origin.local/start")
        .header("x-shared", "origin")
        .send()
        .await?;

    assert_eq!(
        resp.status(),
        200,
        "the redirect reaches the bypassed origin"
    );
    assert_eq!(
        requests.load(Ordering::Relaxed),
        2,
        "the selected proxy received one retry"
    );
    Ok(())
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

    let client = gix_reqwest::Client::builder()
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
    let client = gix_reqwest::Client::builder()
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
            assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
        }
    });
    futures_util::future::join_all(futs).await;
}

#[cfg(feature = "http2")]
#[tokio::test]
async fn highly_concurrent_requests_to_slow_http2_server_with_low_max_concurrent_streams() {
    use support::delay_server;

    let client = gix_reqwest::Client::builder()
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
            assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
        }
    });
    futures_util::future::join_all(futs).await;

    server.shutdown().await;
}
