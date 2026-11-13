use std::io::{self, Read, Write};

use gix_packetline::{
    PacketLineRef,
    decode::{self, PacketLineOrWantedSize},
};

const MAX_PACKET_DATA: usize = 65_516;

use gix_macros::sync as maybe_async;
fn write_packet(output: &mut impl Write, data: Option<&[u8]>) -> io::Result<()> {
    match data {
        Some(data) => gix_packetline::blocking_io::encode::data_to_write(data, output).map(|_| ()),
        None => gix_packetline::blocking_io::encode::flush_to_write(output).map(|_| ()),
    }
}
include!("../message.rs");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_payloads_and_large_replies_roundtrip() -> io::Result<()> {
        for payload in [
            b"gix:epoch:1\0a\0b/\0".to_vec(),
            vec![b'x'; MAX_PACKET_DATA * 2 + 1],
            Vec::new(),
        ] {
            let mut wire = Vec::new();
            write_message(&mut wire, &payload)?;
            assert!(wire.ends_with(b"0000"), "Simple IPC messages terminate with a flush");
            assert_eq!(
                read_message(&mut wire.as_slice(), payload.len())?,
                Some(payload),
                "pkt-line fragmentation preserves NUL payloads"
            );
        }
        Ok(())
    }

    #[test]
    fn probes_incomplete_frames_and_oversized_requests_are_distinct() -> io::Result<()> {
        assert_eq!(
            read_message(&mut &b""[..], 100)?,
            None,
            "native status probes only connect and close"
        );
        assert_eq!(
            read_message(&mut &b"0000"[..], 100)?,
            Some(Vec::new()),
            "an empty query is a real message"
        );
        assert!(
            read_message(&mut &b"0005x"[..], 100).is_err(),
            "a response without final flush is incomplete"
        );
        assert!(
            read_message(&mut &b"0005x0000"[..], 0).is_err(),
            "limits apply before payload allocation"
        );
        assert!(
            read_message(&mut &b"0001"[..], 100).is_err(),
            "Git protocol section delimiters are not Simple IPC framing"
        );
        Ok(())
    }
}
