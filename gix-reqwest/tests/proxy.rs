#![cfg(not(target_arch = "wasm32"))]
#![cfg(not(feature = "rustls-no-provider"))]
mod support;
use support::server;

use std::env;

use std::sync::LazyLock;
use tokio::sync::Mutex;

// serialize tests that read from / write to environment variables
static HTTP_PROXY_ENV_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[cfg(all(feature = "socks", not(feature = "__tls")))]
#[tokio::test]
async fn without_tls_https_is_rejected_before_direct_or_socks_connection() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .expect("a loopback recording listener can be created");
    listener
        .set_nonblocking(true)
        .expect("the listener supports checking for unexpected connections");
    let authority = listener
        .local_addr()
        .expect("the bound listener has an address");
    let proxy = gix_reqwest::Proxy::all(format!("socks5h://{authority}"))
        .expect("the recording proxy URL is valid");

    for proxy in [None, Some(proxy)] {
        let mut builder = gix_reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_millis(200));
        if let Some(proxy) = proxy {
            builder = builder.proxy(proxy);
        }
        let error = builder
            .build()
            .expect("the no-TLS client configuration is valid")
            .get(format!("https://{authority}/"))
            .send()
            .await
            .expect_err("HTTPS requires TLS even when a SOCKS proxy is selected");
        assert!(
            error.is_connect(),
            "unsupported HTTPS is a connection error"
        );
        assert_eq!(
            listener
                .accept()
                .expect_err("HTTPS is rejected before any TCP dial")
                .kind(),
            std::io::ErrorKind::WouldBlock,
            "neither direct nor SOCKS connections can send plaintext HTTPS"
        );
    }
}

