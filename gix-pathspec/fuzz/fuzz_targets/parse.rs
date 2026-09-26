#![no_main]
use anyhow::Result;
use libfuzzer_sys::fuzz_target;
use std::hint::black_box;

fn fuzz(data: &[u8]) -> Result<()> {
    let pattern = gix_pathspec::parse(data, Default::default())?;
    _ = black_box(pattern.is_nil());
    _ = black_box(pattern.prefix_directory());
    _ = black_box(pattern.path());
    // TODO: Fuzz normalize
    _ = black_box(pattern.is_excluded());
    _ = black_box(pattern.to_bstring());
    Ok(())
}

fuzz_target!(|data: &[u8]| {
    _ = black_box(fuzz(data));
});
