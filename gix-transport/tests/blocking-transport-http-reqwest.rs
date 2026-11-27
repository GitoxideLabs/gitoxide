//! Regression tests specific to the blocking `gix-reqwest` HTTP backend.

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
    gix_reqwest as reqwest,
};

mod http_helpers;
use http_helpers::{observe_connection_within_deadline, read_request_lines, response_with_connection_close};

fn serve_redirect(
    listener: std::net::TcpListener,
    delay: std::time::Duration,
) -> std::thread::JoinHandle<std::io::Result<Vec<Vec<String>>>> {
    std::thread::spawn(move || {
        listener.set_nonblocking(true)?;
        let mut requests = Vec::new();
        for response in [
            "HTTP/1.1 302 Found\r\nLocation: /redirected/repo\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        ] {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
            let stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(err)
                        if err.kind() == std::io::ErrorKind::WouldBlock && std::time::Instant::now() < deadline =>
                    {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => return Ok(requests),
                    Err(err) => return Err(err),
                }
            };
            stream.set_read_timeout(Some(std::time::Duration::from_secs(3)))?;
            let mut reader = std::io::BufReader::new(stream);
            requests.push(read_request_lines(&mut reader));
            std::thread::sleep(delay);
            // A timeout can close the socket before the delayed response is sent.
            reader.get_mut().write_all(response.as_bytes()).ok();
        }
        Ok(requests)
    })
}

#[test]
fn redirects_honor_the_client_referer_setting() -> gix_testtools::TestResult {
    use gix_transport::client::blocking_io::http::Http;

    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = gix_testtools::Env::new().set("no_proxy", "*");
    for enabled in [true, false] {
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let url = format!("http://{}/private/repo?secret=query", listener.local_addr()?);
        let server = serve_redirect(listener, std::time::Duration::ZERO);
        let mut remote = http::reqwest::Remote::default();
        remote.configure(&http::Options {
            backend: Some(Arc::new(Mutex::new(http::reqwest::Options {
                configure_client: Some(Box::new(move |builder| Ok(builder.referer(enabled)))),
                ..Default::default()
            }))),
            ..Default::default()
        })?;
        let mut response = remote.get(&url, &url, std::iter::empty::<&str>())?;
        std::io::copy(&mut response.headers, &mut std::io::sink())?;
        let requests = server.join().expect("HTTP server does not panic")?;
        assert_eq!(
            requests.len(),
            2,
            "the redirect is followed with either Referer setting"
        );
        let referer = requests[1].iter().find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("referer").then(|| value.trim())
        });
        assert_eq!(
            referer,
            enabled.then_some(url.as_str()),
            "redirects honor the configured Referer preference"
        );
    }
    Ok(())
}

#[test]
fn redirects_share_the_client_timeout_budget() -> gix_testtools::TestResult {
    use gix_transport::client::blocking_io::http::Http;

    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = gix_testtools::Env::new().set("no_proxy", "*");
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let url = format!("http://{}/repo", listener.local_addr()?);
    // Each response arrives within the limit, but the complete redirect chain exceeds it.
    let server = serve_redirect(listener, std::time::Duration::from_millis(300));
    let mut remote = http::reqwest::Remote::default();
    let mut builds = 0;
    remote.configure(&http::Options {
        backend: Some(Arc::new(Mutex::new(http::reqwest::Options {
            configure_client: Some(Box::new(move |builder| {
                builds += 1;
                Ok(builder
                    .timeout(std::time::Duration::from_millis(500))
                    .user_agent(format!("client-{builds}")))
            })),
            ..Default::default()
        }))),
        follow_redirects: http::options::FollowRedirects::All,
        ..Default::default()
    })?;
    let mut response = remote.get(&url, &url, std::iter::empty::<&str>())?;
    let result = std::io::copy(&mut response.headers, &mut std::io::sink());
    server.join().expect("HTTP server does not panic")?;
    let error = result.expect_err("redirect hops must share the client's total timeout");
    let error = error
        .get_ref()
        .and_then(|error| error.downcast_ref::<gix_error::Error>())
        .expect("reqwest errors retain their typed source");
    assert!(
        error
            .downcast_any_ref::<reqwest::Error>()
            .is_some_and(reqwest::Error::is_timeout),
        "the configured total timeout terminates the request"
    );
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let url = format!("http://{}/repo", listener.local_addr()?);
    let server = serve_redirect(listener, std::time::Duration::ZERO);
    let mut response = remote.get(&url, &url, std::iter::empty::<&str>())?;
    std::io::copy(&mut response.headers, &mut std::io::sink())?;
    let requests = server.join().expect("HTTP server does not panic")?;
    assert_eq!(requests.len(), 2, "a request after a timeout can follow redirects");
    assert!(
        requests.iter().all(|request| request
            .iter()
            .any(|line| line.eq_ignore_ascii_case("user-agent: client-2"))),
        "requests after a timeout use a fresh client so canceled callbacks cannot share their state"
    );
    Ok(())
}