#[tokio::test]
async fn http_proxy() {
    let url = "http://hyper.rs.local/prox";
    let server = server::http(move |req| {
        assert_eq!(req.method(), "GET");
        assert_eq!(req.uri(), url);
        assert_eq!(req.headers()["host"], "hyper.rs.local");

        async { http::Response::default() }
    });

    let proxy = format!("http://{}", server.addr());

    let res = gix_reqwest::Client::builder()
        .proxy(gix_reqwest::Proxy::http(&proxy).unwrap())
        .build()
        .unwrap()
        .get(url)
        .send()
        .await
        .unwrap();

    assert_eq!(res.url().as_str(), url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
}

#[tokio::test]
async fn http_proxy_basic_auth() {
    let url = "http://hyper.rs.local/prox";
    let server = server::http(move |req| {
        assert_eq!(req.method(), "GET");
        assert_eq!(req.uri(), url);
        assert_eq!(req.headers()["host"], "hyper.rs.local");
        assert_eq!(
            req.headers()["proxy-authorization"],
            "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ=="
        );

        async { http::Response::default() }
    });

    let proxy = format!("http://{}", server.addr());

    let res = gix_reqwest::Client::builder()
        .proxy(
            gix_reqwest::Proxy::http(&proxy)
                .unwrap()
                .basic_auth("Aladdin", "open sesame"),
        )
        .build()
        .unwrap()
        .get(url)
        .send()
        .await
        .unwrap();

    assert_eq!(res.url().as_str(), url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
}

#[tokio::test]
async fn http_proxy_does_not_use_later_proxy_basic_auth() {
    http_proxy_does_not_use_later_proxy_headers(false).await;
}

#[tokio::test]
async fn http_proxy_does_not_use_later_proxy_custom_headers() {
    http_proxy_does_not_use_later_proxy_headers(true).await;
}

async fn http_proxy_does_not_use_later_proxy_headers(custom: bool) {
    let url = "http://hyper.rs.local/prox";
    let server = server::http(move |req| {
        assert!(!req.headers().contains_key("proxy-authorization"));
        assert!(!req.headers().contains_key("x-proxy-secret"));

        async { http::Response::default() }
    });

    let later_proxy = gix_reqwest::Proxy::http("http://unused.proxy.local").unwrap();
    let later_proxy = if custom {
        let mut headers = gix_reqwest::header::HeaderMap::new();
        headers.insert("proxy-authorization", "secret".parse().unwrap());
        headers.insert("x-proxy-secret", "secret".parse().unwrap());
        later_proxy.headers(headers)
    } else {
        later_proxy.basic_auth("Aladdin", "open sesame")
    };

    let res = gix_reqwest::Client::builder()
        .proxy(gix_reqwest::Proxy::http(format!("http://{}", server.addr())).unwrap())
        .proxy(later_proxy)
        .build()
        .unwrap()
        .get(url)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
}

#[tokio::test]
async fn http_proxy_basic_auth_parsed() {
    let url = "http://hyper.rs.local/prox";
    let server = server::http(move |req| {
        assert_eq!(req.method(), "GET");
        assert_eq!(req.uri(), url);
        assert_eq!(req.headers()["host"], "hyper.rs.local");
        assert_eq!(
            req.headers()["proxy-authorization"],
            "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ=="
        );

        async { http::Response::default() }
    });

    let proxy = format!("http://Aladdin:open sesame@{}", server.addr());

    let res = gix_reqwest::Client::builder()
        .proxy(gix_reqwest::Proxy::http(&proxy).unwrap())
        .build()
        .unwrap()
        .get(url)
        .send()
        .await
        .unwrap();

    assert_eq!(res.url().as_str(), url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
}

#[tokio::test]
async fn system_http_proxy_basic_auth_parsed() {
    let url = "http://hyper.rs.local/prox";
    let server = server::http(move |req| {
        assert_eq!(req.method(), "GET");
        assert_eq!(req.uri(), url);
        assert_eq!(req.headers()["host"], "hyper.rs.local");
        assert_eq!(
            req.headers()["proxy-authorization"],
            "Basic QWxhZGRpbjpvcGVuc2VzYW1l"
        );

        async { http::Response::default() }
    });

    // avoid races with other tests that change "http_proxy"
    let _env_lock = HTTP_PROXY_ENV_MUTEX.lock().await;

    // save system setting first.
    let system_proxy = env::var("http_proxy");

    // set-up http proxy.
    env::set_var(
        "http_proxy",
        format!("http://Aladdin:opensesame@{}", server.addr()),
    );

    let res = gix_reqwest::Client::builder()
        .build()
        .unwrap()
        .get(url)
        .send()
        .await
        .unwrap();

    assert_eq!(res.url().as_str(), url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);

    // reset user setting.
    match system_proxy {
        Err(_) => env::remove_var("http_proxy"),
        Ok(proxy) => env::set_var("http_proxy", proxy),
    }
}

#[tokio::test]
async fn test_no_proxy() {
    let server = server::http(move |req| {
        assert_eq!(req.method(), "GET");
        assert_eq!(req.uri(), "/4");

        async { http::Response::default() }
    });
    let proxy = format!("http://{}", server.addr());
    let url = format!("http://{}/4", server.addr());

    // set up proxy and use no_proxy to clear up client builder proxies.
    let res = gix_reqwest::Client::builder()
        .proxy(gix_reqwest::Proxy::http(&proxy).unwrap())
        .no_proxy()
        .build()
        .unwrap()
        .get(&url)
        .send()
        .await
        .unwrap();

    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
}

#[tokio::test]
async fn test_custom_headers() {
    let url = "http://hyper.rs.local/prox";
    let server = server::http(move |req| {
        assert_eq!(req.method(), "GET");
        assert_eq!(req.uri(), url);
        assert_eq!(req.headers()["host"], "hyper.rs.local");
        assert_eq!(
            req.headers()["proxy-authorization"],
            "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ=="
        );
        async { http::Response::default() }
    });

    let proxy = format!("http://{}", server.addr());
    let mut headers = gix_reqwest::header::HeaderMap::new();
    headers.insert(
        // gix_reqwest::header::HeaderName::from_static("Proxy-Authorization"),
        gix_reqwest::header::PROXY_AUTHORIZATION,
        "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ==".parse().unwrap(),
    );

    let res = gix_reqwest::Client::builder()
        .proxy(gix_reqwest::Proxy::http(&proxy).unwrap().headers(headers))
        .build()
        .unwrap()
        .get(url)
        .send()
        .await
        .unwrap();

    assert_eq!(res.url().as_str(), url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
}

#[tokio::test]
async fn test_using_system_proxy() {
    let url = "http://not.a.real.sub.hyper.rs.local/prox";
    let server = server::http(move |req| {
        assert_eq!(req.method(), "GET");
        assert_eq!(req.uri(), url);
        assert_eq!(req.headers()["host"], "not.a.real.sub.hyper.rs.local");

        async { http::Response::default() }
    });

    // avoid races with other tests that change "http_proxy"
    let _env_lock = HTTP_PROXY_ENV_MUTEX.lock().await;

    // save system setting first.
    let system_proxy = env::var("http_proxy");
    // set-up http proxy.
    env::set_var("http_proxy", format!("http://{}", server.addr()));

    // system proxy is used by default
    let res = gix_reqwest::get(url).await.unwrap();

    assert_eq!(res.url().as_str(), url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);

    // reset user setting.
    match system_proxy {
        Err(_) => env::remove_var("http_proxy"),
        Ok(proxy) => env::set_var("http_proxy", proxy),
    }
}

#[tokio::test]
async fn http_over_http() {
    let url = "http://hyper.rs.local/prox";

    let server = server::http(move |req| {
        assert_eq!(req.method(), "GET");
        assert_eq!(req.uri(), url);
        assert_eq!(req.headers()["host"], "hyper.rs.local");

        async { http::Response::default() }
    });

    let proxy = format!("http://{}", server.addr());

    let res = gix_reqwest::Client::builder()
        .proxy(gix_reqwest::Proxy::http(&proxy).unwrap())
        .build()
        .unwrap()
        .get(url)
        .send()
        .await
        .unwrap();

    assert_eq!(res.url().as_str(), url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
}

#[cfg(feature = "__tls")]
#[tokio::test]
async fn tunnel_detects_auth_required() {
    let url = "https://hyper.rs.local/prox";

    let server = server::http(move |req| {
        assert_eq!(req.method(), "CONNECT");
        assert_eq!(req.uri(), "hyper.rs.local:443");
        assert!(!req
            .headers()
            .contains_key(http::header::PROXY_AUTHORIZATION));

        async {
            let mut res = http::Response::default();
            *res.status_mut() = http::StatusCode::PROXY_AUTHENTICATION_REQUIRED;
            res
        }
    });

    let proxy = format!("http://{}", server.addr());

    let err = gix_reqwest::Client::builder()
        .proxy(gix_reqwest::Proxy::https(&proxy).unwrap())
        .build()
        .unwrap()
        .get(url)
        .send()
        .await
        .unwrap_err();

    let err = support::error::inspect(err).pop().unwrap();
    assert!(
        err.contains("auth"),
        "proxy auth err expected, got: {:?}",
        err
    );
}

#[cfg(feature = "__tls")]
#[tokio::test]
async fn tunnel_includes_proxy_auth() {
    let url = "https://hyper.rs.local/prox";

    let server = server::http(move |req| {
        assert_eq!(req.method(), "CONNECT");
        assert_eq!(req.uri(), "hyper.rs.local:443");
        assert_eq!(
            req.headers()["proxy-authorization"],
            "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ=="
        );

        async {
            // return 400 to not actually deal with TLS tunneling
            let mut res = http::Response::default();
            *res.status_mut() = http::StatusCode::BAD_REQUEST;
            res
        }
    });

    let proxy = format!("http://Aladdin:open%20sesame@{}", server.addr());

    let err = gix_reqwest::Client::builder()
        .proxy(gix_reqwest::Proxy::https(&proxy).unwrap())
        .build()
        .unwrap()
        .get(url)
        .send()
        .await
        .unwrap_err();

    let err = support::error::inspect(err).pop().unwrap();
    assert!(
        err.contains("unsuccessful"),
        "tunnel unsuccessful expected, got: {:?}",
        err
    );
}

#[cfg(feature = "__rustls")]
#[tokio::test]
async fn tunnel_to_ipv6_literal() {
    let url = "https://[2001:db8::1]/prox";

    let server = server::http(move |req| {
        assert_eq!(req.method(), "CONNECT");
        assert_eq!(req.uri(), "[2001:db8::1]:443");

        async {
            // return 200 so the client goes on to the TLS handshake
            http::Response::default()
        }
    });

    let proxy = format!("http://{}", server.addr());

    let err = gix_reqwest::Client::builder()
        .proxy(gix_reqwest::Proxy::https(&proxy).unwrap())
        .build()
        .unwrap()
        .get(url)
        .send()
        .await
        .unwrap_err();

    let err = support::error::inspect(err).pop().unwrap();
    assert!(
        !err.contains("Invalid Server Name"),
        "server name rejected before the handshake, got: {:?}",
        err
    );
}

#[cfg(feature = "__tls")]
#[tokio::test]
async fn tunnel_includes_user_agent() {
    let url = "https://hyper.rs.local/prox";

    let server = server::http(move |req| {
        assert_eq!(req.method(), "CONNECT");
        assert_eq!(req.uri(), "hyper.rs.local:443");
        assert_eq!(req.headers()["user-agent"], "reqwest-test");

        async {
            // return 400 to not actually deal with TLS tunneling
            let mut res = http::Response::default();
            *res.status_mut() = http::StatusCode::BAD_REQUEST;
            res
        }
    });

    let proxy = format!("http://{}", server.addr());

    let err = gix_reqwest::Client::builder()
        .proxy(gix_reqwest::Proxy::https(&proxy).unwrap())
        .user_agent("reqwest-test")
        .build()
        .unwrap()
        .get(url)
        .send()
        .await
        .unwrap_err();

    let err = support::error::inspect(err).pop().unwrap();
    assert!(
        err.contains("unsuccessful"),
        "tunnel unsuccessful expected, got: {:?}",
        err
    );
}

#[tokio::test]
async fn tunnel_includes_proxy_auth_with_multiple_proxies() {
    let url = "http://hyper.rs.local/prox";
    let server1 = server::http(move |req| {
        assert_eq!(req.method(), "GET");
        assert_eq!(req.uri(), url);
        assert_eq!(req.headers()["host"], "hyper.rs.local");
        assert_eq!(
            req.headers()["proxy-authorization"],
            "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ=="
        );
        assert_eq!(req.headers()["proxy-header"], "proxy2");
        async { http::Response::default() }
    });

    let proxy_url = format!("http://Aladdin:open%20sesame@{}", server1.addr());

    let mut headers1 = gix_reqwest::header::HeaderMap::new();
    headers1.insert("proxy-header", "proxy1".parse().unwrap());

    let mut headers2 = gix_reqwest::header::HeaderMap::new();
    headers2.insert("proxy-header", "proxy2".parse().unwrap());

    let client = gix_reqwest::Client::builder()
        // When processing proxy headers, the first one is iterated,
        // and if the current URL does not match, the proxy is skipped
        .proxy(
            gix_reqwest::Proxy::https(&proxy_url)
                .unwrap()
                .headers(headers1.clone()),
        )
        // When processing proxy headers, the second one is iterated,
        // and for the current URL matching, the proxy will be used
        .proxy(
            gix_reqwest::Proxy::http(&proxy_url)
                .unwrap()
                .headers(headers2.clone()),
        )
        .build()
        .unwrap();

    let res = client.get(url).send().await.unwrap();

    assert_eq!(res.url().as_str(), url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);

    let client = gix_reqwest::Client::builder()
        // When processing proxy headers, the first one is iterated,
        // and for the current URL matching, the proxy will be used
        .proxy(
            gix_reqwest::Proxy::http(&proxy_url)
                .unwrap()
                .headers(headers2),
        )
        // When processing proxy headers, the second one is iterated,
        // and if the current URL does not match, the proxy is skipped
        .proxy(
            gix_reqwest::Proxy::https(&proxy_url)
                .unwrap()
                .headers(headers1),
        )
        .build()
        .unwrap();

    let res = client.get(url).send().await.unwrap();

    assert_eq!(res.url().as_str(), url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
}

#[tokio::test]
async fn http_redirect_uses_selected_proxy_credentials() -> Result<(), gix_reqwest::Error> {
    for custom_headers in [false, true] {
        let destination = server::http(move |req| async move {
            for name in ["authorization", "cookie", "proxy-authorization"] {
                assert!(
                    !req.headers().contains_key(name),
                    "direct request leaked {name}"
                );
            }
            assert_eq!(req.uri().path(), "/end");
            http::Response::default()
        });
        let destination_url = format!("http://{}/end", destination.addr());
        let direct_url = destination_url.clone();
        let expected = if custom_headers {
            "custom-proxy-auth"
        } else {
            "Basic c2Vjb25kOnBhc3N3b3Jk"
        };
        let second_proxy = server::http(move |req| {
            let direct_url = direct_url.clone();
            async move {
                assert_eq!(req.uri().host(), Some("second.invalid"));
                assert_eq!(
                    req.headers()["proxy-authorization"],
                    expected,
                    "selected proxy authenticates each hop"
                );
                assert!(
                    !req.headers().contains_key("authorization"),
                    "origin credentials stay removed"
                );
                assert!(
                    !req.headers().contains_key("cookie"),
                    "origin cookies stay removed"
                );
                let next = match req.uri().path() {
                    "/next" => "/same".to_owned(),
                    "/same" => direct_url,
                    path => panic!("unexpected proxy redirect path {path}"),
                };
                http::Response::builder()
                    .status(302)
                    .header("location", next)
                    .body(gix_reqwest::Body::default())
                    .expect("valid redirect response")
            }
        });
        let first_proxy = server::http(move |req| async move {
            assert_eq!(req.uri().host(), Some("first.invalid"));
            assert_eq!(req.headers()["authorization"], "origin-secret");
            assert_eq!(req.headers()["cookie"], "origin=cookie");
            assert_eq!(
                req.headers()["proxy-authorization"],
                "Basic Zmlyc3Q6cGFzc3dvcmQ=",
                "only the first matching proxy supplies credentials"
            );
            http::Response::builder()
                .status(302)
                .header("location", "http://second.invalid/next")
                .body(gix_reqwest::Body::default())
                .expect("valid redirect response")
        });
        let first_proxy_url = format!("http://first:password@{}", first_proxy.addr());
        let mut selected_proxy =
            gix_reqwest::Proxy::http(format!("http://{}", second_proxy.addr()))?
                .basic_auth("second", "password")
                .no_proxy(gix_reqwest::NoProxy::from_string("127.0.0.1"));
        if custom_headers {
            let mut headers = gix_reqwest::header::HeaderMap::new();
            headers.insert(
                gix_reqwest::header::PROXY_AUTHORIZATION,
                gix_reqwest::header::HeaderValue::from_static(expected),
            );
            selected_proxy = selected_proxy.headers(headers);
        }
        let response = gix_reqwest::Client::builder()
            .no_proxy()
            .proxy(gix_reqwest::Proxy::custom(move |url| {
                (url.host_str() == Some("first.invalid")).then(|| first_proxy_url.clone())
            }))
            .proxy(selected_proxy)
            .build()?
            .get("http://first.invalid/start")
            .header("authorization", "origin-secret")
            .header("cookie", "origin=cookie")
            .send()
            .await?;
        assert_eq!(response.status(), gix_reqwest::StatusCode::OK);
        assert_eq!(response.url().as_str(), destination_url);
    }
    Ok(())
}

#[cfg(all(feature = "socks", feature = "__rustls-aws-lc-rs"))]
#[tokio::test]
async fn socks_tls_info_is_reported_when_enabled() {
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let cert: rustls_pki_types::CertificateDer<'static> =
        include_bytes!("support/server.cert").to_vec().into();
    let key = include_bytes!("support/server.key")
        .to_vec()
        .try_into()
        .unwrap();
    let provider = rustls::crypto::aws_lc_rs::default_provider();
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(provider))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = listener.local_addr().unwrap();
    let proxy = tokio::spawn(async move {
        for _ in 0..2 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut greeting = [0; 3];
            socket.read_exact(&mut greeting).await.unwrap();
            assert_eq!(greeting, [5, 1, 0]);
            socket.write_all(&[5, 0]).await.unwrap();
            let mut request = [0; 5];
            socket.read_exact(&mut request).await.unwrap();
            assert_eq!(&request[..4], &[5, 1, 0, 3]);
            let mut destination = vec![0; request[4] as usize + 2];
            socket.read_exact(&mut destination).await.unwrap();
            assert_eq!(&destination[..destination.len() - 2], b"localhost");
            socket
                .write_all(&[5, 0, 0, 1, 127, 0, 0, 1, 0, 0])
                .await
                .unwrap();
            let mut tls = tokio_rustls::TlsAcceptor::from(Arc::new(config.clone()))
                .accept(socket)
                .await
                .unwrap();
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                head.push(tls.read_u8().await.unwrap());
            }
            tls.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .await
                .unwrap();
            tls.shutdown().await.unwrap();
        }
    });
    for enabled in [false, true] {
        let response = gix_reqwest::Client::builder()
            .tls_backend_rustls()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(5))
            .tls_info(enabled)
            .proxy(gix_reqwest::Proxy::all(format!("socks5h://{proxy_addr}")).unwrap())
            .build()
            .unwrap()
            .get("https://localhost/")
            .send()
            .await
            .unwrap();
        let info = response.extensions().get::<gix_reqwest::tls::TlsInfo>();
        assert_eq!(info.is_some(), enabled);
        if let Some(info) = info {
            assert_eq!(
                info.peer_certificate(),
                Some(&include_bytes!("support/server.cert")[..])
            );
            assert!(info.version().is_some());
        }
    }
    proxy.await.unwrap();
}

#[tokio::test]
async fn return_to_original_host_drops_intermediate_proxy_credentials() {
    let original = server::http(|req| async move {
        if req.uri().path() == "/start" {
            http::Response::builder()
                .status(302)
                .header("location", "http://b.invalid/next")
                .body(gix_reqwest::Body::default())
                .unwrap()
        } else {
            assert_eq!(req.uri().path(), "/end");
            assert!(!req.headers().contains_key("proxy-authorization"));
            http::Response::default()
        }
    });
    let return_url = format!("http://{}/end", original.addr());
    let proxy = server::http(move |req| {
        let return_url = return_url.clone();
        async move {
            assert_eq!(req.headers()["proxy-authorization"], "Basic dXNlcjpwYXNz");
            http::Response::builder()
                .status(302)
                .header("location", return_url)
                .body(gix_reqwest::Body::default())
                .unwrap()
        }
    });
    let proxy_url = format!("http://user:pass@{}", proxy.addr());
    let client = gix_reqwest::Client::builder()
        .no_proxy()
        .proxy(gix_reqwest::Proxy::custom(move |url| {
            (url.host_str() == Some("b.invalid")).then(|| proxy_url.clone())
        }))
        .build()
        .unwrap();
    client
        .get(format!("http://{}/start", original.addr()))
        .send()
        .await
        .unwrap();
}

#[cfg(feature = "__rustls-aws-lc-rs")]
#[tokio::test]
async fn connect_tunnel_tls_info_is_reported_when_enabled() {
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let certificate =
        rustls::pki_types::CertificateDer::from(include_bytes!("support/server.cert").to_vec());
    let key =
        rustls::pki_types::PrivateKeyDer::try_from(include_bytes!("support/server.key").to_vec())
            .unwrap();
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(vec![certificate.clone()], key)
    .unwrap();
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));

    let backends: &[fn(gix_reqwest::ClientBuilder) -> gix_reqwest::ClientBuilder] = &[
        gix_reqwest::ClientBuilder::tls_backend_rustls,
        #[cfg(feature = "__native-tls")]
        gix_reqwest::ClientBuilder::tls_backend_native,
    ];
    for backend in backends {
        for enabled in [false, true] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let acceptor = acceptor.clone();
            let task = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    request.push(socket.read_u8().await.unwrap());
                }
                assert!(request.starts_with(b"CONNECT localhost:443 HTTP/1.1\r\n"));
                socket
                    .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                    .await
                    .unwrap();
                let mut stream = acceptor.accept(socket).await.unwrap();
                request.clear();
                while !request.ends_with(b"\r\n\r\n") {
                    request.push(stream.read_u8().await.unwrap());
                }
                stream
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                    .await
                    .unwrap();
                stream.shutdown().await.unwrap();
            });
            let response = backend(gix_reqwest::Client::builder())
                .danger_accept_invalid_certs(true)
                .tls_info(enabled)
                .timeout(std::time::Duration::from_secs(5))
                .proxy(gix_reqwest::Proxy::all(format!("http://{addr}")).unwrap())
                .build()
                .unwrap()
                .get("https://localhost/")
                .send()
                .await
                .unwrap();
            let info = response.extensions().get::<gix_reqwest::tls::TlsInfo>();
            assert_eq!(info.is_some(), enabled);
            if let Some(info) = info {
                assert_eq!(info.peer_certificate(), Some(certificate.as_ref()));
            }
            task.await.unwrap();
        }
    }
}

