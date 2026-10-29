use std::io::{self, BufReader, Read};

#[test]
fn stops_at_an_empty_line_without_waiting_for_eof() -> gix_testtools::TestResult {
    struct NoMoreInput;
    impl Read for NoMoreInput {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("read beyond the credential request"))
        }
    }
    for input in [b"protocol=https\n\n".as_slice(), b"protocol=https\r\n\r\n"] {
        let mut reader = BufReader::with_capacity(2, input.chain(NoMoreInput));
        let actual = gix_credentials::protocol::read(&mut reader)?;
        assert_eq!(actual, input, "a blank line completes the request before EOF");
    }
    assert_eq!(
        gix_credentials::protocol::read(b"protocol=https".as_slice())?,
        b"protocol=https",
        "EOF also terminates an unterminated request"
    );
    assert!(
        gix_credentials::protocol::read(BufReader::new(NoMoreInput)).is_err(),
        "an input error before the terminator is reported"
    );
    Ok(())
}
