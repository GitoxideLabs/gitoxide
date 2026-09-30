use std::{
    collections::HashMap,
    io::Write,
    path::PathBuf,
    process::Command,
    rc::{Rc, Weak},
};

use gix::{
    Result,
    error::{OptionExt, ResultExt, bail, message},
};

/// Serialize stash and history publication across worktrees, including nested travel and rollback.
pub(super) fn mutation_lock(repo: &gix::Repository) -> Result<Rc<gix::lock::Marker>> {
    use gix::parallel::{Mutable, lock};

    thread_local! {
        // Rc keeps a nested guard on its acquiring thread. Weak entries retain no lock or repository.
        static HELD: Mutable<HashMap<PathBuf, Weak<gix::lock::Marker>>> = Mutable::default();
    }
    let path = gix::path::realpath(repo.common_dir())
        .or_raise(|| message("could not resolve the shared Git directory"))?
        .join("tix-mutation");
    HELD.with(|held| {
        let mut held = lock(held);
        held.retain(|_, guard| guard.strong_count() != 0);
        if let Some(guard) = held.get(&path).and_then(Weak::upgrade) {
            return Ok(guard);
        }
        // ponytail: one lock per repository; split it only if concurrent mutations need more throughput.
        let guard = Rc::new(
            gix::lock::Marker::acquire_to_hold_resource(&path, gix::lock::acquire::Fail::Immediately, None, 0)
                .or_raise(|| message("another Tix operation is changing this repository; retry after it finishes"))?,
        );
        held.insert(path, Rc::downgrade(&guard));
        Ok(guard)
    })
}

pub(crate) fn is_internal_ref(name: &gix::bstr::BStr) -> bool {
    undo::is_queue_ref(name) || rebase::session::is_ref(name)
}

