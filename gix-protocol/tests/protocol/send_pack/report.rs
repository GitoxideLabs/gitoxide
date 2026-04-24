use bstr::{BString, ByteSlice};
use gix_packetline::{blocking_io::StreamingPeekableIter, PacketLineRef};
use gix_protocol::send_pack::report::parse;

#[test]
fn unpack_ok_single_ref_ok() {
    // pkt-line frames: "000eunpack ok\n", "0017ok refs/heads/main\n", "0000"
    // "unpack ok\n" = 10 bytes, total = 14 = 0x0e => 000e
    // "ok refs/heads/main\n" = 19 bytes, total = 23 = 0x17 => 0017
    let bytes: &[u8] = b"\
000eunpack ok\n\
0017ok refs/heads/main\n\
0000";
    let report = parse(&mut &bytes[..]).expect("ok");
    assert_eq!(report.unpack, Ok(()));
    assert_eq!(report.refs.len(), 1);
    assert_eq!(report.refs[0].refname, BString::from("refs/heads/main"));
    assert_eq!(report.refs[0].result, Ok(()));
}

#[test]
fn unpack_ok_one_ref_rejected() {
    // "ng refs/heads/main non-fast-forward\n" = 36 bytes, total = 40 = 0x28 => 0028
    let bytes: &[u8] = b"\
000eunpack ok\n\
0028ng refs/heads/main non-fast-forward\n\
0000";
    let report = parse(&mut &bytes[..]).expect("ok");
    assert_eq!(report.unpack, Ok(()));
    assert_eq!(report.refs.len(), 1);
    let r = &report.refs[0];
    assert_eq!(r.refname, BString::from("refs/heads/main"));
    assert_eq!(r.result.as_ref().unwrap_err().as_bstr(), b"non-fast-forward".as_bstr());
}

#[test]
fn unpack_error_propagates() {
    // "unpack index-pack abort\n" = 24 bytes, total = 28 = 0x1c => 001c
    let bytes: &[u8] = b"\
001cunpack index-pack abort\n\
0000";
    let report = parse(&mut &bytes[..]).expect("ok");
    assert_eq!(
        report.unpack.as_ref().unwrap_err().as_bstr(),
        b"index-pack abort".as_bstr()
    );
    assert!(report.refs.is_empty());
}

#[test]
fn multiple_refs_mixed() {
    // "ok refs/heads/main\n" = 19 bytes, total = 23 = 0x17 => 0017
    // "ng refs/heads/dev fetch-first\n" = 30 bytes, total = 34 = 0x22 => 0022
    let bytes: &[u8] = b"\
000eunpack ok\n\
0017ok refs/heads/main\n\
0022ng refs/heads/dev fetch-first\n\
0000";
    let report = parse(&mut &bytes[..]).expect("ok");
    assert_eq!(report.refs.len(), 2);
    assert_eq!(report.refs[0].result, Ok(()));
    assert_eq!(
        report.refs[1].result.as_ref().unwrap_err().as_bstr(),
        b"fetch-first".as_bstr()
    );
}

#[test]
fn parses_captured_empty_to_new_branch_report() {
    // The s2c stream is: [ref-adv pkt-lines] [flush] [sideband pkt-line(s)] [flush].
    // The report pkt-lines arrive inside sideband band-1 frames (byte prefix \x01).
    // skip_past_first_flush positions us at the sideband portion.
    // We wrap that slice with WithSidebands so band-1 content is yielded as data
    // before passing to parse().
    let s2c = std::fs::read("tests/fixtures/push/empty-to-new-branch.s2c.bin").expect("fixture present");

    let tail = skip_past_first_flush(&s2c).expect("fixture has at least one flush");

    // The tail is a pkt-line stream with sideband framing — the entire report
    // is packed into band-1 of a single outer pkt-line.  Wrap with
    // WithSidebands (with a no-op progress handler) to demux the band before
    // parse() sees the inner pkt-lines.
    let mut iter = StreamingPeekableIter::new(tail, &[PacketLineRef::Flush], false);
    let mut sideband = iter.as_read_with_sidebands(|_is_err, _text| std::ops::ControlFlow::Continue(()));
    let report = parse(&mut sideband).expect("parse report");
    assert_eq!(report.unpack, Ok(()));
    assert_eq!(report.refs.len(), 1, "refs: {:?}", report.refs);
    assert_eq!(report.refs[0].refname, BString::from("refs/heads/main"));
    assert_eq!(report.refs[0].result, Ok(()));
}

fn skip_past_first_flush(buf: &[u8]) -> Option<&[u8]> {
    let mut i = 0;
    while i + 4 <= buf.len() {
        let len = usize::from_str_radix(std::str::from_utf8(&buf[i..i + 4]).ok()?, 16).ok()?;
        if len == 0 {
            return Some(&buf[i + 4..]);
        }
        if len < 4 || i + len > buf.len() {
            return None;
        }
        i += len;
    }
    None
}
