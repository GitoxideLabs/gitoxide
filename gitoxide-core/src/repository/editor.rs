use std::path::PathBuf;

use anyhow::{Context, Result, bail};

pub fn function(repo: gix::Repository, paths: Vec<PathBuf>) -> Result<()> {
    let editor = repo
        .editor_command()
        .context("Could not prepare editor")?
        .context("No editor is configured and the terminal is not capable of running one")?;
    let editor_display = editor.command.to_string_lossy().into_owned();
    let status = editor
        .args(paths)
        .spawn()
        .with_context(|| format!("Could not launch editor {editor_display}"))?
        .wait()
        .with_context(|| format!("Could not wait for editor {editor_display}"))?;
    if !status.success() {
        bail!("Editor {editor_display} exited with {status}");
    }
    Ok(())
}
