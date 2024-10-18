use bstr::ByteSlice;
use gix_transport::Protocol;

use crate::fetch;

fn arguments_v1(features: impl IntoIterator<Item = &'static str>) -> fetch::Arguments {
    fetch::Arguments::new(Protocol::V1, features.into_iter().map(|n| (n, None)).collect(), false)
}

fn arguments_v2(features: impl IntoIterator<Item = &'static str>) -> fetch::Arguments {
    fetch::Arguments::new(Protocol::V2, features.into_iter().map(|n| (n, None)).collect(), false)
}

struct Transport<T> {
    inner: T,
    stateful: bool,
}

#[cfg(feature = "blocking-client")]
mod impls {
    use std::borrow::Cow;

    use bstr::BStr;
    use gix_transport::{
        client,
        client::{Error, MessageKind, RequestWriter, SetServiceResponse, WriteMode},
        Protocol, Service,
    };

    use super::Transport;

    impl<T: client::TransportWithoutIO> client::TransportWithoutIO for Transport<T> {
        fn set_identity(&mut self, identity: client::Account) -> Result<(), Error> {
            self.inner.set_identity(identity)
        }

        fn request(
            &mut self,
            write_mode: WriteMode,
            on_into_read: MessageKind,
            trace: bool,
        ) -> Result<RequestWriter<'_>, Error> {
            self.inner.request(write_mode, on_into_read, trace)
        }

        fn to_url(&self) -> Cow<'_, BStr> {
            self.inner.to_url()
        }

        fn supported_protocol_versions(&self) -> &[Protocol] {
            self.inner.supported_protocol_versions()
        }

        fn connection_persists_across_multiple_requests(&self) -> bool {
            self.stateful
        }

        fn configure(
            &mut self,
            config: &dyn std::any::Any,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            self.inner.configure(config)
        }
    }

    impl<T: client::Transport> client::Transport for Transport<T> {
        fn handshake<'a>(
            &mut self,
            service: Service,
            extra_parameters: &'a [(&'a str, Option<&'a str>)],
        ) -> Result<SetServiceResponse<'_>, Error> {
            self.inner.handshake(service, extra_parameters)
        }
    }
}

#[cfg(feature = "async-client")]
mod impls {
    use std::borrow::Cow;

    use async_trait::async_trait;
    use bstr::BStr;
    use gix_transport::{
        client,
        client::{Error, MessageKind, RequestWriter, SetServiceResponse, WriteMode},
        Protocol, Service,
    };

    use super::Transport;
    impl<T: client::TransportWithoutIO + Send> client::TransportWithoutIO for Transport<T> {
        fn set_identity(&mut self, identity: client::Account) -> Result<(), Error> {
            self.inner.set_identity(identity)
        }

        fn request(
            &mut self,
            write_mode: WriteMode,
            on_into_read: MessageKind,
            trace: bool,
        ) -> Result<RequestWriter<'_>, Error> {
            self.inner.request(write_mode, on_into_read, trace)
        }

        fn to_url(&self) -> Cow<'_, BStr> {
            self.inner.to_url()
        }

        fn supported_protocol_versions(&self) -> &[Protocol] {
            self.inner.supported_protocol_versions()
        }

        fn connection_persists_across_multiple_requests(&self) -> bool {
            self.stateful
        }

        fn configure(
            &mut self,
            config: &dyn std::any::Any,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            self.inner.configure(config)
        }
    }

    #[async_trait(?Send)]
    impl<T: client::Transport + Send> client::Transport for Transport<T> {
        async fn handshake<'a>(
            &mut self,
            service: Service,
            extra_parameters: &'a [(&'a str, Option<&'a str>)],
        ) -> Result<SetServiceResponse<'_>, Error> {
            self.inner.handshake(service, extra_parameters).await
        }
    }
}

fn transport(
    out: &mut Vec<u8>,
    stateful: bool,
) -> Transport<gix_transport::client::git::Connection<&'static [u8], &mut Vec<u8>>> {
    Transport {
        inner: gix_transport::client::git::Connection::new(
            &[],
            out,
            Protocol::V1, // does not matter
            b"does/not/matter".as_bstr().to_owned(),
            None::<(&str, _)>,
            gix_transport::client::git::ConnectMode::Process, // avoid header to be sent
            false,
        ),
        stateful,
    }
}

fn id(hex: &str) -> gix_hash::ObjectId {
    gix_hash::ObjectId::from_hex(hex.as_bytes()).expect("expect valid hex id")
}

mod v1 {
    use bstr::ByteSlice;

    use super::{arguments_v1, id, transport};