#[tokio::test]
async fn redirect_scopes_custom_proxy_headers_and_restores_origin_values() {
    let direct = server::http(|req| async move {
        assert!(!req.headers().contains_key("x-proxy-token"));
        assert!(!req.headers().contains_key("x-first-proxy"));
        assert!(!req.headers().contains_key("authorization"));
        let values: Vec<_> = req.headers().get_all("x-shared").iter().collect();
        assert_eq!(values, ["origin-first", "origin-second"]);
        http::Response::default()
    });
    let direct_url = format!("http://{}/end", direct.addr());
    let second = server::http(move |req| {
        let direct_url = direct_url.clone();
        async move {
            assert_eq!(req.headers()["x-proxy-token"], "second");
            assert!(!req.headers().contains_key("x-first-proxy"));
            assert!(!req.headers().contains_key("authorization"));
            let values: Vec<_> = req.headers().get_all("x-shared").iter().collect();
            assert_eq!(values, ["origin-first", "origin-second"]);
            let destination = if req.uri().path() == "/next" {
                "http://second.invalid/same".to_owned()
            } else {
                assert_eq!(req.uri().path(), "/same");
                direct_url
            };
            http::Response::builder()
                .status(302)
                .header("location", destination)
                .body(gix_reqwest::Body::default())
                .unwrap()
        }
    });
    let first = server::http(|req| async move {
        assert_eq!(req.headers()["x-proxy-token"], "first");
        assert_eq!(req.headers()["x-first-proxy"], "secret");
        assert_eq!(req.headers()["x-shared"], "proxy-override");
        assert_eq!(req.headers()["authorization"], "proxy-authorization");
        http::Response::builder()
            .status(302)
            .header("location", "http://second.invalid/next")
            .body(gix_reqwest::Body::default())
            .unwrap()
    });
    let first_url = format!("http://{}", first.addr());
    let mut first_headers = gix_reqwest::header::HeaderMap::new();
    first_headers.insert("x-proxy-token", "first".parse().unwrap());
    first_headers.insert("x-first-proxy", "secret".parse().unwrap());
    first_headers.insert("x-shared", "proxy-override".parse().unwrap());
    first_headers.insert("authorization", "proxy-authorization".parse().unwrap());
    let mut second_headers = gix_reqwest::header::HeaderMap::new();
    second_headers.insert("x-proxy-token", "second".parse().unwrap());
    let client = gix_reqwest::Client::builder()
        .no_proxy()
        .proxy(
            gix_reqwest::Proxy::custom(move |url| {
                (url.host_str() == Some("first.invalid")).then(|| first_url.clone())
            })
            .headers(first_headers),
        )
        .proxy(
            gix_reqwest::Proxy::http(format!("http://{}", second.addr()))
                .unwrap()
                .headers(second_headers)
                .no_proxy(gix_reqwest::NoProxy::from_string("127.0.0.1")),
        )
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    let mut headers = gix_reqwest::header::HeaderMap::new();
    headers.append("x-shared", "origin-first".parse().unwrap());
    let mut sensitive = gix_reqwest::header::HeaderValue::from_static("origin-second");
    sensitive.set_sensitive(true);
    headers.append("x-shared", sensitive);
    headers.insert("authorization", "origin-authorization".parse().unwrap());
    client
        .get("http://first.invalid/start")
        .headers(headers)
        .send()
        .await
        .unwrap();
}

