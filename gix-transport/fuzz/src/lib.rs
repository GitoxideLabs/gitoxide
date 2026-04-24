use gix_packetline::{blocking_io::StreamingPeekableIter, PacketLineRef};
use gix_transport::client::{capabilities::blocking_recv::Handshake, Capabilities};
use std::{
    hint::black_box,
    io::{Cursor, Read},
};

pub fn inspect_capabilities(caps: &Capabilities) {
    _ = black_box(caps.contains("fetch"));
    _ = black_box(caps.contains("ls-refs"));
    for capability in caps.iter().take(16) {
        _ = black_box(capability.name());
        _ = black_box(capability.value());
        _ = black_box(capability.values().map(|values| values.take(8).count()));
        _ = black_box(capability.supports("agent"));
    }
}

pub fn inspect_handshake(input: &[u8]) {
    let mut stream = StreamingPeekableIter::new(Cursor::new(input), &[PacketLineRef::Flush], false);
    match Handshake::from_lines_with_version_detection(&mut stream) {
        Ok(handshake) => {
            inspect_capabilities(&handshake.capabilities);
            _ = black_box(handshake.protocol);
            if let Some(mut refs) = handshake.refs {
                let mut buf = Vec::new();
                _ = black_box(refs.read_to_end(&mut buf));
                _ = black_box(buf);
            }
        }
        Err(err) => {
            _ = black_box(err);
        }
    };
}
