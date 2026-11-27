#![cfg(not(target_arch = "wasm32"))]
mod support;
use http_body_util::BodyExt;
use support::server;

#[tokio::test]
async fn text_part() {
    let _ = env_logger::try_init();

    let form = gix_reqwest::multipart::Form::new().text("foo", "bar");

    let expected_body = format!(
        "\
         --{0}\r\n\
         Content-Disposition: form-data; name=\"foo\"\r\n\r\n\
         bar\r\n\
         --{0}--\r\n\
         ",
        form.boundary()
    );

    let ct = format!("multipart/form-data; boundary={}", form.boundary());

    let server = server::http(move |mut req| {
        let ct = ct.clone();
        let expected_body = expected_body.clone();
        async move {
            assert_eq!(req.method(), "POST");
            assert_eq!(req.headers()["content-type"], ct);
            assert_eq!(
                req.headers()["content-length"],
                expected_body.len().to_string()
            );

            let mut full: Vec<u8> = Vec::new();
            while let Some(item) = req.body_mut().frame().await {
                full.extend(&*item.unwrap().into_data().unwrap());
            }

            assert_eq!(full, expected_body.as_bytes());

            http::Response::default()
        }
    });

    let url = format!("http://{}/multipart/1", server.addr());

    let res = gix_reqwest::Client::new()
        .post(&url)
        .multipart(form)
        .send()
        .await
        .unwrap();

    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
}

#[cfg(feature = "stream")]
#[tokio::test]
async fn stream_part() {
    use futures_util::stream;
    use std::future;

    let _ = env_logger::try_init();

    let stream =
        gix_reqwest::Body::wrap_stream(stream::once(future::ready(Ok::<_, gix_reqwest::Error>(
            "part1 part2".to_owned(),
        ))));
    let part = gix_reqwest::multipart::Part::stream(stream);

    let form = gix_reqwest::multipart::Form::new()
        .text("foo", "bar")
        .part("part_stream", part);

    let expected_body = format!(
        "\
         --{0}\r\n\
         Content-Disposition: form-data; name=\"foo\"\r\n\
         \r\n\
         bar\r\n\
         --{0}\r\n\
         Content-Disposition: form-data; name=\"part_stream\"\r\n\
         \r\n\
         part1 part2\r\n\
         --{0}--\r\n\
         ",
        form.boundary()
    );

    let ct = format!("multipart/form-data; boundary={}", form.boundary());

    let server = server::http(move |req| {
        let ct = ct.clone();
        let expected_body = expected_body.clone();
        async move {
            assert_eq!(req.method(), "POST");
            assert_eq!(req.headers()["content-type"], ct);
            assert_eq!(req.headers()["transfer-encoding"], "chunked");

            let full = req.collect().await.unwrap().to_bytes();

            assert_eq!(full, expected_body.as_bytes());

            http::Response::default()
        }
    });

    let url = format!("http://{}/multipart/1", server.addr());

    let client = gix_reqwest::Client::new();

    let res = client
        .post(&url)
        .multipart(form)
        .send()
        .await
        .expect("Failed to post multipart");
    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
}

#[cfg(feature = "blocking")]
#[test]
fn blocking_file_part() {
    let _ = env_logger::try_init();

    let form = gix_reqwest::blocking::multipart::Form::new()
        .file("foo", "LICENSE-MIT")
        .unwrap();

    let fcontents = std::fs::read_to_string("LICENSE-MIT").unwrap();

    let expected_body = format!(
        "\
         --{0}\r\n\
         Content-Disposition: form-data; name=\"foo\"; filename=\"LICENSE-MIT\"\r\n\
         Content-Type: application/octet-stream\r\n\r\n\
         {1}\r\n\
         --{0}--\r\n\
         ",
        form.boundary(),
        fcontents
    );

    let ct = format!("multipart/form-data; boundary={}", form.boundary());

    let server = server::http(move |req| {
        let ct = ct.clone();
        let expected_body = expected_body.clone();
        async move {
            assert_eq!(req.method(), "POST");
            assert_eq!(req.headers()["content-type"], ct);
            // files know their exact size
            assert_eq!(
                req.headers()["content-length"],
                expected_body.len().to_string()
            );

            let full = req.collect().await.unwrap().to_bytes();

            assert_eq!(full, expected_body.as_bytes());

            http::Response::default()
        }
    });

    let url = format!("http://{}/multipart/2", server.addr());

    let res = gix_reqwest::blocking::Client::new()
        .post(&url)
        .multipart(form)
        .send()
        .unwrap();

    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
}

#[cfg(feature = "stream")]
#[tokio::test]
async fn async_impl_file_part() {
    let _ = env_logger::try_init();

    let form = gix_reqwest::multipart::Form::new()
        .file("foo", "LICENSE-MIT")
        .await
        .unwrap();

    let fcontents = std::fs::read_to_string("LICENSE-MIT").unwrap();

    let expected_body = format!(
        "\
         --{0}\r\n\
         Content-Disposition: form-data; name=\"foo\"; filename=\"LICENSE-MIT\"\r\n\
         Content-Type: application/octet-stream\r\n\r\n\
         {1}\r\n\
         --{0}--\r\n\
         ",
        form.boundary(),
        fcontents
    );

    let ct = format!("multipart/form-data; boundary={}", form.boundary());

    let server = server::http(move |req| {
        let ct = ct.clone();
        let expected_body = expected_body.clone();
        async move {
            assert_eq!(req.method(), "POST");
            assert_eq!(req.headers()["content-type"], ct);
            // files know their exact size
            assert_eq!(
                req.headers()["content-length"],
                expected_body.len().to_string()
            );
            let full = req.collect().await.unwrap().to_bytes();

            assert_eq!(full, expected_body.as_bytes());

            http::Response::default()
        }
    });

    let url = format!("http://{}/multipart/3", server.addr());

    let res = gix_reqwest::Client::new()
        .post(&url)
        .multipart(form)
        .send()
        .await
        .unwrap();

    assert_eq!(res.url().as_str(), &url);
    assert_eq!(res.status(), gix_reqwest::StatusCode::OK);
}
