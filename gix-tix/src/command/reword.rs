use std::ffi::OsString;

use anyhow::{Context, Result};

#[derive(Debug, clap::Args)]
pub(super) struct Args {
    /// Revision resolving to the commit whose message should be edited.
    #[arg(value_name = "REVSPEC")]
    pub(super) revision: OsString,
}

pub(super) fn run(repository: gix::Repository, args: Args) -> Result<()> {
    let revision = gix::path::os_str_into_bstr(&args.revision)
        .with_context(|| format!("revision {} is not valid UTF-8", args.revision.to_string_lossy()))?;
    let target = repository
        .rev_parse_single(revision)
        .with_context(|| format!("could not resolve revision {revision:?}"))?
        .object()
        .context("could not read reword target")?
        .peel_to_commit()
        .context("reword target does not resolve to a commit")?
        .id;
    let head = repository.head().context("could not read HEAD before rewording")?;
    let head_id = head.id().map(gix::Id::detach).context("cannot reword an unborn HEAD")?;
    let attached_head = !head.is_detached() && target == head_id;
    drop(head);

    let pins = crate::history::all_pins(&repository)?;
    let mut revisions = vec![OsString::from("HEAD"), OsString::from(target.to_string())];
    revisions.extend(
        pins.iter()
            .map(|pin| gix::path::from_bstr(pin.name.as_bstr()).into_owned().into_os_string()),
    );
    let graph = crate::edit::loaded_view_graph_with(&repository, &revisions)?;
    let covering_pin = pins.iter().any(|pin| graph.is_ancestor(target, pin.id));
    if !attached_head && !covering_pin {
        anyhow::bail!("the reword target or one of its descendants must be pinned");
    }

    let repository_path = repository.git_dir().to_owned();
    let bare = repository.is_bare();
    let (editor, document) = crate::edit::reword::document(&repository, target)?;
    drop(repository);
    let Some(edited) = crate::edit::edit_document_without_terminal(
        &editor,
        &document,
        &format!("tix-reword-{}-{}.md", std::process::id(), target.to_hex_with_len(7)),
    )?
    else {
        println!("no reword performed: the editor document was unchanged");
        return Ok(());
    };

    let mut repository = crate::open_repository(&repository_path, bare, false)
        .context("could not reopen repository after editing commit")?;
    repository.object_cache_size(None);
    match crate::edit::reword::apply(repository, &graph, target, &edited)? {
        Some(id) => println!("{}", id.to_hex_with_len(7)),
        None => println!("no reword performed: the edited commit was unchanged"),
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

    fn open(path: &Path, editor: &str) -> gix_testtools::Result<gix::Repository> {
        Ok(gix::open_opts(
            path,
            gix::open::Options::isolated()
                .config_overrides([format!("core.editor={editor}"), "commit.gpgSign=false".to_owned()]),
        )?)
    }

    fn args(revision: &str) -> Args {
        Args {
            revision: revision.into(),
        }
    }

    #[test]
    fn attached_head_needs_no_pin() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        run(
            open(fixture.path(), "sed -i.bak -e 's/^tip$/rewritten tip/'")?,
            args("HEAD"),
        )?;

        assert_eq!(git(fixture.path(), &["log", "-1", "--format=%s"])?, b"rewritten tip\n");
        let repository = crate::open_test_repository(fixture.path())?;
        assert!(!repository.head()?.is_detached());
        assert!(crate::history::all_pins(&repository)?.is_empty());
        Ok(())
    }

    #[test]
    fn detached_head_requires_a_pin_before_the_editor() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        git(fixture.path(), &["checkout", "-q", "--detach", "HEAD"])?;
        let err = run(open(fixture.path(), "false")?, args("HEAD"))
            .expect_err("a detached HEAD does not provide a durable rewrite tip");
        assert!(format!("{err:#}").contains("must be pinned"));
        assert!(crate::history::all_pins(&crate::open_test_repository(fixture.path())?)?.is_empty());
        Ok(())
    }

    #[test]
    fn other_targets_require_a_covering_pin_before_the_editor() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let path = fixture.path();
        let err = run(open(path, "false")?, args("HEAD~1")).expect_err("an ancestor without a pin is rejected");
        assert!(format!("{err:#}").contains("must be pinned"));

        let repository = crate::open_test_repository(path)?;
        let old_tip = repository.head_id()?.detach();
        repository.reference(
            "refs/worktree/tix/pins/keep",
            old_tip,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "test pin",
        )?;
        drop(repository);
        run(
            open(path, "sed -i.bak -e 's/^middle$/rewritten middle/'")?,
            args("HEAD~1"),
        )?;

        let repository = crate::open_test_repository(path)?;
        let new_tip = repository.head_id()?.detach();
        assert_ne!(new_tip, old_tip, "the descendant is lazily reparented");
        assert_eq!(
            repository
                .find_commit(new_tip)?
                .parent_ids()
                .next()
                .map(gix::Id::detach),
            Some(repository.rev_parse_single("HEAD~1")?.detach()),
            "the rewritten descendant points to the edited commit"
        );
        assert_eq!(
            crate::history::all_pins(&repository)?[0].id,
            new_tip,
            "the covering pin follows its rewritten descendant"
        );
        assert!(
            repository
                .find_commit(new_tip)?
                .decode()?
                .extra_headers()
                .find("tix-rebase-parent")
                .is_some(),
            "the descendant remains marked for lazy replay"
        );
        assert_eq!(
            git(path, &["log", "-1", "--format=%s", "HEAD~1"])?,
            b"rewritten middle\n"
        );
        Ok(())
    }

    #[test]
    fn a_pin_can_expose_an_unrelated_reword_stack() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let path = fixture.path();
        git(path, &["branch", "side", "HEAD~2"])?;
        git(path, &["checkout", "-q", "side"])?;
        git(path, &["commit", "-q", "--allow-empty", "-m", "side"])?;
        let side = String::from_utf8(git(path, &["rev-parse", "HEAD"])?)?;
        let side = side.trim();
        git(path, &["checkout", "-q", "main"])?;
        git(path, &["update-ref", "refs/worktree/tix/pins/side", side])?;
        let main = git(path, &["rev-parse", "main"])?;

        run(open(path, "sed -i.bak -e 's/^side$/rewritten side/'")?, args("side"))?;

        assert_eq!(
            git(path, &["rev-parse", "main"])?,
            main,
            "the unrelated checkout stack is untouched"
        );
        assert_eq!(git(path, &["log", "-1", "--format=%s", "side"])?, b"rewritten side\n");
        let repository = crate::open_test_repository(path)?;
        assert_eq!(
            crate::history::all_pins(&repository)?[0].id,
            repository.rev_parse_single("side")?.detach(),
            "the explicit unrelated pin follows the rewritten commit"
        );
        Ok(())
    }
}
