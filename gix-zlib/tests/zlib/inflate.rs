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
fn corrupt_streams_keep_classification_and_context() {
    let mut input = compressed(b"the zlib header protects this stream");
    input[0] = 0xff;
    let err = Inflate::default()
        .once(&input, &mut [0; 64])
        .expect_err("the corrupt header must be rejected");

    assert_eq!(err, "Could not decode zip stream");
    assert!(
        err.downcast_any_ref::<gix_error::CorruptionError>().is_some(),
        "the underlying invalid stream should be classified as corruption"
    );
}
