#![forbid(unsafe_code)]

use std::ffi::OsString;

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let mut args = gix::env::args_os().skip(1).peekable();
    let edit = if args.peek().is_some_and(|arg| arg == "edit") {
        args.next();
        let edit = match args.next().as_deref() {
            Some(value) if value == "amend" => gix_tix::HeadEdit::Amend,
            Some(value) if value == "spill" => gix_tix::HeadEdit::Spill,
            _ => anyhow::bail!("usage: tix edit amend|spill"),
        };
        if args.next().is_some() {
            anyhow::bail!("usage: tix edit amend|spill");
        }
        Some(edit)
    } else {
        None
    };
    let (revisions, options, help) = arguments(args)?;
    if help {
        println!(
            "Usage: tix [--quit-on-finish] [-w|--worktrees] [-h|--hide REVSPEC] [REVISION]...\n       tix edit amend|spill\n\nBrowse commits reachable from HEAD or edit its checked-out commit.\n\nOptions:\n  -h, --hide REVSPEC  Hide this revision and all commits reachable from it\n  -w, --worktrees     Add all worktree HEADs as visible tips\n      --help          Print help"
        );
        return Ok(());
    }

    let current_dir = std::env::current_dir().context("could not determine current directory")?;
    let repository = gix::ThreadSafeRepository::discover_with_environment_overrides(current_dir)
        .context("could not discover repository")?;
    if let Some(edit) = edit {
        match gix_tix::edit_head(repository, edit)? {
            Some(id) => println!("{}", id.to_hex_with_len(7)),
            None => println!(
                "nothing to {}",
                if edit == gix_tix::HeadEdit::Amend {
                    "amend"
                } else {
                    "spill"
                }
            ),
        }
        return Ok(());
    }
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
        } else if arg == "-w" || arg == "--worktrees" {
            options.worktrees = true;
        } else if arg == "-h" || arg == "--hide" {
            let revision = args.next().context("-h/--hide requires a revision to hide")?;
            if revision == "--help" {
                help = true;
                break;
            }
            if revision == "-h"
                || revision == "--hide"
                || revision == "-w"
                || revision == "--worktrees"
                || revision == "--quit-on-finish"
            {
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
            [
                "--quit-on-finish",
                "-w",
                "-h",
                "main",
                "--hide",
                "tag",
                "topic",
                "--help",
            ]
            .into_iter()
            .map(OsString::from),
        )?;

        assert!(options.quit_on_finish);
        assert!(options.worktrees);
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
        assert!(
            arguments(["--hide", "--worktrees"].into_iter().map(OsString::from)).is_err(),
            "worktree options cannot be consumed as hidden revisions"
        );
        Ok(())
    }
}
