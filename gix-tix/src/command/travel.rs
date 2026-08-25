use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
};

use anyhow::{Context, Result};

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub(super) enum To {
    /// Visit the oldest reachable root in HEAD's visible history.
    First,
    /// Visit HEAD's direct visible parent.
    Parent,
    /// Visit HEAD's direct visible child.
    Child,
    /// Visit the reachable leaf in HEAD's visible history.
    Tip,
}

impl To {
    fn name(self) -> &'static str {
        match self {
            To::First => "first",
            To::Parent => "parent",
            To::Child => "child",
            To::Tip => "tip",
        }
    }
}

#[derive(Debug, clap::Args)]
#[command(group(
    clap::ArgGroup::new("destination")
        .required(true)
        .multiple(false)
        .args(["revision", "to"])
))]
pub(super) struct Args {
    /// Check out an encountered replay conflict and write its unmerged index.
    #[arg(long)]
    pub(super) materialize_conflicts: bool,
    /// Revision resolving to the commit to visit.
    #[arg(value_name = "REVSPEC")]
    pub(super) revision: Option<OsString>,
    /// Visit a commit relative to HEAD in the default Tix view.
    #[arg(long, value_enum, value_name = "DESTINATION")]
    pub(super) to: Option<To>,
}

pub(super) fn run(repository: gix::Repository, args: Args) -> Result<()> {
    let head = repository.head().context("could not read HEAD before time-travel")?;
    let head_id = head
        .id()
        .map(gix::Id::detach)
        .context("cannot time-travel from an unborn HEAD")?;
    let detached = head.is_detached();
    drop(head);
    let (selected, resolved_graph) = match (&args.revision, args.to) {
        (Some(revision), None) => super::resolve_commit(&repository, revision, "time-travel destination")?,
        (None, Some(to)) => {
            let hidden = crate::history::available_hidden_revisions(&repository, &[], true)?.0;
            let hidden_tips = crate::history::snapshot(&repository, &[], &hidden, false)?.hidden_tips;
            let graph = crate::edit::loaded_explicit_view_graph(&repository, &[], &hidden)?;
            let selected = relative_destination(&repository, &graph, &hidden_tips, head_id, to)?;
            (selected, Some(graph))
        }
        _ => anyhow::bail!("exactly one time-travel destination is required"),
    };
    if selected == head_id {
        println!("already at {}", crate::change_id::display(&repository, selected, 7)?);
        return Ok(());
    }

    let revisions = vec![OsString::from("HEAD"), OsString::from(selected.to_string())];
    let graph = match resolved_graph {
        Some(graph) => graph,
        None => crate::edit::loaded_explicit_view_graph(&repository, &revisions, &[])?,
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
            ref_changes,
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
            super::record_undo(&repository, "time travel", Ok(ref_changes));
        }
        crate::edit::time_travel::Perform::Conflict(conflict) if args.materialize_conflicts => {
            let (notice, _, ref_rewrites, ref_changes) = conflict.accept()?;
            let repository = crate::open_repository(&repository_path, bare, false)
                .context("could not reopen repository after materializing time-travel")?;
            super::print_ref_rewrites(&repository, &ref_rewrites)?;
            super::record_undo(&repository, "materialize time-travel conflict", Ok(ref_changes));
            anyhow::bail!("{notice}");
        }
        crate::edit::time_travel::Perform::Conflict(_) => {
            anyhow::bail!("time-travel would conflict; retry with --materialize-conflicts to check it out")
        }
    }
    Ok(())
}

fn relative_destination(
    repository: &gix::Repository,
    graph: &crate::history::HistoryGraph,
    hidden_tips: &[gix::ObjectId],
    head: gix::ObjectId,
    to: To,
) -> Result<gix::ObjectId> {
    let order = graph
        .stored_commit_ids()
        .filter(|id| !hidden_tips.iter().any(|hidden| graph.is_ancestor(*id, *hidden)))
        .collect::<Vec<_>>();
    let stored = order.iter().copied().collect::<HashSet<_>>();
    if !stored.contains(&head) {
        anyhow::bail!("HEAD is not present in the default Tix view");
    }

    let candidates = match to {
        To::Parent => visible_parents(graph, head, &stored),
        To::Child => order
            .iter()
            .copied()
            .filter(|id| graph.parents_of(*id).is_some_and(|parents| parents.contains(&head)))
            .collect(),
        To::First => first_candidates(graph, head, &stored, &order),
        To::Tip => terminal_candidates(head, &children_by_parent(graph, &stored, &order), &order),
    };
    match candidates.as_slice() {
        [candidate] => Ok(*candidate),
        [] => anyhow::bail!("HEAD has no {} in the default Tix view", to.name()),
        candidates => {
            let candidates = candidates
                .iter()
                .map(|id| crate::change_id::display_short(repository, *id))
                .collect::<Result<Vec<_>>>()?
                .join("\n  ");
            anyhow::bail!(
                "--to {} is ambiguous; candidates:\n  {candidates}\ntravel to one directly with `tix travel REVSPEC`",
                to.name()
            )
        }
    }
}

