//! Integration tests for upload-pack `serve_v2` verifying wire-level correctness
//! of the `acknowledgments` and `packfile` sections when `done=true`.
use std::io::{Cursor, Write as _};

use bstr::ByteSlice;
use gix_protocol::{
    fetch::response::Acknowledgement,
    handshake::Ref,
    upload_pack::{Delegate, Fetch, FetchOutput, LsRefs, Outcome, ServerConfig, serve_v2},
};
use gix_transport::packetline::{BandRef, PacketLineRef, blocking_io::StreamingPeekableIter};

type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Default)]
struct MockDelegate {
    refs: Vec<Ref>,
    fetch_output: Option<FetchOutput>,
    seen_fetch: Option<Fetch>,
}

impl Delegate for MockDelegate {
    fn ls_refs(&mut self, _request: &LsRefs) -> Result<Vec<Ref>, BoxError> {
        Ok(self.refs.clone())
    }

    fn fetch(&mut self, request: &Fetch) -> Result<FetchOutput, BoxError> {
        self.seen_fetch = Some(request.clone());
        self.fetch_output
            .take()
            .ok_or_else(|| std::io::Error::other("fetch output should be configured").into())
    }
}

/// Fresh clone scenario: `done=true`, no haves, delegate returns empty acknowledgements + pack data.
/// The wire output must contain `packfile` section WITHOUT a preceding `acknowledgments` section.
#[test]
fn serve_v2_done_fresh_clone_omits_acknowledgments_section() -> crate::Result {
    let request = request_bytes(
        "fetch",
        &["agent=git/test"],
        &["want 808e50d724f604f69ab93c6da2919c014667bedb", "done"],
    )?;
    let mut output = Vec::new();
    let fetch_output = FetchOutput::new(Cursor::new(b"PACK\0\0\0\0".to_vec()));
    // Empty acknowledgements = fresh clone with done=true
    assert!(
        fetch_output.acknowledgements.is_empty(),
        "fresh clone should have no acknowledgements"
    );
    let mut delegate = MockDelegate {
        fetch_output: Some(fetch_output),
        ..Default::default()
    };

    let outcome = serve_v2(request.as_slice(), &mut output, &mut delegate, &ServerConfig::default())?;
    assert_eq!(
        outcome,
        Outcome::Fetch {
            acknowledgements_sent: 0,
            shallow_updates_sent: 0,
            wanted_refs_sent: 0,
            pack_bytes_sent: 8,
        },
        "fresh clone with done=true should send zero acknowledgements"
    );
    assert!(
        delegate.seen_fetch.as_ref().expect("request should be captured").done,
        "done flag should be parsed from request"
    );

    // Parse wire output: first section header must be `packfile`, not `acknowledgments`
    let mut reader = StreamingPeekableIter::new(output.as_slice(), &[PacketLineRef::Flush], false);
    let first_section = next_text_line(&mut reader)?;
    assert_eq!(
        first_section.as_bstr(),
        "packfile".as_bytes().as_bstr(),
        "fresh clone with done=true must start with packfile section, no acknowledgments"
    );
    assert_eq!(
        next_band_data(&mut reader)?,
        b"PACK\0\0\0\0",
        "pack data should follow packfile header"
    );
    assert!(
        reader.read_line().is_none(),
        "flush should terminate response"
    );
    assert_eq!(reader.stopped_at(), Some(PacketLineRef::Flush));
    Ok(())
}

