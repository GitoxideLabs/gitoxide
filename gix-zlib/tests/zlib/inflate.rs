use gix_zlib::{Inflate, Status};

use crate::stream::deflate::compressed;

#[test]
fn once_reports_progress_and_reset_allows_reuse() {
    let input = compressed(b"inflate once");
    let mut inflate = Inflate::default();
    let mut output = [0; 32];

    let (status, consumed, written) = inflate.once(&input, &mut output).expect("valid stream");
    assert_eq!(status, Status::StreamEnd);
    assert_eq!(consumed, input.len());
    assert_eq!(&output[..written], b"inflate once");

    inflate.reset();
    let (_, consumed, written) = inflate.once(&input, &mut output).expect("valid stream after reset");
    assert_eq!(consumed, input.len());
    assert_eq!(written, b"inflate once".len());
}

#[test]
fn once_into_a_short_buffer_returns_the_beginning_of_the_stream() {
    let input = compressed(b"more than fits into the output");
    let mut output = [0; 9];
    let (status, _consumed, written) = Inflate::default().once(&input, &mut output).expect("valid stream");
    assert_ne!(status, Status::StreamEnd, "the stream continues past the output buffer");
    assert_eq!(&output[..written], b"more than");
}

#[test]
fn prefix_can_be_continued_with_the_state() {
    let input = compressed(b"header and then the body");
    let mut inflate = Inflate::default();
    let mut header = [0; 6];
    let (status, consumed, written) = inflate.prefix(&input, &mut header).expect("valid stream");
    assert_ne!(status, Status::StreamEnd);
    assert_eq!(&header[..written], b"header");

    let mut body = [0; 32];
    let written = gix_zlib::stream::inflate::read(&mut &input[consumed..], &mut inflate.state, &mut body)
        .expect("the rest of the stream is valid");
    assert_eq!(&body[..written], b" and then the body");
}

#[test]
fn corrupt_streams_keep_classification_and_context() {
    let mut input = compressed(b"the zlib header protects this stream");
    input[0] = 0xff;
    let err = Inflate::default()
        .once(&input, &mut [0; 64])
        .expect_err("the corrupt header must be rejected");

    insta::assert_debug_snapshot!(err, "corrupt streams keep classification and context", @"
    Could not decode zip stream

    Caused by:
        0: Invalid input data
    ");
    assert!(
        err.is_corrupted(),
        "the underlying invalid stream should be classified as corruption"
    );
}
