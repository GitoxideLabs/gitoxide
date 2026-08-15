use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    io::{Read, Write},
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
    /// Todo file to apply; omit or use '-' to read standard input.
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,
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
    apply_document(repo, &edited)
}

fn prepare(repo: &gix::Repository, args: &Todo) -> Result<todo::Prepared> {
    if args.hide.is_empty() {
        anyhow::bail!("rebase todo requires at least one -h/--hide revision");
    }
    let (hide, unavailable) = history::available_hidden_revisions(repo, &args.hide)?;
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
                info: crate::ui::todo_metadata(&app, row, &decorations, &mailmap),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let head = repo.head()?.id().map(gix::Id::detach);
    todo::prepare(repo, base, onto, &commits, head, &resolved_tips, todo::OntoKind::Onto)
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
    apply_document(repo, &document)
}

fn apply_document(repo: gix::Repository, document: &[u8]) -> Result<()> {
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
            if let Some(selected) = outcome.selected {
                let notice =
                    edit::time_travel::checkout_without_replay(&repository_path, bare, selected, &revisions, false)?;
                println!("{}", notice.unwrap_or_else(|| "rebased history".into()));
            } else {
                println!("rebased history");
            }
            Ok(())
        }
        rebase::PlanPerform::Conflict(original) => anyhow::bail!(
            "rebase aborted without changes: conflict while applying {}",
            original.to_hex_with_len(7)
        ),
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

    fn repository() -> gix_testtools::Result<(gix_testtools::tempfile::TempDir, gix::Repository)> {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = gix::open_opts(
            fixture.path(),
            gix::open::Options::isolated().config_overrides([
                "core.editor=:".to_owned(),
                "core.abbrev=7".to_owned(),
                "user.name=todo author".to_owned(),
                "user.email=todo@example.com".to_owned(),
            ]),
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
                onto: None,
                edit_and_apply: false,
                tips: Vec::new(),
            },
        )?;
        let document = String::from_utf8(prepared.document)?;
        assert!(document.contains("<!-- tix-rebase-state-v1"), "state is embedded");
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
                onto: None,
                edit_and_apply: false,
                tips: Vec::new(),
            },
        )
        .expect_err("a hidden revision is required");
        assert!(format!("{err:#}").contains("at least one -h/--hide"));
        Ok(())
    }
}
