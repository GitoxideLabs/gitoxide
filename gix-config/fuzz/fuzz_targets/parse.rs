#![no_main]

use arbitrary::Arbitrary;
use gix_config::{File, parse::Events};
use libfuzzer_sys::fuzz_target;
use std::hint::black_box;

#[derive(Debug, Arbitrary, Clone)]
struct Ctx<'a> {
    parse_from_bytes: &'a [u8],
    parse_file: &'a str,
    parse_events: &'a str,
}

fuzz_target!(|ctx: Ctx| {
    _ = black_box(Events::from_bytes(ctx.parse_from_bytes, None));
    _ = black_box(File::try_from(ctx.parse_file));
    _ = black_box(Events::try_from(ctx.parse_events));
});