#[test]
fn url_credentials_are_sensitive_in_request_hooks() -> gix_testtools::TestResult {
    use gix_transport::client::blocking_io::http::Http;

    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = gix_testtools::Env::new().set("no_proxy", "*");
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let url = format!("http://user:password@{}/repo", listener.local_addr()?);
    let server = std::thread::spawn(move || -> std::io::Result<()> {
        let (stream, _) = listener.accept()?;
        let mut reader = std::io::BufReader::new(stream);
        read_request_lines(&mut reader);
        reader
            .get_mut()
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
    });
    let authorization = Arc::new(Mutex::new(None));
    let mut remote = http::reqwest::Remote::default();
    remote.configure(&http::Options {
        backend: Some(Arc::new(Mutex::new(http::reqwest::Options {
            configure_request: Some(Box::new({
                let authorization = authorization.clone();
                move |request| {
                    *authorization.lock().expect("request hook does not panic") =
                        request.headers().get(reqwest::header::AUTHORIZATION).cloned();
                    Ok(())
                }
            })),
            ..Default::default()
        }))),
        ..Default::default()
    })?;
    let mut response = remote.get(&url, &url, std::iter::empty::<&str>())?;
    std::io::copy(&mut response.headers, &mut std::io::sink())?;
    server.join().expect("HTTP server does not panic")?;
    let authorization = authorization.lock().expect("request hook does not panic");
    let authorization = authorization.as_ref().expect("URL credentials produce Authorization");
    assert_eq!(
        authorization, "Basic dXNlcjpwYXNzd29yZA==",
        "URL credentials retain their encoding"
    );
    assert!(
        authorization.is_sensitive(),
        "request hooks and Debug must redact URL credentials"
    );
    Ok(())
}

#[test]
fn cross_authority_redirects_do_not_reapply_client_default_credentials() -> gix_testtools::TestResult {
    use gix_transport::client::blocking_io::http::Http;

    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = gix_testtools::Env::new().set("no_proxy", "*");
    let original = std::net::TcpListener::bind("127.0.0.1:0")?;
    let target = std::net::TcpListener::bind("127.0.0.1:0")?;
    let original_url = format!("http://{}/repo", original.local_addr()?);
    let target_url = format!("http://{}/repo", target.local_addr()?);
    let server = std::thread::spawn(move || -> std::io::Result<Vec<Vec<String>>> {
        let mut requests = Vec::new();
        for (listener, location) in [
            (original, Some(target_url.clone())),
            (target.try_clone()?, Some(target_url)),
            (target.try_clone()?, None),
            (target.try_clone()?, None),
            (target, None),
        ] {
            let (stream, _) = listener.accept()?;
            stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
            let mut reader = std::io::BufReader::new(stream);
            requests.push(read_request_lines(&mut reader));
            match location {
                Some(location) => write!(
                    reader.get_mut(),
                    "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                )?,
                None => reader
                    .get_mut()
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")?,
            }
        }
        Ok(requests)
    });
    let mut remote = http::reqwest::Remote::default();
    remote.configure(&http::Options {
        backend: Some(Arc::new(Mutex::new(http::reqwest::Options {
            configure_client: Some(Box::new(|builder| {
                let mut headers = reqwest::header::HeaderMap::new();
                for name in [
                    "authorization",
                    "cookie",
                    "cookie2",
                    "www-authenticate",
                    "proxy-authorization",
                ] {
                    headers.insert(
                        reqwest::header::HeaderName::from_static(name),
                        reqwest::header::HeaderValue::from_static("client-secret"),
                    );
                }
                Ok(builder.default_headers(headers).user_agent("custom-client"))
            })),
            ..Default::default()
        }))),
        ..Default::default()
    })?;
    let mut response = remote.get(&original_url, &original_url, std::iter::empty::<&str>())?;
    std::io::copy(&mut response.headers, &mut std::io::sink())?;
    std::io::copy(&mut response.body, &mut std::io::sink())?;
    // Git performs another logical request using the cached redirected base URL.
    let mut response = remote.get(&original_url, &original_url, std::iter::empty::<&str>())?;
    std::io::copy(&mut response.headers, &mut std::io::sink())?;
    std::io::copy(&mut response.body, &mut std::io::sink())?;
    let mut response = remote.get(&original_url, &original_url, ["Authorization: explicit-target"])?;
    std::io::copy(&mut response.headers, &mut std::io::sink())?;
    let requests = server.join().expect("HTTP server does not panic")?;
    assert!(
        requests[0].iter().any(|line| line.contains("client-secret")),
        "defaults reach the original authority"
    );
    for request in &requests[1..] {
        assert!(
            !request.iter().any(|line| line.contains("client-secret")),
            "redirects cannot reapply default secrets: {request:?}"
        );
        assert!(
            request
                .iter()
                .any(|line| line.eq_ignore_ascii_case("user-agent: custom-client")),
            "redirects retain ordinary client defaults"
        );
    }
    assert!(
        requests
            .last()
            .expect("the cached target receives subsequent requests")
            .iter()
            .any(|line| line.eq_ignore_ascii_case("authorization: explicit-target")),
        "explicit target authentication survives suppression of client defaults"
    );
    Ok(())
}

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

    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = gix_testtools::Env::new().set("no_proxy", "*");
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    let url = format!("http://{addr}/repo");
    let server = std::thread::spawn(move || -> std::io::Result<Vec<Vec<String>>> {
        let mut requests = Vec::new();
        for _ in 0..3 {
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
    let mut options = http::Options {
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
    for index in 0..3 {
        let url = if index == 2 {
            assert_eq!(calls.load(Ordering::SeqCst), 2, "unchanged proxies reuse the client");
            options.proxy = Some(format!("http://{addr}"));
            options.no_proxy = Some(String::new());
            client.configure(&options)?;
            "http://git.invalid/repo"
        } else {
            &url
        };
        let mut response = client.get(url, url, std::iter::empty::<&str>())?;
        std::io::copy(&mut response.headers, &mut std::io::sink())?;
        let mut body = Vec::new();
        std::io::copy(&mut response.body, &mut body)?;
        assert_eq!(body, b"ok", "both requests use the initialized client");
    }
    assert_eq!(
        calls.load(Ordering::SeqCst),
        3,
        "one failed setup, one shared client, and one rebuild for the configured proxy"
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
