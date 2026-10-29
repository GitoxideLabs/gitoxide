use std::{
    ffi::OsString,
    io,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

use crate::{
    Repository, Result,
    bstr::BString,
    error::{OptionExt, message},
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
        repo.require_hook_trust()?;
        let directory = repo.hooks_directory_in(&repo.current_dir().join(repo.git_dir()))?;
        Ok(Self {
            repo,
            directory,
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
        Ok(self
            .run_with_worktree(name, args, input, quarantine, None, progress)?
            .unwrap_or(true))
    }

    /// Return `None` when no executable deployment hook is configured.
    pub fn push_to_checkout(
        &self,
        worktree: &Repository,
        new_id: gix_hash::ObjectId,
        progress: &mut impl FnMut(bool, &[u8]) -> io::Result<()>,
    ) -> Result<Option<bool>> {
        self.run_with_worktree(
            "push-to-checkout",
            [new_id.to_string().into()],
            Vec::new(),
            None,
            Some(worktree),
            progress,
        )
    }

    fn run_with_worktree(
        &self,
        name: &str,
        args: impl IntoIterator<Item = OsString>,
        input: Vec<u8>,
        quarantine: Option<&Path>,
        worktree: Option<&Repository>,
        progress: &mut impl FnMut(bool, &[u8]) -> io::Result<()>,
    ) -> Result<Option<bool>> {
        push::plan::check_interrupt(self.interrupt)?;
        let Some(path) = gix_hook::find(name, &self.directory)? else {
            return Ok(None);
        };
        let mut context = self.repo.command_context()?;
        context.stderr = None;
        context.no_replace_objects = Some(true);
        let args: Vec<_> = args.into_iter().collect();
        let args: Vec<_> = args.iter().map(OsString::as_os_str).collect();
        let environment: Vec<_> = self
            .environment
            .iter()
            .map(|(key, value)| (key.as_os_str(), value.as_os_str()))
            .collect();
        let mut command = gix_hook::prepare(
            &path,
            &gix_hook::Context {
                working_dir: &self.repo.current_dir().join(self.repo.git_dir()),
                common_dir: &self.repo.current_dir().join(self.repo.common_dir()),
                command: context,
            },
            gix_hook::Options {
                arguments: &args,
                environment: &environment,
                stdin: None,
            },
        )?;
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
        if let Some(worktree) = worktree {
            let root = worktree
                .workdir()
                .ok_or_raise(|| message("A deployment hook requires a worktree").validation())?;
            command
                .env("GIT_DIR", worktree.current_dir().join(worktree.git_dir()))
                .env("GIT_WORK_TREE", worktree.current_dir().join(root));
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
        let status = gix_hook::run_prepared(
            command,
            Some(input),
            gix_hook::OutputMode::Combined,
            self.interrupt,
            |_, bytes| progress(false, bytes),
        )?;
        Ok(Some(status.success()))
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
