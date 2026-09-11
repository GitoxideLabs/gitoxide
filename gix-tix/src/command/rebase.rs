use std::{
    ffi::{OsStr, OsString},
    io::{IsTerminal, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

use anyhow::{Context, Result};
use gix::ObjectId;

use crate::{
    app::App,
    edit::{self, rebase, todo},
    history::{self, Authors, Decorations, Event, HistoryGraph},
};

#[derive(Debug, clap::Subcommand)]
pub(super) enum Command {
    /// Produce a self-contained rebase todo, or edit and apply it immediately.
    Todo(Todo),
    /// Apply a self-contained rebase todo from FILE or standard input.
    #[command(
        after_long_help = "Conflicts change nothing by default. To opt in, write the continuation only when needed:\n  tix rebase apply --materialize-conflicts todo.continue.md todo.md\nResolve the index, then run:\n  tix rebase apply todo.continue.md\nUse --materialize-conflicts=- to write a continuation to non-terminal stdout."
    )]
    Apply(Apply),
}

#[derive(Debug, clap::Args)]
#[command(
    after_long_help = "Without --edit-and-apply, the todo is written to stdout. With it, Git's normal editor selection is used; GIT_EDITOR=<command> overrides it.\n\nExamples:\n  tix rebase todo -x main topic >todo.md\n  ${GIT_EDITOR:-editor} todo.md\n  tix rebase apply todo.md\n  tix rebase todo --edit-and-apply -x main topic\n  tix rebase todo --edit-and-apply --materialize-conflicts todo.continue.md -x main topic"
)]
pub(super) struct Todo {
    /// Hide this revision and derive the editable fork point from it.
    #[arg(short = 'x', long, value_name = "REVSPEC")]
    hide: Vec<OsString>,
    /// Do not infer hidden local branches from remote HEADs.
    #[arg(long)]
    no_auto_hide: bool,
    /// Rebase the derived scope onto this commit instead of its fork point.
    #[arg(long, value_name = "REV", conflicts_with = "update_base")]
    onto: Option<OsString>,
    /// Rebase onto the newer hidden local branch tip associated with the fork point.
    #[arg(long)]
    update_base: bool,
    /// Open the todo in Git's editor and apply it after the editor exits.
    #[arg(long)]
    edit_and_apply: bool,
    /// On conflict, materialize it and write a continuation todo to FILE, or stdout if omitted or '-'.
    #[arg(
        long,
        value_name = "CONTINUE",
        num_args = 0..=1,
        default_missing_value = "-",
        requires = "edit_and_apply"
    )]
    materialize_conflicts: Option<PathBuf>,
    /// Visible traversal tips, or HEAD if omitted.
    #[arg(value_name = "TIP")]
    tips: Vec<OsString>,
}

#[derive(Debug, clap::Args)]
pub(super) struct Apply {
    /// On conflict, materialize it and write a continuation todo to FILE, or stdout if omitted or '-'.
    #[arg(long, value_name = "CONTINUE", num_args = 0..=1, default_missing_value = "-")]
    pub(super) materialize_conflicts: Option<PathBuf>,
    /// Todo file to apply; omit or use '-' to read standard input.
    #[arg(value_name = "FILE")]
    pub(super) file: Option<PathBuf>,
}

pub(super) fn run(repo: gix::Repository, command: Command) -> Result<()> {
    match command {
        Command::Todo(args) => todo(repo, args),
        Command::Apply(args) => apply(repo, args),
    }
}

fn todo(repo: gix::Repository, args: Todo) -> Result<()> {
    let prepared = prepare(&repo, &args)?;
    if !args.edit_and_apply {
        std::io::stdout()
            .write_all(&prepared.document)
            .context("could not write the rebase todo")?;
        return Ok(());
    }

    let editor = repo
        .editor_command()
        .context("could not prepare Git editor")?
        .context("no Git editor is available")?;
    let edited = edit::edit_document_without_terminal(
        editor,
        &prepared.document,
        &format!("tix-rebase-{}.md", std::process::id()),
    )?
    .unwrap_or(prepared.document);
    apply_document(repo, &edited, args.materialize_conflicts.as_deref())
}

