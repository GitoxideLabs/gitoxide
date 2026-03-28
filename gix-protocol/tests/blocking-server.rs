use gix_hash::ObjectId;
use gix_packetline::blocking_io::encode::{data_to_write, delim_to_write, flush_to_write};
use gix_packetline::blocking_io::StreamingPeekableIter;
use gix_packetline::PacketLineRef;
use gix_protocol::serve::upload_pack::ack::{write_ack, write_nak, AckStatus};
use gix_protocol::serve::upload_pack::want_haves::{parse_haves, parse_wants};
use gix_protocol::serve::upload_pack::{serve_upload_pack_v1, serve_upload_pack_v2};
use gix_protocol::serve::{write_capabilities_v2, write_v1, write_v2_ls_refs, RefAdvertisement};
use gix_transport::server::blocking_io::connection::Connection;
use gix_transport::{Protocol, Service};

fn read_data_line(reader: &mut StreamingPeekableIter<&[u8]>) -> Vec<u8> {
    match reader.read_line().unwrap().unwrap().unwrap() {
        PacketLineRef::Data(d) => d.to_vec(),
        other => panic!("expected data line, got {other:?}"),
    }
}

fn assert_flushed(reader: &mut StreamingPeekableIter<&[u8]>) {
    assert!(reader.read_line().is_none(), "expected flush/end of iteration");
}

fn hex_id(byte: u8) -> ObjectId {
    ObjectId::from([byte; 20])
}

#[test]
fn empty_refs_writes_null_oid_with_capabilities() {
    let mut out = Vec::new();
    write_v1(&mut out, &[], &["ofs-delta", "side-band-64k"]).unwrap();

    let mut reader = StreamingPeekableIter::new(&out[..], &[PacketLineRef::Flush], false);
    let line = read_data_line(&mut reader);
    let null_hex = "0000000000000000000000000000000000000000";
    let expected = format!("{null_hex} capabilities^{{}}\0ofs-delta side-band-64k\n");
    assert_eq!(line, expected.as_bytes());
    assert_flushed(&mut reader);
}

#[test]
fn single_ref_has_capabilities_on_first_line() {
    let oid = hex_id(0xaa);
    let refs = [RefAdvertisement {
        name: b"refs/heads/main",
        object_id: &oid,
        peeled: None,
        symref_target: None,
    }];
    let mut out = Vec::new();
    write_v1(&mut out, &refs, &["ofs-delta"]).unwrap();

    let mut reader = StreamingPeekableIter::new(&out[..], &[PacketLineRef::Flush], false);
    let line = read_data_line(&mut reader);
    let expected = format!("{} refs/heads/main\0ofs-delta\n", oid.to_hex());
    assert_eq!(line, expected.as_bytes());
    assert_flushed(&mut reader);
}

#[test]
fn multiple_refs_only_first_has_capabilities() {
    let oid1 = hex_id(0xaa);
    let oid2 = hex_id(0xbb);
    let refs = [
        RefAdvertisement {
            name: b"refs/heads/main",
            object_id: &oid1,
            peeled: None,
            symref_target: None,
        },
        RefAdvertisement {
            name: b"refs/heads/dev",
            object_id: &oid2,
            peeled: None,
            symref_target: None,
        },
    ];
    let mut out = Vec::new();
    write_v1(&mut out, &refs, &["ofs-delta"]).unwrap();

    let mut reader = StreamingPeekableIter::new(&out[..], &[PacketLineRef::Flush], false);

    let first = read_data_line(&mut reader);
    let expected_first = format!("{} refs/heads/main\0ofs-delta\n", oid1.to_hex());
    assert_eq!(first, expected_first.as_bytes());

    let second = read_data_line(&mut reader);
    let expected_second = format!("{} refs/heads/dev\n", oid2.to_hex());
    assert_eq!(second, expected_second.as_bytes());

    assert_flushed(&mut reader);
}

