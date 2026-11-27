mod support;

use http::header::{CONTENT_LENGTH, CONTENT_TYPE, TRANSFER_ENCODING};
use http_body_util::BodyExt;
#[cfg(feature = "json")]
use std::collections::HashMap;
use support::server;

#[test]
fn test_response_text() {
    let server = server::http(move |_req| async { http::Response::new("Hello".into()) });

    let url = format!("http://{}/text", server.addr());
    let res = gix_reqwest::blocking::get(&url).unwrap();
    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
    assert_eq!(res.content_length(), Some(5));

    let body = res.text().unwrap();
    assert_eq!(b"Hello", body.as_bytes());
}

#[test]
fn donot_set_content_length_0_if_have_no_body() {
    let server = server::http(move |req| async move {
        let headers = req.headers();
        assert_eq!(headers.get(CONTENT_LENGTH), None);
        assert!(headers.get(CONTENT_TYPE).is_none());
        assert!(headers.get(TRANSFER_ENCODING).is_none());
        http::Response::default()
    });

    let url = format!("http://{}/content-length", server.addr());
    let res = gix_reqwest::blocking::Client::builder()
        .no_proxy()
        .build()
        .expect("client builder")
        .get(&url)
        .send()
        .expect("request");

    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
}

#[test]
#[cfg(feature = "charset")]
fn test_response_non_utf_8_text() {
    let server = server::http(move |_req| async {
        http::Response::builder()
            .header("content-type", "text/plain; charset=gbk")
            .body(b"\xc4\xe3\xba\xc3"[..].into())
            .unwrap()
    });

    let url = format!("http://{}/text", server.addr());
    let res = gix_reqwest::blocking::get(&url).unwrap();
    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
    assert_eq!(res.content_length(), Some(4));

    let body = res.text().unwrap();
    assert_eq!("你好", &body);
    assert_eq!(b"\xe4\xbd\xa0\xe5\xa5\xbd", body.as_bytes()); // Now it's utf-8
}

#[test]
#[cfg(feature = "json")]
fn test_response_json() {
    let server = server::http(move |_req| async { http::Response::new("\"Hello\"".into()) });

    let url = format!("http://{}/json", server.addr());
    let res = gix_reqwest::blocking::get(&url).unwrap();
    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
    assert_eq!(res.content_length(), Some(7));

    let body = res.json::<String>().unwrap();
    assert_eq!("Hello", body);
}

#[test]
fn test_response_copy_to() {
    let server = server::http(move |_req| async { http::Response::new("Hello".into()) });

    let url = format!("http://{}/1", server.addr());
    let mut res = gix_reqwest::blocking::get(&url).unwrap();
    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);

    let mut dst = Vec::new();
    res.copy_to(&mut dst).unwrap();
    assert_eq!(dst, b"Hello");
}

#[test]
fn test_get() {
    let server = server::http(move |_req| async { http::Response::default() });

    let url = format!("http://{}/1", server.addr());
    let res = gix_reqwest::blocking::get(&url).unwrap();

    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
    assert_eq!(res.remote_addr(), Some(server.addr()));

    assert_eq!(res.text().unwrap().len(), 0)
}

#[test]
fn test_post() {
    let server = server::http(move |req| async move {
        assert_eq!(req.method(), "POST");
        assert_eq!(req.headers()["content-length"], "5");

        let data = req.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&*data, b"Hello");

        http::Response::default()
    });

    let url = format!("http://{}/2", server.addr());
    let res = gix_reqwest::blocking::Client::new()
        .post(&url)
        .body("Hello")
        .send()
        .unwrap();

    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
}

#[test]
fn test_post_large_reader() {
    let body: Vec<u8> = (0..200_000u32).map(|i| i as u8).collect();
    let expected = body.clone();
    let server = server::http(move |req| {
        let expected = expected.clone();
        async move {
            assert_eq!(req.method(), "POST");

            let data = req.into_body().collect().await.unwrap().to_bytes();
            assert_eq!(&*data, &*expected);

            http::Response::default()
        }
    });

    let url = format!("http://{}/large", server.addr());
    let client = gix_reqwest::blocking::Client::new();

    let sized = gix_reqwest::blocking::Body::sized(std::io::Cursor::new(body.clone()), 200_000);
    let res = client.post(&url).body(sized).send().unwrap();
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);

    let chunked = gix_reqwest::blocking::Body::new(std::io::Cursor::new(body));
    let res = client.post(&url).body(chunked).send().unwrap();
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
}

#[test]
#[cfg(feature = "form")]
fn test_post_form() {
    let server = server::http(move |req| async move {
        assert_eq!(req.method(), "POST");
        assert_eq!(req.headers()["content-length"], "24");
        assert_eq!(
            req.headers()["content-type"],
            "application/x-www-form-urlencoded"
        );

        let data = req.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&*data, b"hello=world&sean=monstar");

        http::Response::default()
    });

    let form = &[("hello", "world"), ("sean", "monstar")];

    let url = format!("http://{}/form", server.addr());
    let res = gix_reqwest::blocking::Client::new()
        .post(&url)
        .form(form)
        .send()
        .expect("request send");

    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
}

