use bstr::{BString, ByteSlice};

use crate::{Protocol, Service};

///
#[cfg(feature = "blocking-server")]
pub mod blocking_io;

/// The request parsed from a client's initial connect message.
///
/// Parsed from the `git-proto-request` format described in the
/// [git transport documentation](https://git-scm.com/docs/pack-protocol#_git_transport).
#[derive(Debug, Clone)]
pub struct ClientRequest {
    /// The requested service, e.g. `UploadPack` or `ReceivePack`.
    pub service: Service,
    /// The repository path, e.g. `/repo.git`.
    pub repository_path: BString,
    /// The virtual host and optional port from `host=<host>[:<port>]`.
    pub virtual_host: Option<(String, Option<u16>)>,
    /// The protocol version requested, defaulting to V1 if unspecified.
    pub desired_protocol: Protocol,
    /// Additional key-value parameters beyond `version=`.
    pub extra_parameters: Vec<(BString, Option<BString>)>,
}

/// Errors from parsing a client connect message
#[derive(thiserror::Error, Debug)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Unknown service: {service}")]
    UnknownService { service: BString },
    #[error("Malformed message - unable to parse message")]
    MalformedMessage,
}

/// Parse a git daemon connect message into a [`ClientRequest`].
pub fn parse_connect_message(bytes: &[u8]) -> Result<ClientRequest, Error> {
    let (service_bytes, rest) = bytes.split_once_str(b" ").ok_or(Error::MalformedMessage)?;

    let service = match service_bytes {
        b"git-upload-pack" => Service::UploadPack,
        b"git-receive-pack" => Service::ReceivePack,
        _ => {
            return Err(Error::UnknownService {
                service: service_bytes.into(),
            })
        }
    };

    let mut segments = rest.split_str(b"\0");
    let path: BString = segments.next().ok_or(Error::MalformedMessage)?.into();

    let mut virtual_host = None;
    let mut desired_protocol = Protocol::V1;
    let mut extra_parameters = Vec::new();

    for segment in segments {
        if segment.is_empty() {
            continue;
        }

        if let Some(host_value) = segment.strip_prefix(b"host=") {
            let host_str = std::str::from_utf8(host_value).map_err(|_| Error::MalformedMessage)?;
            virtual_host = Some(match host_str.rsplit_once(':') {
                Some((host, port)) => {
                    let port = port.parse::<u16>().map_err(|_| Error::MalformedMessage)?;
                    (host.to_owned(), Some(port))
                }
                None => (host_str.to_owned(), None),
            });
        } else if let Some(version_value) = segment.strip_prefix(b"version=") {
            let version_str = std::str::from_utf8(version_value).map_err(|_| Error::MalformedMessage)?;
            desired_protocol = match version_str {
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

    Ok(ClientRequest {
        service,
        repository_path: path,
        virtual_host,
        desired_protocol,
        extra_parameters,
    })
}
