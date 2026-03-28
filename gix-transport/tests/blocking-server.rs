use std::io::Write;

use gix_transport::{packetline::blocking_io::Writer, server, Protocol, Service};

/// Helper: write a connect message as a packetline, the way a real git client does.
fn write_connect_message(message: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut writer = Writer::new(&mut buf);
    writer.enable_binary_mode();
    writer.write_all(message).expect("write to vec cannot fail");
    writer.flush().expect("flush to vec cannot fail");
    buf
}

#[test]
fn version_1_without_host_and_version() {
    let request = server::parse_connect_message(b"git-upload-pack hello/world\0").expect("valid message");

    assert_eq!(request.service, Service::UploadPack);
    assert_eq!(request.repository_path, "hello/world");
    assert_eq!(request.virtual_host, None);
    assert_eq!(request.desired_protocol, Protocol::V1);
    assert!(request.extra_parameters.is_empty());
}

#[test]
fn version_2_without_host_and_version() {
    let request = server::parse_connect_message(b"git-upload-pack hello\\world\0\0version=2\0").expect("valid message");

    assert_eq!(request.service, Service::UploadPack);
    assert_eq!(request.repository_path, r"hello\world");
    assert_eq!(request.virtual_host, None);
    assert_eq!(request.desired_protocol, Protocol::V2);
    assert!(request.extra_parameters.is_empty());
}

#[test]
fn version_2_with_extra_parameters() {
    let request =
        server::parse_connect_message(b"git-upload-pack /path/project.git\0\0version=2\0key=value\0value-only\0")
            .expect("valid message");

    assert_eq!(request.service, Service::UploadPack);
    assert_eq!(request.repository_path, "/path/project.git");
    assert_eq!(request.virtual_host, None);
    assert_eq!(request.desired_protocol, Protocol::V2);
    assert_eq!(request.extra_parameters.len(), 2);
    assert_eq!(request.extra_parameters[0].0, "key");
    assert_eq!(request.extra_parameters[0].1, Some("value".into()));
    assert_eq!(request.extra_parameters[1].0, "value-only");
    assert_eq!(request.extra_parameters[1].1, None);
}

#[test]
fn with_host_without_port() {
    let request = server::parse_connect_message(b"git-upload-pack hello\\world\0host=host\0").expect("valid message");

    assert_eq!(request.service, Service::UploadPack);
    assert_eq!(request.repository_path, r"hello\world");
    assert_eq!(request.virtual_host, Some(("host".to_owned(), None)));
    assert_eq!(request.desired_protocol, Protocol::V1);
    assert!(request.extra_parameters.is_empty());
}

#[test]
fn with_host_without_port_and_extra_parameters() {
    let request = server::parse_connect_message(b"git-upload-pack hello\\world\0host=host\0\0key=value\0value-only\0")
        .expect("valid message");

    assert_eq!(request.service, Service::UploadPack);
    assert_eq!(request.repository_path, r"hello\world");
    assert_eq!(request.virtual_host, Some(("host".to_owned(), None)));
    assert_eq!(request.desired_protocol, Protocol::V1);
    assert_eq!(request.extra_parameters.len(), 2);
    assert_eq!(request.extra_parameters[0].0, "key");
    assert_eq!(request.extra_parameters[0].1, Some("value".into()));
    assert_eq!(request.extra_parameters[1].0, "value-only");
    assert_eq!(request.extra_parameters[1].1, None);
}

#[test]
fn with_host_with_port() {
    let request =
        server::parse_connect_message(b"git-upload-pack hello\\world\0host=host:404\0").expect("valid message");

    assert_eq!(request.service, Service::UploadPack);
    assert_eq!(request.repository_path, r"hello\world");
    assert_eq!(request.virtual_host, Some(("host".to_owned(), Some(404))));
    assert_eq!(request.desired_protocol, Protocol::V1);
    assert!(request.extra_parameters.is_empty());
}

#[test]
fn with_strange_host_and_port() {
    let request =
        server::parse_connect_message(b"git-upload-pack --upload-pack=attack\0host=--proxy=other-attack:404\0")
            .expect("valid message");

    assert_eq!(request.service, Service::UploadPack);
    assert_eq!(request.repository_path, "--upload-pack=attack");
    assert_eq!(
        request.virtual_host,
        Some(("--proxy=other-attack".to_owned(), Some(404)))
    );
    assert_eq!(request.desired_protocol, Protocol::V1);
    assert!(request.extra_parameters.is_empty());
}

// --- daemon::accept tests ---
// These simulate a real client writing the connect message as a packetline,
// then verify the server's daemon::accept() reads and parses it correctly.

#[test]
fn daemon_accept_v1_with_host() {
    let client_bytes = write_connect_message(b"git-upload-pack /repo.git\0host=myhost\0");

    let (connection, request) =
        server::blocking_io::daemon::accept(&client_bytes[..], Vec::new(), false).expect("valid connection");

    assert_eq!(connection.service, Service::UploadPack);
    assert_eq!(connection.repository_path, "/repo.git");
    assert_eq!(connection.protocol, Protocol::V1);
    assert_eq!(request.virtual_host, Some(("myhost".to_owned(), None)));
    assert!(request.extra_parameters.is_empty());
}

#[test]
fn daemon_accept_v2_with_host_and_port() {
    let client_bytes = write_connect_message(b"git-upload-pack /repo.git\0host=myhost:9418\0\0version=2\0");

    let (connection, request) =
        server::blocking_io::daemon::accept(&client_bytes[..], Vec::new(), false).expect("valid connection");

    assert_eq!(connection.service, Service::UploadPack);
    assert_eq!(connection.repository_path, "/repo.git");
    assert_eq!(connection.protocol, Protocol::V2);
    assert_eq!(request.virtual_host, Some(("myhost".to_owned(), Some(9418))));
}

#[test]
fn daemon_accept_receive_pack() {
    let client_bytes = write_connect_message(b"git-receive-pack /repo.git\0host=myhost\0");

    let (connection, _request) =
        server::blocking_io::daemon::accept(&client_bytes[..], Vec::new(), false).expect("valid connection");

    assert_eq!(connection.service, Service::ReceivePack);
    assert_eq!(connection.repository_path, "/repo.git");
}