#[test]
fn peeled_tag_emits_caret_brace_line() {
    let tag_oid = hex_id(0xcc);
    let commit_oid = hex_id(0xdd);
    let refs = [RefAdvertisement {
        name: b"refs/tags/v1.0",
        object_id: &tag_oid,
        peeled: Some(&commit_oid),
        symref_target: None,
    }];
    let mut out = Vec::new();
    write_v1(&mut out, &refs, &["ofs-delta"]).unwrap();

    let mut reader = StreamingPeekableIter::new(&out[..], &[PacketLineRef::Flush], false);

    let tag_line = read_data_line(&mut reader);
    let expected_tag = format!("{} refs/tags/v1.0\0ofs-delta\n", tag_oid.to_hex());
    assert_eq!(tag_line, expected_tag.as_bytes());

    let peel_line = read_data_line(&mut reader);
    let expected_peel = format!("{} refs/tags/v1.0^{{}}\n", commit_oid.to_hex());
    assert_eq!(peel_line, expected_peel.as_bytes());

    assert_flushed(&mut reader);
}

#[test]
fn mixed_refs_and_peeled_tags() {
    let head_oid = hex_id(0xaa);
    let tag_oid = hex_id(0xbb);
    let commit_oid = hex_id(0xcc);
    let dev_oid = hex_id(0xdd);
    let refs = [
        RefAdvertisement {
            name: b"HEAD",
            object_id: &head_oid,
            peeled: None,
            symref_target: None,
        },
        RefAdvertisement {
            name: b"refs/tags/v1.0",
            object_id: &tag_oid,
            peeled: Some(&commit_oid),
            symref_target: None,
        },
        RefAdvertisement {
            name: b"refs/heads/dev",
            object_id: &dev_oid,
            peeled: None,
            symref_target: None,
        },
    ];
    let mut out = Vec::new();
    write_v1(&mut out, &refs, &["multi_ack", "thin-pack"]).unwrap();

    let mut reader = StreamingPeekableIter::new(&out[..], &[PacketLineRef::Flush], false);

    let line = read_data_line(&mut reader);
    assert_eq!(
        line,
        format!("{} HEAD\0multi_ack thin-pack\n", head_oid.to_hex()).as_bytes()
    );

    let line = read_data_line(&mut reader);
    assert_eq!(line, format!("{} refs/tags/v1.0\n", tag_oid.to_hex()).as_bytes());

    let line = read_data_line(&mut reader);
    assert_eq!(
        line,
        format!("{} refs/tags/v1.0^{{}}\n", commit_oid.to_hex()).as_bytes()
    );

    let line = read_data_line(&mut reader);
    assert_eq!(line, format!("{} refs/heads/dev\n", dev_oid.to_hex()).as_bytes());

    assert_flushed(&mut reader);
}

#[test]
fn symref_is_encoded_in_capabilities() {
    let head_oid = hex_id(0xaa);
    let main_oid = hex_id(0xaa);
    let refs = [
        RefAdvertisement {
            name: b"HEAD",
            object_id: &head_oid,
            peeled: None,
            symref_target: Some(b"refs/heads/main"),
        },
        RefAdvertisement {
            name: b"refs/heads/main",
            object_id: &main_oid,
            peeled: None,
            symref_target: None,
        },
    ];
    let mut out = Vec::new();
    write_v1(&mut out, &refs, &["ofs-delta"]).unwrap();

    let mut reader = StreamingPeekableIter::new(&out[..], &[PacketLineRef::Flush], false);

    let line = read_data_line(&mut reader);
    let expected = format!("{} HEAD\0symref=HEAD:refs/heads/main ofs-delta\n", head_oid.to_hex());
    assert_eq!(line, expected.as_bytes());

    let line = read_data_line(&mut reader);
    assert_eq!(line, format!("{} refs/heads/main\n", main_oid.to_hex()).as_bytes());

    assert_flushed(&mut reader);
}

// --- V2 ls-refs tests ---

#[test]
fn v2_ls_refs_single_ref() {
    let oid = hex_id(0xaa);
    let refs = [RefAdvertisement {
        name: b"refs/heads/main",
        object_id: &oid,
        peeled: None,
        symref_target: None,
    }];
    let mut out = Vec::new();
    write_v2_ls_refs(&mut out, &refs).unwrap();

    let mut reader = StreamingPeekableIter::new(&out[..], &[PacketLineRef::Flush], false);
    let line = read_data_line(&mut reader);
    assert_eq!(line, format!("{} refs/heads/main\n", oid.to_hex()).as_bytes());
    assert_flushed(&mut reader);
}

