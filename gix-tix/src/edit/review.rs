use std::{path::Path, process::Command};

use anyhow::{Context, Result};
use gix::{
    ObjectId,
    bstr::{BStr, BString, ByteSlice},
};

use crate::{history, open_repository};

const HEADER: &[u8] = b"tix-rebase";
const ONTO: &[u8] = b"onto ";

#[derive(Debug)]
pub(crate) struct Started {
    pub commit: ObjectId,
    pub reference: gix::refs::FullName,
}

pub(crate) fn reference(commit: &gix::objs::Commit) -> Result<Option<gix::refs::FullName>> {
    commit
        .extra_headers
        .iter()
        .find_map(|(name, value)| {
            (name.as_slice() == HEADER)
                .then(|| value.as_slice().strip_prefix(ONTO))
                .flatten()
        })
        .map(|name| {
            if history::review_number(name.as_bstr()).is_none() {
                anyhow::bail!("review commit names an invalid review reference");
            }
            BString::from(name)
                .try_into()
                .context("review commit names an invalid reference")
        })
        .transpose()
}

pub(crate) fn is_review(commit: &gix::objs::Commit) -> bool {
    reference(commit).ok().flatten().is_some()
}

pub(super) fn deletions(
    repo: &gix::Repository,
    commit: &gix::objs::Commit,
) -> Result<Vec<(gix::refs::FullName, gix::refs::Target)>> {
    let Some(name) = reference(commit)? else {
        return Ok(Vec::new());
    };
    resources(repo, name)
}

pub(super) fn resources(
    repo: &gix::Repository,
    name: gix::refs::FullName,
) -> Result<Vec<(gix::refs::FullName, gix::refs::Target)>> {
    let stash = stash_reference(name.as_bstr())?;
    let mut out = Vec::new();
    for name in [name, stash] {
        if let Some(reference) = repo.try_find_reference(name.as_ref())? {
            out.push((name, reference.target().into_owned()));
        }
    }
    Ok(out)
}

pub(super) fn stash_reference(review: &BStr) -> Result<gix::refs::FullName> {
    let number = history::review_number(review).context("review reference has no numeric identity")?;
    format!(
        "{}{}",
        String::from_utf8_lossy(history::REVIEW_STASH_PREFIX),
        number.to_str_lossy()
    )
    .try_into()
    .context("generated an invalid review stash reference")
}

#[tracing::instrument(skip_all, fields(%tip, %base))]
pub(crate) fn start(
    repository_path: &Path,
    bare: bool,
    graph: &history::HistoryGraph,
    tip: ObjectId,
    base: ObjectId,
) -> Result<Started> {
    let repo = open_repository(repository_path, bare, false).context("could not open repository to start review")?;
    let workdir = repo.workdir().context("review requires a worktree")?.to_owned();
    let head = repo.head().context("could not read HEAD before review")?;
    let restore = (
        head.referent_name().map(|name| name.as_bstr().to_owned()),
        head.id().map(gix::Id::detach),
    );
    let reattach = (restore.1 == Some(tip)).then(|| restore.0.clone()).flatten();
    if tip == base || !graph.is_ancestor(base, tip) {
        anyhow::bail!("the review base must be an ancestor of the reviewed commit");
    }
    for (label, id) in [("reviewed commit", tip), ("review base", base)] {
        let commit = repo
            .find_commit(id)
            .with_context(|| format!("could not find {label}"))?
            .decode()?
            .into_owned()?;
        if super::rebase::is_pending(&commit) {
            anyhow::bail!("{label} has a pending rebase");
        }
    }
    ensure_clean(&workdir)?;

    let name = next_reference(&repo)?;
    let mut commit = gix::objs::Commit {
        tree: repo.find_commit(base)?.tree_id()?.detach(),
        parents: [base].into_iter().collect(),
        author: repo
            .author()
            .context("no Git author is configured")?
            .context("could not resolve the Git author")?
            .to_owned()?,
        committer: repo
            .committer()
            .context("no Git committer is configured")?
            .context("could not resolve the Git committer")?
            .to_owned()?,
        encoding: None,
        message: "review".into(),
        extra_headers: Vec::new(),
    };
    commit
        .extra_headers
        .push((HEADER.into(), format!("onto {name}").into()));
    let id = repo
        .write_object(&commit)
        .context("could not write review commit")?
        .detach();
    drop(repo);

    git(&workdir, ["checkout", "--quiet", "--detach", &tip.to_string()])
        .context("could not check out the reviewed commit")?;
    let review_name = name.as_bstr().to_str_lossy();
    let create_ref = match reattach.as_ref() {
        Some(target) => git(
            &workdir,
            ["symbolic-ref", review_name.as_ref(), target.to_str_lossy().as_ref()],
        ),
        None => git(&workdir, ["update-ref", review_name.as_ref(), &tip.to_string()]),
    };
    if let Err(err) = create_ref {
        restore_checkout(&workdir, &restore)?;
        return Err(err.context("could not create review reference"));
    }
    if let Err(err) = git(
        &workdir,
        ["update-ref", "--no-deref", "HEAD", &id.to_string(), &tip.to_string()],
    ) {
        let _ = git(&workdir, ["update-ref", "--no-deref", "-d", review_name.as_ref()]);
        restore_checkout(&workdir, &restore)?;
        return Err(err.context("could not attach the worktree to the review commit"));
    }
    if let Err(err) = git(&workdir, ["read-tree", &id.to_string()]) {
        let _ = git(
            &workdir,
            ["update-ref", "--no-deref", "HEAD", &tip.to_string(), &id.to_string()],
        );
        let _ = git(&workdir, ["update-ref", "--no-deref", "-d", review_name.as_ref()]);
        restore_checkout(&workdir, &restore)?;
        return Err(err.context("could not reset the index to the review base"));
    }
    Ok(Started {
        commit: id,
        reference: name,
    })
}

