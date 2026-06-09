//! Async integration tests for `upload_pack::async_io` module.
//!
//! Tests verify that the async bridge functions produce correct protocol output
//! using in-memory async streams.

use std::io::Write as _;

use bstr::ByteSlice as _;
use futures_lite::io::Cursor;
use gix_protocol::fetch::response::{Acknowledgement, ShallowUpdate};
use gix_protocol::handshake::Ref;
use gix_protocol::upload_pack::async_io::{self, AsyncFetchOutput, write_fetch_response};
use gix_protocol::upload_pack::{self, Delegate, Fetch, FetchOutput, LsRefs, Outcome, ServerConfig};
use gix_transport::packetline::{
    BandRef, PacketLineRef,
    blocking_io::{StreamingPeekableIter, Writer, encode},
};

type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

// ---------------------------------------------------------------------------
// Mock delegates
// ---------------------------------------------------------------------------

/// A mock delegate that returns a fixed set of refs for `ls_refs`.
struct LsRefsDelegate {
    refs: Vec<Ref>,
}

impl Delegate for LsRefsDelegate {
    fn ls_refs(&mut self, _request: &LsRefs) -> Result<Vec<Ref>, BoxError> {
        Ok(self.refs.clone())
    }

    fn fetch(&mut self, _request: &Fetch) -> Result<FetchOutput, BoxError> {
        unreachable!("fetch should not be called in ls-refs test")
    }
}

/// A mock delegate for testing upload-pack fetch operations.
struct FetchDelegate {
    acknowledgements: Vec<Acknowledgement>,
    pack_data: Option<Vec<u8>>,
}

impl Delegate for FetchDelegate {
    fn ls_refs(&mut self, _request: &LsRefs) -> Result<Vec<Ref>, BoxError> {
        Ok(Vec::new())
    }

