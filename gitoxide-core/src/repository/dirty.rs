use crate::OutputFormat;
use gix::{
    Result,
    error::{ResultExt, bail},
};

pub enum Mode {
    IsClean,
    IsDirty,
}

pub fn check(repo: gix::Repository, mode: Mode, out: &mut dyn std::io::Write, format: OutputFormat) -> Result<()> {
    if format != OutputFormat::Human {
        bail!(gix::error::unsupported("JSON output isn't implemented yet"));
    }
    let is_dirty = repo.is_dirty()?;
    let res = match (is_dirty, mode) {
        (false, Mode::IsClean) => Ok("The repository is clean"),
        (true, Mode::IsClean) => Err("The repository has changes"),
        (false, Mode::IsDirty) => Err("The repository is clean"),
        (true, Mode::IsDirty) => Ok("The repository has changes"),
    };

    let suffix = "(not counting untracked files)";
    match res {
        Ok(msg) => writeln!(out, "{msg} {suffix}").or_error()?,
        Err(msg) => bail!("{msg} {suffix}".conflict()),
    }
    Ok(())
}
