#![no_main]

use gix_packetline::{
    blocking_io::{encode, StreamingPeekableIter, Writer},
    decode,
    PacketLineRef,
};
use libfuzzer_sys::fuzz_target;
use std::{
    hint::black_box,
    io::{Cursor, Read, Write},
};

fn inspect_line(line: PacketLineRef<'_>) {
    _ = black_box(line.as_slice());
    if let Some(data) = line.as_slice() {
        _ = black_box(line.as_error());
        _ = black_box(line.check_error());
        if !data.is_empty() {
            _ = black_box(line.as_text());
            _ = black_box(line.decode_band());
        }
    }
}

fn replay_encoded(mut input: &[u8]) {
    while !input.is_empty() {
        match decode::streaming(input) {
            Ok(decode::Stream::Complete {
                line,
                bytes_consumed,
            }) => {
                inspect_line(line);
                if bytes_consumed == 0 || bytes_consumed > input.len() {
                    break;
                }
                input = &input[bytes_consumed..];
            }
            Ok(decode::Stream::Incomplete { .. }) => break,
            Err(err) => {
                _ = black_box(err);
                break;
            }
        }
    }
}

fn fuzz(input: &[u8]) {
    match decode::streaming(input) {
        Ok(decode::Stream::Complete { line, .. }) => inspect_line(line),
        Ok(decode::Stream::Incomplete { .. }) => {}
        Err(err) => {
            _ = black_box(err);
        }
    }

    if let Ok(line) = decode::all_at_once(input) {
        inspect_line(line);
    }

    let mut iter = StreamingPeekableIter::new(
        Cursor::new(input),
        &[
            PacketLineRef::Flush,
            PacketLineRef::Delimiter,
            PacketLineRef::ResponseEnd,
        ],
        false,
    );
    _ = black_box(iter.peek_line());
    for _ in 0..8 {
        match iter.read_line() {
            Some(Ok(Ok(line))) => inspect_line(line),
            Some(Ok(Err(err))) => {
                _ = black_box(err);
                break;
            }
            Some(Err(err)) => {
                _ = black_box(err);
                break;
            }
            None => break,
        }
    }

    {
        let mut iter = StreamingPeekableIter::new(Cursor::new(input), &[PacketLineRef::Flush], false);
        let mut reader = iter.as_read();
        let mut buf = [0u8; 256];
        _ = black_box(reader.read(&mut buf));
    }

    {
        let mut iter = StreamingPeekableIter::new(Cursor::new(input), &[PacketLineRef::Flush], false);
        let mut reader = iter.as_read();
        let mut line = String::new();
        _ = black_box(reader.read_line_to_string(&mut line));
        _ = black_box(line);
    }

    if !input.is_empty() {
        let mut writer = Writer::new(Vec::new());
        if input[0] & 1 == 0 {
            writer.enable_binary_mode();
        } else {
            writer.enable_text_mode();
        }
        let _ = black_box(writer.write_all(&input[..input.len().min(1024)]));
        let mut encoded = writer.into_inner();
        _ = black_box(encode::flush_to_write(&mut encoded));
        replay_encoded(&encoded);
    }
}

fuzz_target!(|input: &[u8]| {
    fuzz(input);
});
