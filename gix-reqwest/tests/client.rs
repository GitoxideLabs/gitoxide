#![cfg(not(target_arch = "wasm32"))]
#![cfg(not(feature = "rustls-no-provider"))]
mod support;

use support::server;

use http::header::{self, CONTENT_LENGTH, CONTENT_TYPE, COOKIE, TRANSFER_ENCODING, USER_AGENT};
#[cfg(feature = "json")]
use std::collections::HashMap;

use gix_reqwest::Client;
use tokio::io::AsyncWriteExt;

#[tokio::test]
async fn auto_headers() {
    let server = server::http(move |req| async move {
        assert_eq!(req.method(), "GET");

        assert_eq!(req.headers()["accept"], "*/*");
        assert_eq!(req.headers().get("user-agent"), None);
        if cfg!(feature = "gzip") {
            assert!(req.headers()["accept-encoding"]
                .to_str()
                .unwrap()
                .contains("gzip"));
        }
        if cfg!(feature = "brotli") {
            assert!(req.headers()["accept-encoding"]
                .to_str()
                .unwrap()
                .contains("br"));
        }
        if cfg!(feature = "zstd") {
            assert!(req.headers()["accept-encoding"]
                .to_str()
                .unwrap()
                .contains("zstd"));
        }
        if cfg!(feature = "deflate") {
            assert!(req.headers()["accept-encoding"]
                .to_str()
                .unwrap()
                .contains("deflate"));
        }

        http::Response::default()
    });

    let url = format!("http://{}/1", server.addr());
    let res = gix_reqwest::Client::builder()
        .no_proxy()
        .build()
        .unwrap()
        .get(&url)
        .send()
        .await
        .unwrap();

    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
    assert_eq!(res.remote_addr(), Some(server.addr()));
}

#[tokio::test]
async fn test_overwrite_headers() {
    let server = server::http(move |req| async move {
        let path = req.uri().path();
        match path {
            "/1" => {
                assert_eq!(req.method(), "GET");
                assert_eq!(req.headers()[USER_AGENT], "my-custom-agent");
                let mut cookies = req.headers().get_all(COOKIE).iter();
                assert_eq!(cookies.next().unwrap(), "a=b");
                assert_eq!(cookies.next().unwrap(), "c=d");
                assert_eq!(cookies.next(), None);
            }
            "/2" => {
                assert_eq!(req.method(), "GET");
                assert_eq!(req.headers()[USER_AGENT], "my-custom-agent");
                let mut cookies = req.headers().get_all(COOKIE).iter();
                assert_eq!(cookies.next().unwrap(), "e=f");
                assert_eq!(cookies.next().unwrap(), "g=h");
                assert_eq!(cookies.next(), None);
            }
            "/3" => {
                assert_eq!(req.method(), "GET");
                assert_eq!(req.headers()[USER_AGENT], "default-agent");
                let mut cookies = req.headers().get_all(COOKIE).iter();
                assert_eq!(cookies.next().unwrap(), "a=b");
                assert_eq!(cookies.next().unwrap(), "c=d");
                assert_eq!(cookies.next(), None);
            }
            "/4" => {
                assert_eq!(req.method(), "GET");
                assert_eq!(req.headers()[USER_AGENT], "default-agent");
                let mut cookies = req.headers().get_all(COOKIE).iter();
                assert_eq!(cookies.next().unwrap(), "e=f");
                assert_eq!(cookies.next().unwrap(), "g=h");
                assert_eq!(cookies.next(), None);
            }
            _ => {
                unreachable!("Unexpected request path: {}", path);
            }
        }

        http::Response::default()
    });

    let mut default_headers = header::HeaderMap::new();
    default_headers.insert(
        USER_AGENT,
        header::HeaderValue::from_static("default-agent"),
    );
    default_headers.insert(COOKIE, header::HeaderValue::from_static("a=b"));
    default_headers.append(COOKIE, header::HeaderValue::from_static("c=d"));

    let client = Client::builder()
        .no_proxy()
        .default_headers(default_headers)
        .build()
        .unwrap();

    let url = format!("http://{}/1", server.addr());
    let res = client
        .get(&url)
        .header(USER_AGENT, "my-custom-agent")
        .send()
        .await
        .unwrap();
    assert_eq!(res.url().as_str(), url);
    assert_eq!(res.status(), http::StatusCode::OK);

    let url = format!("http://{}/2", server.addr());
    let res = client
        .get(&url)
        .headers({
            let mut headers = header::HeaderMap::new();
            headers.insert(
                USER_AGENT,
                header::HeaderValue::from_static("my-custom-agent"),
            );
            headers.insert(COOKIE, header::HeaderValue::from_static("e=f"));
            headers.append(COOKIE, header::HeaderValue::from_static("g=h"));
            headers
        })
        .send()
        .await
        .unwrap();
    assert_eq!(res.url().as_str(), url);
    assert_eq!(res.status(), http::StatusCode::OK);

    let url = format!("http://{}/3", server.addr());
    let res = client.get(&url).send().await.unwrap();
    assert_eq!(res.url().as_str(), url);
    assert_eq!(res.status(), http::StatusCode::OK);

    let url = format!("http://{}/4", server.addr());
    let res = client
        .get(&url)
        .headers({
            let mut headers = header::HeaderMap::new();
            headers.insert(COOKIE, header::HeaderValue::from_static("e=f"));
            headers.append(COOKIE, header::HeaderValue::from_static("g=h"));
            headers
        })
        .send()
        .await
        .unwrap();
    assert_eq!(res.url().as_str(), url);
    assert_eq!(res.status(), http::StatusCode::OK);
}

