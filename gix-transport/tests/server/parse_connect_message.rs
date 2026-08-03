use bstr::BString;
use gix_transport::{Protocol, Service, server};

#[test]
fn minimal_upload_pack() {
    let req = server::parse_connect_message(b"git-upload-pack /repo.git\0").expect("valid");
    assert_eq!(req.service, Service::UploadPack);
    assert_eq!(req.repository_path, "/repo.git");
    assert_eq!(req.virtual_host, None);
    assert_eq!(req.protocol, Protocol::V1);
    assert!(req.extra_parameters.is_empty());
}

#[test]
fn minimal_receive_pack() {
    let req = server::parse_connect_message(b"git-receive-pack /project.git\0").expect("valid");
    assert_eq!(req.service, Service::ReceivePack);
    assert_eq!(req.repository_path, "/project.git");
}

#[test]
fn with_host_no_port() {
    let req = server::parse_connect_message(b"git-upload-pack /repo.git\0host=git.example.com\0").expect("valid");
    assert_eq!(req.virtual_host, Some(("git.example.com".to_owned(), None)));
}

#[test]
fn with_host_and_port() {
    let req =
        server::parse_connect_message(b"git-upload-pack /repo.git\0host=git.example.com:9418\0").expect("valid");
    assert_eq!(req.virtual_host, Some(("git.example.com".to_owned(), Some(9418))));
}

#[test]
fn protocol_v0() {
    let req =
        server::parse_connect_message(b"git-upload-pack /repo.git\0host=h\0\0version=0\0").expect("valid");
    assert_eq!(req.protocol, Protocol::V0);
}

#[test]
fn protocol_v2() {
    let req =
        server::parse_connect_message(b"git-upload-pack /repo.git\0host=h\0\0version=2\0").expect("valid");
    assert_eq!(req.protocol, Protocol::V2);
}

#[test]
fn extra_parameters_are_preserved() {
    let req = server::parse_connect_message(
        b"git-upload-pack /repo.git\0host=h\0\0version=2\0object-format=sha256\0bare\0",
    )
    .expect("valid");
    assert_eq!(req.protocol, Protocol::V2);
    assert_eq!(
        req.extra_parameters,
        vec![
            (BString::from("object-format"), Some(BString::from("sha256"))),
            (BString::from("bare"), None),
        ]
    );
}

#[test]
fn path_with_tilde_expansion() {
    let req = server::parse_connect_message(b"git-upload-pack ~user/repo.git\0").expect("valid");
    assert_eq!(req.repository_path, "~user/repo.git");
}

#[test]
fn path_with_special_characters() {
    let req = server::parse_connect_message(b"git-upload-pack /path/to/my repo.git\0").expect("valid");
    assert_eq!(req.repository_path, "/path/to/my repo.git");
}

#[test]
fn unknown_service_is_rejected() {
    let err = server::parse_connect_message(b"git-unknown /repo.git\0").unwrap_err();
    assert!(matches!(err, server::Error::UnknownService { .. }));
}

#[test]
fn missing_space_is_malformed() {
    let err = server::parse_connect_message(b"git-upload-pack").unwrap_err();
    assert!(matches!(err, server::Error::MalformedMessage));
}

#[test]
fn empty_input_is_malformed() {
    let err = server::parse_connect_message(b"").unwrap_err();
    assert!(matches!(err, server::Error::MalformedMessage));
}

#[test]
fn ipv6_bracketed_with_port() {
    let req = server::parse_connect_message(b"git-upload-pack /repo.git\0host=[::1]:9418\0").expect("valid");
    assert_eq!(req.virtual_host, Some(("::1".to_owned(), Some(9418))));
}

#[test]
fn ipv6_bracketed_without_port() {
    let req = server::parse_connect_message(b"git-upload-pack /repo.git\0host=[fe80::1]\0").expect("valid");
    assert_eq!(req.virtual_host, Some(("fe80::1".to_owned(), None)));
}

#[test]
fn invalid_protocol_version_is_malformed() {
    let err = server::parse_connect_message(b"git-upload-pack /repo.git\0\0version=99\0").unwrap_err();
    assert!(matches!(err, server::Error::MalformedMessage));
}

#[test]
fn multiple_null_separators_are_handled() {
    // The format uses double-NUL to separate host from extra params.
    // Extra empty segments between NULs should be skipped.
    let req = server::parse_connect_message(
        b"git-upload-pack /repo.git\0host=h\0\0\0version=2\0",
    )
    .expect("valid");
    assert_eq!(req.protocol, Protocol::V2);
    assert_eq!(req.virtual_host, Some(("h".to_owned(), None)));
}
