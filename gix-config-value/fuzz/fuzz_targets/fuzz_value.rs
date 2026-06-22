#![no_main]

extern crate libfuzzer_sys;

use anyhow::Result;
use arbitrary::Arbitrary;
use bstr::BStr;
use gix_config_value::{
    color::{Attribute, Name},
    path::interpolate::Context,
    Boolean, Color, Integer, Path,
};
use libfuzzer_sys::fuzz_target;
use std::{fmt::Write, hint::black_box, str::FromStr};

#[derive(Debug, Arbitrary)]
struct Ctx<'a> {
    bool_str: &'a [u8],
    color_str: &'a [u8],
    integer_str: &'a [u8],
    path_str: &'a [u8],
    attribute_str: &'a str,
    name_str: &'a str,
}

fn fuzz(ctx: Ctx) -> Result<()> {
    let b = Boolean::try_from(BStr::new(ctx.bool_str))?;
    _ = black_box(b.is_true());

    _ = black_box(Color::try_from(BStr::new(ctx.color_str)))?;

    let mut buf = String::with_capacity(128);
    let a = Attribute::from_str(ctx.attribute_str)?;
    _ = black_box(write!(&mut buf, "{a}"));

    let name = Name::from_str(ctx.name_str)?;
    _ = black_box(write!(&mut buf, "{name}"));

    let i = Integer::try_from(BStr::new(ctx.integer_str))?;
    _ = black_box(i.to_decimal());

    let p = Path::from(BStr::new(ctx.path_str));
    _ = black_box(p.interpolate(Context::default()));

    Ok(())
}

fuzz_target!(|ctx: Ctx| {
    if let Err(e) = fuzz(ctx) {
        // Exercise display/debug fmt code.
        _ = black_box(format!("{e} {e:?}"));
    }
});
