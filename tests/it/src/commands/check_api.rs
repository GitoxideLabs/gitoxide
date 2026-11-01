//! Run the shared API checker through the repository's existing CLI.
use std::{
    collections::BTreeSet,
    io::{self, Write},
    path::PathBuf,
};

use cargo_smart_release_core::api_check;

use crate::args::CheckApiCommand;

/// Run the release API guard or audit public dependency types.
///
/// With no subcommand, read a JSON release request from stdin. `--baseline-root PATH`
/// uses an existing baseline checkout instead of archiving the requested revision.
/// `compare CRATE [--baseline REV]` checks the working tree against a supported release or revision.
/// `audit JSON [CRATE,...]` inspects an existing rustdoc document, and `boundary MANIFEST`
/// audits the manifest's supported feature profiles and dependency type exceptions.
///
/// Exit with `0` for successful checks, `10` for incompatibility or a boundary violation,
/// and `2` when the check cannot complete.
pub fn check_api(baseline_override: Option<PathBuf>, command: Option<CheckApiCommand>) -> ! {
    let result = match command {
        Some(CheckApiCommand::Compare { krate, baseline }) => {
            api_check::check(&krate, None, baseline.as_deref()).map(|report| report.is_none())
        }
        Some(CheckApiCommand::Boundary { manifest }) => api_check::boundary(&manifest).map(|report| report.is_none()),
        Some(CheckApiCommand::Audit { json, allowed }) => {
            let allowed = allowed.map(|allowed| allowed.split(',').map(str::to_owned).collect::<BTreeSet<_>>());
            api_check::audit(&json, allowed.as_ref()).and_then(|audit| {
                let mut stdout = io::stdout().lock();
                serde_json::to_writer_pretty(&mut stdout, &audit.dependencies)?;
                writeln!(stdout)?;
                Ok(audit.disallowed.is_empty())
            })
        }
        None => serde_json::from_reader(io::stdin().lock())
            .map_err(Into::into)
            .and_then(|request| api_check::release(&request, baseline_override.as_deref()))
            .map(|report| report.is_none()),
    };
    std::process::exit(match result {
        Ok(true) => 0,
        Ok(false) => 10,
        Err(err) => {
            eprintln!("API check could not complete: {err:#}");
            2
        }
    });
}
