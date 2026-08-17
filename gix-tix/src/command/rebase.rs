use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    io::{IsTerminal, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

use anyhow::{Context, Result};
use gix::ObjectId;

use crate::{
    app::App,
    edit::{self, rebase, todo},
    history::{self, Authors, Decorations, Event, HistoryGraph},
};

#[derive(Debug, clap::Subcommand)]
pub(super) enum Command {
    /// Produce a self-contained rebase todo, or edit and apply it immediately.
    #[command(disable_help_flag = true)]
    Todo(Todo),
    /// Apply a self-contained rebase todo from FILE or standard input.
    #[command(
        after_long_help = "Conflicts change nothing by default. To opt in, write the continuation only when needed:\n  tix rebase apply --materialize-conflicts todo.continue.md todo.md\nResolve the index, then run:\n  tix rebase apply todo.continue.md\nUse --materialize-conflicts=- to write a continuation to non-terminal stdout."
    )]
    Apply(Apply),
}

#[derive(Debug, clap::Args)]
#[command(
    after_long_help = "Without --edit-and-apply, the todo is written to stdout. With it, Git's normal editor selection is used; GIT_EDITOR=<command> overrides it.\n\nExamples:\n  tix rebase todo -h main topic >todo.md\n  ${GIT_EDITOR:-editor} todo.md\n  tix rebase apply todo.md\n  tix rebase todo --edit-and-apply -h main topic"
)]
pub(super) struct Todo {
    /// Print help.
    #[arg(long, action = clap::ArgAction::HelpLong)]
    help: Option<bool>,
    /// Hide this revision and derive the editable fork point from it.
    #[arg(short = 'h', long, value_name = "REVSPEC")]
    hide: Vec<OsString>,
    /// Do not infer hidden local branches from remote HEADs.
    #[arg(long)]
    no_auto_hide: bool,
    /// Rebase the derived scope onto this commit instead of its fork point.
    #[arg(long, value_name = "REV")]
    onto: Option<OsString>,
    /// Open the todo in Git's editor and apply it after the editor exits.
    #[arg(long)]
    edit_and_apply: bool,
    /// Visible traversal tips, or HEAD if omitted.
    #[arg(value_name = "TIP")]
    tips: Vec<OsString>,
}

#[derive(Debug, clap::Args)]
pub(super) struct Apply {
    /// On conflict, materialize it and write a continuation todo to FILE, or stdout if omitted or '-'.
    #[arg(long, value_name = "CONTINUE", num_args = 0..=1, default_missing_value = "-")]
    pub(super) materialize_conflicts: Option<PathBuf>,
    /// Todo file to apply; omit or use '-' to read standard input.
    #[arg(value_name = "FILE")]
    pub(super) file: Option<PathBuf>,
}

pub(super) fn run(repo: gix::Repository, command: Command) -> Result<()> {
    match command {
        Command::Todo(args) => todo(repo, args),
        Command::Apply(args) => apply(repo, args),
    }
}

fn todo(repo: gix::Repository, args: Todo) -> Result<()> {
    let prepared = prepare(&repo, &args)?;
    if !args.edit_and_apply {
        std::io::stdout()
            .write_all(&prepared.document)
            .context("could not write the rebase todo")?;
        return Ok(());
    }

    let editor = repo.editor().context("no Git editor is available")?;
    let edited = edit::edit_document_without_terminal(
        &editor,
        &prepared.document,
        &format!("tix-rebase-{}.md", std::process::id()),
    )?
    .unwrap_or(prepared.document);
    apply_document(repo, &edited, None)
}