#[tracing::instrument(skip_all, fields(%review))]
pub(crate) fn finish(repo: gix::Repository, graph: &history::HistoryGraph, review: ObjectId) -> Result<ObjectId> {
    let workdir = repo
        .workdir()
        .context("finishing review requires a worktree")?
        .to_owned();
    if repo.head_id()?.detach() != review {
        anyhow::bail!("the review commit must be checked out before it can be finished");
    }
    ensure_clean(&workdir)?;
    let commit = repo.find_commit(review)?.decode()?.into_owned()?;
    let review_ref = reference(&commit)?.context("the selected commit is not an active review")?;
    let base = commit
        .parents
        .first()
        .copied()
        .context("a review commit must have a base")?;
    let mut reference = repo
        .find_reference(review_ref.as_ref())
        .context("the review reference is missing")?;
    let reattach = reference.target().try_name().map(ToOwned::to_owned);
    let tip = reference
        .peel_to_id()
        .context("the review reference does not resolve")?
        .detach();
    let delete_refs = resources(&repo, review_ref.clone())?;
    for (label, id) in [("reviewed commit", tip), ("review base", base)] {
        let endpoint = repo.find_commit(id)?.decode()?.into_owned()?;
        if super::rebase::is_pending(&endpoint) {
            anyhow::bail!("{label} has a pending rebase");
        }
    }
    let finished = super::rebase::finish_review(&repo, graph, review, tip, review_ref, delete_refs)?
        .selected
        .context("finishing review did not produce a commit")?;
    if let Some(reattach) = reattach {
        let mut reference = repo
            .find_reference(reattach.as_ref())
            .context("the branch to restore after review is missing")?;
        if reference.peel_to_id()?.detach() != finished {
            anyhow::bail!("the branch to restore after review no longer points to the finished commit");
        }
        git(
            &workdir,
            ["symbolic-ref", "HEAD", reattach.as_bstr().to_str_lossy().as_ref()],
        )
        .context("could not reattach HEAD after review")?;
    }
    Ok(finished)
}

pub(super) fn ensure_clean(workdir: &Path) -> Result<()> {
    if is_dirty(workdir)? {
        anyhow::bail!("review requires a clean index and worktree");
    }
    Ok(())
}

