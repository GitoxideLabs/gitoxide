//! Encode the send-pack command list into pkt-line frames.
//!
//! Grammar (ABNF, from `Documentation/gitprotocol-pack.adoc`):
//!
//! ```text
//! update-request  =  *shallow command-list [pack-file]
//! command-list    =  PKT-LINE(command NUL capability-list)
//!                    *PKT-LINE(command)
//!                    flush-pkt
//! command         =  create / delete / update
//! create          =  zero-id SP new-id SP name
//! delete          =  old-id SP zero-id SP name
//! update          =  old-id SP new-id SP name
//! ```
//!
//! Note: the spec shows `LF` after each pkt-line, but git's send-pack does NOT
//! emit a trailing `\n` — the captured wire fixtures confirm this.  We match
//! the actual git client behaviour, not the ABNF annotation.

use std::io::Write;

use crate::send_pack::Request;

/// Write the command list + trailing flush-pkt onto `out`.
///
/// `hash_kind` determines the zero-OID width (40 hex chars for sha1, 64 for
/// sha256) so encoding stays parametric across the workspace-wide hash scope.
pub fn encode_into(request: &Request, hash_kind: gix_hash::Kind, out: &mut impl Write) -> std::io::Result<()> {
    let hex_len = hash_kind.len_in_hex();

    for (i, cmd) in request.commands.iter().enumerate() {
        let mut payload = Vec::with_capacity(hex_len * 2 + 3 + cmd.refname.len() + 64);
        // old-id SP new-id SP name
        write_oid_hex(&mut payload, &cmd.old_oid, hex_len)?;
        payload.push(b' ');
        write_oid_hex(&mut payload, &cmd.new_oid, hex_len)?;
        payload.push(b' ');
        payload.extend_from_slice(&cmd.refname);
        if i == 0 {
            // NUL capability-list
            // git's send-pack builds cap_buf with a leading space before every
            // capability (" report-status", " side-band-64k", …), so the wire
            // format is: <command>\0 cap1 cap2 …\n  (leading SP after NUL).
            payload.push(0);
            for cap in request.capabilities.iter() {
                payload.push(b' ');
                payload.extend_from_slice(cap);
            }
        }
        // No trailing LF — git's send-pack omits it despite the ABNF annotation.
        gix_transport::packetline::blocking_io::encode::data_to_write(&payload, &mut *out)?;
    }
    gix_transport::packetline::blocking_io::encode::flush_to_write(&mut *out)?;
    Ok(())
}

fn write_oid_hex(out: &mut Vec<u8>, oid: &gix_hash::ObjectId, hex_len: usize) -> std::io::Result<()> {
    if oid.is_null() {
        out.extend(std::iter::repeat_n(b'0', hex_len));
    } else {
        write!(out, "{}", oid.to_hex())?;
    }
    Ok(())
}
