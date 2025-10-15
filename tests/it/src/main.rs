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
        Subcommands::BlameCopyRoyal {
            dry_run,
            worktree_dir: worktree_root,
            destination_dir,
            asset_dir,
            file,
            verbatim,
        } => commands::blame_copy_royal(
            dry_run,
            &worktree_root,
            destination_dir,
            asset_dir,
            &file,
            commands::blame_copy_royal::Options { verbatim },
        ),
        Subcommands::CopyRoyal {
            dry_run,
            worktree_dir: worktree_root,
            destination_dir,
            patterns,
        } => commands::copy_royal(dry_run, &worktree_root, destination_dir, patterns),
        Subcommands::CreateDiffCases {
            dry_run,
            sliders_file,
            worktree_dir,
            destination_dir,
            count,
            asset_dir,
        } => commands::create_diff_cases(dry_run, sliders_file, &worktree_dir, destination_dir, count, asset_dir),
        Subcommands::CheckMode {} => commands::check_mode(),
        Subcommands::Env {} => commands::env(),
    }
}

mod args;
use args::{Args, Subcommands};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clap() {
        use clap::CommandFactory;
        Args::command().debug_assert();
    }
}
