#![no_main]

use gix_error::Result;
use libfuzzer_sys::fuzz_target;

use std::hint::black_box;

use arbitrary::{Arbitrary, Unstructured};
use gix_attributes::{
    Search,
    search::{MetadataCollection, Outcome},
};
use gix_glob::pattern::Case;

fn arbitrary_case(u: &mut Unstructured) -> arbitrary::Result<Case> {
    Ok(*u.choose(&[Case::Sensitive, Case::Fold])?)
}

// Keep fuzz-generated path patterns small enough that pathological glob matching doesn't dominate fuzzing time.
// We don't mitigate this in gix-glob as memoization made typical matches slower, and we want to stay on par with Git.
const MAX_FUZZ_PATH_PATTERN_LEN: usize = 256;

#[derive(Debug, Arbitrary)]
struct Ctx<'a> {
    pattern: &'a str,
    #[arbitrary(with = arbitrary_case)]
    case: Case,
}

fn fuzz(Ctx { pattern, case }: Ctx) -> Result<()> {
    if pattern.len() > MAX_FUZZ_PATH_PATTERN_LEN {
        return Ok(());
    }

    let mut search = Search::default();
    let mut collection = MetadataCollection::default();
    search.add_patterns_buffer(
        format!("{pattern} attr").as_bytes(),
        Default::default(),
        None,
        &mut collection,
        true,
    );
    let mut out = Outcome::default();
    out.initialize(&collection);
    _ = black_box(search.pattern_matching_relative_path("relative/path".into(), case, None, &mut out));
    Ok(())
}

fuzz_target!(|ctx: Ctx| {
    _ = black_box(fuzz(ctx));
});
