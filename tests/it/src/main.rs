use clap::Parser;

mod commands;

fn main() -> anyhow::Result<()> {
    let args: Args = Args::parse();
    match args.cmd {
        Subcommands::GitToSh {
            count,
            verbatim,
            output_dir,
            repo_dir,
            name,
            committish,
            patterns,
        } => commands::git_to_sh(
            &output_dir,
            &repo_dir,
            &name,
            &committish,
            std::io::stdout(),
            commands::git_to_sh::Options {
                patterns,
                verbatim,
                max_count: count,
            },
        ),
        Subcommands::CopyRoyal {
            dry_run,
            worktree_dir: worktree_root,
            destination_dir,
            patterns,
        } => commands::copy_royal(dry_run, &worktree_root, destination_dir, patterns),
        Subcommands::CheckMode {} => commands::check_mode(),
        Subcommands::Env {} => commands::env(),
    }
}

mod args;
use args::{Args, Subcommands};
