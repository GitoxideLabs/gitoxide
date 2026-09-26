use std::{
    io::{self, Read, Write},
    net::TcpListener,
    time::{Duration, Instant},
};

#[cfg(feature = "http-client-reqwest")]
use gix_error::{ErrorExt, ResultExt};
use gix_transport::client::blocking_io::http::{self, Http};

use super::Remote;
use crate::http_helpers::read_request_lines;

fn proxy_environment() -> gix_testtools::Env<'static> {
    [
        "http_proxy",
        "HTTP_PROXY",
        "https_proxy",
        "HTTPS_PROXY",
        "all_proxy",
        "ALL_PROXY",
        "no_proxy",
        "NO_PROXY",
    ]
    .into_iter()
    .fold(gix_testtools::Env::new(), gix_testtools::Env::unset)
}

fn accept(listener: TcpListener) -> io::Result<std::net::TcpStream> {
    listener.set_nonblocking(true)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let stream = loop {
        match listener.accept() {
            Ok((stream, _)) => break stream,
            Err(err) if err.kind() == io::ErrorKind::WouldBlock && Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(err) => return Err(err),
        }
    };
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    Ok(stream)
}

fn serve(listener: TcpListener, status: Option<u16>) -> std::thread::JoinHandle<io::Result<Vec<String>>> {
    std::thread::spawn(move || {
        let stream = accept(listener)?;
        let mut reader = io::BufReader::new(stream);
        let lines = read_request_lines(&mut reader);
        let body_len = lines
            .iter()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length:")?
                    .trim()
                    .parse()
                    .ok()
            })
            .unwrap_or(0);
        reader.read_exact(&mut vec![0; body_len])?;
        if let Some(status) = status {
            write!(
                reader.get_mut(),
                "HTTP/1.1 {status} status\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok"
            )?;
        }
        Ok(lines)
    })
}

fn request<H: Http>(
    remote: &mut H,
    url: &str,
    listener: TcpListener,
    post: bool,
) -> gix_testtools::Result<Vec<String>> {
    let server = serve(listener, Some(200));
    let response = if post {
        let mut response = remote.post(
            url,
            url,
            ["Accept: */*"],
            http::PostBodyDataKind::BoundedAndFitsIntoMemory,
        )?;
        response.post_body.write_all(b"pack")?;
        drop(response.post_body);
        http::GetResponse {
            headers: response.headers,
            body: response.body,
        }
    } else {
        remote.get(url, url, ["Accept: */*"])?
    };
    let mut headers = response.headers;
    let mut body = response.body;
    io::copy(&mut headers, &mut io::sink())?;
    let mut received = String::new();
    body.read_to_string(&mut received)?;
    assert_eq!(received, "ok", "the request must reach the selected listener");
    Ok(server.join().expect("recording server must not panic")?)
}

#[test]
fn configured_proxy_is_used_for_get_and_post() -> gix_testtools::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = proxy_environment();
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let mut remote = Remote::default();
    remote.configure(&http::Options {
        // Also exercise curl's scheme-less proxy syntax.
        proxy: Some(listener.local_addr()?.to_string()),
        no_proxy: Some(String::new()),
        ..Default::default()
    })?;
    let url = "http://git.invalid/repo/info/refs?service=git-upload-pack";
    for (post, method) in [(false, "GET"), (true, "POST")] {
        let lines = request(&mut remote, url, listener.try_clone()?, post)?;
        assert_eq!(
            lines[0],
            format!("{method} {url} HTTP/1.1"),
            "the proxy receives an absolute URL"
        );
    }
    let proxy = listener.local_addr()?.to_string();
    let server = serve(listener, Some(200));
    let directory = gix_testtools::tempfile::tempdir()?;
    gix_testtools::git_command(directory.path())
        .args([
            "-c",
            &format!("http.proxy={proxy}"),
            "ls-remote",
            "http://git.invalid/repo",
        ])
        .output()?;
    let lines = server.join().expect("recording server must not panic")?;
    assert_eq!(
        lines[0],
        format!("GET {url} HTTP/1.1"),
        "Git uses the same proxy request target"
    );
    Ok(())
}

