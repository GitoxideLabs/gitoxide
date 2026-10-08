use crate::TestResult;
use bstr::ByteSlice;
#[cfg(all(feature = "async-client", not(feature = "blocking-client")))]
use gix_packetline::async_io::{StreamingPeekableIter, encode};
#[cfg(feature = "blocking-client")]
use gix_packetline::blocking_io::{StreamingPeekableIter, encode};
use gix_transport::client::Capabilities;
#[cfg(all(feature = "async-client", not(feature = "blocking-client")))]
use gix_transport::client::capabilities::async_recv::Handshake;
#[cfg(feature = "blocking-client")]
use gix_transport::client::capabilities::blocking_recv::Handshake;

#[test]
fn from_bytes() -> TestResult {
    let (caps, delim_pos) = Capabilities::from_bytes(
        &b"7814e8a05a59c0cf5fb186661d1551c75d1299b5 HEAD\0\
           multi_ack thin-pack side-band side-band-64k ofs-delta \
           shallow deepen-since deepen-not deepen-relative \
           no-progress include-tag multi_ack_detailed \
           symref=HEAD:refs/heads/master \
           object-format=sha1 \
           agent=git/2.28.0"[..],
    )?;
    assert_eq!(delim_pos, 45);
    assert_eq!(
        caps.iter().map(|c| c.name().to_owned()).collect::<Vec<_>>(),
        vec![
            "multi_ack",
            "thin-pack",
            "side-band",
            "side-band-64k",
            "ofs-delta",
            "shallow",
            "deepen-since",
            "deepen-not",
            "deepen-relative",
            "no-progress",
            "include-tag",
            "multi_ack_detailed",
            "symref",
            "object-format",
            "agent"
        ]
        .into_iter()
        .map(|s| s.as_bytes().as_bstr())
        .collect::<Vec<_>>()
    );
    let object_format = caps.capability("object-format").expect("cap exists");
    assert!(
        object_format.supports("sha1").expect("there is a value"),
        "sha1 is supported"
    );
    assert!(
        !object_format.supports("sha2").expect("there is a value"),
        "sha2 is not supported"
    );
    assert_eq!(
        caps.iter()
            .filter_map(|c| c.value().map(ToOwned::to_owned))
            .collect::<Vec<_>>(),
        vec![
            b"HEAD:refs/heads/master".as_bstr(),
            b"sha1".as_bstr(),
            b"git/2.28.0".as_bstr()
        ]
    );
    Ok(())
}

#[test]
fn malformed_advertisements_are_corruption() {
    let cases = [
        (
            Capabilities::from_bytes(b"HEAD").map(|(caps, _)| caps),
            "Capabilities were missing entirely as there was no 0 byte".to_owned(),
        ),
        (
            Capabilities::from_bytes(b"HEAD\0").map(|(caps, _)| caps),
            "there was not a single capability behind the delimiter".to_owned(),
        ),
        (
            Capabilities::from_lines(" \n".into()),
            "a version line was expected, but none was retrieved".to_owned(),
        ),
        (
            Capabilities::from_lines("version".into()),
            format!("expected 'version X', got {:?}", b"version".as_slice()),
        ),
        (
            Capabilities::from_lines("protocol 2".into()),
            format!("expected 'version X', got {:?}", b"protocol 2".as_slice()),
        ),
    ];
    for (result, expected) in cases {
        let err = result.expect_err("the capability advertisement is malformed");
        assert_eq!(
            err.to_string(),
            expected,
            "classification must not change the diagnostic"
        );
        assert_eq!(
            err.classify().map(|class| class.class()).collect::<Vec<_>>(),
            [gix_error::Class::Corruption],
            "the peer's malformed advertisement is not invalid caller configuration"
        );
        assert_eq!(err.iter_errors().count(), 1, "classification adds no synthetic cause");
    }
}

