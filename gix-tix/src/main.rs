#![forbid(unsafe_code)]

use std::ffi::OsString;

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let (revisions, options, help) = arguments(gix::env::args_os().skip(1))?;
    if help {
        println!(
            "Usage: tix [--quit-on-finish] [-h|--hide REVSPEC] [REVISION]...\n\nBrowse commits reachable from HEAD or the given revisions.\n\nOptions:\n  -h, --hide REVSPEC  Hide this revision and all commits reachable from it\n      --help          Print help"
        );
        return Ok(());
    }

    let current_dir = std::env::current_dir().context("could not determine current directory")?;
    let repository = gix::ThreadSafeRepository::discover_with_environment_overrides(current_dir)
        .context("could not discover repository")?;
    gix_tix::run(repository, revisions, options)
}

fn arguments(mut args: impl Iterator<Item = OsString>) -> Result<(Vec<OsString>, gix_tix::Options, bool)> {
    let mut revisions = Vec::new();
    let mut options = gix_tix::Options::default();
    let mut help = false;
    while let Some(arg) = args.next() {
        if arg == "--help" {
            help = true;
            break;
        } else if arg == "--quit-on-finish" {
            options.quit_on_finish = true;
        } else if arg == "-h" || arg == "--hide" {
            let revision = args.next().context("-h/--hide requires a revision to hide")?;
            if revision == "--help" {
                help = true;
                break;
            }
            if revision == "-h" || revision == "--hide" || revision == "--quit-on-finish" {
                anyhow::bail!("-h/--hide requires a revision to hide");
            }
            options.hide.push(revision);
        } else {
            revisions.push(arg);
        }
    }
    Ok((revisions, options, help))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;

    #[test]
    fn separates_options_from_revisions() -> Result<()> {
        let (revisions, options, help) = arguments(
            ["--quit-on-finish", "-h", "main", "--hide", "tag", "topic", "--help"]
                .into_iter()
                .map(OsString::from),
        )?;

        assert!(options.quit_on_finish);
        assert_eq!(options.hide, ["main", "tag"], "both hide options are retained");
        assert_eq!(revisions, ["topic"], "only positional revisions remain");
        assert!(help, "--help remains available without claiming -h");
        assert!(
            arguments(["-h"].into_iter().map(OsString::from)).is_err(),
            "a missing hidden revision is rejected"
        );
        for args in [["--help", "-h"], ["-h", "--help"]] {
            assert!(
                arguments(args.into_iter().map(OsString::from))?.2,
                "--help wins regardless of its position"
            );
        }
        Ok(())
    }
}