#[test]
fn configured_proxy_and_empty_bypass_override_environment() -> gix_testtools::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let unused = TcpListener::bind("127.0.0.1:0")?;
    let _env = proxy_environment()
        .set("http_proxy", format!("http://{}", unused.local_addr()?))
        .set("no_proxy", "*");
    drop(unused);
    let mut remote = Remote::default();
    remote.configure(&http::Options {
        proxy: Some(format!("http://{}", listener.local_addr()?)),
        no_proxy: Some(String::new()),
        ..Default::default()
    })?;
    request(&mut remote, "http://git.invalid/repo", listener, false)?;
    Ok(())
}

#[test]
fn configured_bypass_applies_to_environment_proxy() -> gix_testtools::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let _env = proxy_environment()
        .set("http_proxy", format!("http://{}", listener.local_addr()?))
        .set("no_proxy", "*");
    let mut remote = Remote::default();
    remote.configure(&http::Options {
        no_proxy: Some(String::new()),
        ..Default::default()
    })?;
    request(&mut remote, "http://git.invalid/repo", listener, false)?;
    Ok(())
}

#[test]
fn bypass_and_empty_proxy_use_direct_connection() -> gix_testtools::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let unused = TcpListener::bind("127.0.0.1:0")?;
    let proxy = format!("http://{}", unused.local_addr()?);
    drop(unused);
    let _env = proxy_environment().set("http_proxy", proxy.clone());
    for (proxy, no_proxy) in [
        (proxy.clone(), "*"),
        ("htpp://proxy.invalid".into(), "*"),
        (proxy.clone(), "example.com,127.0.0.1"),
        (proxy, "127.0.0.0/8"),
        (String::new(), ""),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let url = format!("http://{}/repo", listener.local_addr()?);
        let mut remote = Remote::default();
        remote.configure(&http::Options {
            proxy: Some(proxy),
            no_proxy: Some(no_proxy.into()),
            ..Default::default()
        })?;
        let lines = request(&mut remote, &url, listener, false)?;
        assert_eq!(
            lines[0], "GET /repo HTTP/1.1",
            "bypassed requests go directly to the origin"
        );
    }
    Ok(())
}

#[test]
fn reconfiguration_changes_proxy() -> gix_testtools::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = proxy_environment();
    let mut remote = Remote::default();
    for _ in 0..2 {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        remote.configure(&http::Options {
            proxy: Some(format!("http://{}", listener.local_addr()?)),
            no_proxy: Some(String::new()),
            ..Default::default()
        })?;
        request(&mut remote, "http://git.invalid/repo", listener, false)?;
    }
    Ok(())
}

