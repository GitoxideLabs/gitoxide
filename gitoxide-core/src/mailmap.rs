use gix::{
    Result,
    error::{Exn, ResultExt, bail, message},
};
use std::{collections::HashSet, io::Write, path::Path};

use crate::OutputFormat;

pub const PROGRESS_RANGE: std::ops::RangeInclusive<u8> = 1..=2;

pub fn verify(path: impl AsRef<Path>, format: OutputFormat, mut out: impl Write) -> Result<()> {
    if format != OutputFormat::Human {
        bail!(gix::error::unsupported("Only 'human' format is currently supported"));
    }
    let path = path.as_ref();
    let buf = std::fs::read(path).or_raise(|| message!("Failed to read mailmap file at \"{}\"", path.display()))?;
    let mut errors = Vec::new();
    for err in gix::mailmap::parse(&buf).filter_map(Result::err) {
        writeln!(out, "{err}").or_error()?;
        errors.push(err);
    }

    let mut seen = HashSet::<(_, _)>::default();
    for entry in gix::mailmap::parse(&buf).filter_map(std::result::Result::ok) {
        if !seen.insert((entry.old_email(), entry.old_name())) {
            writeln!(
                out,
                "NOTE: entry ({:?}, {:?}) -> ({:?}, {:?}) is being overwritten",
                entry.old_email(),
                entry.old_name(),
                entry.new_email(),
                entry.new_name()
            )
            .or_error()?;
        }
    }

    if errors.is_empty() {
        writeln!(out, "{} lines OK", gix::mailmap::parse(&buf).count()).or_error()?;
        Ok(())
    } else {
        let context = message!("{} lines in \"{}\" could not be parsed", errors.len(), path.display()).corrupted();
        Err(Exn::raise_all(errors, context).into())
    }
}
