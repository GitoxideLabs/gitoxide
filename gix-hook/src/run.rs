use crate::{Context, Options};
use bstr::BString;
use gix_error::{ErrorExt, Result, ResultExt, ensure, message, validation};
use std::{
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
};

/// Prepare the traditional hook at `path` using the resolved execution `context`.
///
/// Relative paths start at `context.working_dir`. Paths and arguments are literal. Environment
/// overrides reach Windows interpreter lookup and take precedence over repository context afterward.
/// Standard input is closed by default and stdout is captured; stderr follows command context.
/// Use [`run()`] for captured stderr, piped input, and Unix execution-time shell fallback.
pub fn prepare(path: &Path, context: &Context<'_>, options: Options<'_>) -> Result<Command> {
    command(gix_command::prepare(context.working_dir.join(path)), context, options)
}

fn command(preparation: gix_command::Prepare, context: &Context<'_>, options: Options<'_>) -> Result<Command> {
    ensure!(
        context.working_dir.is_absolute() && context.common_dir.is_absolute(),
        validation("Hook working and common directories must be absolute")
    );
    let mut preparation = preparation
        .current_dir(context.working_dir)
        .with_context(context.command.clone())
        .env("GIT_COMMON_DIR", context.common_dir)
        .args(options.arguments.iter().copied());
    for (name, value) in options.environment {
        preparation = preparation.env(*name, *value);
    }
    let mut command = Command::try_from(preparation)?;
    if context.command.worktree_dir.is_none() {
        command.env_remove("GIT_WORK_TREE");
    }
    // PATH participates in Windows lookup before conversion; explicit values win over context afterward.
    command.envs(options.environment.iter().copied());
    Ok(command)
}

/// Run resolved configured commands followed by the traditional hook for `event`, serially.
///
/// `configured` comes from [`crate::configured_commands()`]; `None` disables the entire event.
/// Each hook receives identical arguments, environment, and a fresh copy of stdin. Exit failures
/// are retained in order and do not stop later hooks; preparation, launch, and communication errors
/// stop execution. Neither parallel configuration nor a hook's nonzero exit status changes this order.
/// Relative `hooks_directory` paths start at `context.working_dir` for both discovery and execution.
/// The caller selects directories for the event, for example using [`crate::is_receive_event()`].
pub fn run_event_serially(
    event: &str,
    configured: Option<&[(BString, BString)]>,
    hooks_directory: &Path,
    context: &Context<'_>,
    options: Options<'_>,
) -> Result<Vec<Output>> {
    crate::validate_name(event)?;
    let Some(configured) = configured else {
        return Ok(Vec::new());
    };
    let hooks_directory = context.working_dir.join(hooks_directory);
    let traditional = crate::find(event, &hooks_directory)?;
    let mut outputs = Vec::with_capacity(configured.len() + usize::from(traditional.is_some()));
    for (name, script) in configured {
        let diagnostic = || message!("Could not run configured hook {name:?} for event {event:?}");
        let preparation = gix_command::prepare(gix_path::from_bstr(script)?.into_owned())
            .command_may_be_shell_script_disallow_manual_argument_splitting();
        let command = command(preparation, context, options).or_raise(diagnostic)?;
        outputs.push(run_prepared(command, options.stdin).or_raise(diagnostic)?);
    }
    if let Some(path) = traditional {
        outputs.push(run(&path, context, options)?);
    }
    Ok(outputs)
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