#[tokio::test]
async fn donot_set_content_length_0_if_have_no_body() {
    let server = server::http(move |req| async move {
        let headers = req.headers();
        assert_eq!(headers.get(CONTENT_LENGTH), None);
        assert!(headers.get(CONTENT_TYPE).is_none());
        assert!(headers.get(TRANSFER_ENCODING).is_none());
        dbg!(&headers);
        http::Response::default()
    });

    let url = format!("http://{}/content-length", server.addr());
    let res = gix_reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("client builder")
        .get(&url)
        .send()
        .await
        .expect("request");

    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
}

#[tokio::test]
async fn user_agent() {
    let server = server::http(move |req| async move {
        assert_eq!(req.headers()["user-agent"], "reqwest-test-agent");
        http::Response::default()
    });

    let url = format!("http://{}/ua", server.addr());
    let res = gix_reqwest::Client::builder()
        .user_agent("reqwest-test-agent")
        .build()
        .expect("client builder")
        .get(&url)
        .send()
        .await
        .expect("request");

    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
}

#[tokio::test]
async fn response_text() {
    let _ = env_logger::try_init();

    let server = server::http(move |_req| async { http::Response::new("Hello".into()) });

    let client = Client::new();

    let res = client
        .get(&format!("http://{}/text", server.addr()))
        .send()
        .await
        .expect("Failed to get");
    assert_eq!(res.content_length(), Some(5));
    let text = res.text().await.expect("Failed to get text");
    assert_eq!("Hello", text);
}

#[tokio::test]
async fn response_bytes() {
    let _ = env_logger::try_init();

    let server = server::http(move |_req| async { http::Response::new("Hello".into()) });

    let client = Client::new();

    let res = client
        .get(&format!("http://{}/bytes", server.addr()))
        .send()
        .await
        .expect("Failed to get");
    assert_eq!(res.content_length(), Some(5));
    let bytes = res.bytes().await.expect("res.bytes()");
    assert_eq!("Hello", bytes);
}

#[tokio::test]
#[cfg(feature = "json")]
async fn response_json() {
    let _ = env_logger::try_init();

    let server = server::http(move |_req| async { http::Response::new("\"Hello\"".into()) });

    let client = Client::new();

    let res = client
        .get(&format!("http://{}/json", server.addr()))
        .send()
        .await
        .expect("Failed to get");
    let text = res.json::<String>().await.expect("Failed to get json");
    assert_eq!("Hello", text);
}

