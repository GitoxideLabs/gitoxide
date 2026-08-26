use std::{
    ffi::OsString,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

#[derive(Debug, clap::Args)]
pub(super) struct MessageArgs {
    /// Use this message instead of opening an editor; repeat to add paragraphs.
    #[arg(short = 'm', long, value_name = "MESSAGE", conflicts_with = "file")]
    pub(super) message: Vec<OsString>,
    /// Read the new message from this file, or from standard input with `-`.
    #[arg(short = 'f', long, value_name = "FILE", conflicts_with = "message")]
    pub(super) file: Option<PathBuf>,
    /// Set the author actor while preserving the original author date.
    #[arg(long, value_name = "NAME <EMAIL>")]
    pub(super) author: Option<OsString>,
}

#[derive(Debug, clap::Args)]
pub(super) struct Args {
    /// Revision resolving to the commit whose message should be edited.
    #[arg(value_name = "REVSPEC")]
    pub(super) revision: OsString,
    #[command(flatten)]
    pub(super) edit: MessageArgs,
}

pub(super) fn run(repository: gix::Repository, args: Args) -> Result<()> {
    let (target, resolved_graph) = super::resolve_commit(&repository, &args.revision, "reword target")?;
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
    let graph = match resolved_graph {
        Some(graph) => graph,
        None => crate::edit::loaded_view_graph_with(&repository, &revisions)?,
    };
    ensure_retained_target(&graph, target, &pins, attached_head)?;

    let author = args
        .edit
        .author
        .as_deref()
        .map(gix::path::os_str_into_bstr)
        .transpose()
        .context("author is not valid UTF-8")?;

    if let Some(message) = explicit_message(&args.edit, std::io::stdin())? {
        let output_repository = repository.clone();
        return finish(
            &output_repository,
            crate::edit::reword::apply_message_reporting(
                repository,
                &graph,
                target,
                &message,
                author.map(AsRef::as_ref),
            )?,
        );
    }

    let repository_path = repository.git_dir().to_owned();
    let bare = repository.is_bare();
    let change_id = crate::change_id::for_commit(&repository, target)?;
    let (editor, document) = crate::edit::reword::document_with_author(&repository, target, author.map(AsRef::as_ref))?;
    drop(repository);
    let edited = crate::edit::edit_document_without_terminal(
        editor,
        &document,
        &format!("tix-reword-{}-{}.md", std::process::id(), target.to_hex_with_len(7)),
    )?;
    let edited = match edited {
        Some(edited) => edited,
        None if author.is_some() => document,
        None => {
            println!("no reword performed: the editor document was unchanged");
            return Ok(());
        }
    };

    let mut repository = crate::open_repository(&repository_path, bare, false)
        .context("could not reopen repository after editing commit")?;
    repository.object_cache_size(None);
    let (graph, target) = crate::edit::reword::relocate_after_editor(&repository, &[], &[], change_id)?;
    let pins = crate::history::all_pins(&repository)?;
    let head = repository.head().context("could not read HEAD after editing commit")?;
    let attached_head = !head.is_detached() && head.id().map(gix::Id::detach) == Some(target);
    drop(head);
    ensure_retained_target(&graph, target, &pins, attached_head)?;
    let output_repository = repository.clone();
    finish_editor(
        &output_repository,
        crate::edit::reword::apply(repository, &graph, target, &edited)?,
    )
}

fn ensure_retained_target(
    graph: &crate::history::HistoryGraph,
    target: gix::ObjectId,
    pins: &[crate::history::Pin],
    attached_head: bool,
) -> Result<()> {
    if !attached_head && !pins.iter().any(|pin| graph.is_ancestor(target, pin.id)) {
        anyhow::bail!("the reword target or one of its descendants must be pinned");
    }
    Ok(())
}

pub(super) fn explicit_message(args: &MessageArgs, mut stdin: impl Read) -> Result<Option<Vec<u8>>> {
    if !args.message.is_empty() {
        let mut out = Vec::new();
        for (index, message) in args.message.iter().enumerate() {
            if index > 0 {
                out.extend_from_slice(b"\n\n");
            }
            out.extend_from_slice(
                gix::path::os_str_into_bstr(message)
                    .with_context(|| format!("message {} is not valid UTF-8", index + 1))?,
            );
        }
        return Ok(Some(out));
    }
    let Some(path) = args.file.as_deref() else {
        return Ok(None);
    };
    if path == Path::new("-") {
        let mut out = Vec::new();
        stdin
            .read_to_end(&mut out)
            .context("could not read the commit message from standard input")?;
        Ok(Some(out))
    } else {
        std::fs::read(path)
            .with_context(|| format!("could not read commit message at {}", path.display()))
            .map(Some)
    }
}

fn finish(repository: &gix::Repository, outcome: crate::edit::reword::Outcome) -> Result<()> {
    match outcome.commit {
        Some(id) => println!("{}", crate::change_id::display(repository, id, 7)?),
        None => println!("no reword performed: the edited commit was unchanged"),
    }
    super::print_ref_rewrites(repository, &outcome.ref_rewrites)?;
    super::record_undo(repository, "reword commit", Ok(outcome.ref_changes));
    Ok(())
}

fn finish_editor(repository: &gix::Repository, outcome: crate::edit::reword::Outcome) -> Result<()> {
    let title = if outcome.commit.is_some() {
        "reword commit"
    } else {
        "edit commit enrichment"
    };
    match (outcome.commit, outcome.enrichment) {
        (Some(id), _) => println!("{}", crate::change_id::display(repository, id, 7)?),
        (None, Some(_)) => println!("{}", crate::change_id::display(repository, outcome.target, 7)?),
        (None, None) => println!("no reword performed: the edited commit was unchanged"),
    }
    super::print_ref_rewrites(repository, &outcome.ref_rewrites)?;
    super::record_undo(repository, title, Ok(outcome.ref_changes));
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
        Ok(crate::test_repository::open_with(
            path,
            [format!("core.editor={editor}")],
        )?)
    }

    fn args(revision: &str) -> Args {
        Args {
            revision: revision.into(),
            edit: MessageArgs {
                message: Vec::new(),
                file: None,
                author: None,
            },
        }
    }

    #[test]
    fn explicit_message_sources_are_complete_and_git_like() -> gix_testtools::Result {
        let mut message_args = args("HEAD");
        message_args.edit.message = vec!["title".into(), "body".into()];
        assert_eq!(
            explicit_message(&message_args.edit, &b"ignored"[..])?,
            Some(b"title\n\nbody".to_vec()),
            "repeated messages become paragraphs without reading stdin"
        );

        let mut file_args = args("HEAD");
        file_args.edit.file = Some("-".into());
        assert_eq!(
            explicit_message(&file_args.edit, &b"from stdin\n"[..])?,
            Some(b"from stdin\n".to_vec()),
            "a dash reads the entire message from stdin"
        );

        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let path = fixture.path().join("message.md");
        std::fs::write(&path, b"from file\n\nbody\n")?;
        let mut file_args = args("HEAD");
        file_args.edit.file = Some(path);
        assert_eq!(
            explicit_message(&file_args.edit, &b"ignored"[..])?,
            Some(b"from file\n\nbody\n".to_vec()),
            "a file supplies the complete message"
        );
        Ok(())
    }

    #[test]
    fn message_bypasses_the_editor_and_rewords_head() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repository = crate::test_repository::open(fixture.path())?;
        let original = repository.head_id()?.detach();
        crate::enrich::toggle(&repository, original)?;
        crate::enrich::set_note(&repository, original, Some(b"kept\n\nbody"))?;
        let mut initial = args("HEAD");
        initial.edit.message = vec!["replacement title".into(), ";literal body".into()];
        initial.edit.author = Some("Agent <agent@example.com>".into());
        run(open(fixture.path(), "false")?, initial)?;

        assert_eq!(
            git(fixture.path(), &["log", "-1", "--format=%B"])?,
            b"replacement title\n\n;literal body\n\n",
            "an explicit message bypasses the editor and retains editor-comment-looking lines"
        );
        assert_eq!(
            git(fixture.path(), &["log", "-1", "--format=%an <%ae>"])?,
            b"Agent <agent@example.com>\n",
            "an explicit message applies the author without opening an editor"
        );
        let repository = crate::test_repository::open(fixture.path())?;
        let head = repository.head_id()?.detach();
        assert_eq!(
            crate::enrich::load(
                &mut crate::enrich::open(&repository)?,
                crate::change_id::for_commit(&repository, head)?
            )?,
            crate::enrich::Enrichment {
                todo: true,
                note: Some("kept\n\nbody".into()),
            },
            "explicit messages preserve enrichments"
        );
        let rewritten = git(fixture.path(), &["rev-parse", "HEAD"])?;
        let mut same = args("HEAD");
        same.edit.message = vec!["replacement title\n\n;literal body".into()];
        run(open(fixture.path(), "false")?, same)?;
        assert_eq!(
            git(fixture.path(), &["rev-parse", "HEAD"])?,
            rewritten,
            "an unchanged explicit message does not rewrite the commit"
        );

        let mut empty = args("HEAD");
        empty.edit.message = vec!["   ".into()];
        let err = run(open(fixture.path(), "false")?, empty)
            .expect_err("an empty cleaned message must not replace the commit message");
        assert!(format!("{err:#}").contains("message is empty"));
        Ok(())
    }

    #[test]
    fn author_prefills_the_editor_and_preserves_message_and_date() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let message = git(fixture.path(), &["log", "-1", "--format=%B"])?;
        let author_date = git(fixture.path(), &["log", "-1", "--format=%aI"])?;
        let mut reword = args("HEAD");
        reword.edit.author = Some("Agent <agent@example.com>".into());
        run(open(fixture.path(), ":")?, reword)?;

        assert_eq!(git(fixture.path(), &["log", "-1", "--format=%B"])?, message);
        assert_eq!(git(fixture.path(), &["log", "-1", "--format=%aI"])?, author_date);
        assert_eq!(
            git(fixture.path(), &["log", "-1", "--format=%an <%ae>"])?,
            b"Agent <agent@example.com>\n"
        );

        let old = git(fixture.path(), &["rev-parse", "HEAD"])?;
        let mut invalid = args("HEAD");
        invalid.edit.author = Some("missing-email".into());
        let err = run(open(fixture.path(), "false")?, invalid)
            .expect_err("an invalid author is rejected before invoking the editor");
        assert!(format!("{err:#}").contains("author identity"));
        assert_eq!(git(fixture.path(), &["rev-parse", "HEAD"])?, old);
        Ok(())
    }

    #[test]
    fn attached_head_needs_no_pin() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        run(
            open(
                fixture.path(),
                &crate::test_repository::replacing_editor("tip", "rewritten tip"),
            )?,
            args("HEAD"),
        )?;

        assert_eq!(git(fixture.path(), &["log", "-1", "--format=%s"])?, b"rewritten tip\n");
        let repository = crate::test_repository::open(fixture.path())?;
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
        assert!(crate::history::all_pins(&crate::test_repository::open(fixture.path())?)?.is_empty());
        Ok(())
    }

    #[test]
    fn other_targets_require_a_covering_pin_before_the_editor() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let path = fixture.path();
        let err = run(open(path, "false")?, args("HEAD~1")).expect_err("an ancestor without a pin is rejected");
        assert!(format!("{err:#}").contains("must be pinned"));

        let repository = crate::test_repository::open(path)?;
        let old_tip = repository.head_id()?.detach();
        repository.reference(
            "refs/worktree/tix/pins/keep",
            old_tip,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "test pin",
        )?;
        let change_id = crate::change_id::for_commit(&repository, repository.rev_parse_single("HEAD~1")?.detach())?
            .to_reverse_hex_with_len(7)
            .to_string();
        drop(repository);
        run(
            open(
                path,
                &crate::test_repository::replacing_editor("middle", "rewritten middle"),
            )?,
            args(&change_id),
        )?;

        let repository = crate::test_repository::open(path)?;
        let new_tip = repository.head_id()?.detach();
        assert_ne!(new_tip, old_tip, "the checked-out descendant is reparented");
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
                .is_none(),
            "the checked-out descendant is replayed eagerly"
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

        run(
            open(
                path,
                &crate::test_repository::replacing_editor("side", "rewritten side"),
            )?,
            args("side"),
        )?;

        assert_eq!(
            git(path, &["rev-parse", "main"])?,
            main,
            "the unrelated checkout stack is untouched"
        );
        assert_eq!(git(path, &["log", "-1", "--format=%s", "side"])?, b"rewritten side\n");
        let repository = crate::test_repository::open(path)?;
        assert_eq!(
            crate::history::all_pins(&repository)?[0].id,
            repository.rev_parse_single("side")?,
            "the explicit unrelated pin follows the rewritten commit"
        );
        Ok(())
    }
}