/// Fetch with common objects: `done=true`, delegate returns acknowledgements with
/// `[Common(id), Ready]` and pack data.
/// The wire output must contain `acknowledgments` section with `ready` line followed by `packfile`.
#[test]
fn serve_v2_done_with_common_objects_includes_acknowledgments_with_ready() -> crate::Result {
    let common_id = gix_hash::ObjectId::from_hex(b"808e50d724f604f69ab93c6da2919c014667bedb")?;
    let request = request_bytes(
        "fetch",
        &["agent=git/test"],
        &[
            "want 9e320b9180e0b5580af68fa3255b7f3d9ecd5af0",
            &format!("have {common_id}"),
            "done",
        ],
    )?;
    let mut output = Vec::new();
    let mut fetch_output = FetchOutput::new(Cursor::new(b"PACK\0\0\0\0".to_vec()));
    fetch_output
        .acknowledgements
        .push(Acknowledgement::Common(common_id));
    fetch_output.acknowledgements.push(Acknowledgement::Ready);
    let mut delegate = MockDelegate {
        fetch_output: Some(fetch_output),
        ..Default::default()
    };

    let outcome = serve_v2(request.as_slice(), &mut output, &mut delegate, &ServerConfig::default())?;
    assert_eq!(
        outcome,
        Outcome::Fetch {
            acknowledgements_sent: 2,
            shallow_updates_sent: 0,
            wanted_refs_sent: 0,
            pack_bytes_sent: 8,
        },
        "fetch with common objects and done=true should send Common + Ready"
    );
    assert!(
        delegate.seen_fetch.as_ref().expect("request should be captured").done,
        "done flag should be parsed from request"
    );

    // Parse wire output: acknowledgments section with ready, then packfile
    let mut reader = StreamingPeekableIter::new(output.as_slice(), &[PacketLineRef::Flush], false);
    assert_eq!(
        next_text_line(&mut reader)?.as_bstr(),
        "acknowledgments".as_bytes().as_bstr(),
        "response should start with acknowledgments section"
    );
    assert_eq!(
        next_text_line(&mut reader)?.as_bstr(),
        format!("ACK {common_id} common").as_bytes().as_bstr(),
        "first acknowledgement should be Common for the shared object"
    );
    assert_eq!(
        next_text_line(&mut reader)?.as_bstr(),
        "ready".as_bytes().as_bstr(),
        "acknowledgments section should end with ready line when done=true"
    );
    expect_delimiter(&mut reader)?;
    assert_eq!(
        next_text_line(&mut reader)?.as_bstr(),
        "packfile".as_bytes().as_bstr(),
        "packfile section should follow acknowledgments"
    );
    assert_eq!(
        next_band_data(&mut reader)?,
        b"PACK\0\0\0\0",
        "pack data should follow packfile header"
    );
    assert!(
        reader.read_line().is_none(),
        "flush should terminate response"
    );
    assert_eq!(reader.stopped_at(), Some(PacketLineRef::Flush));
    Ok(())
}

/// Client sends `have` lines with objects the server doesn't recognize, `done=true`.
/// Delegate returns empty `FetchOutput` (no acknowledgements, no pack). Wire output should be just a flush.
#[test]
fn serve_v2_done_all_unknown_haves_no_pack_produces_empty_response() -> crate::Result {
    let request = request_bytes(
        "fetch",
        &["agent=git/test"],
        &[
            "want 9e320b9180e0b5580af68fa3255b7f3d9ecd5af0",
            "have aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "have bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "done",
        ],
    )?;
    let mut output = Vec::new();
    let fetch_output = FetchOutput::without_pack();
    let mut delegate = MockDelegate {
        fetch_output: Some(fetch_output),
        ..Default::default()
    };

    let outcome = serve_v2(request.as_slice(), &mut output, &mut delegate, &ServerConfig::default())?;
    assert_eq!(
        outcome,
        Outcome::Fetch {
            acknowledgements_sent: 0,
            shallow_updates_sent: 0,
            wanted_refs_sent: 0,
            pack_bytes_sent: 0,
        },
        "server with nothing to say should send zero in all sections"
    );
    assert!(
        delegate.seen_fetch.as_ref().expect("request should be captured").done,
        "done flag should be parsed from request"
    );

    // Wire output should be just a flush (no sections at all)
    let mut reader = StreamingPeekableIter::new(output.as_slice(), &[PacketLineRef::Flush], false);
    assert!(
        reader.read_line().is_none(),
        "empty response should contain only flush"
    );
    assert_eq!(reader.stopped_at(), Some(PacketLineRef::Flush));
    Ok(())
}

