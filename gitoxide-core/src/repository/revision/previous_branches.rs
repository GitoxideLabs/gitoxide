use crate::OutputFormat;
use gix::{
    Result,
    error::{OptionExt, ResultExt},
};

pub fn previous_branches(repo: gix::Repository, mut out: impl std::io::Write, format: OutputFormat) -> Result<()> {
    let branches = repo
        .head()?
        .prior_checked_out_branches()?
        .ok_or_raise(|| gix::error::not_found("The reflog for HEAD is required"))?;
    match format {
        OutputFormat::Human => {
            for (name, id) in branches {
                writeln!(out, "{id} {name}").or_error()?;
            }
        }
        #[cfg(feature = "serde")]
        OutputFormat::Json => {
            serde_json::to_writer_pretty(&mut out, &branches).or_error()?;
        }
    }
    Ok(())
}
