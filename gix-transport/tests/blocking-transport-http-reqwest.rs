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
fn request_failure_without_status_preserves_error_source() -> gix_testtools::TestResult {
    // Bind then immediately drop a listener so the port reliably refuses connections: a failure
    // with no HTTP status, exercising the path that previously stringified the error.
    let addr = {
        let server = std::net::TcpListener::bind("127.0.0.1:0")?;
        server.local_addr()?
    };

    let url = format!("http://{addr}/repo");
    let mut client = http::connect::<http::reqwest::Remote>(url.as_str().try_into()?, Protocol::V1, false);

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
    Ok(())
}

#[test]
fn redirects_are_not_followed_with_configure_request_hook() -> gix_testtools::TestResult {
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
        configure_client: Some(Box::new(|builder| {
            // Client customization must not override Git's protection for private request headers.
            Ok(builder.redirect(reqwest::redirect::Policy::limited(20)))
        })),
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
    let err = result.err().expect("the redirect must be rejected");
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&gix_error::TestError::from(err), &[
        (&redirect_addr.to_string(), "127.0.0.1:<original-port>"),
        (&redirected_addr.to_string(), "127.0.0.1:<redirect-port>"),
    ]), "redirects are rejected after private request headers have been configured", @"
    An IO error occurred when talking to the server

    Caused by:
        0: I/O error (Other)
        1: error following redirect for url (http://127.0.0.1:<original-port>/repo/info/refs?service=git-upload-pack)
        2: refusing to follow redirect after request headers were configured
    ");
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
fn relative_redirects_normalize_the_updated_base_url() -> gix_testtools::TestResult {
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
fn cross_authority_redirects_are_not_followed_without_matching_tail() -> gix_testtools::TestResult {
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

    let err = result.err().expect("the redirect must be rejected");
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&gix_error::TestError::from(err), &[
        (&redirect_addr.to_string(), "127.0.0.1:<original-port>"),
        (&redirected_addr.to_string(), "127.0.0.1:<redirect-port>"),
    ]), "redirect rejection retains the mismatched request path", @r#"
    An IO error occurred when talking to the server

    Caused by:
        0: I/O error (Other)
        1: error following redirect for url (http://127.0.0.1:<original-port>/repo/info/refs?service=git-upload-pack)
        2: redirect url "http://127.0.0.1:<redirect-port>/not-the-request-tail" does not end with expected request suffix "/info/refs?service=git-upload-pack"
    "#);
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

#[test]
fn client_configuration_is_lazy_retained_on_restart_and_cached() -> gix_testtools::Result {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use gix_transport::client::blocking_io::http::Http;

    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let url = format!("http://{}/repo", listener.local_addr()?);
    let server = std::thread::spawn(move || -> std::io::Result<Vec<Vec<String>>> {
        let mut requests = Vec::new();
        for _ in 0..2 {
            let (stream, _) = listener.accept()?;
            stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
            let mut reader = std::io::BufReader::new(stream);
            requests.push(read_request_lines(&mut reader));
            reader
                .get_mut()
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")?;
        }
        Ok(requests)
    });
    let calls = Arc::new(AtomicUsize::new(0));
    let backend: Arc<Mutex<dyn std::any::Any + Send + Sync + 'static>> = Arc::new(Mutex::new(http::reqwest::Options {
        configure_client: Some(Box::new({
            let calls = calls.clone();
            move |builder| {
                if calls.fetch_add(1, Ordering::SeqCst) == 0 {
                    gix_error::bail!(gix_error::message("client configuration failed once"));
                }
                Ok(builder.no_proxy().user_agent("gix-custom-client"))
            }
        })),
        ..Default::default()
    }));
    let options = http::Options {
        backend: Some(backend),
        extra_headers: vec!["configured-header: retained".into()],
        ..Default::default()
    };
    let mut client = http::reqwest::Remote::default();
    client.configure(&options)?;
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "configuration waits for the first request"
    );

    let err = client
        .get(&url, &url, std::iter::empty::<&str>())
        .err()
        .expect("the first client configuration should fail");
    assert!(
        format!("{err:?}").contains("client configuration failed once"),
        "client setup preserves the callback error: {err:?}"
    );
    for _ in 0..2 {
        let mut response = client.get(&url, &url, std::iter::empty::<&str>())?;
        std::io::copy(&mut response.headers, &mut std::io::sink())?;
        let mut body = Vec::new();
        std::io::copy(&mut response.body, &mut body)?;
        assert_eq!(body, b"ok", "both requests use the initialized client");
    }
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "one failed setup and one shared successful client"
    );
    for request in server.join().expect("HTTP server thread completed")? {
        assert!(
            request
                .iter()
                .any(|line| line.eq_ignore_ascii_case("user-agent: gix-custom-client")),
            "the builder customization must affect requests: {request:?}"
        );
        assert!(
            request
                .iter()
                .any(|line| line.eq_ignore_ascii_case("configured-header: retained")),
            "worker restarts must retain HTTP configuration: {request:?}"
        );
    }
    Ok(())
}