/// Calling `Response::error_for_status`` on a response with status in 4xx
/// returns an error.
#[test]
fn test_error_for_status_4xx() {
    let server = server::http(move |_req| async {
        http::Response::builder()
            .status(400)
            .body(Default::default())
            .unwrap()
    });

    let url = format!("http://{}/1", server.addr());
    let res = gix_reqwest::blocking::get(&url).unwrap();

    let err = res.error_for_status().unwrap_err();
    assert!(err.is_status());
    assert_eq!(err.status(), Some(gix_reqwest::StatusCode::BAD_REQUEST));
}

/// Calling `Response::error_for_status`` on a response with status in 5xx
/// returns an error.
#[test]
fn test_error_for_status_5xx() {
    let server = server::http(move |_req| async {
        http::Response::builder()
            .status(500)
            .body(Default::default())
            .unwrap()
    });

    let url = format!("http://{}/1", server.addr());
    let res = gix_reqwest::blocking::get(&url).unwrap();

    let err = res.error_for_status().unwrap_err();
    assert!(err.is_status());
    assert_eq!(
        err.status(),
        Some(gix_reqwest::StatusCode::INTERNAL_SERVER_ERROR)
    );
}

#[test]
fn test_default_headers() {
    let server = server::http(move |req| async move {
        assert_eq!(req.headers()["reqwest-test"], "orly");
        http::Response::default()
    });

    let mut headers = http::HeaderMap::with_capacity(1);
    headers.insert("reqwest-test", "orly".parse().unwrap());
    let client = gix_reqwest::blocking::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap();

    let url = format!("http://{}/1", server.addr());
    let res = client.get(&url).send().unwrap();

    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
}

#[test]
fn test_override_default_headers() {
    let server = server::http(move |req| {
        async move {
            // not 'iamatoken'
            assert_eq!(req.headers()[&http::header::AUTHORIZATION], "secret");
            http::Response::default()
        }
    });

    let mut headers = http::HeaderMap::with_capacity(1);
    headers.insert(
        http::header::AUTHORIZATION,
        http::header::HeaderValue::from_static("iamatoken"),
    );
    let client = gix_reqwest::blocking::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap();

    let url = format!("http://{}/3", server.addr());
    let res = client
        .get(&url)
        .header(
            http::header::AUTHORIZATION,
            http::header::HeaderValue::from_static("secret"),
        )
        .send()
        .unwrap();

    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
}

#[test]
fn test_appended_headers_not_overwritten() {
    let server = server::http(move |req| async move {
        let mut accepts = req.headers().get_all("accept").into_iter();
        assert_eq!(accepts.next().unwrap(), "application/json");
        assert_eq!(accepts.next().unwrap(), "application/json+hal");
        assert_eq!(accepts.next(), None);

        http::Response::default()
    });

    let client = gix_reqwest::blocking::Client::new();

    let url = format!("http://{}/4", server.addr());
    let res = client
        .get(&url)
        .header(header::ACCEPT, "application/json")
        .header(header::ACCEPT, "application/json+hal")
        .send()
        .unwrap();

    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);

    // make sure this also works with default headers
    use gix_reqwest::header;
    let mut headers = header::HeaderMap::with_capacity(1);
    headers.insert(
        header::ACCEPT,
        header::HeaderValue::from_static("text/html"),
    );
    let client = gix_reqwest::blocking::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap();

    let url = format!("http://{}/4", server.addr());
    let res = client
        .get(&url)
        .header(header::ACCEPT, "application/json")
        .header(header::ACCEPT, "application/json+hal")
        .send()
        .unwrap();

    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
}

#[cfg_attr(not(debug_assertions), ignore)]
#[test]
#[should_panic]
fn test_blocking_inside_a_runtime() {
    let server = server::http(move |_req| async { http::Response::new("Hello".into()) });

    let url = format!("http://{}/text", server.addr());

    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("new rt");

    rt.block_on(async move {
        let _should_panic = gix_reqwest::blocking::get(&url);
    });
}

#[cfg(feature = "default-tls")]
#[test]
fn test_allowed_methods_blocking() {
    let resp = gix_reqwest::blocking::Client::builder()
        .https_only(true)
        .build()
        .expect("client builder")
        .get("https://google.com")
        .send();

    assert_eq!(resp.is_err(), false);

    let resp = gix_reqwest::blocking::Client::builder()
        .https_only(true)
        .build()
        .expect("client builder")
        .get("http://google.com")
        .send();

    assert_eq!(resp.is_err(), true);
}

/// Test that a [`gix_reqwest::blocking::Body`] can be created from [`bytes::Bytes`].
#[test]
fn test_body_from_bytes() {
    let body = "abc";
    // No external calls are needed. Only the request building is tested.
    let request = gix_reqwest::blocking::Client::builder()
        .build()
        .expect("Could not build the client")
        .put("https://google.com")
        .body(bytes::Bytes::from(body))
        .build()
        .expect("Invalid body");

    assert_eq!(request.body().unwrap().as_bytes(), Some(body.as_bytes()));
}

