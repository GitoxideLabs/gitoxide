use anyhow::{Context, Result};
use gix::bstr::{BString, ByteSlice};

use super::rebase;

const AUTHOR: &[u8] = b"Author: ";
const AUTHOR_DATE: &[u8] = b"AuthorDate: ";
const COMMITTER: &[u8] = b"Committer: ";
const COMMITTER_DATE: &[u8] = b"CommitterDate: ";
const COMMENT_CHAR: &[u8] = b"CommentChar: ";
const MESSAGE: &[u8] = b"Message:";
const TODO: &[u8] = b"Todo";
pub(super) const DEFAULT_COMMENT_CHAR: &[u8] = b";";
pub(super) const ASSISTED_BY: &[u8] = b"Assisted-by: GPT 5.6";
pub(super) const CO_AUTHORED_BY: &[u8] = b"Co-authored-by: GPT 5.6 <codex@openai.com>";

pub(super) struct Edit<'a> {
    pub author: &'a [u8],
    pub author_time: gix::date::Time,
    pub committer: &'a [u8],
    pub committer_time: gix::date::Time,
    pub message: BString,
    pub enrichment: crate::enrich::Headers,
}

pub(crate) struct Outcome {
    pub commit: Option<gix::ObjectId>,
    pub enrichment: Option<crate::enrich::Enrichment>,
    pub ref_rewrites: Vec<rebase::RefRewrite>,
    pub ref_changes: Vec<super::undo::RefChange>,
}

#[tracing::instrument(skip_all, fields(commit_id = %id))]
pub(crate) fn document(repo: &gix::Repository, id: gix::ObjectId) -> Result<(std::ffi::OsString, Vec<u8>)> {
    document_with_author(repo, id, None)
}

pub(crate) fn document_with_author(
    repo: &gix::Repository,
    id: gix::ObjectId,
    author: Option<&[u8]>,
) -> Result<(std::ffi::OsString, Vec<u8>)> {
    let editor = repo.editor().context("no Git editor is available")?;
    let mut commit = repo
        .find_commit(id)
        .context("could not find commit to reword")?
        .decode()
        .context("could not decode commit to reword")?
        .into_owned()
        .context("could not own commit to reword")?;
    if let Some(author) = author {
        commit.author = actor(author, commit.author.time, "author")?;
    }
    let committer = repo
        .committer()
        .context("no Git committer is configured")?
        .context("could not resolve the Git committer")?
        .to_owned()
        .context("could not own the Git committer")?;
    let enrichment = crate::enrich::load(&mut crate::enrich::open(repo)?, crate::change_id::for_commit(repo, id)?)?;

    let mut out = Vec::new();
    write_headers(&mut out, &commit.author, &committer, &enrichment)?;
    out.push(b'\n');
    out.extend_from_slice(&commit.message);
    if !out.ends_with(b"\n") {
        out.push(b'\n');
    }
    let suggestions = missing_agent_trailers(&commit.message);
    if suggestions.iter().any(Option::is_some) {
        if !out.ends_with(b"\n\n") {
            out.push(b'\n');
        }
        for trailer in suggestions.into_iter().flatten() {
            out.extend_from_slice(DEFAULT_COMMENT_CHAR);
            out.extend_from_slice(trailer);
            out.push(b'\n');
        }
    }
    Ok((editor, out))
}

pub(super) fn missing_agent_trailers(message: &[u8]) -> [Option<&'static [u8]>; 2] {
    let mut has_assisted_by = false;
    let mut has_co_authored_by = false;
    if let Some(body) = gix::objs::commit::MessageRef::from_bytes(message).body() {
        for trailer in body.trailers() {
            has_assisted_by |= trailer.is_assisted_by();
            has_co_authored_by |= trailer.is_co_authored_by();
        }
    }
    [
        (!has_assisted_by).then_some(ASSISTED_BY),
        (!has_co_authored_by).then_some(CO_AUTHORED_BY),
    ]
}

