#[cfg(feature = "blocking-io")]
use std::io::Read;

use bstr::{BString, ByteSlice};
#[cfg(all(not(feature = "blocking-io"), feature = "async-io"))]
use futures_lite::io::AsyncReadExt;
use gix_odb::pack;
#[cfg(all(feature = "async-io", not(feature = "blocking-io")))]
use gix_packetline::async_io::StreamingPeekableIter;
#[cfg(all(feature = "blocking-io", not(feature = "async-io")))]
use gix_packetline::blocking_io::StreamingPeekableIter;
use gix_packetline::{read::ProgressAction, PacketLineRef};

use crate::read::streaming_peek_iter::fixture_bytes;

#[cfg(all(not(feature = "blocking-io"), feature = "async-io"))]
mod util {
    use std::{io::Result, pin::Pin};

    use futures_io::{AsyncBufRead, AsyncRead};
    use futures_lite::{future, AsyncBufReadExt, AsyncReadExt};

    pub struct BlockOn<T>(pub T);

    impl<T: AsyncRead + Unpin> std::io::Read for BlockOn<T> {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
            future::block_on(self.0.read(buf))
        }
    }

    impl<T: AsyncBufRead + Unpin> std::io::BufRead for BlockOn<T> {
        fn fill_buf(&mut self) -> Result<&[u8]> {
            future::block_on(self.0.fill_buf())
        }

        fn consume(&mut self, amt: usize) {
            Pin::new(&mut self.0).consume(amt);
        }
    }
}

#[maybe_async::test(feature = "blocking-io", async(feature = "async-io", async_std::test))]
async fn read_pack_with_progress_extraction() -> crate::Result {
    let buf = fixture_bytes("v1/01-clone.combined-output");
    let mut rd = StreamingPeekableIter::new(&buf[..], &[PacketLineRef::Flush], false);

    // Read without sideband decoding
    let mut out = Vec::new();
    rd.as_read().read_to_end(&mut out).await?;
    assert_eq!(out.as_bstr(), b"808e50d724f604f69ab93c6da2919c014667bedb HEAD\0multi_ack thin-pack side-band side-band-64k ofs-delta shallow deepen-since deepen-not deepen-relative no-progress include-tag multi_ack_detailed symref=HEAD:refs/heads/master object-format=sha1 agent=git/2.28.0\n808e50d724f604f69ab93c6da2919c014667bedb refs/heads/master\n".as_bstr());

    let res = rd.read_line().await;
    assert_eq!(
        res.expect("line")??.as_text().expect("data line").0.as_bstr(),
        b"NAK".as_bstr()
    );
    let mut seen_texts = Vec::<BString>::new();
    let mut do_nothing = |is_err: bool, data: &[u8]| -> ProgressAction {
        assert!(!is_err);
        seen_texts.push(data.as_bstr().into());
        std::ops::ControlFlow::Continue(())
    };
    let pack_read = rd.as_read_with_sidebands(&mut do_nothing);
    #[cfg(all(not(feature = "blocking-io"), feature = "async-io"))]
    let mut pack_entries = pack::data::input::BytesToEntriesIter::new_from_header(
        util::BlockOn(pack_read),
        pack::data::input::Mode::Verify,
        pack::data::input::EntryDataMode::Ignore,
        gix_hash::Kind::Sha1,
    )?;
    #[cfg(feature = "blocking-io")]
    let mut pack_entries = pack::data::input::BytesToEntriesIter::new_from_header(
        pack_read,
        pack::data::input::Mode::Verify,
        pack::data::input::EntryDataMode::Ignore,
        gix_hash::Kind::Sha1,
    )?;
    let all_but_last = pack_entries.size_hint().0 - 1;
    let last = pack_entries.nth(all_but_last).expect("last entry")?;
    drop(pack_entries);

    assert_eq!(
        last.trailer
            .expect("trailer to exist on last entry")
            .to_hex()
            .to_string(),
        "150a1045f04dc0fc2dbf72313699fda696bf4126"
    );
    assert_eq!(
        seen_texts,
        [
            "Enumerating objects: 3, done.",
            "Counting objects:  33% (1/3)\r",
            "Counting objects:  66% (2/3)\r",
            "Counting objects: 100% (3/3)\r",
            "Counting objects: 100% (3/3), done.",
            "Total 3 (delta 0), reused 0 (delta 0), pack-reused 0"
        ]
        .iter()
        .map(|v| v.as_bytes().as_bstr().to_owned())
        .collect::<Vec<_>>()
    );
    Ok(())
}

