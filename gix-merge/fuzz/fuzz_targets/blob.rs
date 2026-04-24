#![no_main]
use anyhow::Result;
use arbitrary::Arbitrary;
use gix_merge::blob::builtin_driver::text::{self, Conflict, ConflictStyle};
use gix_merge::blob::Resolution;
use libfuzzer_sys::fuzz_target;
use std::hint::black_box;
use std::num::NonZero;

fn fuzz_text_merge(
    Ctx {
        base,
        ours,
        theirs,
        marker_size,
    }: Ctx,
) -> Result<()> {
    let mut buf = Vec::new();
    let mut input = imara_diff::InternedInput::default();
    // Fuzz this merge entrypoint with Histogram only. Repetitive adversarial text can drive the
    // Myers-family algorithms into pathological runtimes under sanitizer and coverage
    // instrumentation, which makes them unsuitable for this libFuzzer target and obscures
    // gix-merge-specific bugs behind diff-algorithm timeouts.
    for (left, right) in [(ours, theirs), (theirs, ours)] {
        input.clear();
        let prepared = text::Merge::new(&mut input, left, base, right, imara_diff::Algorithm::Histogram);
        let resolution = prepared.run(&mut buf, Default::default(), Conflict::default());
        if resolution == Resolution::Conflict {
            for conflict in [
                Conflict::ResolveWithOurs,
                Conflict::ResolveWithTheirs,
                Conflict::ResolveWithUnion,
                Conflict::Keep {
                    style: ConflictStyle::Diff3,
                    marker_size,
                },
                Conflict::Keep {
                    style: ConflictStyle::ZealousDiff3,
                    marker_size,
                },
            ] {
                prepared.run(&mut buf, Default::default(), conflict);
            }
        }
    }
    Ok(())
}

#[derive(Debug, Arbitrary)]
struct Ctx<'a> {
    base: &'a [u8],
    ours: &'a [u8],
    theirs: &'a [u8],
    marker_size: NonZero<u8>,
}

fuzz_target!(|ctx: Ctx<'_>| {
    _ = black_box(fuzz_text_merge(ctx));
});
