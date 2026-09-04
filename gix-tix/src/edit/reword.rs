use anyhow::{Context, Result};
use gix::bstr::{BString, ByteSlice};

use super::rebase;

const AUTHOR: &[u8] = b"Author: ";
const CONFIGURED_AUTHOR: &[u8] = b"ConfiguredAuthor: ";
const AUTHOR_DATE: &[u8] = b"AuthorDate: ";
const COMMITTER: &[u8] = b"Committer: ";
const COMMITTER_DATE: &[u8] = b"CommitterDate: ";
const COMMENT_CHAR: &[u8] = b"CommentChar: ";
const MESSAGE: &[u8] = b"Message:";
const TODO: &[u8] = b"Todo";
const DEFAULT_COMMENT_CHAR: &[u8] = b";";
const ASSISTED_BY_KEY: &str = "tix.trailer.assistedBy";
const CO_AUTHORED_BY_KEY: &str = "tix.trailer.coAuthoredBy";
const AGENT_TRAILERS: [(&str, &[u8], &[u8]); 2] = [
    (ASSISTED_BY_KEY, b"Assisted-by: ", b"GPT 5.6"),
    (CO_AUTHORED_BY_KEY, b"Co-authored-by: ", b"GPT 5.6 <codex@openai.com>"),
];

struct AgentTrailer {
    key: &'static str,
    label: &'static [u8],
    value: BString,
    source: Option<gix::config::file::Metadata>,
}

pub(super) struct Edit<'a> {
    pub author: &'a [u8],
    pub author_time: gix::date::Time,
    pub committer: &'a [u8],
    pub committer_time: gix::date::Time,
    pub message: BString,
    pub enrichment: crate::enrich::Headers,
}

pub(crate) struct Outcome {
    pub target: gix::ObjectId,
    pub commit: Option<gix::ObjectId>,
    pub enrichment: Option<crate::enrich::Enrichment>,
    pub ref_rewrites: Vec<rebase::RefRewrite>,
    pub ref_changes: Vec<super::undo::RefChange>,
}

pub(crate) enum Perform {
    Complete(Outcome),
    Conflict(rebase::Conflict),
}

impl Perform {
    fn complete(self) -> Result<Outcome> {
        match self {
            Perform::Complete(outcome) => Ok(outcome),
            Perform::Conflict(_) => anyhow::bail!("rewording the commit would cause a merge conflict"),
        }
    }
}

pub(crate) fn relocate_after_editor(
    repo: &gix::Repository,
    revisions: &[std::ffi::OsString],
    hidden_revisions: &[std::ffi::OsString],
    change_id: gix::hash::ChangeId,
) -> Result<(crate::history::HistoryGraph, gix::ObjectId)> {
    let graph = super::loaded_explicit_view_graph(repo, revisions, hidden_revisions)?;
    let mut matches = Vec::new();
    for id in graph.stored_commit_ids() {
        if crate::change_id::for_commit(repo, id)? == change_id {
            matches.push(id);
        }
    }
    match matches.as_slice() {
        [target] => Ok((graph, *target)),
        [] => anyhow::bail!("change ID {change_id} is no longer present in the Tix view"),
        candidates => {
            let candidates = candidates
                .iter()
                .map(|id| crate::change_id::display_short(repo, *id))
                .collect::<Result<Vec<_>>>()?
                .join("\n  ");
            anyhow::bail!("change ID {change_id} is ambiguous in the Tix view; candidates:\n  {candidates}")
        }
    }
}

#[tracing::instrument(skip_all, fields(commit_id = %id))]
pub(crate) fn document(repo: &gix::Repository, id: gix::ObjectId) -> Result<(gix::command::Prepare, Vec<u8>)> {
    document_with_author(repo, id, None)
}