    #[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
    async fn include_tag() {
        let mut out = Vec::new();
        let mut t = transport(&mut out, true);
        let mut arguments = arguments_v1(["include-tag", "feature-b"].iter().copied());
        assert!(arguments.can_use_include_tag());

        arguments.use_include_tag();
        arguments.want(id("ff333369de1221f9bfbbe03a3a13e9a09bc1ffff"));
        arguments.send(&mut t, true).await.expect("sending to buffer to work");
        assert_eq!(
            out.as_bstr(),
            b"0048want ff333369de1221f9bfbbe03a3a13e9a09bc1ffff feature-b include-tag
00000009done
"
            .as_bstr()
        );
    }

    #[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
    async fn no_include_tag() {
        let mut out = Vec::new();
        let mut t = transport(&mut out, true);
        let mut arguments = arguments_v1(["include-tag", "feature-b"].iter().copied());
        assert!(arguments.can_use_include_tag());

        arguments.want(id("ff333369de1221f9bfbbe03a3a13e9a09bc1ffff"));
        arguments.send(&mut t, true).await.expect("sending to buffer to work");
        assert_eq!(
            out.as_bstr(),
            b"003cwant ff333369de1221f9bfbbe03a3a13e9a09bc1ffff feature-b
00000009done
"
            .as_bstr(),
            "it's possible to not have it enabled, even though it's advertised by the server"
        );
    }

    #[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
    async fn haves_and_wants_for_clone() {
        let mut out = Vec::new();
        let mut t = transport(&mut out, true);
        let mut arguments = arguments_v1(["feature-a", "feature-b"].iter().copied());
        assert!(
            !arguments.can_use_include_tag(),
            "needs to be enabled by features in V1"
        );

        arguments.want(id("7b333369de1221f9bfbbe03a3a13e9a09bc1c907"));
        arguments.want(id("ff333369de1221f9bfbbe03a3a13e9a09bc1ffff"));
        arguments.send(&mut t, true).await.expect("sending to buffer to work");
        assert_eq!(
            out.as_bstr(),
            b"0046want 7b333369de1221f9bfbbe03a3a13e9a09bc1c907 feature-a feature-b
0032want ff333369de1221f9bfbbe03a3a13e9a09bc1ffff
00000009done
"
            .as_bstr()
        );
    }

    #[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
    async fn haves_and_wants_for_fetch_stateless() {
        let mut out = Vec::new();
        let mut t = transport(&mut out, false);
        let mut arguments = arguments_v1(["feature-a", "shallow", "deepen-since", "deepen-not"].iter().copied());

        arguments.deepen(1);
        arguments.shallow(id("7b333369de1221f9bfbbe03a3a13e9a09bc1c9ff"));
        arguments.want(id("7b333369de1221f9bfbbe03a3a13e9a09bc1c907"));
        arguments.deepen_since(12345);
        arguments.deepen_not("refs/heads/main".into());
        arguments.have(id("0000000000000000000000000000000000000000"));
        arguments.send(&mut t, false).await.expect("sending to buffer to work");

        arguments.have(id("1111111111111111111111111111111111111111"));
        arguments.send(&mut t, true).await.expect("sending to buffer to work");
        assert_eq!(
            out.as_bstr(),
            b"005cwant 7b333369de1221f9bfbbe03a3a13e9a09bc1c907 feature-a shallow deepen-since deepen-not
0035shallow 7b333369de1221f9bfbbe03a3a13e9a09bc1c9ff
000ddeepen 1
0017deepen-since 12345
001fdeepen-not refs/heads/main
00000032have 0000000000000000000000000000000000000000
0000005cwant 7b333369de1221f9bfbbe03a3a13e9a09bc1c907 feature-a shallow deepen-since deepen-not
0035shallow 7b333369de1221f9bfbbe03a3a13e9a09bc1c9ff
000ddeepen 1
0017deepen-since 12345
001fdeepen-not refs/heads/main
00000032have 1111111111111111111111111111111111111111
0009done
"
            .as_bstr()
        );
    }

    #[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
    async fn haves_and_wants_for_fetch_stateful() {
        let mut out = Vec::new();
        let mut t = transport(&mut out, true);
        let mut arguments = arguments_v1(["feature-a", "shallow"].iter().copied());

        arguments.deepen(1);
        arguments.want(id("7b333369de1221f9bfbbe03a3a13e9a09bc1c907"));
        arguments.have(id("0000000000000000000000000000000000000000"));
        arguments.send(&mut t, false).await.expect("sending to buffer to work");

        arguments.have(id("1111111111111111111111111111111111111111"));
        arguments.send(&mut t, true).await.expect("sending to buffer to work");
        assert_eq!(
            out.as_bstr(),
            b"0044want 7b333369de1221f9bfbbe03a3a13e9a09bc1c907 feature-a shallow
000ddeepen 1
00000032have 0000000000000000000000000000000000000000
00000032have 1111111111111111111111111111111111111111
0009done
"
            .as_bstr()
        );
    }
}