#[tracing::instrument(skip_all, fields(commit_id = %old_id))]
pub(crate) fn apply(
    repo: gix::Repository,
    graph: &crate::history::HistoryGraph,
    old_id: gix::ObjectId,
    edited: &[u8],
) -> Result<Outcome> {
    let edit = parse(edited)?;
    if edit.message.is_empty() {
        anyhow::bail!("the edited commit message is empty");
    }

    let mut commit = repo
        .find_commit(old_id)
        .context("could not find commit after editing")?
        .decode()
        .context("could not decode commit after editing")?
        .into_owned()
        .context("could not own commit after editing")?;
    let author = actor(edit.author, edit.author_time, "author")?;
    let commit_changed = author != commit.author || edit.message != commit.message;
    let rebased = if commit_changed {
        commit.author = author;
        commit.committer = actor(edit.committer, edit.committer_time, "committer")?;
        commit.message = edit.message;
        Some(apply_commit(&repo, graph, old_id, commit)?)
    } else {
        None
    };
    let commit = rebased
        .as_ref()
        .and_then(|outcome| outcome.selected)
        .filter(|new_id| *new_id != old_id);
    let enrich_ref: gix::refs::FullName = crate::enrich::REF_NAME.try_into().expect("valid enrich ref");
    let enrich_before = super::undo::state(&repo, enrich_ref.as_ref())?;
    let enrichment = crate::enrich::apply_headers(&repo, commit.unwrap_or(old_id), &edit.enrichment)?;
    let enrich_after = super::undo::state(&repo, enrich_ref.as_ref())?;
    let (ref_rewrites, mut ref_changes) = rebased.map_or_else(
        || (Vec::new(), Vec::new()),
        |outcome| (outcome.ref_rewrites, outcome.ref_changes),
    );
    if enrich_before != enrich_after {
        ref_changes.push(super::undo::RefChange {
            name: enrich_ref,
            before: enrich_before,
            after: enrich_after,
        });
    }
    Ok(Outcome {
        commit,
        enrichment,
        ref_rewrites,
        ref_changes,
    })
}

#[tracing::instrument(skip_all, fields(commit_id = %old_id))]
pub(crate) fn apply_message_reporting(
    repo: gix::Repository,
    graph: &crate::history::HistoryGraph,
    old_id: gix::ObjectId,
    message: &[u8],
    author: Option<&[u8]>,
) -> Result<Outcome> {
    let message = cleanup_message(message, None);
    if message.is_empty() {
        anyhow::bail!("the edited commit message is empty");
    }
    let mut commit = repo
        .find_commit(old_id)
        .context("could not find commit to reword")?
        .decode()
        .context("could not decode commit to reword")?
        .into_owned()
        .context("could not own commit to reword")?;
    let changed_author = author
        .map(|author| actor(author, commit.author.time, "author"))
        .transpose()?;
    if commit.message == message && changed_author.as_ref().is_none_or(|author| *author == commit.author) {
        return Ok(Outcome {
            commit: None,
            enrichment: None,
            ref_rewrites: Vec::new(),
            ref_changes: Vec::new(),
        });
    }
    if let Some(author) = changed_author {
        commit.author = author;
    }
    commit.message = message;
    let outcome = apply_commit(&repo, graph, old_id, commit)?;
    Ok(Outcome {
        commit: outcome.selected.filter(|new_id| *new_id != old_id),
        enrichment: None,
        ref_rewrites: outcome.ref_rewrites,
        ref_changes: outcome.ref_changes,
    })
}

fn apply_commit(
    repo: &gix::Repository,
    graph: &crate::history::HistoryGraph,
    old_id: gix::ObjectId,
    commit: gix::objs::Commit,
) -> Result<rebase::Outcome> {
    rebase::perform(
        repo,
        graph,
        rebase::Edit::Replace { target: old_id, commit },
        rebase::Signature::RedoIfNeeded,
        rebase::Tree::LeaveAsIsAndMark,
    )?
    .complete()
}