#[test]
#[cfg(feature = "json")]
fn blocking_add_json_default_content_type_if_not_set_manually() {
    use http::header::HeaderValue;

    let mut map = HashMap::new();
    map.insert("body", "json");
    let content_type = HeaderValue::from_static("application/vnd.api+json");
    let req = gix_reqwest::blocking::Client::new()
        .post("https://google.com/")
        .header(CONTENT_TYPE, &content_type)
        .json(&map)
        .build()
        .expect("request is not valid");

    assert_eq!(content_type, req.headers().get(CONTENT_TYPE).unwrap());
}

#[test]
#[cfg(feature = "json")]
fn blocking_update_json_content_type_if_set_manually() {
    let mut map = HashMap::new();
    map.insert("body", "json");
    let req = gix_reqwest::blocking::Client::new()
        .post("https://google.com/")
        .json(&map)
        .build()
        .expect("request is not valid");

    assert_eq!("application/json", req.headers().get(CONTENT_TYPE).unwrap());
}

#[test]
#[cfg(feature = "__tls")]
fn test_response_no_tls_info_for_http() {
    let server = server::http(move |_req| async { http::Response::new("Hello".into()) });

    let url = format!("http://{}/text", server.addr());

    let client = gix_reqwest::blocking::Client::builder()
        .tls_info(true)
        .build()
        .unwrap();

    let res = client.get(&url).send().unwrap();
    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
    assert_eq!(res.content_length(), Some(5));
    let tls_info = res.extensions().get::<gix_reqwest::tls::TlsInfo>();
    assert_eq!(tls_info.is_none(), true);

    let body = res.text().unwrap();
    assert_eq!(b"Hello", body.as_bytes());
}

// Yields its bytes in two reads, once the server has seen the client close.
// Even if the first chunk slips into the body channel while hyper is
// tearing down the connection, the second send can't complete - see
// the `send_to_early_response_server` helper below.
struct GatedReader {
    closed_rx: Option<std::sync::mpsc::Receiver<()>>,
    remaining: usize,
}

impl std::io::Read for GatedReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if let Some(closed_rx) = self.closed_rx.take() {
            closed_rx
                .recv_timeout(std::time::Duration::from_secs(5))
                .expect("client did not close the connection");
        }
        let n = self.remaining.min(16).min(buf.len());
        buf[..n].fill(b'x');
        self.remaining -= n;
        Ok(n)
    }
}

// Helper for testing cases where the server responds early (before the body
// is fully sent) such that we can verify that the blocking client handles
// early server responses correctly with a variety of request body types.
fn send_to_early_response_server(
    make_body: impl FnOnce(GatedReader) -> gix_reqwest::blocking::Body,
) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // This server responds without reading the request body, then waits for
    // the client to close the connection before letting the body produce
    // any bytes. This reproduces the case where the response is received
    // before the body is fully sent, e.g. an HTTP 412 for a failed
    // `If-None-Match: *` precondition.
    let (closed_tx, closed_rx) = std::sync::mpsc::channel();
    let (head_tx, head_rx) = std::sync::mpsc::channel();
    let server = server::low_level_with_response(move |raw_request, client_socket| {
        let closed_tx = closed_tx.clone();
        let _ = head_tx.send(String::from_utf8_lossy(raw_request).to_ascii_lowercase());
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

    let url = format!("http://{}/object", server.addr());
    let reader = GatedReader {
        closed_rx: Some(closed_rx),
        remaining: 32,
    };
    let res = gix_reqwest::blocking::Client::new()
        .put(&url)
        .body(make_body(reader))
        .send()
        .unwrap();

    assert_eq!(res.status(), gix_reqwest::StatusCode::PRECONDITION_FAILED);
    head_rx.recv().expect("server did not see a request")
}

// Test that the blocking client correctly handles an early server response
// with a sized request body.
#[test]
fn test_response_before_streamed_body_is_sent() {
    let head =
        send_to_early_response_server(|reader| gix_reqwest::blocking::Body::sized(reader, 32));
    assert!(head.contains("\r\ncontent-length: 32\r\n"));
}

// Test that the blocking client correctly handles an early server response
// with a chunked request body - covered as a separate scenario since the
// code paths for sized and chunked bodies differ, though they do converge
// at the same point, so this is defensive.
#[test]
fn test_response_before_chunked_body_is_sent() {
    let head = send_to_early_response_server(gix_reqwest::blocking::Body::new);
    assert!(head.contains("\r\ntransfer-encoding: chunked\r\n"));
}

#[test]
fn test_streamed_body_read_error() {
    struct FailingReader;

    impl std::io::Read for FailingReader {
        fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("read failed"))
        }
    }

    let server = server::http(move |_req| async { http::Response::default() });

    let url = format!("http://{}/object", server.addr());
    let body = gix_reqwest::blocking::Body::sized(FailingReader, 16);
    let err = gix_reqwest::blocking::Client::new()
        .put(&url)
        .body(body)
        .send()
        .unwrap_err();

    assert!(err.is_body());
    assert_eq!(
        std::error::Error::source(&err).unwrap().to_string(),
        "read failed"
    );
}
