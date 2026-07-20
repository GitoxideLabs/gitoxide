use std::{collections::HashMap, io::Read, sync::Arc};

use bstr::{BStr, BString};

use crate::{
    Driver, driver,
    driver::{Operation, Process, State, process, process::client::invoke},
};

/// What to do if delay is supported by a process filter.
#[derive(Default, Debug, Copy, Clone)]
pub enum Delay {
    /// Use delayed processing for this entry.
    ///
    /// Note that it's up to the filter to determine whether or not the processing should be delayed.
    ///
    /// This is the default as the return value as the respective callers have to match on an enum that
    /// makes this possibility (and the special handling involved) obvious.
    #[default]
    Allow,
    /// Do not delay the processing, and force it to happen immediately. In this case, no delayed processing will occur
    /// even if the filter supports it.
    Forbid,
}

/// The error returned by [State::apply()][super::State::apply()].
// TODO(review): this implementation hand-preserves `#[error(transparent)]` semantics for `Init`:
//                `Display` passes the formatter through and `source()` forwards to the inner
//                error's source, exactly like the `thiserror`-generated code did.
#[derive(Debug)]
#[expect(missing_docs)]
pub enum Error {
    Init(driver::init::Error),
    WriteSource(std::io::Error),
    DelayNotAllowed,
    ProcessInvoke {
        source: process::client::invoke::Error,
        command: String,
    },
    ProcessStatus {
        status: driver::process::Status,
        command: String,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Init(err) => std::fmt::Display::fmt(err, f),
            Error::WriteSource(_) => f.write_str("Could not write entire object to driver"),
            Error::DelayNotAllowed => f.write_str("Filter process delayed an entry even though that was not requested"),
            Error::ProcessInvoke { command, .. } => write!(f, "Failed to invoke '{command}' command"),
            Error::ProcessStatus { status, command } => write!(
                f,
                "The invoked command '{command}' in process indicated an error: {status:?}"
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Init(err) => err.source(),
            Error::WriteSource(err) => Some(err),
            Error::DelayNotAllowed | Error::ProcessStatus { .. } => None,
            Error::ProcessInvoke { source, .. } => Some(source),
        }
    }
}