fn prepare(repo: &gix::Repository, args: &Todo) -> Result<todo::Prepared> {
    let (hide, unavailable) = history::available_hidden_revisions(repo, &args.hide, !args.no_auto_hide)?;
    if hide.is_empty() {
        anyhow::bail!(
            "rebase todo requires at least one -h/--hide revision when no remote HEAD maps to a local branch"
        );
    }
    for (revision, err) in unavailable {
        eprintln!(
            "warning: ignoring unavailable hidden revision {}: {err}",
            revision.to_string_lossy()
        );
    }
    let resolved_tips = history::snapshot(repo, &args.tips, &hide, false)?.view_tips;

    let authors = gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(Authors::default()));
    let mut app = App::new(usize::MAX);
    let mut decorations = Decorations::default();
    let mut complete = false;
    history::load(
        repo,
        &args.tips,
        &hide,
        false,
        &authors,
        &AtomicBool::new(false),
        |event| {
            match event {
                Event::Decorations(value) => decorations = value,
                Event::Commits(commits) => app.extend_commits(commits),
                Event::HiddenCommits(commits) => app.extend_hidden_commits(commits),
                Event::Complete(_) => complete = true,
                Event::VisibleComplete | Event::Cancelled => {}
            }
            true
        },
    )?;
    if !complete {
        anyhow::bail!("history traversal did not produce a graph");
    }
    let mut candidates = app.hidden_rebase_candidates();
    if candidates.len() != 1 {
        if candidates.is_empty() {
            anyhow::bail!("the hidden and visible revisions have no editable fork point");
        }
        candidates.sort_by_key(|(id, _)| *id);
        anyhow::bail!(
            "the revisions have multiple editable fork points: {}",
            candidates
                .iter()
                .map(|(id, _)| id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let (base, scope) = candidates.pop().context("one rebase candidate was expected")?;
    let onto = args
        .onto
        .as_deref()
        .map(|revision| resolve_commit(repo, revision, "onto revision"))
        .transpose()?
        .unwrap_or(base);

    let mut notes = repo.notes().context("could not open Git notes")?;
    let row_indices: HashMap<_, _> = app
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| (row.id, index))
        .collect();
    for id in &scope {
        let index = row_indices
            .get(id)
            .copied()
            .context("an editable commit disappeared from the history view")?;
        if !app.rows[index].metadata_loaded {
            let (metadata, attributions) =
                history::load_metadata(repo, *id, &authors).context("could not load editable commit metadata")?;
            app.set_metadata(index, metadata, attributions);
        }
        let loaded = notes
            .get(*id)
            .context("could not load commit notes")?
            .into_iter()
            .map(|note| {
                let mut blob = note.blob;
                blob.take_data().into()
            })
            .collect();
        app.set_notes(*id, loaded);
    }
    let mailmap = repo.open_mailmap();
    let commits = scope
        .iter()
        .map(|id| {
            let row = row_indices
                .get(id)
                .and_then(|index| app.rows.get(*index))
                .context("an editable commit disappeared while formatting the todo")?;
            Ok(todo::Commit {
                id: *id,
                parents: row.parent_ids.iter().copied().collect(),
                info: crate::ui::todo_metadata(&app, row, &mailmap),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    todo::prepare(repo, base, onto, &commits, &resolved_tips, todo::OntoKind::Onto)
}

fn resolve_commit(repo: &gix::Repository, revision: &OsStr, description: &str) -> Result<ObjectId> {
    let revision =
        gix::path::os_str_into_bstr(revision).with_context(|| format!("{description} is not valid UTF-8"))?;
    let id = repo
        .rev_parse_single(revision)
        .with_context(|| format!("could not resolve {description}"))?;
    id.object()
        .with_context(|| format!("could not read {description}"))?
        .try_into_commit()
        .with_context(|| format!("{description} does not name a commit"))?;
    Ok(id.detach())
}

fn apply(repo: gix::Repository, args: Apply) -> Result<()> {
    let mut document = Vec::new();
    match args.file.as_deref() {
        None => {
            std::io::stdin()
                .read_to_end(&mut document)
                .context("could not read the rebase todo from standard input")?;
        }
        Some(path) if path == Path::new("-") => {
            std::io::stdin()
                .read_to_end(&mut document)
                .context("could not read the rebase todo from standard input")?;
        }
        Some(path) => {
            document =
                std::fs::read(path).with_context(|| format!("could not read rebase todo at {}", path.display()))?;
        }
    }
    apply_document(repo, &document, args.materialize_conflicts.as_deref())
}

fn apply_document(repo: gix::Repository, document: &[u8], materialize_conflicts: Option<&Path>) -> Result<()> {
    let Some(parsed) = todo::parse(&repo, document)? else {
        println!("no rebase performed: the todo was cancelled");
        return Ok(());
    };
    let graph = HistoryGraph::for_commits(&repo, &parsed.plan.scope)?;
    let repository_path = repo.git_dir().to_owned();
    let bare = repo.is_bare();
    let tips = parsed.tips;
    match rebase::perform_plan(&repo, &graph, parsed.plan)? {
        rebase::PlanPerform::Complete(outcome) => {
            let revisions = mapped_revisions(&tips, |id| outcome.map(id));
            if outcome.selected.is_some() {
                let notice = edit::time_travel::checkout_plan(&repository_path, bare, &outcome, &revisions, false)?;
                println!("{}", notice.unwrap_or_else(|| "rebased history".into()));
            } else {
                println!("rebased history");
            }
            Ok(())
        }
        rebase::PlanPerform::Conflict(conflict) => {
            let Some(destination) = materialize_conflicts else {
                anyhow::bail!(
                    "rebase aborted without changes: conflict while applying {}; pass --materialize-conflicts to opt in",
                    conflict.original().to_hex_with_len(7)
                );
            };
            if destination == Path::new("-") && std::io::stdout().is_terminal() {
                anyhow::bail!(
                    "rebase aborted without changes: refusing to materialize a conflict without a continuation output file"
                );
            }
            let plan = conflict.continuation_plan();
            let mapped_tips = tips.iter().filter_map(|id| conflict.map(*id)).collect();
            let continuation = todo::prepare_continuation(conflict.repository(), &plan, mapped_tips)?.document;
            let revisions = mapped_revisions(&tips, |id| conflict.map(id));
            if destination == Path::new("-") {
                let mut stdout = std::io::stdout().lock();
                stdout
                    .write_all(&continuation)
                    .and_then(|_| stdout.flush())
                    .context("could not write the continuation rebase todo")?;
            } else {
                let mut output = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(destination)
                    .with_context(|| {
                        format!("could not create continuation rebase todo at {}", destination.display())
                    })?;
                output.write_all(&continuation).with_context(|| {
                    format!("could not write continuation rebase todo at {}", destination.display())
                })?;
            }
            let materialized =
                edit::time_travel::materialize_plan_conflict(conflict, &repository_path, bare, &revisions, false);
            let (notice, _) = match materialized {
                Ok(materialized) => materialized,
                Err(err) => {
                    if destination != Path::new("-") {
                        let _ = std::fs::remove_file(destination);
                    }
                    return Err(err);
                }
            };
            eprintln!("{notice}; continue with `tix rebase apply {}`", destination.display());
            anyhow::bail!("rebase stopped at a materialized conflict")
        }
    }
}

fn mapped_revisions(tips: &[ObjectId], mut map: impl FnMut(ObjectId) -> Option<ObjectId>) -> Vec<OsString> {
    tips.iter()
        .filter_map(|id| map(*id))
        .map(|id| OsString::from(id.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn repository() -> gix_testtools::Result<(gix_testtools::tempfile::TempDir, gix::Repository)> {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["core.abbrev=7", "user.name=todo author", "user.email=todo@example.com"],
        )?;
        Ok((fixture, repo))
    }

    #[test]
    fn generates_a_self_contained_todo_from_hidden_and_visible_revisions() -> gix_testtools::Result {
        let (_fixture, repo) = repository()?;
        let prepared = prepare(
            &repo,
            &Todo {
                help: None,
                hide: vec!["HEAD~2".into()],
                no_auto_hide: false,
                onto: None,
                edit_and_apply: false,
                tips: Vec::new(),
            },
        )?;
        let document = String::from_utf8(prepared.document)?;
        assert!(document.contains("<!-- tix-rebase-state-v2"), "state is embedded");
        assert!(document.contains("`@pick "), "HEAD is the generated checkout");
        assert!(
            document.contains("2000-01-02 author middle"),
            "default TUI metadata is present"
        );
        assert!(
            document.contains("2000-01-03 author tip"),
            "the subject is always present"
        );
        Ok(())
    }

    #[test]
    fn requires_a_hidden_revision_at_runtime() -> gix_testtools::Result {
        let (_fixture, repo) = repository()?;
        let err = prepare(
            &repo,
            &Todo {
                help: None,
                hide: Vec::new(),
                no_auto_hide: true,
                onto: None,
                edit_and_apply: false,
                tips: Vec::new(),
            },
        )
        .expect_err("a hidden revision is required");
        assert!(format!("{err:#}").contains("at least one -h/--hide"));
        Ok(())
    }

    #[test]
    fn infers_the_hidden_base_from_a_remote_head() -> gix_testtools::Result {
        let (fixture, _repo) = repository()?;
        for args in [
            &["branch", "base", "HEAD~2"][..],
            &["config", "remote.origin.url", "https://example.com/repo"][..],
            &["config", "remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*"][..],
            &["update-ref", "refs/remotes/origin/base", "refs/heads/base"][..],
            &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/base"][..],
        ] {
            let output = Command::new("git").current_dir(fixture.path()).args(args).output()?;
            assert!(
                output.status.success(),
                "git {args:?} prepares the remote default: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["core.abbrev=7", "user.name=todo author", "user.email=todo@example.com"],
        )?;
        let prepared = prepare(
            &repo,
            &Todo {
                help: None,
                hide: Vec::new(),
                no_auto_hide: false,
                onto: None,
                edit_and_apply: false,
                tips: Vec::new(),
            },
        )?;
        assert!(
            String::from_utf8(prepared.document)?.contains("# Rebase from"),
            "the inferred local default branch provides the rebase base"
        );
        Ok(())
    }

    #[test]
    fn materialized_conflicts_emit_an_applicable_continuation_todo() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_conflict.sh")?;
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=todo author", "user.email=todo@example.com"],
        )?;
        let base = repo.rev_parse_single("HEAD~2")?.detach();
        let middle = repo.rev_parse_single("HEAD~1")?.detach();
        let tip = repo.head_id()?.detach();
        let prepared = todo::prepare(
            &repo,
            base,
            base,
            &[
                todo::Commit {
                    id: tip,
                    parents: vec![middle],
                    info: "tip".into(),
                },
                todo::Commit {
                    id: middle,
                    parents: vec![base],
                    info: "middle".into(),
                },
            ],
            &[tip],
            todo::OntoKind::Onto,
        )?;
        let generated = std::str::from_utf8(&prepared.document)?;
        let state = &generated[generated
            .find("<!-- tix-rebase-state-v2")
            .expect("generated state is present")..];
        let edited = format!(
            "`@pick {}` tip\n──── fork {} ────\n\n{state}",
            tip.to_hex_with_len(7),
            base.to_hex_with_len(7)
        );
        let output_dir = gix_testtools::tempfile::tempdir()?;
        let output = output_dir.path().join("continue.md");
        let err = apply_document(repo, edited.as_bytes(), Some(&output)).expect_err("the conflict stops the command");
        assert!(format!("{err:#}").contains("materialized conflict"));
        let continuation = std::fs::read(&output)?;
        assert!(
            continuation
                .windows(40)
                .any(|window| window.iter().all(|byte| *byte == b'0')),
            "the conflicting command is represented by the full null object ID"
        );
        let unresolved = Command::new("git")
            .arg("-C")
            .arg(fixture.path())
            .args(["diff", "--name-only", "--diff-filter=U"])
            .output()?;
        assert!(unresolved.status.success());
        assert_eq!(
            unresolved.stdout, b"file\n",
            "materialization writes the unmerged index"
        );
        let materialized = crate::test_repository::open(fixture.path())?;
        let conflict_commit = materialized.head_commit()?;
        assert_eq!(
            conflict_commit.tree_id()?.detach(),
            conflict_commit
                .parent_ids()
                .next()
                .expect("a cherry-picked commit has a parent")
                .object()?
                .peel_to_tree()?
                .id,
            "the materialized conflict commit records the ours tree"
        );

        std::fs::write(fixture.path().join("file"), b"resolved\n")?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["add", "file"])
                .status()?
                .success()
        );
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=todo author", "user.email=todo@example.com"],
        )?;
        apply_document(repo, &continuation, None)?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["diff", "--name-only", "--diff-filter=U"])
                .output()?
                .stdout
                .is_empty(),
            "the continuation consumes the resolved index"
        );
        Ok(())
    }

    #[test]
    fn successful_apply_does_not_create_a_continuation_file() -> gix_testtools::Result {
        let (_fixture, repo) = repository()?;
        let prepared = prepare(
            &repo,
            &Todo {
                help: None,
                hide: vec!["HEAD~2".into()],
                no_auto_hide: false,
                onto: None,
                edit_and_apply: false,
                tips: Vec::new(),
            },
        )?;
        let output_dir = gix_testtools::tempfile::tempdir()?;
        let output = output_dir.path().join("unused.md");
        apply_document(repo, &prepared.document, Some(&output))?;
        assert!(!output.exists(), "continuation output is created only after a conflict");
        Ok(())
    }
}
