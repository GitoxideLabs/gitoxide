//! Worktree removal and associated branch cleanup.

use std::path::PathBuf;

use anyhow::{Context, Result};
use gix::bstr::{BString, ByteSlice};

use super::LogicalHead;

pub(crate) type BranchCleanup = (gix::refs::FullName, gix::refs::Target);

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum BranchCleanupOutcome {
    Deleted(String),
    DeletedWithWarning { branch: String, warning: String },
    Retained { branch: String, reason: String },
}

pub(crate) fn run(
    repository: gix::Repository,
    target: Option<PathBuf>,
    force_count: u8,
    force_delete: bool,
) -> Result<()> {
    let target = match target {
        Some(target) => target,
        None => repository
            .workdir()
            .map(ToOwned::to_owned)
            .context("removing the current worktree requires a linked worktree")?,
    };

    let current_git_dir = repository.git_dir().to_owned();
    let mut common_repository = repository.main_repo().context("could not open the common repository")?;
    let target = common_repository
        .prepare_remove_worktree(&target)
        .context("could not resolve the worktree to remove")?;
    let target_path = target.base().to_owned();
    let inspected = target
        .repository()
        .context("could not open the worktree to inspect its branch")
        .and_then(|repository| {
            let is_current = repository.git_dir() == current_git_dir;
            let cleanup = branch_cleanup_for_repository(&repository, force_delete)?;
            Ok((is_current, cleanup))
        });
    let (is_current, cleanup) = match inspected {
        Ok(inspected) => (inspected.0, Ok(inspected.1)),
        Err(err) => (
            repository.workdir().is_some_and(|workdir| workdir == target_path),
            Err(err),
        ),
    };
    let handoff = handoff_destination(&common_repository)?;
    if is_current {
        std::env::set_current_dir(&handoff)
            .with_context(|| format!("could not leave worktree before removing it for {}", handoff.display()))?;
    }
    drop(repository);

    if let Err(err) = target.remove(force(force_count), gix::progress::Discard) {
        if is_current
            && matches!(&err, gix::worktree::remove::Error::Remove(_))
            && let Err(handoff_err) = emit_handoff(&handoff)
        {
            eprintln!(
                "warning: worktree removal failed after it started, and its shell handoff failed: {handoff_err:#}"
            );
        }
        return Err(err).with_context(|| format!("could not remove worktree {}", target_path.display()));
    }

    match cleanup {
        Ok(Some(cleanup)) => match delete_branch(&mut common_repository, cleanup) {
            BranchCleanupOutcome::Deleted(branch) => {
                eprintln!("removed worktree {} and branch {branch}", target_path.display());
            }
            BranchCleanupOutcome::DeletedWithWarning { branch, warning } => {
                eprintln!(
                    "warning: removed worktree {} and branch {branch}; branch configuration cleanup failed: {warning}",
                    target_path.display()
                );
            }
            BranchCleanupOutcome::Retained { branch, reason } => eprintln!(
                "warning: removed worktree {}; kept branch {branch}: {reason}",
                target_path.display()
            ),
        },
        Ok(None) => eprintln!("removed worktree {}", target_path.display()),
        Err(err) => eprintln!(
            "warning: removed worktree {}; branch cleanup was skipped: {err:#}",
            target_path.display()
        ),
    }
    emit_handoff(&handoff).context("the worktree was removed, but its shell handoff could not be written")?;
    Ok(())
}

fn emit_handoff(path: &std::path::Path) -> Result<()> {
    if !super::write_shell_handoff(path, false)? {
        println!("{}", path.display());
    }
    Ok(())
}

fn force(count: u8) -> gix::worktree::remove::Force {
    match count {
        0 => gix::worktree::remove::Force::Never,
        1 => gix::worktree::remove::Force::DiscardChanges,
        _ => gix::worktree::remove::Force::OverrideLock,
    }
}

fn handoff_destination(repository: &gix::Repository) -> Result<PathBuf> {
    if let Some(workdir) = repository.workdir() {
        return Ok(workdir.to_owned());
    }
    repository
        .common_dir()
        .parent()
        .map(ToOwned::to_owned)
        .context("the bare repository has no parent directory for the worktree handoff")
}