impl From<driver::init::Error> for Error {
    fn from(err: driver::init::Error) -> Self {
        Error::Init(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::WriteSource(err)
    }
}

/// Additional information for use in the [`State::apply()`] method.
#[derive(Debug, Copy, Clone)]
pub struct Context<'a, 'b> {
    /// The repo-relative using slashes as separator of the entry currently being processed.
    pub rela_path: &'a BStr,
    /// The name of the reference that `HEAD` is pointing to. It's passed to `process` filters if present.
    pub ref_name: Option<&'b BStr>,
    /// The root-level tree that contains the current entry directly or indirectly, or the commit owning the tree (if available).
    ///
    /// This is passed to `process` filters if present.
    pub treeish: Option<gix_hash::ObjectId>,
    /// The actual blob-hash of the data we are processing. It's passed to `process` filters if present.
    ///
    /// Note that this hash might be different from the `$Id$` of the respective `ident` filter, as the latter generates the hash itself.
    pub blob: Option<gix_hash::ObjectId>,
}

/// Apply operations to filter programs.
impl State {
    /// Apply `operation` of `driver` to the bytes read from `src` and return a reader to immediately consume the output
    /// produced by the filter. `rela_path` is the repo-relative path of the entry to handle.
    /// It's possible that the filter stays inactive, in which case the `src` isn't consumed and has to be used by the caller.
    ///
    /// Each call to this method will cause the corresponding filter to be invoked unless `driver` indicates a `process` filter,
    /// which is only launched once and maintained using this state.
    ///
    /// Note that it's not an error if there is no filter process for `operation` or if a long-running process doesn't support
    /// the desired capability.
    ///
    /// ### Deviation
    ///
    /// If a long-running process returns the 'abort' status after receiving the data, it will be removed similarly to how `git` does it.
    /// However, if it returns an unsuccessful error status later, it will not be removed, but reports the error only.
    /// If any other non-'error' status is received, the process will be stopped. But that doesn't happen if such a status is received
    /// after reading the filtered result.
    pub fn apply<'a>(
        &'a mut self,
        driver: &Driver,
        src: &mut impl std::io::Read,
        operation: Operation,
        ctx: Context<'_, '_>,
    ) -> Result<Option<Box<dyn std::io::Read + 'a>>, Error> {
        match self.apply_delayed(driver, src, operation, Delay::Forbid, ctx)? {
            Some(MaybeDelayed::Delayed(_)) => {
                unreachable!("we forbid delaying the entry")
            }
            Some(MaybeDelayed::Immediate(read)) => Ok(Some(read)),
            None => Ok(None),
        }
    }

    /// Like [`apply()`](Self::apply()), but use `delay` to determine if the filter result may be delayed or not.
    ///
    /// Poll [`list_delayed_paths()`](Self::list_delayed_paths()) until it is empty and query the available paths again.
    /// Note that even though it's possible, the API assumes that commands aren't mixed when delays are allowed.
    pub fn apply_delayed<'a>(
        &'a mut self,
        driver: &Driver,
        src: &mut impl std::io::Read,
        operation: Operation,
        delay: Delay,
        ctx: Context<'_, '_>,
    ) -> Result<Option<MaybeDelayed<'a>>, Error> {
        match self.maybe_launch_process(driver, operation, ctx.rela_path)? {
            Some(Process::SingleFile { mut child, command }) => {
                // To avoid deadlock when the filter immediately echoes input to output (like `cat`),
                // we need to write to stdin and read from stdout concurrently. If we write all data
                // to stdin before reading from stdout, and the pipe buffer fills up, both processes
                // will block: the filter blocks writing to stdout (buffer full), and we block writing
                // to stdin (waiting for the filter to consume data).
                //
                // Solution: Read all data into a buffer, then spawn a thread to write it to stdin
                // while we can immediately read from stdout.
                //
                // TODO(perf): This keeps the entire input in memory until the writer is done. For
                // required drivers, output remains streamed; for non-required drivers, the input is
                // retained as a fallback while the entire output is buffered, making peak storage
                // approximately input plus output (the `Arc` clone does not copy the input). Git's
                // `apply_single_file_filter()` can instead have its async worker copy directly from
                // an input file descriptor while the main thread reads output
                // (`convert.c::filter_buffer_or_fd()`), avoiding an input-sized allocation in that
                // case. Find a way to similarly pump the borrowed reader concurrently.
                let mut input_data = Vec::new();
                std::io::copy(src, &mut input_data)?;

                let stdin = child.stdin.take().expect("configured");
                let input_data: Arc<[u8]> = input_data.into();
                let fallback = (!driver.required).then(|| Arc::clone(&input_data));
                let write_thread = WriterThread::write_all_in_background(input_data, stdin)?;

                Ok(Some(MaybeDelayed::Immediate(Box::new(ReadFilterOutput {
                    inner: child.stdout.take(),
                    child: Some((child, command)),
                    write_thread: Some(write_thread),
                    fallback,
                    buffered: None,
                }))))
            }
            Some(Process::MultiFile { client, key }) => {
                let command = operation.as_str();
                if !client.capabilities().contains(command) {
                    return Ok(None);
                }

                let invoke_result = client.invoke(
                    command,
                    &mut [
                        ("pathname", Some(ctx.rela_path.to_owned())),
                        ("ref", ctx.ref_name.map(ToOwned::to_owned)),
                        ("treeish", ctx.treeish.map(|id| id.to_hex().to_string().into())),
                        ("blob", ctx.blob.map(|id| id.to_hex().to_string().into())),
                        (
                            "can-delay",
                            match delay {
                                Delay::Allow if client.capabilities().contains("delay") => Some("1".into()),
                                Delay::Forbid | Delay::Allow => None,
                            },
                        ),
                    ]
                    .into_iter()
                    .filter_map(|(key, value)| value.map(|v| (key, v))),
                    src,
                );
                let status = match invoke_result {
                    Ok(status) => status,
                    Err(err) => {
                        let invoke::Error::Io(io_err) = &err;
                        handle_io_err(io_err, &mut self.running, key.0.as_ref());
                        return Err(Error::ProcessInvoke {
                            command: command.into(),
                            source: err,
                        });
                    }
                };

                if status.is_delayed() {
                    if matches!(delay, Delay::Forbid) {
                        return Err(Error::DelayNotAllowed);
                    }
                    Ok(Some(MaybeDelayed::Delayed(key)))
                } else if status.is_success() {
                    // TODO: find a way to not have to do the 'borrow-dance'.
                    let client = self.running.remove(&key.0).expect("present for borrowcheck dance");
                    self.running.insert(key.0.clone(), client);
                    let client = self.running.get_mut(&key.0).expect("just inserted");

                    Ok(Some(MaybeDelayed::Immediate(Box::new(client.as_read()))))
                } else {
                    let message = status.message().unwrap_or_default();
                    match message {
                        "abort" => {
                            client.capabilities_mut().remove(command);
                        }
                        "error" => {}
                        _strange => {
                            let client = self.running.remove(&key.0).expect("we definitely have it");
                            client.into_child().kill().ok();
                        }
                    }
                    Err(Error::ProcessStatus {
                        command: command.into(),
                        status,
                    })
                }
            }
            None => Ok(None),
        }
    }
}

