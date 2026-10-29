use crate::TestResult;
use bstr::ByteSlice;
use gix_protocol::handshake;
use gix_transport::Protocol;
use gix_utils::progress;

use crate::fetch::{_impl::FetchConnection, CloneDelegate, LsRemoteDelegate, helper_unused, oid, transport};

#[crate::bisync::bisync]
#[cfg_attr(feature = "blocking-client", test)]
#[cfg_attr(all(feature = "async-client", not(feature = "blocking-client")), async_std::test)]
async fn fetch_receives_pack_after_ready_with_no_done() -> TestResult {
    use gix_error::{Result, ResultExt};
    use gix_protocol::fetch::{self, negotiate};

    struct Negotiation;
    impl fetch::Negotiate for Negotiation {
        fn mark_complete_and_common_ref(&mut self) -> Result<negotiate::Action> {
            Ok(negotiate::Action::MustNegotiate {
                remote_ref_target_known: vec![false],
            })
        }

        fn add_wants(&mut self, arguments: &mut fetch::Arguments, _known: &[bool]) -> bool {
            arguments.want(oid("6504930888c9c5337e7e065c964f87b60d16a7d7"));
            true
        }

        fn one_round(
            &mut self,
            _state: &mut negotiate::one_round::State,
            arguments: &mut fetch::Arguments,
            previous_response: Option<&fetch::Response>,
        ) -> Result<(negotiate::Round, bool)> {
            let is_done = previous_response.is_some();
            if !is_done {
                arguments.have(oid("f22743895a3024bb0c958335981439f1fa747d57"));
            }
            Ok((
                negotiate::Round {
                    haves_sent: usize::from(!is_done),
                    in_vain: 0,
                    haves_to_send: 16,
                    previous_response_had_at_least_one_in_common: is_done,
                },
                is_done,
            ))
        }
    }

    let temp = gix_testtools::tempfile::tempdir()?;
    for no_done in [false, true] {
        let capabilities = format!(
            "{} HEAD\0multi_ack_detailed side-band-64k{}",
            "6504930888c9c5337e7e065c964f87b60d16a7d7",
            if no_done { " no-done" } else { "" }
        );
        let mut handshake = gix_protocol::Handshake {
            server_protocol_version: Protocol::V1,
            capabilities: gix_transport::client::Capabilities::from_bytes(capabilities.as_bytes())?.0,
            refs: None,
            v1_shallow_updates: None,
        };
        // Git's response contains ready, NAK, a final ACK, and the sideband pack.
        let mut transport = transport(
            Vec::new(),
            "v1/fetch.response",
            Protocol::V1,
            gix_transport::client::git::ConnectMode::Process,
        );
        let mut pack = Vec::new();
        let outcome = gix_protocol::fetch(
            &mut Negotiation,
            |reader, _, _| {
                reader.read_to_end(&mut pack).or_error()?;
                Ok(true)
            },
            progress::Discard,
            &std::sync::atomic::AtomicBool::new(false),
            fetch::Context {
                handshake: &mut handshake,
                transport: &mut transport,
                user_agent: ("agent", Some("test".into())),
                trace_packetlines: false,
            },
            fetch::Options {
                shallow_file: temp.path().join("shallow"),
                shallow: &fetch::Shallow::NoChange,
                tags: fetch::Tags::None,
                reject_shallow_remote: false,
            },
        )
        .await?
        .expect("negotiation receives a pack");
        assert!(pack.starts_with(b"PACK"), "the final ACK is consumed before the pack");
        assert_eq!(
            outcome.negotiate.rounds.len(),
            if no_done { 1 } else { 2 },
            "only no-done allows receiving a pack before the client is done"
        );
        assert_eq!(
            transport.into_inner().1.contains_str("done\n"),
            !no_done,
            "without no-done, ready still requires sending done"
        );
    }
    Ok(())
}