#[tokio::test]
async fn same_authority_redirect_removes_headers_from_previous_proxy() {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    let use_proxy = Arc::new(AtomicBool::new(true));
    let callback = use_proxy.clone();
    let direct = server::http(|req| async move {
        assert_eq!(req.uri().path(), "/end");
        assert!(!req.headers().contains_key("x-proxy-token"));
        assert!(!req.headers().contains_key("proxy-authorization"));
        assert_eq!(req.headers()["authorization"], "origin-secret");
        http::Response::default()
    });
    let proxy = server::http(move |req| {
        let use_proxy = use_proxy.clone();
        async move {
            assert_eq!(req.headers()["x-proxy-token"], "secret");
            assert_eq!(req.headers()["proxy-authorization"], "Basic dXNlcjpwYXNz");
            assert_eq!(req.headers()["authorization"], "origin-secret");
            use_proxy.store(false, Ordering::SeqCst);
            http::Response::builder()
                .status(302)
                .header("location", "/end")
                .header("connection", "close")
                .body(gix_reqwest::Body::default())
                .unwrap()
        }
    });
    let proxy_url = format!("http://user:pass@{}", proxy.addr());
    let mut headers = gix_reqwest::header::HeaderMap::new();
    headers.insert("x-proxy-token", "secret".parse().unwrap());
    let client = gix_reqwest::Client::builder()
        .no_proxy()
        .resolve("origin.invalid", direct.addr())
        .proxy(
            gix_reqwest::Proxy::custom(move |_url| {
                callback.load(Ordering::SeqCst).then(|| proxy_url.clone())
            })
            .headers(headers),
        )
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    client
        .get("http://origin.invalid/start")
        .header("authorization", "origin-secret")
        .send()
        .await
        .unwrap();
}

