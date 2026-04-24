//! Parser for the `report-status` response (receive-pack → send-pack).
//!
//! Grammar (simplified from `Documentation/gitprotocol-pack.adoc`):
//!
//! ```text
//! report-status     =  PKT-LINE("unpack" SP ("ok" / error-msg) LF)
//!                      *command-status
//!                      flush-pkt
//! command-status    =  PKT-LINE("ok" SP refname LF) /
//!                      PKT-LINE("ng" SP refname SP error-msg LF)
//! ```
//!
//! Reference: `vendor/git/send-pack.c::receive_status()`.

use bstr::BString;

use crate::send_pack::{Error, RefStatus, Report};

/// Parse a `report-status` pkt-line stream from `input`.
///
/// The stream must be positioned after any sideband demux — `input` yields
/// raw report bytes as pkt-lines. Terminates on the flush-pkt (`0000`).
pub fn parse(input: &mut impl std::io::Read) -> Result<Report, Error> {
    use gix_transport::packetline::{blocking_io::StreamingPeekableIter, PacketLineRef};

    let mut reader = StreamingPeekableIter::new(
        input,
        &[PacketLineRef::Flush],
        false, // trace
    );

    let unpack = match reader.read_line() {
        Some(Ok(Ok(line))) => {
            let data = line.as_slice().ok_or(Error::UnexpectedEof)?;
            parse_unpack(data)?
        }
        Some(Ok(Err(_)) | Err(_)) => return Err(Error::UnexpectedEof),
        None => return Err(Error::UnexpectedEof),
    };

    let mut refs = Vec::new();
    while let Some(item) = reader.read_line() {
        let line = match item {
            Ok(Ok(l)) => l,
            Ok(Err(_)) => break, // decode error — treat as done
            Err(_) => return Err(Error::UnexpectedEof),
        };
        let data = line.as_slice().ok_or(Error::UnexpectedEof)?;
        refs.push(parse_command_status(data)?);
    }

    Ok(Report { unpack, refs })
}

fn trim_lf(line: &[u8]) -> &[u8] {
    if line.last().copied() == Some(b'\n') {
        &line[..line.len() - 1]
    } else {
        line
    }
}

fn parse_unpack(line: &[u8]) -> Result<Result<(), BString>, Error> {
    let line = trim_lf(line);
    let rest = line.strip_prefix(b"unpack ").ok_or_else(|| Error::MalformedReport {
        line: BString::from(line),
    })?;
    if rest == b"ok" {
        Ok(Ok(()))
    } else {
        Ok(Err(BString::from(rest)))
    }
}

fn parse_command_status(line: &[u8]) -> Result<RefStatus, Error> {
    let line = trim_lf(line);
    if let Some(rest) = line.strip_prefix(b"ok ") {
        Ok(RefStatus {
            refname: BString::from(rest),
            result: Ok(()),
        })
    } else if let Some(rest) = line.strip_prefix(b"ng ") {
        let sp = rest
            .iter()
            .position(|&b| b == b' ')
            .ok_or_else(|| Error::MalformedReport {
                line: BString::from(line),
            })?;
        let (refname, reason) = rest.split_at(sp);
        let reason = &reason[1..]; // strip the leading space
        Ok(RefStatus {
            refname: BString::from(refname),
            result: Err(BString::from(reason)),
        })
    } else {
        Err(Error::MalformedReport {
            line: BString::from(line),
        })
    }
}