#[test]
fn v2_ls_refs_with_symref_and_peeled() {
    let head_oid = hex_id(0xaa);
    let tag_oid = hex_id(0xbb);
    let commit_oid = hex_id(0xcc);
    let refs = [
        RefAdvertisement {
            name: b"HEAD",
            object_id: &head_oid,
            peeled: None,
            symref_target: Some(b"refs/heads/main"),
        },
        RefAdvertisement {
            name: b"refs/tags/v1.0",
            object_id: &tag_oid,
            peeled: Some(&commit_oid),
            symref_target: None,
        },
    ];
    let mut out = Vec::new();
    write_v2_ls_refs(&mut out, &refs).unwrap();

    let mut reader = StreamingPeekableIter::new(&out[..], &[PacketLineRef::Flush], false);

    let line = read_data_line(&mut reader);
    assert_eq!(
        line,
        format!("{} HEAD symref-target:refs/heads/main\n", head_oid.to_hex()).as_bytes()
    );

    let line = read_data_line(&mut reader);
    assert_eq!(
        line,
        format!("{} refs/tags/v1.0 peeled:{}\n", tag_oid.to_hex(), commit_oid.to_hex()).as_bytes()
    );

    assert_flushed(&mut reader);
}

// --- V2 capabilities tests ---

#[test]
fn v2_capabilities_plain() {
    let mut out = Vec::new();
    write_capabilities_v2(&mut out, &[("ls-refs", None), ("fetch", None)]).unwrap();

    let mut reader = StreamingPeekableIter::new(&out[..], &[PacketLineRef::Flush], false);

    let line = read_data_line(&mut reader);
    assert_eq!(line, b"version 2\n");

    let line = read_data_line(&mut reader);
    assert_eq!(line, b"ls-refs\n");

    let line = read_data_line(&mut reader);
    assert_eq!(line, b"fetch\n");

    assert_flushed(&mut reader);
}

#[test]
fn v2_capabilities_with_values() {
    let mut out = Vec::new();
    write_capabilities_v2(
        &mut out,
        &[("ls-refs", None), ("fetch", Some("shallow")), ("server-option", None)],
    )
    .unwrap();

    let mut reader = StreamingPeekableIter::new(&out[..], &[PacketLineRef::Flush], false);

    let line = read_data_line(&mut reader);
    assert_eq!(line, b"version 2\n");

    let line = read_data_line(&mut reader);
    assert_eq!(line, b"ls-refs\n");

    let line = read_data_line(&mut reader);
    assert_eq!(line, b"fetch=shallow\n");

    let line = read_data_line(&mut reader);
    assert_eq!(line, b"server-option\n");

    assert_flushed(&mut reader);
}

// --- want/have parsing tests ---

fn write_want_have_input(lines: &[&str]) -> Vec<u8> {
    let mut buf = Vec::new();
    for line in lines {
        data_to_write(format!("{line}\n").as_bytes(), &mut buf).unwrap();
    }
    flush_to_write(&mut buf).unwrap();
    buf
}

#[test]
fn parse_wants_single_no_caps() {
    let oid = hex_id(0xaa);
    let input = write_want_have_input(&[&format!("want {}", oid.to_hex())]);
    let mut reader = StreamingPeekableIter::new(&input[..], &[PacketLineRef::Flush], false);

    let result = parse_wants(&mut reader).unwrap();
    assert_eq!(result.wants.len(), 1);
    assert_eq!(result.wants[0].id, oid);
    assert!(result.capabilities.is_empty());
}