#[maybe_async::test(feature = "blocking-io", async(feature = "async-io", async_std::test))]
async fn read_line_trait_method_reads_one_packet_line_at_a_time() -> crate::Result {
    let buf = fixture_bytes("v1/01-clone.combined-output-no-binary");

    let mut rd = StreamingPeekableIter::new(&buf[..], &[PacketLineRef::Flush], false);

    let mut out = String::new();
    let mut r = rd.as_read();
    r.read_line_to_string(&mut out).await?;
    assert_eq!(out, "808e50d724f604f69ab93c6da2919c014667bedb HEAD\0multi_ack thin-pack side-band side-band-64k ofs-delta shallow deepen-since deepen-not deepen-relative no-progress include-tag multi_ack_detailed symref=HEAD:refs/heads/master object-format=sha1 agent=git/2.28.0\n");
    out.clear();
    r.read_line_to_string(&mut out).await?;
    assert_eq!(out, "808e50d724f604f69ab93c6da2919c014667bedb refs/heads/master\n");
    out.clear();
    r.read_line_to_string(&mut out).await?;
    assert_eq!(out, "", "flush means empty lines…");
    out.clear();
    r.read_line_to_string(&mut out).await?;
    assert_eq!(out, "", "…which can't be overcome unless the reader is reset");
    assert_eq!(
        r.stopped_at(),
        Some(PacketLineRef::Flush),
        "it knows what stopped the reader"
    );

    drop(r);
    rd.reset();

    let mut r = rd.as_read();
    r.read_line_to_string(&mut out).await?;
    assert_eq!(out, "NAK\n");

    drop(r);

    let mut r = rd.as_read_with_sidebands(|_, _| std::ops::ControlFlow::Continue(()));
    out.clear();
    r.read_line_to_string(&mut out).await?;
    assert_eq!(out, "&");

    out.clear();
    r.read_line_to_string(&mut out).await?;
    assert_eq!(out, "");

    Ok(())
}

#[maybe_async::test(feature = "blocking-io", async(feature = "async-io", async_std::test))]
async fn readline_reads_one_packet_line_at_a_time() -> crate::Result {
    let buf = fixture_bytes("v1/01-clone.combined-output-no-binary");

    let mut rd = StreamingPeekableIter::new(&buf[..], &[PacketLineRef::Flush], false);

    let mut r = rd.as_read();
    let line = r.read_data_line().await.unwrap()??.as_bstr().unwrap();
    assert_eq!(line, "808e50d724f604f69ab93c6da2919c014667bedb HEAD\0multi_ack thin-pack side-band side-band-64k ofs-delta shallow deepen-since deepen-not deepen-relative no-progress include-tag multi_ack_detailed symref=HEAD:refs/heads/master object-format=sha1 agent=git/2.28.0\n");
    let line = r.read_data_line().await.unwrap()??.as_bstr().unwrap();
    assert_eq!(line, "808e50d724f604f69ab93c6da2919c014667bedb refs/heads/master\n");
    let line = r.read_data_line().await;
    assert!(line.is_none(), "flush means `None`");
    let line = r.read_data_line().await;
    assert!(line.is_none(), "…which can't be overcome unless the reader is reset");
    assert_eq!(
        r.stopped_at(),
        Some(PacketLineRef::Flush),
        "it knows what stopped the reader"
    );

    drop(r);
    rd.reset();

    let mut r = rd.as_read();
    let line = r.read_data_line().await.unwrap()??.as_bstr().unwrap();
    assert_eq!(line.as_bstr(), "NAK\n");

    drop(r);

    let mut r = rd.as_read_with_sidebands(|_, _| std::ops::ControlFlow::Continue(()));
    let line = r.read_data_line().await.unwrap()??.as_bstr().unwrap();
    assert_eq!(
        line.as_bstr(),
        "\x02Enumerating objects: 3, done.\n",
        "sidebands are ignored entirely here"
    );
    for _ in 0..6 {
        let _discard_more_progress = r.read_data_line().await.unwrap()??.as_bstr().unwrap();
    }
    let line = r.read_data_line().await;
    assert!(line.is_none(), "and we have reached the end");

    Ok(())
}

