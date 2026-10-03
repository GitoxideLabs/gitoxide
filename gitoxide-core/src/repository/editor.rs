use gix::{
    Result,
    error::{OptionExt, ResultExt, bail, message, unsupported},
};
use std::path::PathBuf;

pub fn function(repo: gix::Repository, paths: Vec<PathBuf>) -> Result<()> {
    let editor = repo
        .editor_command()
        .or_raise(|| message("Could not prepare editor"))?
        .ok_or_raise(|| unsupported("No editor is configured and the terminal is not capable of running one"))?;
    let editor_display = editor.command.to_string_lossy().into_owned();
    let mut command: std::process::Command = editor.args(paths).into();
    // Program metadata already names a directly launched editor. For shell commands, it only names
    // the shell, so retain the configured editor command in the message instead of losing that detail.
    let editor_display = if editor_display == command.get_program().to_string_lossy() {
        String::new()
    } else {
        format!(" {editor_display}")
    };
    let status = command
        .spawn()
        .or_raise(|| message!("Could not launch editor{editor_display}").with_program(command.get_program()))?
        .wait()
        .or_raise(|| message!("Could not wait for editor{editor_display}").with_program(command.get_program()))?;
    if !status.success() {
        bail!("Editor{editor_display} failed".with_command_status(&command, status));
    }
    Ok(())
}
