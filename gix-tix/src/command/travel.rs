use std::ffi::OsString;

use anyhow::{Context, Result};

#[derive(Debug, clap::Args)]
pub(super) struct Args {
    /// Check out an encountered replay conflict and write its unmerged index.
    #[arg(long)]
    pub(super) materialize_conflicts: bool,
    /// Revision resolving to the commit to visit.
    #[arg(value_name = "REVSPEC")]
    pub(super) revision: OsString,
}

pub(super) fn run(repository: gix::Repository, args: Args) -> Result<()> {
    let (selected, resolved_graph) = super::resolve_commit(&repository, &args.revision, "time-travel destination")?;
    let head = repository.head().context("could not read HEAD before time-travel")?;
    let head_id = head
        .id()
        .map(gix::Id::detach)
        .context("cannot time-travel from an unborn HEAD")?;
    let detached = head.is_detached();
    drop(head);
    if selected == head_id {
        println!("already at {}", crate::change_id::display(&repository, selected, 7)?);
        return Ok(());
    }

    let revisions = vec![OsString::from("HEAD"), OsString::from(selected.to_string())];
    let graph = match resolved_graph {
        Some(graph) => graph,
        None => crate::edit::loaded_view_graph_with(&repository, &revisions)?,
    };
    let forward = graph.is_ancestor(head_id, selected);
    if detached && !forward {
        let source_is_pinned = crate::history::all_pins(&repository)?
            .into_iter()
            .any(|pin| graph.is_ancestor(head_id, pin.id));
        if !source_is_pinned {
            anyhow::bail!(
                "detached HEAD or one of its descendants must be pinned before travelling into the past or sideways"
            );
        }
    }

    let reviews = crate::history::all_reviews(&repository)?
        .into_iter()
        .map(|review| review.id)
        .collect::<Vec<_>>();
    let repository_path = repository.git_dir().to_owned();
    let bare = repository.is_bare();
    drop(repository);
    match crate::edit::time_travel::perform(&repository_path, bare, selected, &graph, &reviews, &[], false)? {
        crate::edit::time_travel::Perform::Complete {
            notice,
            selected,
            ref_rewrites,
        } => {
            let repository = crate::open_repository(&repository_path, bare, false)
                .context("could not reopen repository after time-travel")?;
            println!(
                "{}",
                super::notice_with_change_id(
                    &repository,
                    &notice.unwrap_or_else(|| format!("already at {}", selected.to_hex_with_len(7))),
                    selected,
                )?
            );
            super::print_ref_rewrites(&repository, &ref_rewrites)?;
        }
        crate::edit::time_travel::Perform::Conflict(conflict) if args.materialize_conflicts => {
            let (notice, _, ref_rewrites) = conflict.accept()?;
            let repository = crate::open_repository(&repository_path, bare, false)
                .context("could not reopen repository after materializing time-travel")?;
            super::print_ref_rewrites(&repository, &ref_rewrites)?;
            anyhow::bail!("{notice}");
        }
        crate::edit::time_travel::Perform::Conflict(_) => {
            anyhow::bail!("time-travel would conflict; retry with --materialize-conflicts to check it out")
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{path::Path, process::Command};

    use gix::bstr::ByteSlice;

    use super::*;

    fn git(path: &Path, args: &[&str]) -> gix_testtools::Result<Vec<u8>> {
        let output = Command::new("git").arg("-C").arg(path).args(args).output()?;
        if !output.status.success() {
            return Err(format!("git {} failed: {}", args.join(" "), output.stderr.trim().to_str_lossy()).into());
        }
        Ok(output.stdout)
    }

    fn args(revision: &str) -> Args {
        Args {
            materialize_conflicts: false,
            revision: revision.into(),
        }
    }

    #[test]
    fn attached_past_travel_saves_and_returns_to_the_branch() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repository = crate::test_repository::open(fixture.path())?;
        let middle = repository.rev_parse_single("HEAD~1")?.detach();
        let change_id = crate::change_id::for_commit(&repository, middle)?
            .to_reverse_hex_with_len(7)
            .to_string();
        run(repository, args(&change_id))?;

        let repository = crate::test_repository::open(fixture.path())?;
        assert_eq!(repository.head_id()?.detach(), middle);
        assert!(repository.head()?.is_detached());
        let pins = crate::history::all_pins(&repository)?;
        assert_eq!(pins.len(), 1, "leaving the attached tip creates one source pin");
        assert_eq!(
            pins[0].target.try_name().expect("the source pin is symbolic").as_bstr(),
            b"refs/heads/main".as_bstr(),
            "the source pin follows the departed branch"
        );

        run(repository, args("main"))?;
        let repository = crate::test_repository::open(fixture.path())?;
        assert_eq!(
            repository.head()?.referent_name().map(|name| name.as_bstr().to_owned()),
            Some(b"refs/heads/main".as_bstr().to_owned()),
            "travelling to the pinned destination reattaches HEAD"
        );
        assert!(crate::history::all_pins(&repository)?.is_empty());
        run(repository, args("HEAD"))?;
        let repository = crate::test_repository::open(fixture.path())?;
        assert!(
            !repository.head()?.is_detached(),
            "travelling to the current attached HEAD is a no-op"
        );
        assert!(crate::history::all_pins(&repository)?.is_empty());
        Ok(())
    }

    #[test]
    fn detached_travel_needs_a_source_pin_except_toward_descendants() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let path = fixture.path();
        git(path, &["branch", "side", "HEAD~2"])?;
        git(path, &["checkout", "-q", "side"])?;
        git(path, &["commit", "-q", "--allow-empty", "-m", "side"])?;
        git(path, &["checkout", "-q", "--detach", "main~1"])?;
        let repository = crate::test_repository::open(path)?;
        run(repository, args("main"))?;
        let repository = crate::test_repository::open(path)?;
        assert!(repository.head()?.is_detached());
        assert!(crate::history::all_pins(&repository)?.is_empty());

        let before_rejected = repository.head_id()?.detach();
        let err = run(repository, args("HEAD~1")).expect_err("past travel from detached HEAD needs a pin");
        assert!(format!("{err:#}").contains("must be pinned"));
        let repository = crate::test_repository::open(path)?;
        assert_eq!(
            repository.head_id()?.detach(),
            before_rejected,
            "the rejected command does not move HEAD"
        );
        let err = run(repository, args("side")).expect_err("sideways travel from detached HEAD needs a pin");
        assert!(format!("{err:#}").contains("must be pinned"));
        let repository = crate::test_repository::open(path)?;
        let tip = repository.head_id()?.detach();
        repository.reference(
            "refs/worktree/tix/pins/keep",
            tip,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "test pin",
        )?;
        run(repository, args("side"))?;
        Ok(())
    }

    #[test]
    fn replay_conflicts_are_unobservable_until_materialization_is_requested() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_conflict.sh")?;
        let path = fixture.path();
        git(
            path,
            &["config", "gitoxide.commit.committerDate", "2001-01-01T00:00:00 +0000"],
        )?;
        let repository = crate::test_repository::open(path)?;
        let middle = repository.rev_parse_single("HEAD~1")?.detach();
        std::fs::write(path.join("after"), "after\n")?;
        git(path, &["add", "after"])?;
        git(path, &["commit", "-q", "-m", "after"])?;
        let graph = crate::edit::loaded_graph(&repository)?;
        crate::edit::rebase::perform(
            &repository,
            &graph,
            crate::edit::rebase::Edit::Remove { target: middle },
            crate::edit::rebase::Signature::RedoIfNeeded,
            crate::edit::rebase::Tree::LeaveAsIsAndMark,
        )?
        .complete()?;
        let root = repository.rev_parse_single("HEAD~2")?.detach();
        let tip = repository.find_reference("refs/heads/main")?.id().detach();
        drop(repository);

        run(crate::test_repository::open(path)?, args(&root.to_string()))?;
        let before = gix_testtools::repository::snapshot(path)?;
        let err = run(crate::test_repository::open(path)?, args(&tip.to_string()))
            .expect_err("a conflict needs explicit materialization");
        assert!(format!("{err:#}").contains("--materialize-conflicts"));
        assert_eq!(
            gix_testtools::repository::snapshot(path)?,
            before,
            "declining materialization leaves the complete repository unchanged"
        );

        let err = run(
            crate::test_repository::open(path)?,
            Args {
                materialize_conflicts: true,
                revision: tip.to_string().into(),
            },
        )
        .expect_err("a materialized conflict remains an incomplete command");
        assert!(format!("{err:#}").contains("ready to resolve conflicts"));
        assert!(
            crate::test_repository::open(path)?
                .index_or_empty()?
                .entries()
                .iter()
                .any(|entry| entry.stage() != gix::index::entry::Stage::Unconflicted),
            "opt-in materialization writes the unresolved index"
        );
        Ok(())
    }
}