/// Client sends wants but no haves, `done=false`. Delegate returns NAK acknowledgement and no pack.
/// Wire output should have `acknowledgments` section with NAK, delimiter, then flush.
#[test]
fn serve_v2_no_done_ongoing_negotiation_nak() -> crate::Result {
    let request = request_bytes(
        "fetch",
        &["agent=git/test"],
        &["want 9e320b9180e0b5580af68fa3255b7f3d9ecd5af0"],
    )?;
    let mut output = Vec::new();
    let mut fetch_output = FetchOutput::without_pack();
    fetch_output.acknowledgements.push(Acknowledgement::Nak);
    let mut delegate = MockDelegate {
        fetch_output: Some(fetch_output),
        ..Default::default()
    };

    let outcome = serve_v2(request.as_slice(), &mut output, &mut delegate, &ServerConfig::default())?;
    assert_eq!(
        outcome,
        Outcome::Fetch {
            acknowledgements_sent: 1,
            shallow_updates_sent: 0,
            wanted_refs_sent: 0,
            pack_bytes_sent: 0,
        },
        "ongoing negotiation with NAK should send one acknowledgement and no pack"
    );
    assert!(
        !delegate.seen_fetch.as_ref().expect("request should be captured").done,
        "done flag should be false for ongoing negotiation"
    );

    // Parse wire output: acknowledgments section with NAK, delimiter, then flush
    let mut reader = StreamingPeekableIter::new(output.as_slice(), &[PacketLineRef::Flush], false);
    assert_eq!(
        next_text_line(&mut reader)?.as_bstr(),
        "acknowledgments".as_bytes().as_bstr(),
        "response should start with acknowledgments section"
    );
    assert_eq!(
        next_text_line(&mut reader)?.as_bstr(),
        "NAK".as_bytes().as_bstr(),
        "acknowledgments section should contain NAK"
    );
    expect_delimiter(&mut reader)?;
    assert!(
        reader.read_line().is_none(),
        "flush should terminate response after acknowledgments"
    );
    assert_eq!(reader.stopped_at(), Some(PacketLineRef::Flush));
    Ok(())
}

/// Client sends haves the server knows, `done=false`. Delegate returns Common acknowledgement, no pack.
/// Wire output should have `acknowledgments` section with ACK <id> common, delimiter, then flush.
#[test]
fn serve_v2_no_done_ongoing_negotiation_common_only() -> crate::Result {
    let common_id = gix_hash::ObjectId::from_hex(b"808e50d724f604f69ab93c6da2919c014667bedb")?;
    let request = request_bytes(
        "fetch",
        &["agent=git/test"],
        &[
            "want 9e320b9180e0b5580af68fa3255b7f3d9ecd5af0",
            &format!("have {common_id}"),
        ],
    )?;
    let mut output = Vec::new();
    let mut fetch_output = FetchOutput::without_pack();
    fetch_output
        .acknowledgements
        .push(Acknowledgement::Common(common_id));
    let mut delegate = MockDelegate {
        fetch_output: Some(fetch_output),
        ..Default::default()
    };

    let outcome = serve_v2(request.as_slice(), &mut output, &mut delegate, &ServerConfig::default())?;
    assert_eq!(
        outcome,
        Outcome::Fetch {
            acknowledgements_sent: 1,
            shallow_updates_sent: 0,
            wanted_refs_sent: 0,
            pack_bytes_sent: 0,
        },
        "ongoing negotiation with common should send one acknowledgement and no pack"
    );
    assert!(
        !delegate.seen_fetch.as_ref().expect("request should be captured").done,
        "done flag should be false for ongoing negotiation"
    );

    // Parse wire output: acknowledgments section with ACK <id> common, no ready, delimiter, flush
    let mut reader = StreamingPeekableIter::new(output.as_slice(), &[PacketLineRef::Flush], false);
    assert_eq!(
        next_text_line(&mut reader)?.as_bstr(),
        "acknowledgments".as_bytes().as_bstr(),
        "response should start with acknowledgments section"
    );
    assert_eq!(
        next_text_line(&mut reader)?.as_bstr(),
        format!("ACK {common_id} common").as_bytes().as_bstr(),
        "acknowledgments section should contain ACK for common object"
    );
    expect_delimiter(&mut reader)?;
    assert!(
        reader.read_line().is_none(),
        "flush should terminate response with no packfile section"
    );
    assert_eq!(reader.stopped_at(), Some(PacketLineRef::Flush));
    Ok(())
}

