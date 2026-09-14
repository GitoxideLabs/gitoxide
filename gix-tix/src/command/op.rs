use std::io::Write;

use anyhow::{Context, Result};

use crate::edit::undo;

#[derive(Debug, clap::Subcommand)]
pub(super) enum Command {
    /// Print this worktree's operation history, newest first, with @ at the current position.
    Log,
    /// Undo one operation, preserving local changes or failing if they would be overwritten.
    Undo,
    /// Redo one operation, preserving local changes or failing if they would be overwritten.
    Redo,
    /// Clear this worktree's undo and redo history without reversing any operations.
    Clear,
}

pub(super) fn run(
    repository: &gix::Repository,
    command: Option<Command>,
    mut out: impl Write,
    mut err: impl Write,
) -> Result<()> {
    if !matches!(command, None | Some(Command::Log)) {
        crate::edit::rebase::session::ensure_idle(repository)?;
    }
    match command.unwrap_or(Command::Log) {
        Command::Log => {
            let history = undo::history(repository)?;
            let width = history.titles.len().to_string().len();
            let marker = |position| if position == history.applied { '@' } else { ' ' };
            for (index, title) in history.titles.iter().enumerate().rev() {
                let position = index + 1;
                let state = if position <= history.applied {
                    "applied"
                } else {
                    "undone"
                };
                writeln!(out, "{} {position:>width$}  {state:<7}  {title}", marker(position))
                    .context("could not write operation history")?;
            }
            writeln!(out, "{} {:>width$}  start of undo history", marker(0), 0)
                .context("could not write operation history")?;
        }
        Command::Clear => undo::clear(repository)?,
        direction @ (Command::Undo | Command::Redo) => {
            let (planned, verb, past) = if matches!(direction, Command::Undo) {
                (undo::plan_undo(repository), "undo", "Undid")
            } else {
                (undo::plan_redo(repository), "redo", "Redid")
            };
            let Some(plan) = planned.with_context(|| format!("could not plan {verb}"))? else {
                writeln!(err, "nothing to {verb}").context("could not write operation feedback")?;
                return Ok(());
            };
            let title = plan.title.clone();
            let position = plan.position.clone();
            plan.apply(repository)
                .with_context(|| format!("could not {verb} {title}"))?;
            writeln!(err, "{past}: {title} ({} undo, {} redo)", position.undo, position.redo)
                .context("could not write operation feedback")?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    fn invoke(repository: &gix::Repository, arguments: &[&str]) -> Result<(String, String)> {
        let platform = super::super::Cli::try_parse_from(arguments)?.platform;
        platform.validate_command_options()?;
        let Some(super::super::Command::Op { command }) = platform.command else {
            anyhow::bail!("expected an operation command");
        };
        let mut out = Vec::new();
        let mut err = Vec::new();
        run(repository, command, &mut out, &mut err)?;
        Ok((String::from_utf8(out)?, String::from_utf8(err)?))
    }

    #[test]
    fn log_and_recovery_commands_share_the_persisted_queue() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repository = crate::test_repository::open(fixture.path())?;
        let empty = gix_testtools::repository::snapshot(fixture.path())?;
        assert_eq!(
            invoke(&repository, &["tix", "op"])?,
            ("@ 0  start of undo history\n".into(), String::new()),
            "bare op writes only its empty history to stdout"
        );
        for verb in ["undo", "redo"] {
            assert_eq!(
                invoke(&repository, &["tix", "op", verb])?,
                (String::new(), format!("nothing to {verb}\n")),
                "empty boundaries succeed with feedback only on stderr"
            );
        }
        assert_eq!(
            gix_testtools::repository::snapshot(fixture.path())?,
            empty,
            "reading an empty queue and attempting empty steps creates no repository state"
        );

        for content in ["first\n", "second\n", "third\n"] {
            std::fs::write(fixture.path().join("tip"), content)?;
            super::super::Cli::try_parse_from(["tix", "new", "--worktree", "-m", content])?
                .platform
                .run(crate::test_repository::open(fixture.path())?.into_sync())?;
        }
        let complete = gix_testtools::repository::snapshot(fixture.path())?;
        let (out, err) = invoke(&repository, &["tix", "op", "undo"])?;
        assert!(out.is_empty(), "undo leaves stdout available for data");
        assert_eq!(err, "Undid: create commit (2 undo, 1 redo)\n");
        assert_eq!(std::fs::read(fixture.path().join("tip"))?, b"second\n");
        let middle = gix_testtools::repository::snapshot(fixture.path())?;
        let log = (
            "  3  undone   create commit\n@ 2  applied  create commit\n  1  applied  create commit\n  0  start of undo history\n".into(),
            String::new(),
        );
        assert_eq!(invoke(&repository, &["tix", "op", "log"])?, log);
        assert_eq!(invoke(&repository, &["tix", "op"])?, log, "bare op defaults to log");
        assert_eq!(
            gix_testtools::repository::snapshot(fixture.path())?,
            middle,
            "listing operations leaves the queue and checkout untouched"
        );

        assert_eq!(
            invoke(&repository, &["tix", "op", "redo"])?,
            (String::new(), "Redid: create commit (3 undo, 0 redo)\n".into())
        );
        let redone = gix_testtools::repository::snapshot(fixture.path())?;
        assert_eq!(redone.head, complete.head, "redo restores the checkout");
        assert_eq!(redone.index_tree, complete.index_tree, "redo restores the index");
        assert_eq!(redone.worktree, complete.worktree, "redo restores the worktree");

        for _ in 0..3 {
            invoke(&repository, &["tix", "op", "undo"])?;
        }
        assert_eq!(
            invoke(&repository, &["tix", "op", "log"])?.0,
            "  3  undone   create commit\n  2  undone   create commit\n  1  undone   create commit\n@ 0  start of undo history\n",
            "undoing every operation marks the sentinel as the current position"
        );
        super::super::Cli::try_parse_from(["tix", "new", "--allow-empty", "-m", "replacement"])?
            .platform
            .run(crate::test_repository::open(fixture.path())?.into_sync())?;
        assert_eq!(
            invoke(&repository, &["tix", "op", "log"])?.0,
            "@ 1  applied  create commit\n  0  start of undo history\n",
            "a new operation replaces the undone tail"
        );
        assert_eq!(invoke(&repository, &["tix", "op", "redo"])?.1, "nothing to redo\n");
        assert_eq!(
            invoke(&repository, &["tix", "op", "clear"])?,
            (String::new(), String::new()),
            "clearing history succeeds silently"
        );
        assert_eq!(invoke(&repository, &["tix", "op"])?.0, "@ 0  start of undo history\n");
        Ok(())
    }

    #[test]
    fn undo_propagates_conflict_rejection_without_changing_the_repository() -> gix_testtools::Result {
        let (fixture, _) = crate::edit::head::tests::merge_conflict_fixture()?;
        let repository = crate::test_repository::open(fixture.path())?;
        let head_commit_id = repository.head_id()?.detach();
        super::super::Cli::try_parse_from(["tix", "pin", &head_commit_id.to_string()])?
            .platform
            .run(crate::test_repository::open(fixture.path())?.into_sync())?;
        let before = gix_testtools::repository::snapshot(fixture.path())?;
        let err = invoke(&repository, &["tix", "op", "undo"]).expect_err("an unmerged index blocks undo");
        assert!(format!("{err:#}").contains("cannot undo/redo with unresolved index conflicts"));
        assert_eq!(
            gix_testtools::repository::snapshot(fixture.path())?,
            before,
            "failed undo preserves references, cursor, index, and files"
        );
        Ok(())
    }
}
