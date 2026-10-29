use gix_error::Result;
use gix_transport::client::Capabilities;

/// Convert a hexadecimal hash into its corresponding `ObjectId` or _panic_.
fn oid(hex: &str) -> gix_hash::ObjectId {
    gix_hash::ObjectId::from_hex(hex.as_bytes()).expect("40 bytes hex")
}

use gix_protocol::handshake::{Ref, refs};

#[crate::bisync::bisync]
#[cfg_attr(feature = "blocking-client", test)]
#[cfg_attr(all(feature = "async-client", not(feature = "blocking-client")), async_std::test)]
async fn extract_references_from_v2_refs() -> gix_testtools::TestResult {
    let input = &mut Fixture(
        "808e50d724f604f69ab93c6da2919c014667bedb HEAD symref-target:refs/heads/main
808e50d724f604f69ab93c6da2919c014667bedb MISSING_NAMESPACE_TARGET symref-target:(null)
unborn HEAD symref-target:refs/heads/main
unborn refs/heads/symbolic symref-target:refs/heads/target
808e50d724f604f69ab93c6da2919c014667bedb refs/heads/main
7fe1b98b39423b71e14217aa299a03b7c937d656 refs/tags/foo peeled:808e50d724f604f69ab93c6da2919c014667bedb
7fe1b98b39423b71e14217aa299a03b7c937d6ff refs/tags/blaz
978f927e6397113757dfec6332e7d9c7e356ac25 refs/heads/symbolic symref-target:refs/tags/v1.0 peeled:4d979abcde5cea47b079c38850828956c9382a56
"
            .as_bytes(),
    );

    let out = refs::from_v2_refs(input).await?;

    assert_eq!(
        out,
        vec![
            Ref::Symbolic {
                full_ref_name: "HEAD".into(),
                target: "refs/heads/main".into(),
                tag: None,
                object: oid("808e50d724f604f69ab93c6da2919c014667bedb")
            },
            Ref::Direct {
                full_ref_name: "MISSING_NAMESPACE_TARGET".into(),
                object: oid("808e50d724f604f69ab93c6da2919c014667bedb")
            },
            Ref::Unborn {
                full_ref_name: "HEAD".into(),
                target: "refs/heads/main".into(),
            },
            Ref::Unborn {
                full_ref_name: "refs/heads/symbolic".into(),
                target: "refs/heads/target".into(),
            },
            Ref::Direct {
                full_ref_name: "refs/heads/main".into(),
                object: oid("808e50d724f604f69ab93c6da2919c014667bedb")
            },
            Ref::Peeled {
                full_ref_name: "refs/tags/foo".into(),
                tag: oid("7fe1b98b39423b71e14217aa299a03b7c937d656"),
                object: oid("808e50d724f604f69ab93c6da2919c014667bedb")
            },
            Ref::Direct {
                full_ref_name: "refs/tags/blaz".into(),
                object: oid("7fe1b98b39423b71e14217aa299a03b7c937d6ff")
            },
            Ref::Symbolic {
                full_ref_name: "refs/heads/symbolic".into(),
                target: "refs/tags/v1.0".into(),
                tag: Some(oid("978f927e6397113757dfec6332e7d9c7e356ac25")),
                object: oid("4d979abcde5cea47b079c38850828956c9382a56")
            },
        ]
    );
    Ok(())
}

#[crate::bisync::bisync]
#[cfg_attr(feature = "blocking-client", test)]
#[cfg_attr(all(feature = "async-client", not(feature = "blocking-client")), async_std::test)]
async fn extract_references_from_v1_refs() -> gix_testtools::TestResult {
    let input = &mut Fixture(
        "73a6868963993a3328e7d8fe94e5a6ac5078a944 HEAD
21c9b7500cb144b3169a6537961ec2b9e865be81 MISSING_NAMESPACE_TARGET
73a6868963993a3328e7d8fe94e5a6ac5078a944 refs/heads/main
8e472f9ccc7d745927426cbb2d9d077de545aa4e refs/pull/13/head
dce0ea858eef7ff61ad345cc5cdac62203fb3c10 refs/tags/gix-commitgraph-v0.0.0
21c9b7500cb144b3169a6537961ec2b9e865be81 refs/tags/gix-commitgraph-v0.0.0^{}"
            .as_bytes(),
    );
    let (out, shallow) = refs::from_v1_refs_received_as_part_of_handshake_and_capabilities(
        input,
        Capabilities::from_bytes(b"\0symref=HEAD:refs/heads/main symref=MISSING_NAMESPACE_TARGET:(null)")?
            .0
            .iter(),
    )
    .await?;
    assert!(shallow.is_empty());
    assert_eq!(
        out,
        vec![
            Ref::Symbolic {
                full_ref_name: "HEAD".into(),
                target: "refs/heads/main".into(),
                tag: None,
                object: oid("73a6868963993a3328e7d8fe94e5a6ac5078a944")
            },
            Ref::Direct {
                full_ref_name: "MISSING_NAMESPACE_TARGET".into(),
                object: oid("21c9b7500cb144b3169a6537961ec2b9e865be81")
            },
            Ref::Direct {
                full_ref_name: "refs/heads/main".into(),
                object: oid("73a6868963993a3328e7d8fe94e5a6ac5078a944")
            },
            Ref::Direct {
                full_ref_name: "refs/pull/13/head".into(),
                object: oid("8e472f9ccc7d745927426cbb2d9d077de545aa4e")
            },
            Ref::Peeled {
                full_ref_name: "refs/tags/gix-commitgraph-v0.0.0".into(),
                tag: oid("dce0ea858eef7ff61ad345cc5cdac62203fb3c10"),
                object: oid("21c9b7500cb144b3169a6537961ec2b9e865be81")
            },
        ]
    );
    Ok(())
}

