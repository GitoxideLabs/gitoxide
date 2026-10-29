use std::{
    ffi::OsString,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{atomic::AtomicBool, mpsc},
    time::Duration,
};

use crate::{
    Repository, Result,
    bstr::BString,
    error::{ResultExt, message},
    push::{self, UpdateOutcome},
};

pub(super) struct Hooks<'a> {
    repo: &'a Repository,
    directory: PathBuf,
    push_options: Option<&'a [BString]>,
    interrupt: &'a AtomicBool,
    environment: &'a [(OsString, OsString)],
    shallow_file: Option<&'a Path>,
}

impl<'a> Hooks<'a> {
    pub fn new(
        repo: &'a Repository,
        push_options: Option<&'a [BString]>,
        interrupt: &'a AtomicBool,
        environment: &'a [(OsString, OsString)],
    ) -> Result<Self> {
        let directory = repo
            .config_snapshot()
            .trusted_path("core.hooksPath")?
            .map_or_else(|| repo.common_dir().join("hooks"), |path| repo.git_dir().join(path));
        Ok(Self {
            repo,
            directory: repo.current_dir().join(directory),
            push_options,
            interrupt,
            environment,
            shallow_file: None,
        })
    }

    pub fn with_shallow_file(mut self, path: Option<&'a Path>) -> Self {
        self.shallow_file = path;
        self
    }

    pub fn run(
        &self,
        name: &str,
        args: impl IntoIterator<Item = OsString>,
        input: Vec<u8>,
        quarantine: Option<&Path>,
        progress: &mut impl FnMut(bool, &[u8]) -> io::Result<()>,
    ) -> Result<bool> {
        push::plan::check_interrupt(self.interrupt)?;
        let path = self.directory.join(name);
        match std::fs::metadata(&path) {
            Ok(metadata) if metadata.is_file() && executable(&metadata) => {}
            Ok(_) => return Ok(true),
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(true),
            Err(err) => return Err(err).or_error(),
        }
        let mut context = self.repo.command_context()?;
        context.stderr = None;
        context.no_replace_objects = Some(true);
        let (mut output, write_output) = io::pipe().or_error()?;
        let mut prepare = gix_command::prepare(path)
            .with_context(context)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(write_output.try_clone().or_error()?))
            .stderr(Stdio::from(write_output));
        prepare.env.extend_from_slice(self.environment);
        let mut command: Command = prepare.try_into()?;
        command.current_dir(self.repo.git_dir());
        // Repository-local process overrides belong to the sender, never the receiver.
        for key in [
            "GIT_COMMON_DIR",
            "GIT_INDEX_FILE",
            "GIT_OBJECT_DIRECTORY",
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            "GIT_SHALLOW_FILE",
            "GIT_QUARANTINE_PATH",
            "GIT_CONFIG_PARAMETERS",
            "GIT_CONFIG_COUNT",
        ] {
            command.env_remove(key);
        }
        if self.repo.workdir().is_none() {
            command.env_remove("GIT_WORK_TREE");
        }
        if self.repo.refs.namespace.is_none() {
            command.env_remove("GIT_NAMESPACE");
        }
        if let Some(quarantine) = quarantine {
            command
                .env("GIT_OBJECT_DIRECTORY", quarantine)
                .env("GIT_QUARANTINE_PATH", quarantine)
                .env(
                    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
                    quoted_path(&self.repo.current_dir().join(self.repo.objects.store_ref().path()))?,
                );
        }
        if let Some(path) = self.shallow_file {
            command.env("GIT_SHALLOW_FILE", self.repo.current_dir().join(path));
        }
        // Push options are passed only to the two receive hooks, as in receive-pack.
        command.env_remove("GIT_PUSH_OPTION_COUNT");
        if matches!(name, "pre-receive" | "post-receive")
            && let Some(options) = self.push_options
        {
            command.env("GIT_PUSH_OPTION_COUNT", options.len().to_string());
            for (index, option) in options.iter().enumerate() {
                command.env(
                    format!("GIT_PUSH_OPTION_{index}"),
                    gix_path::from_bstr(option)?.as_os_str(),
                );
            }
        }
        let child = command.spawn().or_raise(|| message!("Could not start {name} hook"))?;
        drop(command);
        let mut child = ChildGuard(Some(child));
        let stdin = child.0.as_mut().expect("child is available until reaped").stdin.take();
        let (input_sender, input_receiver) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let result = stdin.map_or(Ok(()), |mut stdin| stdin.write_all(&input));
            let _ = input_sender.send(result);
        });
        // Own only pipe handles on these threads, so cancellation never waits on a hook descendant
        // that inherited a pipe. The bounded channel prevents unbounded progress buffering.
        let (sender, receiver) = mpsc::sync_channel(16);
        std::thread::spawn(move || {
            let mut buffer = [0; 8192];
            loop {
                let result = match output.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(len) => Ok(buffer[..len].to_vec()),
                    Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                    Err(err) => Err(err),
                };
                let failed = result.is_err();
                if sender.send(result).is_err() || failed {
                    break;
                }
            }
        });
        let mut status = None;
        let mut output_done = false;
        while status.is_none() || !output_done {
            push::plan::check_interrupt(self.interrupt)?;
            if !output_done {
                match receiver.recv_timeout(Duration::from_millis(20)) {
                    Ok(bytes) => progress(false, &bytes.or_error()?).or_error()?,
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
                    .or_error()?;
            }
        }
        child.0.take();
        let success = status.expect("loop waits for child exit").success();
        // The hook's exit status decides whether it accepted its input. A descendant may keep
        // stdin open after the hook and its output have finished; never wait for that descendant.
        if success
            && let Ok(Err(err)) = input_receiver.try_recv()
            && err.kind() != io::ErrorKind::BrokenPipe
        {
            return Err(err).or_raise(|| message!("Could not send input to {name} hook"));
        }
        Ok(success)
    }
}

pub(super) fn input(
    updates: impl IntoIterator<Item = impl std::borrow::Borrow<UpdateOutcome>>,
    hash: gix_hash::Kind,
) -> Vec<u8> {
    let mut out = Vec::new();
    for update in updates {
        let update = update.borrow();
        out.extend_from_slice(
            format!(
                "{} {} ",
                update.old_id.unwrap_or_else(|| hash.null()),
                update.update.new_id.unwrap_or_else(|| hash.null())
            )
            .as_bytes(),
        );
        out.extend_from_slice(update.update.destination.as_bstr());
        out.push(b'\n');
    }
    out
}

fn executable(metadata: &std::fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        gix_fs::is_executable(metadata)
    }
    #[cfg(not(unix))]
    {
        metadata.is_file()
    }
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

pub(super) fn quoted_path(path: &Path) -> Result<OsString> {
    let bytes = gix_path::into_bstr(path)?;
    let quoted = gix_quote::ansi_c::quote(bytes.as_ref());
    let quoted = if quoted.starts_with(b"\"") {
        quoted.into_owned()
    } else {
        let mut out = BString::from("\"");
        out.extend_from_slice(&quoted);
        out.push(b'"');
        out
    };
    Ok(gix_path::from_bstr(quoted)?.into_owned().into_os_string())
}
