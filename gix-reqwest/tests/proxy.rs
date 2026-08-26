#![cfg(not(target_arch = "wasm32"))]
#![cfg(not(feature = "rustls-no-provider"))]
mod support;
use support::server;

use std::env;

use std::sync::LazyLock;
use tokio::sync::Mutex;

// serialize tests that read from / write to environment variables
static HTTP_PROXY_ENV_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

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
