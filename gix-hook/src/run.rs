use crate::{Context, Options, OutputMode, Stream};
use bstr::BString;
use gix_error::{ErrorExt, Result, ResultExt, ensure, message, validation};
use std::{
    io::{self, Read, Write},
    path::Path,
    process::{Child, Command, ExitStatus, Output, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
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
        outputs.push(capture(command, options.stdin).or_raise(diagnostic)?);
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
    capture(prepare(path, context, options)?, options.stdin)
}

fn capture(command: Command, input: Option<&[u8]>) -> Result<Output> {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let status = run_prepared(
        command,
        input.map(ToOwned::to_owned),
        OutputMode::Separate,
        &AtomicBool::new(false),
        |stream, bytes| {
            match stream {
                Stream::Stdout => stdout.extend_from_slice(bytes),
                Stream::Stderr => stderr.extend_from_slice(bytes),
            }
            Ok(())
        },
    )?;
    Ok(Output { status, stdout, stderr })
}

/// Execute a prepared hook, streaming output and returning its exit status.
///
/// Use a command returned by [`prepare()`]. The command's stdin, stdout and stderr settings are replaced.
/// `input` is written on a separate
/// thread; `None` closes stdin. Output is drained concurrently with bounded buffering according to
/// `output`. Arguments, per-variable environment overrides and working directory are preserved during
/// Git's Unix shell fallback for executable-format errors. Other command customizations, including
/// `env_clear()` and Unix process credentials, cannot be inspected and are not retained by that fallback.
/// No signal handlers are installed.
///
/// A set `should_interrupt`, a callback error, or an I/O error stops execution and kills and reaps
/// the owned child. Already delivered output cannot be rolled back. Reader and writer threads own
/// only pipes, so cancellation never waits for descendants that inherited them. After a hook exits,
/// remaining output is drained, but a descendant holding only stdin does not delay completion.
/// Broken input pipes are ignored; nonzero exit statuses are returned for the caller to interpret.
pub fn run_prepared(
    command: Command,
    input: Option<Vec<u8>>,
    output: OutputMode,
    should_interrupt: &AtomicBool,
    mut on_output: impl FnMut(Stream, &[u8]) -> io::Result<()>,
) -> Result<ExitStatus> {
    interrupted(should_interrupt)?;
    let program = command.get_program().to_owned();
    let (child, readers) = spawn(command, input.is_some(), output)?;
    let mut child = ChildGuard(Some(child));
    let (input_sender, input_receiver) = mpsc::sync_channel(1);
    if let Some(input) = input {
        let mut stdin = child
            .0
            .as_mut()
            .expect("child is available until reaped")
            .stdin
            .take()
            .expect("hook input was configured as piped");
        std::thread::Builder::new()
            .name("gitoxide.hook.stdin".into())
            .spawn(move || {
                let _ = input_sender.send(stdin.write_all(&input));
            })
            .or_raise(|| message("Could not start hook input writer"))?;
    }
    let (sender, receiver) = mpsc::sync_channel(16);
    for (stream, mut reader) in readers {
        let sender = sender.clone();
        std::thread::Builder::new()
            .name("gitoxide.hook.output".into())
            .spawn(move || {
                let mut buffer = [0; 8192];
                loop {
                    let result = match reader.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(len) => Ok((stream, buffer[..len].to_vec())),
                        Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                        Err(err) => Err(err),
                    };
                    let failed = result.is_err();
                    if sender.send(result).is_err() || failed {
                        break;
                    }
                }
            })
            .or_raise(|| message("Could not start hook output reader"))?;
    }
    drop(sender);
    let communicate = || message("Could not communicate with hook").with_program(&program);
    let mut status = None;
    let mut output_done = false;
    while status.is_none() || !output_done {
        interrupted(should_interrupt)?;
        if !output_done {
            match receiver.recv_timeout(Duration::from_millis(20)) {
                Ok(bytes) => {
                    let (stream, bytes) = bytes.or_raise(communicate)?;
                    on_output(stream, &bytes).or_error()?;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => output_done = true,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        } else {
            std::thread::sleep(Duration::from_millis(20));
        }
        if status.is_none() {
            status = child
                .0
                .as_mut()
                .expect("child is available until reaped")
                .try_wait()
                .or_raise(communicate)?;
        }
    }
    child.0.take();
    if let Ok(Err(err)) = input_receiver.try_recv()
        && err.kind() != io::ErrorKind::BrokenPipe
    {
        return Err(err.and_raise(communicate()));
    }
    Ok(status.expect("loop waits for child exit"))
}

fn interrupted(should_interrupt: &AtomicBool) -> Result<()> {
    ensure!(
        !should_interrupt.load(Ordering::Relaxed),
        io::Error::new(io::ErrorKind::Interrupted, "Hook execution interrupted")
    );
    Ok(())
}

fn spawn(mut command: Command, has_input: bool, output: OutputMode) -> Result<(Child, Vec<(Stream, io::PipeReader)>)> {
    let (stdout, stdout_writer) = io::pipe().or_error()?;
    let (stderr, stderr_writer) = match output {
        OutputMode::Separate => {
            let (reader, writer) = io::pipe().or_error()?;
            (Some(reader), writer)
        }
        OutputMode::Combined => (None, stdout_writer.try_clone().or_error()?),
    };
    configure_io(&mut command, has_input, &stdout_writer, &stderr_writer).or_error()?;
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
        configure_io(&mut shell, has_input, &stdout_writer, &stderr_writer)?;
        shell.spawn()
    });
    let child = spawned.or_raise(|| message("Could not execute hook").with_program(command.get_program()))?;
    let readers = match stderr {
        Some(stderr) => vec![(Stream::Stdout, stdout), (Stream::Stderr, stderr)],
        None => vec![(Stream::Stderr, stdout)],
    };
    Ok((child, readers))
}

fn configure_io(
    command: &mut Command,
    has_input: bool,
    stdout: &io::PipeWriter,
    stderr: &io::PipeWriter,
) -> io::Result<()> {
    command
        .stdin(if has_input { Stdio::piped() } else { Stdio::null() })
        .stdout(stdout.try_clone()?)
        .stderr(stderr.try_clone()?);
    Ok(())
}

struct ChildGuard(Option<Child>);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