fn visible_parents(
    graph: &crate::history::HistoryGraph,
    id: gix::ObjectId,
    stored: &HashSet<gix::ObjectId>,
) -> Vec<gix::ObjectId> {
    graph
        .parents_of(id)
        .unwrap_or_default()
        .into_iter()
        .filter(|parent| stored.contains(parent))
        .collect()
}

fn first_candidates(
    graph: &crate::history::HistoryGraph,
    start: gix::ObjectId,
    stored: &HashSet<gix::ObjectId>,
    order: &[gix::ObjectId],
) -> Vec<gix::ObjectId> {
    let mut pending = vec![start];
    let mut seen = HashSet::new();
    let mut terminals = HashSet::new();
    while let Some(id) = pending.pop() {
        if !seen.insert(id) {
            continue;
        }
        let parents = visible_parents(graph, id, stored);
        if parents.is_empty() {
            terminals.insert(id);
        } else {
            pending.extend(parents);
        }
    }
    order.iter().copied().filter(|id| terminals.contains(id)).collect()
}

fn children_by_parent(
    graph: &crate::history::HistoryGraph,
    stored: &HashSet<gix::ObjectId>,
    order: &[gix::ObjectId],
) -> HashMap<gix::ObjectId, Vec<gix::ObjectId>> {
    let mut children = HashMap::<_, Vec<_>>::new();
    for &id in order {
        for parent in visible_parents(graph, id, stored) {
            children.entry(parent).or_default().push(id);
        }
    }
    children
}

