mod sideband;

pub mod streaming_peek_iter {
    use std::{io, path::PathBuf};

    use bstr::ByteSlice;
    #[cfg(all(feature = "async-io", not(feature = "blocking-io")))]
    use gix_packetline::async_io::StreamingPeekableIter;
    #[cfg(all(feature = "blocking-io", not(feature = "async-io")))]
    use gix_packetline::blocking_io::StreamingPeekableIter;
    use gix_packetline::PacketLineRef;

    fn fixture_path(path: &str) -> PathBuf {
        PathBuf::from("tests/fixtures").join(path)
    }

    pub fn fixture_bytes(path: &str) -> Vec<u8> {
        std::fs::read(fixture_path(path)).expect("readable fixture")
    }

    fn first_line() -> PacketLineRef<'static> {
        PacketLineRef::Data(b"7814e8a05a59c0cf5fb186661d1551c75d1299b5 HEAD\0multi_ack thin-pack side-band side-band-64k ofs-delta shallow deepen-since deepen-not deepen-relative no-progress include-tag multi_ack_detailed symref=HEAD:refs/heads/master object-format=sha1 agent=git/2.28.0\n")
    }

    #[maybe_async::test(feature = "blocking-io", async(feature = "async-io", async_std::test))]
    async fn peek_follows_read_line_delimiter_logic() -> crate::Result {
        let mut rd = StreamingPeekableIter::new(&b"0005a00000005b"[..], &[PacketLineRef::Flush], false);
        let res = rd.peek_line().await;
        assert_eq!(res.expect("line")??, PacketLineRef::Data(b"a"));
        rd.read_line().await;

        let res = rd.peek_line().await;
        assert!(res.is_none(), "we hit the delimiter, and thus are EOF");
        assert_eq!(
            rd.stopped_at(),
            Some(PacketLineRef::Flush),
            "Stopped tracking is done even when peeking"
        );
        let res = rd.peek_line().await;
        assert!(res.is_none(), "we are still done, no way around it");
        rd.reset();
        let res = rd.peek_line().await;
        assert_eq!(
            res.expect("line")??,
            PacketLineRef::Data(b"b"),
            "after resetting, we get past the delimiter"
        );
        Ok(())
    }

    #[maybe_async::test(feature = "blocking-io", async(feature = "async-io", async_std::test))]
    async fn peek_follows_read_line_err_logic() -> crate::Result {
        let mut rd = StreamingPeekableIter::new(&b"0005a0009ERR e0000"[..], &[PacketLineRef::Flush], false);
        rd.fail_on_err_lines(true);
        let res = rd.peek_line().await;
        assert_eq!(res.expect("line")??, PacketLineRef::Data(b"a"));
        rd.read_line().await;
        let res = rd.peek_line().await;
        assert_eq!(
            res.expect("line").unwrap_err().to_string(),
            "e",
            "io errors are used to communicate remote errors when peeking"
        );
        let res = rd.peek_line().await;
        assert!(res.is_none(), "we are still done, no way around it");
        assert_eq!(rd.stopped_at(), None, "we stopped not because of a delimiter");
        rd.reset();
        let res = rd.peek_line().await;
        assert!(res.is_none(), "it should stop due to the delimiter");
        assert_eq!(
            rd.stopped_at(),
            Some(PacketLineRef::Flush),
            "Stopped tracking is done even when peeking"
        );
        Ok(())
    }

    #[maybe_async::test(feature = "blocking-io", async(feature = "async-io", async_std::test))]
    async fn peek_eof_is_none() -> crate::Result {
        let mut rd = StreamingPeekableIter::new(&b"0005a0009ERR e0000"[..], &[PacketLineRef::Flush], false);
        rd.fail_on_err_lines(false);
        let res = rd.peek_line().await;
        assert_eq!(res.expect("line")??, PacketLineRef::Data(b"a"));
        rd.read_line().await;
        let res = rd.peek_line().await;
        assert_eq!(
            res.expect("line")??,
            PacketLineRef::Data(b"ERR e"),
            "we read the ERR but it's not interpreted as such"
        );
        rd.read_line().await;

        let res = rd.peek_line().await;
        assert!(res.is_none(), "we peek into the flush packet, which is EOF");
        assert_eq!(rd.stopped_at(), Some(PacketLineRef::Flush));
        Ok(())
    }

    #[maybe_async::test(feature = "blocking-io", async(feature = "async-io", async_std::test))]
    async fn peek_non_data() -> crate::Result {
        let mut rd = StreamingPeekableIter::new(&b"000000010002"[..], &[PacketLineRef::ResponseEnd], false);
        let res = rd.read_line().await;
        assert_eq!(res.expect("line")??, PacketLineRef::Flush);
        let res = rd.read_line().await;
        assert_eq!(res.expect("line")??, PacketLineRef::Delimiter);
        rd.reset_with(&[PacketLineRef::Flush]);
        let res = rd.read_line().await;
        assert_eq!(res.expect("line")??, PacketLineRef::ResponseEnd);
        for _ in 0..2 {
            let res = rd.peek_line().await;
            assert_eq!(
                res.expect("error").unwrap_err().kind(),
                std::io::ErrorKind::UnexpectedEof,
                "peeks on error/eof repeat the error"
            );
        }
        assert_eq!(
            rd.stopped_at(),
            None,
            "The reader is configured to ignore ResponseEnd, and thus hits the end of stream"
        );
        Ok(())
    }

    #[maybe_async::test(feature = "blocking-io", async(feature = "async-io", async_std::test))]
    async fn fail_on_err_lines() -> crate::Result {
        let input = b"00010009ERR e0002";
        let mut rd = StreamingPeekableIter::new(&input[..], &[], false);
        let res = rd.read_line().await;
        assert_eq!(res.expect("line")??, PacketLineRef::Delimiter);
        let res = rd.read_line().await;
        assert_eq!(
            res.expect("line")??.as_bstr(),
            Some(b"ERR e".as_bstr()),
            "by default no special handling"
        );

        let mut rd = StreamingPeekableIter::new(&input[..], &[], false);
        rd.fail_on_err_lines(true);
        let res = rd.read_line().await;
        assert_eq!(res.expect("line")??, PacketLineRef::Delimiter);
        let res = rd.read_line().await;
        assert_eq!(
            res.expect("line").unwrap_err().to_string(),
            "e",
            "io errors are used to communicate remote errors"
        );
        let res = rd.read_line().await;
        assert!(res.is_none(), "iteration is done after the first error");

        rd.replace(input);
        let res = rd.read_line().await;
        assert_eq!(res.expect("line")??, PacketLineRef::Delimiter);
        let res = rd.read_line().await;
        assert_eq!(
            res.expect("line")??.as_bstr(),
            Some(b"ERR e".as_bstr()),
            "a 'replace' also resets error handling to the default: false"
        );
        Ok(())
    }

    #[maybe_async::test(feature = "blocking-io", async(feature = "async-io", async_std::test))]
    async fn oversized_packet_lengths_are_reported_instead_of_panicking() -> crate::Result {
        let mut rd = StreamingPeekableIter::new(&b"ffff\n"[..], &[], false);
        let err = rd
            .read_line()
            .await
            .expect("a decode error instead of EOF")
            .expect("no IO error expected")
            .expect_err("decode should fail for oversized lengths");
        assert!(matches!(
            err,
            gix_packetline::decode::Error::DataLengthLimitExceeded { length_in_bytes: 65535 }
        ));
        Ok(())
    }

    #[maybe_async::test(feature = "blocking-io", async(feature = "async-io", async_std::test))]
    async fn peek() -> crate::Result {
        let bytes = fixture_bytes("v1/fetch/01-many-refs.response");
        let mut rd = StreamingPeekableIter::new(&bytes[..], &[PacketLineRef::Flush], false);
        let res = rd.peek_line().await;
        assert_eq!(res.expect("line")??, first_line(), "peek returns first line");
        let res = rd.peek_line().await;
        assert_eq!(
            res.expect("line")??,
            first_line(),
            "peeked lines are never exhausted, unless they are finally read"
        );
        let res = rd.read_line().await;
        assert_eq!(res.expect("line")??, first_line(), "read_line returns the peek once");
        let res = rd.read_line().await;
        assert_eq!(
            res.expect("line")??.as_bstr(),
            Some(b"7814e8a05a59c0cf5fb186661d1551c75d1299b5 refs/heads/master\n".as_bstr()),
            "the next read_line returns the next line"
        );
        let res = rd.peek_line().await;
        assert_eq!(
            res.expect("line")??.as_bstr(),
            Some(b"7814e8a05a59c0cf5fb186661d1551c75d1299b5 refs/remotes/origin/HEAD\n".as_bstr()),
            "peek always gets the next line verbatim"
        );
        let res = exhaust(&mut rd).await;
        assert_eq!(res, 1559);
        assert_eq!(
            rd.stopped_at(),
            Some(PacketLineRef::Flush),
            "A flush packet line ends every pack file"
        );
        Ok(())
    }

    #[maybe_async::test(feature = "blocking-io", async(feature = "async-io", async_std::test))]
    async fn read_from_file_and_reader_advancement() -> crate::Result {
        let mut bytes = fixture_bytes("v1/fetch/01-many-refs.response");
        bytes.extend(fixture_bytes("v1/fetch/01-many-refs.response"));
        let mut rd = StreamingPeekableIter::new(&bytes[..], &[PacketLineRef::Flush], false);
        let res = rd.read_line().await;
        assert_eq!(res.expect("line")??, first_line());
        let res = exhaust(&mut rd).await;
        assert_eq!(res + 1, 1561, "it stops after seeing the flush byte");
        rd.reset();
        let res = exhaust(&mut rd).await;
        assert_eq!(
            res, 1561,
            "it should read the second part of the identical file from the previously advanced reader"
        );

        // this reset is will cause actual io::Errors to occur
        rd.reset();
        let res = rd.read_line().await;
        assert_eq!(
            res.expect("some error").unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof,
            "trying to keep reading from exhausted input results in Some() containing the original error"
        );
        Ok(())
    }

    #[maybe_async::maybe_async]
    async fn exhaust(rd: &mut StreamingPeekableIter<&[u8]>) -> i32 {
        let mut count = 0;
        while rd.read_line().await.is_some() {
            count += 1;
        }
        count
    }
}
