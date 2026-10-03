#![forbid(unsafe_code)]

use gix::{
    Result,
    error::{ResultExt, message},
};

fn main() -> Result<()> {
    let command = gix_tix::command::parse();
    let current_dir = std::env::current_dir().or_raise(|| message("could not determine current directory"))?;
    let repository = gix::ThreadSafeRepository::discover_with_environment_overrides(current_dir)
        .or_raise(|| message("could not discover repository"))?;
    command.run(repository)
}