mod v2 {
    use bstr::ByteSlice;

    use super::{arguments_v2, id, transport};

    #[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
    async fn include_tag() {
        let mut out = Vec::new();
        let mut t = transport(&mut out, true);
        let mut arguments = arguments_v2(["does not matter for us here"].iter().copied());
        assert!(arguments.can_use_include_tag(), "always on in V2");
        arguments.use_include_tag();

        arguments.want(id("ff333369de1221f9bfbbe03a3a13e9a09bc1ffff"));
        arguments.send(&mut t, true).await.expect("sending to buffer to work");
        assert_eq!(
                out.as_bstr(),
                b"0012command=fetch
0001000ethin-pack
000eofs-delta
0010include-tag
0032want ff333369de1221f9bfbbe03a3a13e9a09bc1ffff
0009done
0000"
                    .as_bstr(),
                "we filter features/capabilities without value as these apparently shouldn't be listed (remote dies otherwise)"
            );
    }

    #[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
    async fn haves_and_wants_for_clone_stateful() {
        let mut out = Vec::new();
        let mut t = transport(&mut out, true);
        let mut arguments = arguments_v2(["feature-a", "shallow"].iter().copied());
        assert!(arguments.is_stateless(true), "V2 is stateless…");
        assert!(arguments.is_stateless(false), "…in all cases");

        arguments.add_feature("no-progress");
        arguments.deepen(1);
        arguments.deepen_relative();
        arguments.want(id("7b333369de1221f9bfbbe03a3a13e9a09bc1c907"));
        arguments.want(id("ff333369de1221f9bfbbe03a3a13e9a09bc1ffff"));
        arguments.send(&mut t, true).await.expect("sending to buffer to work");
        assert_eq!(
                out.as_bstr(),
                b"0012command=fetch
0001000ethin-pack
000eofs-delta
0010no-progress
000ddeepen 1
0014deepen-relative
0032want 7b333369de1221f9bfbbe03a3a13e9a09bc1c907
0032want ff333369de1221f9bfbbe03a3a13e9a09bc1ffff
0009done
0000"
                    .as_bstr(),
                "we filter features/capabilities without value as these apparently shouldn't be listed (remote dies otherwise)"
            );
    }

    #[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
    async fn haves_and_wants_for_fetch_stateless_and_stateful() {
        for is_stateful in &[false, true] {
            let mut out = Vec::new();
            let mut t = transport(&mut out, *is_stateful);
            let mut arguments = arguments_v2(Some("shallow"));

            arguments.add_feature("no-progress");
            arguments.deepen(1);
            arguments.deepen_since(12345);
            arguments.shallow(id("7b333369de1221f9bfbbe03a3a13e9a09bc1c9ff"));
            arguments.want(id("7b333369de1221f9bfbbe03a3a13e9a09bc1c907"));
            arguments.deepen_not("refs/heads/main".into());
            arguments.have(id("0000000000000000000000000000000000000000"));
            arguments.send(&mut t, false).await.expect("sending to buffer to work");

            arguments.have(id("1111111111111111111111111111111111111111"));
            arguments.send(&mut t, true).await.expect("sending to buffer to work");
            assert_eq!(
                out.as_bstr(),
                b"0012command=fetch
0001000ethin-pack
000eofs-delta
0010no-progress
000ddeepen 1
0017deepen-since 12345
0035shallow 7b333369de1221f9bfbbe03a3a13e9a09bc1c9ff
0032want 7b333369de1221f9bfbbe03a3a13e9a09bc1c907
001fdeepen-not refs/heads/main
0032have 0000000000000000000000000000000000000000
00000012command=fetch
0001000ethin-pack
000eofs-delta
0010no-progress
000ddeepen 1
0017deepen-since 12345
0035shallow 7b333369de1221f9bfbbe03a3a13e9a09bc1c9ff
0032want 7b333369de1221f9bfbbe03a3a13e9a09bc1c907
001fdeepen-not refs/heads/main
0032have 1111111111111111111111111111111111111111
0009done
0000"
                    .as_bstr(),
                "V2 is stateless by default, so it repeats all but 'haves' in each request"
            );
        }
    }

    #[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
    async fn ref_in_want() {
        let mut out = Vec::new();
        let mut t = transport(&mut out, false);
        let mut arguments = arguments_v2(["ref-in-want"].iter().copied());

        arguments.want_ref(b"refs/heads/main".as_bstr());
        arguments.send(&mut t, true).await.expect("sending to buffer to work");
        assert_eq!(
            out.as_bstr(),
            b"0012command=fetch
0001000ethin-pack
000eofs-delta
001dwant-ref refs/heads/main
0009done
0000"
                .as_bstr()
        );
    }
}