pub(super) fn write_headers(
    out: &mut Vec<u8>,
    author: &gix::actor::Signature,
    committer: &gix::actor::Signature,
    enrichment: &crate::enrich::Enrichment,
) -> Result<()> {
    write_actor(out, AUTHOR, author);
    write_date(out, AUTHOR_DATE, author.time)?;
    write_actor(out, COMMITTER, committer);
    write_date(out, COMMITTER_DATE, committer.time)?;
    out.extend_from_slice(COMMENT_CHAR);
    out.extend_from_slice(DEFAULT_COMMENT_CHAR);
    out.push(b'\n');
    if !enrichment.todo {
        out.extend_from_slice(DEFAULT_COMMENT_CHAR);
    }
    out.extend_from_slice(TODO);
    out.push(b'\n');
    if enrichment.note.is_none() {
        out.extend_from_slice(DEFAULT_COMMENT_CHAR);
    }
    out.extend_from_slice(MESSAGE);
    if let Some(note) = enrichment.note.as_deref() {
        out.push(b' ');
        out.extend_from_slice(gix::objs::commit::MessageRef::from_bytes(note).summary().as_ref());
    }
    out.push(b'\n');
    Ok(())
}

fn write_actor(out: &mut Vec<u8>, label: &[u8], actor: &gix::actor::Signature) {
    out.extend_from_slice(label);
    out.extend_from_slice(&actor.name);
    out.extend_from_slice(b" <");
    out.extend_from_slice(&actor.email);
    out.extend_from_slice(b">\n");
}

fn write_date(out: &mut Vec<u8>, label: &[u8], time: gix::date::Time) -> Result<()> {
    out.extend_from_slice(label);
    out.extend_from_slice(
        time.format(gix::date::time::format::ISO8601)
            .context("could not format commit date")?
            .as_bytes(),
    );
    out.push(b'\n');
    Ok(())
}

pub(super) fn parse(input: &[u8]) -> Result<Edit<'_>> {
    let mut parts = input.splitn(6, |byte| *byte == b'\n');
    let author = header(parts.next(), AUTHOR)?;
    let author_time = date(header(parts.next(), AUTHOR_DATE)?, "author")?;
    let committer = header(parts.next(), COMMITTER)?;
    let committer_time = date(header(parts.next(), COMMITTER_DATE)?, "committer")?;
    let comment_char = header(parts.next(), COMMENT_CHAR)?;
    if comment_char.contains(&b'\r') {
        anyhow::bail!("CommentChar must not contain a line ending");
    }
    let remainder = parts.next().context("the enrichment headers are missing")?;
    let mut enrichment = crate::enrich::Headers::default();
    let mut todo_seen = false;
    let mut message_seen = false;
    let mut message_offset = None;
    let mut consumed = 0;
    for line in remainder.lines_with_terminator() {
        consumed += line.len();
        let line = trim_cr(line.strip_suffix(b"\n").unwrap_or(line));
        if line.is_empty() {
            message_offset = Some(consumed);
            break;
        }
        if line.starts_with(comment_char) {
            continue;
        }
        if line == TODO {
            if std::mem::replace(&mut todo_seen, true) {
                anyhow::bail!("duplicate Todo header");
            }
            enrichment.todo = true;
        } else if let Some(title) = line.strip_prefix(MESSAGE) {
            if std::mem::replace(&mut message_seen, true) {
                anyhow::bail!("duplicate Message header");
            }
            let title = title.trim();
            enrichment.message = (!title.is_empty()).then(|| title.into());
        } else {
            anyhow::bail!("unknown commit header: {}", line.as_bstr());
        }
    }
    let message_offset = message_offset.context("expected an empty line after the commit headers")?;
    let message = cleanup_message(
        remainder
            .get(message_offset..)
            .context("the commit message is missing")?,
        Some(comment_char),
    );
    Ok(Edit {
        author,
        author_time,
        committer,
        committer_time,
        message,
        enrichment,
    })
}

