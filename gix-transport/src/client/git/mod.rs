use bstr::BString;

use crate::Protocol;

/// The way to connect to a process speaking the `git` protocol.
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum ConnectMode {
    /// A git daemon.
    Daemon,
    /// A spawned `git` process to upload a pack to the client.
    Process,
}

/// Connection state shared between blocking and async connections.
pub(crate) struct ConnectionState {
    pub(in crate::client) path: BString,
    pub(in crate::client) virtual_host: Option<(String, Option<u16>)>,
    pub(in crate::client) desired_version: Protocol,
    custom_url: Option<BString>,
    pub(in crate::client) mode: ConnectMode,
}

mod message {
    use bstr::{BString, ByteVec};

    use crate::{Protocol, Service};

    pub fn connect(
        service: Service,
        desired_version: Protocol,
        path: &[u8],
        virtual_host: Option<&(String, Option<u16>)>,
        extra_parameters: &[(&str, Option<&str>)],
    ) -> BString {
        let mut out = bstr::BString::from(service.as_str());
        out.push(b' ');
        let path = gix_url::expand_path::for_shell(path.into());
        out.extend_from_slice(&path);
        out.push(0);
        if let Some((host, port)) = virtual_host {
            out.push_str("host=");
            out.extend_from_slice(host.as_bytes());
            if let Some(port) = port {
                out.push_byte(b':');
                out.push_str(format!("{port}"));
            }
            out.push(0);
        }
        // We only send the version when needed, as otherwise a V2 server who is asked for V1 will respond with 'version 1'
        // as extra lines in the reply, which we don't want to handle. Especially since an old server will not respond with that
        // line (is what I assume, at least), so it's an optional part in the response to understand and handle. There is no value
        // in that, so let's help V2 servers to respond in a way that assumes V1.
        let extra_params_need_null_prefix = if desired_version != Protocol::V1 {
            out.push(0);
            out.push_str(format!("version={}", desired_version as usize));
            out.push(0);
            false
        } else {
            true
        };

        if !extra_parameters.is_empty() {
            if extra_params_need_null_prefix {
                out.push(0);
            }
            for (key, value) in extra_parameters {
                match value {
                    Some(value) => out.push_str(format!("{key}={value}")),
                    None => out.push_str(key),
                }
                out.push(0);
            }
        }
        out
    }
    #[cfg(test)]
    mod tests {
        use crate::{Protocol, Service, client::git};

        #[test]
        fn version_1_without_host_and_version() {
            assert_eq!(
                git::message::connect(Service::UploadPack, Protocol::V1, b"hello/world", None, &[]),
                "git-upload-pack hello/world\0"
            );
        }
        #[test]
        fn version_2_without_host_and_version() {
            assert_eq!(
                git::message::connect(Service::UploadPack, Protocol::V2, br"hello\world", None, &[]),
                "git-upload-pack hello\\world\0\0version=2\0"
            );
        }
        #[test]
        fn version_2_without_host_and_version_and_extra_parameters() {
            assert_eq!(
                git::message::connect(
                    Service::UploadPack,
                    Protocol::V2,
                    b"/path/project.git",
                    None,
                    &[("key", Some("value")), ("value-only", None)]
                ),
                "git-upload-pack /path/project.git\0\0version=2\0key=value\0value-only\0"
            );
        }
        #[test]
        fn with_host_without_port() {
            assert_eq!(
                git::message::connect(
                    Service::UploadPack,
                    Protocol::V1,
                    br"hello\world",
                    Some(&("host".into(), None)),
                    &[]
                ),
                "git-upload-pack hello\\world\0host=host\0"
            );
        }
        #[test]
        fn with_host_without_port_and_extra_parameters() {
            assert_eq!(
                git::message::connect(
                    Service::UploadPack,
                    Protocol::V1,
                    br"hello\world",
                    Some(&("host".into(), None)),
                    &[("key", Some("value")), ("value-only", None)]
                ),
                "git-upload-pack hello\\world\0host=host\0\0key=value\0value-only\0"
            );
        }
        #[test]
        fn with_host_with_port() {
            assert_eq!(
                git::message::connect(
                    Service::UploadPack,
                    Protocol::V1,
                    br"hello\world",
                    Some(&("host".into(), Some(404))),
                    &[]
                ),
                "git-upload-pack hello\\world\0host=host:404\0"
            );
        }

        #[test]
        fn with_strange_host_and_port() {
            assert_eq!(
                git::message::connect(
                    Service::UploadPack,
                    Protocol::V1,
                    b"--upload-pack=attack",
                    Some(&("--proxy=other-attack".into(), Some(404))),
                    &[]
                ),
                "git-upload-pack --upload-pack=attack\0host=--proxy=other-attack:404\0",
                "we explicitly allow possible `-arg` arguments to be passed to the git daemon - the remote must protect against exploitation, we don't want to prevent legitimate cases"
            );
        }
    }
}

///
#[cfg(feature = "async-client")]
pub mod async_io;

///
#[cfg(feature = "blocking-client")]
pub mod blocking_io;
