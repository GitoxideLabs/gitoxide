#![no_main]

use arbitrary::Arbitrary;
use gix_ref::file::log;
use libfuzzer_sys::fuzz_target;
use std::hint::black_box;

#[derive(Arbitrary, Debug)]
struct Ctx<'a> {
    line_ref: &'a [u8],
    multi_line_reverse: &'a [u8],
    multi_line_forward: &'a [u8],
}

fn fuzz(ctx: Ctx) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let line = log::LineRef::from_bytes(ctx.line_ref)?;
    _ = black_box(line.previous_oid());
    _ = black_box(line.new_oid());

    let mut buf = [0u8; 1024];
    let read = std::io::Cursor::new(ctx.multi_line_reverse);
    let iter = gix_ref::file::log::iter::reverse(read, &mut buf)?;
    _ = black_box(iter.map(|x| black_box(x)).count());

    let iter = gix_ref::file::log::iter::forward(ctx.multi_line_forward);
    _ = black_box(iter.map(|x| black_box(x)).count());

    Ok(())
}

fuzz_target!(|ctx: Ctx| {
    _ = black_box(fuzz(ctx));
});
