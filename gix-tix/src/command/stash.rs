use gix::error::{ResultExt, message};
use std::{collections::HashMap, io::Write};

use gix::Result;

use crate::edit::stash;

#[derive(Debug, clap::Subcommand)]
pub(super) enum Command {
    /// List all commit stashes and this worktree's review stashes, including orphaned associations.
    List,
    /// Restore a listed stash at HEAD, consuming it only after every saved change is restored.
    Restore {
        /// Full stash reference or unique stash-object hash prefix (at least four hex digits).
        #[arg(value_name = "STASH")]
        stash: String,
    },
}

pub(super) fn run(
    repo: &gix::Repository,
    command: Option<Command>,
    mut out: impl Write,
    mut err: impl Write,
) -> Result<()> {
    if !matches!(command, Some(Command::List)) {
        crate::edit::rebase::session::ensure_idle(repo)?;
    }
    match command {
        Some(Command::List) => {
            let graph = crate::edit::loaded_view_graph(repo)?;
            let mut visible = HashMap::new();
            for commit_id in graph.stored_commit_ids() {
                *visible
                    .entry(crate::change_id::for_commit(repo, commit_id)?)
                    .or_insert(0) += 1;
            }
            writeln!(out, "STASH    BASE     STATE       REFERENCE").or_error()?;
            for saved in stash::all(repo)? {
                let stash_commit_id = saved.target.try_id();
                let base_commit_id = stash_commit_id.and_then(|commit_id| {
                    repo.find_commit(commit_id)
                        .ok()
                        .and_then(|commit| commit.parent_ids().next().map(gix::Id::detach))
                });
                let state = if base_commit_id.is_none() {
                    "invalid"
                } else {
                    match stash::associated_change(saved.name.as_bstr()) {
                        Ok(Some(change_id)) => match visible.get(&change_id) {
                            Some(1) => "available",
                            Some(_) => "ambiguous",
                            None => "orphaned",
                        },
                        Ok(None) => "review",
                        Err(_) => "unassociated",
                    }
                };
                let short = |commit_id: Option<gix::ObjectId>| {
                    commit_id.map_or_else(|| "-".to_owned(), |id| id.to_hex_with_len(7).to_string())
                };
                writeln!(
                    out,
                    "{:<8} {:<8} {state:<11} {}",
                    short(stash_commit_id.map(ToOwned::to_owned)),
                    short(base_commit_id),
                    saved.name
                )
                .or_error()?;
            }
        }
        Some(Command::Restore { stash: selector }) => {
            writeln!(err, "{}", stash::restore_selected(repo, &selector)?).or_error()?;
        }
        None => {
            let head_commit_id = repo
                .head_id()
                .or_raise(|| message("stashing changes requires a born HEAD"))?
                .detach();
            let notice = stash::save_manual(repo.git_dir(), repo.is_bare(), head_commit_id)?;
            writeln!(err, "{}", super::notice_with_change_id(repo, &notice, head_commit_id)?).or_error()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listing_exposes_orphans_and_invalid_associations_without_mutation() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let head_commit_id = repo.head_id()?.detach();
        std::fs::write(fixture.path().join("saved"), "saved work\n")?;
        stash::save_manual(repo.git_dir(), false, head_commit_id)?;
        let name = stash::reference(&repo, head_commit_id)?;
        let stash_commit_id = repo.find_reference(name.as_ref())?.id().detach();
        let mut orphan = repo.head_commit()?.decode()?.into_owned()?;
        orphan.message = "unreachable stash association".into();
        let orphan_commit_id = repo.write_object(&orphan)?.detach();
        let orphan_name = stash::reference(&repo, orphan_commit_id)?;
        for name in [orphan_name.as_bstr(), "refs/tix/stash/broken-association".into()] {
            repo.reference(
                name,
                stash_commit_id,
                gix::refs::transaction::PreviousValue::MustNotExist,
                "test stash",
            )?;
        }
        let before = gix_testtools::repository::snapshot(fixture.path())?;
        let mut out = Vec::new();
        let mut err = Vec::new();
        run(&repo, Some(Command::List), &mut out, &mut err)?;
        let out = String::from_utf8(out)?;
        assert!(out.contains("available"), "the current stash is listed: {out}");
        assert!(out.contains("orphaned"), "the unreachable association is listed: {out}");
        assert!(
            out.contains("unassociated"),
            "malformed association names are still recoverable: {out}"
        );
        assert!(
            out.contains(&orphan_name.to_string()),
            "the full recovery reference is printed: {out}"
        );
        assert!(err.is_empty(), "listing writes primary data only to stdout");
        assert!(
            stash::restore_selected(&repo, &stash_commit_id.to_string()).is_err(),
            "one object retained by multiple refs requires an explicit reference"
        );
        assert_eq!(
            gix_testtools::repository::snapshot(fixture.path())?,
            before,
            "listing and rejecting an ambiguous restore change no repository state"
        );
        let mut duplicate = repo.head_commit()?.decode()?.into_owned()?;
        duplicate.message = "another visible version".into();
        crate::change_id::inherit(&repo, &mut duplicate, head_commit_id)?;
        let duplicate_commit_id = repo.write_object(&duplicate)?.detach();
        repo.reference(
            "refs/worktree/tix/pins/otherversion",
            duplicate_commit_id,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "test ambiguous stash association",
        )?;
        let mut ambiguous = Vec::new();
        run(&repo, Some(Command::List), &mut ambiguous, &mut err)?;
        assert!(
            String::from_utf8(ambiguous)?.contains("ambiguous"),
            "listing identifies multiple visible versions of a stashed change"
        );
        run(
            &repo,
            Some(Command::Restore {
                stash: orphan_name.to_string(),
            }),
            Vec::new(),
            &mut err,
        )?;
        assert!(!err.is_empty(), "restoration reports its result on stderr");
        assert_eq!(std::fs::read(fixture.path().join("saved"))?, b"saved work\n");
        assert!(
            repo.try_find_reference(orphan_name.as_ref())?.is_none(),
            "only the selected reference is consumed"
        );
        assert!(
            repo.try_find_reference(name.as_ref())?.is_some(),
            "other saved references remain"
        );
        Ok(())
    }

    #[test]
    fn failed_explicit_restoration_retains_the_stash_and_returns_an_error() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let head_commit_id = repo.head_id()?.detach();
        std::fs::write(fixture.path().join("untracked"), "saved\n")?;
        stash::save_manual(repo.git_dir(), false, head_commit_id)?;
        let name = stash::reference(&repo, head_commit_id)?;
        std::fs::write(fixture.path().join("untracked"), "local\n")?;
        let err =
            stash::restore_selected(&repo, &name.to_string()).expect_err("the existing untracked file blocks restore");
        assert!(err.to_string().contains("stash reference remains"), "{err:#}");
        assert!(
            repo.try_find_reference(name.as_ref())?.is_some(),
            "the complete saved state remains available"
        );
        assert_eq!(
            std::fs::read(fixture.path().join("untracked"))?,
            b"local\n",
            "the local file is preserved"
        );
        Ok(())
    }
}
