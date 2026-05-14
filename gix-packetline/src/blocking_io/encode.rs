use std::io;

use crate::{
    BandRef, Channel, DELIMITER_LINE, ERR_PREFIX, ErrorRef, FLUSH_LINE, MAX_DATA_LEN, PacketLineRef, RESPONSE_END_LINE,
    TextRef,
    encode::{Error, u16_to_hex},
};

/// Write a response-end message to `out`.
pub fn response_end_to_write(mut out: impl io::Write) -> io::Result<usize> {
    out.write_all(RESPONSE_END_LINE).map(|_| 4)
}

/// Write a delim message to `out`.
pub fn delim_to_write(mut out: impl io::Write) -> io::Result<usize> {
    out.write_all(DELIMITER_LINE).map(|_| 4)
}

/// Write a flush message to `out`.
pub fn flush_to_write(mut out: impl io::Write) -> io::Result<usize> {
    out.write_all(FLUSH_LINE).map(|_| 4)
}

/// Write an error `message` to `out`.
pub fn error_to_write(message: &[u8], out: impl io::Write) -> io::Result<usize> {
    prefixed_data_to_write(ERR_PREFIX, message, out)
}

/// Serialize `error` to `out`.
///
/// This includes a marker to allow decoding it outside a sideband channel, returning the amount of bytes written.
pub fn write_error(error: &ErrorRef<'_>, out: impl io::Write) -> io::Result<usize> {
    error_to_write(error.0, out)
}

/// Write `data` of `kind` to `out` using sideband encoding.
pub fn band_to_write(kind: Channel, data: &[u8], out: impl io::Write) -> io::Result<usize> {
    prefixed_data_to_write(&[kind as u8], data, out)
}

/// Serialize `band` to `out`, returning the amount of bytes written.
///
/// The data written to `out` can be decoded with [`PacketLineRef::decode_band()`].
pub fn write_band(band: &BandRef<'_>, out: impl io::Write) -> io::Result<usize> {
    match band {
        BandRef::Data(d) => band_to_write(Channel::Data, d, out),
        BandRef::Progress(d) => band_to_write(Channel::Progress, d, out),
        BandRef::Error(d) => band_to_write(Channel::Error, d, out),
    }
}

/// Write a `data` message to `out`.
pub fn data_to_write(data: &[u8], out: impl io::Write) -> io::Result<usize> {
    prefixed_data_to_write(&[], data, out)
}

/// Serialize `line` to `out` in git `packetline` format, returning the amount of bytes written to `out`.
pub fn write_packet_line(line: &PacketLineRef<'_>, out: impl io::Write) -> io::Result<usize> {
    match line {
        PacketLineRef::Data(d) => data_to_write(d, out),
        PacketLineRef::Flush => flush_to_write(out),
        PacketLineRef::Delimiter => delim_to_write(out),
        PacketLineRef::ResponseEnd => response_end_to_write(out),
    }
}

/// Write a `text` message to `out`, which is assured to end in a newline.
pub fn text_to_write(text: &[u8], out: impl io::Write) -> io::Result<usize> {
    prefixed_and_suffixed_data_to_write(&[], text, b"\n", out)
}

/// Serialize `text` to `out`, appending a newline if there is none, returning the amount of bytes written.
pub fn write_text(text: &TextRef<'_>, out: impl io::Write) -> io::Result<usize> {
    text_to_write(text.0, out)
}

fn prefixed_data_to_write(prefix: &[u8], data: &[u8], out: impl io::Write) -> io::Result<usize> {
    prefixed_and_suffixed_data_to_write(prefix, data, &[], out)
}

fn prefixed_and_suffixed_data_to_write(
    prefix: &[u8],
    data: &[u8],
    suffix: &[u8],
    mut out: impl io::Write,
) -> io::Result<usize> {
    let data_len = prefix.len() + data.len() + suffix.len();
    if data_len > MAX_DATA_LEN {
        return Err(io::Error::other(Error::DataLengthLimitExceeded {
            length_in_bytes: data_len,
        }));
    }
    if data.is_empty() {
        return Err(io::Error::other(Error::DataIsEmpty));
    }

    let data_len = data_len + 4;
    let buf = u16_to_hex(data_len as u16);

    out.write_all(&buf)?;
    if !prefix.is_empty() {
        out.write_all(prefix)?;
    }
    out.write_all(data)?;
    if !suffix.is_empty() {
        out.write_all(suffix)?;
    }
    Ok(data_len)
}