/// Inspect a worktree's logical branch and return the guarded cleanup to perform after removal.
pub(crate) fn branch_cleanup_for_repository(
    repository: &gix::Repository,
    force_delete: bool,
) -> Result<Option<BranchCleanup>> {
    let head = super::logical_head(repository)?;
    let inferred = crate::history::available_hidden_revisions(repository, &[], true)?.0;
    let defaults = if inferred.is_empty() {
        Default::default()
    } else {
        crate::history::referenced_refs(repository, &inferred)?
    };
    branch_cleanup(&head, &defaults, force_delete, |observed_tip, default_tip| {
        if observed_tip == default_tip {
            return Ok(true);
        }
        repository
            .merge_base(observed_tip, default_tip)
            .map(|base| base.as_ref() == observed_tip)
            .context("could not determine whether the worktree branch was merged")
    })
}

pub(crate) fn branch_cleanup<E>(
    head: &LogicalHead,
    defaults: &std::collections::HashMap<BString, gix::refs::Target>,
    force_delete: bool,
    is_ancestor: impl FnOnce(gix::ObjectId, gix::ObjectId) -> std::result::Result<bool, E>,
) -> std::result::Result<Option<BranchCleanup>, E> {
    let (Some(branch), Some(observed_tip)) = (head.branch.as_ref(), head.commit_id) else {
        return Ok(None);
    };
    if defaults
        .keys()
        .any(|name| name.starts_with(b"refs/heads/") && name.as_bstr() == branch.as_bstr())
    {
        return Ok(None);
    }
    let defaults = defaults
        .iter()
        .filter_map(|(name, target)| {
            name.starts_with(b"refs/heads/")
                .then(|| target.try_id().map(|id| (name, id.to_owned())))
                .flatten()
        })
        .collect::<Vec<_>>();
    if !force_delete {
        let [(_, default_tip)] = defaults.as_slice() else {
            return Ok(None);
        };
        if !is_ancestor(observed_tip, *default_tip)? {
            return Ok(None);
        }
    }
    Ok(Some((branch.clone(), gix::refs::Target::Object(observed_tip))))
}

pub(crate) fn delete_branch(
    repository: &mut gix::Repository,
    (branch, expected): BranchCleanup,
) -> BranchCleanupOutcome {
    let short_name = branch.shorten().to_string();
    let result = repository.delete_local_branches_if_unchanged([(branch, expected)]);
    branch_cleanup_outcome(short_name, result)
}

