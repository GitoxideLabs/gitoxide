use std::{ffi::OsString, path::PathBuf};

use anyhow::{Context, Result};
use gix::bstr::{BString, ByteSlice};

#[derive(Debug, clap::Subcommand)]
pub(super) enum Command {
    /// Manage enrichments keyed by a commit's change ID.
    #[command(subcommand)]
    Commit(Commit),
    /// Manage enrichments keyed by a commit's tree ID.
    #[command(subcommand)]
    Tree(Tree),
    /// Manage enrichments for a patch within the same Tix change.
    #[command(subcommand)]
    Patch(Patch),
}

#[derive(Debug, clap::Subcommand)]
pub(super) enum Commit {
    /// Mark the commit as todo, or clear the mark.
    Todo(BooleanTarget),
    /// Edit the commit's Tix note.
    Note(NoteTarget),
    /// Edit the commit's ordinary Git note.
    GitNote(NoteTarget),
}

#[derive(Debug, clap::Subcommand)]
pub(super) enum Tree {
    /// Mark the tree as passing checks, or clear the mark.
    ChecksPass(BooleanTarget),
}

#[derive(Debug, clap::Subcommand)]
pub(super) enum Patch {
    /// Mark the patch as reviewed and refactored, or clear the mark.
    Refackiewed(BooleanTarget),
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

#[derive(Debug, clap::Args)]
pub(super) struct NoteTarget {
    #[command(flatten)]
    target: Target,
    /// Use this message instead of opening an editor; repeat to add paragraphs.
    #[arg(short = 'm', long, value_name = "MESSAGE", conflicts_with = "file")]
    message: Vec<OsString>,
    /// Read the new message from this file, or from standard input with `-`.
    #[arg(short = 'f', long, value_name = "FILE", conflicts_with = "message")]
    file: Option<PathBuf>,
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
        Command::Patch(Patch::Refackiewed(args)) => refackiewed(&repository, args),
    }
}