fn check_proxy_credentials<H: Http + Default>(
    use_helper: bool,
    bypass: bool,
    status: Option<u16>,
    https: bool,
    environment_proxy: bool,
) -> gix_testtools::Result {
    use std::sync::{Arc, Mutex};

    use gix_credentials::{helper::Action, protocol::Context};

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?;
    let mut options = http::Options {
        proxy: Some(format!(
            "http://user{}@{address}",
            if use_helper { "" } else { ":p%3Aass" }
        )),
        proxy_auth_method: http::options::ProxyAuthMethod::Basic,
        no_proxy: Some(if bypass { "127.0.0.1" } else { "" }.into()),
        ..Default::default()
    };
    let actions = Arc::new(Mutex::new(Vec::new()));
    if use_helper {
        let actions = actions.clone();
        options.proxy_authenticate = Some((
            Action::get_for_url(options.proxy.as_deref().expect("proxy is set")),
            Arc::new(Mutex::new(move |action: Action| {
                actions.lock().expect("no panics").push(action.as_arg(true).to_owned());
                if bypass {
                    return Ok(None);
                }
                Ok(if let Action::Get(_) = action {
                    Some(gix_credentials::protocol::Outcome {
                        identity: gix_sec::identity::Account {
                            username: "user".into(),
                            password: "p:ass".into(),
                            oauth_refresh_token: None,
                        },
                        next: Context {
                            username: Some("user".into()),
                            password: Some("p:ass".into()),
                            ..Default::default()
                        }
                        .into(),
                    })
                } else {
                    None
                })
            })),
        ));
    }
    let _environment = if environment_proxy {
        Some(gix_testtools::Env::new().set(
            if https { "https_proxy" } else { "http_proxy" },
            options.proxy.take().expect("proxy is configured"),
        ))
    } else {
        None
    };
    let mut remote = H::default();
    remote.configure(&options)?;
    let url = if bypass {
        format!("http://{address}/repo")
    } else if https {
        "https://git.invalid/repo".into()
    } else {
        "http://git.invalid/repo".into()
    };
    let server = serve(listener, status);
    let mut response = remote.get(&url, &url, ["Accept: */*"])?;
    let result = io::copy(&mut response.headers, &mut io::sink());
    assert_eq!(
        result.is_ok(),
        !https && status.is_some_and(|status| status < 400),
        "HTTP errors and dropped connections are reported: {status:?}"
    );
    io::copy(&mut response.body, &mut io::sink())?;
    let lines = server.join().expect("recording server must not panic")?;
    if https {
        assert_eq!(
            lines[0], "CONNECT git.invalid:443 HTTP/1.1",
            "HTTPS authenticates the proxy tunnel"
        );
    }
    let proxy_auth = lines
        .iter()
        .find(|line| line.to_ascii_lowercase().starts_with("proxy-authorization:"));
    assert_eq!(
        proxy_auth.and_then(|line| line.split_once(':').map(|(_, value)| value.trim())),
        (!bypass).then_some("Basic dXNlcjpwOmFzcw=="),
        "decoded credentials authenticate only requests that use the proxy: helper={use_helper}, bypass={bypass}"
    );
    assert!(
        !lines
            .iter()
            .any(|line| line.to_ascii_lowercase().starts_with("authorization:")),
        "proxy credentials must never become origin credentials"
    );
    if use_helper {
        let expected = match status {
            _ if bypass => Vec::new(),
            Some(407) => vec!["get", "erase"],
            Some(_) if !https => vec!["get", "store"],
            _ => vec!["get"],
        };
        assert_eq!(
            *actions.lock().expect("no panics"),
            expected,
            "only proxy authentication rejection invalidates credentials: status={status:?}, https={https}, response={result:?}"
        );
    }
    Ok(())
}

#[test]
fn basic_proxy_credentials_do_not_authenticate_the_origin() -> gix_testtools::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = proxy_environment();
    for bypass in [false, true] {
        check_proxy_credentials::<Remote>(false, bypass, Some(200), false, false)?;
    }
    check_proxy_credentials::<Remote>(true, false, Some(200), false, false)?;
    Ok(())
}

#[test]
fn proxy_authentication_survives_http_redirects_without_leaking_to_bypassed_origins() -> gix_testtools::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = proxy_environment();
    for bypass in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let proxy_address = listener.local_addr()?;
        let target = if bypass {
            TcpListener::bind("127.0.0.1:0")?
        } else {
            listener.try_clone()?
        };
        let target_url = if bypass {
            format!("http://{}/repo", target.local_addr()?)
        } else {
            "http://redirected.invalid/repo".into()
        };
        let server = std::thread::spawn(move || -> io::Result<_> {
            let mut reader = io::BufReader::new(accept(listener)?);
            let first = read_request_lines(&mut reader);
            write!(
                reader.get_mut(),
                "HTTP/1.1 302 Found\r\nLocation: {target_url}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )?;
            drop(reader);
            let mut reader = io::BufReader::new(accept(target)?);
            let second = read_request_lines(&mut reader);
            reader
                .get_mut()
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")?;
            Ok([first, second])
        });
        let mut remote = Remote::default();
        remote.configure(&http::Options {
            proxy: Some(format!("http://user:pass@{proxy_address}")),
            proxy_auth_method: http::options::ProxyAuthMethod::Basic,
            no_proxy: Some(if bypass { "127.0.0.1" } else { "" }.into()),
            ..Default::default()
        })?;
        let mut response = remote.get(
            "http://git.invalid/repo",
            "http://git.invalid/repo",
            ["Authorization: Basic b3JpZ2luOnNlY3JldA=="],
        )?;
        io::copy(&mut response.headers, &mut io::sink())?;
        io::copy(&mut response.body, &mut io::sink())?;
        let [first, second] = server.join().expect("redirect proxy must not panic")?;
        for (lines, uses_proxy) in [(&first, true), (&second, !bypass)] {
            let auth = lines.iter().find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("proxy-authorization").then(|| value.trim())
            });
            assert_eq!(
                auth,
                uses_proxy.then_some("Basic dXNlcjpwYXNz"),
                "each proxied request is authenticated"
            );
        }
        assert!(
            !second
                .iter()
                .any(|line| line.to_ascii_lowercase().starts_with("authorization:")),
            "origin credentials do not cross authorities"
        );
    }
    Ok(())
}

