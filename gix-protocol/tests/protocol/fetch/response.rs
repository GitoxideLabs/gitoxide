use crate::fetch::Cursor;
#[cfg(all(feature = "async-client", not(feature = "blocking-client")))]
use gix_packetline::async_io::StreamingPeekableIter;
#[cfg(all(feature = "blocking-client", not(feature = "async-client")))]
use gix_packetline::blocking_io::StreamingPeekableIter;

fn mock_reader(path: &str) -> StreamingPeekableIter<Cursor> {
    use crate::fixture_bytes;
    let buf = fixture_bytes(path);
    StreamingPeekableIter::new(Cursor::new(buf), &[gix_packetline::PacketLineRef::Flush], false)
}

fn id(hex: &str) -> gix_hash::ObjectId {
    gix_hash::ObjectId::from_hex(hex.as_bytes()).expect("expect valid hex id")
}

mod v1 {
    mod from_line_reader {
        #[cfg(feature = "blocking-client")]
        use std::io::Read;

        #[cfg(feature = "async-client")]
        use futures_lite::io::AsyncReadExt;
        use gix_protocol::fetch::{
            self,
            response::{Acknowledgement, ShallowUpdate},
        };
        use gix_transport::Protocol;

        use crate::fetch::response::{id, mock_reader};

        #[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
        async fn clone() -> crate::Result {
            let mut provider = mock_reader("v1/clone-only.response");
            let mut reader = provider.as_read_without_sidebands();
            let r = fetch::Response::from_line_reader(Protocol::V1, &mut reader, true, false).await?;
            assert_eq!(r.acknowledgements(), &[Acknowledgement::Nak]);
            assert!(r.has_pack());
            let mut buf = Vec::new();
            let bytes_read = reader.read_to_end(&mut buf).await?;
            assert_eq!(bytes_read, 1090, "should be able to read the whole pack");
            Ok(())
        }

        #[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
        async fn shallow_clone() -> crate::Result {
            let mut provider = mock_reader("v1/clone-deepen-1.response");
            let mut reader = provider.as_read_without_sidebands();
            let r = fetch::Response::from_line_reader(Protocol::V1, &mut reader, true, false).await?;
            assert_eq!(
                r.shallow_updates(),
                &[ShallowUpdate::Shallow(id("808e50d724f604f69ab93c6da2919c014667bedb"))]
            );
            assert_eq!(r.acknowledgements(), &[Acknowledgement::Nak]);
            assert!(r.has_pack());
            let mut buf = Vec::new();
            let bytes_read = reader.read_to_end(&mut buf).await?;
            assert_eq!(bytes_read, 1989, "should be able to read the whole pack");
            Ok(())
        }

        #[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
        async fn empty_shallow_clone_due_to_depth_being_too_high() -> crate::Result {
            let mut provider = mock_reader("v1/clone-deepen-5.response");
            let mut reader = provider.as_read_without_sidebands();
            let r = fetch::Response::from_line_reader(Protocol::V1, &mut reader, true, false).await?;
            assert!(r.shallow_updates().is_empty());
            assert_eq!(r.acknowledgements(), &[Acknowledgement::Nak]);
            assert!(r.has_pack());
            let mut buf = Vec::new();
            let bytes_read = reader.read_to_end(&mut buf).await?;
            assert_eq!(bytes_read, 1988, "should be able to read the whole pack");
            Ok(())
        }

        #[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
        async fn unshallow_fetch() -> crate::Result {
            let mut provider = mock_reader("v1/fetch-unshallow.response");
            let mut reader = provider.as_read_without_sidebands();
            let r = fetch::Response::from_line_reader(Protocol::V1, &mut reader, true, true).await?;
            assert_eq!(
                r.acknowledgements(),
                &[
                    Acknowledgement::Common(id("f99771fe6a1b535783af3163eba95a927aae21d5")),
                    Acknowledgement::Common(id("2d9d136fb0765f2e24c44a0f91984318d580d03b")),
                    Acknowledgement::Common(id("dfd0954dabef3b64f458321ef15571cc1a46d552")),
                ]
            );
            assert_eq!(
                r.shallow_updates(),
                &[
                    ShallowUpdate::Unshallow(id("2d9d136fb0765f2e24c44a0f91984318d580d03b")),
                    ShallowUpdate::Unshallow(id("dfd0954dabef3b64f458321ef15571cc1a46d552"))
                ]
            );
            assert!(r.has_pack());
            let mut pack = Vec::new();
            reader.read_to_end(&mut pack).await?;
            assert_eq!(
                pack.len(),
                2662,
                "should be able to read the whole pack (and progress info)"
            );
            Ok(())
        }

