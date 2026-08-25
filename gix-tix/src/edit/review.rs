use std::{path::Path, process::Command};

use anyhow::{Context, Result};
use gix::{
    ObjectId,
    bstr::{BStr, BString, ByteSlice},
    refs::Target,
};

use crate::{history, open_repository};

const HEADER: &[u8] = b"tix-rebase";
const ONTO: &[u8] = b"onto ";
pub(super) const RETURN_TO: &[u8] = b"tix-review-return-to";

#[derive(Debug)]
pub(crate) struct Started {
    pub commit: ObjectId,
    pub reference: gix::refs::FullName,
    pub checkout_error: Option<anyhow::Error>,
}

pub(crate) struct Finished {
    pub commit: ObjectId,
    pub outcome: super::rebase::Outcome,
}

pub(crate) enum Finish {
    Complete(Finished),
    Conflict(super::rebase::Conflict),
    SelectReturn { tip: ObjectId },
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

pub(super) fn return_to(commit: &gix::objs::Commit) -> Result<Option<gix::refs::FullName>> {
    commit
        .extra_headers
        .iter()
        .find(|(name, _)| name.as_slice() == RETURN_TO)
        .map(|(_, value)| {
            gix::refs::FullName::try_from(value.clone()).context("review commit names an invalid return reference")
        })
        .transpose()
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
        head.referent_name().map(ToOwned::to_owned),
        head.id().map(gix::Id::detach),
    );
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
    let name = next_reference(&repo)?;
    let departure_pin = match restore.1 {
        Some(id) => {
            let target = restore.0.clone().map_or(Target::Object(id), Target::Symbolic);
            let pin_name = return_pin_reference(name.as_bstr())?;
            Some(super::time_travel::create_named_pin(
                &repo,
                pin_name,
                target,
                id,
                "tix review departure",
            )?)
        }
        None => None,
    };

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
    let return_to = departure_pin.as_ref().map(|pin| pin.name.clone());
    if let Some(return_to) = return_to {
        commit
            .extra_headers
            .push((RETURN_TO.into(), return_to.as_bstr().to_owned()));
    }
    let id = repo
        .write_object(&commit)
        .context("could not write review commit")?
        .detach();
    drop(repo);

    let review_name = name.as_bstr().to_str_lossy();
    let create_ref = git(&workdir, ["update-ref", review_name.as_ref(), &tip.to_string()]);
    if let Err(err) = create_ref {
        remove_new_departure_pin(repository_path, bare, departure_pin.as_ref())?;
        return Err(err.context("could not create review reference"));
    }

    let checkout_error = (|| {
        git(&workdir, ["checkout", "--quiet", "--detach", &tip.to_string()])
            .context("could not check out the reviewed commit")?;
        git(
            &workdir,
            ["update-ref", "--no-deref", "HEAD", &id.to_string(), &tip.to_string()],
        )
        .context("could not attach the worktree to the review commit")?;
        git(&workdir, ["read-tree", &id.to_string()]).context("could not reset the index to the review base")
    })()
    .err();
    Ok(Started {
        commit: id,
        reference: name,
        checkout_error,
    })
}

#[tracing::instrument(skip_all, fields(%review))]
#[cfg(test)]
pub(crate) fn finish(
    repo: gix::Repository,
    graph: &history::HistoryGraph,
    review: ObjectId,
    fallback: Option<ObjectId>,
) -> Result<Finish> {
    finish_with_progress(repo, graph, review, fallback, |_| {})
}