#[cfg(feature = "http-client-reqwest")]
#[test]
fn reqwest_preserves_explicit_origin_authorization_with_url_credentials() -> gix_testtools::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = proxy_environment();
    for explicit in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let url = format!("http://user:pass@{}/repo", listener.local_addr()?);
        let mut remote = http::reqwest::Remote::default();
        remote.configure(&http::Options {
            proxy: Some(String::new()),
            extra_headers: if explicit {
                vec!["Authorization: Bearer explicit-token".into()]
            } else {
                Vec::new()
            },
            ..Default::default()
        })?;
        let lines = request(&mut remote, &url, listener, false)?;
        let authorization = lines.iter().find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("authorization").then(|| value.trim())
        });
        assert_eq!(
            authorization,
            Some(if explicit {
                "Bearer explicit-token"
            } else {
                "Basic dXNlcjpwYXNz"
            }),
            "explicit authentication takes precedence over URL credentials"
        );
    }
    Ok(())
}

#[cfg(feature = "http-client-reqwest")]
#[test]
fn reqwest_proxy_setup_errors_release_streamed_uploads() -> gix_testtools::Result {
    use std::sync::{Arc, Mutex, mpsc};

    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = proxy_environment().set("http_proxy", "http://invalid:port");
    for helper_error in [false, true] {
        let (send, receive) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let mut remote = http::reqwest::Remote::default();
            if helper_error {
                remote
                    .configure(&http::Options {
                        proxy: Some("http://user@127.0.0.1:9".into()),
                        proxy_authenticate: Some((
                            gix_credentials::helper::Action::get_for_url("http://user@127.0.0.1:9"),
                            Arc::new(Mutex::new(|_| {
                                Err(
                                    gix_error::message("The handler asked to stop trying to obtain credentials")
                                        .raise_erased(),
                                )
                            })),
                        )),
                        ..Default::default()
                    })
                    .expect("options can be configured");
            }
            let mut response = remote
                .post(
                    "http://git.invalid/repo",
                    "http://git.invalid/repo",
                    ["Accept: */*"],
                    http::PostBodyDataKind::Unbounded,
                )
                .expect("streamed requests return their pipes before sending the body");
            let write_error = response
                .post_body
                .write_all(b"pack")
                .expect_err("failed setup releases the upload writer");
            drop(response.post_body);
            let header_error =
                io::copy(&mut response.headers, &mut io::sink()).expect_err("the setup error is reported");
            send.send((write_error.kind(), format!("{header_error:?}")))
                .expect("test receives the result");
        });
        let (kind, error) = receive.recv_timeout(Duration::from_secs(5))?;
        worker.join().expect("upload worker must not panic");
        assert_eq!(
            kind,
            io::ErrorKind::BrokenPipe,
            "the failed request closes its upload pipe"
        );
        assert!(
            error.contains(if helper_error {
                "Could not obtain proxy credentials"
            } else {
                "Invalid proxy URL"
            }),
            "the original proxy setup error remains available: {error}"
        );
    }
    Ok(())
}