#[crate::bisync::bisync]
#[cfg_attr(feature = "blocking-client", test)]
#[cfg_attr(all(feature = "async-client", not(feature = "blocking-client")), async_std::test)]
async fn extract_references_from_v1_refs_with_shallow() -> gix_testtools::TestResult {
    use gix_protocol::fetch::response::ShallowUpdate;
    let input = &mut Fixture(
        "73a6868963993a3328e7d8fe94e5a6ac5078a944 HEAD
21c9b7500cb144b3169a6537961ec2b9e865be81 MISSING_NAMESPACE_TARGET
73a6868963993a3328e7d8fe94e5a6ac5078a944 refs/heads/main
8e472f9ccc7d745927426cbb2d9d077de545aa4e refs/pull/13/head
dce0ea858eef7ff61ad345cc5cdac62203fb3c10 refs/tags/gix-commitgraph-v0.0.0
21c9b7500cb144b3169a6537961ec2b9e865be81 refs/tags/gix-commitgraph-v0.0.0^{}
shallow 21c9b7500cb144b3169a6537961ec2b9e865be81
shallow dce0ea858eef7ff61ad345cc5cdac62203fb3c10"
            .as_bytes(),
    );
    let (out, shallow) = refs::from_v1_refs_received_as_part_of_handshake_and_capabilities(
        input,
        Capabilities::from_bytes(b"\0symref=HEAD:refs/heads/main symref=MISSING_NAMESPACE_TARGET:(null)")?
            .0
            .iter(),
    )
    .await?;

    assert_eq!(
        shallow,
        vec![
            ShallowUpdate::Shallow(oid("21c9b7500cb144b3169a6537961ec2b9e865be81")),
            ShallowUpdate::Shallow(oid("dce0ea858eef7ff61ad345cc5cdac62203fb3c10"))
        ]
    );
    assert_eq!(
        out,
        vec![
            Ref::Symbolic {
                full_ref_name: "HEAD".into(),
                target: "refs/heads/main".into(),
                tag: None,
                object: oid("73a6868963993a3328e7d8fe94e5a6ac5078a944")
            },
            Ref::Direct {
                full_ref_name: "MISSING_NAMESPACE_TARGET".into(),
                object: oid("21c9b7500cb144b3169a6537961ec2b9e865be81")
            },
            Ref::Direct {
                full_ref_name: "refs/heads/main".into(),
                object: oid("73a6868963993a3328e7d8fe94e5a6ac5078a944")
            },
            Ref::Direct {
                full_ref_name: "refs/pull/13/head".into(),
                object: oid("8e472f9ccc7d745927426cbb2d9d077de545aa4e")
            },
            Ref::Peeled {
                full_ref_name: "refs/tags/gix-commitgraph-v0.0.0".into(),
                tag: oid("dce0ea858eef7ff61ad345cc5cdac62203fb3c10"),
                object: oid("21c9b7500cb144b3169a6537961ec2b9e865be81")
            },
        ]
    );
    Ok(())
}

