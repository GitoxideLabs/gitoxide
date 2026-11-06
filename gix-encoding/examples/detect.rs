use std::io::{self, Read, Write};

use gix_error::{OptionExt, Result, ResultExt, bail, message};

const DEFAULT_MAX_BYTES: u64 = 1024 * 1024;

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let max_bytes = match (args.next(), args.next()) {
        (None, None) => DEFAULT_MAX_BYTES,
        (Some(max_bytes), None) => max_bytes
            .to_str()
            .ok_or_raise(|| message("max-bytes must be valid UTF-8"))?
            .parse::<u64>()
            .or_raise(|| message("max-bytes must be a non-negative integer that fits in u64"))?,
        _ => bail!(message!("usage: detect [max-bytes] (default: {DEFAULT_MAX_BYTES})")),
    };
    run(io::stdin().lock(), io::stdout().lock(), max_bytes)
}

fn run(input: impl Read, mut output: impl Write, max_bytes: u64) -> Result<()> {
    let mut input = input.take(max_bytes);
    let mut bytes = Vec::new();
    input
        .read_to_end(&mut bytes)
        .or_raise(|| message("failed to read stdin"))?;
    // Do not probe past the cap: a pipe may remain open without providing another byte.
    let extent = if input.limit() == 0 {
        "limit reached; input may continue and the final character may be incomplete"
    } else {
        "EOF"
    };
    writeln!(
        output,
        "Sampled {} bytes ({extent}).\n{:#?}",
        bytes.len(),
        gix_encoding::detect(&bytes)
    )
    .or_raise(|| message("failed to write the detection result to stdout"))?;
    output.flush().or_raise(|| message("failed to flush stdout"))?;
    Ok(())
}