#[cfg(feature = "http-client-reqwest")]
#[test]
fn reqwest_proxy_redirects_preserve_post_body_rules() -> gix_testtools::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = proxy_environment();
    for streaming in [false, true] {
        for status in [301, 302, 303, 307, 308] {
            let listener = TcpListener::bind("127.0.0.1:0")?;
            let address = listener.local_addr()?;
            let keeps_body = status >= 307;
            let follows = !streaming || !keeps_body;
            let server = std::thread::spawn(move || -> io::Result<_> {
                let mut reader = io::BufReader::new(accept(listener.try_clone()?)?);
                read_request_lines(&mut reader);
                let mut body = [0; 4];
                reader.read_exact(&mut body)?;
                assert_eq!(&body, b"pack", "the first POST contains the upload");
                write!(
                    reader.get_mut(),
                    "HTTP/1.1 {status} Redirect\r\nLocation: http://redirected.invalid/repo\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                )?;
                drop(reader);
                if !follows {
                    return Ok(None);
                }
                let mut reader = io::BufReader::new(accept(listener)?);
                let lines = read_request_lines(&mut reader);
                let mut body = if keeps_body { vec![0; 4] } else { Vec::new() };
                reader.read_exact(&mut body)?;
                reader
                    .get_mut()
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")?;
                Ok(Some((lines, body)))
            });
            let mut remote = http::reqwest::Remote::default();
            remote.configure(&http::Options {
                proxy: Some(format!("http://{address}")),
                no_proxy: Some(String::new()),
                ..Default::default()
            })?;
            let mut response = remote.post(
                "http://git.invalid/repo",
                "http://git.invalid/repo",
                [
                    "Content-Length: 4",
                    "Content-Type: application/x-git-upload-pack-request",
                ],
                if streaming {
                    http::PostBodyDataKind::Unbounded
                } else {
                    http::PostBodyDataKind::BoundedAndFitsIntoMemory
                },
            )?;
            response.post_body.write_all(b"pack")?;
            drop(response.post_body);
            io::copy(&mut response.headers, &mut io::sink())?;
            io::copy(&mut response.body, &mut io::sink())?;
            let redirected = server.join().expect("redirect server must not panic")?;
            assert_eq!(
                redirected.is_some(),
                follows,
                "streamed uploads cannot be replayed for {status}"
            );
            if let Some((lines, body)) = redirected {
                let method = if keeps_body { "POST" } else { "GET" };
                assert_eq!(
                    lines[0],
                    format!("{method} http://redirected.invalid/repo HTTP/1.1"),
                    "redirect method for {status}"
                );
                assert_eq!(
                    body.as_slice(),
                    if keeps_body { b"pack".as_slice() } else { b"" },
                    "redirect body for {status}"
                );
                assert_eq!(
                    lines
                        .iter()
                        .any(|line| line.to_ascii_lowercase().starts_with("content-type:")),
                    keeps_body,
                    "redirects to GET drop the upload headers"
                );
            }
        }
    }
    Ok(())
}

#[cfg(feature = "http-client-reqwest")]
#[test]
fn reqwest_proxy_credentials_come_from_helpers() -> gix_testtools::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = proxy_environment();
    for status in [Some(200), Some(201), Some(401), Some(404), Some(500), Some(407), None] {
        check_proxy_credentials::<http::reqwest::Remote>(true, false, status, false, false)?;
    }
    check_proxy_credentials::<http::reqwest::Remote>(true, true, Some(200), false, false)?;
    for status in [Some(200), Some(407)] {
        check_proxy_credentials::<http::reqwest::Remote>(true, false, status, false, true)?;
    }
    #[cfg(any(
        feature = "http-client-reqwest-rust-tls",
        feature = "http-client-reqwest-rust-tls-trust-dns",
        feature = "http-client-reqwest-native-tls"
    ))]
    for status in [Some(407), Some(403), None] {
        check_proxy_credentials::<http::reqwest::Remote>(true, false, status, true, false)?;
        check_proxy_credentials::<http::reqwest::Remote>(true, false, status, true, true)?;
    }
    Ok(())
}