#[cfg(feature = "__rustls-aws-lc-rs")]
#[tokio::test]
async fn http_to_https_redirect_keeps_custom_proxy_headers_in_connect_only() {
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let certificate =
        rustls::pki_types::CertificateDer::from(include_bytes!("support/server.cert").to_vec());
    let key =
        rustls::pki_types::PrivateKeyDer::try_from(include_bytes!("support/server.key").to_vec())
            .unwrap();
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(vec![certificate], key)
    .unwrap();
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            request.push(socket.read_u8().await.unwrap());
        }
        let request = std::str::from_utf8(&request).unwrap();
        assert!(request.starts_with("GET http://localhost/start HTTP/1.1\r\n"));
        assert!(request.contains("x-proxy-token: secret\r\n"));
        socket.write_all(b"HTTP/1.1 302 Found\r\nLocation: https://localhost/end\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
        socket.shutdown().await.unwrap();

        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            request.push(socket.read_u8().await.unwrap());
        }
        let request = std::str::from_utf8(&request).unwrap();
        assert!(request.starts_with("CONNECT localhost:443 HTTP/1.1\r\n"));
        assert!(request.contains("x-proxy-token: secret\r\n"));
        socket
            .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            .await
            .unwrap();
        let mut stream = acceptor.accept(socket).await.unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            request.push(stream.read_u8().await.unwrap());
        }
        let request = std::str::from_utf8(&request).unwrap();
        assert!(request.starts_with("GET /end HTTP/1.1\r\n"));
        assert!(!request.contains("x-proxy-token:"));
        assert!(request.contains("x-shared: origin-value\r\n"));
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
            .await
            .unwrap();
        stream.shutdown().await.unwrap();
    });
    let mut headers = gix_reqwest::header::HeaderMap::new();
    headers.insert("x-proxy-token", "secret".parse().unwrap());
    headers.insert("x-shared", "proxy-override".parse().unwrap());
    gix_reqwest::Client::builder()
        .tls_backend_rustls()
        .danger_accept_invalid_certs(true)
        .proxy(
            gix_reqwest::Proxy::all(format!("http://{addr}"))
                .unwrap()
                .headers(headers),
        )
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap()
        .get("http://localhost/start")
        .header("x-shared", "origin-value")
        .send()
        .await
        .unwrap();
    task.await.unwrap();
}

