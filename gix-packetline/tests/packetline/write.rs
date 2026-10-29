#[cfg(feature = "blocking-io")]
use std::io::Write;

use bstr::ByteSlice;
#[cfg(all(feature = "async-io", not(feature = "blocking-io")))]
use futures_lite::prelude::*;
#[cfg(all(feature = "async-io", not(feature = "blocking-io")))]
use gix_packetline::async_io::Writer;
#[cfg(feature = "blocking-io")]
use gix_packetline::blocking_io::Writer;

const MAX_DATA_LEN: usize = 65516;
const MAX_LINE_LEN: usize = 4 + MAX_DATA_LEN;

#[expect(clippy::unused_io_amount)]
#[crate::bisync::bisync]
#[cfg_attr(feature = "blocking-io", test)]
#[cfg_attr(all(feature = "async-io", not(feature = "blocking-io")), async_std::test)]
async fn each_write_results_in_one_line() -> gix_testtools::TestResult {
    let mut w = Writer::new(Vec::new());
    w.write_all(b"hello").await?;
    w.write(b"world!").await?;
    let buf = w.into_inner();
    assert_eq!(buf.as_bstr(), b"0009hello000aworld!".as_bstr());
    Ok(())
}

#[expect(clippy::unused_io_amount)]
#[crate::bisync::bisync]
#[cfg_attr(feature = "blocking-io", test)]
#[cfg_attr(all(feature = "async-io", not(feature = "blocking-io")), async_std::test)]
async fn write_text_and_write_binary() -> gix_testtools::TestResult {
    let buf = {
        let mut w = Writer::new(Vec::new());
        w.enable_text_mode();
        w.write_all(b"hello").await?;
        w.enable_binary_mode();
        w.write(b"world").await?;
        w.into_inner()
    };
    assert_eq!(buf.as_bstr(), b"000ahello\n0009world".as_bstr());
    Ok(())
}

#[expect(clippy::unused_io_amount)]
#[crate::bisync::bisync]
#[cfg_attr(feature = "blocking-io", test)]
#[cfg_attr(all(feature = "async-io", not(feature = "blocking-io")), async_std::test)]
async fn huge_writes_are_split_into_lines() -> gix_testtools::TestResult {
    let buf = {
        let data = vec![0u8; MAX_DATA_LEN * 2];
        let mut w = Writer::new(Vec::new());
        w.write(&data).await?;
        w.into_inner()
    };
    assert_eq!(buf.len(), MAX_LINE_LEN * 2);
    Ok(())
}

#[crate::bisync::bisync]
#[cfg_attr(feature = "blocking-io", test)]
#[cfg_attr(all(feature = "async-io", not(feature = "blocking-io")), async_std::test)]
async fn empty_writes_emit_empty_packets() -> gix_testtools::TestResult {
    let mut writer = Writer::new(Vec::new());
    assert_eq!(writer.write(&[]).await?, 0);
    writer.write_all(b"next").await?;
    assert_eq!(
        writer.into_inner(),
        b"00040008next",
        "zero-length data is not a stream terminator"
    );
    Ok(())
}
