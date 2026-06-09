//! Receive-pack protocol-contract tests backed by real `git push` client captures.
//!
//! On the currently tested git client (`git/2.39.5`), push requests are framed as V1 command
//! sections (including optional push-options) even when protocol v2 is requested globally.
use std::io::{BufRead as _, BufReader, Cursor, Read as _};

use bstr::ByteSlice;
use gix_protocol::receive_pack::{self, RefStatus, Request, Response, UnpackStatus};
use gix_transport::packetline::{BandRef, PacketLineRef, blocking_io::StreamingPeekableIter};

#[derive(Default)]
struct RecordingDelegate {
    response: Response,
    seen_request: Option<Request>,
    seen_pack_prefix: Option<[u8; 4]>,
}

impl receive_pack::Delegate for RecordingDelegate {
    fn receive(
        &mut self,
        request: &Request,
        pack_data: &mut dyn std::io::Read,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync + 'static>> {
        self.seen_request = Some(request.clone());

        let mut prefix = [0u8; 4];
        match pack_data.read_exact(&mut prefix) {
            Ok(()) => self.seen_pack_prefix = Some(prefix),
            Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => self.seen_pack_prefix = None,
            Err(err) => return Err(err.into()),
        }

        Ok(self.response.clone())
    }
}

#[test]
fn parse_v1_request_from_real_push_basic_transcript() -> crate::Result {
    let mut input = BufReader::new(Cursor::new(fixture_request("push-basic.request")));
    let request = receive_pack::parse_v1_request(&mut input)?;

    assert_eq!(request.updates.len(), 1);
    assert_eq!(request.push_options.len(), 0);
    assert!(request.has_capability("report-status-v2"));
    assert!(request.has_capability("side-band-64k"));
    assert!(
        request.capabilities.iter().any(
            |capability| capability.name.as_bstr() == "object-format".as_bytes().as_bstr()
                && capability.value.as_ref().map(|value| value.as_bstr()) == Some("sha1".as_bytes().as_bstr())
        ),
        "real client transcript should include object-format=sha1"
    );
    assert!(
        request
            .capabilities
            .iter()
            .any(|capability| capability.name.as_bstr() == "agent".as_bytes().as_bstr()
                && capability
                    .value
                    .as_ref()
                    .is_some_and(|value| value.as_bstr().as_bytes().starts_with(b"git/"))),
        "real client transcript should include agent=git/<version>"
    );
    assert_eq!(
        request.updates[0].old_id.to_string(),
        "0000000000000000000000000000000000000000"
    );
    assert_eq!(
        request.updates[0].new_id.to_string(),
        "477eab3a52feaff241c8797fde5454349f125087"
    );
    assert_eq!(
        request.updates[0].ref_name.as_bstr(),
        "refs/heads/main".as_bytes().as_bstr()
    );

    let mut pack_prefix = [0u8; 4];
    input.read_exact(&mut pack_prefix)?;
    assert_eq!(pack_prefix, *b"PACK", "pack bytes should follow the command section");
    Ok(())
}

#[test]
fn parse_v1_request_from_real_push_with_option_transcript() -> crate::Result {
    let mut input = BufReader::new(Cursor::new(fixture_request("push-with-option.request")));
    let request = receive_pack::parse_v1_request(&mut input)?;

    assert_eq!(request.updates.len(), 1);
    assert!(request.has_capability("push-options"));
    assert_eq!(
        request.push_options,
        vec![bstr::BString::from("trace=1")],
        "client-sent push-options section should be preserved"
    );
    assert_eq!(
        request.updates[0].old_id.to_string(),
        "477eab3a52feaff241c8797fde5454349f125087"
    );
    assert_eq!(
        request.updates[0].new_id.to_string(),
        "2f189a1d2c8b843619bcd29aafaa95aa0bfeb4bd"
    );

    let mut pack_prefix = [0u8; 4];
    input.read_exact(&mut pack_prefix)?;
    assert_eq!(pack_prefix, *b"PACK");
    Ok(())
}

#[test]
fn parse_v1_request_from_real_delete_transcript_has_no_pack() -> crate::Result {
    let mut input = BufReader::new(Cursor::new(fixture_request("delete-main.request")));
    let request = receive_pack::parse_v1_request(&mut input)?;

    assert_eq!(request.updates.len(), 1);
    assert_eq!(request.push_options.len(), 0);
    assert_eq!(
        request.updates[0].old_id.to_string(),
        "2f189a1d2c8b843619bcd29aafaa95aa0bfeb4bd"
    );
    assert_eq!(
        request.updates[0].new_id.to_string(),
        "0000000000000000000000000000000000000000"
    );
    assert!(
        input.fill_buf()?.is_empty(),
        "delete transcript should not contain pack data"
    );
    Ok(())
}

#[test]
fn serve_v1_from_real_push_transcript_writes_sideband_report_status() -> crate::Result {
    let request = fixture_request("push-basic.request");
    let mut output = Vec::new();
    let mut delegate = RecordingDelegate {
        response: Response {
            unpack_status: UnpackStatus::Ok,
            ref_statuses: vec![RefStatus::Ok {
                ref_name: "refs/heads/main".into(),
            }],
            sideband_messages: Vec::new(),
        },
        ..Default::default()
    };

    let outcome = receive_pack::serve_v1(request.as_slice(), &mut output, &mut delegate)?;
    assert_eq!(outcome.updates_received, 1);
    assert_eq!(outcome.push_options_received, 0);
    assert_eq!(outcome.ref_statuses_sent, 1);
    assert!(outcome.report_status_sent);
    assert!(outcome.sideband_bytes_sent > 0);
    assert_eq!(delegate.seen_pack_prefix, Some(*b"PACK"));

    let mut sideband_reader = StreamingPeekableIter::new(output.as_slice(), &[PacketLineRef::Flush], false);
    let mut report_status_payload = Vec::<u8>::new();
    while let Some(line) = sideband_reader.read_line() {
        let line = line??;
        match line.decode_band()? {
            BandRef::Data(data) => report_status_payload.extend_from_slice(data),
            BandRef::Progress(_) | BandRef::Error(_) => {}
        }
    }
    assert_eq!(sideband_reader.stopped_at(), Some(PacketLineRef::Flush));

    let mut report_status_reader =
        StreamingPeekableIter::new(report_status_payload.as_slice(), &[PacketLineRef::Flush], false);
    assert_eq!(
        next_text_line(&mut report_status_reader)?.as_bstr(),
        "unpack ok".as_bytes().as_bstr()
    );
    assert_eq!(
        next_text_line(&mut report_status_reader)?.as_bstr(),
        "ok refs/heads/main".as_bytes().as_bstr()
    );
    assert!(report_status_reader.read_line().is_none());
    assert_eq!(report_status_reader.stopped_at(), Some(PacketLineRef::Flush));
    Ok(())
}

fn fixture_request(name: &str) -> Vec<u8> {
    crate::fixture_bytes(&format!("receive-pack/v1/{name}"))
}

fn next_text_line(reader: &mut StreamingPeekableIter<&[u8]>) -> Result<bstr::BString, Box<dyn std::error::Error>> {
    let line = reader
        .read_line()
        .expect("expected packetline")
        .expect("read should succeed")
        .expect("decode should succeed");
    Ok(line.as_text().expect("expected text packetline").as_bstr().to_owned())
}