#[crate::bisync::bisync]
#[cfg_attr(feature = "blocking-client", test)]
#[cfg_attr(all(feature = "async-client", not(feature = "blocking-client")), async_std::test)]
async fn clone() -> TestResult {
    for with_keepalive in [false, true] {
        let out = Vec::new();
        let mut dlg = CloneDelegate::default();
        let fixture = format!(
            "v1/clone{}.response",
            if with_keepalive {
                "-with-keepalive"
            } else {
                Default::default()
            }
        );
        crate::fetch(
            transport(
                out,
                &fixture,
                Protocol::V1,
                gix_transport::client::git::ConnectMode::Daemon,
            ),
            &mut dlg,
            helper_unused,
            progress::Discard,
            FetchConnection::TerminateOnSuccessfulCompletion,
            "agent",
            false,
        )
        .await?;
        assert_eq!(dlg.pack_bytes, 876, "{fixture}: It be able to read pack bytes");
    }
    Ok(())
}

#[crate::bisync::bisync]
#[cfg_attr(feature = "blocking-client", test)]
#[cfg_attr(all(feature = "async-client", not(feature = "blocking-client")), async_std::test)]
async fn clone_empty_with_capabilities() -> TestResult {
    let out = Vec::new();
    let mut dlg = CloneDelegate::default();
    crate::fetch(
        transport(
            out,
            "v1/clone-empty-with-capabilities.response",
            Protocol::V1,
            gix_transport::client::git::ConnectMode::Daemon,
        ),
        &mut dlg,
        helper_unused,
        progress::Discard,
        FetchConnection::TerminateOnSuccessfulCompletion,
        "agent",
        false,
    )
    .await?;
    assert_eq!(dlg.pack_bytes, 0, "there is no pack");
    Ok(())
}

#[crate::bisync::bisync]
#[cfg_attr(feature = "blocking-client", test)]
#[cfg_attr(all(feature = "async-client", not(feature = "blocking-client")), async_std::test)]
async fn ls_remote() -> TestResult {
    let out = Vec::new();
    let mut delegate = LsRemoteDelegate::default();
    let mut transport = transport(
        out,
        "v1/clone.response",
        Protocol::V1,
        gix_transport::client::git::ConnectMode::Daemon,
    );
    crate::fetch(
        &mut transport,
        &mut delegate,
        helper_unused,
        progress::Discard,
        FetchConnection::AllowReuse,
        "agent",
        false,
    )
    .await?;

    assert_eq!(
        delegate.refs,
        vec![
            handshake::Ref::Symbolic {
                full_ref_name: "HEAD".into(),
                object: oid("808e50d724f604f69ab93c6da2919c014667bedb"),
                tag: None,
                target: "refs/heads/master".into()
            },
            handshake::Ref::Direct {
                full_ref_name: "refs/heads/master".into(),
                object: oid("808e50d724f604f69ab93c6da2919c014667bedb")
            }
        ]
    );
    assert_eq!(
        transport.into_inner().1.as_bstr(),
        b"003agit-upload-pack does/not/matter\x00\x00value-only\x00key=value\x000000".as_bstr(),
        "we don't have to send anything in V1, except for the final flush byte to indicate we are done"
    );
    Ok(())
}

#[crate::bisync::bisync]
#[cfg_attr(feature = "blocking-client", test)]
#[cfg_attr(all(feature = "async-client", not(feature = "blocking-client")), async_std::test)]
async fn ls_remote_handshake_failure_due_to_downgrade() -> TestResult {
    let out = Vec::new();
    let delegate = LsRemoteDelegate::default();

    crate::fetch(
        transport(
            out,
            "v1/clone.response",
            Protocol::V2,
            gix_transport::client::git::ConnectMode::Process,
        ),
        delegate,
        helper_unused,
        progress::Discard,
        FetchConnection::AllowReuse,
        "agent",
        false,
    )
    .await?;
    Ok(())
}

