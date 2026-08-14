use std::{ffi::OsStr, io::Write, process::Command};

use anyhow::{Context, Result};

pub(super) fn loaded_graph(repo: &gix::Repository) -> Result<crate::history::HistoryGraph> {
    use std::sync::atomic::AtomicBool;

    if repo.head_id().is_err() {
        return Ok(crate::history::HistoryGraph::default());
    }
    let authors = gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(
        crate::history::Authors::default(),
    ));
    let mut revisions = Vec::new();
    for reference in repo.references()?.all()? {
        let reference = reference.map_err(|err| anyhow::anyhow!("could not read reference: {err}"))?;
        let Some(id) = reference.try_id() else { continue };
        if reference.name().as_bstr() == b"HEAD"
            || repo
                .find_header(id)
                .context("could not inspect reference target")?
                .kind()
                != gix::object::Kind::Commit
        {
            continue;
        }
        revisions.push(
            gix::path::from_bstr(reference.name().as_bstr())
                .into_owned()
                .into_os_string(),
        );
    }
    if repo.head().is_ok_and(|head| head.referent_name().is_none()) {
        revisions.push("HEAD".into());
    }
    let mut graph = None;
    crate::history::load(
        repo,
        &revisions,
        &[],
        false,
        &authors,
        &AtomicBool::new(false),
        |event| {
            if let crate::history::Event::Complete(value) = event {
                graph = Some(value);
            }
            true
        },
    )?;
    graph.context("history traversal did not produce a graph")
}

pub(crate) mod create;
pub(crate) mod forget;
pub(crate) mod head;
pub(crate) mod rebase;
pub(crate) mod reword;
pub(crate) mod split;
pub(crate) mod time_travel;
pub(crate) mod todo;

#[tracing::instrument(skip_all, fields(filename))]
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

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    #[test]
    fn edit_graph_ignores_refs_that_do_not_point_to_commits() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        let output = Command::new("git")
            .arg("-C")
            .arg(fixture.path())
            .args(["update-ref", "refs/cache/tree", "HEAD^{tree}"])
            .output()?;
        assert!(
            output.status.success(),
            "the non-commit ref is created: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let repo = gix::open_opts(fixture.path(), gix::open::Options::isolated())?;
        let head = repo.head_id()?.detach();
        let graph = loaded_graph(&repo)?;
        assert!(graph.parents_of(head).is_some(), "HEAD remains part of the edit graph");
        Ok(())
    }
}