fn terminal_candidates(
    start: gix::ObjectId,
    adjacent: &HashMap<gix::ObjectId, Vec<gix::ObjectId>>,
    order: &[gix::ObjectId],
) -> Vec<gix::ObjectId> {
    let mut pending = vec![start];
    let mut seen = HashSet::new();
    let mut terminals = HashSet::new();
    while let Some(id) = pending.pop() {
        if !seen.insert(id) {
            continue;
        }
        match adjacent.get(&id).filter(|next| !next.is_empty()) {
            Some(next) => pending.extend(next),
            None => {
                terminals.insert(id);
            }
        }
    }
    order.iter().copied().filter(|id| terminals.contains(id)).collect()
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

    fn observable_state(path: &Path) -> gix_testtools::Result<(Vec<u8>, Vec<u8>)> {
        Ok((
            git(path, &["status", "--porcelain=v2", "--branch"])?,
            git(path, &["show-ref"])?,
        ))
    }

    fn args(revision: &str) -> Args {
        Args {
            materialize_conflicts: false,
            revision: Some(revision.into()),
            to: None,
        }
    }

    fn relative_args(to: To) -> Args {
        Args {
            materialize_conflicts: false,
            revision: None,
            to: Some(to),
        }
    }

    #[test]
    fn change_ids_ignore_non_visible_tracking_history() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let path = fixture.path();
        let repository = crate::test_repository::open(path)?;
        let main = repository.rev_parse_single("main")?.detach();
        let original_topic = repository.rev_parse_single("topic")?.detach();
        let change_id = crate::change_id::for_commit(&repository, main)?;
        let mut topic = repository.find_commit(original_topic)?.decode()?.into_owned()?;
        topic
            .extra_headers
            .push((crate::change_id::HEADER.into(), change_id.to_string().into()));
        let topic = repository.write_object(&topic)?.detach();
        drop(repository);

        git(path, &["update-ref", "refs/heads/topic", &topic.to_string()])?;
        git(path, &["config", "remote.origin.url", "https://example.com/repo"])?;
        git(
            path,
            &["config", "remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*"],
        )?;
        git(path, &["config", "branch.topic.remote", "origin"])?;
        git(path, &["config", "branch.topic.merge", "refs/heads/main"])?;
        git(path, &["update-ref", "refs/remotes/origin/main", &main.to_string()])?;
        git(path, &["switch", "-q", "topic"])?;

        let repository = crate::test_repository::open(path)?;
        let graph = crate::edit::loaded_view_graph(&repository)?;
        assert!(
            graph.index(main).is_some(),
            "the tracking commit is loaded for topology"
        );
        assert!(
            !graph.stored_commit_ids().any(|id| id == main),
            "the tracking commit is absent from the visible view"
        );
        assert!(
            graph.stored_commit_ids().any(|id| id == topic),
            "the checked-out topic is visible"
        );
        run(repository, args(&change_id.to_reverse_hex().to_string()))?;
        assert_eq!(
            crate::test_repository::open(path)?.head_id()?,
            topic,
            "the visible change ID resolves to the already checked-out topic"
        );
        Ok(())
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
        assert_eq!(repository.head_id()?, middle);
        assert!(repository.head()?.is_detached());
        let pins = crate::history::all_pins(&repository)?;
        assert_eq!(pins.len(), 1, "leaving the attached tip creates one source pin");
        assert_eq!(
            pins[0].target.try_name().expect("the source pin is symbolic"),
            "refs/heads/main",
            "the source pin follows the departed branch"
        );

        run(repository, args("main"))?;
        let repository = crate::test_repository::open(fixture.path())?;
        assert_eq!(
            repository.head()?.referent_name().expect("HEAD is attached"),
            "refs/heads/main",
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
            repository.head_id()?,
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
    fn relative_targets_follow_the_current_default_view_component() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let path = fixture.path();
        let repository = crate::test_repository::open(path)?;
        let tip = repository.head_id()?.detach();
        let middle = repository.rev_parse_single("HEAD~1")?.detach();
        let root = repository.rev_parse_single("HEAD~2")?.detach();
        drop(repository);

        let orphan = gix::ObjectId::from_hex(git(path, &["commit-tree", "HEAD^{tree}", "-m", "orphan"])?.trim())?;
        let orphan = orphan.to_string();
        git(path, &["update-ref", "refs/worktree/tix/pins/unrelated", &orphan])?;
        let tip_hex = tip.to_string();
        let upstream = gix::ObjectId::from_hex(
            git(path, &["commit-tree", "HEAD^{tree}", "-p", &tip_hex, "-m", "upstream"])?.trim(),
        )?
        .to_string();
        git(path, &["update-ref", "refs/remotes/origin/main", &upstream])?;
        git(path, &["config", "branch.main.remote", "origin"])?;
        git(path, &["config", "branch.main.merge", "refs/heads/main"])?;

        run(crate::test_repository::open(path)?, relative_args(To::Tip))?;
        assert_eq!(
            crate::test_repository::open(path)?.head_id()?,
            tip,
            "tip travel at the attached tip is a no-op"
        );
        let before = observable_state(path)?;
        let err = run(crate::test_repository::open(path)?, relative_args(To::Child))
            .expect_err("the visible tip has no child");
        assert!(format!("{err:#}").contains("no child"));
        assert_eq!(
            observable_state(path)?,
            before,
            "a missing child leaves the repository unchanged"
        );

        run(crate::test_repository::open(path)?, relative_args(To::Parent))?;
        assert_eq!(
            crate::test_repository::open(path)?.head_id()?,
            middle,
            "parent travels down by one commit"
        );
        run(crate::test_repository::open(path)?, relative_args(To::First))?;
        assert_eq!(
            crate::test_repository::open(path)?.head_id()?,
            root,
            "first reaches this component's oldest commit"
        );
        run(crate::test_repository::open(path)?, relative_args(To::First))?;
        assert_eq!(
            crate::test_repository::open(path)?.head_id()?,
            root,
            "first travel at the oldest commit is a no-op"
        );

        let before = observable_state(path)?;
        let err = run(crate::test_repository::open(path)?, relative_args(To::Parent))
            .expect_err("the visible root has no parent");
        assert!(format!("{err:#}").contains("no parent"));
        assert_eq!(
            observable_state(path)?,
            before,
            "a missing parent leaves the repository unchanged"
        );

        run(crate::test_repository::open(path)?, relative_args(To::Child))?;
        assert_eq!(
            crate::test_repository::open(path)?.head_id()?,
            middle,
            "child travels up by one commit"
        );
        run(crate::test_repository::open(path)?, relative_args(To::Tip))?;
        let repository = crate::test_repository::open(path)?;
        assert_eq!(repository.head_id()?, tip, "tip reaches this component's leaf");
        assert_eq!(
            repository.head()?.referent_name().expect("HEAD is attached"),
            "refs/heads/main",
            "travelling to the branch tip reattaches HEAD"
        );
        Ok(())
    }

    #[test]
    fn first_stops_above_the_inferred_hidden_base() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let path = fixture.path();
        git(path, &["config", "remote.origin.url", "."])?;
        git(
            path,
            &["config", "remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*"],
        )?;
        git(path, &["update-ref", "refs/remotes/origin/main", "main"])?;
        git(
            path,
            &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main"],
        )?;
        git(path, &["checkout", "-q", "-b", "topic"])?;
        git(path, &["commit", "-q", "--allow-empty", "-m", "first topic commit"])?;
        let first = crate::test_repository::open(path)?.head_id()?.detach();
        git(path, &["commit", "-q", "--allow-empty", "-m", "topic tip"])?;

        run(crate::test_repository::open(path)?, relative_args(To::First))?;

        assert_eq!(
            crate::test_repository::open(path)?.head_id()?,
            first,
            "first selects the oldest commit displayed above the inferred base"
        );
        Ok(())
    }

    #[test]
    fn relative_target_ambiguity_lists_every_candidate() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let path = fixture.path();
        let repository = crate::test_repository::open(path)?;
        let old_tip = repository.head_id()?.detach();
        let root = repository.rev_parse_single("HEAD~2")?.detach();
        drop(repository);

        let orphan = gix::ObjectId::from_hex(git(path, &["commit-tree", "HEAD^{tree}", "-m", "orphan"])?.trim())?;
        let old_tip_hex = old_tip.to_string();
        let orphan_hex = orphan.to_string();
        let merge = gix::ObjectId::from_hex(
            git(
                path,
                &[
                    "commit-tree",
                    "HEAD^{tree}",
                    "-p",
                    &old_tip_hex,
                    "-p",
                    &orphan_hex,
                    "-m",
                    "merge",
                ],
            )?
            .trim(),
        )?;
        let merge_hex = merge.to_string();
        let left = gix::ObjectId::from_hex(
            git(path, &["commit-tree", "HEAD^{tree}", "-p", &merge_hex, "-m", "left"])?.trim(),
        )?;
        let right = gix::ObjectId::from_hex(
            git(path, &["commit-tree", "HEAD^{tree}", "-p", &merge_hex, "-m", "right"])?.trim(),
        )?;
        git(path, &["update-ref", "refs/heads/main", &merge_hex, &old_tip_hex])?;
        let left_hex = left.to_string();
        let right_hex = right.to_string();
        git(path, &["update-ref", "refs/worktree/tix/pins/left", &left_hex])?;
        git(path, &["update-ref", "refs/worktree/tix/pins/right", &right_hex])?;

        let repository = crate::test_repository::open(path)?;
        let cases = [
            (To::First, vec![root, orphan]),
            (To::Parent, vec![old_tip, orphan]),
            (To::Child, vec![left, right]),
            (To::Tip, vec![left, right]),
        ];
        let expected = cases
            .iter()
            .map(|(to, candidates)| {
                Ok::<_, anyhow::Error>((
                    *to,
                    candidates
                        .iter()
                        .map(|id| crate::change_id::display_short(&repository, *id))
                        .collect::<Result<Vec<_>>>()?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        drop(repository);
        let before = observable_state(path)?;
        for (to, candidates) in expected {
            let err = run(crate::test_repository::open(path)?, relative_args(to))
                .expect_err("multiple relative destinations are ambiguous");
            let message = format!("{err:#}");
            for candidate in candidates {
                assert!(
                    message.contains(&candidate),
                    "{to:?} ambiguity lists candidate {candidate}: {message}"
                );
            }
            assert!(
                message.contains("tix travel REVSPEC"),
                "ambiguity suggests direct travel: {message}"
            );
            assert_eq!(
                observable_state(path)?,
                before,
                "ambiguity leaves the repository unchanged"
            );
        }
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
        let root = repository.rev_parse_single("HEAD~3")?.detach();
        git(path, &["checkout", "-q", "--detach", &root.to_string()])?;
        let graph = crate::edit::loaded_graph(&repository)?;
        crate::edit::rebase::perform(
            &repository,
            &graph,
            crate::edit::rebase::Edit::Remove { target: middle },
            crate::edit::rebase::Signature::RedoIfNeeded,
            crate::edit::rebase::Tree::LeaveAsIsAndMark,
        )?
        .complete()?;
        let tip = repository.find_reference("refs/heads/main")?.id().detach();
        drop(repository);

        git(path, &["checkout", "-q", "main"])?;
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
                revision: Some(tip.to_string().into()),
                to: None,
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
