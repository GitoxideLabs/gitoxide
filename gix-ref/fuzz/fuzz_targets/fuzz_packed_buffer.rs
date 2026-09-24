#![no_main]

use arbitrary::Arbitrary;
use bstr::BStr;
use gix_ref::{packed::Buffer, FullNameRef};
use libfuzzer_sys::fuzz_target;
use std::hint::black_box;

#[derive(Debug, Arbitrary)]
struct Ctx<'a> {
    packed_file_contents: &'a [u8],
    name: &'a [u8],
}

fn fuzz(ctx: Ctx) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let buffer = Buffer::from_bytes(ctx.packed_file_contents, gix_hash::Kind::Sha1)?;
    _ = black_box(buffer.iter()?.count());

    let full_name_ref: &FullNameRef = BStr::new(ctx.name).try_into()?;
    let name = full_name_ref.as_partial_name();

    _ = black_box(buffer.try_find(name));
    _ = black_box(buffer.find(name));

    Ok(())
}

fuzz_target!(|ctx: Ctx| {
    _ = black_box(fuzz(ctx));
});
