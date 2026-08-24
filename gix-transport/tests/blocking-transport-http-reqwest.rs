//! Regression tests specific to the blocking `reqwest` HTTP backend.

pub mod bisync {
    pub use gix_macros::{discard as only_async, keep as only_sync, sync as bisync};
}

use std::{
    error::Error,
    io::Write,
    sync::{Arc, Mutex},
};

use gix_transport::{
    Protocol, Service,
    client::{
        TransportWithoutIO,
        blocking_io::{Transport, http},
    },
};

mod http_helpers;
use http_helpers::{observe_connection_within_deadline, read_request_lines, response_with_connection_close};

/// Regression for <https://github.com/GitoxideLabs/gitoxide/issues/2140>: when a request fails
/// without an HTTP status (for example a connection or TLS failure), the underlying error must be
/// kept as `source()` instead of being stringified, so callers can see the real cause.
#[test]
fn request_failure_without_status_preserves_error_source() {
    // Bind then immediately drop a listener so the port reliably refuses connections: a failure
    // with no HTTP status, exercising the path that previously stringified the error.
    let addr = {
        let server = std::net::TcpListener::bind("127.0.0.1:0").expect("can bind an ephemeral port");
        server.local_addr().expect("listener has a local address")
    };

    let url = format!("http://{addr}/repo");
    let mut client =
        http::connect::<http::reqwest::Remote>(url.as_str().try_into().expect("the url is valid"), Protocol::V1, false);

    let error = client
        .handshake(Service::UploadPack, &[])
        .err()
        .expect("a refused connection must produce an error");
    let io_error = error
        .source()
        .and_then(|source| source.downcast_ref::<std::io::Error>())
        .expect("the transport error wraps an io::Error");
    assert!(
        io_error.source().is_some(),
        "the underlying error must be preserved as source(), not stringified: {io_error:?}"
    );
}

#[test]
fn redirects_are_not_followed_with_configure_request_hook() -> Result<(), Box<dyn Error + Send + Sync>> {
    let redirected_listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let redirected_addr = redirected_listener.local_addr()?;
    let redirect_listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let redirect_addr = redirect_listener.local_addr()?;

    let redirected = observe_connection_within_deadline(redirected_listener);
    let redirect = std::thread::spawn(move || -> Vec<String> {
        let (stream, _) = redirect_listener.accept().expect("accept redirecting GET");
        let mut reader = std::io::BufReader::new(stream);
        let request = read_request_lines(&mut reader);
        reader
            .get_mut()
            .write_all(
                format!(
                    "HTTP/1.1 302 Found\r\n\
                     Location: http://127.0.0.1:{}/repo/info/refs?service=git-upload-pack\r\n\
                     Content-Length: 0\r\n\
                     Connection: close\r\n\r\n",
                    redirected_addr.port()
                )
                .as_bytes(),
            )
            .expect("write redirect response");
        reader.get_mut().shutdown(std::net::Shutdown::Both).ok();
        request
    });

    let mut client = http::connect::<http::reqwest::Remote>(
        format!("http://127.0.0.1:{}/repo", redirect_addr.port()).try_into()?,
        Protocol::V1,
        false,
    );
    let backend: Arc<Mutex<dyn std::any::Any + Send + Sync + 'static>> = Arc::new(Mutex::new(http::reqwest::Options {
        configure_request: Some(Box::new(|request| {
            request.headers_mut().insert(
                reqwest::header::HeaderName::from_static("private-token"),
                reqwest::header::HeaderValue::from_static("original-secret"),
            );
            Ok(())
        })),
    }));
    let options = http::Options {
        backend: Some(backend),
        ..Default::default()
    };
    client.configure(&options)?;

    let result = client.handshake(Service::UploadPack, &[]);
    let original_get = redirect.join().expect("thread");
    let redirected_was_contacted_within_deadline = redirected.join().expect("thread");

    assert!(result.is_err(), "redirects with a request hook should fail");
    match result {
        Ok(_) => unreachable!("handshake must fail"),
        Err(err) => {
            let err = format!("{err:?}");
            assert!(
                err.contains("refusing to follow redirect after request headers were configured"),
                "error should indicate that it failed due to redirection, got {err}"
            );
        }
    }
    assert!(
        original_get
            .iter()
            .any(|line| line.to_ascii_lowercase().starts_with("private-token:")),
        "the original request should still receive the configured request hook header, got {original_get:?}"
    );
    assert!(
        !redirected_was_contacted_within_deadline,
        "request hook headers must not be replayed to redirected hosts"
    );
    Ok(())
}

