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
async fn each_write_results_in_one_line() -> gix_error::TestResult {
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
async fn write_text_and_write_binary() -> gix_error::TestResult {
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
async fn huge_writes_are_split_into_lines() -> gix_error::TestResult {
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
async fn empty_writes_fail_with_error() {
    let res = Writer::new(Vec::new()).write(&[]).await;
    assert_eq!(
        res.unwrap_err().to_string(),
        "empty packet lines are not permitted as '0004' is invalid"
    );
}
