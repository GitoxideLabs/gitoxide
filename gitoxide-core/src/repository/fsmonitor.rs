//! CLI adaptation of the shared blocking filesystem-monitor daemon.

use std::{
    io::Write,
    sync::atomic::{AtomicBool, Ordering},
};

use gix::{
    Result,
    bstr::{BString, ByteSlice},
};

pub use gix::fsmonitor::daemon::Command;

pub fn execute(
    repo: gix::Repository,
    command: Command,
    config: Vec<BString>,
    out: impl Write,
    should_interrupt: &AtomicBool,
) -> Result<()> {
    repo.fsmonitor_daemon_execute(
        command,
        out,
        || should_interrupt.load(Ordering::Relaxed) || gix::interrupt::is_triggered(),
        |context| {
            let mut arguments = vec![
                "--no-verbose".into(),
                "--repository".into(),
                context.worktree.as_os_str().to_owned(),
            ];
            for value in config {
                arguments.push("--config".into());
                arguments.push(gix::path::from_bstr(value.as_bstr())?.into_owned().into_os_string());
            }
            arguments.extend(["fsmonitor".into(), "run".into()]);
            Ok(arguments)
        },
    )
}
