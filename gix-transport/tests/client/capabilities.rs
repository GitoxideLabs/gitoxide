use bstr::ByteSlice;
#[cfg(all(feature = "async-client", not(feature = "blocking-client")))]
use gix_packetline::async_io::{StreamingPeekableIter, encode};
#[cfg(all(feature = "blocking-client", not(feature = "async-client")))]
use gix_packetline::blocking_io::{StreamingPeekableIter, encode};
use gix_transport::client::Capabilities;
#[cfg(all(feature = "async-client", not(feature = "blocking-client")))]
use gix_transport::client::capabilities::async_recv::Handshake;
#[cfg(all(feature = "blocking-client", not(feature = "async-client")))]
use gix_transport::client::capabilities::blocking_recv::Handshake;

#[test]
fn from_bytes() -> crate::Result {
    let (caps, delim_pos) = Capabilities::from_bytes(&b"7814e8a05a59c0cf5fb186661d1551c75d1299b5 HEAD\0multi_ack thin-pack side-band side-band-64k ofs-delta shallow deepen-since deepen-not deepen-relative no-progress include-tag multi_ack_detailed symref=HEAD:refs/heads/master object-format=sha1 agent=git/2.28.0"[..])?;
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

#[maybe_async::test(feature = "blocking-client", async(feature = "async-client", async_std::test))]
async fn from_lines_with_version_detection_v0() -> crate::Result {
    let mut buf = Vec::<u8>::new();
    encode::flush_to_write(&mut buf).await?;
    let mut stream = StreamingPeekableIter::new(buf.as_slice(), &[gix_packetline::PacketLineRef::Flush], false);
    let caps = Handshake::from_lines_with_version_detection(&mut stream)
        .await
        .expect("we can parse V0 as very special case, useful for testing stateful connections in other crates")
        .capabilities;
    assert!(caps.contains("multi_ack_detailed"));
    assert!(caps.contains("side-band-64k"));
    Ok(())
}
