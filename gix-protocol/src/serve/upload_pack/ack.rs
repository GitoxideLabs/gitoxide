use std::io::{self, Write};

use crate::transport::packetline::blocking_io::encode::data_to_write;

/// The status of an ACK response during negotiation.
pub enum AckStatus {
    /// The server has this object in common with the client.
    Common,
    /// The server has enough common objects and is ready to send the pack.
    Ready,
    /// Final ACK sent after the client's `done`, before the pack.
    Final,
}

/// Write an ACK line.
pub fn write_ack<W: Write>(writer: &mut W, id: &gix_hash::oid, status: AckStatus) -> io::Result<()> {
    let mut line = Vec::new();
    line.extend_from_slice(b"ACK ");
    id.write_hex_to(&mut line)?;
    match status {
        AckStatus::Common => {
            line.extend_from_slice(b" common");
        }
        AckStatus::Ready => {
            line.extend_from_slice(b" ready");
        }
        AckStatus::Final => {}
    }
    line.push(b'\n');
    data_to_write(&line, &mut *writer)?;
    Ok(())
}

/// Write a NAK line.
pub fn write_nak<W: Write>(writer: &mut W) -> io::Result<()> {
    data_to_write(b"NAK\n", &mut *writer)?;
    Ok(())
}