        #[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
        async fn fetch_acks_without_pack() -> crate::Result {
            let mut provider = mock_reader("v1/fetch-no-pack.response");
            let r =
                fetch::Response::from_line_reader(Protocol::V1, &mut provider.as_read_without_sidebands(), true, true)
                    .await?;
            assert_eq!(
                r.acknowledgements(),
                &[
                    Acknowledgement::Common(id("47ee0b7fe4f3a7d776c78794873e6467e1c47e59")),
                    Acknowledgement::Common(id("3f02c0ad360d96e8dbba92f97b42ebbaa4319db1")),
                    Acknowledgement::Nak,
                ]
            );
            Ok(())
        }

        #[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
        async fn fetch_acks_and_pack() -> crate::Result {
            let mut provider = mock_reader("v1/fetch.response");
            let mut reader = provider.as_read_without_sidebands();
            let r = fetch::Response::from_line_reader(Protocol::V1, &mut reader, true, true).await?;
            assert_eq!(
                r.acknowledgements(),
                &[
                    Acknowledgement::Common(id("6504930888c9c5337e7e065c964f87b60d16a7d7")),
                    Acknowledgement::Common(id("fe17165c392110d1305674c06e4aec35728bfab7")),
                    Acknowledgement::Common(id("f22743895a3024bb0c958335981439f1fa747d57")),
                    Acknowledgement::Ready,
                    Acknowledgement::Nak,
                ]
            );
            assert!(r.has_pack());
            let mut buf = Vec::new();
            let bytes_read = reader.read_to_end(&mut buf).await?;
            assert_eq!(bytes_read, 9703, "should be able to read the whole pack");
            Ok(())
        }
    }

    mod arguments {
        use bstr::ByteSlice;
        use gix_protocol::{fetch, Command};
        use gix_transport::{client::Capabilities, Protocol};

        use crate::fetch::{response::id, transport};

        #[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
        async fn all() -> crate::Result {
            let (caps, _) = Capabilities::from_bytes(&b"7814e8a05a59c0cf5fb186661d1551c75d1299b5 HEAD\0multi_ack thin-pack filter side-band side-band-64k ofs-delta shallow deepen-since deepen-not deepen-relative no-progress include-tag multi_ack_detailed symref=HEAD:refs/heads/master object-format=sha1 agent=git/2.28.0"[..])?;
            let mut args = fetch::Arguments::new(
                Protocol::V1,
                Command::Fetch.default_features(Protocol::V1, &caps),
                false,
            );
            assert!(
                args.is_stateless(true /* transport is stateless */),
                "V1 is stateless if the transport is connection oriented"
            );
            assert!(
                !args.is_stateless(false /* transport is stateless */),
                "otherwise V1 is stateful"
            );
            assert!(args.can_use_shallow());
            assert!(args.can_use_deepen());
            assert!(args.can_use_deepen_not());
            assert!(args.can_use_deepen_relative());
            assert!(args.can_use_deepen_since());
            assert!(args.can_use_filter());
            assert!(args.can_use_include_tag());
            assert!(
                !args.can_use_ref_in_want(),
                "V2 only feature, and we initialize capabilities with V1 for convenience"
            );
            assert!(args.is_empty());

            args.shallow(id("97c5a932b3940a09683e924ef6a92b31a6f7c6de"));
            args.deepen(1);
            args.deepen_relative();
            args.deepen_since(123456);
            args.deepen_not("tag".into());
            args.want(id("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
            args.have(id("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"));

            let mut out = Vec::new();
            let mut transport = transport(
                &mut out,
                "v1/clone.response",
                Protocol::V2,
                gix_transport::client::git::ConnectMode::Daemon,
            );

            let _response = args.send(&mut transport, true).await?;
            drop(_response);
            assert_eq!(out.as_slice().as_bstr(), "009ewant aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa thin-pack side-band-64k ofs-delta shallow deepen-since deepen-not deepen-relative multi_ack_detailed filter\n000ddeepen 1\n0014deepen-relative\n0018deepen-since 123456\n0013deepen-not tag\n0035shallow 97c5a932b3940a09683e924ef6a92b31a6f7c6de\n00000032have bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n0009done\n");
            Ok(())
        }
    }
}
mod v2 {
    mod from_line_reader {
        #[cfg(feature = "blocking-client")]
        use std::io::Read;

        #[cfg(feature = "async-client")]
        use futures_lite::io::AsyncReadExt;
        use gix_protocol::fetch::{
            self,
            response::{Acknowledgement, ShallowUpdate},
        };
        #[cfg(feature = "async-client")]
        use gix_transport::client::async_io::HandleProgress;
        #[cfg(feature = "blocking-client")]
        use gix_transport::client::blocking_io::HandleProgress;
        use gix_transport::Protocol;

