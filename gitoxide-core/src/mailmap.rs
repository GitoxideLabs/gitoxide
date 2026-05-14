use std::{collections::HashSet, io::Write, path::Path};

use anyhow::{Context, bail};

use crate::OutputFormat;

pub const PROGRESS_RANGE: std::ops::RangeInclusive<u8> = 1..=2;

pub fn verify(path: impl AsRef<Path>, format: OutputFormat, mut out: impl Write) -> anyhow::Result<()> {
    if format != OutputFormat::Human {
        bail!("Only 'human' format is currently supported");
    }
    let path = path.as_ref();
    let buf = std::fs::read(path).with_context(|| format!("Failed to read mailmap file at '{}'", path.display()))?;
    let mut err_count = 0;
    for err in gix::mailmap::parse(&buf).filter_map(Result::err) {
        err_count += 1;
        writeln!(out, "{err}")?;
    }

    let mut seen = HashSet::<(_, _)>::default();
    for entry in gix::mailmap::parse(&buf).filter_map(Result::ok) {
        if !seen.insert((entry.old_email(), entry.old_name())) {
            writeln!(
                out,
                "NOTE: entry ({:?}, {:?}) -> ({:?}, {:?}) is being overwritten",
                entry.old_email(),
                entry.old_name(),
                entry.new_email(),
                entry.new_name()
            )?;
        }
    }

    if err_count == 0 {
        writeln!(out, "{} lines OK", gix::mailmap::parse(&buf).count())?;
        Ok(())
    } else {
        bail!("{} lines in '{}' could not be parsed", err_count, path.display());
    }
}