pub(crate) fn finish_with_progress(
    repo: gix::Repository,
    graph: &history::HistoryGraph,
    review: ObjectId,
    fallback: Option<ObjectId>,
    report: impl FnMut(super::rebase::Progress),
) -> Result<Finish> {
    let workdir = repo
        .workdir()
        .context("finishing review requires a worktree")?
        .to_owned();
    let head = repo.head_id()?.detach();
    if !graph.is_ancestor(review, head) {
        anyhow::bail!("HEAD must be the review commit or one of its successors before it can be finished");
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
    let legacy_reattach = reference.target().try_name().map(ToOwned::to_owned);
    let tip = reference
        .peel_to_id()
        .context("the review reference does not resolve")?
        .detach();
    let mut delete_refs = resources(&repo, review_ref.clone())?;
    let return_name = return_to(&commit)?.or(legacy_reattach);
    let has_return = return_name.is_some();
    let checkout = if let Some(id) = fallback {
        if !graph.is_ancestor(tip, id) {
            anyhow::bail!("the selected review return commit does not descend from the reviewed commit");
        }
        Some((id, None))
    } else {
        return_name
            .as_ref()
            .map(|name| {
                let Some(mut reference) = repo.try_find_reference(name.as_ref())? else {
                    return Ok(None);
                };
                let checkout_reference = if name.as_bstr().starts_with(history::PIN_PREFIX) {
                    reference.target().try_name().map(ToOwned::to_owned)
                } else {
                    Some(name.clone())
                };
                let id = reference
                    .peel_to_id()
                    .context("the review return reference does not resolve")?
                    .detach();
                if !graph.is_ancestor(tip, id) {
                    anyhow::bail!("the review return reference no longer descends from the reviewed commit");
                }
                Ok(Some((id, checkout_reference)))
            })
            .transpose()?
            .flatten()
    };
    if fallback.is_none() && has_return && checkout.is_none() {
        return Ok(Finish::SelectReturn { tip });
    }
    if let Some(name) = return_name
        .as_ref()
        .filter(|name| name.as_bstr().starts_with(history::REVIEW_PIN_PREFIX))
        && let Some(reference) = repo.try_find_reference(name.as_ref())?
    {
        delete_refs.push((name.clone(), reference.target().into_owned()));
    }
    for (label, id) in [("reviewed commit", tip), ("review base", base)] {
        let endpoint = repo.find_commit(id)?.decode()?.into_owned()?;
        if super::rebase::is_pending(&endpoint) {
            anyhow::bail!("{label} has a pending rebase");
        }
    }
    match super::rebase::finish_review_with_progress(
        &repo,
        graph,
        review,
        tip,
        review_ref,
        delete_refs,
        checkout,
        report,
    )? {
        super::rebase::Perform::Complete(outcome) => {
            let finished = outcome
                .map(review)
                .context("finishing review did not produce a commit")?;
            Ok(Finish::Complete(Finished {
                commit: finished,
                outcome,
            }))
        }
        super::rebase::Perform::Conflict(conflict) => Ok(Finish::Conflict(conflict)),
    }
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
        let return_pin = return_pin_reference(name.as_bstr())?;
        if repo.try_find_reference(name.as_ref())?.is_none() && repo.try_find_reference(return_pin.as_ref())?.is_none()
        {
            return Ok(name);
        }
    }
    unreachable!("u64 review numbers cannot be exhausted")
}

fn return_pin_reference(review: &BStr) -> Result<gix::refs::FullName> {
    let number = history::review_number(review).context("review reference has no numeric identity")?;
    format!(
        "{}{}",
        String::from_utf8_lossy(history::REVIEW_PIN_PREFIX),
        number.to_str_lossy()
    )
    .try_into()
    .context("generated an invalid review return pin reference")
}

fn git<const N: usize>(workdir: &Path, args: [&str; N]) -> Result<()> {
    let output = Command::new("git").arg("-C").arg(workdir).args(args).output()?;
    if output.status.success() {
        Ok(())
    } else {
        anyhow::bail!("{}", String::from_utf8_lossy(&output.stderr).trim())
    }
}