/// Tests all optional sections together: `done=true`, delegate returns acknowledgements with
/// multiple Common + Ready, wanted-refs, and pack data.
#[test]
fn serve_v2_done_multiple_common_haves_with_wanted_refs_and_pack() -> crate::Result {
    use gix_protocol::fetch::response::WantedRef;

    let id1 = gix_hash::ObjectId::from_hex(b"808e50d724f604f69ab93c6da2919c014667bedb")?;
    let id2 = gix_hash::ObjectId::from_hex(b"9e320b9180e0b5580af68fa3255b7f3d9ecd5af0")?;
    let wanted_id = gix_hash::ObjectId::from_hex(b"dce0ea858eef7ff61ad345cc5cdac62203fb3c10")?;
    let request = request_bytes(
        "fetch",
        &["agent=git/test"],
        &[
            "want 9e320b9180e0b5580af68fa3255b7f3d9ecd5af0",
            &format!("have {id1}"),
            &format!("have {id2}"),
            "want-ref refs/heads/main",
            "done",
        ],
    )?;
    let mut output = Vec::new();
    let mut fetch_output = FetchOutput::new(Cursor::new(b"PACK\0\0\0\0".to_vec()));
    fetch_output
        .acknowledgements
        .push(Acknowledgement::Common(id1));
    fetch_output
        .acknowledgements
        .push(Acknowledgement::Common(id2));
    fetch_output.acknowledgements.push(Acknowledgement::Ready);
    fetch_output.wanted_refs.push(WantedRef {
        id: wanted_id,
        path: "refs/heads/main".into(),
    });
    let mut delegate = MockDelegate {
        fetch_output: Some(fetch_output),
        ..Default::default()
    };

    let outcome = serve_v2(request.as_slice(), &mut output, &mut delegate, &ServerConfig::default())?;
    assert_eq!(
        outcome,
        Outcome::Fetch {
            acknowledgements_sent: 3,
            shallow_updates_sent: 0,
            wanted_refs_sent: 1,
            pack_bytes_sent: 8,
        },
        "all sections should be counted correctly"
    );

    // Parse wire output: acknowledgments, wanted-refs, packfile
    let mut reader = StreamingPeekableIter::new(output.as_slice(), &[PacketLineRef::Flush], false);
    assert_eq!(
        next_text_line(&mut reader)?.as_bstr(),
        "acknowledgments".as_bytes().as_bstr(),
        "response should start with acknowledgments section"
    );
    assert_eq!(
        next_text_line(&mut reader)?.as_bstr(),
        format!("ACK {id1} common").as_bytes().as_bstr(),
        "first ACK should be for id1"
    );
    assert_eq!(
        next_text_line(&mut reader)?.as_bstr(),
        format!("ACK {id2} common").as_bytes().as_bstr(),
        "second ACK should be for id2"
    );
    assert_eq!(
        next_text_line(&mut reader)?.as_bstr(),
        "ready".as_bytes().as_bstr(),
        "acknowledgments section should end with ready"
    );
    expect_delimiter(&mut reader)?;
    assert_eq!(
        next_text_line(&mut reader)?.as_bstr(),
        "wanted-refs".as_bytes().as_bstr(),
        "wanted-refs section should follow acknowledgments"
    );
    assert_eq!(
        next_text_line(&mut reader)?.as_bstr(),
        format!("{wanted_id} refs/heads/main").as_bytes().as_bstr(),
        "wanted-ref line should contain id and ref path"
    );
    expect_delimiter(&mut reader)?;
    assert_eq!(
        next_text_line(&mut reader)?.as_bstr(),
        "packfile".as_bytes().as_bstr(),
        "packfile section should follow wanted-refs"
    );
    assert_eq!(
        next_band_data(&mut reader)?,
        b"PACK\0\0\0\0",
        "pack data should follow packfile header"
    );
    assert!(
        reader.read_line().is_none(),
        "flush should terminate response"
    );
    assert_eq!(reader.stopped_at(), Some(PacketLineRef::Flush));
    Ok(())
}

