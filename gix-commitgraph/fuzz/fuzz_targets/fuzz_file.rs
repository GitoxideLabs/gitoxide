#![no_main]

use anyhow::Result;
use gix_commitgraph::File;
use libfuzzer_sys::fuzz_target;
use std::hint::black_box;

fn fuzz(data: &[u8]) -> Result<()> {
    let data = {
        let mut d = memmap2::MmapMut::map_anon(data.len())?;
        d.copy_from_slice(data);
        d.make_read_only()?
    };
    let file = File::new(data, "does not matter".into())?;

    _ = black_box(file.iter_base_graph_ids().count());
    _ = black_box(file.iter_commits().count());
    _ = black_box(file.iter_ids().count());

    let _ = black_box(file.checksum());
    let _ = black_box(file.verify_checksum());
    let _ = black_box(file.object_hash());

    Ok(())
}

fuzz_target!(|data: &[u8]| {
    _ = black_box(fuzz(data));
});
