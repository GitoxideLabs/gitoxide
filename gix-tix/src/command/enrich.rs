use std::ffi::OsString;

use anyhow::{Context, Result};
use gix::bstr::ByteSlice;

#[derive(Debug, clap::Subcommand)]
pub(super) enum Command {
    /// Manage enrichments keyed by a commit's change ID.
    #[command(subcommand)]
    Commit(Commit),
    /// Manage enrichments keyed by a commit's tree ID.
    #[command(subcommand)]
    Tree(Tree),
}

#[derive(Debug, clap::Subcommand)]
pub(super) enum Commit {
    /// Mark the commit as todo, or clear the mark.
    Todo(BooleanTarget),
    /// Edit the commit's Tix note with Git's editor.
    Note(Target),
    /// Edit the commit's ordinary Git note with Git's editor.
    GitNote(Target),
}

#[derive(Debug, clap::Subcommand)]
pub(super) enum Tree {
    /// Mark the tree as passing checks, or clear the mark.
    ChecksPass(BooleanTarget),
}

#[derive(Debug, clap::Args)]
pub(super) struct BooleanTarget {
    /// Clear the enrichment instead of setting it.
    #[arg(long)]
    clear: bool,
    #[command(flatten)]
    target: Target,
}

#[derive(Debug, clap::Args)]
pub(super) struct Target {
    /// Commit whose enrichment should be changed.
    #[arg(default_value = "HEAD", value_name = "REVSPEC")]
    revision: OsString,
}

pub(super) fn run(repository: gix::Repository, command: Command) -> Result<()> {
    match command {
        Command::Commit(Commit::Todo(args)) => {
            let target = resolve(&repository, &args.target)?;
            let reference = crate::enrich::REF_NAME.try_into().expect("valid enrich ref");
            let (enrichment, changes) = tracked_ref_update(&repository, reference, |repository| {
                crate::enrich::ensure_todo(repository, target, !args.clear)
            })?;
            super::record_undo(
                &repository,
                if enrichment.todo {
                    "mark commit todo"
                } else {
                    "clear commit todo"
                },
                changes,
            );
            feedback(
                &repository,
                target,
                if enrichment.todo {
                    "marked commit todo"
                } else {
                    "cleared commit todo"
                },
            )
        }
        Command::Commit(Commit::Note(args)) => edit_note(&repository, &args),
        Command::Commit(Commit::GitNote(args)) => edit_git_note(&repository, &args),
        Command::Tree(Tree::ChecksPass(args)) => {
            let target = resolve(&repository, &args.target)?;
            let reference = crate::enrich::TREE_REF_NAME.try_into().expect("valid tree enrich ref");
            let (enrichment, changes) = tracked_ref_update(&repository, reference, |repository| {
                crate::enrich::ensure_checks_pass(repository, target, !args.clear)
            })?;
            super::record_undo(
                &repository,
                if enrichment.checks_pass {
                    "mark tree checks-pass"
                } else {
                    "clear tree checks-pass"
                },
                changes,
            );
            feedback(
                &repository,
                target,
                if enrichment.checks_pass {
                    "marked tree checks-pass"
                } else {
                    "cleared tree checks-pass"
                },
            )
        }
    }
}

fn resolve(repository: &gix::Repository, target: &Target) -> Result<gix::ObjectId> {
    super::resolve_commit(repository, &target.revision, "enrichment target").map(|(id, _)| id)
}

fn feedback(repository: &gix::Repository, target: gix::ObjectId, status: &str) -> Result<()> {
    println!("{} {status}", crate::change_id::display(repository, target, 7)?);
    Ok(())
}

fn tracked_ref_update<T>(
    repository: &gix::Repository,
    name: gix::refs::FullName,
    update: impl FnOnce(&gix::Repository) -> Result<T>,
) -> Result<(T, Result<Vec<crate::edit::undo::RefChange>>)> {
    let before = crate::edit::undo::state(repository, name.as_ref());
    let value = update(repository)?;
    let changes = before.and_then(|before| {
        crate::edit::undo::state(repository, name.as_ref()).map(|after| {
            (before != after)
                .then_some(crate::edit::undo::RefChange { name, before, after })
                .into_iter()
                .collect()
        })
    });
    Ok((value, changes))
}

fn edit_note(repository: &gix::Repository, args: &Target) -> Result<()> {
    let target = resolve(repository, args)?;
    let enrichment = crate::enrich::load(
        &mut crate::enrich::open(repository)?,
        crate::change_id::for_commit(repository, target)?,
    )?;
    let document = enrichment.note.clone().unwrap_or_default();
    let editor = repository.editor().context("no Git editor is available")?;
    let edited = crate::edit::edit_document_without_terminal(
        &editor,
        &document,
        &format!("tix-note-{}-{}.md", std::process::id(), target.to_hex_with_len(7)),
    )?;
    let cleaned = crate::edit::reword::cleanup_message(edited.as_deref().unwrap_or(&document), None);
    let desired = (!cleaned.is_empty()).then_some(cleaned.as_bstr());
    let (status, changes) = if enrichment.note.as_ref().map(|note| note.as_bstr()) == desired {
        ("note unchanged", Ok(Vec::new()))
    } else {
        let reference = crate::enrich::REF_NAME.try_into().expect("valid enrich ref");
        let (_, changes) = tracked_ref_update(repository, reference, |repository| {
            crate::enrich::set_note(repository, target, desired.map(AsRef::as_ref))
        })?;
        (
            if desired.is_some() {
                "saved note"
            } else {
                "cleared note"
            },
            changes,
        )
    };
    super::record_undo(repository, "edit commit note", changes);
    feedback(repository, target, status)
}