/// A type to represent delayed or immediate apply-filter results.
pub enum MaybeDelayed<'a> {
    /// Using the delayed protocol, this entry has been sent to a long-running process and needs to be
    /// checked for again, later, using the [`driver::Key`] to refer to the filter who owes a response.
    ///
    /// Note that the path to the entry is also needed to obtain the filtered result later.
    Delayed(driver::Key),
    /// The filtered result can be read from the contained reader right away.
    ///
    /// Note that it must be consumed in full or till a read error occurs.
    Immediate(Box<dyn std::io::Read + 'a>),
}

/// A helper to manage writing to stdin on a separate thread to avoid deadlock.
struct WriterThread {
    handle: Option<std::thread::JoinHandle<std::io::Result<()>>>,
}

impl WriterThread {
    /// Spawn a thread that will write all data from `data` to `stdin`.
    fn write_all_in_background(data: Arc<[u8]>, mut stdin: std::process::ChildStdin) -> std::io::Result<Self> {
        let handle = std::thread::Builder::new()
            .name("gix-filter-stdin-writer".into())
            .stack_size(128 * 1024)
            .spawn(move || {
                use std::io::Write;
                stdin.write_all(&data)?;
                // Explicitly drop stdin to close the pipe and signal EOF to the child
                drop(stdin);
                Ok(())
            })?;

        Ok(Self { handle: Some(handle) })
    }

    /// Wait for the writer thread to complete and return any error that occurred.
    fn join(&mut self) -> std::io::Result<()> {
        let Some(handle) = self.handle.take() else {
            return Ok(());
        };
        handle.join().map_err(|panic_info| {
            let msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                format!("Writer thread panicked: {s}")
            } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                format!("Writer thread panicked: {s}")
            } else {
                "Writer thread panicked while writing to filter stdin".to_string()
            };
            std::io::Error::other(msg)
        })?
    }
}

impl Drop for WriterThread {
    fn drop(&mut self) {
        // Best effort join on drop.
        if let Err(_err) = self.join() {
            gix_trace::debug!(err = %_err, "Failed to join writer thread during drop");
        }
    }
}

/// A utility type to facilitate streaming the output of a filter process.
struct ReadFilterOutput {
    inner: Option<std::process::ChildStdout>,
    /// Present until the process is waited on, then taken so subsequent reads don't wait again.
    child: Option<(std::process::Child, std::process::Command)>,
    /// The thread writing to stdin, if any. Must be joined when reading is done.
    write_thread: Option<WriterThread>,
    /// Original input to return if a non-required driver fails.
    fallback: Option<Arc<[u8]>>,
    /// Fully buffered output of a non-required driver, or its original input after failure.
    buffered: Option<std::io::Cursor<BufferedOutput>>,
}

