#![no_main]

use gix_credentials::protocol::Context;
use libfuzzer_sys::fuzz_target;
use std::hint::black_box;

fn fuzz(input: &[u8]) {
    if let Ok(ctx) = Context::from_bytes(input) {
        inspect_context(ctx.clone());

        let mut with_http = ctx.clone();
        _ = black_box(with_http.destructure_url_in_place(true));
        _ = black_box(with_http);

        let mut without_http = ctx;
        _ = black_box(without_http.destructure_url_in_place(false));
        _ = black_box(without_http);
    }
}

fn inspect_context(mut ctx: Context) {
    let mut out = Vec::new();
    _ = black_box(ctx.write_to(&mut out));
    _ = black_box(ctx.to_bstring());
    _ = black_box(ctx.to_url());
    _ = black_box(ctx.to_prompt("Username"));
    _ = black_box(ctx.clone().redacted());
    ctx.clear_secrets();
    _ = black_box(ctx);
    if let Ok(roundtrip) = Context::from_bytes(&out) {
        _ = black_box(roundtrip);
    }
}

fuzz_target!(|input: &[u8]| {
    fuzz(input);
});
