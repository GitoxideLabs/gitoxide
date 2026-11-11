//! Standalone entrypoint for the shared blocking filesystem-monitor daemon.

use anyhow::{Context, bail};
use gix::fsmonitor::daemon::Command;
use std::{ffi::OsString, io, path::PathBuf};

fn main() -> anyhow::Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let Some(command) = arguments.next() else {
        bail!("usage: gix-fsmonitor <run|start|stop|status|flush|query|hook|path> [--repo PATH]; use --help");
    };
    if command == "--help" || command == "-h" {
        println!("gix-fsmonitor <run|start|stop|status|flush|query [TOKEN]|hook 2 TOKEN|path> [--repo PATH]");
        println!("run, start, and hook accept --idle-timeout SECONDS (0 disables idle shutdown; default: 0)");
        return Ok(());
    }
    let mut repo = PathBuf::from(".");
    let mut idle_timeout = 0;
    let mut ready_file = None;
    let mut positional = Vec::<OsString>::new();
    while let Some(argument) = arguments.next() {
        if argument == "--repo" {
            repo = arguments.next().context("--repo needs a path")?.into();
        } else if argument == "--idle-timeout" {
            if !matches!(command.to_str(), Some("run" | "start" | "hook")) {
                bail!("--idle-timeout is only supported by run, start, and hook");
            }
            idle_timeout = arguments
                .next()
                .context("--idle-timeout needs a nonnegative number of seconds")?
                .to_str()
                .context("--idle-timeout must be a nonnegative number of seconds")?
                .parse::<u64>()
                .context("--idle-timeout must be a nonnegative number of seconds")?;
        } else if argument == "--ready-file" {
            if command != "run" {
                bail!("--ready-file is only supported by run");
            }
            ready_file = Some(arguments.next().context("--ready-file needs a path")?.into());
        } else {
            positional.push(argument);
        }
    }
    let command = match command.to_str() {
        Some("run") if positional.is_empty() => Command::Run {
            idle_timeout,
            ready_file,
        },
        Some("start") if positional.is_empty() => Command::Start { idle_timeout },
        Some("status") if positional.is_empty() => Command::Status,
        Some("path") if positional.is_empty() => Command::Path,
        Some("stop") if positional.is_empty() => Command::Stop,
        Some("flush") if positional.is_empty() => Command::Flush,
        Some("query") if positional.len() <= 1 => Command::Query {
            token: positional
                .first()
                .map(|value| value.as_encoded_bytes())
                .unwrap_or_default()
                .into(),
        },
        Some("hook") if positional.len() == 2 && positional[0] == "2" => Command::Hook {
            version: 2,
            token: positional[1].as_encoded_bytes().into(),
            idle_timeout,
        },
        _ => bail!("invalid command or arguments; use --help"),
    };
    let repo = gix::ThreadSafeRepository::discover_with_environment_overrides(repo)
        .context("opening the repository for filesystem monitoring")?
        .to_thread_local();
    repo.fsmonitor_daemon_execute(
        command,
        io::stdout().lock(),
        || false,
        |context| {
            Ok(vec![
                "run".into(),
                "--repo".into(),
                context.worktree.as_os_str().to_owned(),
            ])
        },
    )
    .map_err(Into::into)
}