enum BufferedOutput {
    Filtered(Vec<u8>),
    Original(Arc<[u8]>),
}

impl AsRef<[u8]> for BufferedOutput {
    fn as_ref(&self) -> &[u8] {
        match self {
            BufferedOutput::Filtered(data) => data,
            BufferedOutput::Original(data) => data,
        }
    }
}

impl ReadFilterOutput {
    /// Buffer all output to verify that the non-required driver succeeded before exposing it,
    /// falling back to the original input if reading, writing, or the process itself fails.
    fn buffer_non_required_driver(&mut self, fallback: Arc<[u8]>) -> &mut std::io::Cursor<BufferedOutput> {
        let mut output = Vec::new();
        let read_result = self.inner.take().expect("configured").read_to_end(&mut output);
        let write_result = self.write_thread.take().map_or(Ok(()), |mut thread| thread.join());
        let (mut child, _command) = self.child.take().expect("configured");
        let status = child.wait();
        // A filter may deliberately close stdin before consuming all input, which makes the writer
        // see a broken pipe even though the filter produced valid output and exited successfully.
        // Other write failures invalidate the filtered output; read and exit status are checked below.
        let write_succeeded = match &write_result {
            Ok(()) => true,
            Err(err) => err.kind() == std::io::ErrorKind::BrokenPipe,
        };
        let succeeded =
            read_result.is_ok() && write_succeeded && status.as_ref().is_ok_and(std::process::ExitStatus::success);

        if !succeeded {
            gix_trace::debug!(
                ?_command,
                ?read_result,
                ?write_result,
                ?status,
                "Non-required filter driver failed; using original input"
            );
        }
        self.buffered = Some(std::io::Cursor::new(if succeeded {
            BufferedOutput::Filtered(output)
        } else {
            BufferedOutput::Original(fallback)
        }));
        self.buffered.as_mut().expect("just initialized")
    }
}

pub(crate) fn handle_io_err(err: &std::io::Error, running: &mut HashMap<BString, process::Client>, process: &BStr) {
    if matches!(
        err.kind(),
        std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::UnexpectedEof
    ) {
        running.remove(process).expect("present or we wouldn't be here");
    }
}

impl std::io::Read for ReadFilterOutput {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if let Some(buffered) = self.buffered.as_mut() {
            return buffered.read(buf);
        }
        if let Some(fallback) = self.fallback.take() {
            return self.buffer_non_required_driver(fallback).read(buf);
        }

        match self.inner.as_mut() {
            Some(inner) => {
                let num_read = match inner.read(buf) {
                    Ok(n) => n,
                    Err(e) => {
                        // On read error, ensure we join the writer thread before propagating the error.
                        // This is expected to finish with failure as well as it should fail to write
                        // to the process which now fails to produce output (that we try to read).
                        if let Some(mut write_thread) = self.write_thread.take() {
                            // Try to join but prioritize the original read error
                            if let Err(_thread_err) = write_thread.join() {
                                gix_trace::debug!(thread_err = %_thread_err, read_err = %e, "write to stdin error during failed read");
                            }
                        }
                        return Err(e);
                    }
                };

                if num_read == 0 {
                    self.inner.take();

                    // Join the writer thread first to ensure all data has been written
                    // and that resources are freed now.
                    let write_result = self.write_thread.take().map_or(Ok(()), |mut thread| thread.join());

                    if let Some((mut child, cmd)) = self.child.take() {
                        let status = child.wait()?;
                        if !status.success() {
                            return Err(std::io::Error::other(format!("Driver process {cmd:?} failed")));
                        }
                    }

                    write_result?;
                }
                Ok(num_read)
            }
            None => Ok(0),
        }
    }
}