fn prepare(repo: &gix::Repository, args: &Todo) -> Result<todo::Prepared> {
    let (hide, unavailable) = history::available_hidden_revisions(repo, &args.hide, !args.no_auto_hide)?;
    if hide.is_empty() {
        anyhow::bail!(
            "rebase todo requires at least one -x/--hide revision when no remote HEAD maps to a local branch"
        );
    }
    for (revision, err) in unavailable {
        eprintln!(
            "warning: ignoring unavailable hidden revision {}: {err}",
            revision.to_string_lossy()
        );
    }
    let refs = history::snapshot(repo, &args.tips, &hide, false)?;

    let authors = gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(Authors::default()));
    let mut app = App::new(usize::MAX);
    let mut decorations = Decorations::default();
    let mut graph = None;
    history::load(
        repo,
        &args.tips,
        &hide,
        false,
        &authors,
        &AtomicBool::new(false),
        |event| {
            match event {
                Event::Decorations(value) => decorations = value,
                Event::Commits(commits) => app.extend_commits(commits),
                Event::HiddenCommits(commits) => app.extend_hidden_commits(commits),
                Event::Complete(value) => graph = Some(value),
                Event::VisibleComplete | Event::Cancelled => {}
            }
            true
        },
    )?;
    let graph = graph.context("history traversal did not produce a graph")?;
    app.set_auto_merges(&graph, &decorations, &refs.pins);
    crate::update_hidden_branch_updates(&mut app, Some(&graph), &refs);
    let mut candidates = app.hidden_rebase_candidates();
    if candidates.len() != 1 {
        if candidates.is_empty() {
            anyhow::bail!("the hidden and visible revisions have no editable fork point");
        }
        candidates.sort_by_key(|(id, _)| *id);
        anyhow::bail!(
            "the revisions have multiple editable fork points: {}",
            candidates
                .iter()
                .map(|(id, _)| id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let (base, scope) = candidates.pop().context("one rebase candidate was expected")?;
    let (onto, onto_kind) = if args.update_base {
        let onto = app
            .hidden_branch_update(base)
            .context("--update-base found no newer hidden local branch tip for the derived base")?;
        (onto, todo::OntoKind::UpdatedBase)
    } else {
        (
            args.onto
                .as_deref()
                .map(|revision| resolve_commit(repo, revision, "onto revision"))
                .transpose()?
                .unwrap_or(base),
            todo::OntoKind::Onto,
        )
    };

    let commits = crate::load_rebase_todo_commits(repo, &mut app, &authors, &scope)?;
    todo::prepare(repo, base, onto, &commits, &refs.view_tips, onto_kind, true)
}

fn resolve_commit(repo: &gix::Repository, revision: &OsStr, description: &str) -> Result<ObjectId> {
    let revision =
        gix::path::os_str_into_bstr(revision).with_context(|| format!("{description} is not valid UTF-8"))?;
    crate::history::resolve_revision(repo, revision)
        .with_context(|| format!("could not resolve {description}"))
        .map(|(id, _reference)| id)
}

fn apply(repo: gix::Repository, args: Apply) -> Result<()> {
    let mut document = Vec::new();
    match args.file.as_deref() {
        None => {
            std::io::stdin()
                .read_to_end(&mut document)
                .context("could not read the rebase todo from standard input")?;
        }
        Some(path) if path == Path::new("-") => {
            std::io::stdin()
                .read_to_end(&mut document)
                .context("could not read the rebase todo from standard input")?;
        }
        Some(path) => {
            document =
                std::fs::read(path).with_context(|| format!("could not read rebase todo at {}", path.display()))?;
        }
    }
    apply_document(repo, &document, args.materialize_conflicts.as_deref())
}

fn apply_document(repo: gix::Repository, document: &[u8], materialize_conflicts: Option<&Path>) -> Result<()> {
    let Some(parsed) = todo::parse(&repo, document)? else {
        println!("no rebase performed: the todo was cancelled");
        return Ok(());
    };
    let view = edit::loaded_view_graph(&repo)?;
    let mut scope = view.edit_commit_ids();
    scope.extend_from_slice(&parsed.plan.scope);
    let mut graph = HistoryGraph::for_commits(&repo, &scope)?;
    graph.bounded_history = view.bounded_history;
    let tips = parsed.tips;
    let revisions = mapped_revisions(&tips, Some);
    match rebase::perform_plan_with_progress(
        &repo,
        &graph,
        parsed.plan,
        rebase::CheckoutOptions {
            revisions: &revisions,
            ..Default::default()
        },
        |_| {},
    )? {
        rebase::PlanPerform::Complete(outcome) => {
            let notice = outcome.notice.as_deref().unwrap_or("rebased history");
            if let Some(selected) = outcome.selected {
                eprintln!("{}", super::notice_with_change_id(&repo, notice, selected)?);
            } else {
                eprintln!("{notice}");
            }
            super::print_ref_rewrites(&repo, &outcome.ref_rewrites)?;
            super::record_undo(&repo, "rebase history", Ok(outcome.ref_changes));
            Ok(())
        }
        rebase::PlanPerform::Conflict(conflict) => {
            handle_plan_conflict(&repo, conflict, materialize_conflicts, &tips, "rebase")
        }
    }
}

pub(super) fn handle_plan_conflict(
    repo: &gix::Repository,
    mut conflict: rebase::PlanConflict,
    materialize_conflicts: Option<&Path>,
    tips: &[ObjectId],
    operation: &str,
) -> Result<()> {
    let Some(destination) = materialize_conflicts else {
        anyhow::bail!(
            "{operation} aborted without changes: conflict while applying {}; pass --materialize-conflicts to opt in",
            conflict.original().to_hex_with_len(7)
        );
    };
    if destination == Path::new("-") && std::io::stdout().is_terminal() {
        anyhow::bail!(
            "{operation} aborted without changes: refusing to materialize a conflict without a continuation output file"
        );
    }
    conflict.persist_objects()?;
    let plan = conflict.continuation_plan();
    let mapped_tips = tips.iter().filter_map(|id| conflict.map(*id)).collect();
    let continuation = todo::prepare_continuation(conflict.repository(), &plan, mapped_tips, true)?.document;
    let revisions = mapped_revisions(tips, |id| conflict.map(id));
    if destination == Path::new("-") {
        let mut stdout = std::io::stdout().lock();
        stdout
            .write_all(&continuation)
            .and_then(|_| stdout.flush())
            .context("could not write the continuation rebase todo")?;
    } else {
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)
            .with_context(|| format!("could not create continuation rebase todo at {}", destination.display()))?;
        output
            .write_all(&continuation)
            .with_context(|| format!("could not write continuation rebase todo at {}", destination.display()))?;
    }
    let materialized = edit::time_travel::materialize_plan_conflict_reporting(conflict, &revisions, false);
    let (notice, _, ref_rewrites, ref_changes) = match materialized {
        Ok(materialized) => materialized,
        Err(err) => {
            if destination != Path::new("-") {
                let _ = std::fs::remove_file(destination);
            }
            return Err(err);
        }
    };
    if destination == Path::new("-") {
        for line in super::ref_rewrite_lines(repo, &ref_rewrites)? {
            eprintln!("{line}");
        }
    } else {
        super::print_ref_rewrites(repo, &ref_rewrites)?;
    }
    super::record_undo(repo, "materialize rebase conflict", Ok(ref_changes));
    eprintln!("{notice}; continue with `tix rebase apply {}`", destination.display());
    anyhow::bail!("{operation} stopped at a materialized conflict")
}

fn mapped_revisions(tips: &[ObjectId], mut map: impl FnMut(ObjectId) -> Option<ObjectId>) -> Vec<OsString> {
    tips.iter()
        .filter_map(|id| map(*id))
        .map(|id| OsString::from(id.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rebase::FoldMessage;
    use std::process::Command;

    fn repository() -> gix_testtools::Result<(gix_testtools::tempfile::TempDir, gix::Repository)> {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["core.abbrev=7", "user.name=todo author", "user.email=todo@example.com"],
        )?;
        Ok((fixture, repo))
    }

    fn git(path: &Path, args: &[&str]) -> gix_testtools::Result<Vec<u8>> {
        let output = Command::new("git")
            .current_dir(path)
            .env("GIT_EDITOR", ":")
            .env("GIT_SEQUENCE_EDITOR", "cat")
            .args(args)
            .output()?;
        if !output.status.success() {
            return Err(format!("git {args:?} failed: {}", String::from_utf8_lossy(&output.stderr)).into());
        }
        Ok(output.stdout)
    }

    fn autosquash_args(base_commit_id: ObjectId) -> Todo {
        Todo {
            hide: vec![base_commit_id.to_string().into()],
            no_auto_hide: true,
            onto: None,
            update_base: false,
            edit_and_apply: false,
            materialize_conflicts: None,
            tips: Vec::new(),
        }
    }

    #[test]
    fn autosquash_applies_all_markers_and_keeps_the_branch_at_the_descendant() -> gix_testtools::Result {
        for (message, changes_tree, mode) in [
            ("fixup! middle\n\nDiscarded commentary", true, FoldMessage::Discard),
            ("squash! middle\n\nAdditional explanation", true, FoldMessage::Append),
            (
                "amend! middle\n\nReplacement title\n\nReplacement body",
                true,
                FoldMessage::Replace,
            ),
            ("amend! middle\n\nMessage-only replacement", false, FoldMessage::Replace),
        ] {
            let (fixture, repo) = repository()?;
            let base_commit_id = repo.rev_parse_single("HEAD~2")?.detach();
            let target_commit_id = repo.rev_parse_single("HEAD~1")?.detach();
            if changes_tree {
                std::fs::write(fixture.path().join("middle"), b"corrected middle\n")?;
                git(fixture.path(), &["add", "middle"])?;
            }
            git(
                fixture.path(),
                &[
                    "commit",
                    "-q",
                    "--allow-empty",
                    "--author=Correction Author <correction@example.com>",
                    "-m",
                    message,
                ],
            )?;
            let source_commit_id = repo.head_id()?.detach();

            // Git supplies the expected rewritten trees, target authorship, and fixup/amend messages.
            git(
                fixture.path(),
                &[
                    "-c",
                    "rebase.updateRefs=false",
                    "rebase",
                    "--autosquash",
                    &base_commit_id.to_string(),
                ],
            )?;
            let oracle = crate::test_repository::open(fixture.path())?;
            let expected_head = oracle.head_commit()?.decode()?.into_owned()?;
            let expected_target = oracle
                .find_commit(oracle.rev_parse_single("HEAD~1")?)?
                .decode()?
                .into_owned()?;
            git(fixture.path(), &["reset", "--hard", &source_commit_id.to_string()])?;

            let mut args = autosquash_args(base_commit_id);
            let repo = crate::test_repository::open_with(fixture.path(), ["core.editor=false"])?;
            let prepared = prepare(&repo, &args)?;
            assert!(
                prepared.apply_unchanged,
                "autosquash makes the generated todo actionable"
            );
            let parsed = todo::parse(&repo, &prepared.document)?.context("the generated todo is actionable")?;
            let target = parsed
                .plan
                .steps
                .iter()
                .find(|step| step.commit == rebase::PlanCommit::Pick(target_commit_id))
                .context("the target remains a pick")?;
            assert_eq!(
                target.squash,
                [rebase::PlanFold {
                    commit_id: source_commit_id,
                    message: mode,
                }],
                "the message marker selects its fold behavior"
            );
            if changes_tree {
                apply_document(repo, &prepared.document, None)?;
            } else {
                args.edit_and_apply = true;
                todo(crate::test_repository::open(fixture.path())?, args)?;
            }

            let actual = crate::test_repository::open(fixture.path())?;
            let actual_head = actual.head_commit()?.decode()?.into_owned()?;
            let actual_target = actual
                .find_commit(actual.rev_parse_single("HEAD~1")?)?
                .decode()?
                .into_owned()?;
            assert_eq!(actual_head.tree, expected_head.tree, "the final tree agrees with Git");
            assert_eq!(
                actual_target.tree, expected_target.tree,
                "the folded target tree agrees with Git"
            );
            assert_eq!(
                actual_target.author, expected_target.author,
                "folding retains the target author and date"
            );
            assert_eq!(
                actual_head.message,
                b"tip\n".as_slice(),
                "the descendant remains the checked-out tip"
            );
            assert_eq!(
                actual_target.parents.as_slice(),
                [base_commit_id],
                "the correction leaves no extra commit"
            );
            assert_eq!(
                git(fixture.path(), &["symbolic-ref", "HEAD"])?,
                b"refs/heads/main\n",
                "the attached branch follows the surviving descendant when its former tip is folded backward"
            );
            if mode == FoldMessage::Append {
                let actual_message = actual_target.message.to_string();
                assert!(
                    actual_message.contains("squash! middle")
                        && actual_message.contains("Additional explanation")
                        && actual_message.contains("Co-authored-by: Correction Author <correction@example.com>"),
                    "squash retains Tix's source sections and co-author credit"
                );
            } else {
                assert_eq!(
                    actual_target.message, expected_target.message,
                    "fixup and amend messages agree with Git"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn autosquash_hash_targets_keep_gits_fold_order() -> gix_testtools::Result {
        let (fixture, repo) = repository()?;
        let base_commit_id = repo.rev_parse_single("HEAD~2")?.detach();
        let mut source_commit_ids = Vec::<ObjectId>::new();
        for index in 0..4 {
            let path = format!("correction-{index}");
            std::fs::write(fixture.path().join(&path), format!("correction {index}\n"))?;
            git(fixture.path(), &["add", &path])?;
            let target = if index < 2 {
                "middle".to_owned()
            } else {
                source_commit_ids[0].to_string()
            };
            git(fixture.path(), &["commit", "-qm", &format!("fixup! {target}")])?;
            source_commit_ids.push(repo.head_id()?.detach());
        }
        let original_head_commit_id = repo.head_id()?.detach();
        let oracle_todo = String::from_utf8(git(
            fixture.path(),
            &[
                "-c",
                "core.abbrev=40",
                "-c",
                "rebase.updateRefs=false",
                "rebase",
                "-i",
                "--autosquash",
                &base_commit_id.to_string(),
            ],
        )?)?;
        let expected_order = oracle_todo
            .lines()
            .filter_map(|line| {
                line.strip_prefix("fixup ")
                    .and_then(|line| line.split_whitespace().next())
            })
            .map(|id| ObjectId::from_hex(id.as_bytes()))
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(
            expected_order,
            [
                source_commit_ids[0],
                source_commit_ids[2],
                source_commit_ids[3],
                source_commit_ids[1]
            ],
            "Git inserts hash-targeted fixups after their target, ahead of later root fixups"
        );
        let expected_tree_id = repo.head_commit()?.tree_id()?.detach();
        git(
            fixture.path(),
            &["reset", "--hard", &original_head_commit_id.to_string()],
        )?;
        let prepared = prepare(&repo, &autosquash_args(base_commit_id))?;
        let parsed = todo::parse(&repo, &prepared.document)?.context("autosquash is actionable")?;
        assert_eq!(
            parsed
                .plan
                .steps
                .iter()
                .flat_map(|step| step.squash.iter().map(|fold| fold.commit_id))
                .collect::<Vec<_>>(),
            expected_order,
            "the generated Tix todo preserves Git's fold order"
        );
        apply_document(repo, &prepared.document, None)?;
        assert_eq!(
            crate::test_repository::open(fixture.path())?.head_commit()?.tree_id()?,
            expected_tree_id,
            "all nested corrections reach the final tree"
        );
        Ok(())
    }

    #[test]
    fn autosquash_continuations_preserve_remaining_message_modes() -> gix_testtools::Result {
        for (message, mode) in [
            ("fixup! middle\n\nDiscarded later message", FoldMessage::Discard),
            ("squash! middle\n\nAppended later message", FoldMessage::Append),
            ("amend! middle\n\nReplaced later message", FoldMessage::Replace),
        ] {
            let fixture = gix_testtools::scripted_fixture_writable("rebase_conflict.sh")?;
            let repo = crate::test_repository::open(fixture.path())?;
            let base_commit_id = repo.rev_parse_single("HEAD~2")?.detach();
            // The first correction depends on the intervening tip's same-line edit and must conflict when moved.
            std::fs::write(fixture.path().join("file"), b"source\n")?;
            git(fixture.path(), &["commit", "-qam", "fixup! middle"])?;
            std::fs::write(fixture.path().join("pending"), b"pending correction\n")?;
            git(fixture.path(), &["add", "pending"])?;
            git(fixture.path(), &["commit", "-qm", message])?;
            let prepared = prepare(&repo, &autosquash_args(base_commit_id))?;
            let before = gix_testtools::repository::snapshot(fixture.path())?;
            let err = apply_document(repo.clone(), &prepared.document, None)
                .expect_err("the non-adjacent correction conflicts");
            assert!(
                format!("{err:#}").contains("aborted without changes"),
                "conflicts remain opt-in: {err:#}"
            );
            assert_eq!(
                gix_testtools::repository::snapshot(fixture.path())?,
                before,
                "an unmaterialized conflict changes nothing"
            );

            let output_dir = gix_testtools::tempfile::tempdir()?;
            let output = output_dir.path().join("continue.md");
            let err = apply_document(repo, &prepared.document, Some(&output))
                .expect_err("materializing the first correction stops the command");
            assert!(
                format!("{err:#}").contains("materialized conflict"),
                "the continuation is available: {err:#}"
            );
            let continuation = std::fs::read(output)?;
            let repo = crate::test_repository::open(fixture.path())?;
            let parsed = todo::parse(&repo, &continuation)?.context("the continuation is actionable")?;
            assert_eq!(
                parsed
                    .plan
                    .steps
                    .iter()
                    .flat_map(|step| step.squash.iter().map(|fold| fold.message))
                    .collect::<Vec<_>>(),
                [mode],
                "serialization preserves the unapplied fold's message mode"
            );
            // Keep the earlier version when resolving, allowing the original tip to replay without another conflict.
            std::fs::write(fixture.path().join("file"), b"middle\n")?;
            git(fixture.path(), &["add", "file"])?;
            apply_document(repo, &continuation, None)?;
            let actual = crate::test_repository::open(fixture.path())?;
            let target = actual
                .find_commit(actual.rev_parse_single("HEAD~1")?)?
                .decode()?
                .into_owned()?;
            match mode {
                FoldMessage::Discard => assert_eq!(
                    target.message,
                    b"middle\n".as_slice(),
                    "fixup discards the pending message"
                ),
                FoldMessage::Append => assert!(
                    target.message.to_string().contains("Appended later message"),
                    "squash appends the pending message"
                ),
                FoldMessage::Replace => assert_eq!(
                    target.message,
                    b"Replaced later message\n".as_slice(),
                    "amend replaces the pending message"
                ),
            }
            assert_eq!(
                std::fs::read(fixture.path().join("pending"))?,
                b"pending correction\n",
                "continuation applies the remaining patch"
            );
            assert_eq!(
                std::fs::read(fixture.path().join("file"))?,
                b"tip\n",
                "the descendant replays after resolution"
            );
        }
        Ok(())
    }

    #[test]
    fn generates_a_self_contained_todo_from_hidden_and_visible_revisions() -> gix_testtools::Result {
        let (_fixture, repo) = repository()?;
        let prepared = prepare(
            &repo,
            &Todo {
                hide: vec!["HEAD~2".into()],
                no_auto_hide: false,
                onto: None,
                update_base: false,
                edit_and_apply: false,
                materialize_conflicts: None,
                tips: Vec::new(),
            },
        )?;
        let document = String::from_utf8(prepared.document)?;
        assert!(document.contains("<!-- tix-rebase-state-v2"), "state is embedded");
        assert!(document.contains("`@pick "), "HEAD is the generated checkout");
        assert!(
            document.contains("2000-01-02 author middle"),
            "default TUI metadata is present"
        );
        assert!(
            document.contains("2000-01-03 author tip"),
            "the subject is always present"
        );
        Ok(())
    }

    #[test]
    fn requires_a_hidden_revision_at_runtime() -> gix_testtools::Result {
        let (_fixture, repo) = repository()?;
        let err = prepare(
            &repo,
            &Todo {
                hide: Vec::new(),
                no_auto_hide: true,
                onto: None,
                update_base: false,
                edit_and_apply: false,
                materialize_conflicts: None,
                tips: Vec::new(),
            },
        )
        .expect_err("a hidden revision is required");
        assert!(format!("{err:#}").contains("at least one -x/--hide"));
        Ok(())
    }

    #[test]
    fn infers_the_hidden_base_from_a_remote_head() -> gix_testtools::Result {
        let (fixture, _repo) = repository()?;
        for args in [
            &["branch", "base", "HEAD~2"][..],
            &["config", "remote.origin.url", "https://example.com/repo"][..],
            &["config", "remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*"][..],
            &["update-ref", "refs/remotes/origin/base", "refs/heads/base"][..],
            &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/base"][..],
        ] {
            let output = Command::new("git").current_dir(fixture.path()).args(args).output()?;
            assert!(
                output.status.success(),
                "git {args:?} prepares the remote default: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["core.abbrev=7", "user.name=todo author", "user.email=todo@example.com"],
        )?;
        let prepared = prepare(
            &repo,
            &Todo {
                hide: Vec::new(),
                no_auto_hide: false,
                onto: None,
                update_base: false,
                edit_and_apply: false,
                materialize_conflicts: None,
                tips: Vec::new(),
            },
        )?;
        assert!(
            String::from_utf8(prepared.document)?.contains("# Rebase from"),
            "the inferred local default branch provides the rebase base"
        );
        Ok(())
    }

    #[test]
    fn auto_merges_round_trip_as_picks_and_can_be_dropped() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let source_commit_id = repo.head_id()?.detach();
        let graph = edit::loaded_view_graph(&repo)?;
        let operation = edit::auto_merge::perform(
            &repo,
            &graph,
            source_commit_id,
            edit::auto_merge::Change::Add("refs/heads/C".try_into()?),
            rebase::CheckoutOptions::default(),
            |_| {},
        )?;
        operation.result.context("creation prepares a merge")?.complete()?;

        let merge_commit_id = repo.head_id()?.detach();
        let prepared = prepare(
            &repo,
            &Todo {
                hide: vec!["main".into()],
                no_auto_hide: true,
                onto: None,
                update_base: false,
                edit_and_apply: false,
                materialize_conflicts: None,
                tips: Vec::new(),
            },
        )?;
        let parsed = todo::parse(&repo, &prepared.document)?.context("the generated todo is actionable")?;
        assert_eq!(
            parsed
                .plan
                .checkout
                .as_ref()
                .and_then(|checkout| match checkout.target {
                    rebase::PlanParent::Step(index) => Some(&parsed.plan.steps[index].commit),
                    _ => None,
                }),
            Some(&rebase::PlanCommit::Pick(merge_commit_id)),
            "the generated todo keeps checkout at AutoMerge: {}",
            String::from_utf8_lossy(&prepared.document)
        );
        assert!(
            parsed
                .plan
                .steps
                .iter()
                .any(|step| step.commit == rebase::PlanCommit::Pick(merge_commit_id)),
            "AutoMerge uses the ordinary pick syntax"
        );
        apply_document(repo.clone(), &prepared.document, None)?;
        assert_eq!(
            repo.head_id()?,
            merge_commit_id,
            "unchanged inputs keep the generated commit ID"
        );

        let merge_pick = format!("`@pick {}", crate::change_id::display_short(&repo, merge_commit_id)?);
        let source_pick = format!("`pick {}", crate::change_id::display_short(&repo, source_commit_id)?);
        let dropped = String::from_utf8(prepared.document)?
            .lines()
            .filter(|line| !line.starts_with(&merge_pick))
            .map(|line| line.replacen(&source_pick, &source_pick.replacen("`pick", "`@pick", 1), 1))
            .collect::<Vec<_>>()
            .join("\n");
        apply_document(repo.clone(), dropped.as_bytes(), None)?;
        assert!(
            edit::auto_merge::Definition::from_commit(&repo.head_commit()?.decode()?.into_owned()?)?.is_none(),
            "deleting the pick drops the AutoMerge"
        );
        assert_eq!(
            repo.find_reference("refs/heads/A")?.peel_to_commit()?.id,
            source_commit_id,
            "dropping a merge keeps its input ref"
        );
        Ok(())
    }

    #[test]
    fn update_base_uses_the_newer_hidden_local_branch_tip() -> gix_testtools::Result {
        let (fixture, repo) = repository()?;
        drop(repo);
        for args in [
            &["branch", "base", "HEAD~2"][..],
            &["checkout", "-q", "base"][..],
            &[
                "-c",
                "user.name=updated base",
                "-c",
                "user.email=updated@example.com",
                "-c",
                "commit.gpgSign=false",
                "commit",
                "-q",
                "--allow-empty",
                "-m",
                "updated base",
            ][..],
            &["checkout", "-q", "main"][..],
        ] {
            let output = Command::new("git").current_dir(fixture.path()).args(args).output()?;
            assert!(
                output.status.success(),
                "git {args:?} prepares the updated base: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["core.abbrev=7", "user.name=todo author", "user.email=todo@example.com"],
        )?;
        let updated = repo.rev_parse_single("base")?.detach();
        let prepared = prepare(
            &repo,
            &Todo {
                hide: vec!["base".into()],
                no_auto_hide: false,
                onto: None,
                update_base: true,
                edit_and_apply: false,
                materialize_conflicts: None,
                tips: Vec::new(),
            },
        )?;
        let document = String::from_utf8(prepared.document)?;
        assert!(
            prepared.apply_unchanged,
            "moving to the updated base makes an unchanged todo actionable"
        );
        assert!(
            document.contains(&format!(
                "{} (updated-base)",
                crate::change_id::display_short(&repo, updated)?
            )),
            "the TUI-selected hidden tip is labelled as the updated base: {document}"
        );

        let err = prepare(
            &repo,
            &Todo {
                hide: vec!["HEAD~2".into()],
                no_auto_hide: false,
                onto: None,
                update_base: true,
                edit_and_apply: false,
                materialize_conflicts: None,
                tips: Vec::new(),
            },
        )
        .expect_err("a derived revision without a hidden local branch has no update target");
        assert!(format!("{err:#}").contains("no newer hidden local branch tip"));
        Ok(())
    }

    #[test]
    fn materialized_conflicts_emit_an_applicable_continuation_todo() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_conflict.sh")?;
        // This descendant is rewritten into object memory and conflicts after the earlier conflict is resolved.
        std::fs::write(fixture.path().join("file"), b"after\n")?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["add", "file"])
                .status()?
                .success()
        );
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["commit", "-q", "-m", "after"])
                .status()?
                .success()
        );
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=todo author", "user.email=todo@example.com"],
        )?;
        let base = repo.rev_parse_single("HEAD~3")?.detach();
        let middle = repo.rev_parse_single("HEAD~2")?.detach();
        let tip = repo.rev_parse_single("HEAD~1")?.detach();
        let after = repo.head_id()?.detach();
        let prepared = todo::prepare(
            &repo,
            base,
            base,
            &[
                todo::Commit {
                    id: tip,
                    parents: vec![middle],
                    info: "tip".into(),
                },
                todo::Commit {
                    id: middle,
                    parents: vec![base],
                    info: "middle".into(),
                },
                todo::Commit {
                    id: after,
                    parents: vec![tip],
                    info: "after".into(),
                },
            ],
            &[after],
            todo::OntoKind::Onto,
            true,
        )?;
        let generated = std::str::from_utf8(&prepared.document)?;
        let state = &generated[generated
            .find("<!-- tix-rebase-state-v2")
            .expect("generated state is present")..];
        let edited = format!(
            "`@pick {}` after\n`pick {}` tip\n──── fork {} ────\n\n{state}",
            after.to_hex_with_len(7),
            tip.to_hex_with_len(7),
            base.to_hex_with_len(7)
        );
        let output_dir = gix_testtools::tempfile::tempdir()?;
        let output = output_dir.path().join("continue.md");
        let err = apply_document(repo, edited.as_bytes(), Some(&output)).expect_err("the conflict stops the command");
        assert!(
            format!("{err:#}").contains("materialized conflict"),
            "the conflict is materialized after its continuation is written: {err:#}"
        );
        let continuation = std::fs::read(&output)?;
        assert!(
            continuation
                .windows(40)
                .any(|window| window.iter().all(|byte| *byte == b'0')),
            "the conflicting command is represented by the full null object ID"
        );
        let unresolved = Command::new("git")
            .arg("-C")
            .arg(fixture.path())
            .args(["diff", "--name-only", "--diff-filter=U"])
            .output()?;
        assert!(unresolved.status.success());
        assert_eq!(
            unresolved.stdout, b"file\n",
            "materialization writes the unmerged index"
        );
        let materialized = crate::test_repository::open(fixture.path())?;
        let conflict_commit = materialized.head_commit()?;
        assert_eq!(
            conflict_commit.tree_id()?,
            conflict_commit
                .parent_ids()
                .next()
                .expect("a cherry-picked commit has a parent")
                .object()?
                .peel_to_tree()?
                .id,
            "the materialized conflict commit records the ours tree"
        );

        std::fs::write(fixture.path().join("file"), b"resolved first\n")?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["add", "file"])
                .status()?
                .success()
        );
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["commit", "-q", "--amend", "--no-edit"])
                .status()?
                .success(),
            "resolving through the CLI may amend the materialized conflict before continuing"
        );
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=todo author", "user.email=todo@example.com"],
        )?;
        let first_resolution = repo.head_id()?.detach();
        let next_output = output_dir.path().join("continue-again.md");
        let err = apply_document(repo, &continuation, Some(&next_output))
            .expect_err("the descendant conflict stops the continuation");
        assert!(
            format!("{err:#}").contains("materialized conflict"),
            "the second conflict is materialized: {err:#}"
        );
        assert!(
            crate::history::all_pins(&crate::test_repository::open(fixture.path())?)?
                .iter()
                .all(|pin| pin.id != first_resolution),
            "continuing does not pin the superseded CLI resolution"
        );
        std::fs::write(fixture.path().join("file"), b"resolved again\n")?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["add", "file"])
                .status()?
                .success()
        );
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["commit", "-q", "--amend", "--no-edit"])
                .status()?
                .success(),
            "the final conflict may also be amended before continuing"
        );
        let repo = crate::test_repository::open_with(
            fixture.path(),
            ["user.name=todo author", "user.email=todo@example.com"],
        )?;
        apply_document(repo, &std::fs::read(next_output)?, None)?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["diff", "--name-only", "--diff-filter=U"])
                .output()?
                .stdout
                .is_empty(),
            "the continuation consumes the resolved index"
        );
        assert!(
            crate::history::all_pins(&crate::test_repository::open(fixture.path())?)?.is_empty(),
            "successive materialized conflicts do not leave departure pins"
        );
        Ok(())
    }

    #[test]
    fn successful_apply_does_not_create_a_continuation_file() -> gix_testtools::Result {
        let (_fixture, repo) = repository()?;
        let prepared = prepare(
            &repo,
            &Todo {
                hide: vec!["HEAD~2".into()],
                no_auto_hide: false,
                onto: None,
                update_base: false,
                edit_and_apply: false,
                materialize_conflicts: None,
                tips: Vec::new(),
            },
        )?;
        let output_dir = gix_testtools::tempfile::tempdir()?;
        let output = output_dir.path().join("unused.md");
        apply_document(repo, &prepared.document, Some(&output))?;
        assert!(!output.exists(), "continuation output is created only after a conflict");
        Ok(())
    }
}
