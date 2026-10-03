#![no_main]

use arbitrary::Arbitrary;
use bstr::{BStr, BString};
use gix_error::{Result, ResultExt};
use gix_ref::{FullName, FullNameRef, PartialName, namespace::expand};
use libfuzzer_sys::fuzz_target;
use std::hint::black_box;

#[derive(Debug, Arbitrary)]
struct Ctx<'a> {
    full_name: &'a [u8],
    full_name_ref: &'a [u8],
    partial_name: Vec<u8>,
    partial_name_join: &'a [u8],
}

fn fuzz(ctx: Ctx) -> Result<()> {
    let mut full_name = FullName::try_from(BStr::new(ctx.full_name)).or_error()?;
    _ = black_box(full_name.category_and_short_name());

    let full_name_ref = <&FullNameRef>::try_from(BStr::new(ctx.full_name_ref)).or_error()?;
    _ = black_box(full_name_ref.category_and_short_name());

    let partial_name_ref = full_name_ref.as_partial_name();
    _ = black_box(partial_name_ref.to_partial_path());

    let namespace = expand(partial_name_ref).or_error()?;
    let full_name = full_name.prefix_namespace(&namespace);
    _ = black_box(full_name.strip_namespace(&namespace));

    let partial_name = PartialName::try_from(BString::new(ctx.partial_name)).or_error()?;
    _ = black_box(partial_name.join(BStr::new(ctx.partial_name_join)).or_error()?);

    Ok(())
}

fuzz_target!(|ctx: Ctx| {
    _ = black_box(fuzz(ctx));
});