pub(crate) fn document_with_author(
    repo: &gix::Repository,
    id: gix::ObjectId,
    author: Option<&[u8]>,
) -> Result<(gix::command::Prepare, Vec<u8>)> {
    let editor = repo
        .editor_command()
        .context("could not prepare Git editor")?
        .context("no Git editor is available")?;
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
    let configured_author = repo
        .author()
        .transpose()
        .context("could not resolve the Git author")?
        .map(|author| author.to_owned().context("could not own the Git author"))
        .transpose()?
        .filter(|author| author.name != commit.author.name || author.email != commit.author.email);
    let committer = repo
        .committer()
        .context("no Git committer is configured")?
        .context("could not resolve the Git committer")?
        .to_owned()
        .context("could not own the Git committer")?;
    let enrichment = crate::enrich::load(&mut crate::enrich::open(repo)?, crate::change_id::for_commit(repo, id)?)?;

    let mut out = Vec::new();
    write_headers(
        &mut out,
        &commit.author,
        configured_author.as_ref(),
        &committer,
        &enrichment,
    )?;
    out.push(b'\n');
    out.extend_from_slice(&commit.message);
    if !out.ends_with(b"\n") {
        out.push(b'\n');
    }
    write_missing_agent_trailers(&mut out, repo, &commit.message)?;
    Ok((editor, out))
}

fn missing_agent_trailers(message: &[u8]) -> [bool; 2] {
    let mut has_assisted_by = false;
    let mut has_co_authored_by = false;
    if let Some(body) = gix::objs::commit::MessageRef::from_bytes(message).body() {
        for trailer in body.trailers() {
            has_assisted_by |= trailer.is_assisted_by();
            has_co_authored_by |= trailer.is_co_authored_by();
        }
    }
    [!has_assisted_by, !has_co_authored_by]
}

pub(super) fn write_missing_agent_trailers(out: &mut Vec<u8>, repo: &gix::Repository, message: &[u8]) -> Result<()> {
    let missing = missing_agent_trailers(message);
    let trailers = AGENT_TRAILERS
        .into_iter()
        .zip(missing)
        .filter(|(_, missing)| *missing)
        .map(|((key, label, default), _)| agent_trailer(repo, key, label, default))
        .collect::<Result<Vec<_>>>()?;
    if trailers.is_empty() {
        return Ok(());
    }

    if !out.ends_with(b"\n\n") {
        out.push(b'\n');
    }
    for trailer in &trailers {
        out.extend_from_slice(DEFAULT_COMMENT_CHAR);
        out.extend_from_slice(trailer.label);
        out.extend_from_slice(&trailer.value);
        out.push(b'\n');
    }
    for trailer in &trailers {
        out.extend_from_slice(b"; ");
        out.extend_from_slice(trailer.key.as_bytes());
        if let Some(source) = &trailer.source {
            if let Some(path) = &source.path {
                out.extend_from_slice(b" is configured in ");
                out.extend_from_slice(gix::path::into_bstr(path).as_ref());
            } else {
                out.extend_from_slice(b" is configured via ");
                out.extend_from_slice(config_source_name(source.source));
            }
        } else {
            out.extend_from_slice(b" is unset; using the built-in default");
        }
        out.extend_from_slice(b".\n");
    }
    Ok(())
}

fn agent_trailer(
    repo: &gix::Repository,
    key: &'static str,
    label: &'static [u8],
    default: &'static [u8],
) -> Result<AgentTrailer> {
    let config = repo.config_snapshot();
    let (value, source) = match config.raw_value_with_section(key) {
        Ok((value, section)) => {
            if value.trim().is_empty() || value.contains(&b'\n') || value.contains(&b'\r') {
                anyhow::bail!("Git configuration `{key}` must be a non-empty single-line trailer value");
            }
            (value, Some(section.meta().clone()))
        }
        Err(_) => (default.into(), None),
    };
    Ok(AgentTrailer {
        key,
        label,
        value,
        source,
    })
}

fn config_source_name(source: gix::config::Source) -> &'static [u8] {
    match source {
        gix::config::Source::Env => b"the environment",
        gix::config::Source::Cli => b"a command-line override",
        gix::config::Source::Api => b"an API override",
        gix::config::Source::EnvOverride => b"an environment override",
        gix::config::Source::GitInstallation
        | gix::config::Source::System
        | gix::config::Source::Git
        | gix::config::Source::User
        | gix::config::Source::Local
        | gix::config::Source::Worktree => b"Git configuration",
    }
}