#[cfg(feature = "http-client-reqwest")]
#[test]
fn reqwest_rejects_unsupported_proxy_configuration_on_every_attempt() -> gix_testtools::Result {
    use http::options::ProxyAuthMethod;

    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = proxy_environment();
    for (proxy, method, message) in [
        (
            "htpp://proxy.invalid",
            ProxyAuthMethod::AnyAuth,
            "Unsupported proxy scheme",
        ),
        (
            "http://proxy.invalid:invalid",
            ProxyAuthMethod::AnyAuth,
            "Invalid proxy URL",
        ),
        (
            "socks5://localhost/proxy.sock",
            ProxyAuthMethod::AnyAuth,
            "Unix socket proxy paths",
        ),
        (
            "socks4://alice@proxy.invalid",
            ProxyAuthMethod::AnyAuth,
            "SOCKS4 proxy user IDs",
        ),
        (
            "socks4a://alice@proxy.invalid",
            ProxyAuthMethod::AnyAuth,
            "SOCKS4 proxy user IDs",
        ),
        (
            "http://user:secret@proxy.invalid",
            ProxyAuthMethod::Digest,
            "only supports Basic",
        ),
        (
            "http://user:secret@proxy.invalid",
            ProxyAuthMethod::Negotiate,
            "only supports Basic",
        ),
        (
            "http://user:secret@proxy.invalid",
            ProxyAuthMethod::Ntlm,
            "only supports Basic",
        ),
    ] {
        let mut remote = http::reqwest::Remote::default();
        remote.configure(&http::Options {
            proxy: Some(proxy.into()),
            proxy_auth_method: method,
            ..Default::default()
        })?;
        for _ in 0..2 {
            let error = match remote.get("http://127.0.0.1:9/repo", "http://127.0.0.1:9/repo", ["Accept: */*"]) {
                Ok(mut response) => format!(
                    "{:?}",
                    io::copy(&mut response.headers, &mut io::sink())
                        .expect_err("invalid proxy settings must fail on every request")
                ),
                Err(error) => format!("{error:?}"),
            };
            assert!(
                format!("{error:?}").contains(message),
                "expected {message:?}, got {error:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn environment_proxy_fallback_and_precedence() -> gix_testtools::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    for variable in ["http_proxy", "all_proxy", "ALL_PROXY"] {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let environment = proxy_environment().set(variable, format!("http://{}", listener.local_addr()?));
        let environment = if variable != "http_proxy" {
            environment.set("http_proxy", "")
        } else {
            environment
        };
        #[cfg(not(windows))]
        let environment = if variable == "ALL_PROXY" {
            environment.set("all_proxy", "")
        } else {
            environment
        };
        // Windows environment variable names are case-insensitive.
        #[cfg(not(windows))]
        let environment = environment
            .set("HTTP_PROXY", "http://127.0.0.1:9")
            .set("no_proxy", "example.invalid")
            .set("NO_PROXY", "*");
        let _env = environment;
        request(&mut Remote::default(), "http://git.invalid/repo", listener, false)?;
    }
    Ok(())
}

// Reqwest 0.13's connector rejects SOCKS URLs when built without TLS.
#[cfg(any(
    feature = "http-client-curl",
    feature = "http-client-reqwest-rust-tls",
    feature = "http-client-reqwest-rust-tls-trust-dns",
    feature = "http-client-reqwest-native-tls"
))]
#[test]
fn socks5h_resolves_the_destination_at_the_proxy() -> gix_testtools::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = proxy_environment();
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let mut remote = Remote::default();
    remote.configure(&http::Options {
        proxy: Some(format!("socks5h://{}", listener.local_addr()?)),
        no_proxy: Some(String::new()),
        ..Default::default()
    })?;
    let server = std::thread::spawn(move || -> gix_testtools::Result {
        let mut stream = accept(listener)?;
        let mut greeting = [0; 2];
        stream.read_exact(&mut greeting)?;
        assert_eq!(greeting[0], 5, "SOCKS5 negotiation");
        let mut methods = vec![0; usize::from(greeting[1])];
        stream.read_exact(&mut methods)?;
        assert!(methods.contains(&0), "the proxy can select unauthenticated access");
        stream.write_all(&[5, 0])?;
        let mut connect = [0; 5];
        stream.read_exact(&mut connect)?;
        assert_eq!(&connect[..4], [5, 1, 0, 3], "SOCKS5h sends a domain, without local DNS");
        let mut host = vec![0; usize::from(connect[4])];
        stream.read_exact(&mut host)?;
        assert_eq!(host, b"git.invalid", "the proxy receives the original hostname");
        let mut port = [0; 2];
        stream.read_exact(&mut port)?;
        assert_eq!(
            u16::from_be_bytes(port),
            80,
            "the proxy connects to the destination port"
        );
        // Reject after recording the request; no external server is needed.
        stream.write_all(&[5, 4, 0, 1, 0, 0, 0, 0, 0, 0])?;
        Ok(())
    });
    let mut response = remote.get("http://git.invalid/repo", "http://git.invalid/repo", ["Accept: */*"])?;
    let error = io::copy(&mut response.headers, &mut io::sink()).expect_err("the proxy rejection is reported");
    server
        .join()
        .expect("SOCKS server must not panic")
        .map_err(|server_error| format!("SOCKS server failed: {server_error}; client error: {error:?}"))?;
    Ok(())
}