/// Delegate returns a completely empty `FetchOutput` (no acknowledgements, no shallow_updates,
/// no wanted_refs, no pack_data). Wire output should be just a flush.
#[test]
fn serve_v2_done_empty_fetch_output_no_sections() -> crate::Result {
    let request = request_bytes(
        "fetch",
        &["agent=git/test"],
        &["want 9e320b9180e0b5580af68fa3255b7f3d9ecd5af0", "done"],
    )?;
    let mut output = Vec::new();
    let fetch_output = FetchOutput::without_pack();
    let mut delegate = MockDelegate {
        fetch_output: Some(fetch_output),
        ..Default::default()
    };

    let outcome = serve_v2(request.as_slice(), &mut output, &mut delegate, &ServerConfig::default())?;
    assert_eq!(
        outcome,
        Outcome::Fetch {
            acknowledgements_sent: 0,
            shallow_updates_sent: 0,
            wanted_refs_sent: 0,
            pack_bytes_sent: 0,
        },
        "completely empty FetchOutput should produce zero counts"
    );

    // Wire output should be just a flush
    let mut reader = StreamingPeekableIter::new(output.as_slice(), &[PacketLineRef::Flush], false);
    assert!(
        reader.read_line().is_none(),
        "empty FetchOutput should produce only a flush on the wire"
    );
    assert_eq!(reader.stopped_at(), Some(PacketLineRef::Flush));
    Ok(())
}

/// `done=true`, delegate returns acknowledgements with Common + Ready, shallow updates,
/// and pack data. Verifies section ordering: acknowledgments, shallow-info, packfile.
#[test]
fn serve_v2_done_with_shallow_updates_between_acks_and_pack() -> crate::Result {
    use gix_protocol::fetch::response::ShallowUpdate;

    let common_id = gix_hash::ObjectId::from_hex(b"808e50d724f604f69ab93c6da2919c014667bedb")?;
    let shallow_id = gix_hash::ObjectId::from_hex(b"dce0ea858eef7ff61ad345cc5cdac62203fb3c10")?;
    let request = request_bytes(
        "fetch",
        &["agent=git/test"],
        &[
            "want 9e320b9180e0b5580af68fa3255b7f3d9ecd5af0",
            &format!("have {common_id}"),
            "done",
        ],
    )?;
    let mut output = Vec::new();
    let mut fetch_output = FetchOutput::new(Cursor::new(b"PACK\0\0\0\0".to_vec()));
    fetch_output
        .acknowledgements
        .push(Acknowledgement::Common(common_id));
    fetch_output.acknowledgements.push(Acknowledgement::Ready);
    fetch_output
        .shallow_updates
        .push(ShallowUpdate::Shallow(shallow_id));
    let mut delegate = MockDelegate {
        fetch_output: Some(fetch_output),
        ..Default::default()
    };

    let outcome = serve_v2(request.as_slice(), &mut output, &mut delegate, &ServerConfig::default())?;
    assert_eq!(
        outcome,
        Outcome::Fetch {
            acknowledgements_sent: 2,
            shallow_updates_sent: 1,
            wanted_refs_sent: 0,
            pack_bytes_sent: 8,
        },
        "all sections should be counted correctly with shallow updates"
    );

    // Parse wire output: acknowledgments, shallow-info, packfile
    let mut reader = StreamingPeekableIter::new(output.as_slice(), &[PacketLineRef::Flush], false);
    assert_eq!(
        next_text_line(&mut reader)?.as_bstr(),
        "acknowledgments".as_bytes().as_bstr(),
        "response should start with acknowledgments section"
    );
    assert_eq!(
        next_text_line(&mut reader)?.as_bstr(),
        format!("ACK {common_id} common").as_bytes().as_bstr(),
        "first line should be ACK for common object"
    );
    assert_eq!(
        next_text_line(&mut reader)?.as_bstr(),
        "ready".as_bytes().as_bstr(),
        "acknowledgments section should end with ready"
    );
    expect_delimiter(&mut reader)?;
    assert_eq!(
        next_text_line(&mut reader)?.as_bstr(),
        "shallow-info".as_bytes().as_bstr(),
        "shallow-info section should follow acknowledgments"
    );
    assert_eq!(
        next_text_line(&mut reader)?.as_bstr(),
        format!("shallow {shallow_id}").as_bytes().as_bstr(),
        "shallow-info should contain shallow line with id"
    );
    expect_delimiter(&mut reader)?;
    assert_eq!(
        next_text_line(&mut reader)?.as_bstr(),
        "packfile".as_bytes().as_bstr(),
        "packfile section should follow shallow-info"
    );
    assert_eq!(
        next_band_data(&mut reader)?,
        b"PACK\0\0\0\0",
        "pack data should follow packfile header"
    );
    assert!(
        reader.read_line().is_none(),
        "flush should terminate response"
    );
    assert_eq!(reader.stopped_at(), Some(PacketLineRef::Flush));
    Ok(())
}