#[tracing::instrument(skip_all, fields(commit_id = %old_id))]
pub(crate) fn apply(
    repo: gix::Repository,
    graph: &crate::history::HistoryGraph,
    old_id: gix::ObjectId,
    edited: &[u8],
) -> Result<Outcome> {
    apply_conflict_reporting(repo, graph, old_id, edited, |_| {})?.complete()
}

#[tracing::instrument(skip_all, fields(commit_id = %old_id))]
pub(crate) fn apply_conflict_reporting(
    repo: gix::Repository,
    graph: &crate::history::HistoryGraph,
    old_id: gix::ObjectId,
    edited: &[u8],
    mut report: impl FnMut(rebase::Progress),
) -> Result<Perform> {
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
    let (rebased, enrichment, enrich_change) = if commit_changed {
        commit.author = author;
        commit.committer = actor(edit.committer, edit.committer_time, "committer")?;
        commit.message = edit.message;
        let (performed, enrichment) =
            apply_commit_conflict_with_enrichment(&repo, graph, old_id, commit, &edit.enrichment, &mut report)?;
        let outcome = match performed {
            rebase::Perform::Complete(outcome) => outcome,
            rebase::Perform::Conflict(conflict) => return Ok(Perform::Conflict(conflict)),
        };
        (Some(outcome), enrichment, None)
    } else {
        let name: gix::refs::FullName = crate::enrich::REF_NAME.try_into().expect("valid enrich ref");
        let before = super::undo::state(&repo, name.as_ref())?;
        let enrichment = crate::enrich::apply_headers(&repo, old_id, &edit.enrichment)?;
        let after = super::undo::state(&repo, name.as_ref())?;
        let change = (before != after).then_some(super::undo::RefChange { name, before, after });
        (None, enrichment, change)
    };
    let commit = rebased
        .as_ref()
        .and_then(|outcome| outcome.selected)
        .filter(|new_id| *new_id != old_id);
    let (ref_rewrites, mut ref_changes) = rebased.map_or_else(
        || (Vec::new(), Vec::new()),
        |outcome| (outcome.ref_rewrites, outcome.ref_changes),
    );
    if let Some(change) = enrich_change {
        ref_changes.push(change);
    }
    Ok(Perform::Complete(Outcome {
        target: old_id,
        commit,
        enrichment,
        ref_rewrites,
        ref_changes,
    }))
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
            target: old_id,
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
        target: old_id,
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
    apply_commit_conflict(repo, graph, old_id, commit)?.complete()
}

fn apply_commit_conflict(
    repo: &gix::Repository,
    graph: &crate::history::HistoryGraph,
    old_id: gix::ObjectId,
    commit: gix::objs::Commit,
) -> Result<rebase::Perform> {
    rebase::perform(
        repo,
        graph,
        rebase::Edit::Replace { target: old_id, commit },
        rebase::Signature::RedoIfNeeded,
        rebase::Tree::LeaveAsIsAndMark,
    )
}

fn apply_commit_conflict_with_enrichment(
    repo: &gix::Repository,
    graph: &crate::history::HistoryGraph,
    old_id: gix::ObjectId,
    commit: gix::objs::Commit,
    headers: &crate::enrich::Headers,
    report: impl FnMut(rebase::Progress),
) -> Result<(rebase::Perform, Option<crate::enrich::Enrichment>)> {
    rebase::perform_with_enrichment_and_progress(
        repo,
        graph,
        rebase::Edit::Replace { target: old_id, commit },
        rebase::Signature::RedoIfNeeded,
        rebase::Tree::LeaveAsIsAndMark,
        headers,
        report,
    )
}