fn remove_new_departure_pin(repository_path: &Path, bare: bool, pin: Option<&history::Pin>) -> Result<()> {
    let Some(pin) = pin else { return Ok(()) };
    let repo =
        open_repository(repository_path, bare, false).context("could not reopen repository to remove review pin")?;
    super::time_travel::delete_pin(&repo, pin).context("could not remove review departure pin")
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
        assert!(started.checkout_error.is_none());

        let repo = crate::test_repository::open(fixture.path())?;
        assert_eq!(repo.head_id()?, started.commit, "HEAD selects the review commit");
        assert_eq!(
            repo.find_commit(started.commit)?.tree_id()?,
            repo.find_commit(base)?.tree_id()?
        );
        let review_ref = repo.find_reference(started.reference.as_ref())?;
        assert_eq!(
            review_ref.id(),
            tip,
            "the review resource remains anchored to the reviewed commit"
        );
        let commit = repo.find_commit(started.commit)?.decode()?.into_owned()?;
        assert_eq!(reference(&commit)?, Some(started.reference.clone()));
        let pins = history::all_pins(&repo)?;
        assert_eq!(pins.len(), 1, "review start preserves its departure with a pin");
        assert_eq!(pins[0].id, tip);
        assert_eq!(
            pins[0].target.try_name().expect("the pin is symbolic"),
            "refs/heads/main",
            "an attached departure uses a symbolic pin"
        );
        assert_eq!(
            return_to(&commit)?.expect("the review records a return"),
            pins[0].name,
            "the review commit records its departure pin"
        );
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

        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=reviewer", "user.email=reviewer@example.com"],
        )?;
        let graph = super::super::loaded_graph(&repo)?;
        let amended = super::super::head::perform(repo, &graph, super::super::head::Kind::Amend, None)?
            .expect("the reviewed worktree delta amends the review commit");
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=reviewer", "user.email=reviewer@example.com"],
        )?;
        let amended_commit = repo.find_commit(amended)?.decode()?.into_owned()?;
        assert!(is_review(&amended_commit));
        assert_eq!(
            return_to(&amended_commit)?.expect("the review records a return"),
            pins[0].name,
            "review amendments preserve the return action"
        );
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
        run(
            fixture.path(),
            &[
                "update-ref",
                "--no-deref",
                "-d",
                pins[0].name.as_bstr().to_str_lossy().as_ref(),
            ],
        )?;
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=reviewer", "user.email=reviewer@example.com"],
        )?;
        let graph = super::super::loaded_graph(&repo)?;
        let Finish::SelectReturn { tip: return_tip } = finish(repo, &graph, amended, None)? else {
            panic!("the deleted return pin requires replacement selection")
        };
        assert_eq!(return_tip, tip, "fallback checkout must descend from the reviewed tip");
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=reviewer", "user.email=reviewer@example.com"],
        )?;
        assert!(
            repo.try_find_reference(started.reference.as_ref())?.is_some(),
            "asking for a replacement leaves the review untouched"
        );
        let Finish::Complete(finished) = finish(repo, &graph, amended, Some(tip))? else {
            panic!("the selected descendant completes the review")
        };
        assert_eq!(
            finished
                .outcome
                .checkout_reference
                .as_ref()
                .map(gix::refs::FullName::as_bstr),
            None,
            "a selected replacement commit is checked out detached"
        );
        assert_eq!(
            finished.outcome.selected,
            Some(finished.commit),
            "the reviewed tip maps to the newly finished review commit"
        );
        super::super::time_travel::checkout_plan(fixture.path(), false, &finished.outcome, &[], false)?;
        let finished = finished.commit;
        let repo = crate::test_repository::open(fixture.path())?;
        assert_eq!(repo.head_id()?, finished);
        assert_eq!(
            repo.head()?.referent_name().map(gix::refs::FullNameRef::as_bstr),
            None,
            "replacement commit selection deliberately leaves HEAD detached"
        );
        assert_eq!(
            repo.find_commit(finished)?.parent_ids().next().map(gix::Id::detach),
            Some(tip)
        );
        assert!(!is_review(&repo.find_commit(finished)?.decode()?.into_owned()?));
        assert!(
            return_to(&repo.find_commit(finished)?.decode()?.into_owned()?)?.is_none(),
            "finished commits contain no review return action"
        );
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
    fn a_blocked_checkout_keeps_the_prepared_review_and_dirty_worktree() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let tip = repo.rev_parse_single("refs/patches/tip")?.detach();
        let base = repo
            .find_commit(tip)?
            .parent_ids()
            .next()
            .expect("the reviewed tip has a parent")
            .detach();
        drop(repo);
        std::fs::write(fixture.path().join("tip"), "departure\n")?;
        run(fixture.path(), &["commit", "-qam", "departure"])?;
        let departure = ObjectId::from_hex(run(fixture.path(), &["rev-parse", "HEAD"])?.trim())?;
        std::fs::write(fixture.path().join("tip"), "dirty\n")?;

        let repo = crate::test_repository::open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        drop(repo);
        let started = start(fixture.path(), false, &graph, tip, base)?;
        let checkout_error = started
            .checkout_error
            .as_ref()
            .expect("conflicting dirt blocks only the checkout");
        assert!(format!("{checkout_error:#}").contains("could not check out the reviewed commit"));

        let repo = crate::test_repository::open(fixture.path())?;
        assert_eq!(repo.head_id()?, departure, "the failed checkout leaves HEAD untouched");
        assert_eq!(repo.find_reference(started.reference.as_ref())?.id(), tip);
        let commit = repo.find_commit(started.commit)?.decode()?.into_owned()?;
        let return_name = return_to(&commit)?.expect("the prepared review records its return pin");
        let mut return_ref = repo.find_reference(return_name.as_ref())?;
        assert_eq!(return_ref.peel_to_id()?.detach(), departure);
        assert_eq!(std::fs::read(fixture.path().join("tip"))?, b"dirty\n");
        assert_eq!(run(fixture.path(), &["show", ":tip"])?, b"departure\n");
        Ok(())
    }

    #[test]
    fn a_changed_review_and_its_successors_are_spliced_before_target_successors() -> gix_testtools::Result {
        let fixture = gix_testtools::tempfile::tempdir()?;
        run(fixture.path(), &["init", "-q", "-b", "main"])?;
        crate::test_repository::disable_autocrlf(fixture.path())?;
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
        let repo = open()?;
        let pins = history::all_pins(&repo)?;
        assert_eq!(pins.len(), 1, "review start preserves the checked-out descendant tip");
        assert_eq!(pins[0].id, old_successor);
        assert_eq!(
            pins[0].target.try_name().expect("the pin is symbolic"),
            "refs/heads/main",
            "an attached review departure remains attached through a symbolic pin"
        );
        drop(repo);

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
        run(fixture.path(), &["checkout", "--detach", &review_successor.to_string()])?;

        let repo = open()?;
        let graph = super::super::loaded_graph(&repo)?;
        let Finish::Complete(finished) = finish(repo, &graph, review, None)? else {
            panic!("the recorded return pin exists")
        };
        super::super::time_travel::checkout_plan(fixture.path(), false, &finished.outcome, &[], false)?;
        let finished = finished.commit;
        let repo = open()?;
        let successor = repo.find_reference("refs/heads/main")?.id().detach();
        assert_eq!(
            repo.head()?.referent_name().expect("HEAD is attached"),
            "refs/heads/main",
            "finishing returns to the branch that contained the reviewed commit"
        );
        assert_eq!(repo.head_id()?, successor);
        assert!(
            history::all_pins(&repo)?.is_empty(),
            "returning consumes the departure pin"
        );
        assert!(
            run(fixture.path(), &["status", "--porcelain=v1", "--untracked-files=all"])?.is_empty(),
            "the restored branch has a matching index and worktree"
        );
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
        assert!(
            !super::super::rebase::is_pending(&repo.find_commit(successor)?.decode()?.into_owned()?),
            "the checked-out review return path is fully replayed"
        );
        crate::test_repository::clear_autocrlf(fixture.path())?;
        insta::assert_snapshot!(
            "changed-review-with-successors",
            gix_testtools::repository::snapshot_portable(fixture.path())?.to_string()
        );
        Ok(())
    }

    #[test]
    fn deleting_a_review_returns_to_the_preserved_departure() -> gix_testtools::Result {
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
        std::fs::write(fixture.path().join("file"), "reviewed\n")?;
        run(
            fixture.path(),
            &["-c", "commit.gpgSign=false", "commit", "-qam", "reviewed"],
        )?;
        let reviewed = ObjectId::from_hex(run(fixture.path(), &["rev-parse", "HEAD"])?.trim())?;
        std::fs::write(fixture.path().join("tip"), "tip\n")?;
        run(fixture.path(), &["add", "tip"])?;
        run(
            fixture.path(),
            &["-c", "commit.gpgSign=false", "commit", "-q", "-m", "tip"],
        )?;
        let tip = ObjectId::from_hex(run(fixture.path(), &["rev-parse", "HEAD"])?.trim())?;

        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=reviewer", "user.email=reviewer@example.com"],
        )?;
        let graph = super::super::loaded_graph(&repo)?;
        drop(repo);
        let started = start(fixture.path(), false, &graph, reviewed, base)?;
        let repo = crate::test_repository::open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let forgotten = super::super::forget::perform(repo, &graph, started.commit)?;
        let return_to = forgotten
            .review_return
            .context("review deletion has a return checkout")?;
        let (returned, _) =
            super::super::time_travel::checkout_review_return(fixture.path(), false, &return_to, &[], false)?;

        let repo = crate::test_repository::open(fixture.path())?;
        assert_eq!(returned, tip);
        assert_eq!(repo.head_id()?, tip);
        assert_eq!(
            repo.head()?.referent_name().expect("HEAD is attached"),
            "refs/heads/main",
            "cancelling reattaches the original branch"
        );
        assert!(history::all_pins(&repo)?.is_empty(), "the return consumes its pin");
        assert!(
            repo.try_find_reference(started.reference.as_ref())?.is_none(),
            "the cancelled review resource is removed"
        );
        assert!(
            run(fixture.path(), &["status", "--porcelain=v1", "--untracked-files=all"])?.is_empty(),
            "cancelling restores the original checkout without review changes"
        );

        drop(repo);
        run(fixture.path(), &["checkout", "-q", "--detach", &tip.to_string()])?;
        let repo = crate::test_repository::open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        drop(repo);
        let started = start(fixture.path(), false, &graph, reviewed, base)?;
        let repo = crate::test_repository::open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let forgotten = super::super::forget::perform(repo, &graph, started.commit)?;
        let return_to = forgotten
            .review_return
            .context("detached review deletion has a return checkout")?;
        super::super::time_travel::checkout_review_return(fixture.path(), false, &return_to, &[], false)?;
        let repo = crate::test_repository::open(fixture.path())?;
        assert!(repo.head()?.is_detached(), "cancelling restores detached HEAD");
        assert_eq!(repo.head_id()?, tip);
        assert!(history::all_pins(&repo)?.is_empty());
        Ok(())
    }

    #[test]
    fn review_return_pin_survives_squashing_the_reviewed_tip() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let open = || crate::test_repository::open(fixture.path());
        let repo = open()?;
        let tip = repo.rev_parse_single("refs/patches/tip")?.detach();
        let middle = repo.rev_parse_single("refs/patches/middle")?.detach();
        let base = repo
            .find_commit(middle)?
            .parent_ids()
            .next()
            .expect("middle has a parent")
            .detach();
        let graph = super::super::loaded_graph(&repo)?;
        drop(repo);

        let started = start(fixture.path(), false, &graph, tip, base)?;
        let repo = open()?;
        let review = repo.find_commit(started.commit)?.decode()?.into_owned()?;
        let return_name = return_to(&review)?.expect("the review records its return pin");
        let graph = super::super::loaded_graph(&repo)?;
        drop(repo);
        super::super::time_travel::perform(fixture.path(), false, middle, &graph, &[started.commit], &[], false)?
            .complete()?;

        let repo = open()?;
        let graph = super::super::loaded_graph(&repo)?;
        let plan = super::super::rebase::squash_plan(&repo, &graph, tip, middle)?;
        let outcome = super::super::rebase::perform_plan(&repo, &graph, plan)?.complete()?;
        let combined = outcome.map(middle).expect("the squash retains its target");
        drop(repo);
        super::super::time_travel::checkout_plan(fixture.path(), false, &outcome, &[], false)?;
        run(
            fixture.path(),
            &["checkout", "--quiet", "--detach", &started.commit.to_string()],
        )?;
        super::super::time_travel::checkout_without_replay(fixture.path(), false, combined, &[], false)?;

        let repo = open()?;
        let mut return_ref = repo.find_reference(return_name.as_ref())?;
        assert_eq!(
            return_ref.peel_to_id()?.detach(),
            combined,
            "checking out a rewritten destination preserves the review's return pin"
        );
        assert_eq!(
            return_name.as_bstr(),
            b"refs/worktree/tix/pins/review/1",
            "review-owned pins have an explicit namespace"
        );
        let graph = super::super::loaded_graph(&repo)?;
        drop(repo);

        super::super::time_travel::perform(
            fixture.path(),
            false,
            started.commit,
            &graph,
            &[started.commit],
            &[],
            false,
        )?
        .complete()?;
        run(fixture.path(), &["add", "--all"])?;
        let repo = open()?;
        let graph = super::super::loaded_graph(&repo)?;
        let review = super::super::head::perform(repo, &graph, super::super::head::Kind::Amend, None)?
            .expect("staged review changes amend the review commit");
        let repo = open()?;
        let graph = super::super::loaded_graph(&repo)?;
        let Finish::Complete(finished) = finish(repo, &graph, review, None)? else {
            panic!("the owned return pin finishes the review without fallback selection")
        };
        super::super::time_travel::checkout_plan(fixture.path(), false, &finished.outcome, &[], false)?;

        let repo = open()?;
        assert_eq!(
            repo.head()?.referent_name().expect("HEAD is attached"),
            "refs/heads/main"
        );
        assert_eq!(repo.head_id()?, finished.commit);
        assert!(
            repo.try_find_reference(return_name.as_ref())?.is_none(),
            "finishing consumes its review-owned return pin"
        );
        Ok(())
    }
}