        use crate::fetch::response::{id, mock_reader};

        #[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
        async fn clone() -> crate::Result {
            for keepalive in [false, true] {
                let fixture = format!(
                    "v2/clone-only{}.response",
                    if keepalive {
                        "-with-keepalive"
                    } else {
                        Default::default()
                    }
                );
                let mut provider = mock_reader(&fixture);
                let mut reader = provider.as_read_without_sidebands();
                let r = fetch::Response::from_line_reader(Protocol::V2, &mut reader, true, true).await?;
                assert!(r.acknowledgements().is_empty(), "it should go straight to the packfile");
                assert!(r.has_pack());
                reader.set_progress_handler(Some(Box::new(|_is_err, _text| std::ops::ControlFlow::Continue(()))));
                let mut buf = Vec::new();
                let bytes_read = reader.read_to_end(&mut buf).await?;
                assert_eq!(bytes_read, 876, "should be able to read the whole pack");
            }
            Ok(())
        }

        #[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
        async fn shallow_clone() -> crate::Result {
            let mut provider = mock_reader("v2/clone-deepen-1.response");
            let mut reader = provider.as_read_without_sidebands();
            let r = fetch::Response::from_line_reader(Protocol::V2, &mut reader, true, true).await?;
            assert!(r.acknowledgements().is_empty(), "it should go straight to the packfile");
            assert_eq!(
                r.shallow_updates(),
                &[ShallowUpdate::Shallow(id("808e50d724f604f69ab93c6da2919c014667bedb"))]
            );
            assert!(r.has_pack());
            let mut pack = Vec::new();
            reader.read_to_end(&mut pack).await?;
            assert_eq!(pack.len(), 1991, "should be able to read the whole pack");
            Ok(())
        }

        #[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
        async fn unshallow_fetch() -> crate::Result {
            let mut provider = mock_reader("v2/fetch-unshallow.response");
            let mut reader = provider.as_read_without_sidebands();
            let r = fetch::Response::from_line_reader(Protocol::V2, &mut reader, true, true).await?;
            assert_eq!(
                r.acknowledgements(),
                &[
                    Acknowledgement::Common(id("f99771fe6a1b535783af3163eba95a927aae21d5")),
                    Acknowledgement::Common(id("2d9d136fb0765f2e24c44a0f91984318d580d03b")),
                    Acknowledgement::Common(id("dfd0954dabef3b64f458321ef15571cc1a46d552")),
                    Acknowledgement::Ready,
                ]
            );
            assert_eq!(
                r.shallow_updates(),
                &[
                    ShallowUpdate::Unshallow(id("2d9d136fb0765f2e24c44a0f91984318d580d03b")),
                    ShallowUpdate::Unshallow(id("dfd0954dabef3b64f458321ef15571cc1a46d552"))
                ]
            );
            assert!(r.has_pack());
            let mut pack = Vec::new();
            reader.read_to_end(&mut pack).await?;
            assert_eq!(
                pack.len(),
                2664,
                "should be able to read the whole pack (and progress info)"
            );
            Ok(())
        }

        #[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
        async fn empty_shallow_clone() -> crate::Result {
            let mut provider = mock_reader("v2/clone-deepen-5.response");
            let mut reader = provider.as_read_without_sidebands();
            let r = fetch::Response::from_line_reader(Protocol::V2, &mut reader, true, true).await?;
            assert!(r.acknowledgements().is_empty(), "it should go straight to the packfile");
            assert!(r.shallow_updates().is_empty(), "it should go straight to the packfile");
            assert!(r.has_pack());
            let mut pack = Vec::new();
            let bytes_read = reader.read_to_end(&mut pack).await?;
            assert_eq!(bytes_read, 1989, "should be able to read the whole pack");
            Ok(())
        }

        #[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
        async fn clone_with_sidebands() -> crate::Result {
            let mut provider = mock_reader("v2/clone-only-2.response");
            let mut reader = provider.as_read_without_sidebands();
            let r = fetch::Response::from_line_reader(Protocol::V2, &mut reader, true, true).await?;
            assert!(r.acknowledgements().is_empty(), "it should go straight to the packfile");
            assert!(r.has_pack());

            let mut buf = Vec::new();
            reader.set_progress_handler(Some(Box::new(|is_err: bool, _data: &[u8]| {
                assert!(!is_err, "fixture does not have an error");
                std::ops::ControlFlow::Continue(())
            }) as HandleProgress));
            let bytes_read = reader.read_to_end(&mut buf).await?;
            assert_eq!(bytes_read, 1643, "should be able to read the whole pack");
            assert_eq!(&buf[..4], b"PACK");
            assert_eq!(
                gix_hash::ObjectId::from_bytes_or_panic(&buf[buf.len() - gix_hash::Kind::Sha1.len_in_bytes()..])
                    .to_string(),
                "f34c9be7e0c3ef2c3ed7c62cc7791dbf6dc5ec9a"
            );
            Ok(())
        }

