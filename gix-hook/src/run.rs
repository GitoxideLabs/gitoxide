use crate::{Context, Options};
use gix_error::{ErrorExt, Result, ResultExt, ensure, message, validation};
use std::{
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
};

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

/// Execute the traditional hook at `path`, capturing stdout, stderr and exit status.
///
/// Preparation preserves literal paths, arguments and environment through [`Context`] and [`Options`]. On Unix,
/// executable-format errors alone are retried through Git's shell, supporting scripts without a
/// shebang. Windows interpreter lookup is performed during preparation and is not retried here.
///
/// Input is written on a real thread while both output pipes drain, including in serial builds.
/// Broken input pipes do not hide the hook's output or status. Nonzero exit statuses remain in the
/// returned output; launch and communication failures are errors.
pub fn run(path: &Path, context: &Context<'_>, options: Options<'_>) -> Result<Output> {
    run_prepared(prepare(path, context, options)?, options.stdin)
}

fn run_prepared(mut command: Command, input: Option<&[u8]>) -> Result<Output> {
    configure_io(&mut command, input.is_some());
    let spawned = command.spawn();
    #[cfg(unix)]
    let spawned = spawned.or_else(|err| {
        if err.raw_os_error() != Some(libc::ENOEXEC) {
            return Err(err);
        }
        let mut shell = gix_path::env::shell_command();
        shell.arg(command.get_program()).args(command.get_args());
        if let Some(dir) = command.get_current_dir() {
            shell.current_dir(dir);
        }
        for (key, value) in command.get_envs() {
            match value {
                Some(value) => shell.env(key, value),
                None => shell.env_remove(key),
            };
        }
        configure_io(&mut shell, input.is_some());
        shell.spawn()
    });
    let child = spawned.or_raise(|| message("Could not execute hook").with_program(command.get_program()))?;
    let communicate = || message("Could not communicate with hook").with_program(command.get_program());
    let Some(input) = input else {
        return child.wait_with_output().or_raise(communicate);
    };
    let mut child = child;
    let mut stdin = child.stdin.take().expect("hook input was configured as piped");
    // Always use a real thread: serial feature configurations must still drain output while writing input.
    std::thread::scope(|scope| {
        let writer = match std::thread::Builder::new()
            .name("gitoxide.hook.stdin".into())
            .spawn_scoped(scope, move || stdin.write_all(input))
        {
            Ok(writer) => writer,
            Err(source) => {
                let killed = child.kill();
                let waited = child.wait();
                let mut err = source.and_raise(message("Could not start hook input writer"));
                if let Err(source) = killed {
                    err = err.and_raise(source);
                }
                if let Err(source) = waited {
                    err = err.and_raise(source);
                }
                return Err(err);
            }
        };
        let output = child.wait_with_output();
        let written = writer
            .join()
            .map_err(|_| message("Hook input writer panicked").raise())?;
        if let Err(err) = written
            && err.kind() != std::io::ErrorKind::BrokenPipe
        {
            return Err(err.and_raise(communicate()));
        }
        output.or_raise(communicate)
    })
}

fn configure_io(command: &mut Command, has_input: bool) {
    command
        .stdin(if has_input { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
}
