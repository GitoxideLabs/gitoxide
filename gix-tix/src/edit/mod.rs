use std::{ffi::OsStr, io::Write, process::Command};

use anyhow::{Context, Result};

pub(crate) mod create;
pub(crate) mod refs;
pub(crate) mod reword;
pub(crate) mod time_travel;

pub(crate) fn edit_document(
    terminal: &mut ratatui::DefaultTerminal,
    editor: &OsStr,
    document: &[u8],
    filename: &str,
    enhanced_keyboard: bool,
) -> Result<Option<Vec<u8>>> {
    let mut tempfile = gix::tempfile::writable_at(
        std::env::temp_dir().join(filename),
        gix::tempfile::ContainingDirectory::Exists,
        gix::tempfile::AutoRemove::Tempfile,
    )
    .context("could not create commit message file")?
    .take()
    .context("commit message file disappeared")?;
    tempfile
        .write_all(document)
        .context("could not write commit message file")?;
    tempfile.flush().context("could not flush commit message file")?;

    if editor != ":" {
        crate::with_suspended_terminal(terminal, enhanced_keyboard, || {
            let status = Command::from(
                gix::command::prepare(editor)
                    .arg(tempfile.path())
                    .command_may_be_shell_script_allow_manual_argument_splitting(),
            )
            .status()
            .with_context(|| format!("could not launch Git editor {}", editor.to_string_lossy()))?;
            if !status.success() {
                anyhow::bail!("Git editor {} exited with {status}", editor.to_string_lossy());
            }
            Ok(())
        })?;
    }
    let edited = std::fs::read(tempfile.path()).context("could not read edited commit message")?;
    Ok((edited != document).then_some(edited))
}
