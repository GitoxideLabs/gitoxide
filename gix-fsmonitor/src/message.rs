/// Read one Simple IPC message from `input`, accepting at most `limit` payload bytes.
/// A connect-and-close probe has no message at all.
#[maybe_async]
pub(crate) async fn read_message(input: &mut (impl Read + Unpin), limit: usize) -> io::Result<Option<Vec<u8>>> {
    let mut message = Vec::new();
    let mut received_packet = false;
    loop {
        let mut header = [0; 4];
        match input.read(&mut header[..1]).await? {
            0 if !received_packet => return Ok(None),
            0 => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "IPC message ended before flush",
                ));
            }
            _ => {
                input.read_exact(&mut header[1..]).await?;
            }
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
                input.read_exact(&mut message[offset..]).await?;
            }
        }
    }
}

#[maybe_async]
pub(crate) async fn write_message(output: &mut (impl Write + Unpin), message: &[u8]) -> io::Result<()> {
    for chunk in message.chunks(MAX_PACKET_DATA) {
        write_packet(&mut *output, Some(chunk)).await?;
    }
    write_packet(&mut *output, None).await?;
    output.flush().await
}