#[test]
fn parse_wants_first_line_has_capabilities() {
    let oid1 = hex_id(0xaa);
    let oid2 = hex_id(0xbb);
    let input = write_want_have_input(&[
        &format!("want {} ofs-delta side-band-64k", oid1.to_hex()),
        &format!("want {}", oid2.to_hex()),
    ]);
    let mut reader = StreamingPeekableIter::new(&input[..], &[PacketLineRef::Flush], false);

    let result = parse_wants(&mut reader).unwrap();
    assert_eq!(result.wants.len(), 2);
    assert_eq!(result.wants[0].id, oid1);
    assert_eq!(result.wants[1].id, oid2);
    assert_eq!(result.capabilities, vec!["ofs-delta", "side-band-64k"]);
}

#[test]
fn parse_wants_ignores_caps_on_subsequent_lines() {
    let oid1 = hex_id(0xaa);
    let oid2 = hex_id(0xbb);
    let input = write_want_have_input(&[
        &format!("want {} cap1", oid1.to_hex()),
        &format!("want {} cap2", oid2.to_hex()),
    ]);
    let mut reader = StreamingPeekableIter::new(&input[..], &[PacketLineRef::Flush], false);

    let result = parse_wants(&mut reader).unwrap();
    assert_eq!(result.wants.len(), 2);
    assert_eq!(result.capabilities, vec!["cap1"]);
}

#[test]
fn parse_haves_with_done() {
    let oid1 = hex_id(0xaa);
    let oid2 = hex_id(0xbb);
    let mut input = Vec::new();
    data_to_write(format!("have {}\n", oid1.to_hex()).as_bytes(), &mut input).unwrap();
    data_to_write(format!("have {}\n", oid2.to_hex()).as_bytes(), &mut input).unwrap();
    data_to_write(b"done\n", &mut input).unwrap();
    flush_to_write(&mut input).unwrap();

    let mut reader = StreamingPeekableIter::new(&input[..], &[PacketLineRef::Flush], false);

    let result = parse_haves(&mut reader).unwrap();
    assert_eq!(result.haves.len(), 2);
    assert_eq!(result.haves[0], oid1);
    assert_eq!(result.haves[1], oid2);
    assert!(result.done);
}

#[test]
fn parse_haves_without_done_ends_at_flush() {
    let oid = hex_id(0xaa);
    let input = write_want_have_input(&[&format!("have {}", oid.to_hex())]);
    let mut reader = StreamingPeekableIter::new(&input[..], &[PacketLineRef::Flush], false);

    let result = parse_haves(&mut reader).unwrap();
    assert_eq!(result.haves.len(), 1);
    assert_eq!(result.haves[0], oid);
    assert!(!result.done);
}

#[test]
fn parse_haves_empty_round() {
    let mut input = Vec::new();
    flush_to_write(&mut input).unwrap();

    let mut reader = StreamingPeekableIter::new(&input[..], &[PacketLineRef::Flush], false);

    let result = parse_haves(&mut reader).unwrap();
    assert!(result.haves.is_empty());
    assert!(!result.done);
}

// --- ACK/NAK tests ---

#[test]
fn ack_common() {
    let oid = hex_id(0xaa);
    let mut out = Vec::new();
    write_ack(&mut out, &oid, AckStatus::Common).unwrap();

    let mut reader = StreamingPeekableIter::new(&out[..], &[PacketLineRef::Flush], false);
    let line = read_data_line(&mut reader);
    assert_eq!(line, format!("ACK {} common\n", oid.to_hex()).as_bytes());
}

#[test]
fn ack_ready() {
    let oid = hex_id(0xbb);
    let mut out = Vec::new();
    write_ack(&mut out, &oid, AckStatus::Ready).unwrap();

    let mut reader = StreamingPeekableIter::new(&out[..], &[PacketLineRef::Flush], false);
    let line = read_data_line(&mut reader);
    assert_eq!(line, format!("ACK {} ready\n", oid.to_hex()).as_bytes());
}

#[test]
fn ack_final() {
    let oid = hex_id(0xcc);
    let mut out = Vec::new();
    write_ack(&mut out, &oid, AckStatus::Final).unwrap();

    let mut reader = StreamingPeekableIter::new(&out[..], &[PacketLineRef::Flush], false);
    let line = read_data_line(&mut reader);
    assert_eq!(line, format!("ACK {}\n", oid.to_hex()).as_bytes());
}

