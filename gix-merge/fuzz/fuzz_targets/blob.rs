#![no_main]
use anyhow::Result;
use arbitrary::Arbitrary;
use gix_merge::blob::builtin_driver::text::{Conflict, ConflictStyle, Options};
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
    let mut input = imara_diff::intern::InternedInput::default();
    for diff_algorithm in [
        imara_diff::Algorithm::Histogram,
        imara_diff::Algorithm::Myers,
        imara_diff::Algorithm::MyersMinimal,
    ] {
        let mut opts = Options {
            diff_algorithm,
            conflict: Default::default(),
        };
        for (left, right) in [(ours, theirs), (theirs, ours)] {
            let resolution = gix_merge::blob::builtin_driver::text(
                &mut buf,
                &mut input,
                Default::default(),
                left,
                base,
                right,
                opts,
            );
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
                    opts.conflict = conflict;
                    gix_merge::blob::builtin_driver::text(
                        &mut buf,
                        &mut input,
                        Default::default(),
                        left,
                        base,
                        right,
                        opts,
                    );
                }
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