#[cfg(test)]
pub(super) fn loaded_graph(repo: &gix::Repository) -> Result<crate::history::HistoryGraph> {
    if repo.head_id().is_err() {
        return Ok(crate::history::HistoryGraph::default());
    }
    let mut revisions = Vec::new();
    for reference in repo.references()?.all()? {
        let reference = reference?;
        if reference.name().as_bstr().starts_with(crate::history::STASH_PREFIX)
            || reference
                .name()
                .as_bstr()
                .starts_with(crate::history::REVIEW_STASH_PREFIX)
            || is_internal_ref(reference.name().as_bstr())
            || replay_refs::is_ref(reference.name().as_bstr())
        {
            continue;
        }
        let Some(id) = reference.try_id() else { continue };
        if reference.name().as_bstr() == b"HEAD" || repo.find_header(id)?.kind() != gix::object::Kind::Commit {
            continue;
        }
        revisions.push(
            gix::path::from_bstr(reference.name().as_bstr())?
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

pub(super) fn loaded_explicit_view_graph(
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

    let authors = gix::parallel::OwnShared::new(gix::parallel::Mutable::new(crate::history::Authors::default()));
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
    graph.ok_or_raise(|| message("history traversal did not produce a graph"))
}

pub(crate) mod auto_merge;
pub(crate) mod create;
pub(crate) mod delete;
pub(crate) mod discard;
pub(crate) mod enrich;
pub(crate) mod head;
pub(crate) mod rebase;
pub(crate) mod replay_refs;
pub(crate) mod review;
pub(crate) mod reword;
pub(crate) mod split;
pub(crate) mod stash;
pub(crate) mod time_travel;
pub(crate) mod todo;
pub(crate) mod transplant;
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
    .or_raise(|| message("could not create commit message file"))?;
    tempfile
        .write_all(document)
        .or_raise(|| message("could not write commit message file"))?;
    tempfile
        .flush()
        .or_raise(|| message("could not flush commit message file"))?;
    let path = tempfile
        .with_mut(|tempfile| tempfile.path().to_owned())
        .or_raise(|| message("commit message file disappeared"))?;
    let _tempfile = tempfile
        .close()
        .or_raise(|| message("could not close commit message file"))?;

    let editor_display = editor.command.to_string_lossy().into_owned();
    let mut command = Command::try_from(editor.arg(&path))?;
    let editor_display = if editor_display == command.get_program().to_string_lossy() {
        String::new()
    } else {
        format!(" {editor_display}")
    };
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // gix-command uses CREATE_NO_WINDOW for background helpers. Interactive editors
        // must inherit our console instead of running in a separate, invisible console.
        command.creation_flags(0);
    }
    let status = command
        .status()
        .or_raise(|| message!("could not launch Git editor{editor_display}").with_program(command.get_program()))?;
    if !status.success() {
        bail!("Git editor{editor_display} failed".with_command_status(&command, status));
    }
    let edited = std::fs::read(path).or_raise(|| message("could not read edited commit message"))?;
    Ok((edited != document).then_some(edited))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn nested_mutations_release_the_lock_only_after_the_last_guard() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let outer = mutation_lock(&repo)?;
        let inner = mutation_lock(&repo)?;
        drop(outer);
        let path = fixture.path().to_owned();
        let competing = std::thread::spawn(move || -> gix::Result<()> {
            let repo = crate::test_repository::open(path)?;
            let _guard = mutation_lock(&repo)?;
            Ok(())
        })
        .join()
        .expect("the competing thread does not panic");
        assert!(competing.is_err(), "another thread cannot reuse this thread's lock");
        drop(inner);
        assert!(
            !repo.common_dir().join("tix-mutation.lock").exists(),
            "dropping the final guard releases the on-disk lock"
        );
        let _guard = mutation_lock(&repo)?;
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn editor_inherits_the_windows_console() -> gix_testtools::Result {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        if std::env::var_os("GIX_TIX_TEST_CONSOLE").is_none() {
            // Cargo may run without a console. Give this test its own hidden console so
            // inheritance is observable without opening a window or using the user's terminal.
            let output_dir = gix_testtools::tempfile::tempdir()?;
            let thread = std::thread::current();
            let test_name = thread.name().expect("libtest names its test threads");
            let mut command = Command::new("pwsh");
            let output = gix_testtools::configure_git_environment(&mut command, output_dir.path())
                .env("GIX_TIX_TEST_CONSOLE", "1")
                .args(["-NoProfile", "-NonInteractive", "-File"])
                .arg(fixtures.join("with-console.ps1"))
                .arg(std::env::current_exe()?)
                .arg(test_name)
                .arg(output_dir.path())
                .output()?;
            assert!(
                output.status.success(),
                "the editor test succeeds in its private console: {}\n{}\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return Ok(());
        }

        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        let editor = format!(
            "pwsh -NoProfile -NonInteractive -File \"{}\" {}",
            fixtures.join("console-editor.ps1").display(),
            std::process::id()
        );
        let repo = crate::test_repository::open_with(fixture.path(), [format!("core.editor={editor}")])?;
        for shell in [false, true] {
            let editor = repo.editor_command()?.expect("the console editor is configured");
            let editor = if shell { editor.with_shell() } else { editor };
            assert_eq!(
                edit_document_without_terminal(editor, b"original\n", "tix-console-editor.md")?,
                Some(b"edited with an inherited console\n".to_vec()),
                "both direct and shell editors share the caller's console and edit the file (shell: {shell})"
            );
        }
        Ok(())
    }

    fn git(path: &Path, args: &[&str]) -> gix_testtools::Result<Vec<u8>> {
        let output = gix_testtools::git_command(path).args(args).output()?;
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
    #[cfg(unix)]
    fn editor_errors_keep_command_context_without_repeating_metadata() -> gix::error::TestResult {
        for (editor, expected, exit_code) in [
            (gix::command::prepare("/usr/bin/false"), "Git editor failed", 1),
            (
                gix::command::prepare("exit 23;").with_shell(),
                "Git editor exit 23; failed",
                23,
            ),
        ] {
            let err = edit_document_without_terminal(editor, b"message", "tix-editor-error.md")
                .expect_err("a failing editor cannot supply an edited document");
            let diagnostic = err
                .downcast_any_ref::<gix::error::Message>()
                .expect("editor failures carry metadata");
            assert_eq!(
                diagnostic.message, expected,
                "prose retains editor arguments, not the program-only name"
            );
            let rendered = diagnostic.to_string();
            assert_eq!(
                rendered.matches("exit status:").count(),
                1,
                "status appears only in metadata"
            );
            assert_eq!(
                err.metadata_merged()["exit_code"],
                exit_code.into(),
                "the exit code is retained"
            );
        }

        let fixture = gix_testtools::tempfile::tempdir()?;
        let editor = gix::command::prepare("editor --wait")
            .with_shell()
            .with_shell_program(fixture.path().join("missing-shell"));
        let err = edit_document_without_terminal(editor, b"message", "tix-editor-launch-error.md")
            .expect_err("a missing shell cannot launch the editor");
        assert_eq!(
            err.downcast_any_ref::<gix::error::Message>()
                .expect("launch failures carry metadata")
                .message,
            "could not launch Git editor editor --wait",
            "launch failures retain the full editor command, not just shell metadata"
        );
        let editor_path = fixture.path().join("missing-editor");
        let err = edit_document_without_terminal(
            gix::command::prepare(editor_path.clone()),
            b"message",
            "tix-editor-missing.md",
        )
        .expect_err("a missing editor cannot launch");
        let diagnostic = err
            .downcast_any_ref::<gix::error::Message>()
            .expect("launch failures carry metadata");
        assert_eq!(
            diagnostic.message, "could not launch Git editor",
            "program-only names are metadata, not prose"
        );
        assert_eq!(
            diagnostic.values["program"],
            editor_path.into(),
            "the missing editor path remains in metadata"
        );
        Ok(())
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
