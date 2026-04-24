#![no_main]

use gix_bitmap::ewah;
use libfuzzer_sys::fuzz_target;
use std::hint::black_box;

fn fuzz(input: &[u8]) {
    let Ok((bitmap, rest)) = ewah::decode(input) else {
        return;
    };

    _ = black_box(rest);
    _ = black_box(bitmap.num_bits());

    let mut visited = 0usize;
    _ = black_box(bitmap.for_each_set_bit(|idx| {
        visited = visited.saturating_add(idx);
        Some(())
    }));
    _ = black_box(visited);

    let mut visited = 0usize;
    _ = black_box(bitmap.for_each_set_bit(|idx| {
        visited = visited.saturating_add(idx);
        (idx < 128).then_some(())
    }));
    _ = black_box(visited);
}

fuzz_target!(|input: &[u8]| {
    fuzz(input);
});