#[tokio::test]
async fn body_pipe_response() {
    use http_body_util::BodyExt;
    let _ = env_logger::try_init();

    let server = server::http(move |req| async move {
        if req.uri() == "/get" {
            http::Response::new("pipe me".into())
        } else {
            assert_eq!(req.uri(), "/pipe");
            assert_eq!(req.headers()["content-length"], "7");

            let full: Vec<u8> = req
                .into_body()
                .collect()
                .await
                .expect("must succeed")
                .to_bytes()
                .to_vec();

            assert_eq!(full, b"pipe me");

            http::Response::default()
        }
    });

    let client = Client::new();

    let res1 = client
        .get(&format!("http://{}/get", server.addr()))
        .send()
        .await
        .expect("get1");

    assert_eq!(res1.status(), gix_reqwest::StatusCode::OK);
    assert_eq!(res1.content_length(), Some(7));

    // and now ensure we can "pipe" the response to another request
    let res2 = client
        .post(&format!("http://{}/pipe", server.addr()))
        .body(res1)
        .send()
        .await
        .expect("res2");

    assert_eq!(res2.status(), gix_reqwest::StatusCode::OK);
}

struct FailingResolver;

impl gix_reqwest::dns::Resolve for FailingResolver {
    fn resolve(&self, _name: gix_reqwest::dns::Name) -> gix_reqwest::dns::Resolving {
        Box::pin(async { Err("simulated resolver failure".into()) })
    }
}

#[tokio::test]
async fn dns_resolution_failure_is_dns_error() {
    let _ = env_logger::builder().is_test(true).try_init();

    let client = gix_reqwest::Client::builder()
        .no_proxy()
        .dns_resolver(std::sync::Arc::new(FailingResolver))
        .build()
        .expect("client builder");

    let err = client
        .get("http://does.not.resolve.invalid/")
        .send()
        .await
        .expect_err("request should fail during resolution");

    assert!(err.is_dns(), "expected a DNS error, got: {err:?}");
    // DNS errors are a refinement of connect errors.
    assert!(err.is_connect(), "expected is_connect() to also be true");
}

#[tokio::test]
async fn overridden_dns_resolution_with_gai() {
    let _ = env_logger::builder().is_test(true).try_init();
    let server = server::http(move |_req| async { http::Response::new("Hello".into()) });

    let overridden_domain = "rust-lang.org";
    let url = format!(
        "http://{overridden_domain}:{}/domain_override",
        server.addr().port()
    );
    let client = gix_reqwest::Client::builder()
        .no_proxy()
        .resolve(overridden_domain, server.addr())
        .build()
        .expect("client builder");
    let req = client.get(&url);
    let res = req.send().await.expect("request");

    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
    let text = res.text().await.expect("Failed to get text");
    assert_eq!("Hello", text);
}

