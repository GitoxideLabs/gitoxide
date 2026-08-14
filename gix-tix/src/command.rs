use std::ffi::OsString;

use anyhow::{Context, Result};
use clap::Parser;

/// Arguments and commands shared by the standalone `tix` binary and `gix tix`.
#[derive(Debug, clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Platform {
    /// Print help.
    #[arg(long, action = clap::ArgAction::HelpLong)]
    help: Option<bool>,
    /// Exit once all commits and graph lanes have been computed.
    #[arg(long)]
    quit_on_finish: bool,
    /// Add all worktree HEADs as visible traversal tips.
    #[arg(short = 'w', long)]
    worktrees: bool,
    /// Hide this revision and every commit reachable from it.
    #[arg(short = 'h', long, value_name = "REVSPEC")]
    hide: Vec<OsString>,
    #[command(subcommand)]
    command: Option<Command>,
    /// Revisions whose reachable commits should be shown, or HEAD if omitted.
    revisions: Vec<OsString>,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    /// Add staged changes, or worktree changes when nothing is staged, to HEAD.
    Amend,
    /// Move the changes introduced by HEAD into the worktree.
    Spill,
}

#[derive(Debug, clap::Parser)]
#[command(
    name = "tix",
    about = "Browse commits or edit the checked-out commit",
    disable_help_flag = true
)]
struct Cli {
    #[command(flatten)]
    platform: Platform,
}

/// Parse the standalone `tix` command line.
pub fn parse() -> Platform {
    Cli::parse_from(gix::env::args_os()).platform
}

impl Platform {
    /// Run this command against `repository`.
    pub fn run(self, repository: gix::ThreadSafeRepository) -> Result<()> {
        let Platform {
            help: _,
            quit_on_finish,
            worktrees,
            hide,
            command,
            revisions,
        } = self;
        let Some(command) = command else {
            return crate::run(
                repository,
                revisions,
                crate::Options {
                    quit_on_finish,
                    hide,
                    worktrees,
                },
            );
        };

        let (kind, verb) = match command {
            Command::Amend => (crate::edit::head::Kind::Amend, "amend"),
            Command::Spill => (crate::edit::head::Kind::Spill, "spill"),
        };
        let _log_guard = crate::logging::init().context("could not initialize tix diagnostics")?;
        let repository = repository.to_thread_local();
        let graph = crate::edit::loaded_view_graph(&repository)?;
        match crate::edit::head::perform(repository, &graph, kind, None)? {
            Some(id) => println!("{}", id.to_hex_with_len(7)),
            None => println!("nothing to {verb}"),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, error::ErrorKind};

    use super::*;

    #[test]
    fn clap_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_tui_options_and_top_level_commands() {
        let cli = Cli::try_parse_from(["tix", "--quit-on-finish", "-w", "-h", "main", "--hide", "tag", "topic"])
            .expect("TUI arguments parse");
        assert!(cli.platform.quit_on_finish);
        assert!(cli.platform.worktrees);
        assert_eq!(cli.platform.hide, ["main", "tag"], "hide options append");
        assert_eq!(
            cli.platform.revisions,
            ["topic"],
            "positional revisions remain visible tips"
        );
        assert!(cli.platform.command.is_none(), "omitting a command launches the TUI");

        assert!(matches!(
            Cli::try_parse_from(["tix", "amend"])
                .expect("amend parses")
                .platform
                .command,
            Some(Command::Amend)
        ));
        assert!(matches!(
            Cli::try_parse_from(["tix", "spill"])
                .expect("spill parses")
                .platform
                .command,
            Some(Command::Spill)
        ));
    }

    #[test]
    fn preserves_hide_and_help_semantics() {
        assert_eq!(
            Cli::try_parse_from(["tix", "--help"])
                .expect_err("long help exits through clap")
                .kind(),
            ErrorKind::DisplayHelp
        );
        assert_eq!(
            Cli::try_parse_from(["tix", "-h"])
                .expect_err("hide requires a value")
                .kind(),
            ErrorKind::InvalidValue
        );
        assert_eq!(
            Cli::try_parse_from(["tix", "amend", "topic"])
                .expect_err("commands reject TUI arguments")
                .kind(),
            ErrorKind::UnknownArgument
        );

        let cli = Cli::try_parse_from(["tix", "--", "amend"]).expect("-- makes amend a revision");
        assert!(cli.platform.command.is_none());
        assert_eq!(cli.platform.revisions, ["amend"]);
    }
}
