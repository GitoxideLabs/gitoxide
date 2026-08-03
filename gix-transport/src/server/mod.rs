//! Server-side transport primitives for handling incoming git protocol connections.
//!
//! This module provides the types and parsing logic needed to accept a git protocol
//! connection, determine which service the client is requesting, and hand off to the
//! appropriate protocol handler (e.g. `upload-pack` or `receive-pack`).

use bstr::{BString, ByteSlice};

use crate::{Protocol, Service};

///
#[cfg(feature = "blocking-client")]
pub mod blocking_io;

///
#[cfg(feature = "async-client")]
pub mod async_io;

/// The request parsed from a client's initial connect message.
///
/// Parsed from the `git-proto-request` format described in the
/// [git pack-protocol documentation](https://git-scm.com/docs/pack-protocol#_git_transport).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectRequest {
    /// The requested service, e.g. `UploadPack` or `ReceivePack`.
    pub service: Service,
    /// The repository path the client wants to access, e.g. `/repo.git`.
    pub repository_path: BString,
    /// The virtual host and optional port from `host=<host>[:<port>]`.
    pub virtual_host: Option<(String, Option<u16>)>,
    /// The protocol version requested via `version=N` extra parameter.
    /// Defaults to `V1` if unspecified.
    pub protocol: Protocol,
    /// Additional key-value parameters beyond `version=` and `host=`.
    pub extra_parameters: Vec<(BString, Option<BString>)>,
}

/// Errors from parsing a git daemon connect message.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Unknown service: {service:?}")]
    UnknownService { service: BString },
    #[error("Malformed connect message")]
    MalformedMessage,
}

/// Parse a git daemon connect message into a [`ConnectRequest`].
///
/// The input `bytes` should be the raw data payload of the first packetline sent by the client,
/// in the `git-proto-request` format: `<service> <path>\0[host=<host>[:<port>]\0][extra params]`.
pub fn parse_connect_message(bytes: &[u8]) -> Result<ConnectRequest, Error> {
    let (service_bytes, rest) = bytes.split_once_str(b" ").ok_or(Error::MalformedMessage)?;

    let service = match service_bytes {
        b"git-upload-pack" => Service::UploadPack,
        b"git-receive-pack" => Service::ReceivePack,
        _ => {
            return Err(Error::UnknownService {
                service: service_bytes.into(),
            });
        }
    };

    let mut segments = rest.split_str(b"\0");
    let path: BString = segments.next().ok_or(Error::MalformedMessage)?.into();

    let mut virtual_host = None;
    let mut protocol = Protocol::V1;
    let mut extra_parameters = Vec::new();

    for segment in segments {
        if segment.is_empty() {
            continue;
        }

        if let Some(host_value) = segment.strip_prefix(b"host=") {
            let host_str = std::str::from_utf8(host_value).map_err(|_| Error::MalformedMessage)?;
            virtual_host = Some(parse_host_port(host_str)?);
        } else if let Some(version_value) = segment.strip_prefix(b"version=") {
            let version_str = std::str::from_utf8(version_value).map_err(|_| Error::MalformedMessage)?;
            protocol = match version_str {
                "0" => Protocol::V0,
                "1" => Protocol::V1,
                "2" => Protocol::V2,
                _ => return Err(Error::MalformedMessage),
            };
        } else {
            match segment.split_once_str(b"=") {
                Some((key, value)) => extra_parameters.push((key.into(), Some(value.into()))),
                None => extra_parameters.push((segment.into(), None)),
            }
        }
    }

    Ok(ConnectRequest {
        service,
        repository_path: path,
        virtual_host,
        protocol,
        extra_parameters,
    })
}

fn parse_host_port(host_str: &str) -> Result<(String, Option<u16>), Error> {
    // IPv6 bracket notation: [::1]:port
    if let Some(bracketed) = host_str.strip_prefix('[') {
        if let Some((addr, rest)) = bracketed.split_once(']') {
            let port = if let Some(port_str) = rest.strip_prefix(':') {
                Some(port_str.parse::<u16>().map_err(|_| Error::MalformedMessage)?)
            } else {
                None
            };
            return Ok((addr.to_owned(), port));
        }
        return Err(Error::MalformedMessage);
    }

    // Regular host:port — only split on the last colon to avoid confusing IPv6 without brackets.
    match host_str.rsplit_once(':') {
        Some((host, port_str)) => match port_str.parse::<u16>() {
            Ok(port) => Ok((host.to_owned(), Some(port))),
            Err(_) => Ok((host_str.to_owned(), None)),
        },
        None => Ok((host_str.to_owned(), None)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_upload_pack_simple() {
        let msg = b"git-upload-pack /repo.git\0";
        let req = parse_connect_message(msg).expect("valid message");
        assert_eq!(req.service, Service::UploadPack);
        assert_eq!(req.repository_path, "/repo.git");
        assert_eq!(req.virtual_host, None);
        assert_eq!(req.protocol, Protocol::V1);
        assert!(req.extra_parameters.is_empty());
    }

    #[test]
    fn parse_receive_pack_with_host() {
        let msg = b"git-receive-pack /project.git\0host=example.org\0";
        let req = parse_connect_message(msg).expect("valid message");
        assert_eq!(req.service, Service::ReceivePack);
        assert_eq!(req.repository_path, "/project.git");
        assert_eq!(req.virtual_host, Some(("example.org".to_owned(), None)));
    }

    #[test]
    fn parse_with_host_and_port() {
        let msg = b"git-upload-pack /repo.git\0host=example.org:9418\0";
        let req = parse_connect_message(msg).expect("valid message");
        assert_eq!(req.virtual_host, Some(("example.org".to_owned(), Some(9418))));
    }

    #[test]
    fn parse_with_protocol_v2() {
        let msg = b"git-upload-pack /repo.git\0host=example.org\0\0version=2\0";
        let req = parse_connect_message(msg).expect("valid message");
        assert_eq!(req.protocol, Protocol::V2);
    }

    #[test]
    fn parse_with_extra_parameters() {
        let msg = b"git-upload-pack /repo.git\0host=example.org\0\0version=2\0ci=true\0key=value\0";
        let req = parse_connect_message(msg).expect("valid message");
        assert_eq!(req.protocol, Protocol::V2);
        assert_eq!(
            req.extra_parameters,
            vec![
                (BString::from("ci"), Some(BString::from("true"))),
                (BString::from("key"), Some(BString::from("value"))),
            ]
        );
    }

    #[test]
    fn parse_unknown_service_fails() {
        let msg = b"git-unknown /repo.git\0";
        let err = parse_connect_message(msg).unwrap_err();
        assert!(matches!(err, Error::UnknownService { .. }));
    }

    #[test]
    fn parse_empty_message_fails() {
        let err = parse_connect_message(b"").unwrap_err();
        assert!(matches!(err, Error::MalformedMessage));
    }

    #[test]
    fn parse_ipv6_host_with_brackets() {
        let msg = b"git-upload-pack /repo.git\0host=[::1]:9418\0";
        let req = parse_connect_message(msg).expect("valid message");
        assert_eq!(req.virtual_host, Some(("::1".to_owned(), Some(9418))));
    }

    #[test]
    fn parse_ipv6_host_without_port() {
        let msg = b"git-upload-pack /repo.git\0host=[::1]\0";
        let req = parse_connect_message(msg).expect("valid message");
        assert_eq!(req.virtual_host, Some(("::1".to_owned(), None)));
    }
}
