#![forbid(unsafe_code)]

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let command = gix_tix::command::parse();
    if !command.requires_repository() {
        return command.run_without_repository(gix_tix::command::Invocation::Tix);
    }
    let current_dir = std::env::current_dir().context("could not determine current directory")?;
    let repository = gix::ThreadSafeRepository::discover_with_environment_overrides(current_dir)
        .context("could not discover repository")?;
    command.run(repository)
}