#[crate::bisync::bisync]
#[cfg_attr(feature = "blocking-client", test)]
#[cfg_attr(all(feature = "async-client", not(feature = "blocking-client")), async_std::test)]
async fn malformed_advertisements_keep_the_capabilities_error() -> gix_testtools::TestResult {
    for line in [b"HEAD".as_slice(), b"HEAD\0"] {
        let mut buf = Vec::new();
        encode::data_to_write(line, &mut buf).await?;
        encode::flush_to_write(&mut buf).await?;
        let mut stream = StreamingPeekableIter::new(buf.as_slice(), &[gix_packetline::PacketLineRef::Flush], false);
        let err = Handshake::from_lines_with_version_detection(&mut stream)
            .await
            .err()
            .expect("malformed capabilities fail the handshake");
        assert!(
            matches!(err, gix_transport::client::Error::Capabilities { .. }),
            "the typed capability-parsing error remains available"
        );
        assert!(
            gix_error::classify(&err).is_corrupted(),
            "the transport error exposes the parser's classification"
        );
        let err = gix_error::Error::from(err);
        assert!(err.is_corrupted(), "erasing the error preserves corruption");
        assert!(
            err.downcast_any_ref::<gix_transport::client::Error>().is_some(),
            "erasing the error preserves typed recovery"
        );
    }
    Ok(())
}

#[crate::bisync::bisync]
#[cfg_attr(feature = "blocking-client", test)]
#[cfg_attr(all(feature = "async-client", not(feature = "blocking-client")), async_std::test)]
async fn unsupported_versions_are_classified() -> gix_testtools::TestResult {
    for line in ["version 1", "version 3", "version 42"] {
        let err = Capabilities::from_lines(line.into()).expect_err("only version 2 is supported by this parser");
        assert!(err.is_unsupported(), "unsupported versions are not malformed data");

        let mut buf = Vec::new();
        encode::text_to_write(line.as_bytes(), &mut buf).await?;
        encode::flush_to_write(&mut buf).await?;
        let mut stream = StreamingPeekableIter::new(buf.as_slice(), &[gix_packetline::PacketLineRef::Flush], false);
        let err = Handshake::from_lines_with_version_detection(&mut stream)
            .await
            .err()
            .expect("these version advertisements are currently unsupported");
        if line == "version 1" {
            assert!(
                matches!(err, gix_transport::client::Error::Capabilities { .. }),
                "the documented explicit-v1-header limitation keeps its existing error"
            );
        } else {
            assert!(
                matches!(&err, gix_transport::client::Error::UnsupportedProtocolVersion(version) if version == line),
                "the unsupported protocol version remains available for typed recovery"
            );
        }
        assert!(
            gix_error::classify(&err).any(|class| class.class() == gix_error::Class::Unsupported),
            "valid but unsupported protocol versions must not be called corrupt"
        );
        assert!(
            gix_error::Error::from(err).is_unsupported(),
            "erasing an unsupported-protocol error preserves its classification"
        );
    }
    Ok(())
}

#[test]
fn from_bytes_with_sha256_object_format() -> TestResult {
    let (caps, _delim_pos) = Capabilities::from_bytes(
        &b"7814e8a05a59c0cf5fb186661d1551c75d1299b5 HEAD\0\
           side-band-64k \
           object-format=sha256 \
           agent=git/2.40.0"[..],
    )?;
    let object_format = caps.capability("object-format").expect("cap exists");
    assert!(
        object_format.supports("sha256").expect("there is a value"),
        "sha256 is supported"
    );
    assert!(
        !object_format.supports("sha1").expect("there is a value"),
        "sha1 is not supported when the server advertises sha256"
    );
    Ok(())
}

#[crate::bisync::bisync]
#[cfg_attr(feature = "blocking-client", test)]
#[cfg_attr(all(feature = "async-client", not(feature = "blocking-client")), async_std::test)]
async fn from_lines_with_version_detection_v0() -> TestResult {
    let mut buf = Vec::<u8>::new();
    encode::flush_to_write(&mut buf).await?;
    let mut stream = StreamingPeekableIter::new(buf.as_slice(), &[gix_packetline::PacketLineRef::Flush], false);
    let caps = Handshake::from_lines_with_version_detection(&mut stream)
        .await?
        .capabilities;
    assert!(caps.contains("multi_ack_detailed"));
    assert!(caps.contains("side-band-64k"));
    Ok(())
}