#[test]
fn nak() {
    let mut out = Vec::new();
    write_nak(&mut out).unwrap();

    let mut reader = StreamingPeekableIter::new(&out[..], &[PacketLineRef::Flush], false);
    let line = read_data_line(&mut reader);
    assert_eq!(line, b"NAK\n");
}

// --- upload-pack orchestrator tests ---

fn build_client_input(wants: &[ObjectId], haves: &[ObjectId], done: bool) -> Vec<u8> {
    let mut buf = Vec::new();
    for (i, oid) in wants.iter().enumerate() {
        let line = if i == 0 {
            format!("want {} ofs-delta\n", oid.to_hex())
        } else {
            format!("want {}\n", oid.to_hex())
        };
        data_to_write(line.as_bytes(), &mut buf).unwrap();
    }
    flush_to_write(&mut buf).unwrap();
    for oid in haves {
        data_to_write(format!("have {}\n", oid.to_hex()).as_bytes(), &mut buf).unwrap();
    }
    if done {
        data_to_write(b"done\n", &mut buf).unwrap();
    }
    flush_to_write(&mut buf).unwrap();
    buf
}

#[test]
fn upload_pack_v1_fresh_clone() {
    let ref_oid = hex_id(0xaa);
    let input = build_client_input(&[ref_oid], &[], true);
    let mut output = Vec::new();

    let mut conn = Connection::new(
        &input[..],
        &mut output,
        Service::UploadPack,
        "/repo.git",
        Protocol::V1,
        false,
    );

    let refs = [RefAdvertisement {
        name: b"refs/heads/main",
        object_id: &ref_oid,
        peeled: None,
        symref_target: None,
    }];
    let mut pack_written = false;
    serve_upload_pack_v1(
        &mut conn,
        &refs,
        |_oid| false, // client has nothing
        |wants, common, _writer| {
            assert_eq!(wants.len(), 1);
            assert_eq!(wants[0], ref_oid);
            assert!(common.is_empty());
            pack_written = true;
            Ok(())
        },
        &["ofs-delta"],
    )
    .unwrap();

    assert!(pack_written);

    // Verify output: ref advertisement + NAK + NAK
    let mut reader = StreamingPeekableIter::new(&output[..], &[PacketLineRef::Flush], false);
    // First line: ref advertisement
    let line = read_data_line(&mut reader);
    assert_eq!(
        line,
        format!("{} refs/heads/main\0ofs-delta\n", ref_oid.to_hex()).as_bytes()
    );
    // Flush after ref advertisement
    assert_flushed(&mut reader);
}

#[test]
fn upload_pack_v1_with_common_objects() {
    let ref_oid = hex_id(0xaa);
    let common_oid = hex_id(0xbb);
    let input = build_client_input(&[ref_oid], &[common_oid], true);
    let mut output = Vec::new();

    let mut conn = Connection::new(
        &input[..],
        &mut output,
        Service::UploadPack,
        "/repo.git",
        Protocol::V1,
        false,
    );

    let refs = [RefAdvertisement {
        name: b"refs/heads/main",
        object_id: &ref_oid,
        peeled: None,
        symref_target: None,
    }];
    serve_upload_pack_v1(
        &mut conn,
        &refs,
        |oid| oid == common_oid, // server has this object
        |wants, common, _writer| {
            assert_eq!(wants, &[ref_oid]);
            assert_eq!(common, &[common_oid]);
            Ok(())
        },
        &["ofs-delta"],
    )
    .unwrap();
}

#[test]
fn upload_pack_v1_empty_wants_returns_early() {
    // Client sends no wants — up to date
    let mut input = Vec::new();
    flush_to_write(&mut input).unwrap(); // empty wants section
    let mut output = Vec::new();

    let ref_oid = hex_id(0xaa);
    let mut conn = Connection::new(
        &input[..],
        &mut output,
        Service::UploadPack,
        "/repo.git",
        Protocol::V1,
        false,
    );

    let refs = [RefAdvertisement {
        name: b"refs/heads/main",
        object_id: &ref_oid,
        peeled: None,
        symref_target: None,
    }];
    serve_upload_pack_v1(
        &mut conn,
        &refs,
        |_| false,
        |_, _, _| panic!("should not generate pack"),
        &["ofs-delta"],
    )
    .unwrap();
}