#[tokio::test]
async fn overridden_dns_resolution_with_gai_multiple() {
    let _ = env_logger::builder().is_test(true).try_init();
    let server = server::http(move |_req| async { http::Response::new("Hello".into()) });

    let overridden_domain = "rust-lang.org";
    let url = format!(
        "http://{overridden_domain}:{}/domain_override",
        server.addr().port()
    );
    // the server runs on IPv4 localhost, so provide both IPv4 and IPv6 and let the happy eyeballs
    // algorithm decide which address to use.
    let client = gix_reqwest::Client::builder()
        .no_proxy()
        .resolve_to_addrs(
            overridden_domain,
            &[
                std::net::SocketAddr::new(
                    std::net::IpAddr::V6(std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
                    server.addr().port(),
                ),
                server.addr(),
            ],
        )
        .build()
        .expect("client builder");
    let req = client.get(&url);
    let res = req.send().await.expect("request");

    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
    let text = res.text().await.expect("Failed to get text");
    assert_eq!("Hello", text);
}

#[cfg(feature = "hickory-dns")]
#[tokio::test]
async fn overridden_dns_resolution_with_hickory_dns() {
    let _ = env_logger::builder().is_test(true).try_init();
    let server = server::http(move |_req| async { http::Response::new("Hello".into()) });

    let overridden_domain = "rust-lang.org";
    let url = format!(
        "http://{overridden_domain}:{}/domain_override",
        server.addr().port()
    );
    let client = gix_reqwest::Client::builder()
        .no_proxy()
        .resolve(overridden_domain, server.addr())
        .hickory_dns(true)
        .build()
        .expect("client builder");
    let req = client.get(&url);
    let res = req.send().await.expect("request");

    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
    let text = res.text().await.expect("Failed to get text");
    assert_eq!("Hello", text);
}

#[cfg(feature = "hickory-dns")]
#[tokio::test]
async fn overridden_dns_resolution_with_hickory_dns_multiple() {
    let _ = env_logger::builder().is_test(true).try_init();
    let server = server::http(move |_req| async { http::Response::new("Hello".into()) });

    let overridden_domain = "rust-lang.org";
    let url = format!(
        "http://{overridden_domain}:{}/domain_override",
        server.addr().port()
    );
    // the server runs on IPv4 localhost, so provide both IPv4 and IPv6 and let the happy eyeballs
    // algorithm decide which address to use.
    let client = gix_reqwest::Client::builder()
        .no_proxy()
        .resolve_to_addrs(
            overridden_domain,
            &[
                std::net::SocketAddr::new(
                    std::net::IpAddr::V6(std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
                    server.addr().port(),
                ),
                server.addr(),
            ],
        )
        .hickory_dns(true)
        .build()
        .expect("client builder");
    let req = client.get(&url);
    let res = req.send().await.expect("request");

    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
    let text = res.text().await.expect("Failed to get text");
    assert_eq!("Hello", text);
}

#[cfg(any(feature = "__native-tls", feature = "__rustls",))]
#[test]
fn use_preconfigured_tls_with_bogus_backend() {
    struct DefinitelyNotTls;

    gix_reqwest::Client::builder()
        .use_preconfigured_tls(DefinitelyNotTls)
        .build()
        .expect_err("definitely is not TLS");
}

#[cfg(feature = "__native-tls")]
#[test]
fn use_preconfigured_native_tls_default() {
    extern crate native_tls_crate;

    let tls = native_tls_crate::TlsConnector::builder()
        .build()
        .expect("tls builder");

    gix_reqwest::Client::builder()
        .use_preconfigured_tls(tls)
        .build()
        .expect("preconfigured default tls");
}

#[cfg(feature = "rustls")] // needs a TLS provider
#[test]
fn use_preconfigured_rustls_default() {
    extern crate rustls;

    let root_cert_store = rustls::RootCertStore::empty();
    let tls = rustls::ClientConfig::builder_with_provider(std::sync::Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_root_certificates(root_cert_store)
    .with_no_client_auth();

    gix_reqwest::Client::builder()
        .use_preconfigured_tls(tls)
        .build()
        .expect("preconfigured rustls tls");
}

#[cfg(all(feature = "__tls", not(any(feature = "http2", feature = "http3")),))]
#[tokio::test]
async fn http1_only() {
    let res = gix_reqwest::Client::builder()
        .build()
        .expect("client builder")
        .get("https://google.com")
        .send()
        .await
        .expect("request");

    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
    assert_eq!(res.version(), gix_reqwest::Version::HTTP_11);
}

#[cfg(feature = "__rustls")]
#[tokio::test]
#[ignore = "Needs TLS support in the test server"]
async fn http2_upgrade() {
    let server = server::http(move |_| async move { http::Response::default() });

    let url = format!("https://localhost:{}", server.addr().port());
    let res = gix_reqwest::Client::builder()
        .tls_danger_accept_invalid_certs(true)
        .tls_backend_rustls()
        .build()
        .expect("client builder")
        .get(&url)
        .send()
        .await
        .expect("request");

    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
    assert_eq!(res.version(), gix_reqwest::Version::HTTP_2);
}

#[cfg(feature = "default-tls")]
#[cfg_attr(feature = "http3", ignore = "enabling http3 seems to break this, why?")]
#[tokio::test]
async fn test_allowed_methods() {
    let resp = gix_reqwest::Client::builder()
        .https_only(true)
        .build()
        .expect("client builder")
        .get("https://google.com")
        .send()
        .await;

    assert!(resp.is_ok());

    let resp = gix_reqwest::Client::builder()
        .https_only(true)
        .build()
        .expect("client builder")
        .get("http://google.com")
        .send()
        .await;

    assert!(resp.is_err());
}

#[test]
#[cfg(feature = "json")]
fn add_json_default_content_type_if_not_set_manually() {
    let mut map = HashMap::new();
    map.insert("body", "json");
    let content_type = http::HeaderValue::from_static("application/vnd.api+json");
    let req = Client::new()
        .post("https://google.com/")
        .header(CONTENT_TYPE, &content_type)
        .json(&map)
        .build()
        .expect("request is not valid");

    assert_eq!(content_type, req.headers().get(CONTENT_TYPE).unwrap());
}

#[test]
#[cfg(feature = "json")]
fn update_json_content_type_if_set_manually() {
    let mut map = HashMap::new();
    map.insert("body", "json");
    let req = Client::new()
        .post("https://google.com/")
        .json(&map)
        .build()
        .expect("request is not valid");

    assert_eq!("application/json", req.headers().get(CONTENT_TYPE).unwrap());
}

#[cfg(all(feature = "__tls", not(feature = "rustls-no-provider")))]
#[tokio::test]
async fn test_tls_info() {
    let resp = gix_reqwest::Client::builder()
        .tls_info(true)
        .build()
        .expect("client builder")
        .get("https://google.com")
        .send()
        .await
        .expect("response");
    let tls_info = resp.extensions().get::<gix_reqwest::tls::TlsInfo>();
    assert!(tls_info.is_some());
    let tls_info = tls_info.unwrap();
    let peer_certificate = tls_info.peer_certificate();
    assert!(peer_certificate.is_some());
    let der = peer_certificate.unwrap();
    assert_eq!(der[0], 0x30); // ASN.1 SEQUENCE

    let resp = gix_reqwest::Client::builder()
        .build()
        .expect("client builder")
        .get("https://google.com")
        .send()
        .await
        .expect("response");
    let tls_info = resp.extensions().get::<gix_reqwest::tls::TlsInfo>();
    assert!(tls_info.is_none());
}

#[cfg(all(feature = "__rustls", not(feature = "rustls-no-provider")))]
#[tokio::test]
async fn test_tls_info_version_rustls() {
    let resp = gix_reqwest::Client::builder()
        .tls_backend_rustls()
        .tls_info(true)
        .build()
        .expect("client builder")
        .get("https://google.com")
        .send()
        .await
        .expect("response");
    let tls_info = resp
        .extensions()
        .get::<gix_reqwest::tls::TlsInfo>()
        .expect("tls info");
    let version = tls_info.version().expect("negotiated version");
    assert!(
        version >= gix_reqwest::tls::Version::TLS_1_2,
        "negotiated {version:?}"
    );
}

// native-tls cannot report the negotiated version, so it stays `None` even
// though the rest of the `TlsInfo` is populated.
#[cfg(feature = "__native-tls")]
#[tokio::test]
async fn test_tls_info_version_native_tls() {
    let resp = gix_reqwest::Client::builder()
        .tls_backend_native()
        .tls_info(true)
        .build()
        .expect("client builder")
        .get("https://google.com")
        .send()
        .await
        .expect("response");
    let tls_info = resp
        .extensions()
        .get::<gix_reqwest::tls::TlsInfo>()
        .expect("tls info");
    assert!(tls_info.peer_certificate().is_some());
    assert!(tls_info.version().is_none());
}

#[tokio::test]
async fn close_connection_after_idle_timeout() {
    let mut server = server::http(move |_| async move { http::Response::default() });

    let client = gix_reqwest::Client::builder()
        .pool_idle_timeout(std::time::Duration::from_secs(1))
        .build()
        .unwrap();

    let url = format!("http://{}", server.addr());

    client.get(&url).send().await.unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    assert!(server
        .events()
        .iter()
        .any(|e| matches!(e, server::Event::ConnectionClosed)));
}

#[tokio::test]
async fn http1_reason_phrase() {
    let server = server::low_level_with_response(|_raw_request, client_socket| {
        Box::new(async move {
            client_socket
                .write_all(b"HTTP/1.1 418 I'm not a teapot\r\nContent-Length: 0\r\n\r\n")
                .await
                .expect("response write_all failed");
        })
    });

    let client = Client::new();

    let res = client
        .get(&format!("http://{}", server.addr()))
        .send()
        .await
        .expect("Failed to get");

    assert_eq!(
        res.error_for_status().unwrap_err().to_string(),
        format!(
            "HTTP status client error (418 I'm not a teapot) for url (http://{}/)",
            server.addr()
        )
    );
}

#[tokio::test]
async fn error_has_url() {
    let u = "http://does.not.exist.local/ever";
    let err = gix_reqwest::get(u).await.unwrap_err();
    assert_eq!(err.url().map(AsRef::as_ref), Some(u), "{err:?}");
}

#[tokio::test]
async fn http1_max_headers() {
    // The server responds with 150 headers, well over hyper's default limit of 100.
    let server = server::http(move |_req| async move {
        let mut builder = http::Response::builder();
        for i in 0..150 {
            builder = builder.header(format!("x-custom-{i}"), "value");
        }
        builder.body("Hello".into()).unwrap()
    });

    let url = format!("http://{}/", server.addr());

    // The default client uses hyper's 100-header limit and rejects the response.
    gix_reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .expect_err("the default 100-header limit should reject a 150-header response");

    // Raising the limit lets the same response through.
    let res = gix_reqwest::Client::builder()
        .http1_max_headers(300)
        .build()
        .unwrap()
        .get(&url)
        .send()
        .await
        .expect("a raised http1_max_headers should accept the response");

    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
    assert_eq!(res.headers()["x-custom-0"], "value");
    assert_eq!(res.headers()["x-custom-149"], "value");
}

#[cfg(feature = "stream")]
#[tokio::test]
async fn response_before_streamed_body_is_sent() {
    use tokio::io::AsyncReadExt;

    // This server responds without reading the request body, then waits for
    // the client to close the connection before letting the body produce
    // any bytes. This covers the case where the response is received
    // before the body is fully sent, e.g. an HTTP 412 for a failed
    // `If-None-Match: *` precondition.
    let (closed_tx, mut closed_rx) = tokio::sync::mpsc::unbounded_channel();
    let server = server::low_level_with_response(move |_raw_request, client_socket| {
        let closed_tx = closed_tx.clone();
        Box::new(async move {
            client_socket
                .write_all(
                    b"HTTP/1.1 412 Precondition Failed\r\n\
                      Content-Length: 0\r\n\
                      Connection: close\r\n\
                      \r\n",
                )
                .await
                .expect("response write_all failed");
            client_socket.shutdown().await.expect("shutdown failed");
            let _ = client_socket.read_to_end(&mut Vec::new()).await;
            let _ = closed_tx.send(());
        })
    });

    let body = gix_reqwest::Body::wrap_stream(futures_util::stream::once(async move {
        closed_rx.recv().await;
        Ok::<_, std::convert::Infallible>(vec![b'x'; 16])
    }));
    let res = Client::new()
        .put(format!("http://{}/object", server.addr()))
        .header(CONTENT_LENGTH, 16)
        .body(body)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), gix_reqwest::StatusCode::PRECONDITION_FAILED);
}