pub(crate) fn cleanup_message(input: &[u8], comment_char: Option<&[u8]>) -> BString {
    let mut out = Vec::new();
    let mut empty_lines = 0;
    for line in input.lines_with_terminator() {
        let line = trim_cr(line.strip_suffix(b"\n").unwrap_or(line));
        if comment_char.is_some_and(|comment_char| line.starts_with(comment_char)) {
            continue;
        }
        let line = &line[..line
            .iter()
            .rposition(|byte| !byte.is_ascii_whitespace())
            .map_or(0, |pos| pos + 1)];
        if line.is_empty() {
            empty_lines += 1;
            continue;
        }
        if !out.is_empty() && empty_lines > 0 {
            out.push(b'\n');
        }
        empty_lines = 0;
        out.extend_from_slice(line);
        out.push(b'\n');
    }
    out.into()
}

fn header<'a>(line: Option<&'a [u8]>, prefix: &[u8]) -> Result<&'a [u8]> {
    trim_cr(line.context("a commit header is missing")?)
        .strip_prefix(prefix)
        .filter(|value| !value.is_empty())
        .with_context(|| format!("expected a non-empty {} header", prefix[..prefix.len() - 2].as_bstr()))
}

fn trim_cr(line: &[u8]) -> &[u8] {
    line.strip_suffix(b"\r").unwrap_or(line)
}

fn date(value: &[u8], field: &str) -> Result<gix::date::Time> {
    let value = std::str::from_utf8(value).with_context(|| format!("{field} date is not UTF-8"))?;
    gix::date::parse(value, None)
        .map_err(|err| anyhow::Error::new(err.into_error()))
        .with_context(|| format!("could not parse {field} date"))
}