fn refackiewed(repository: &gix::Repository, args: BooleanTarget) -> Result<()> {
    let (target, resolved_graph) = super::resolve_commit(repository, &args.target.revision, "refackiew target")?;
    let commit = repository.find_commit(target)?.decode()?.into_owned()?;
    let needs_header = !args.clear
        && !crate::edit::rebase::is_pending(&commit)
        && !commit
            .extra_headers
            .iter()
            .any(|(name, _)| name == crate::patch_id::HEADER);
    let graph = if needs_header {
        let pins = crate::history::all_pins(repository)?;
        let head = repository.head()?;
        let attached_head = !head.is_detached() && head.id().map(gix::Id::detach) == Some(target);
        let graph = match resolved_graph {
            Some(graph) => graph,
            None => {
                let revisions = [OsString::from("HEAD"), OsString::from(target.to_string())];
                let hidden = crate::history::available_hidden_revisions(repository, &[], true)?.0;
                crate::edit::loaded_explicit_view_graph(repository, &revisions, &hidden)?
            }
        };
        super::reword::ensure_retained_target(&graph, target, &pins, attached_head)?;
        Some(graph)
    } else {
        None
    };
    let outcome = crate::edit::enrich::refackiewed(repository, graph.as_ref(), target, Some(!args.clear), |_| {})?;
    if let Some(notice) = &outcome.notice {
        eprintln!("{notice}");
    }
    let status = if outcome.enrichment.refackiewed {
        "marked patch refackiewed"
    } else {
        "cleared patch refackiewed"
    };
    eprintln!(
        "{} {status}",
        crate::change_id::display(repository, outcome.selected, 7)?
    );
    super::print_ref_rewrites(repository, &outcome.ref_rewrites)?;
    super::record_undo(
        repository,
        if outcome.enrichment.refackiewed {
            "mark patch refackiewed"
        } else {
            "clear patch refackiewed"
        },
        Ok(outcome.ref_changes),
    );
    Ok(())
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

fn note_message(repository: &gix::Repository, args: &NoteTarget, document: &[u8], filename: &str) -> Result<BString> {
    let edited = match super::reword::explicit_message(&args.message, args.file.as_deref(), std::io::stdin())? {
        Some(message) => Some(message),
        None => {
            let editor = repository
                .editor_command()
                .context("could not prepare Git editor")?
                .context("no Git editor is available")?;
            crate::edit::edit_document_without_terminal(editor, document, filename)?
        }
    };
    Ok(crate::edit::reword::cleanup_message(
        edited.as_deref().unwrap_or(document),
        None,
    ))
}

fn edit_note(repository: &gix::Repository, args: &NoteTarget) -> Result<()> {
    let target = resolve(repository, &args.target)?;
    let enrichment = crate::enrich::load(
        &mut crate::enrich::open(repository)?,
        crate::change_id::for_commit(repository, target)?,
    )?;
    let cleaned = note_message(
        repository,
        args,
        enrichment.note.as_ref().map(|note| note.as_slice()).unwrap_or_default(),
        &format!("tix-note-{}-{}.md", std::process::id(), target.to_hex_with_len(7)),
    )?;
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

fn edit_git_note(repository: &gix::Repository, args: &NoteTarget) -> Result<()> {
    let target = resolve(repository, &args.target)?;
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
    let cleaned = note_message(
        repository,
        args,
        &document,
        &format!("tix-git-note-{}-{}.md", std::process::id(), target.to_hex_with_len(7)),
    )?;
    let (status, changes) = if cleaned == document {
        ("Git note unchanged", Ok(Vec::new()))
    } else {
        let saved = !cleaned.is_empty();
        let (_, changes) = tracked_ref_update(repository, reference.clone(), |_repository| {
            let data: Option<&[u8]> = saved.then_some(cleaned.as_ref());
            match data {
                Some(data) => notes
                    .replace_at_ref(reference.as_ref(), target, data)
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

    fn note_target(revision: &str) -> NoteTarget {
        NoteTarget {
            target: target(revision),
            message: Vec::new(),
            file: None,
        }
    }

    fn note_command(git_note: bool, args: NoteTarget) -> Command {
        Command::Commit(if git_note {
            Commit::GitNote(args)
        } else {
            Commit::Note(args)
        })
    }

    fn saved_note(repository: &gix::Repository, commit_id: gix::ObjectId, git_note: bool) -> Result<Option<Vec<u8>>> {
        if git_note {
            let notes = repository.notes()?;
            let reference = notes.default_ref().context("the fixture has a notes ref")?.to_owned();
            Ok(notes
                .with_refs([reference.as_bstr()])?
                .get(commit_id)?
                .first()
                .map(|note| note.blob.data.clone()))
        } else {
            Ok(crate::enrich::load(
                &mut crate::enrich::open(repository)?,
                crate::change_id::for_commit(repository, commit_id)?,
            )?
            .note
            .map(|note| note.to_vec()))
        }
    }

    #[test]
    fn note_message_options_match_reword() {
        use clap::Parser;

        for kind in ["note", "git-note"] {
            let parse = |args: &[&str]| {
                let command = super::super::Cli::try_parse_from(
                    ["tix", "enrich", "commit", kind]
                        .into_iter()
                        .chain(args.iter().copied()),
                )
                .expect("note message options parse")
                .platform
                .command;
                let Some(super::super::Command::Enrich(Command::Commit(Commit::Note(args) | Commit::GitNote(args)))) =
                    command
                else {
                    panic!("a note command was expected")
                };
                args
            };
            let args = parse(&[]);
            assert_eq!(args.target.revision, "HEAD", "notes still default to HEAD");
            assert!(
                args.message.is_empty() && args.file.is_none(),
                "no source selects the editor"
            );
            let args = parse(&["topic", "-m", "title", "--message", "body"]);
            assert_eq!(args.target.revision, "topic", "the target can precede message options");
            assert_eq!(
                args.message,
                ["title", "body"],
                "short and long message flags append paragraphs"
            );
            for (flag, file) in [("-f", "note.md"), ("--file", "-")] {
                assert_eq!(
                    parse(&[flag, file]).file,
                    Some(file.into()),
                    "files and stdin use reword's flags"
                );
            }
            assert_eq!(
                super::super::Cli::try_parse_from(["tix", "enrich", "commit", kind, "-m", "text", "-f", "note.md"])
                    .expect_err("message and file inputs are mutually exclusive")
                    .kind(),
                clap::error::ErrorKind::ArgumentConflict
            );
            assert!(
                super::super::Cli::try_parse_from([
                    "tix",
                    "enrich",
                    "commit",
                    kind,
                    "--author",
                    "Agent <agent@example.com>"
                ])
                .is_err(),
                "note input does not expose commit-author options"
            );
        }
    }

    #[test]
    fn explicit_note_messages_replace_and_clear_without_an_editor() -> gix_testtools::Result {
        for git_note in [false, true] {
            let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
            let repository = crate::test_repository::open_with(
                fixture.path(),
                ["core.editor=false", "core.notesRef=refs/notes/agents"],
            )?;
            let target_commit_id = repository.rev_parse_single("HEAD~1")?.detach();
            let head_commit_id = repository.head_id()?.detach();
            let change_id = crate::change_id::for_commit(&repository, target_commit_id)?
                .to_reverse_hex_with_len(7)
                .to_string();
            crate::enrich::ensure_todo(&repository, target_commit_id, true)?;
            crate::enrich::set_note(&repository, target_commit_id, Some(b"original\n"))?;
            crate::set_git_note(
                &repository,
                "refs/notes/agents".try_into()?,
                target_commit_id,
                Some(b"original\n"),
            )?;

            for (paragraphs, expected) in [
                (
                    vec!["title", ";literal\n#literal"],
                    Some(b"title\n\n;literal\n#literal\n".as_slice()),
                ),
                (vec!["replacement"], Some(b"replacement\n".as_slice())),
                (vec![" \n\t"], None),
            ] {
                for repeated in [false, true] {
                    let before = gix_testtools::repository::snapshot(fixture.path())?;
                    let mut args = note_target(&change_id);
                    args.message = paragraphs.iter().map(OsString::from).collect();
                    run(repository.clone(), note_command(git_note, args))?;
                    assert_eq!(
                        saved_note(&repository, target_commit_id, git_note)?.as_deref(),
                        expected,
                        "explicit paragraphs replace the note, and empty input clears it"
                    );
                    assert_eq!(
                        saved_note(&repository, target_commit_id, !git_note)?.as_deref(),
                        Some(b"original\n".as_slice()),
                        "the other note namespace remains untouched"
                    );
                    assert!(
                        crate::enrich::load(
                            &mut crate::enrich::open(&repository)?,
                            crate::change_id::for_commit(&repository, target_commit_id)?,
                        )?
                        .todo,
                        "changing a note preserves the todo marker"
                    );
                    assert_eq!(repository.head_id()?, head_commit_id, "notes never rewrite HEAD");
                    if repeated {
                        assert_eq!(
                            gix_testtools::repository::snapshot(fixture.path())?,
                            before,
                            "an identical note leaves references and undo history unchanged"
                        );
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    fn note_files_preserve_message_bytes_and_read_errors_leave_notes_unchanged() -> gix_testtools::Result {
        for git_note in [false, true] {
            let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
            let repository = crate::test_repository::open_with(fixture.path(), ["core.editor=false"])?;
            let head_commit_id = repository.head_id()?.detach();
            let path = fixture.path().join("note.md");
            std::fs::write(&path, b"\r\n;literal \t\r\n\r\nbody \xff\r\n")?;
            let mut args = note_target("HEAD");
            args.file = Some(path.clone());
            run(repository.clone(), note_command(git_note, args))?;
            assert_eq!(
                saved_note(&repository, head_commit_id, git_note)?.as_deref(),
                Some(b";literal\n\nbody \xff\n".as_slice()),
                "file input uses the existing note cleanup without stripping comments or non-UTF-8 bytes"
            );
            std::fs::remove_file(&path)?;
            let before = gix_testtools::repository::snapshot(fixture.path())?;
            let mut args = note_target("HEAD");
            args.file = Some(path);
            let err = run(repository.clone(), note_command(git_note, args))
                .expect_err("a missing file must not replace the existing note");
            assert!(format!("{err:#}").contains("could not read message"), "{err:#}");
            assert_eq!(
                gix_testtools::repository::snapshot(fixture.path())?,
                before,
                "failed input leaves notes, undo history, and worktree unchanged"
            );
        }
        Ok(())
    }

    fn patch_command(revision: &str, clear: bool) -> Command {
        Command::Patch(Patch::Refackiewed(BooleanTarget {
            clear,
            target: target(revision),
        }))
    }

    #[test]
    fn patch_mark_backfills_head_once_and_then_only_changes_notes() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repository = crate::test_repository::open(fixture.path())?;
        let original = repository.head_id()?.detach();
        run(repository.clone(), patch_command("HEAD", false))?;
        let marked = repository.head_id()?.detach();
        assert_ne!(marked, original, "the first mark stores the patch header in history");
        let patch_id = crate::patch_id::for_commit(&repository, marked)?.expect("marking embeds the patch ID");
        let change_id = crate::change_id::for_commit(&repository, marked)?;
        let undo_before = repository.find_reference(crate::edit::undo::TIP_REF)?.id().detach();
        run(repository.clone(), patch_command("HEAD", false))?;
        assert_eq!(
            repository.head_id()?,
            marked,
            "an existing header prevents further history rewrites"
        );
        assert_eq!(
            repository.find_reference(crate::edit::undo::TIP_REF)?.id(),
            undo_before,
            "an idempotent mark does not add another undo entry"
        );
        run(repository.clone(), patch_command("HEAD", true))?;
        assert_eq!(
            repository.head_id()?,
            marked,
            "clearing leaves the embedded identity untouched"
        );
        assert!(
            !crate::enrich::load_patch(&mut crate::enrich::open_patch(&repository)?, change_id, patch_id)?.refackiewed,
            "the CLI clears the selected patch version"
        );
        Ok(())
    }

    #[test]
    fn legacy_patch_mark_uses_reword_retention_rules_but_clear_needs_no_pin() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repository = crate::test_repository::open(fixture.path())?;
        let original_head = repository.head_id()?.detach();
        let original_target = repository.rev_parse_single("HEAD~1")?.detach();
        let err = run(repository.clone(), patch_command("HEAD~1", false))
            .expect_err("a legacy ancestor needs a covering pin before its header can be added");
        assert!(format!("{err:#}").contains("must be pinned"));
        run(repository.clone(), patch_command("HEAD~1", true))?;
        assert_eq!(
            repository.head_id()?,
            original_head,
            "clearing a legacy mark leaves history untouched"
        );
        assert_eq!(repository.rev_parse_single("HEAD~1")?, original_target);
        assert!(repository.try_find_reference(crate::enrich::PATCH_REF_NAME)?.is_none());
        Ok(())
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
        let repository = crate::test_repository::open_with(
            fixture.path(),
            [format!(
                "core.editor={}",
                crate::test_repository::replacing_editor("old", "new")
            )],
        )?;
        let topic = repository.rev_parse_single("topic")?.detach();
        crate::enrich::ensure_todo(&repository, topic, true)?;
        crate::enrich::set_note(&repository, topic, Some(b"old\n"))?;
        let notes = repository.notes()?;
        let reference = notes.default_ref().context("the fixture has a notes ref")?.to_owned();
        crate::set_git_note(&repository, reference.as_ref(), topic, Some(b"old\n"))?;

        run(repository.clone(), Command::Commit(Commit::Note(note_target("topic"))))?;
        run(
            repository.clone(),
            Command::Commit(Commit::GitNote(note_target("topic"))),
        )?;

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
