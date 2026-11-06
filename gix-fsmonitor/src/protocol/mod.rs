use std::io::{self, Read, Write};

use gix_packetline::{
    PacketLineRef,
    decode::{self, PacketLineOrWantedSize},
};

const MAX_PACKET_DATA: usize = 65_516;

/// Read one Simple IPC message. A connect-and-close probe has no message at all.
pub(crate) fn read_message(input: &mut impl Read, limit: usize) -> io::Result<Option<Vec<u8>>> {
    let mut message = Vec::new();
    let mut received_packet = false;
    loop {
        let mut header = [0; 4];
        match input.read(&mut header[..1])? {
            0 if !received_packet => return Ok(None),
            0 => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "IPC message ended before flush",
                ));
            }
            _ => input.read_exact(&mut header[1..])?,
        }
        received_packet = true;
        match decode::hex_prefix(&header).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))? {
            PacketLineOrWantedSize::Line(PacketLineRef::Flush) => return Ok(Some(message)),
            PacketLineOrWantedSize::Line(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "unsupported IPC packet delimiter",
                ));
            }
            PacketLineOrWantedSize::Wanted(length) => {
                let length = usize::from(length);
                if length > MAX_PACKET_DATA || message.len().saturating_add(length) > limit {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "IPC message exceeds limit"));
                }
                let offset = message.len();
                message.resize(offset + length, 0);
                input.read_exact(&mut message[offset..])?;
            }
        }
    }
}

pub(crate) fn write_message(output: &mut impl Write, message: &[u8]) -> io::Result<()> {
    for chunk in message.chunks(MAX_PACKET_DATA) {
        gix_packetline::blocking_io::encode::data_to_write(chunk, &mut *output)?;
    }
    gix_packetline::blocking_io::encode::flush_to_write(&mut *output)?;
    output.flush()
}

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