pub(super) fn actor(value: &[u8], time: gix::date::Time, field: &str) -> Result<gix::actor::Signature> {
    let parsed = gix::actor::SignatureRef::from_bytes(value)
        .with_context(|| format!("could not parse {field} identity"))?
        .trim();
    if parsed.name.is_empty() || parsed.email.is_empty() || !parsed.time.is_empty() {
        anyhow::bail!("{field} must be written as Name <email>");
    }
    Ok(gix::actor::Signature {
        name: parsed.name.into(),
        email: parsed.email.into(),
        time,
    })
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    #[test]
    fn parses_the_edit_document() -> gix_testtools::Result {
        let input = b"Author: A U Thor <author@example.com>\n\
                      AuthorDate: 2026-08-12 10:20:30 +0200\n\
                      Committer: C O Mitter <committer@example.com>\n\
                      CommitterDate: 2026-08-12 11:20:30 +0200\n\
                      CommentChar: ;\n\
                      \n\
                      title\n\nbody\n";
        let edit = parse(input)?;
        assert_eq!(
            edit.author, b"A U Thor <author@example.com>",
            "the author identity is preserved"
        );
        assert_eq!(edit.author_time.offset, 7200, "the author timezone is parsed");
        assert_eq!(
            edit.committer, b"C O Mitter <committer@example.com>",
            "the committer identity is preserved"
        );
        assert_eq!(edit.committer_time.offset, 7200, "the committer timezone is parsed");
        assert_eq!(
            edit.message, b"title\n\nbody\n",
            "the message is preserved byte-for-byte"
        );
        Ok(())
    }

    #[test]
    fn document_does_not_repeat_existing_agent_trailers() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_read_only("history.sh")?;
        let repository = crate::test_repository::open_with(
            fixture,
            [
                "committer.name=Current Committer",
                "committer.email=current@example.com",
            ],
        )?;
        let topic = repository.find_reference("refs/heads/topic")?.id().detach();
        let (editor, document) = document(&repository, topic)?;
        assert_eq!(editor, ":", "the configured editor is returned");
        let edit = parse(&document)?;
        assert_eq!(
            edit.committer, b"Current Committer <current@example.com>",
            "the template shows the repository's current committer"
        );
        assert_eq!(
            edit.committer_time.seconds, 978_307_200,
            "the configured committer date is shown"
        );
        assert!(
            document
                .windows(b"CommentChar: ;\n;Todo\n;Message:\n\n".len())
                .any(|line| line == b"CommentChar: ;\n;Todo\n;Message:\n\n"),
            "the template declares its comment prefix and inactive enrichments"
        );
        assert!(
            !document
                .windows(b";Assisted-by:".len())
                .any(|line| line == b";Assisted-by:")
                && !document
                    .windows(b";Co-authored-by:".len())
                    .any(|line| line == b";Co-authored-by:"),
            "existing trailer keys suppress model-specific suggestions regardless of their values"
        );
        Ok(())
    }

    #[test]
    fn parses_bare_todo_and_single_line_message_headers() -> gix_testtools::Result {
        let document = |headers: &str| {
            format!(
                "Author: A <a@example.com>\n\
                 AuthorDate: 2026-08-12 10:20:30 +0200\n\
                 Committer: C <c@example.com>\n\
                 CommitterDate: 2026-08-12 11:20:30 +0200\n\
                 CommentChar: ;\n{headers}\n\n\
                 title\n"
            )
        };

        let active_document = document("Todo\nMessage: enrichment title");
        let edit = parse(active_document.as_bytes())?;
        assert_eq!(
            edit.enrichment,
            crate::enrich::Headers {
                todo: true,
                message: Some("enrichment title".into()),
            }
        );
        assert_eq!(
            parse(document(";Todo\n;Message:").as_bytes())?.enrichment,
            Default::default()
        );
        assert_eq!(parse(document("Message:").as_bytes())?.enrichment, Default::default());
        assert!(
            parse(document("Todo\nTodo").as_bytes()).is_err(),
            "duplicate Todo is rejected"
        );
        assert!(
            parse(document("Message: one\nMessage: two").as_bytes()).is_err(),
            "duplicate Message is rejected"
        );
        assert!(
            parse(document("Todo: true").as_bytes()).is_err(),
            "Todo has no value syntax"
        );
        Ok(())
    }

    #[test]
    fn enrichment_only_edits_do_not_rewrite_the_commit() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repository = crate::test_repository::open_with(
            fixture.path(),
            [
                "committer.name=Current Committer",
                "committer.email=current@example.com",
            ],
        )?;
        let id = repository.head_id()?.detach();
        crate::enrich::toggle(&repository, id)?;
        crate::enrich::set_note(&repository, id, Some(b"Existing title\n\nexisting body\n"))?;
        let (_, document) = document(&repository, id)?;
        assert!(
            document
                .windows(b"Todo\nMessage: Existing title\n\n".len())
                .any(|window| { window == b"Todo\nMessage: Existing title\n\n" }),
            "active enrichments are shown before the commit message"
        );

        let edited = document.replacen(b"Message: Existing title", b"Message: New title", 1);
        let graph = super::super::loaded_graph(&repository)?;
        let outcome = apply(repository.clone(), &graph, id, &edited)?;
        assert!(
            outcome.commit.is_none(),
            "enrichment changes leave the commit object untouched"
        );
        assert_eq!(repository.head_id()?.detach(), id);
        let enrichment = outcome.enrichment.expect("the enrichment changed");
        assert!(enrichment.todo);
        assert_eq!(
            enrichment.note.as_ref().map(|note| note.as_bstr()),
            Some(b"New title\n\nexisting body\n".as_bstr())
        );
        Ok(())
    }

    #[test]
    fn offers_only_missing_agent_trailer_keys() {
        assert_eq!(
            missing_agent_trailers(b"title\n\nASSISTED-BY: another agent\n"),
            [None, Some(CO_AUTHORED_BY)],
            "trailer keys are matched case-insensitively and independently of their values"
        );
        assert_eq!(
            missing_agent_trailers(b"title\n\nco-AUTHORED-by: Someone <someone@example.com>\n"),
            [Some(ASSISTED_BY), None],
            "either missing trailer remains available for opt-in"
        );
    }

    #[test]
    fn cleanup_honors_git_style_comment_prefixes_and_opted_in_trailers() -> gix_testtools::Result {
        let input = b"Author: A <a@example.com>\n\
                      AuthorDate: 2026-08-12 10:20:30 +0200\n\
                      Committer: C <c@example.com>\n\
                      CommitterDate: 2026-08-12 11:20:30 +0200\n\
                      CommentChar: //\n\
                      \n\
                      \nsubject  \n\n\ninline // stays  \n // indented stays\n//removed\n\nAssisted-by: GPT 5.6\n//Co-authored-by: GPT 5.6 <codex@openai.com>\n";
        assert_eq!(
            parse(input)?.message,
            b"subject\n\ninline // stays\n // indented stays\n\nAssisted-by: GPT 5.6\n".as_bstr(),
            "only column-zero comments are removed and Git whitespace cleanup is applied"
        );

        let empty_comment = input.replacen(b"CommentChar: //", b"CommentChar: ", 1);
        assert!(parse(&empty_comment).is_err(), "the comment prefix cannot be empty");
        Ok(())
    }

    #[test]
    fn marks_only_reworded_descendants_for_lazy_replay() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repository = crate::test_repository::open_with(
            fixture.path(),
            [
                "committer.name=Current Committer",
                "committer.email=current@example.com",
            ],
        )?;
        let middle = repository.rev_parse_single("HEAD~1")?.detach();
        let (_, document) = document(&repository, middle)?;
        let edited = document
            .replacen(
                b"Committer: Current Committer <current@example.com>",
                b"Committer: Edited Committer <edited@example.com>",
                1,
            )
            .replacen(b"\nmiddle\n", b"\nrewritten middle\n", 1);
        let graph = super::super::loaded_graph(&repository)?;
        let new_middle = apply(repository.clone(), &graph, middle, &edited)?
            .commit
            .expect("the message changed");
        let new_tip = repository.head_id()?.detach();
        let rewritten = repository.find_commit(new_middle)?.decode()?.into_owned()?;
        assert_eq!(rewritten.committer.name, b"Current Committer".as_bstr());
        assert_eq!(rewritten.committer.email, b"current@example.com".as_bstr());
        assert_eq!(
            rewritten.committer.time.seconds, 978_307_200,
            "edited committer fields cannot override the current repository committer"
        );
        assert!(
            !rebase::is_pending(&repository.find_commit(new_middle)?.decode()?.into_owned()?),
            "the reworded commit already has its final tree, parent, and signature"
        );
        assert!(
            rebase::has_marker(&repository.find_commit(new_tip)?.decode()?.into_owned()?),
            "the reparented descendant retains its original parent for lazy replay"
        );
        Ok(())
    }

    #[test]
    fn signed_rewords_leave_pending_descendants_unsigned() -> gix_testtools::Result {
        if !gix_testtools::signature::program_available("ssh-keygen") {
            return Ok(());
        }
        let (_key_home, key) = gix_testtools::signature::ssh_private_key()?;
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repository = crate::test_repository::open_with(
            fixture.path(),
            [
                "commit.gpgSign=true".to_owned(),
                "gpg.format=ssh".to_owned(),
                format!("user.signingKey={}", key.display()),
                format!(
                    "gpg.ssh.allowedSignersFile={}",
                    gix_testtools::signature::fixture("ssh-allowed-signers").display()
                ),
            ],
        )?;
        let middle = repository.rev_parse_single("HEAD~1")?.detach();
        let (_, document) = document(&repository, middle)?;
        let edited = document.replacen(b"\nmiddle\n", b"\nrewritten middle\n", 1);
        let graph = super::super::loaded_graph(&repository)?;
        let rewritten = apply(repository.clone(), &graph, middle, &edited)?
            .commit
            .expect("the message changed");
        assert!(
            repository
                .find_commit(rewritten)?
                .verify_signature()?
                .expect("the edited commit is signed")
                .is_valid(),
            "the final edited commit receives its configured signature"
        );

        let descendant = repository.find_commit(repository.head_id()?)?.decode()?.into_owned()?;
        assert!(rebase::has_marker(&descendant), "the descendant remains pending");
        assert!(
            descendant
                .extra_headers
                .iter()
                .filter(|(name, _)| name == "gpgsig" || name == "gpgsig-sha256")
                .all(|(_, value)| value.is_empty()),
            "a pending descendant never carries a usable signature"
        );
        Ok(())
    }

    #[test]
    fn rewrites_direct_refs_except_tags_and_remotes_and_signs_when_enabled() -> gix_testtools::Result {
        if !gix_testtools::signature::program_available("ssh-keygen") {
            return Ok(());
        }
        let (_key_home, key) = gix_testtools::signature::ssh_private_key()?;
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let old_id = crate::test_repository::open(fixture.path())?.head_id()?.detach();
        let git = |args: &[&str]| -> std::io::Result<std::process::ExitStatus> {
            Command::new("git").arg("-C").arg(fixture.path()).args(args).status()
        };
        for name in ["refs/patches/reword", "refs/tags/keep", "refs/remotes/origin/keep"] {
            assert!(
                git(&["update-ref", name, &old_id.to_string()])?.success(),
                "the test reference is created"
            );
        }

        let repository = crate::test_repository::open_with(
            fixture.path(),
            [
                "commit.gpgSign=true".to_owned(),
                "gpg.format=ssh".to_owned(),
                format!("user.signingKey={}", key.display()),
                format!(
                    "gpg.ssh.allowedSignersFile={}",
                    gix_testtools::signature::fixture("ssh-allowed-signers").display()
                ),
            ],
        )?;
        let edited = b"Author: New Author <new-author@example.com>\n\
                       AuthorDate: 2026-08-12 10:20:30 +0200\n\
                       Committer: New Committer <new-committer@example.com>\n\
                       CommitterDate: 2026-08-12 11:20:30 +0200\n\
                       CommentChar: ;\n\
                       \n\
                       rewritten title\n\nrewritten body\n\nAssisted-by: GPT 5.6\n;Co-authored-by: GPT 5.6 <codex@openai.com>\n";
        let graph = super::super::loaded_graph(&repository)?;
        let new_id = apply(repository.clone(), &graph, old_id, edited)?
            .commit
            .expect("the edited commit differs");
        let commit = repository.find_commit(new_id)?;
        let decoded = commit.decode()?;
        assert_eq!(
            decoded.message,
            b"rewritten title\n\nrewritten body\n\nAssisted-by: GPT 5.6\n".as_bstr()
        );
        assert_eq!(decoded.author()?.name, b"New Author".as_bstr());
        assert!(
            !rebase::is_pending(&decoded.into_owned()?),
            "a signed reword with unchanged ancestry needs no later replay"
        );
        assert!(
            commit
                .verify_signature()?
                .expect("configured signing adds a signature")
                .is_valid(),
            "the rewritten commit has a valid configured signature"
        );
        for name in ["refs/heads/main", "refs/patches/reword"] {
            assert_eq!(
                repository.find_reference(name)?.id().detach(),
                new_id,
                "{name} follows the rewrite"
            );
        }
        for name in ["refs/tags/keep", "refs/remotes/origin/keep"] {
            assert_eq!(
                repository.find_reference(name)?.id().detach(),
                old_id,
                "{name} is not rewritten"
            );
        }
        Ok(())
    }
}
