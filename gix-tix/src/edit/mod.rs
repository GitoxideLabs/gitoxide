use std::{io::Write, process::Command};

use anyhow::{Context, Result};

pub(super) fn loaded_graph(repo: &gix::Repository) -> Result<crate::history::HistoryGraph> {
    if repo.head_id().is_err() {
        return Ok(crate::history::HistoryGraph::default());
    }
    let mut revisions = Vec::new();
    for reference in repo.references()?.all()? {
        let reference = reference.map_err(|err| anyhow::anyhow!("could not read reference: {err}"))?;
        if reference.name().as_bstr().starts_with(crate::history::STASH_PREFIX)
            || reference
                .name()
                .as_bstr()
                .starts_with(crate::history::REVIEW_STASH_PREFIX)
            || undo::is_queue_ref(reference.name().as_bstr())
        {
            continue;
        }
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
    load_graph(repo, &revisions, &[])
}

pub(super) fn loaded_view_graph(repo: &gix::Repository) -> Result<crate::history::HistoryGraph> {
    let hidden = crate::history::available_hidden_revisions(repo, &[], true)?.0;
    load_graph(repo, &[], &hidden)
}

pub(super) fn loaded_view_graph_with(
    repo: &gix::Repository,
    revisions: &[std::ffi::OsString],
) -> Result<crate::history::HistoryGraph> {
    load_graph(repo, revisions, &[])
}

pub(super) fn loaded_view_graph_with_hidden(
    repo: &gix::Repository,
    revisions: &[std::ffi::OsString],
    hidden_revisions: &[std::ffi::OsString],
) -> Result<crate::history::HistoryGraph> {
    load_graph(repo, revisions, hidden_revisions)
}

fn load_graph(
    repo: &gix::Repository,
    revisions: &[std::ffi::OsString],
    hidden_revisions: &[std::ffi::OsString],
) -> Result<crate::history::HistoryGraph> {
    use std::sync::atomic::AtomicBool;

    let authors = gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(
        crate::history::Authors::default(),
    ));
    let mut graph = None;
    crate::history::load(
        repo,
        revisions,
        hidden_revisions,
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
pub(crate) mod review;
pub(crate) mod reword;
pub(crate) mod split;
pub(crate) mod stash;
pub(crate) mod time_travel;
pub(crate) mod todo;
pub(crate) mod undo;

#[tracing::instrument(skip_all, fields(filename))]
pub(crate) fn edit_document(
    terminal: &mut ratatui::DefaultTerminal,
    editor: gix::command::Prepare,
    document: &[u8],
    filename: &str,
    enhanced_keyboard: bool,
) -> Result<Option<Vec<u8>>> {
    crate::with_suspended_terminal(terminal, enhanced_keyboard, || {
        edit_document_without_terminal(editor, document, filename)
    })
}

pub(crate) fn edit_document_without_terminal(
    editor: gix::command::Prepare,
    document: &[u8],
    filename: &str,
) -> Result<Option<Vec<u8>>> {
    let mut tempfile = gix::tempfile::writable_at(
        std::env::temp_dir().join(filename),
        gix::tempfile::ContainingDirectory::Exists,
        gix::tempfile::AutoRemove::Tempfile,
    )
    .context("could not create commit message file")?;
    tempfile
        .write_all(document)
        .context("could not write commit message file")?;
    tempfile.flush().context("could not flush commit message file")?;
    let path = tempfile
        .with_mut(|tempfile| tempfile.path().to_owned())
        .context("commit message file disappeared")?;
    let _tempfile = tempfile.close().context("could not close commit message file")?;

    let editor_display = editor.command.to_string_lossy().into_owned();
    let status = Command::from(editor.arg(&path))
        .status()
        .with_context(|| format!("could not launch Git editor {editor_display}"))?;
    if !status.success() {
        anyhow::bail!("Git editor {editor_display} exited with {status}");
    }
    let edited = std::fs::read(path).context("could not read edited commit message")?;
    Ok((edited != document).then_some(edited))
}

#[cfg(test)]
mod tests {
    use std::{path::Path, process::Command};

    use super::*;

    fn git(path: &Path, args: &[&str]) -> gix_testtools::Result<Vec<u8>> {
        let output = Command::new("git").arg("-C").arg(path).args(args).output()?;
        if !output.status.success() {
            return Err(format!(
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        Ok(output.stdout)
    }

    #[test]
    fn edit_graph_ignores_refs_that_do_not_point_to_commits() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        git(fixture.path(), &["update-ref", "refs/cache/tree", "HEAD^{tree}"])?;
        let repo = crate::test_repository::open(fixture.path())?;
        let head = repo.head_id()?.detach();
        let graph = loaded_graph(&repo)?;
        assert!(graph.parents_of(head).is_some(), "HEAD remains part of the edit graph");
        Ok(())
    }

    #[test]
    fn edit_graph_ignores_undo_queue_retention_commits() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        let path = fixture.path();
        let retained = String::from_utf8(git(
            path,
            &["commit-tree", "HEAD^{tree}", "-p", "HEAD", "-m", "undo-only"],
        )?)?;
        let retained = retained.trim();
        git(path, &["update-ref", undo::TIP_REF, retained])?;
        git(path, &["update-ref", undo::CURSOR_REF, retained])?;

        let repo = crate::test_repository::open(path)?;
        let retained = gix::ObjectId::from_hex(retained.as_bytes())?;
        assert!(
            loaded_graph(&repo)?.parents_of(retained).is_none(),
            "undo retention commits never enter the editable graph"
        );
        Ok(())
    }

    #[test]
    fn edit_graph_excludes_unrelated_descendant_merges() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        let path = fixture.path();
        let head = String::from_utf8(git(path, &["rev-parse", "HEAD"])?)?;
        let head = head.trim();
        let tree = String::from_utf8(git(path, &["rev-parse", "HEAD^{tree}"])?)?;
        let tree = tree.trim();
        let pinned = String::from_utf8(git(path, &["commit-tree", tree, "-p", head, "-m", "pinned tip"])?)?;
        let pinned = pinned.trim();
        let merge = String::from_utf8(git(
            path,
            &["commit-tree", tree, "-p", head, "-p", pinned, "-m", "unrelated merge"],
        )?)?;
        let merge = merge.trim();
        git(path, &["update-ref", "refs/worktree/tix/pins/keep", pinned])?;
        git(path, &["update-ref", "refs/heads/unrelated", merge])?;
        git(path, &["checkout", "--detach", head])?;

        let repo = crate::test_repository::open(path)?;
        let graph = loaded_view_graph(&repo)?;
        let pinned = gix::ObjectId::from_hex(pinned.as_bytes())?;
        let merge = gix::ObjectId::from_hex(merge.as_bytes())?;
        assert!(
            graph.parents_of(pinned).is_some(),
            "the applicable pin remains in edit scope"
        );
        assert!(
            graph.parents_of(merge).is_none(),
            "an unrelated ref does not add its descendant merge to edit scope"
        );
        assert!(
            head::perform(repo, &graph, head::Kind::Amend, None)?.is_some(),
            "amending HEAD succeeds despite the unrelated merge"
        );
        Ok(())
    }
}