#[cfg(feature = "blocking-client")]
#[test]
fn rejected_credentials_keep_url_passwords_private() -> gix_testtools::TestResult {
    use gix_credentials::{helper::Action, protocol::Outcome};
    use gix_transport::{
        Service,
        client::{
            self, AuthenticationRequired,
            blocking_io::{RequestWriter, SetServiceResponse},
        },
    };

    #[derive(Default)]
    struct RejectCredentials {
        handshakes: usize,
    }

    impl client::TransportWithoutIO for RejectCredentials {
        fn set_identity(&mut self, _identity: client::Account) -> std::result::Result<(), client::Error> {
            Ok(())
        }

        fn to_url(&self) -> std::borrow::Cow<'_, bstr::BStr> {
            std::borrow::Cow::Borrowed("https://user:password-sentinel@example.com/repository".into())
        }

        fn connection_persists_across_multiple_requests(&self) -> bool {
            false
        }

        fn configure(&mut self, _config: &dyn std::any::Any) -> Result {
            Ok(())
        }
    }

    impl client::blocking_io::Transport for RejectCredentials {
        fn handshake<'a>(
            &mut self,
            _service: Service,
            _extra_parameters: &'a [(&'a str, Option<&'a str>)],
        ) -> std::result::Result<SetServiceResponse<'_>, client::Error> {
            self.handshakes += 1;
            Err(client::Error::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                AuthenticationRequired::default(),
            )))
        }

        fn request(
            &mut self,
            _write_mode: client::WriteMode,
            _on_into_read: client::MessageKind,
            _trace: bool,
        ) -> std::result::Result<RequestWriter<'_>, client::Error> {
            unreachable!("rejected credentials prevent any protocol request")
        }
    }

    for service in [Service::UploadPack, Service::ReceivePack] {
        let mut transport = RejectCredentials::default();
        let mut actions = Vec::new();
        let error = gix_protocol::handshake(
            &mut transport,
            service,
            |action| match action {
                Action::Get(context) => {
                    actions.push("get");
                    Ok(Some(Outcome {
                        identity: client::Account {
                            username: "user".into(),
                            password: "password-sentinel".into(),
                            oauth_refresh_token: None,
                        },
                        next: context.into(),
                    }))
                }
                Action::Erase(_) => {
                    actions.push("erase");
                    Ok(None)
                }
                Action::Store(_) => unreachable!("rejected credentials must not be stored"),
            },
            Vec::new(),
            &mut gix_utils::progress::Discard,
        )
        .expect_err("the remote rejects the retry with credentials");
        assert_eq!(transport.handshakes, 2, "authentication is retried once");
        assert_eq!(actions, ["get", "erase"], "rejected credentials are invalidated");
        for diagnostic in [format!("{error}"), format!("{error:?}")] {
            assert!(
                !diagnostic.contains("password-sentinel"),
                "authentication diagnostics must not expose URL passwords: {diagnostic}"
            );
        }
        assert!(
            error.is_unauthenticated(),
            "omitting the URL preserves the authentication classification"
        );
    }
    Ok(())
}

#[cfg(any(feature = "async-client", feature = "blocking-client"))]
struct Fixture<'a>(&'a [u8]);

#[cfg(feature = "blocking-client")]
impl std::io::Read for Fixture<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

#[cfg(feature = "blocking-client")]
impl std::io::BufRead for Fixture<'_> {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.0.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.0.consume(amt);
    }
}

#[cfg(feature = "blocking-client")]
impl gix_transport::client::blocking_io::ReadlineBufRead for Fixture<'_> {
    fn readline(&mut self) -> Option<std::result::Result<Result<gix_packetline::PacketLineRef<'_>>, std::io::Error>> {
        use bstr::{BStr, ByteSlice};
        let bytes: &BStr = self.0.into();
        let mut lines = bytes.lines();
        let res = lines.next()?;
        self.0 = lines.as_bytes();
        Some(Ok(Ok(gix_packetline::PacketLineRef::Data(res))))
    }

    fn readline_str(&mut self, line: &mut String) -> std::io::Result<usize> {
        use bstr::{BStr, ByteSlice};
        let bytes: &BStr = self.0.into();
        let mut lines = bytes.lines();
        let res = match lines.next() {
            None => return Ok(0),
            Some(line) => line,
        };
        self.0 = lines.as_bytes();
        let len = res.len();
        line.push_str(res.to_str().expect("valid UTF8 in fixture"));
        Ok(len)
    }
}

#[cfg(all(feature = "async-client", not(feature = "blocking-client")))]
impl<'a> Fixture<'a> {
    fn project_inner(self: std::pin::Pin<&mut Self>) -> std::pin::Pin<&mut &'a [u8]> {
        #[expect(unsafe_code)]
        unsafe {
            std::pin::Pin::new(&mut self.get_unchecked_mut().0)
        }
    }
}

#[cfg(all(feature = "async-client", not(feature = "blocking-client")))]
impl futures_io::AsyncRead for Fixture<'_> {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        self.project_inner().poll_read(cx, buf)
    }
}

#[cfg(all(feature = "async-client", not(feature = "blocking-client")))]
impl futures_io::AsyncBufRead for Fixture<'_> {
    fn poll_fill_buf(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<&[u8]>> {
        self.project_inner().poll_fill_buf(cx)
    }

    fn consume(self: std::pin::Pin<&mut Self>, amt: usize) {
        self.project_inner().consume(amt);
    }
}

#[cfg(all(feature = "async-client", not(feature = "blocking-client")))]
#[async_trait::async_trait(?Send)]
impl gix_transport::client::async_io::ReadlineBufRead for Fixture<'_> {
    async fn readline(&mut self) -> Option<std::io::Result<Result<gix_packetline::PacketLineRef<'_>>>> {
        use bstr::{BStr, ByteSlice};
        let bytes: &BStr = self.0.into();
        let mut lines = bytes.lines();
        let res = lines.next()?;
        self.0 = lines.as_bytes();
        Some(Ok(Ok(gix_packetline::PacketLineRef::Data(res))))
    }
    async fn readline_str(&mut self, line: &mut String) -> std::io::Result<usize> {
        use bstr::{BStr, ByteSlice};
        let bytes: &BStr = self.0.into();
        let mut lines = bytes.lines();
        let res = match lines.next() {
            None => return Ok(0),
            Some(line) => line,
        };
        self.0 = lines.as_bytes();
        let len = res.len();
        line.push_str(res.to_str().expect("valid UTF8 in fixture"));
        Ok(len)
    }
}