fn edit_git_note(repository: &gix::Repository, args: &Target) -> Result<()> {
    let target = resolve(repository, args)?;
    let notes = repository.notes()?;
    let reference = notes
        .default_ref()
        .context("no default Git notes reference is configured")?
        .to_owned();
    let mut notes = notes.with_refs([reference.as_bstr()])?;
    let document = notes
        .get(target)?
        .first()
        .map(|note| note.blob.data.clone())
        .unwrap_or_default();
    let editor = repository.editor().context("no Git editor is available")?;
    let edited = crate::edit::edit_document_without_terminal(
        &editor,
        &document,
        &format!("tix-git-note-{}-{}.md", std::process::id(), target.to_hex_with_len(7)),
    )?;
    let cleaned = crate::edit::reword::cleanup_message(edited.as_deref().unwrap_or(&document), None);
    let (status, changes) = if cleaned == document {
        ("Git note unchanged", Ok(Vec::new()))
    } else {
        let saved = !cleaned.is_empty();
        let (_, changes) = tracked_ref_update(repository, reference.clone(), |_repository| {
            let data: Option<&[u8]> = saved.then_some(cleaned.as_ref());
            match data {
                Some(data) => notes
                    .add_to_ref(reference.as_ref(), target, data)
                    .context("could not save Git note")?,
                None => notes
                    .remove(reference.as_ref().as_partial_name().to_owned(), target)
                    .context("could not remove Git note")?,
            };
            Ok(())
        })?;
        (if saved { "saved Git note" } else { "cleared Git note" }, changes)
    };
    super::record_undo(repository, "edit Git note", changes);
    feedback(repository, target, status)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(revision: &str) -> Target {
        Target {
            revision: revision.into(),
        }
    }

    #[test]
    fn boolean_enrichments_are_idempotent_and_target_the_selected_commit() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = crate::test_repository::open(fixture.path())?;
        let topic = repository.rev_parse_single("topic")?.detach();
        let head = repository.head_id()?.detach();

        for clear in [false, false, true] {
            run(
                repository.clone(),
                Command::Commit(Commit::Todo(BooleanTarget {
                    clear,
                    target: target("topic"),
                })),
            )?;
            run(
                repository.clone(),
                Command::Tree(Tree::ChecksPass(BooleanTarget {
                    clear,
                    target: target("topic"),
                })),
            )?;
            assert_eq!(
                crate::enrich::load(
                    &mut crate::enrich::open(&repository)?,
                    crate::change_id::for_commit(&repository, topic)?,
                )?
                .todo,
                !clear,
                "todo is set or cleared without toggling"
            );
            assert_eq!(
                crate::enrich::load_tree(
                    &mut crate::enrich::open_tree(&repository)?,
                    crate::enrich::tree_id(&repository, topic)?,
                )?
                .checks_pass,
                !clear,
                "checks-pass is set or cleared without toggling"
            );
            if !clear {
                assert!(
                    !crate::enrich::load(
                        &mut crate::enrich::open(&repository)?,
                        crate::change_id::for_commit(&repository, head)?,
                    )?
                    .todo,
                    "the default HEAD remains untouched"
                );
                assert!(
                    !crate::enrich::load_tree(
                        &mut crate::enrich::open_tree(&repository)?,
                        crate::enrich::tree_id(&repository, head)?,
                    )?
                    .checks_pass,
                    "the default HEAD tree remains untouched"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn editors_preserve_other_enrichments() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = crate::test_repository::open_with(fixture.path(), ["core.editor=sed -i.bak -e 's/old/new/'"])?;
        let topic = repository.rev_parse_single("topic")?.detach();
        crate::enrich::ensure_todo(&repository, topic, true)?;
        crate::enrich::set_note(&repository, topic, Some(b"old\n"))?;
        let notes = repository.notes()?;
        let reference = notes.default_ref().context("the fixture has a notes ref")?.to_owned();
        crate::set_git_note(&repository, reference.as_ref(), topic, Some(b"old\n"))?;

        run(repository.clone(), Command::Commit(Commit::Note(target("topic"))))?;
        run(repository.clone(), Command::Commit(Commit::GitNote(target("topic"))))?;

        let enrichment = crate::enrich::load(
            &mut crate::enrich::open(&repository)?,
            crate::change_id::for_commit(&repository, topic)?,
        )?;
        assert!(enrichment.todo, "editing a note preserves todo");
        assert_eq!(
            enrichment.note.as_ref().map(|note| note.as_bstr()),
            Some(b"new\n".as_bstr()),
            "the Tix note uses the editor output"
        );
        let mut notes = repository.notes()?.with_refs([reference.as_bstr()])?;
        assert_eq!(
            notes.get(topic)?.first().map(|note| note.blob.data.as_bstr()),
            Some(b"new\n".as_bstr()),
            "the ordinary Git note uses the editor output"
        );
        Ok(())
    }
}