pub(super) fn is_dirty(workdir: &Path) -> Result<bool> {
    let output = Command::new("git")
        .arg("-C")
        .arg(workdir)
        .args(["status", "--porcelain=v1", "--untracked-files=all"])
        .output()
        .context("could not inspect worktree status")?;
    if !output.status.success() {
        anyhow::bail!("{}", String::from_utf8_lossy(&output.stderr).trim());
    }
    Ok(!output.stdout.is_empty())
}

fn next_reference(repo: &gix::Repository) -> Result<gix::refs::FullName> {
    for number in 1_u64.. {
        let name: gix::refs::FullName = format!("{}{number}", String::from_utf8_lossy(history::REVIEW_PREFIX))
            .try_into()
            .context("generated an invalid review reference")?;
        if repo.try_find_reference(name.as_ref())?.is_none() {
            return Ok(name);
        }
    }
    unreachable!("u64 review numbers cannot be exhausted")
}

fn git<const N: usize>(workdir: &Path, args: [&str; N]) -> Result<()> {
    let output = Command::new("git").arg("-C").arg(workdir).args(args).output()?;
    if output.status.success() {
        Ok(())
    } else {
        anyhow::bail!("{}", String::from_utf8_lossy(&output.stderr).trim())
    }
}

fn restore_checkout(workdir: &Path, restore: &(Option<BString>, Option<ObjectId>)) -> Result<()> {
    let mut command = Command::new("git");
    command.arg("-C").arg(workdir).args(["checkout", "--quiet", "--force"]);
    match restore {
        (Some(name), _) => {
            let name = gix::path::from_bstr(name);
            command.arg(name.as_ref());
        }
        (None, Some(id)) => {
            command.args(["--detach", &id.to_string()]);
        }
        (None, None) => anyhow::bail!("cannot restore an unborn checkout after review setup failed"),
    }
    let output = command
        .output()
        .context("could not restore checkout after review setup failed")?;
    if output.status.success() {
        Ok(())
    } else {
        anyhow::bail!("{}", String::from_utf8_lossy(&output.stderr).trim())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(path: &Path, args: &[&str]) -> gix_testtools::Result<Vec<u8>> {
        let output = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(args)
            .env("GIT_AUTHOR_DATE", "2001-01-01T00:00:00 +0000")
            .env("GIT_COMMITTER_DATE", "2001-01-01T00:00:00 +0000")
            .output()?;
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
    fn starts_review_with_base_index_and_tip_worktree() -> gix_testtools::Result {
        let fixture = gix_testtools::tempfile::tempdir()?;
        run(fixture.path(), &["init", "-q", "-b", "main"])?;
        run(fixture.path(), &["config", "user.name", "reviewer"])?;
        run(fixture.path(), &["config", "user.email", "reviewer@example.com"])?;
        std::fs::write(fixture.path().join("file"), "base\n")?;
        run(fixture.path(), &["add", "file"])?;
        run(
            fixture.path(),
            &["-c", "commit.gpgSign=false", "commit", "-q", "-m", "base"],
        )?;
        let base = ObjectId::from_hex(run(fixture.path(), &["rev-parse", "HEAD"])?.trim())?;
        std::fs::write(fixture.path().join("file"), "tip\n")?;
        run(fixture.path(), &["-c", "commit.gpgSign=false", "commit", "-qam", "tip"])?;
        let tip = ObjectId::from_hex(run(fixture.path(), &["rev-parse", "HEAD"])?.trim())?;
        std::fs::write(fixture.path().join("natural"), "natural\n")?;
        run(fixture.path(), &["add", "natural"])?;
        run(
            fixture.path(),
            &["-c", "commit.gpgSign=false", "commit", "-q", "-m", "natural descendant"],
        )?;
        run(fixture.path(), &["branch", "natural"])?;
        run(fixture.path(), &["reset", "--hard", &tip.to_string()])?;

        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=reviewer", "user.email=reviewer@example.com"],
        )?;
        let graph = super::super::loaded_graph(&repo)?;
        drop(repo);
        let started = start(fixture.path(), false, &graph, tip, base)?;

        let repo = crate::test_repository::open(fixture.path())?;
        assert_eq!(
            repo.head_id()?.detach(),
            started.commit,
            "HEAD selects the review commit"
        );
        assert_eq!(
            repo.find_commit(started.commit)?.tree_id()?,
            repo.find_commit(base)?.tree_id()?
        );
        let review_ref = repo.find_reference(started.reference.as_ref())?;
        assert_eq!(
            review_ref.target().try_name().map(gix::refs::FullNameRef::as_bstr),
            Some(BStr::new(b"refs/heads/main")),
            "the review resource remembers the branch to restore"
        );
        let commit = repo.find_commit(started.commit)?.decode()?.into_owned()?;
        assert_eq!(reference(&commit)?, Some(started.reference.clone()));
        assert_eq!(
            std::fs::read(fixture.path().join("file"))?,
            b"tip\n",
            "reviewed content stays in worktree"
        );
        assert_eq!(
            run(fixture.path(), &["diff", "--name-only"])?,
            b"file\n",
            "the reviewed change is unstaged"
        );
        assert!(run(fixture.path(), &["diff", "--cached", "--name-only"])?.is_empty());

        run(fixture.path(), &["add", "file"])?;
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=reviewer", "user.email=reviewer@example.com"],
        )?;
        let graph = super::super::loaded_graph(&repo)?;
        let amended = super::super::head::perform(repo, &graph, super::super::head::Kind::Amend, None)?
            .expect("staging the reviewed delta amends it into the review commit");
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=reviewer", "user.email=reviewer@example.com"],
        )?;
        assert!(is_review(&repo.find_commit(amended)?.decode()?.into_owned()?));
        assert!(run(fixture.path(), &["status", "--porcelain=v1", "--untracked-files=all"])?.is_empty());
        let child = run(
            fixture.path(),
            &[
                "-c",
                "commit.gpgSign=false",
                "commit-tree",
                &format!("{amended}^{{tree}}"),
                "-p",
                &amended.to_string(),
                "-m",
                "review child",
            ],
        )?;
        let child = ObjectId::from_hex(child.trim())?;
        run(
            fixture.path(),
            &["update-ref", "refs/heads/review-child", &child.to_string()],
        )?;
        let stash_ref = stash_reference(started.reference.as_bstr())?;
        run(
            fixture.path(),
            &[
                "update-ref",
                stash_ref.as_bstr().to_str_lossy().as_ref(),
                &tip.to_string(),
            ],
        )?;
        drop(repo);
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=reviewer", "user.email=reviewer@example.com"],
        )?;
        let graph = super::super::loaded_graph(&repo)?;
        let finished = finish(repo, &graph, amended)?;
        let repo = crate::test_repository::open(fixture.path())?;
        assert_eq!(repo.head_id()?.detach(), finished);
        assert_eq!(
            repo.head()?.referent_name().map(gix::refs::FullNameRef::as_bstr),
            Some(BStr::new(b"refs/heads/main")),
            "finishing reattaches HEAD to the branch saved by the review resource"
        );
        assert_eq!(
            repo.find_commit(finished)?.parent_ids().next().map(gix::Id::detach),
            Some(tip)
        );
        assert!(!is_review(&repo.find_commit(finished)?.decode()?.into_owned()?));
        let child = repo.find_reference("refs/heads/review-child")?.id().detach();
        assert_eq!(
            repo.find_commit(child)?.parent_ids().next().map(gix::Id::detach),
            Some(finished)
        );
        let natural = repo.find_reference("refs/heads/natural")?.id().detach();
        assert_eq!(
            repo.find_commit(natural)?.parent_ids().next().map(gix::Id::detach),
            Some(child),
            "the natural descendants follow the review side's single leaf"
        );
        assert!(super::super::rebase::has_marker(
            &repo.find_commit(natural)?.decode()?.into_owned()?
        ));
        assert!(
            repo.try_find_reference(started.reference.as_ref())?.is_none(),
            "finishing removes the review resource"
        );
        assert!(
            repo.try_find_reference(stash_ref.as_ref())?.is_none(),
            "finishing also removes saved review worktree state"
        );
        Ok(())
    }

    #[test]
    fn a_changed_review_and_its_successors_are_spliced_before_target_successors() -> gix_testtools::Result {
        let fixture = gix_testtools::tempfile::tempdir()?;
        run(fixture.path(), &["init", "-q", "-b", "main"])?;
        run(fixture.path(), &["config", "user.name", "reviewer"])?;
        run(fixture.path(), &["config", "user.email", "reviewer@example.com"])?;
        run(
            fixture.path(),
            &["config", "gitoxide.commit.authorDate", "2001-01-01T00:00:00 +0000"],
        )?;
        run(
            fixture.path(),
            &["config", "gitoxide.commit.committerDate", "2001-01-01T00:00:00 +0000"],
        )?;
        std::fs::write(fixture.path().join("file"), "base\n")?;
        run(fixture.path(), &["add", "file"])?;
        run(
            fixture.path(),
            &["-c", "commit.gpgSign=false", "commit", "-q", "-m", "base"],
        )?;
        let base = ObjectId::from_hex(run(fixture.path(), &["rev-parse", "HEAD"])?.trim())?;
        std::fs::write(fixture.path().join("file"), "B\n")?;
        run(fixture.path(), &["-c", "commit.gpgSign=false", "commit", "-qam", "B"])?;
        let reviewed = ObjectId::from_hex(run(fixture.path(), &["rev-parse", "HEAD"])?.trim())?;
        std::fs::write(fixture.path().join("successor"), "A\n")?;
        run(fixture.path(), &["add", "successor"])?;
        run(
            fixture.path(),
            &["-c", "commit.gpgSign=false", "commit", "-q", "-m", "A"],
        )?;
        let old_successor = ObjectId::from_hex(run(fixture.path(), &["rev-parse", "HEAD"])?.trim())?;

        let open = || {
            crate::test_repository::open_with(
                fixture.path(),
                ["user.name=reviewer", "user.email=reviewer@example.com"],
            )
        };
        let repo = open()?;
        let graph = super::super::loaded_graph(&repo)?;
        drop(repo);
        start(fixture.path(), false, &graph, reviewed, base)?;

        std::fs::write(fixture.path().join("reviewed"), "new review change\n")?;
        run(fixture.path(), &["add", "file", "reviewed"])?;
        let repo = open()?;
        let graph = super::super::loaded_graph(&repo)?;
        let review = super::super::head::perform(repo, &graph, super::super::head::Kind::Amend, None)?
            .expect("the staged review change amends the review commit");
        let review_successor = ObjectId::from_hex(
            run(
                fixture.path(),
                &[
                    "-c",
                    "commit.gpgSign=false",
                    "commit-tree",
                    &format!("{review}^{{tree}}"),
                    "-p",
                    &review.to_string(),
                    "-m",
                    "review successor",
                ],
            )?
            .trim(),
        )?;
        run(
            fixture.path(),
            &[
                "update-ref",
                "refs/heads/review-successor",
                &review_successor.to_string(),
            ],
        )?;

        let repo = open()?;
        let graph = super::super::loaded_graph(&repo)?;
        let finished = finish(repo, &graph, review)?;
        let repo = open()?;
        let successor = repo.find_reference("refs/heads/main")?.id().detach();
        assert_ne!(successor, old_successor, "the branch successor is rewritten");
        assert_eq!(
            repo.find_commit(finished)?.parent_ids().next().map(gix::Id::detach),
            Some(reviewed),
            "the finished review is inserted directly after B"
        );
        assert_eq!(
            repo.find_commit(successor)?.parent_ids().next().map(gix::Id::detach),
            Some(repo.find_reference("refs/heads/review-successor")?.id().detach()),
            "A remains the branch tip and follows the review-side history"
        );
        let review_successor = repo.find_reference("refs/heads/review-successor")?.id().detach();
        assert_eq!(
            repo.find_commit(review_successor)?
                .parent_ids()
                .next()
                .map(gix::Id::detach),
            Some(finished),
            "the review successor is inserted before the target history successor"
        );
        assert_eq!(
            repo.find_commit(successor)?.message_raw()?,
            b"A\n".as_bstr(),
            "the target history successor is retained"
        );
        assert!(super::super::rebase::has_marker(
            &repo.find_commit(successor)?.decode()?.into_owned()?
        ));
        insta::assert_snapshot!(
            "changed-review-with-successors",
            gix_testtools::repository::snapshot(fixture.path())?.to_string()
        );
        Ok(())
    }
}