#[test]
fn upload_pack_v1_multi_round_negotiation() {
    let ref_oid = hex_id(0xaa);
    let have1 = hex_id(0xbb);
    let have2 = hex_id(0xcc);
    let have3 = hex_id(0xdd);

    // Build input: wants, then two rounds of haves, then done
    let mut input = Vec::new();
    // wants
    data_to_write(format!("want {} ofs-delta\n", ref_oid.to_hex()).as_bytes(), &mut input).unwrap();
    flush_to_write(&mut input).unwrap();
    // round 1: server doesn't have these
    data_to_write(format!("have {}\n", have1.to_hex()).as_bytes(), &mut input).unwrap();
    flush_to_write(&mut input).unwrap();
    // round 2: server has have2, doesn't have have3
    data_to_write(format!("have {}\n", have2.to_hex()).as_bytes(), &mut input).unwrap();
    data_to_write(format!("have {}\n", have3.to_hex()).as_bytes(), &mut input).unwrap();
    data_to_write(b"done\n", &mut input).unwrap();
    flush_to_write(&mut input).unwrap();

    let mut output = Vec::new();
    let mut conn = Connection::new(
        &input[..],
        &mut output,
        Service::UploadPack,
        "/repo.git",
        Protocol::V1,
        false,
    );

    let refs = [RefAdvertisement {
        name: b"refs/heads/main",
        object_id: &ref_oid,
        peeled: None,
        symref_target: None,
    }];
    serve_upload_pack_v1(
        &mut conn,
        &refs,
        |oid| oid == have2, // only have2 is common
        |wants, common, _writer| {
            assert_eq!(wants, &[ref_oid]);
            assert_eq!(common, &[have2]);
            Ok(())
        },
        &["ofs-delta"],
    )
    .unwrap();

    // Verify output after ref advertisement + flush:
    // Round 1: NAK (no common)
    // Round 2: ACK have2 common, then final ACK have2
    let mut reader = StreamingPeekableIter::new(&output[..], &[PacketLineRef::Flush], false);
    // ref advertisement
    let line = read_data_line(&mut reader);
    assert_eq!(
        line,
        format!("{} refs/heads/main\0ofs-delta\n", ref_oid.to_hex()).as_bytes()
    );
    assert_flushed(&mut reader);
    // reset to read past flush
    reader.reset();
    // round 1: NAK
    let line = read_data_line(&mut reader);
    assert_eq!(line, b"NAK\n");
    // round 2: ACK common
    let line = read_data_line(&mut reader);
    assert_eq!(line, format!("ACK {} common\n", have2.to_hex()).as_bytes());
    // final ACK
    let line = read_data_line(&mut reader);
    assert_eq!(line, format!("ACK {}\n", have2.to_hex()).as_bytes());
}

// --- V2 upload-pack orchestrator tests ---

/// Build V2 fetch command input: command=fetch + 0001 + wants + haves + done + 0000.
fn build_v2_fetch_input(wants: &[ObjectId], haves: &[ObjectId], done: bool) -> Vec<u8> {
    let mut buf = Vec::new();
    data_to_write(b"command=fetch\n", &mut buf).unwrap();
    delim_to_write(&mut buf).unwrap();
    for oid in wants {
        data_to_write(format!("want {}\n", oid.to_hex()).as_bytes(), &mut buf).unwrap();
    }
    for oid in haves {
        data_to_write(format!("have {}\n", oid.to_hex()).as_bytes(), &mut buf).unwrap();
    }
    if done {
        data_to_write(b"done\n", &mut buf).unwrap();
    }
    flush_to_write(&mut buf).unwrap();
    buf
}

/// Build V2 ls-refs command input: command=ls-refs + flush.
fn build_v2_ls_refs_input() -> Vec<u8> {
    let mut buf = Vec::new();
    data_to_write(b"command=ls-refs\n", &mut buf).unwrap();
    flush_to_write(&mut buf).unwrap();
    buf
}