fn branch_cleanup_outcome(
    branch: String,
    result: std::result::Result<(), gix::repository::branch::delete::Error>,
) -> BranchCleanupOutcome {
    match result {
        Ok(()) => BranchCleanupOutcome::Deleted(branch),
        Err(err @ gix::repository::branch::delete::Error::Cleanup { .. }) => BranchCleanupOutcome::DeletedWithWarning {
            branch,
            warning: format!("{:#}", anyhow::Error::new(err)),
        },
        Err(err) => BranchCleanupOutcome::Retained {
            branch,
            reason: format!("{:#}", anyhow::Error::new(err)),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn force_count_maps_to_git_style_safety_levels() {
        use gix::worktree::remove::Force;

        assert_eq!(force(0), Force::Never);
        assert_eq!(force(1), Force::DiscardChanges);
        assert_eq!(force(2), Force::OverrideLock);
        assert_eq!(force(u8::MAX), Force::OverrideLock);
    }

    #[test]
    fn config_cleanup_failure_still_reports_the_branch_as_deleted() {
        let branch = "topic".to_owned();
        let error = gix::repository::branch::delete::Error::Cleanup {
            references: vec!["refs/heads/topic".try_into().expect("valid reference")],
            source: gix::repository::branch::delete::CleanupError::Write(std::io::Error::other("injected")),
        };
        assert!(matches!(
            branch_cleanup_outcome(branch, Err(error)),
            BranchCleanupOutcome::DeletedWithWarning { branch, warning }
                if branch == "topic" && warning.contains("injected")
        ));
    }

    #[test]
    fn force_delete_protects_a_symbolic_default_branch() -> gix_testtools::Result {
        let tip = gix::ObjectId::Sha1([1; 20]);
        let head = LogicalHead {
            branch: Some("refs/heads/main".try_into()?),
            commit_id: Some(tip),
            is_detached: false,
        };
        let defaults = std::collections::HashMap::from([(
            BString::from("refs/heads/main"),
            gix::refs::Target::Symbolic("refs/heads/trunk".try_into()?),
        )]);

        assert!(
            branch_cleanup(
                &head,
                &defaults,
                true,
                |_, _| -> std::result::Result<bool, std::convert::Infallible> {
                    panic!("force-delete does not inspect ancestry")
                }
            )?
            .is_none(),
            "symbolic inferred defaults remain protected"
        );
        Ok(())
    }

    #[test]
    fn removal_cleans_up_only_an_eligible_associated_branch() -> gix_testtools::Result {
        let temp = gix_testtools::tempfile::TempDir::new()?;
        let path = temp.path().join("repo");
        std::fs::create_dir(&path)?;
        git(&path, &["init", "--initial-branch=main"])?;
        git(&path, &["config", "user.name", "Tix Test"])?;
        git(&path, &["config", "user.email", "tix@example.com"])?;
        git(&path, &["config", "commit.gpgSign", "false"])?;
        std::fs::write(path.join("tracked"), "base\n")?;
        git(&path, &["add", "tracked"])?;
        git(&path, &["commit", "-m", "base"])?;
        git(&path, &["branch", "topic"])?;
        let topic_path = temp.path().join("topic");
        git(
            &path,
            &[
                "worktree",
                "add",
                topic_path.to_str().expect("temporary paths are Unicode"),
                "topic",
            ],
        )?;
        for (contents, message) in [("base\none\n", "one"), ("base\none\ntwo\n", "two")] {
            std::fs::write(path.join("tracked"), contents)?;
            git(&path, &["commit", "-am", message])?;
        }
        git(&path, &["remote", "add", "origin", "."])?;
        git(&path, &["update-ref", "refs/remotes/origin/main", "refs/heads/main"])?;
        git(
            &path,
            &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main"],
        )?;
        let repository = crate::test_repository::open(&path)?;
        let main_tip = repository.head_id()?.detach();
        let topic_tip = repository.rev_parse_single("topic")?.detach();
        let defaults = crate::history::referenced_refs(
            &repository,
            &crate::history::available_hidden_revisions(&repository, &[], true)?.0,
        )?;

        assert!(
            branch_cleanup(
                &LogicalHead {
                    branch: Some("refs/heads/main".try_into()?),
                    commit_id: Some(main_tip),
                    is_detached: false,
                },
                &defaults,
                true,
                |_, _| Ok::<_, std::convert::Infallible>(true),
            )?
            .is_none(),
            "force deletion never removes an inferred default branch"
        );
        assert!(
            branch_cleanup(
                &LogicalHead {
                    branch: Some("refs/heads/topic".try_into()?),
                    commit_id: Some(topic_tip),
                    is_detached: false,
                },
                &defaults,
                false,
                |_, _| Ok::<_, std::convert::Infallible>(true),
            )?
            .is_some(),
            "a merged non-default branch is eligible for cleanup"
        );
        assert!(
            branch_cleanup(
                &LogicalHead {
                    branch: Some("refs/heads/topic".try_into()?),
                    commit_id: Some(topic_tip),
                    is_detached: false,
                },
                &defaults,
                false,
                |_, _| Ok::<_, std::convert::Infallible>(false),
            )?
            .is_none(),
            "an unmerged branch is retained without force-delete"
        );
        assert!(
            branch_cleanup(
                &LogicalHead {
                    branch: Some("refs/heads/topic".try_into()?),
                    commit_id: Some(topic_tip),
                    is_detached: false,
                },
                &defaults,
                true,
                |_, _| -> std::result::Result<bool, std::convert::Infallible> {
                    panic!("force-delete does not inspect ancestry")
                },
            )?
            .is_some(),
            "force-delete permits a non-default branch without an ancestry check"
        );

        run(repository, Some(topic_path.clone()), 0, false)?;
        assert!(!topic_path.exists(), "the linked worktree is removed");
        assert!(
            crate::test_repository::open(&path)?
                .try_find_reference("refs/heads/topic")?
                .is_none(),
            "the merged associated branch is removed"
        );
        Ok(())
    }

    fn git(path: &Path, args: &[&str]) -> gix_testtools::Result {
        let output = std::process::Command::new("git")
            .current_dir(path)
            .args(args)
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        Ok(())
    }
}
