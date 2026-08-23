use anyhow::{Context, Result};

#[derive(Debug, clap::Args)]
pub(super) struct Args {
    /// Take changes only from the index.
    #[arg(long, conflicts_with_all = ["worktree", "worktree_untracked"])]
    pub(super) index: bool,
    /// Take only tracked worktree changes, ignoring staged-only changes.
    #[arg(long, conflicts_with_all = ["index", "worktree_untracked"])]
    pub(super) worktree: bool,
    /// Take worktree changes and untracked files, ignoring staged-only changes.
    #[arg(long, conflicts_with_all = ["index", "worktree"])]
    pub(super) worktree_untracked: bool,
    /// Create the commit even when the selected tree is unchanged.
    #[arg(long)]
    pub(super) allow_empty: bool,
    /// Mark the new commit as TODO.
    #[arg(long)]
    pub(super) todo: bool,
    #[command(flatten)]
    pub(super) edit: super::reword::MessageArgs,
}

pub(super) fn run(repository: gix::Repository, args: Args) -> Result<()> {
    let parent = repository
        .head()
        .context("could not read HEAD before creating a commit")?
        .id()
        .map(gix::Id::detach);
    let graph = crate::edit::loaded_graph(&repository)?;
    let source = if args.index {
        crate::edit::create::Source::Index
    } else if args.worktree_untracked {
        crate::edit::create::Source::WorktreeUntracked
    } else if args.worktree {
        crate::edit::create::Source::Worktree
    } else {
        crate::edit::create::Source::Default
    };
    let author = args
        .edit
        .author
        .as_deref()
        .map(gix::path::os_str_into_bstr)
        .transpose()
        .context("author is not valid UTF-8")?;
    let repository_path = repository.git_dir().to_owned();
    let bare = repository.is_bare();
    let prepared = crate::edit::create::prepare_from(repository, parent, source, author, args.todo)?;
    if prepared.is_empty && !args.allow_empty {
        anyhow::bail!("the new commit would be empty; use --allow-empty to create it anyway");
    }

    let explicit = super::reword::explicit_message(&args.edit, std::io::stdin())?;
    let outcome = if let Some(message) = explicit {
        let mut repository = crate::open_repository(&repository_path, bare, false)
            .context("could not reopen repository before creating commit")?;
        repository.object_cache_size(None);
        crate::edit::create::apply_message_reporting(repository, &graph, prepared, &message)?
    } else {
        let Some(edited) = crate::edit::edit_document_without_terminal(
            &prepared.editor,
            &prepared.document,
            &format!("tix-commit-{}.md", std::process::id()),
        )?
        else {
            println!("no commit created: no input was provided");
            return Ok(());
        };
        let mut repository = crate::open_repository(&repository_path, bare, false)
            .context("could not reopen repository after editing commit")?;
        repository.object_cache_size(None);
        crate::edit::create::apply_reporting(repository, &graph, prepared, &edited)?
    };
    let repository = crate::open_repository(&repository_path, bare, false)
        .context("could not reopen repository after creating commit")?;
    let selected = outcome
        .selected
        .context("creating a commit did not produce a selection")?;
    println!("{}", crate::change_id::display(&repository, selected, 7)?);
    super::print_ref_rewrites(&repository, &outcome.ref_rewrites)?;
    super::record_undo(&repository, "create commit", Ok(outcome.ref_changes));
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{path::Path, process::Command};

    use super::*;

    fn args() -> Args {
        Args {
            index: false,
            worktree: false,
            worktree_untracked: false,
            allow_empty: false,
            todo: false,
            edit: super::super::reword::MessageArgs {
                message: vec!["new title".into(), "new body".into()],
                file: None,
                author: Some("New Author <new@example.com>".into()),
            },
        }
    }

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
    fn explicit_message_uses_the_default_staged_tree_and_author() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        run(
            crate::test_repository::open_with(fixture.path(), ["core.editor=false"])?,
            args(),
        )?;

        assert_eq!(git(fixture.path(), &["show", "HEAD:tracked"])?, b"staged\n");
        assert_eq!(
            git(fixture.path(), &["log", "-1", "--format=%B"])?,
            b"new title\n\nnew body\n\n"
        );
        assert_eq!(
            git(fixture.path(), &["log", "-1", "--format=%an <%ae>"])?,
            b"New Author <new@example.com>\n"
        );
        assert_eq!(std::fs::read(fixture.path().join("tracked"))?, b"unstaged\n");
        Ok(())
    }

    #[test]
    fn explicit_message_can_mark_the_new_commit_as_todo() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        let mut input = args();
        input.todo = true;
        run(crate::test_repository::open(fixture.path())?, input)?;

        let repository = crate::test_repository::open(fixture.path())?;
        let id = repository.head_id()?.detach();
        assert!(
            crate::enrich::load(
                &mut crate::enrich::open(&repository)?,
                crate::change_id::for_commit(&repository, id)?,
            )?
            .todo,
            "--todo marks a non-interactive new commit"
        );
        Ok(())
    }

    #[test]
    fn todo_prefills_the_editable_header() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        let repository = crate::test_repository::open(fixture.path())?;
        let parent = repository.head_id()?.detach();
        let prepared = crate::edit::create::prepare_from(
            repository,
            Some(parent),
            crate::edit::create::Source::Default,
            None,
            true,
        )?;
        assert!(
            prepared
                .document
                .windows(b"\nTodo\n".len())
                .any(|window| window == b"\nTodo\n"),
            "--todo activates the existing editable header"
        );
        Ok(())
    }

    #[test]
    fn index_and_worktree_select_only_their_requested_changes() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        std::fs::write(fixture.path().join("staged-only"), b"staged only\n")?;
        git(fixture.path(), &["add", "staged-only"])?;
        let mut worktree = args();
        worktree.worktree = true;
        run(crate::test_repository::open(fixture.path())?, worktree)?;
        assert_eq!(git(fixture.path(), &["show", "HEAD:tracked"])?, b"unstaged\n");
        assert!(git(fixture.path(), &["cat-file", "-e", "HEAD:staged-only"]).is_err());
        assert!(git(fixture.path(), &["cat-file", "-e", "HEAD:untracked"]).is_err());

        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        let mut index = args();
        index.index = true;
        run(crate::test_repository::open(fixture.path())?, index)?;
        assert_eq!(git(fixture.path(), &["show", "HEAD:tracked"])?, b"staged\n");
        Ok(())
    }

    #[test]
    fn worktree_untracked_includes_untracked_but_not_staged_or_ignored_files() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        std::fs::write(fixture.path().join("staged-only"), b"staged only\n")?;
        git(fixture.path(), &["add", "staged-only"])?;
        std::fs::write(fixture.path().join(".git/info/exclude"), b"ignored\n")?;
        std::fs::write(fixture.path().join("ignored"), b"ignored\n")?;
        let mut worktree = args();
        worktree.worktree_untracked = true;
        run(crate::test_repository::open(fixture.path())?, worktree)?;

        assert_eq!(git(fixture.path(), &["show", "HEAD:tracked"])?, b"unstaged\n");
        assert_eq!(git(fixture.path(), &["show", "HEAD:untracked"])?, b"untracked\n");
        assert!(git(fixture.path(), &["cat-file", "-e", "HEAD:staged-only"]).is_err());
        assert!(git(fixture.path(), &["cat-file", "-e", "HEAD:ignored"]).is_err());
        Ok(())
    }

    #[test]
    fn unchanged_selected_trees_require_allow_empty() -> gix_testtools::Result {
        for worktree in [false, true] {
            let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
            crate::test_repository::disable_autocrlf(fixture.path())?;
            if worktree {
                git(fixture.path(), &["checkout", "--", "tracked"])?;
            } else {
                git(fixture.path(), &["reset", "-q", "HEAD"])?;
            }
            let parent = git(fixture.path(), &["rev-parse", "HEAD^{tree}"])?;
            let mut selected = args();
            selected.index = !worktree;
            selected.worktree = worktree;
            let err = run(crate::test_repository::open(fixture.path())?, selected)
                .expect_err("an unchanged selected tree is rejected");
            assert!(format!("{err:#}").contains("--allow-empty"));

            let mut empty = args();
            empty.index = !worktree;
            empty.worktree = worktree;
            empty.allow_empty = true;
            run(crate::test_repository::open(fixture.path())?, empty)?;
            assert_eq!(git(fixture.path(), &["rev-parse", "HEAD^{tree}"])?, parent);
        }
        Ok(())
    }

    #[test]
    fn allow_empty_creates_an_unborn_root() -> gix_testtools::Result {
        let fixture = gix_testtools::tempfile::tempdir()?;
        git(fixture.path(), &["init", "-q", "-b", "main"])?;
        git(fixture.path(), &["config", "user.name", "author"])?;
        git(fixture.path(), &["config", "user.email", "author@example.com"])?;
        let mut empty = args();
        empty.allow_empty = true;
        run(crate::test_repository::open(fixture.path())?, empty)?;

        assert_eq!(git(fixture.path(), &["rev-list", "--count", "HEAD"])?, b"1\n");
        assert_eq!(git(fixture.path(), &["ls-tree", "HEAD"])?, b"");
        Ok(())
    }

    #[test]
    fn unchanged_worktree_untracked_requires_allow_empty() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        crate::test_repository::disable_autocrlf(fixture.path())?;
        git(fixture.path(), &["clean", "-fdq"])?;
        git(fixture.path(), &["checkout", "--", "tracked"])?;
        let parent = git(fixture.path(), &["rev-parse", "HEAD^{tree}"])?;
        let mut selected = args();
        selected.worktree_untracked = true;
        let err = run(crate::test_repository::open(fixture.path())?, selected)
            .expect_err("an unchanged worktree including untracked files is rejected");
        assert!(format!("{err:#}").contains("--allow-empty"));

        let mut empty = args();
        empty.worktree_untracked = true;
        empty.allow_empty = true;
        run(crate::test_repository::open(fixture.path())?, empty)?;
        assert_eq!(git(fixture.path(), &["rev-parse", "HEAD^{tree}"])?, parent);
        Ok(())
    }

    #[test]
    fn unchanged_editor_input_creates_nothing() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        let old = git(fixture.path(), &["rev-parse", "HEAD"])?;
        let mut editor = args();
        editor.edit.message.clear();
        editor.edit.author = None;
        run(
            crate::test_repository::open_with(fixture.path(), ["core.editor=:"])?,
            editor,
        )?;

        assert_eq!(git(fixture.path(), &["rev-parse", "HEAD"])?, old);
        Ok(())
    }
}