        #[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
        async fn fetch_acks_without_pack() -> crate::Result {
            let mut provider = mock_reader("v2/fetch-no-pack.response");
            let r =
                fetch::Response::from_line_reader(Protocol::V2, &mut provider.as_read_without_sidebands(), true, true)
                    .await?;
            assert_eq!(r.acknowledgements(), &[Acknowledgement::Nak]);
            Ok(())
        }

        #[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
        async fn fetch_with_err_response() {
            let mut provider = mock_reader("v2/fetch-err-line.response");
            provider.fail_on_err_lines(true);
            let mut sidebands = provider.as_read_without_sidebands();
            match fetch::Response::from_line_reader(Protocol::V2, &mut sidebands, true, true).await {
                Ok(_) => panic!("need error response"),
                Err(err) => match err {
                    fetch::response::Error::UploadPack(err) => {
                        assert_eq!(err.message, "segmentation fault\n");
                    }
                    err => panic!("we expect upload pack errors, got {err:#?}"),
                },
            }
        }

        #[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
        async fn fetch_acks_and_pack() -> crate::Result {
            let mut provider = mock_reader("v2/fetch.response");
            let mut reader = provider.as_read_without_sidebands();
            let r = fetch::Response::from_line_reader(Protocol::V2, &mut reader, true, true).await?;
            assert_eq!(
                r.acknowledgements(),
                &[
                    Acknowledgement::Common(id("190c3f6b2319c1f4ec854215533caf8623f8f870")),
                    Acknowledgement::Common(id("97c5a932b3940a09683e924ef6a92b31a6f7c6de")),
                    Acknowledgement::Ready,
                ]
            );
            assert!(r.has_pack());
            let mut buf = Vec::new();
            reader.set_progress_handler(Some(Box::new(|a: bool, b: &[u8]| {
                gix_protocol::RemoteProgress::translate_to_progress(a, b, &mut gix_features::progress::Discard);
                std::ops::ControlFlow::Continue(())
            }) as HandleProgress));
            let bytes_read = reader.read_to_end(&mut buf).await?;
            assert_eq!(bytes_read, 5360, "should be able to read the whole pack");
            Ok(())
        }
    }

    mod arguments {
        use bstr::ByteSlice;
        use gix_protocol::{fetch, Command};
        use gix_transport::{client::Capabilities, Protocol};

        use crate::fetch::{response::id, transport};

        #[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
        async fn all() -> crate::Result {
            let (caps, _) = Capabilities::from_bytes(&b"7814e8a05a59c0cf5fb186661d1551c75d1299b5 HEAD\0multi_ack thin-pack filter side-band side-band-64k ofs-delta shallow deepen-since deepen-not deepen-relative no-progress include-tag multi_ack_detailed symref=HEAD:refs/heads/master object-format=sha1 agent=git/2.28.0"[..])?;
            let mut args = fetch::Arguments::new(
                Protocol::V2,
                Command::Fetch.default_features(Protocol::V1, &caps),
                false,
            );
            assert!(args.can_use_shallow());
            assert!(args.can_use_deepen());
            assert!(args.can_use_deepen_not());
            assert!(args.can_use_deepen_relative());
            assert!(args.can_use_deepen_since());
            assert!(args.can_use_filter());
            assert!(args.can_use_include_tag());
            assert!(
                !args.can_use_ref_in_want(),
                "V2 only feature, and we initialize capabilities with V1 for convenience"
            );
            assert!(args.is_empty());

            args.shallow(id("97c5a932b3940a09683e924ef6a92b31a6f7c6de"));
            args.deepen(1);
            args.deepen_relative();
            args.deepen_since(123456);
            args.deepen_not("tag".into());
            args.want(id("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
            args.have(id("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"));

            let mut out = Vec::new();
            let mut transport = transport(
                &mut out,
                "v1/clone.response",
                Protocol::V2,
                gix_transport::client::git::ConnectMode::Daemon,
            );

            let _response = args.send(&mut transport, true).await?;
            drop(_response);
            assert_eq!(out.as_slice().as_bstr(), "0012command=fetch\n0001000ethin-pack\n000eofs-delta\n0035shallow 97c5a932b3940a09683e924ef6a92b31a6f7c6de\n000ddeepen 1\n0014deepen-relative\n0018deepen-since 123456\n0013deepen-not tag\n0032want aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
0032have bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n0009done\n0000");
            Ok(())
        }
    }
}