    fn fetch(&mut self, _request: &Fetch) -> Result<FetchOutput, BoxError> {
        let mut output = if let Some(ref data) = self.pack_data {
            FetchOutput::new(std::io::Cursor::new(data.clone()))
        } else {
            FetchOutput::without_pack()
        };
        output.acknowledgements.clone_from(&self.acknowledgements);
        Ok(output)
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Build a valid protocol V2 ls-refs request as raw bytes.
fn build_ls_refs_request(arguments: &[&str]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut out = Vec::new();
    {
        let mut writer = Writer::new(&mut out);
        writer.enable_text_mode();
        writer.write_all(b"command=ls-refs")?;
        encode::delim_to_write(writer.inner_mut())?;
        for arg in arguments {
            writer.write_all(arg.as_bytes())?;
        }
        encode::flush_to_write(writer.inner_mut())?;
    }
    Ok(out)
}

/// Build a valid protocol V2 fetch request as raw bytes.
fn build_fetch_request(
    want_oid: &str,
    have_oid: Option<&str>,
    done: bool,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut out = Vec::new();
    {
        let mut writer = Writer::new(&mut out);
        writer.enable_text_mode();
        writer.write_all(b"command=fetch")?;
        encode::delim_to_write(writer.inner_mut())?;
        writer.write_all(format!("want {want_oid}").as_bytes())?;
        if let Some(have) = have_oid {
            writer.write_all(format!("have {have}").as_bytes())?;
        }
        if done {
            writer.write_all(b"done")?;
        }
        encode::flush_to_write(writer.inner_mut())?;
    }
    Ok(out)
}

/// Read the next text packetline from the reader.
fn next_text_line(
    reader: &mut StreamingPeekableIter<&[u8]>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let line = reader
        .read_line()
        .expect("expected packetline")
        .expect("read should succeed")
        .expect("decode should succeed");
    let text = line.as_text().expect("expected text packetline");
    Ok(text.as_slice().to_vec())
}

/// Assert the next packetline is a delimiter.
fn expect_delimiter(
    reader: &mut StreamingPeekableIter<&[u8]>,
) -> Result<(), Box<dyn std::error::Error>> {
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[async_std::test]
async fn serve_v2_bridges_async_transport_for_ls_refs() -> Result<(), Box<dyn std::error::Error>> {
    let request = build_ls_refs_request(&["symrefs", "ref-prefix refs/heads/"])?;
    let mut input = Cursor::new(request);
    let mut output = Cursor::new(Vec::<u8>::new());

    let oid_a = gix_hash::ObjectId::from_hex(b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        .expect("valid hex for oid_a");
    let oid_b = gix_hash::ObjectId::from_hex(b"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
        .expect("valid hex for oid_b");
    let oid_c = gix_hash::ObjectId::from_hex(b"cccccccccccccccccccccccccccccccccccccccc")
        .expect("valid hex for oid_c");

    let mut delegate = LsRefsDelegate {
        refs: vec![
            Ref::Symbolic {
                full_ref_name: "refs/heads/main".into(),
                target: "refs/heads/main".into(),
                tag: None,
                object: oid_a,
            },
            Ref::Direct {
                full_ref_name: "refs/heads/feature".into(),
                object: oid_b,
            },
            Ref::Direct {
                full_ref_name: "refs/tags/v1.0".into(),
                object: oid_c,
            },
        ],
    };

    let outcome = async_io::serve_v2(&mut input, &mut output, &mut delegate, &ServerConfig::default()).await?;

    assert_eq!(
        outcome,
        Outcome::LsRefs { refs_sent: 2 },
        "only refs matching 'refs/heads/' prefix should be sent"
    );

    let output_bytes = output.into_inner();
    let mut reader =
        StreamingPeekableIter::new(output_bytes.as_slice(), &[PacketLineRef::Flush], false);

    let line1 = next_text_line(&mut reader)?;
    assert!(
        line1.contains_str("refs/heads/main"),
        "first ref line should contain refs/heads/main, got: {:?}",
        line1.as_bstr()
    );
    assert!(
        line1.contains_str("symref-target:"),
        "first ref line should contain symref-target since symrefs was requested, got: {:?}",
        line1.as_bstr()
    );

    let line2 = next_text_line(&mut reader)?;
    assert!(
        line2.contains_str("refs/heads/feature"),
        "second ref line should contain refs/heads/feature, got: {:?}",
        line2.as_bstr()
    );
    let expected_prefix = oid_b.to_string();
    assert!(
        line2.starts_with(expected_prefix.as_bytes()),
        "second ref line should start with the object id, got: {:?}",
        line2.as_bstr()
    );

    assert!(
        reader.read_line().is_none(),
        "there should be no more lines before flush"
    );
    assert_eq!(
        reader.stopped_at(),
        Some(PacketLineRef::Flush),
        "output should end with a flush packet"
    );

    Ok(())
}

#[async_std::test]
async fn serve_v2_bridges_async_transport_for_fetch_with_pack_data(
) -> Result<(), Box<dyn std::error::Error>> {
    let want_oid_hex = "808e50d724f604f69ab93c6da2919c014667bedb";
    let have_oid_hex = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let have_oid =
        gix_hash::ObjectId::from_hex(have_oid_hex.as_bytes()).expect("valid hex for have OID");

    let pack_data = b"PACK test data for sideband framing";

    let request = build_fetch_request(want_oid_hex, Some(have_oid_hex), true)?;
    let mut input = Cursor::new(request);
    let mut output = Cursor::new(Vec::<u8>::new());

    let mut delegate = FetchDelegate {
        acknowledgements: vec![Acknowledgement::Common(have_oid), Acknowledgement::Ready],
        pack_data: Some(pack_data.to_vec()),
    };

    let outcome = async_io::serve_v2(&mut input, &mut output, &mut delegate, &ServerConfig::default()).await?;

    match outcome {
        Outcome::Fetch {
            acknowledgements_sent,
            shallow_updates_sent,
            wanted_refs_sent,
            pack_bytes_sent,
        } => {
            assert_eq!(
                acknowledgements_sent, 2,
                "should report 2 acknowledgements (Common + Ready)"
            );
            assert_eq!(shallow_updates_sent, 0, "no shallow updates expected");
            assert_eq!(wanted_refs_sent, 0, "no wanted refs expected");
            assert_eq!(
                pack_bytes_sent,
                pack_data.len() as u64,
                "pack_bytes_sent should equal input pack data length"
            );
        }
        other => panic!("expected Outcome::Fetch, got {other:?}"),
    }

    let output_bytes = output.into_inner();
    let mut reader = StreamingPeekableIter::new(
        output_bytes.as_slice(),
        &[PacketLineRef::Flush, PacketLineRef::Delimiter],
        false,
    );

    let ack_header = next_text_line(&mut reader)?;
    assert_eq!(
        ack_header.as_slice(),
        b"acknowledgments",
        "first line should be acknowledgments section header"
    );

    let ack_line_1 = next_text_line(&mut reader)?;
    let expected_ack = format!("ACK {have_oid_hex} common");
    assert_eq!(
        ack_line_1.as_slice(),
        expected_ack.as_bytes(),
        "first ACK line should be Common acknowledgement"
    );

    let ack_line_2 = next_text_line(&mut reader)?;
    assert_eq!(
        ack_line_2.as_slice(),
        b"ready",
        "second ACK line should be Ready"
    );

    // Consume the delimiter that ends the acknowledgments section
    assert!(
        reader.read_line().is_none(),
        "reader should stop at delimiter after acknowledgments section"
    );
    assert_eq!(
        reader.stopped_at(),
        Some(PacketLineRef::Delimiter),
        "acknowledgments section should end with delimiter"
    );

    reader.reset_with(&[PacketLineRef::Flush]);

    let packfile_header = next_text_line(&mut reader)?;
    assert_eq!(
        packfile_header.as_slice(),
        b"packfile",
        "next section should be packfile header"
    );

    let mut received_pack_data = Vec::new();
    loop {
        let line = reader.read_line();
        match line {
            None => break,
            Some(Ok(Ok(packet))) => match packet.decode_band() {
                Ok(band) => match band {
                    BandRef::Data(data) => received_pack_data.extend_from_slice(data),
                    BandRef::Progress(_) => {}
                    BandRef::Error(err) => panic!("unexpected error band in output: {:?}", err),
                },
                Err(_) => break,
            },
            Some(Ok(Err(decode_err))) => {
                panic!("decode error while reading sideband packets: {decode_err}");
            }
            Some(Err(io_err)) => {
                panic!("IO error while reading sideband packets: {io_err}");
            }
        }
    }

    assert_eq!(
        received_pack_data.as_slice(),
        pack_data.as_slice(),
        "concatenated sideband payloads should equal original pack data"
    );

    assert_eq!(
        reader.stopped_at(),
        Some(PacketLineRef::Flush),
        "response should end with flush packet"
    );

    Ok(())
}

#[async_std::test]
async fn write_fetch_response_streams_async_pack_data_correctly(
) -> Result<(), Box<dyn std::error::Error>> {
    let pack_bytes: &[u8] = b"PACK\x00\x00\x00\x02test pack data bytes here";
    let common_id = gix_hash::ObjectId::from_hex(b"808e50d724f604f69ab93c6da2919c014667bedb")
        .expect("valid hex for common_id");

    let mut response = AsyncFetchOutput::new(Cursor::new(pack_bytes.to_vec()));
    response
        .acknowledgements
        .push(Acknowledgement::Common(common_id));

    let mut output = Cursor::new(Vec::<u8>::new());
    let bytes_sent = write_fetch_response(&mut output, &mut response).await?;

    assert_eq!(
        bytes_sent,
        pack_bytes.len() as u64,
        "returned byte count should equal the input pack data length"
    );

    let output_bytes = output.into_inner();
    let mut reader =
        StreamingPeekableIter::new(output_bytes.as_slice(), &[PacketLineRef::Flush], false);

    assert_eq!(
        next_text_line(&mut reader)?.as_slice(),
        b"acknowledgments",
        "response should start with acknowledgments section header"
    );
    let expected_ack_line = format!("ACK {common_id} common");
    assert_eq!(
        next_text_line(&mut reader)?.as_slice(),
        expected_ack_line.as_bytes(),
        "acknowledgments section should contain ACK line for common object"
    );

    expect_delimiter(&mut reader)?;

    assert_eq!(
        next_text_line(&mut reader)?.as_slice(),
        b"packfile",
        "packfile section header should follow acknowledgments"
    );

    let mut concatenated_payloads = Vec::new();
    loop {
        match reader.read_line() {
            None => break,
            Some(Ok(Ok(packet))) => match packet.decode_band() {
                Ok(band) => match band {
                    BandRef::Data(data) => concatenated_payloads.extend_from_slice(data),
                    BandRef::Progress(_) => {}
                    BandRef::Error(err) => panic!("unexpected error band: {err:?}"),
                },
                Err(_) => break,
            },
            Some(Ok(Err(e))) => panic!("decode error: {e}"),
            Some(Err(e)) => panic!("IO error: {e}"),
        }
    }

    assert_eq!(
        concatenated_payloads.as_slice(),
        pack_bytes,
        "concatenated sideband channel 1 payloads should equal original pack bytes"
    );

    assert_eq!(
        reader.stopped_at(),
        Some(PacketLineRef::Flush),
        "response should end with a flush packet"
    );

    Ok(())
}

/// When `AsyncFetchOutput` has no pack data, only the metadata sections
/// (acknowledgments, shallow-info) should be written, and the returned byte count
/// should be 0.
#[async_std::test]
async fn write_fetch_response_without_pack_data_writes_only_metadata_sections(
) -> Result<(), Box<dyn std::error::Error>> {
    let shallow_id =
        gix_hash::ObjectId::from_hex(b"dce0ea858eef7ff61ad345cc5cdac62203fb3c10")?;

    let mut response = AsyncFetchOutput::without_pack();
    response.acknowledgements.push(Acknowledgement::Nak);
    response
        .shallow_updates
        .push(ShallowUpdate::Shallow(shallow_id));

    let mut output = Cursor::new(Vec::<u8>::new());
    let pack_bytes_sent = write_fetch_response(&mut output, &mut response).await?;

    assert_eq!(
        pack_bytes_sent, 0,
        "no pack bytes should be sent when pack_data is None"
    );

    let output_bytes = output.into_inner();
    let mut reader =
        StreamingPeekableIter::new(output_bytes.as_slice(), &[PacketLineRef::Flush], false);

    // Acknowledgments section
    assert_eq!(
        next_text_line(&mut reader)?.as_slice(),
        b"acknowledgments",
        "response should start with acknowledgments section header"
    );
    assert_eq!(
        next_text_line(&mut reader)?.as_slice(),
        b"NAK",
        "acknowledgments section should contain NAK"
    );
    expect_delimiter(&mut reader)?;

    // Shallow-info section (last section, no pack follows)
    assert_eq!(
        next_text_line(&mut reader)?.as_slice(),
        b"shallow-info",
        "shallow-info section should follow acknowledgments"
    );
    let expected_shallow = format!("shallow {shallow_id}");
    assert_eq!(
        next_text_line(&mut reader)?.as_slice(),
        expected_shallow.as_bytes(),
        "shallow-info should contain shallow line with correct id"
    );

    // No packfile section - should go directly to flush (no trailing delimiter on last section)
    assert!(
        reader.read_line().is_none(),
        "no packfile section should be present; response should end with flush"
    );
    assert_eq!(
        reader.stopped_at(),
        Some(PacketLineRef::Flush),
        "response should be terminated by a flush packet"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Property tests
// ---------------------------------------------------------------------------

/// **Validates: Requirements 2.2, 5.2, 6.2, 7.2, 10.2**
///
/// Property 1: BlockOn-bridged functions produce identical results to blocking counterparts.
/// For a set of representative inputs (ls-refs, fetch, capability advertisement), call both
/// blocking and async versions. Compare output byte buffers and return values for equality.
#[async_std::test]
async fn property_blockon_bridged_output_matches_blocking_output_byte_for_byte(
) -> Result<(), Box<dyn std::error::Error>> {
    use gix_protocol::upload_pack::Capability;

    // --- Test write_ls_refs_response ---
    let test_cases_ls_refs: Vec<(LsRefs, Vec<Ref>)> = vec![
        // Case 1: empty refs, no filters
        (LsRefs::default(), Vec::new()),
        // Case 2: refs with symrefs filter
        (
            LsRefs {
                symrefs: true,
                ref_prefixes: vec!["refs/heads/".into()],
                ..Default::default()
            },
            vec![
                Ref::Symbolic {
                    full_ref_name: "refs/heads/main".into(),
                    target: "refs/heads/main".into(),
                    tag: None,
                    object: gix_hash::ObjectId::from_hex(
                        b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    )
                    .expect("valid hex"),
                },
                Ref::Direct {
                    full_ref_name: "refs/tags/v1.0".into(),
                    object: gix_hash::ObjectId::from_hex(
                        b"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                    )
                    .expect("valid hex"),
                },
            ],
        ),
        // Case 3: refs with peel option
        (
            LsRefs {
                peel: true,
                ..Default::default()
            },
            vec![Ref::Direct {
                full_ref_name: "refs/heads/feature".into(),
                object: gix_hash::ObjectId::from_hex(
                    b"cccccccccccccccccccccccccccccccccccccccc",
                )
                .expect("valid hex"),
            }],
        ),
    ];

    for (request, refs) in &test_cases_ls_refs {
        // Blocking version
        let mut blocking_output = Vec::<u8>::new();
        let blocking_result =
            upload_pack::write_ls_refs_response(&mut blocking_output, request, refs)?;

        // Async version
        let mut async_output = Cursor::new(Vec::<u8>::new());
        let async_result =
            async_io::write_ls_refs_response(&mut async_output, request, refs).await?;

        assert_eq!(
            blocking_result, async_result,
            "write_ls_refs_response return value should be identical for blocking and async"
        );
        assert_eq!(
            blocking_output,
            async_output.into_inner(),
            "write_ls_refs_response output bytes should be identical for blocking and async"
        );
    }

    // --- Test write_v2_capability_advertisement ---
    let test_cases_caps: Vec<Vec<Capability>> = vec![
        // Case 1: empty capabilities
        Vec::new(),
        // Case 2: single capability with no values
        vec![Capability {
            name: "ls-refs".into(),
            values: Vec::new(),
        }],
        // Case 3: multiple capabilities with values
        vec![
            Capability {
                name: "ls-refs".into(),
                values: Vec::new(),
            },
            Capability {
                name: "fetch".into(),
                values: vec!["shallow".into(), "filter".into()],
            },
            Capability {
                name: "server-option".into(),
                values: Vec::new(),
            },
        ],
    ];

    for capabilities in &test_cases_caps {
        // Blocking version
        let mut blocking_output = Vec::<u8>::new();
        upload_pack::write_v2_capability_advertisement(&mut blocking_output, capabilities)?;

        // Async version
        let mut async_output = Cursor::new(Vec::<u8>::new());
        async_io::write_v2_capability_advertisement(&mut async_output, capabilities).await?;

        assert_eq!(
            blocking_output,
            async_output.into_inner(),
            "write_v2_capability_advertisement output bytes should be identical for blocking and async"
        );
    }

    // --- Test write_fetch_response (blocking FetchOutput vs async AsyncFetchOutput) ---
    let common_id = gix_hash::ObjectId::from_hex(b"808e50d724f604f69ab93c6da2919c014667bedb")
        .expect("valid hex for common_id");
    let shallow_id = gix_hash::ObjectId::from_hex(b"dce0ea858eef7ff61ad345cc5cdac62203fb3c10")
        .expect("valid hex for shallow_id");

    let pack_test_data: Vec<Vec<u8>> = vec![
        // Case 1: small pack data
        b"PACK test data".to_vec(),
        // Case 2: larger pack data
        vec![0xAB; 1000],
    ];

    for pack_data in &pack_test_data {
        // Blocking version
        let mut blocking_fetch_output = FetchOutput::new(std::io::Cursor::new(pack_data.clone()));
        blocking_fetch_output
            .acknowledgements
            .push(Acknowledgement::Common(common_id));
        blocking_fetch_output
            .shallow_updates
            .push(ShallowUpdate::Shallow(shallow_id));
        let mut blocking_output = Vec::<u8>::new();
        let blocking_bytes =
            upload_pack::write_fetch_response(&mut blocking_output, &mut blocking_fetch_output)?;

        // Async version
        let mut async_fetch_output = AsyncFetchOutput::new(Cursor::new(pack_data.clone()));
        async_fetch_output
            .acknowledgements
            .push(Acknowledgement::Common(common_id));
        async_fetch_output
            .shallow_updates
            .push(ShallowUpdate::Shallow(shallow_id));
        let mut async_output = Cursor::new(Vec::<u8>::new());
        let async_bytes =
            write_fetch_response(&mut async_output, &mut async_fetch_output).await?;

        assert_eq!(
            blocking_bytes, async_bytes,
            "write_fetch_response byte count should be identical for blocking and async"
        );
        assert_eq!(
            blocking_output,
            async_output.into_inner(),
            "write_fetch_response output bytes should be identical for blocking and async"
        );
    }

    // Case: no pack data
    {
        let mut blocking_fetch_output = FetchOutput::without_pack();
        blocking_fetch_output
            .acknowledgements
            .push(Acknowledgement::Nak);
        let mut blocking_output = Vec::<u8>::new();
        let blocking_bytes =
            upload_pack::write_fetch_response(&mut blocking_output, &mut blocking_fetch_output)?;

        let mut async_fetch_output = AsyncFetchOutput::without_pack();
        async_fetch_output
            .acknowledgements
            .push(Acknowledgement::Nak);
        let mut async_output = Cursor::new(Vec::<u8>::new());
        let async_bytes =
            write_fetch_response(&mut async_output, &mut async_fetch_output).await?;

        assert_eq!(
            blocking_bytes, async_bytes,
            "write_fetch_response byte count should match for no-pack case"
        );
        assert_eq!(
            blocking_output,
            async_output.into_inner(),
            "write_fetch_response output bytes should match for no-pack case"
        );
    }

    Ok(())
}

/// **Validates: Requirements 3.2, 3.3**
///
/// Property 2: Async write_fetch_response correctly frames pack data as sideband channel 1.
/// Test with various pack data sizes (empty, small, exactly MAX_SIDEBAND_DATA_BYTES, larger
/// requiring multiple chunks). Extract sideband payloads from output, concatenate, and verify
/// they equal the original input.
#[async_std::test]
async fn property_pack_data_round_trip_through_sideband_framing(
) -> Result<(), Box<dyn std::error::Error>> {
    // Test sizes: 0, 10, 1000, exactly 65515 (MAX_SIDEBAND_DATA_BYTES), and 100000 (multiple chunks)
    let test_sizes: &[usize] = &[0, 10, 1000, 65515, 100_000];

    for &size in test_sizes {
        // Create pack data of the given size with a recognizable pattern
        let pack_data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();

        let mut response = AsyncFetchOutput::new(Cursor::new(pack_data.clone()));
        // Add an acknowledgement so we can identify the packfile section boundary
        let ack_id = gix_hash::ObjectId::from_hex(b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
            .expect("valid hex");
        response
            .acknowledgements
            .push(Acknowledgement::Common(ack_id));

        let mut output = Cursor::new(Vec::<u8>::new());
        let bytes_sent = write_fetch_response(&mut output, &mut response).await?;

        let output_bytes = output.into_inner();

        if size == 0 {
            // AsyncFetchOutput::new wraps Some(reader), so pack_data is Some even for 0 bytes.
            // The code writes "packfile" header then reads 0 bytes, so packfile section exists
            // but has no data bands.
            assert_eq!(
                bytes_sent, 0,
                "empty pack data should report 0 bytes sent (size={size})"
            );
        } else {
            assert_eq!(
                bytes_sent, size as u64,
                "bytes_sent should equal input size for size={size}"
            );
        }

        // Parse output to extract sideband channel 1 payloads
        let mut reader = StreamingPeekableIter::new(
            output_bytes.as_slice(),
            &[PacketLineRef::Flush, PacketLineRef::Delimiter],
            false,
        );

        // Skip acknowledgments section
        let header = next_text_line(&mut reader)?;
        assert_eq!(
            header.as_slice(),
            b"acknowledgments",
            "expected acknowledgments header for size={size}"
        );
        // Skip ACK line
        let _ack_line = next_text_line(&mut reader)?;
        // Consume the delimiter that ends acks section
        assert!(
            reader.read_line().is_none(),
            "reader should stop at delimiter after acks for size={size}"
        );
        assert_eq!(
            reader.stopped_at(),
            Some(PacketLineRef::Delimiter),
            "acknowledgments section should end with delimiter for size={size}"
        );

        // Reset to look for packfile section
        reader.reset_with(&[PacketLineRef::Flush]);

        // Read packfile header
        let packfile_header = next_text_line(&mut reader)?;
        assert_eq!(
            packfile_header.as_slice(),
            b"packfile",
            "expected packfile header for size={size}"
        );

        // Extract all sideband channel 1 payloads
        let mut concatenated = Vec::new();
        loop {
            let line = reader.read_line();
            match line {
                None => break,
                Some(Ok(Ok(packet))) => match packet.decode_band() {
                    Ok(band) => match band {
                        BandRef::Data(data) => {
                            // Verify each chunk is at most MAX_SIDEBAND_DATA_BYTES
                            assert!(
                                data.len() <= 65515,
                                "sideband payload should not exceed MAX_SIDEBAND_DATA_BYTES, got {} for size={size}",
                                data.len()
                            );
                            concatenated.extend_from_slice(data);
                        }
                        BandRef::Progress(_) => {}
                        BandRef::Error(err) => {
                            panic!("unexpected error band for size={size}: {err:?}")
                        }
                    },
                    Err(_) => break,
                },
                Some(Ok(Err(e))) => panic!("decode error for size={size}: {e}"),
                Some(Err(e)) => panic!("IO error for size={size}: {e}"),
            }
        }

        assert_eq!(
            concatenated, pack_data,
            "concatenated sideband payloads should equal original pack data for size={size}"
        );

        assert_eq!(
            reader.stopped_at(),
            Some(PacketLineRef::Flush),
            "response should end with flush packet for size={size}"
        );
    }

    Ok(())
}

/// **Validates: Requirements 3.4**
///
/// Property 3: write_fetch_response byte count equals pack bytes consumed.
/// Test with known-length pack data inputs of varying sizes and verify the returned u64
/// equals the input length.
#[async_std::test]
async fn property_returned_byte_count_equals_pack_data_length(
) -> Result<(), Box<dyn std::error::Error>> {
    // Test a range of sizes including edge cases
    let test_sizes: &[usize] = &[0, 1, 100, 1000, 8192, 65515, 65516, 100_000, 200_000];

    for &size in test_sizes {
        let pack_data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();

        let mut response = AsyncFetchOutput::new(Cursor::new(pack_data));
        let mut output = Cursor::new(Vec::<u8>::new());
        let bytes_sent = write_fetch_response(&mut output, &mut response).await?;

        assert_eq!(
            bytes_sent, size as u64,
            "returned byte count should equal input pack data length for size={size}"
        );
    }

    // Also verify that without_pack() returns 0
    {
        let mut response = AsyncFetchOutput::without_pack();
        response.acknowledgements.push(Acknowledgement::Nak);
        let mut output = Cursor::new(Vec::<u8>::new());
        let bytes_sent = write_fetch_response(&mut output, &mut response).await?;

        assert_eq!(
            bytes_sent, 0,
            "without_pack() should always return 0 bytes sent"
        );
    }

    Ok(())
}
