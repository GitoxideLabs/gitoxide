use crate::{Context, Options};
use gix_error::{Result, ensure, validation};
use std::{path::Path, process::Command};

/// Prepare a traditional hook with resolved paths, literal arguments and environment overrides.
///
/// Relative hook paths start at the absolute working directory. Interpreter lookup follows
/// `gix-command`; arguments and environment are applied to the prepared command afterward.
pub fn prepare(path: &Path, context: &Context<'_>, options: Options<'_>) -> Result<Command> {
    ensure!(
        context.working_dir.is_absolute() && context.common_dir.is_absolute(),
        validation("Hook working and common directories must be absolute")
    );
    let mut command =
        Command::try_from(gix_command::prepare(context.working_dir.join(path)).with_context(context.command.clone()))?;
    command
        .current_dir(context.working_dir)
        .env("GIT_COMMON_DIR", context.common_dir);
    if context.command.worktree_dir.is_none() {
        command.env_remove("GIT_WORK_TREE");
    }
    command
        .args(options.arguments)
        .envs(options.environment.iter().copied());
    Ok(command)
}