pub(super) fn write_headers(
    out: &mut Vec<u8>,
    author: &gix::actor::Signature,
    configured_author: Option<&gix::actor::Signature>,
    committer: &gix::actor::Signature,
    enrichment: &crate::enrich::Enrichment,
) -> Result<()> {
    write_actor(out, AUTHOR, author);
    if let Some(author) = configured_author {
        out.extend_from_slice(DEFAULT_COMMENT_CHAR);
        write_actor(out, CONFIGURED_AUTHOR, author);
    }
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
    let has_configured_author = input
        .splitn(3, |byte| *byte == b'\n')
        .nth(1)
        .map(trim_cr)
        .is_some_and(|line| {
            line.starts_with(CONFIGURED_AUTHOR)
                || line
                    .strip_prefix(DEFAULT_COMMENT_CHAR)
                    .is_some_and(|line| line.starts_with(CONFIGURED_AUTHOR))
        });
    let mut parts = input.splitn(6 + usize::from(has_configured_author), |byte| *byte == b'\n');
    let mut author = header(parts.next(), AUTHOR)?;
    let mut line = parts.next();
    if line
        .map(trim_cr)
        .and_then(|line| line.strip_prefix(DEFAULT_COMMENT_CHAR))
        .is_some_and(|line| line.starts_with(CONFIGURED_AUTHOR))
    {
        line = parts.next();
    } else if line
        .map(trim_cr)
        .is_some_and(|line| line.starts_with(CONFIGURED_AUTHOR))
    {
        author = header(line, CONFIGURED_AUTHOR)?;
        line = parts.next();
    }
    let author_time = date(header(line, AUTHOR_DATE)?, "author")?;
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
    fn offers_a_different_configured_author_for_selection() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_read_only("history.sh")?;
        let repository = crate::test_repository::open(fixture)?;
        let topic = repository.find_reference("refs/heads/topic")?.id().detach();
        let (_, topic_document) = document(&repository, topic)?;
        let offered = b"Author: Codex <Codex@OpenAI.com>\n\
                        ;ConfiguredAuthor: author <author@example.com>\n\
                        AuthorDate: ";
        assert!(
            topic_document.windows(offered.len()).any(|window| window == offered),
            "a differing configured author is offered directly below the commit author"
        );

        let original = parse(&topic_document)?;
        let selected = topic_document.replacen(b";ConfiguredAuthor: ", b"ConfiguredAuthor: ", 1);
        let configured = parse(&selected)?;
        assert_eq!(
            original.author, b"Codex <Codex@OpenAI.com>",
            "the commented configured author does not override the commit author"
        );
        assert_eq!(
            configured.author, b"author <author@example.com>",
            "uncommenting the configured author makes it authoritative"
        );
        assert_eq!(
            configured.author_time, original.author_time,
            "selecting the configured identity retains the commit's author date"
        );

        let main = repository.find_reference("refs/heads/main")?.id().detach();
        let (_, main_document) = document(&repository, main)?;
        assert!(
            !main_document
                .windows(CONFIGURED_AUTHOR.len())
                .any(|window| window == CONFIGURED_AUTHOR),
            "a matching configured author is not repeated"
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
        assert_eq!(editor.command, ":", "the configured editor is returned");
        assert!(editor.use_shell, "the shell provides the colon builtin");
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
    fn configured_agent_trailers_show_their_source_after_the_adjacent_suggestions() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["config", "--local", "tix.trailer.assistedBy", "Custom Assistant"])
                .status()?
                .success(),
            "the local trailer configuration is written"
        );
        let repository = crate::test_repository::open(fixture.path())?;
        let main = repository.find_reference("refs/heads/main")?.id().detach();
        let (_, document) = document(&repository, main)?;

        let trailers = b";Assisted-by: Custom Assistant\n\
                         ;Co-authored-by: GPT 5.6 <codex@openai.com>\n";
        assert!(
            document.windows(trailers.len()).any(|window| window == trailers),
            "configured and default suggestions remain adjacent: {}",
            document.as_bstr()
        );
        let mut configured = b"; tix.trailer.assistedBy is configured in ".to_vec();
        configured.extend_from_slice(gix::path::into_bstr(fixture.path().join(".git/config")).as_ref());
        configured.extend_from_slice(b".\n");
        assert!(
            document.windows(configured.len()).any(|window| window == configured),
            "the winning configuration file is shown: {}",
            document.as_bstr()
        );
        assert!(
            document
                .windows(b"; tix.trailer.coAuthoredBy is unset; using the built-in default.\n".len())
                .any(|window| window == b"; tix.trailer.coAuthoredBy is unset; using the built-in default.\n"),
            "the unset key is advertised: {}",
            document.as_bstr()
        );
        Ok(())
    }

    #[test]
    fn agent_trailer_overrides_without_files_name_their_source() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_read_only("history.sh")?;
        let repository = crate::test_repository::open_with(
            fixture,
            [
                "tix.trailer.assistedBy=API Assistant",
                "tix.trailer.coAuthoredBy=API Agent <agent@example.com>",
            ],
        )?;
        let main = repository.find_reference("refs/heads/main")?.id().detach();
        let (_, document) = document(&repository, main)?;
        let expected = b";Assisted-by: API Assistant\n\
                         ;Co-authored-by: API Agent <agent@example.com>\n\
                         ; tix.trailer.assistedBy is configured via an API override.\n\
                         ; tix.trailer.coAuthoredBy is configured via an API override.\n";
        assert!(
            document.windows(expected.len()).any(|window| window == expected),
            "source-less overrides are identified after the suggestions: {}",
            document.as_bstr()
        );
        Ok(())
    }

    #[test]
    fn invalid_agent_trailer_configuration_is_rejected() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_read_only("history.sh")?;
        for value in ["", "first\nsecond"] {
            let repository = crate::test_repository::open_with(&fixture, [format!("tix.trailer.assistedBy={value}")])?;
            let main = repository.find_reference("refs/heads/main")?.id().detach();
            let err = match document(&repository, main) {
                Ok(_) => panic!("invalid trailer values fail before opening the editor"),
                Err(err) => err,
            };
            assert!(
                format!("{err:#}").contains("tix.trailer.assistedBy"),
                "the invalid configuration key is identified: {err:#}"
            );
        }
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
        assert_eq!(outcome.target, id, "the outcome identifies the edited commit");
        assert!(
            outcome.commit.is_none(),
            "enrichment changes leave the commit object untouched"
        );
        assert_eq!(repository.head_id()?, id);
        let enrichment = outcome.enrichment.expect("the enrichment changed");
        assert!(enrichment.todo);
        assert_eq!(
            enrichment.note.as_ref().map(|note| note.as_bstr()),
            Some(b"New title\n\nexisting body\n".as_bstr())
        );
        Ok(())
    }

    #[test]
    fn relocates_an_editor_reword_onto_a_concurrent_amend() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let path = fixture.path();
        let repository = crate::test_repository::open_with(
            path,
            [
                "committer.name=Current Committer",
                "committer.email=current@example.com",
            ],
        )?;
        let original = repository.head_id()?.detach();
        let change_id = crate::change_id::for_commit(&repository, original)?;
        let (_, document) = document(&repository, original)?;
        let edited = document.replacen(b"\ntip\n", b"\nreworded tip\n", 1);

        let mut outside = repository.find_commit(original)?.decode()?.into_owned()?;
        outside.parents = [original].into_iter().collect();
        outside.message = "outside the view".into();
        crate::change_id::inherit(&repository, &mut outside, original)?;
        let outside = repository.write_object(&outside)?.detach();
        repository.reference(
            "refs/heads/outside",
            outside,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "test outside-view successor",
        )?;

        std::fs::write(path.join("concurrent"), b"amended while editing\n")?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(path)
                .args(["add", "concurrent"])
                .status()?
                .success(),
            "the concurrent tree change is staged"
        );
        let graph = super::super::loaded_view_graph(&repository)?;
        let amended =
            crate::edit::head::amend_index(repository.clone(), &graph)?.expect("the staged tree changes amend HEAD");
        let amended_commit = repository.find_commit(amended)?.decode()?.into_owned()?;
        assert_eq!(
            crate::change_id::for_commit(&repository, amended)?,
            change_id,
            "the concurrent amend preserves the stable identity"
        );

        let (graph, target) = relocate_after_editor(&repository, &[], &[], change_id)?;
        assert_eq!(target, amended, "the fresh view resolves the amended successor");
        let outcome = apply(repository.clone(), &graph, target, &edited)?;
        assert_eq!(outcome.target, amended, "the outcome reports the relocated target");
        let rewritten = outcome.commit.expect("the edited message rewrites the amended commit");
        let rewritten_commit = repository.find_commit(rewritten)?.decode()?.into_owned()?;
        assert_eq!(repository.head_id()?, rewritten, "HEAD follows the reword");
        assert_eq!(
            rewritten_commit.tree, amended_commit.tree,
            "the reword retains the concurrently amended tree"
        );
        assert_eq!(
            rewritten_commit.parents, amended_commit.parents,
            "the reword retains the current topology"
        );
        assert_eq!(rewritten_commit.message, b"reworded tip\n".as_bstr());
        assert_eq!(
            repository.find_reference("refs/heads/outside")?.id(),
            outside,
            "a duplicate identity outside the active view is neither selected nor rewritten"
        );
        Ok(())
    }

    #[test]
    fn editor_relocation_requires_one_visible_commit() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let path = fixture.path();
        let repository = crate::test_repository::open(path)?;
        let head = repository.head_id()?.detach();
        let index_before = std::fs::read(repository.index_path())?;
        let worktree_before = std::fs::read(path.join("tip"))?;

        let missing = gix::hash::ChangeId::from(gix::ObjectId::Sha1([42; 20]));
        let err = relocate_after_editor(&repository, &[], &[], missing)
            .expect_err("an identity outside the current view cannot be edited");
        assert!(
            format!("{err:#}").contains("no longer present"),
            "the missing identity is explained: {err:#}"
        );

        let change_id = crate::change_id::for_commit(&repository, head)?;
        let mut colliding_change_id = change_id.to_reverse_hex().to_string().into_bytes();
        let last = colliding_change_id.last_mut().expect("a change ID is not empty");
        *last = if *last == b'k' { b'l' } else { b'k' };
        let colliding_change_id = gix::hash::ChangeId::from_reverse_hex(&colliding_change_id)?;
        let mut prefix_collision = repository.find_commit(head)?.decode()?.into_owned()?;
        prefix_collision.message = "prefix collision".into();
        prefix_collision
            .extra_headers
            .retain(|(name, _)| name.as_slice() != crate::change_id::HEADER.as_bytes());
        prefix_collision
            .extra_headers
            .push((crate::change_id::HEADER.into(), colliding_change_id.to_string().into()));
        let prefix_collision = repository.write_object(&prefix_collision)?.detach();
        let prefix_pin = "refs/worktree/tix/pins/prefix-collision";
        repository.reference(
            prefix_pin,
            prefix_collision,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "test change ID prefix collision",
        )?;
        let (_, resolved) = relocate_after_editor(&repository, &[], &[], change_id)?;
        assert_eq!(
            resolved, head,
            "a later prefix collision cannot make the captured full identity ambiguous"
        );

        let mut duplicate = repository.find_commit(head)?.decode()?.into_owned()?;
        duplicate.message = "duplicate identity".into();
        crate::change_id::inherit(&repository, &mut duplicate, head)?;
        let duplicate = repository.write_object(&duplicate)?.detach();
        let pin = "refs/worktree/tix/pins/duplicate";
        repository.reference(
            pin,
            duplicate,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "test duplicate identity",
        )?;

        let err = relocate_after_editor(&repository, &[], &[], change_id)
            .expect_err("two visible commits with the same identity are ambiguous");
        let message = format!("{err:#}");
        assert!(message.contains("ambiguous"), "ambiguity is explicit: {message}");
        for candidate in [head, duplicate] {
            assert!(
                message.contains(&candidate.to_hex_with_len(7).to_string()),
                "the error lists candidate {candidate}: {message}"
            );
        }
        assert_eq!(repository.head_id()?, head, "failed relocation leaves HEAD alone");
        assert_eq!(
            repository.find_reference(pin)?.id(),
            duplicate,
            "failed relocation leaves pins alone"
        );
        assert_eq!(
            repository.find_reference(prefix_pin)?.id(),
            prefix_collision,
            "failed relocation leaves prefix-collision pins alone"
        );
        assert_eq!(std::fs::read(repository.index_path())?, index_before);
        assert_eq!(std::fs::read(path.join("tip"))?, worktree_before);
        Ok(())
    }

    #[test]
    fn offers_only_missing_agent_trailer_keys() {
        assert_eq!(
            missing_agent_trailers(b"title\n\nASSISTED-BY: another agent\n"),
            [false, true],
            "trailer keys are matched case-insensitively and independently of their values"
        );
        assert_eq!(
            missing_agent_trailers(b"title\n\nco-AUTHORED-by: Someone <someone@example.com>\n"),
            [true, false],
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
    fn eagerly_replays_checked_out_reword_descendants() -> gix_testtools::Result {
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
            !rebase::is_pending(&repository.find_commit(new_tip)?.decode()?.into_owned()?),
            "the checked-out descendant is replayed eagerly"
        );
        Ok(())
    }

    #[test]
    fn head_edits_ignore_pending_history_below_the_hidden_base() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repository = crate::test_repository::open(fixture.path())?;
        let old_tip = repository.head_id()?.detach();
        let middle = repository.rev_parse_single("HEAD~1")?.detach();
        let base = repository.rev_parse_single("HEAD~2")?.detach();

        let mut pending = repository.find_commit(middle)?.decode()?.into_owned()?;
        pending
            .extra_headers
            .push(("tix-rebase-parent".into(), base.to_string().into()));
        let pending = repository.write_object(&pending)?.detach();
        let mut boundary = repository.find_commit(old_tip)?.decode()?.into_owned()?;
        boundary.parents = [pending].into_iter().collect();
        boundary.message = "hidden base".into();
        let boundary = repository.write_object(&boundary)?.detach();
        let mut head = repository.find_commit(old_tip)?.decode()?.into_owned()?;
        head.parents = [boundary].into_iter().collect();
        head.message = "head".into();
        let head = repository.write_object(&head)?.detach();
        repository
            .find_reference("refs/heads/main")?
            .set_target_id(head, "prepare hidden pending ancestry")?;
        repository.reference(
            "refs/heads/base",
            boundary,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "prepare inferred hidden base",
        )?;
        let boundary = boundary.to_string();
        for args in [
            vec!["config", "remote.origin.url", "."],
            vec!["config", "remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*"],
            vec!["update-ref", "refs/remotes/origin/base", &boundary],
            vec!["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/base"],
        ] {
            assert!(
                Command::new("git")
                    .arg("-C")
                    .arg(fixture.path())
                    .args(&args)
                    .status()?
                    .success(),
                "git {args:?} prepares remote HEAD inference"
            );
        }
        let boundary = gix::ObjectId::from_hex(boundary.as_bytes())?;
        drop(repository);
        let repository = crate::test_repository::open(fixture.path())?;

        let graph = super::super::loaded_view_graph(&repository)?;
        let outcome = apply_message_reporting(repository.clone(), &graph, head, b"reworded head\n", None)?;
        let rewritten = outcome.commit.expect("the message changed");
        assert_eq!(
            repository
                .find_commit(rewritten)?
                .parent_ids()
                .next()
                .map(gix::Id::detach),
            Some(boundary),
            "the reword keeps the hidden base"
        );
        std::fs::write(fixture.path().join("tip"), b"amended\n")?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["add", "tip"])
                .status()?
                .success(),
            "the index change is staged"
        );
        let graph = super::super::loaded_view_graph(&repository)?;
        let amended =
            super::super::head::amend_index(repository.clone(), &graph)?.expect("the staged tree changes amend HEAD");
        assert_eq!(
            repository
                .find_commit(amended)?
                .parent_ids()
                .next()
                .map(gix::Id::detach),
            Some(boundary),
            "the index amend keeps the hidden base"
        );
        assert!(
            rebase::is_pending(&repository.find_commit(pending)?.decode()?.into_owned()?),
            "pending history below the hidden base is unrelated"
        );
        Ok(())
    }

    #[test]
    fn signed_rewords_sign_eager_checked_out_descendants() -> gix_testtools::Result {
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

        assert!(
            repository
                .find_commit(repository.head_id()?)?
                .verify_signature()?
                .expect("the eagerly replayed descendant is signed")
                .is_valid(),
            "the checked-out descendant receives its configured signature"
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
                repository.find_reference(name)?.id(),
                new_id,
                "{name} follows the rewrite"
            );
        }
        for name in ["refs/tags/keep", "refs/remotes/origin/keep"] {
            assert_eq!(repository.find_reference(name)?.id(), old_id, "{name} is not rewritten");
        }
        Ok(())
    }
}