#[test]
fn upload_pack_v2_ls_refs_command() {
    let ref_oid = hex_id(0xaa);
    let input = build_v2_ls_refs_input();
    let mut output = Vec::new();

    let mut conn = Connection::new(
        &input[..],
        &mut output,
        Service::UploadPack,
        "/repo.git",
        Protocol::V2,
        false,
    );

    let refs = [RefAdvertisement {
        name: b"refs/heads/main",
        object_id: &ref_oid,
        peeled: None,
        symref_target: None,
    }];
    serve_upload_pack_v2(
        &mut conn,
        &refs,
        |_| false,
        |_, _, _| panic!("should not generate pack for ls-refs"),
        &[("ls-refs", None), ("fetch", None)],
    )
    .unwrap();

    // Output: capabilities + ls-refs response
    let mut reader = StreamingPeekableIter::new(&output[..], &[PacketLineRef::Flush], false);
    // capabilities
    let line = read_data_line(&mut reader);
    assert_eq!(line, b"version 2\n");
    let line = read_data_line(&mut reader);
    assert_eq!(line, b"ls-refs\n");
    let line = read_data_line(&mut reader);
    assert_eq!(line, b"fetch\n");
    assert_flushed(&mut reader);
    reader.reset();
    // ls-refs response
    let line = read_data_line(&mut reader);
    assert_eq!(line, format!("{} refs/heads/main\n", ref_oid.to_hex()).as_bytes());
    assert_flushed(&mut reader);
}

#[test]
fn upload_pack_v2_fresh_clone() {
    let ref_oid = hex_id(0xaa);
    let input = build_v2_fetch_input(&[ref_oid], &[], true);
    let mut output = Vec::new();

    let mut conn = Connection::new(
        &input[..],
        &mut output,
        Service::UploadPack,
        "/repo.git",
        Protocol::V2,
        false,
    );

    let refs = [RefAdvertisement {
        name: b"refs/heads/main",
        object_id: &ref_oid,
        peeled: None,
        symref_target: None,
    }];
    let mut pack_written = false;
    serve_upload_pack_v2(
        &mut conn,
        &refs,
        |_| false,
        |wants, common, _writer| {
            assert_eq!(wants, &[ref_oid]);
            assert!(common.is_empty());
            pack_written = true;
            Ok(())
        },
        &[("ls-refs", None), ("fetch", None)],
    )
    .unwrap();

    assert!(pack_written);
}

#[test]
fn upload_pack_v2_fetch_with_common_objects() {
    let ref_oid = hex_id(0xaa);
    let common_oid = hex_id(0xbb);
    let input = build_v2_fetch_input(&[ref_oid], &[common_oid], true);
    let mut output = Vec::new();

    let mut conn = Connection::new(
        &input[..],
        &mut output,
        Service::UploadPack,
        "/repo.git",
        Protocol::V2,
        false,
    );

    let refs = [RefAdvertisement {
        name: b"refs/heads/main",
        object_id: &ref_oid,
        peeled: None,
        symref_target: None,
    }];
    serve_upload_pack_v2(
        &mut conn,
        &refs,
        |oid| oid == common_oid,
        |wants, common, _writer| {
            assert_eq!(wants, &[ref_oid]);
            assert_eq!(common, &[common_oid]);
            Ok(())
        },
        &[("ls-refs", None), ("fetch", None)],
    )
    .unwrap();

    // Verify output contains acknowledgments section with ACK
    let mut reader = StreamingPeekableIter::new(&output[..], &[PacketLineRef::Flush], false);
    // skip capabilities
    while reader.read_line().is_some() {}
    reader.reset();
    // acknowledgments section
    let line = read_data_line(&mut reader);
    assert_eq!(line, b"acknowledgments\n");
    let line = read_data_line(&mut reader);
    assert_eq!(line, format!("ACK {} common\n", common_oid.to_hex()).as_bytes());
    let line = read_data_line(&mut reader);
    assert_eq!(line, b"ready\n");
}