#[cfg(any(
    feature = "http-client-curl-rust-tls",
    feature = "http-client-curl-openssl",
    feature = "http-client-reqwest-rust-tls",
    feature = "http-client-reqwest-rust-tls-trust-dns",
    feature = "http-client-reqwest-native-tls"
))]
#[test]
fn redirects_select_environment_proxy_for_new_scheme() -> gix_testtools::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    for use_http_proxy in [false, true] {
        let http_listener = TcpListener::bind("127.0.0.1:0")?;
        let https_proxy = TcpListener::bind("127.0.0.1:0")?;
        let http_address = http_listener.local_addr()?;
        let environment = proxy_environment().set("HTTPS_PROXY", format!("http://{}", https_proxy.local_addr()?));
        let _env = if use_http_proxy {
            environment.set("http_proxy", format!("http://{http_address}"))
        } else {
            environment
        };
        let redirect = std::thread::spawn(move || -> io::Result<Vec<String>> {
            let mut reader = io::BufReader::new(accept(http_listener)?);
            let lines = read_request_lines(&mut reader);
            reader.get_mut().write_all(
                b"HTTP/1.1 302 Found\r\nLocation: https://git.invalid/repo\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            )?;
            Ok(lines)
        });
        let tunnel = serve(https_proxy, Some(403));
        let url = if use_http_proxy {
            "http://git.invalid/repo".into()
        } else {
            format!("http://{http_address}/repo")
        };
        let mut remote = Remote::default();
        let mut response = remote.get(&url, &url, ["Accept: */*"])?;
        let error = io::copy(&mut response.headers, &mut io::sink()).expect_err("the proxy rejected the tunnel");
        let lines = redirect.join().expect("redirect server must not panic")?;
        assert_eq!(
            lines[0],
            if use_http_proxy {
                "GET http://git.invalid/repo HTTP/1.1"
            } else {
                "GET /repo HTTP/1.1"
            },
            "the initial request uses the HTTP proxy only when configured"
        );
        let lines = tunnel
            .join()
            .expect("CONNECT server must not panic")
            .map_err(|server_error| format!("HTTPS proxy was not used: {server_error}; client error: {error:?}"))?;
        assert_eq!(
            lines[0], "CONNECT git.invalid:443 HTTP/1.1",
            "the redirect selects HTTPS_PROXY, independently of the initial HTTP proxy"
        );
    }
    Ok(())
}

