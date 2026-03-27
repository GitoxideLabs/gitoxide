use std::io::{self, Read, Write};

use gix_hash::ObjectId;

use bstr::ByteSlice;

use crate::serve::{
    upload_pack::{parse_haves, parse_wants, write_ack, write_nak, AckStatus, Error},
    write_capabilities_v2, write_v1, write_v2_ls_refs, RefAdvertisement,
};
use crate::transport::{
    packetline::{
        blocking_io::encode::{data_to_write, delim_to_write, flush_to_write},
        PacketLineRef,
    },
    server::blocking_io::connection::Connection,
};
use crate::Command;

/// Serve a V1 upload-pack session.
pub fn serve_upload_pack_v1<R: Read, W: Write>(
    connection: &mut Connection<R, W>,
    refs: &[RefAdvertisement<'_>],
    has_object: impl Fn(&gix_hash::oid) -> bool,
    generate_pack: impl FnOnce(&[ObjectId], &[ObjectId], &mut dyn Write) -> io::Result<()>,
    capabilities: &[&str],
) -> Result<(), Error> {
    write_v1(&mut connection.writer, refs, capabilities)?;

    let wants = parse_wants(&mut connection.line_provider)?;
    if wants.wants.is_empty() {
        return Ok(());
    }

    connection.line_provider.reset();

    let mut common = Vec::new();
    loop {
        let haves = parse_haves(&mut connection.line_provider)?;
        let mut found_common = false;
        for oid in &haves.haves {
            if has_object(oid) {
                write_ack(&mut connection.writer, oid, AckStatus::Common)?;
                common.push(*oid);
                found_common = true;
            }
        }

        if haves.done {
            break;
        }

        if !found_common {
            write_nak(&mut connection.writer)?;
        }
        connection.line_provider.reset();
    }

    if let Some(last) = common.last() {
        write_ack(&mut connection.writer, last, AckStatus::Final)?;
    } else {
        write_nak(&mut connection.writer)?;
    }

    let want_ids: Vec<ObjectId> = wants.wants.iter().map(|w| w.id).collect();
    generate_pack(&want_ids, &common, &mut connection.writer)?;

    Ok(())
}

/// Serve a V2 upload-pack session.
pub fn serve_upload_pack_v2<R: Read, W: Write>(
    connection: &mut Connection<R, W>,
    refs: &[RefAdvertisement<'_>],
    has_object: impl Fn(&gix_hash::oid) -> bool,
    generate_pack: impl FnOnce(&[ObjectId], &[ObjectId], &mut dyn Write) -> io::Result<()>,
    capabilities: &[(&str, Option<&str>)],
) -> Result<(), Error> {
    write_capabilities_v2(&mut connection.writer, capabilities)?;

    loop {
        connection.line_provider.reset();

        let line = match connection.line_provider.read_line() {
            Some(Ok(line)) => line?,
            Some(Err(e)) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Some(Err(e)) => return Err(e.into()),
            None => break, // connection closed
        };
        let command = match line {
            PacketLineRef::Data(data) => parse_command(data)?,
            _ => break,
        };

        match command {
            Command::LsRefs => {
                while let Some(line) = connection.line_provider.read_line() {
                    let _ = line??;
                }
                write_v2_ls_refs(&mut connection.writer, refs)?;
            }
            Command::Fetch => {
                // V2 sends all arguments in one flush-terminated group.
                let mut want_ids = Vec::new();
                let mut have_ids = Vec::new();
                let mut done = false;

                while let Some(line) = connection.line_provider.read_line() {
                    let line = line??;
                    match line {
                        PacketLineRef::Data(data) => {
                            let data = data.trim();
                            if let Some(hex) = data.strip_prefix(b"want ") {
                                let id =
                                    ObjectId::from_hex(hex).map_err(|_| Error::UnexpectedLine { line: hex.into() })?;
                                want_ids.push(id);
                            } else if let Some(hex) = data.strip_prefix(b"have ") {
                                let id =
                                    ObjectId::from_hex(hex).map_err(|_| Error::UnexpectedLine { line: hex.into() })?;
                                have_ids.push(id);
                            } else if data == b"done" {
                                done = true;
                            }
                            // Skip unknown lines (capabilities like thin-pack, ofs-delta).
                        }
                        PacketLineRef::Delimiter => {}
                        _ => break,
                    }
                }

                data_to_write(b"acknowledgments\n", &mut connection.writer)?;
                let mut common = Vec::new();
                for oid in &have_ids {
                    if has_object(oid) {
                        write_ack(&mut connection.writer, oid, AckStatus::Common)?;
                        common.push(*oid);
                    }
                }
                if common.is_empty() {
                    write_nak(&mut connection.writer)?;
                }

                if !done {
                    flush_to_write(&mut connection.writer)?;
                    continue;
                }

                data_to_write(b"ready\n", &mut connection.writer)?;
                delim_to_write(&mut connection.writer)?;

                data_to_write(b"packfile\n", &mut connection.writer)?;
                generate_pack(&want_ids, &common, &mut connection.writer)?;
                flush_to_write(&mut connection.writer)?;
                break;
            }
        }
    }

    Ok(())
}

fn parse_command(data: &[u8]) -> Result<Command, Error> {
    let name = data
        .trim()
        .strip_prefix(b"command=")
        .ok_or_else(|| Error::UnexpectedLine { line: data.into() })?;
    match name {
        b"ls-refs" => Ok(Command::LsRefs),
        b"fetch" => Ok(Command::Fetch),
        _ => Err(Error::UnexpectedLine { line: data.into() }),
    }
}
