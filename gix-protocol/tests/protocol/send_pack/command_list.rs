use bstr::{BString, ByteSlice};
use gix_hash::ObjectId;
use gix_protocol::send_pack::{command_list::encode_into, Command, Request};

fn oid(hex: &str) -> ObjectId {
    // Accepts a short spec; pads with zeros to 40 chars (sha1 tests).
    let mut s = hex.to_string();
    while s.len() < 40 {
        s.push('0');
    }
    ObjectId::from_hex(s.as_bytes()).expect("valid oid")
}

#[test]
fn single_create_with_capabilities() {
    let req = Request {
        commands: vec![Command {
            refname: BString::from("refs/heads/main"),
            old_oid: ObjectId::null(gix_hash::Kind::Sha1),
            new_oid: oid("abcdef"),
        }],
        capabilities: vec![
            BString::from("report-status"),
            BString::from("side-band-64k"),
            BString::from("agent=gix/test"),
        ],
    };
    let mut out = Vec::new();
    encode_into(&req, gix_hash::Kind::Sha1, &mut out).unwrap();

    // Expect one pkt-line (length prefix + payload), then flush (0000).
    // Payload: "<zero-oid> <new-oid> refs/heads/main\0 report-status side-band-64k agent=gix/test"
    // Note the leading SP after NUL — git's send-pack always prefixes each
    // capability with a space (cap_buf starts with " report-status", …).
    // No trailing LF — git's send-pack omits it despite the ABNF annotation.
    let payload_text = format!(
        "{zero} {new} refs/heads/main\0 report-status side-band-64k agent=gix/test",
        zero = "0".repeat(40),
        new = "abcdef".to_owned() + &"0".repeat(34),
    );
    let expected_len = 4 + payload_text.len();
    let len_hex = format!("{expected_len:04x}");

    let mut expected = Vec::new();
    expected.extend_from_slice(len_hex.as_bytes());
    expected.extend_from_slice(payload_text.as_bytes());
    expected.extend_from_slice(b"0000");

    assert_eq!(out, expected, "actual: {:?}", out.as_bstr());
}

#[test]
fn multiple_commands_only_first_carries_caps() {
    let req = Request {
        commands: vec![
            Command {
                refname: BString::from("refs/heads/a"),
                old_oid: ObjectId::null(gix_hash::Kind::Sha1),
                new_oid: oid("aa"),
            },
            Command {
                refname: BString::from("refs/heads/b"),
                old_oid: ObjectId::null(gix_hash::Kind::Sha1),
                new_oid: oid("bb"),
            },
        ],
        capabilities: vec![BString::from("report-status")],
    };
    let mut out = Vec::new();
    encode_into(&req, gix_hash::Kind::Sha1, &mut out).unwrap();
    let s = out.as_bstr();
    // First pkt payload contains NUL + capabilities; second does not.
    assert_eq!(s.iter().filter(|&&b| b == 0).count(), 1);
    assert!(s.ends_with(b"0000"));
}

#[test]
fn delete_only_uses_zero_new_oid() {
    let req = Request {
        commands: vec![Command {
            refname: BString::from("refs/heads/gone"),
            old_oid: oid("dd"),
            new_oid: ObjectId::null(gix_hash::Kind::Sha1),
        }],
        capabilities: vec![BString::from("report-status")],
    };
    let mut out = Vec::new();
    encode_into(&req, gix_hash::Kind::Sha1, &mut out).unwrap();
    assert!(out
        .as_bstr()
        .contains_str(format!(" {} refs/heads/gone", "0".repeat(40))));
}

#[test]
fn matches_captured_empty_to_new_branch_fixture() {
    // The captured c2s stream starts with the client's command list
    // (pkt-line framed: "<hexlen>0<zero-oid> <new-oid> refs/heads/main\0<caps>\n")
    // followed by a flush ("0000"), then the pack ("PACK...").
    // We compare only the command-list prefix through (and including) the flush.
    let c2s = std::fs::read("tests/fixtures/push/empty-to-new-branch.c2s.bin")
        .expect("fixture present — run gix-protocol/tests/fixtures/push/capture.sh");

    let flush_end = find_command_list_end(&c2s).expect("fixture contains a flush-pkt");
    let fixture_cmd_list = &c2s[..flush_end];

    // Parse the fixture to recover the new-oid and capability list,
    // then rebuild with encode_into and compare byte-for-byte.
    let parsed = parse_single_create(fixture_cmd_list);

    let req = Request {
        commands: vec![Command {
            refname: BString::from("refs/heads/main"),
            old_oid: ObjectId::null(gix_hash::Kind::Sha1),
            new_oid: parsed.new_oid,
        }],
        capabilities: parsed.capabilities,
    };
    let mut ours = Vec::new();
    encode_into(&req, gix_hash::Kind::Sha1, &mut ours).expect("encode");

    assert_eq!(
        ours.as_bstr(),
        fixture_cmd_list.as_bstr(),
        "our encoding diverges from captured git client"
    );
}

/// Find the index just after the first flush-pkt ("0000") in `buf`.
/// Returns `Some(flush_idx + 4)` — the start of whatever follows the flush.
fn find_command_list_end(buf: &[u8]) -> Option<usize> {
    // The command list consists of pkt-lines (4-hex-length-prefixed), terminated
    // by a flush-pkt ("0000"). Walk pkt-lines until we see a flush.
    let mut i = 0;
    while i + 4 <= buf.len() {
        let len_str = std::str::from_utf8(&buf[i..i + 4]).ok()?;
        let len = usize::from_str_radix(len_str, 16).ok()?;
        if len == 0 {
            return Some(i + 4); // flush-pkt end
        }
        if len < 4 || i + len > buf.len() {
            return None; // malformed
        }
        i += len;
    }
    None
}

struct ParsedCreate {
    new_oid: ObjectId,
    capabilities: Vec<BString>,
}

/// Parse a single-pkt-line command-list into its new-oid and capability list.
///
/// Expects: `<hexlen><old-oid> <new-oid> <refname>\0 <cap1> <cap2>...`
///
/// git's send-pack always prefixes each capability with a space, so the
/// capabilities string after NUL starts with a leading space.  We split on
/// space and skip the empty token that arises from that leading space.
/// No trailing LF is expected — git's send-pack omits it.
fn parse_single_create(cmd_list: &[u8]) -> ParsedCreate {
    let len = usize::from_str_radix(std::str::from_utf8(&cmd_list[..4]).expect("hex length"), 16).expect("valid hex");
    let payload = &cmd_list[4..len]; // no trailing LF
    let nul = payload.iter().position(|&b| b == 0).expect("NUL separator");
    let head = &payload[..nul];
    let caps_line = &payload[nul + 1..];
    let mut parts = head.split(|&b| b == b' ');
    let _old = parts.next().expect("old-oid");
    let new = parts.next().expect("new-oid");
    ParsedCreate {
        new_oid: ObjectId::from_hex(new).expect("valid new-oid in fixture"),
        // caps_line starts with a leading space (git's wire format), so
        // splitting on space yields an empty token first — skip it.
        capabilities: caps_line
            .split(|&b| b == b' ')
            .filter(|c| !c.is_empty())
            .map(BString::from)
            .collect(),
    }
}