#[cfg(feature = "blocking-client")]
#[test]
fn raw_fetch_sideband_preserves_bytes_and_callback_errors() -> TestResult {
    use gix_error::{Result, ResultExt};
    use gix_protocol::fetch::{self, negotiate};
    struct Negotiation;
    impl fetch::Negotiate for Negotiation {
        fn mark_complete_and_common_ref(&mut self) -> Result<negotiate::Action> {
            Ok(negotiate::Action::MustNegotiate {
                remote_ref_target_known: vec![false],
            })
        }
        fn add_wants(&mut self, arguments: &mut fetch::Arguments, _: &[bool]) -> bool {
            arguments.want(oid("6504930888c9c5337e7e065c964f87b60d16a7d7"));
            true
        }
        fn one_round(
            &mut self,
            _: &mut negotiate::one_round::State,
            _: &mut fetch::Arguments,
            _: Option<&fetch::Response>,
        ) -> Result<(negotiate::Round, bool)> {
            Ok((
                negotiate::Round {
                    haves_sent: 0,
                    in_vain: 0,
                    haves_to_send: 16,
                    previous_response_had_at_least_one_in_common: false,
                },
                true,
            ))
        }
    }
    let custom = b"notice: \xff\0\nCounting objects: 50% (1/2)\rsecond line\n";
    let mut response = crate::fixture_bytes("v1/fetch.response");
    let mut offset = 0;
    while offset < response.len() {
        let length = usize::from_str_radix(std::str::from_utf8(&response[offset..offset + 4])?, 16)?;
        if response.get(offset + 4) == Some(&1) {
            let mut packet = format!("{:04x}", custom.len() + 5).into_bytes();
            packet.push(2);
            packet.extend_from_slice(custom);
            response.splice(offset..offset, packet);
            break;
        }
        offset += length.max(4);
    }
    assert!(offset < response.len(), "fixture has a sideband pack");
    let directory = gix_testtools::tempfile::tempdir()?;
    for fail in [false, true] {
        let capabilities = "\0multi_ack_detailed side-band-64k no-done";
        let mut handshake = gix_protocol::Handshake {
            server_protocol_version: Protocol::V1,
            capabilities: gix_transport::client::Capabilities::from_bytes(capabilities.as_bytes())?.0,
            refs: None,
            v1_shallow_updates: None,
        };
        let mut transport = gix_transport::client::git::blocking_io::Connection::new(
            std::io::Cursor::new(response.clone()),
            Vec::new(),
            Protocol::V1,
            "/repo",
            None::<(&str, Option<u16>)>,
            gix_transport::client::git::ConnectMode::Process,
            false,
        );
        let mut observed = Vec::new();
        let result = gix_protocol::fetch_with_remote_progress(
            &mut Negotiation,
            |reader, _, _| {
                std::io::copy(reader, &mut std::io::sink()).or_error()?;
                Ok(true)
            },
            progress::Discard,
            &std::sync::atomic::AtomicBool::default(),
            fetch::Context {
                handshake: &mut handshake,
                transport: &mut transport,
                user_agent: ("agent", None),
                trace_packetlines: false,
            },
            fetch::Options {
                shallow_file: directory.path().join("shallow"),
                shallow: &fetch::Shallow::NoChange,
                tags: fetch::Tags::None,
                reject_shallow_remote: false,
            },
            |_, bytes| {
                observed.push(bytes.to_vec());
                if fail {
                    Err(std::io::Error::other("raw sideband stopped"))
                } else {
                    Ok(())
                }
            },
        );
        if fail {
            let error = result.expect_err("callback aborts the transfer");
            assert_eq!(
                error
                    .downcast_any_ref::<std::io::Error>()
                    .expect("callback error preserved")
                    .to_string(),
                "raw sideband stopped"
            );
        } else {
            result?;
            assert!(
                observed.iter().any(|bytes| bytes == custom),
                "arbitrary bytes and mixed progress/text survive unchanged"
            );
        }
    }
    Ok(())
}