#[test]
fn relative_redirects_normalize_the_updated_base_url() -> Result<(), Box<dyn Error + Send + Sync>> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    let port = addr.port();

    let server = std::thread::spawn(move || -> (Vec<String>, Vec<String>) {
        let (stream, _) = listener.accept().expect("accept redirecting GET");
        let mut reader = std::io::BufReader::new(stream);
        let original_get = read_request_lines(&mut reader);
        reader
            .get_mut()
            .write_all(
                b"HTTP/1.1 302 Found\r\n\
                  Location: ../../../redirected/repo/info/refs?service=git-upload-pack\r\n\
                  Content-Length: 0\r\n\
                  Connection: close\r\n\r\n",
            )
            .expect("write non-root relative redirect response");
        reader.get_mut().shutdown(std::net::Shutdown::Both).ok();

        let (stream, _) = listener.accept().expect("accept redirected GET");
        let mut reader = std::io::BufReader::new(stream);
        let redirected_get = read_request_lines(&mut reader);
        reader
            .get_mut()
            .write_all(&response_with_connection_close(include_bytes!(
                "fixtures/v1/http-handshake.response"
            )))
            .expect("write redirected handshake response");
        reader.get_mut().shutdown(std::net::Shutdown::Both).ok();
        (original_get, redirected_get)
    });

    let mut client = http::connect::<http::reqwest::Remote>(
        format!("http://127.0.0.1:{port}/original/repo").try_into()?,
        Protocol::V1,
        false,
    );

    client.handshake(Service::UploadPack, &[]).map(drop)?;
    let (original_get, redirected_get) = server.join().expect("thread");

    assert!(
        !original_get.is_empty(),
        "the original host should receive the initial request"
    );
    assert!(
        redirected_get
            .iter()
            .any(|line| line == "GET /redirected/repo/info/refs?service=git-upload-pack HTTP/1.1"),
        "the redirected request path should be normalized by reqwest before it is sent, got {redirected_get:?}"
    );
    assert_eq!(
        client.to_url().as_ref(),
        format!("http://127.0.0.1:{port}/redirected/repo"),
        "the public transport URL should store the normalized redirected base"
    );
    Ok(())
}

#[test]
fn cross_authority_redirects_are_not_followed_without_matching_tail() -> Result<(), Box<dyn Error + Send + Sync>> {
    let redirected_listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let redirected_addr = redirected_listener.local_addr()?;
    let redirect_listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let redirect_addr = redirect_listener.local_addr()?;

    let redirected = observe_connection_within_deadline(redirected_listener);
    let redirect = std::thread::spawn(move || -> Vec<String> {
        let (stream, _) = redirect_listener.accept().expect("accept redirecting GET");
        let mut reader = std::io::BufReader::new(stream);
        let request = read_request_lines(&mut reader);
        reader
            .get_mut()
            .write_all(
                format!(
                    "HTTP/1.1 302 Found\r\n\
                     Location: http://127.0.0.1:{}/not-the-request-tail\r\n\
                     Content-Length: 0\r\n\
                     Connection: close\r\n\r\n",
                    redirected_addr.port()
                )
                .as_bytes(),
            )
            .expect("write redirect response");
        reader.get_mut().shutdown(std::net::Shutdown::Both).ok();
        request
    });

    let mut client = http::connect::<http::reqwest::Remote>(
        format!("http://127.0.0.1:{}/repo", redirect_addr.port()).try_into()?,
        Protocol::V1,
        false,
    );
    let options = http::Options {
        follow_redirects: http::options::FollowRedirects::All,
        ..Default::default()
    };
    client.configure(&options)?;

    let result = client.handshake(Service::UploadPack, &[]);
    let original_get = redirect.join().expect("thread");
    let redirected_was_contacted = redirected.join().expect("thread");

    match result {
        Ok(_) => unreachable!("tail-mismatched cross-authority redirects should fail"),
        Err(err) => {
            let err = format!("{err:?}");
            assert!(
                err.contains("not-the-request-tail"),
                "error should indicate that it failed due to redirection, got {err}"
            );
        }
    }
    assert!(
        !original_get.is_empty(),
        "the original request should still be sent before the redirect is rejected"
    );
    assert!(
        !redirected_was_contacted,
        "tail-mismatched cross-authority redirects must be rejected before sending the redirected request"
    );
    Ok(())
}