fn next_text_line(reader: &mut StreamingPeekableIter<&[u8]>) -> Result<bstr::BString, Box<dyn std::error::Error>> {
    let line = reader
        .read_line()
        .expect("expected packetline")
        .expect("read should succeed")
        .expect("decode should succeed");
    Ok(line.as_text().expect("expected text packetline").as_bstr().to_owned())
}

fn expect_delimiter(reader: &mut StreamingPeekableIter<&[u8]>) -> Result<(), Box<dyn std::error::Error>> {
    let line = reader
        .read_line()
        .expect("expected packetline")
        .expect("read should succeed")
        .expect("decode should succeed");
    match line {
        PacketLineRef::Delimiter => Ok(()),
        other => Err(format!("expected delimiter, got {other:?}").into()),
    }
}

fn next_band_data(reader: &mut StreamingPeekableIter<&[u8]>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let line = reader
        .read_line()
        .expect("expected packetline")
        .expect("read should succeed")
        .expect("decode should succeed");
    match line.decode_band()? {
        BandRef::Data(data) => Ok(data.to_vec()),
        other => Err(format!("expected data band, got {other:?}").into()),
    }
}

/// When serve_v2 receives a request with a mismatched `object-format` (e.g., sha256 against a
/// SHA-1 server), the delegate's methods must never be called — validation rejects the request
/// before any repository access occurs.
#[test]
fn serve_v2_delegate_not_called_on_validation_failure() -> crate::Result {
    use std::sync::atomic::{AtomicBool, Ordering};

    /// A delegate that sets a flag (and panics) if any method is invoked.
    struct NeverCalledDelegate {
        was_called: AtomicBool,
    }

    impl Delegate for NeverCalledDelegate {
        fn ls_refs(&mut self, _request: &LsRefs) -> Result<Vec<Ref>, BoxError> {
            self.was_called.store(true, Ordering::SeqCst);
            panic!("delegate ls_refs should not be called on validation failure");
        }

        fn fetch(&mut self, _request: &Fetch) -> Result<FetchOutput, BoxError> {
            self.was_called.store(true, Ordering::SeqCst);
            panic!("delegate fetch should not be called on validation failure");
        }
    }

    // Build a fetch request declaring object-format=sha256
    let request = request_bytes(
        "fetch",
        &["object-format=sha256"],
        &["want 9e320b9180e0b5580af68fa3255b7f3d9ecd5af0", "done"],
    )?;
    let mut output = Vec::new();
    let mut delegate = NeverCalledDelegate {
        was_called: AtomicBool::new(false),
    };

    // Server is configured for SHA-1, so sha256 request should be rejected
    let config = ServerConfig::default();
    assert_eq!(
        config.object_hash,
        gix_hash::Kind::Sha1,
        "default config should be SHA-1"
    );

    let result = serve_v2(request.as_slice(), &mut output, &mut delegate, &config);

    assert!(result.is_err(), "serve_v2 should return an error for mismatched object-format");
    let err = result.unwrap_err();
    assert!(
        matches!(err, gix_protocol::upload_pack::Error::UnsupportedObjectFormat { .. }),
        "error should be UnsupportedObjectFormat, got: {err:?}"
    );
    assert!(
        !delegate.was_called.load(Ordering::SeqCst),
        "delegate methods must not be called when validation fails"
    );

    Ok(())
}

fn request_bytes(
    command: &str,
    features: &[&str],
    arguments: &[&str],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use gix_transport::packetline::blocking_io::{Writer, encode};

    let mut out = Vec::new();
    let mut writer = Writer::new(&mut out);
    writer.enable_text_mode();
    writer.write_all(format!("command={command}").as_bytes())?;
    for feature in features {
        writer.write_all(feature.as_bytes())?;
    }
    if arguments.is_empty() {
        encode::flush_to_write(writer.inner_mut())?;
        return Ok(out);
    }

    encode::delim_to_write(writer.inner_mut())?;
    for argument in arguments {
        writer.write_all(argument.as_bytes())?;
    }
    encode::flush_to_write(writer.inner_mut())?;
    Ok(out)
}
