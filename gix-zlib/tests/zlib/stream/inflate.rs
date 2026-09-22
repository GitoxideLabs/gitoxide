use std::io;
use std::io::Write as _;

use gix_zlib::stream::deflate;
use gix_zlib::{Decompress, stream::inflate};

#[test]
fn errors_keep_the_underlying_cause() {
    let mut checksum_mismatch = deflated(b"the trailing checksum protects this data");
    *checksum_mismatch.last_mut().expect("the stream is never empty") ^= 0xff;
    let err = inflate::read(&mut checksum_mismatch.as_slice(), &mut Decompress::new(), &mut [0; 128])
        .expect_err("an invalid checksum must fail");
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    insta::assert_debug_snapshot!(err, "errors keep the underlying cause", @"
    Custom {
        kind: InvalidInput,
        error: corrupt deflate stream: incorrect data check
        |
        └─ Invalid input data,
    }
    ");

    let mut corrupt_header = deflated(b"the header check protects this data");
    corrupt_header[0] = 0xff;
    let err = inflate::read(&mut corrupt_header.as_slice(), &mut Decompress::new(), &mut [0; 128])
        .expect_err("a corrupt stream must fail");
    insta::assert_debug_snapshot!(err, "errors keep the underlying cause", @"
    Custom {
        kind: InvalidInput,
        error: corrupt deflate stream: incorrect header check
        |
        └─ Invalid input data,
    }
    ");
}

fn deflated(data: &[u8]) -> Vec<u8> {
    let mut out = deflate::Write::new(Vec::new(), gix_zlib::Compression::BEST_SPEED);
    out.write_all(data).expect("in-memory writes never fail");
    out.flush().expect("in-memory flushes never fail");
    out.into_inner()
}
