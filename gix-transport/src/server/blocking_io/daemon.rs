use gix_packetline::PacketLineRef;

use crate::{
    packetline::blocking_io::StreamingPeekableIter,
    server::{blocking_io::connection::Connection, parse_connect_message, ClientRequest, Error},
};

/// Accept a git daemon connection by reading the initial connect message.
///
/// Reads the first packetline from `reader`, parses it as a
/// `git-proto-request`, and returns a [`Connection`] ready for
/// protocol communication along with the full [`ClientRequest`].
pub fn accept<R, W>(reader: R, writer: W, trace: bool) -> Result<(Connection<R, W>, ClientRequest), Error>
where
    R: std::io::Read,
    W: std::io::Write,
{
    let mut line_provider = StreamingPeekableIter::new(reader, &[PacketLineRef::Flush], trace);

    let line = line_provider
        .read_line()
        .ok_or(Error::MalformedMessage)?
        .map_err(|_| Error::MalformedMessage)?
        .map_err(|_| Error::MalformedMessage)?;

    let data = match line {
        PacketLineRef::Data(d) => d,
        _ => return Err(Error::MalformedMessage),
    };

    let request = parse_connect_message(data)?;

    let connection = Connection {
        line_provider,
        writer,
        service: request.service,
        repository_path: request.repository_path.clone(),
        protocol: request.desired_protocol,
    };

    Ok((connection, request))
}