#[tokio::test]
async fn proxy_overlay_does_not_restore_payload_headers_after_body_dropping_redirect() {
    for (method, status) in [
        (gix_reqwest::Method::POST, 301),
        (gix_reqwest::Method::POST, 302),
        (gix_reqwest::Method::POST, 303),
        (gix_reqwest::Method::HEAD, 303),
    ] {
        let original_method = method.clone();
        let redirected_method = if method == gix_reqwest::Method::HEAD {
            gix_reqwest::Method::HEAD
        } else {
            gix_reqwest::Method::GET
        };
        let proxy = server::http(move |req| {
            let original_method = original_method.clone();
            let redirected_method = redirected_method.clone();
            async move {
                if req.uri().path() == "/start" {
                    assert_eq!(req.method(), original_method);
                    assert_eq!(req.headers()["content-type"], "proxy-type");
                    http::Response::builder()
                        .status(status)
                        .header("location", "/next")
                        .body(gix_reqwest::Body::default())
                        .unwrap()
                } else {
                    assert_eq!(req.method(), redirected_method);
                    for name in [
                        "content-type",
                        "content-length",
                        "content-encoding",
                        "transfer-encoding",
                    ] {
                        assert!(
                            !req.headers().contains_key(name),
                            "payload header {name} returned after {original_method} {status} redirect"
                        );
                    }
                    if req.uri().path() == "/next" {
                        http::Response::builder()
                            .status(307)
                            .header("location", "/end")
                            .body(gix_reqwest::Body::default())
                            .unwrap()
                    } else {
                        assert_eq!(req.uri().path(), "/end");
                        http::Response::default()
                    }
                }
            }
        });
        let mut headers = gix_reqwest::header::HeaderMap::new();
        headers.insert("content-type", "proxy-type".parse().unwrap());
        gix_reqwest::Client::builder()
            .no_proxy()
            .proxy(
                gix_reqwest::Proxy::http(format!("http://{}", proxy.addr()))
                    .unwrap()
                    .headers(headers),
            )
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap()
            .request(method, "http://origin.invalid/start")
            .header("content-type", "origin-type")
            .body("upload")
            .send()
            .await
            .unwrap();
    }
}
