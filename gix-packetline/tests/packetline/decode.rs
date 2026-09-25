mod streaming {
    use gix_error::Result;
    use gix_packetline::{
        ErrorRef, PacketLineRef,
        decode::{Stream, streaming},
    };

    fn assert_complete(res: Result<Stream>, expected_consumed: usize, expected_value: PacketLineRef) -> Result {
        match res? {
            Stream::Complete { line, bytes_consumed } => {
                assert_eq!(bytes_consumed, expected_consumed);
                assert_eq!(line.as_bstr(), expected_value.as_bstr());
            }
            Stream::Incomplete { .. } => panic!("expected parsing to be complete, not partial"),
        }
        Ok(())
    }

    mod round_trip {
        use bstr::ByteSlice;
        use gix_packetline::{Channel, PacketLineRef, decode, decode::streaming};

        use crate::decode::streaming::assert_complete;
        #[cfg(all(feature = "async-io", not(feature = "blocking-io")))]
        use gix_packetline::async_io::encode as encode_io;
        #[cfg(feature = "blocking-io")]
        use gix_packetline::blocking_io::encode as encode_io;

        #[crate::bisync::bisync]
        #[cfg_attr(feature = "blocking-io", test)]
        #[cfg_attr(all(feature = "async-io", not(feature = "blocking-io")), async_std::test)]
        async fn trailing_line_feeds_are_removed_explicitly() -> gix_error::TestResult {
            let line = decode::all_at_once(b"0006a\n")?;
            assert_eq!(line.as_text().expect("text").0.as_bstr(), b"a".as_bstr());
            let mut out = Vec::new();
            encode_io::write_text(&line.as_text().expect("text"), &mut out)
                .await
                .expect("write to memory works");
            assert_eq!(out, b"0006a\n", "it appends a newline in text mode");
            Ok(())
        }

        #[crate::bisync::bisync]
        #[cfg_attr(feature = "blocking-io", test)]
        #[cfg_attr(all(feature = "async-io", not(feature = "blocking-io")), async_std::test)]
        async fn all_kinds_of_packetlines() -> gix_error::TestResult {
            for (line, bytes) in &[
                (PacketLineRef::ResponseEnd, 4),
                (PacketLineRef::Delimiter, 4),
                (PacketLineRef::Flush, 4),
                (PacketLineRef::Data(b"hello there"), 15),
            ] {
                let mut out = Vec::new();
                encode_io::write_packet_line(line, &mut out).await?;
                assert_complete(streaming(&out), *bytes, *line)?;
            }
            Ok(())
        }

        #[crate::bisync::bisync]
        #[cfg_attr(feature = "blocking-io", test)]
        #[cfg_attr(all(feature = "async-io", not(feature = "blocking-io")), async_std::test)]
        async fn error_line() -> gix_error::TestResult {
            let mut out = Vec::new();
            encode_io::write_error(
                &PacketLineRef::Data(b"the error").as_error().expect("data line"),
                &mut out,
            )
            .await?;
            let line = decode::all_at_once(&out)?;
            assert_eq!(line.check_error().expect("err").0, b"the error");
            Ok(())
        }

        #[crate::bisync::bisync]
        #[cfg_attr(feature = "blocking-io", test)]
        #[cfg_attr(all(feature = "async-io", not(feature = "blocking-io")), async_std::test)]
        async fn side_bands() -> gix_error::TestResult {
            for channel in &[Channel::Data, Channel::Error, Channel::Progress] {
                let mut out = Vec::new();
                let band = PacketLineRef::Data(b"band data")
                    .as_band(*channel)
                    .expect("data is valid for band");
                encode_io::write_band(&band, &mut out).await?;
                let line = decode::all_at_once(&out)?;
                assert_eq!(line.decode_band().expect("valid band"), band);
            }
            Ok(())
        }

        #[test]
        fn empty_sideband_payload_is_invalid_instead_of_panicking() {
            let err = PacketLineRef::Data(b"")
                .decode_band()
                .expect_err("empty data cannot contain a sideband designator");
            insta::assert_debug_snapshot!(err, "empty sideband data is reported as malformed input", @"attempt to decode a non-data line into a side-channel band");
        }
    }

    #[test]
    fn flush() -> gix_error::TestResult {
        assert_complete(streaming(b"0000someotherstuff"), 4, PacketLineRef::Flush)?;
        Ok(())
    }

    #[test]
    fn trailing_line_feeds_are_not_removed_automatically() -> gix_error::TestResult {
        assert_complete(streaming(b"0006a\n"), 6, PacketLineRef::Data(b"a\n"))?;
        Ok(())
    }

    #[test]
    fn ignore_extra_bytes() -> gix_error::TestResult {
        assert_complete(streaming(b"0006a\nhello"), 6, PacketLineRef::Data(b"a\n"))?;
        Ok(())
    }

    #[test]
    fn error_on_oversized_line() {
        let err = (streaming(b"ffff")).expect_err("the packet line is invalid");
        insta::assert_debug_snapshot!(err, "error on oversized line", @"The data received claims to be larger than the maximum allowed size: got 65535, exceeds 65516");
    }

    #[test]
    fn error_on_error_line() -> gix_error::TestResult {
        let line = PacketLineRef::Data(b"ERR the error");
        assert_complete(
            streaming(b"0011ERR the error-and just ignored because not part of the size"),
            17,
            line,
        )?;
        assert_eq!(
            line.check_error().expect("error to be parsed here"),
            ErrorRef(b"the error")
        );
        Ok(())
    }

    #[test]
    fn error_on_invalid_hex() {
        let err = (streaming(b"fooo")).expect_err("the packet line is invalid");
        insta::assert_debug_snapshot!(err, "error on invalid hex", @"Failed to decode the first four hex bytes indicating the line length: Invalid character");
    }

    #[test]
    fn error_on_empty_line() {
        let err = (streaming(b"0004")).expect_err("the packet line is invalid");
        insta::assert_debug_snapshot!(err, "error on empty line", @"Received an invalid empty line");
    }

    mod incomplete {
        use gix_error::Result;
        use gix_packetline::decode::{Stream, streaming};

        fn assert_incomplete(res: Result<Stream>, expected_missing: usize) -> Result {
            match res? {
                Stream::Complete { .. } => {
                    panic!("expected parsing to be partial, not complete");
                }
                Stream::Incomplete { bytes_needed } => {
                    assert_eq!(bytes_needed, expected_missing);
                }
            }
            Ok(())
        }

        #[test]
        fn missing_hex_bytes() -> gix_error::TestResult {
            assert_incomplete(streaming(b"0"), 3)?;
            assert_incomplete(streaming(b"00"), 2)?;
            Ok(())
        }

        #[test]
        fn missing_data_bytes() -> gix_error::TestResult {
            assert_incomplete(streaming(b"0005"), 1)?;
            assert_incomplete(streaming(b"0006a"), 1)?;
            Ok(())
        }
    }
}
