use std::io::{self, Write};

use gix_encoding::{Encoding, read};
use gix_error::{OptionExt, Result, ResultExt, bail, message};

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let (Some(from), Some(to), None) = (args.next(), args.next(), args.next()) else {
        bail!(message("usage: transcode encoding-in encoding-out"));
    };
    let from =
        Encoding::for_label(from.as_encoded_bytes()).ok_or_raise(|| message!("unknown input encoding: {from:?}"))?;
    let to = Encoding::for_label(to.as_encoded_bytes()).ok_or_raise(|| message!("unknown output encoding: {to:?}"))?;

    let mut input = read::Transcoder::new(io::stdin().lock(), from, to);
    let mut output = io::BufWriter::new(io::stdout().lock());
    io::copy(&mut input, &mut output)
        .or_raise(|| message!("failed to transcode stdin from {from} to {to} on stdout"))?;
    output.flush().or_raise(|| message("failed to flush stdout"))?;
    Ok(())
}