#[maybe_async::test(feature = "blocking-io", async(feature = "async-io", async_std::test))]
async fn peek_past_an_actual_eof_is_an_error() -> crate::Result {
    let input = b"0009ERR e";
    let mut rd = StreamingPeekableIter::new(&input[..], &[], false);
    let mut reader = rd.as_read();
    let res = reader.peek_data_line().await;
    assert_eq!(res.expect("one line")??, b"ERR e");

    let mut buf = String::new();
    reader.read_line_to_string(&mut buf).await?;
    assert_eq!(
        buf, "ERR e",
        "by default ERR lines won't propagate as failure but are merely text"
    );

    let res = reader.peek_data_line().await;
    assert_eq!(
        res.expect("an err").expect_err("foo").kind(),
        std::io::ErrorKind::UnexpectedEof,
        "peeking past the end is not an error as the caller should make sure we don't try 'invalid' reads"
    );
    Ok(())
}

#[maybe_async::test(feature = "blocking-io", async(feature = "async-io", async_std::test))]
async fn peek_past_a_delimiter_is_no_error() -> crate::Result {
    let input = b"0009hello0000";
    let mut rd = StreamingPeekableIter::new(&input[..], &[PacketLineRef::Flush], false);
    let mut reader = rd.as_read();
    let res = reader.peek_data_line().await;
    assert_eq!(res.expect("one line")??, b"hello");

    let mut buf = String::new();
    reader.read_line_to_string(&mut buf).await?;
    assert_eq!(buf, "hello");

    let res = reader.peek_data_line().await;
    assert!(
        res.is_none(),
        "peeking past a flush packet is a 'natural' event that should not cause an error"
    );
    Ok(())
}

#[maybe_async::test(feature = "blocking-io", async(feature = "async-io", async_std::test))]
async fn handling_of_err_lines() {
    let input = b"0009ERR e0009ERR x0000";
    let mut rd = StreamingPeekableIter::new(&input[..], &[], false);
    rd.fail_on_err_lines(true);
    let mut buf = [0u8; 2];
    let mut reader = rd.as_read();
    let res = reader.read(buf.as_mut()).await;
    let err = res.unwrap_err();
    assert_eq!(err.to_string(), "e", "it respects errors and passes them on");
    assert_eq!(
        err.into_inner()
            .expect("inner err")
            .downcast::<gix_packetline::read::Error>()
            .expect("it's this type")
            .message,
        "e",
    );
    let res = reader.read(buf.as_mut()).await;
    assert_eq!(
        res.expect("read to succeed - EOF"),
        0,
        "it stops reading after an error despite there being more to read"
    );
    reader.reset_with(&[PacketLineRef::Flush]);
    let res = reader.read(buf.as_mut()).await;
    assert_eq!(
        res.unwrap_err().to_string(),
        "x",
        "after a reset it continues reading, but retains the 'fail_on_err_lines' setting"
    );
    assert_eq!(
        reader.stopped_at(),
        None,
        "An error can also be the reason, which is not distinguishable from an EOF"
    );
}