#[cfg(any(
    feature = "http-client-curl-rust-tls",
    feature = "http-client-curl-openssl",
    feature = "http-client-reqwest-rust-tls",
    feature = "http-client-reqwest-rust-tls-trust-dns",
    feature = "http-client-reqwest-native-tls"
))]
#[test]
fn https_uses_connect_and_https_proxy_environment() -> gix_testtools::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let _env = proxy_environment()
        .set("http_proxy", "http://proxy.invalid:invalid")
        .set("HTTPS_PROXY", format!("http://user:pass@{}", listener.local_addr()?));
    let server = std::thread::spawn(move || -> io::Result<Vec<String>> {
        let mut reader = io::BufReader::new(accept(listener)?);
        let lines = read_request_lines(&mut reader);
        reader
            .get_mut()
            .write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")?;
        Ok(lines)
    });
    let mut remote = Remote::default();
    remote.configure(&http::Options {
        proxy_auth_method: http::options::ProxyAuthMethod::Basic,
        no_proxy: Some(String::new()),
        ..Default::default()
    })?;
    let mut response = remote.get("https://git.invalid/repo", "https://git.invalid/repo", ["Accept: */*"])?;
    assert!(
        io::copy(&mut response.headers, &mut io::sink()).is_err(),
        "the proxy rejected the tunnel"
    );
    let lines = server.join().expect("CONNECT server must not panic")?;
    assert_eq!(
        lines[0], "CONNECT git.invalid:443 HTTP/1.1",
        "HTTPS uses a proxy tunnel"
    );
    assert!(
        lines.iter().any(|line| {
            line.split_once(':').is_some_and(|(name, value)| {
                name.eq_ignore_ascii_case("proxy-authorization") && value.trim() == "Basic dXNlcjpwYXNz"
            })
        }),
        "proxy credentials are attached to CONNECT: {lines:?}"
    );
    Ok(())
}

#[cfg(any(
    feature = "http-client-reqwest-rust-tls",
    feature = "http-client-reqwest-rust-tls-trust-dns",
    feature = "http-client-reqwest-native-tls"
))]
#[test]
fn reqwest_defers_invalid_https_proxy_until_redirect() -> gix_testtools::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = proxy_environment().set("HTTPS_PROXY", "socks5://localhost/proxy.sock");
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let url = format!("http://{}/repo", listener.local_addr()?);
    let mut remote = http::reqwest::Remote::default();
    remote.configure(&http::Options {
        follow_redirects: http::options::FollowRedirects::All,
        ..Default::default()
    })?;
    request(&mut remote, &url, listener.try_clone()?, false)?;

    let redirect = std::thread::spawn(move || -> io::Result<()> {
        let mut reader = io::BufReader::new(accept(listener)?);
        read_request_lines(&mut reader);
        reader.get_mut().write_all(
            b"HTTP/1.1 302 Found\r\nLocation: https://git.invalid/repo\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        )
    });
    let mut response = remote.get(&url, &url, ["Accept: */*"])?;
    let error = io::copy(&mut response.headers, &mut io::sink()).expect_err("the HTTPS proxy is invalid");
    assert!(
        format!("{error:?}").contains("Unix socket proxy paths"),
        "the cached client rejects the redirect before sending an unproxied request: {error:?}"
    );
    redirect.join().expect("redirect server must not panic")?;
    Ok(())
}

#[cfg(feature = "http-client-reqwest")]
#[test]
fn reqwest_checks_proxy_errors_after_request_url_changes() -> gix_testtools::Result {
    use std::sync::{Arc, Mutex};

    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = proxy_environment().set("HTTPS_PROXY", "socks5://localhost/proxy.sock");
    let mut remote = http::reqwest::Remote::default();
    remote.configure(&http::Options {
        backend: Some(Arc::new(Mutex::new(http::reqwest::Options {
            configure_request: Some(Box::new(|request| {
                *request.url_mut() = "https://git.invalid/repo".parse::<reqwest::Url>().or_erased()?;
                Ok(())
            })),
        }))),
        ..Default::default()
    })?;
    let mut response = remote.get("http://git.invalid/repo", "http://git.invalid/repo", ["Accept: */*"])?;
    let error = io::copy(&mut response.headers, &mut io::sink()).expect_err("the HTTPS proxy is invalid");
    assert!(
        format!("{error:?}").contains("Unix socket proxy paths"),
        "request URL changes must not bypass proxy validation: {error:?}"
    );
    Ok(())
}
