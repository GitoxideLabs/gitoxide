use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use gix::{
    ObjectId,
    bstr::{BString, ByteSlice},
    prelude::ObjectIdExt,
};

use super::rebase;

const HELP: &str = r#"

<!--
# Rebase todo help

- Each fork section lists a stack from its oldest commit to its newest. Blank lines are ignored.
- `pick <id>` keeps a commit. Delete its line to drop it, or move the line to reorder it. Each listed commit may be picked only once.
- `squash <id>` folds a commit into the preceding command in the same fork. Its full message is retained with a source heading, and additional authors become `Co-authored-by` trailers.
- `## fork <id>` starts a stack at an existing commit or an earlier picked commit. The selected hidden boundary is labelled `(base)` with its title; a newer hidden tip used by rebase-update is `(updated-base)`, and an explicit command-line target is `(onto)`. Other fork headings stay terse. Delete a fork heading to continue its commits on the preceding stack; add one to create a fork. A listed commit must be picked before it can be a fork target.
- `empty <title>` creates an empty commit with the text after the command as its title.
- Commands may be plain text or enclosed in backticks. Text after a backticked command and text after a fork ID is display-only context.
- Prefix `pick`, `squash`, or `empty` with `@` to choose the post-rebase checkout. Retain exactly one generated marker; if none was generated, add at most one. A checkout marker requires a worktree.
- Saving an unchanged document in the history-view editor is a no-op unless listed commits have a pending rebase. Explicit `tix rebase apply` and `--edit-and-apply` apply valid unchanged plans. The ancestry ending at `@` is cherry-picked and re-signed; other stacks remain lazily rebased with invalidated signatures until time travel reaches them.
- Mutable refs on original leaves stay on the primary resulting leaf; the first continuation in todo order is primary. Other mutable refs follow their commits. Tags and remote-tracking refs stay unchanged, and new unreferenced leaves are pinned.
- A todo conflict aborts without changing the repository and marks the offending commit in the history view; concurrent ref changes also abort the update.
- Commit states are display-only: `↻` means a lazy rebase is pending, `◌` an empty signature awaits signing, `◐` a signature is present but unverified, and `○` means unsigned.
-->
"#;

const STATE_START: &str = "<!-- tix-rebase-state-v1\n";
const STATE_END: &str = "-->";
const STATE_CLOSE: &str = "\n-->";

pub(crate) struct Commit {
    pub id: ObjectId,
    pub parents: Vec<ObjectId>,
    pub info: String,
}

#[derive(Debug)]
pub(crate) struct Prepared {
    pub document: Vec<u8>,
    pub apply_unchanged: bool,
}

#[derive(Clone, Copy)]
pub(crate) enum OntoKind {
    UpdatedBase,
    Onto,
}

struct State {
    base: ObjectId,
    onto: ObjectId,
    tips: Vec<ObjectId>,
    scope: Vec<ObjectId>,
    marker_required: bool,
    checkout_allowed: bool,
    expected_refs: Vec<rebase::ExpectedRef>,
}

#[derive(Debug)]
pub(crate) struct Parsed {
    pub plan: rebase::Plan,
    pub tips: Vec<ObjectId>,
}

struct Section {
    parent: ObjectId,
    commits: Vec<ObjectId>,
}

#[tracing::instrument(skip_all, fields(base = %base, commits = commits.len()))]
pub(crate) fn prepare(
    repo: &gix::Repository,
    base: ObjectId,
    onto: ObjectId,
    commits: &[Commit],
    head: Option<ObjectId>,
    resolved_tips: &[ObjectId],
    onto_kind: OntoKind,
) -> Result<Prepared> {
    repo.find_commit(base)
        .context("could not find the selected rebase base")?;
    repo.find_commit(onto).context("could not find the rebase target")?;
    let scope: Vec<_> = commits.iter().map(|commit| commit.id).collect();
    let scope_set: HashSet<_> = scope.iter().copied().collect();
    let marker_required = head.is_some_and(|head| scope_set.contains(&head));
    let mut tips = scope_set.clone();
    for commit in commits {
        for parent in &commit.parents {
            tips.remove(parent);
        }
    }
    let tips = tips.into_iter().collect::<Vec<_>>();
    let expected_refs = rebase::capture_refs(repo, &scope, &tips)?;
    let has_pending = commits.iter().try_fold(false, |pending, commit| {
        Ok::<_, anyhow::Error>(pending || rebase::is_pending(&repo.find_commit(commit.id)?.decode()?.into_owned()?))
    })?;
    let apply_unchanged = base != onto || has_pending;

    let mut children = HashMap::<ObjectId, Vec<ObjectId>>::new();
    let by_id: HashMap<_, _> = commits.iter().map(|commit| (commit.id, commit)).collect();
    for commit in commits {
        let parent = commit
            .parents
            .first()
            .copied()
            .context("an editable commit has no parent")?;
        if parent != base && !scope_set.contains(&parent) {
            anyhow::bail!("an editable commit is not connected to the selected base");
        }
        children.entry(parent).or_default().push(commit.id);
    }
    let mut sections = Vec::new();
    for child in children.get(&base).into_iter().flatten().copied() {
        let mut section = Section {
            parent: onto,
            commits: Vec::new(),
        };
        let mut branches = Vec::new();
        walk(child, &children, &mut section, &mut branches);
        sections.push(section);
        sections.extend(branches);
    }

    let source = short(repo, base)?;
    let title = if base == onto {
        format!("# Rebase from `{source}`")
    } else {
        format!("# Rebase from `{source}` onto `{}`", short(repo, onto)?)
    };
    let state = State {
        base,
        onto,
        tips: if resolved_tips.is_empty() {
            tips
        } else {
            resolved_tips.to_vec()
        },
        scope: scope.clone(),
        marker_required,
        checkout_allowed: repo.workdir().is_some(),
        expected_refs,
    };
    let mut document = b"<!-- Rebase help follows the editable todo. -->\n".to_vec();
    document.extend_from_slice(title.as_bytes());
    document.extend_from_slice(b"\n\n");
    let anchor_kind = if base == onto {
        "base"
    } else {
        match onto_kind {
            OntoKind::UpdatedBase => "updated-base",
            OntoKind::Onto => "onto",
        }
    };
    let anchor_title = anchor_title(repo, onto)?;
    for (section_index, section) in sections.iter().enumerate() {
        if section_index > 0 {
            document.push(b'\n');
        }
        write_fork_heading(
            &mut document,
            repo,
            section.parent,
            (section.parent == onto).then_some((anchor_kind, anchor_title.as_str())),
        )?;
        for id in &section.commits {
            let commit = by_id[id];
            let verb = if Some(*id) == head { "@pick" } else { "pick" };
            let states = commit_states(repo, *id)?;
            document.extend_from_slice(
                format!(
                    "`{verb} {}` {states}{}\n",
                    short(repo, *id)?,
                    escape_markdown(&commit.info)
                )
                .as_bytes(),
            );
        }
    }
    if sections.is_empty() {
        write_fork_heading(&mut document, repo, onto, Some((anchor_kind, anchor_title.as_str())))?;
    }
    document.extend_from_slice(HELP.as_bytes());
    write_state(&mut document, &state);
    Ok(Prepared {
        document,
        apply_unchanged,
    })
}

fn write_state(out: &mut Vec<u8>, state: &State) {
    out.extend_from_slice(STATE_START.as_bytes());
    out.extend_from_slice(format!("base {}\nonto {}\n", state.base, state.onto).as_bytes());
    for tip in &state.tips {
        out.extend_from_slice(format!("tip {tip}\n").as_bytes());
    }
    for id in &state.scope {
        out.extend_from_slice(format!("scope {id}\n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "marker-required {}\ncheckout-allowed {}\n",
            state.marker_required, state.checkout_allowed
        )
        .as_bytes(),
    );
    for reference in &state.expected_refs {
        let name = gix::quote::ansi_c::quote(reference.name.as_bstr());
        out.extend_from_slice(
            format!(
                "ref {} {} {}\n",
                reference.old,
                reference.follows_tip,
                name.to_str_lossy()
            )
            .as_bytes(),
        );
    }
    out.extend_from_slice(STATE_END.as_bytes());
    out.push(b'\n');
}

fn commit_states(repo: &gix::Repository, id: ObjectId) -> Result<String> {
    let commit = repo
        .find_commit(id)
        .context("could not load a commit state for the rebase todo")?
        .decode()
        .context("could not decode a commit state for the rebase todo")?
        .into_owned()
        .context("could not own a commit state for the rebase todo")?;
    let pending = commit.extra_headers.iter().any(|(name, _)| name == "tix-rebase-parent");
    let mut empty_signature = false;
    let mut signature = false;
    for (name, value) in &commit.extra_headers {
        if name != "gpgsig" && name != "gpgsig-sha256" {
            continue;
        }
        if value.is_empty() {
            empty_signature = true;
        } else {
            signature = true;
        }
    }
    let mut out = Vec::with_capacity(3);
    if pending {
        out.push("↻");
    }
    if empty_signature {
        out.push("◌");
    }
    if signature {
        out.push("◐");
    }
    if !empty_signature && !signature {
        out.push("○");
    }
    Ok(format!("{} ", out.join(" ")))
}

fn anchor_title(repo: &gix::Repository, id: ObjectId) -> Result<String> {
    let message = repo
        .find_commit(id)
        .context("could not load the rebase anchor")?
        .message_raw()
        .context("could not decode the rebase anchor message")?
        .to_owned();
    let mut notes = repo
        .notes()
        .map_err(gix::Exn::into_error)
        .context("could not open Git notes for the rebase anchor")?;
    let has_notes = !notes
        .get(id)
        .map_err(gix::Exn::into_error)
        .context("could not load rebase anchor notes")?
        .is_empty();
    let mut out = String::new();
    if crate::history::contains_agent_marker(&message) {
        out.push_str("[A] ");
    }
    if has_notes {
        out.push_str("[N] ");
    }
    out.push_str(
        &gix::objs::commit::MessageRef::from_bytes(&message)
            .summary()
            .to_str_lossy(),
    );
    Ok(out)
}

fn write_fork_heading(
    out: &mut Vec<u8>,
    repo: &gix::Repository,
    id: ObjectId,
    annotation: Option<(&str, &str)>,
) -> Result<()> {
    out.extend_from_slice(format!("## fork {}", short(repo, id)?).as_bytes());
    if let Some((kind, title)) = annotation {
        out.extend_from_slice(format!(" ({kind}) {}", escape_markdown(title)).as_bytes());
    }
    out.push(b'\n');
    Ok(())
}

fn walk(id: ObjectId, children: &HashMap<ObjectId, Vec<ObjectId>>, section: &mut Section, sections: &mut Vec<Section>) {
    section.commits.push(id);
    let Some(child_ids) = children.get(&id) else { return };
    if let Some(first) = child_ids.first() {
        walk(*first, children, section, sections);
    }
    for child in child_ids.iter().skip(1) {
        let mut branch = Section {
            parent: id,
            commits: Vec::new(),
        };
        let mut nested = Vec::new();
        walk(*child, children, &mut branch, &mut nested);
        sections.push(branch);
        sections.extend(nested);
    }
}

fn short(repo: &gix::Repository, id: ObjectId) -> Result<String> {
    Ok(id
        .attach(repo)
        .shorten()
        .context("could not shorten a rebase todo ID")?
        .to_string())
}

fn escape_markdown(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if matches!(ch, '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

fn parse_state(repo: &gix::Repository, input: &str) -> Result<Option<State>> {
    let Some(start) = input.find(STATE_START) else {
        if input.contains("<!-- tix-rebase-state-") {
            anyhow::bail!("the rebase todo uses an unsupported state version");
        }
        return Ok(None);
    };
    let body = &input[start + STATE_START.len()..];
    let end = body
        .find(STATE_CLOSE)
        .context("the rebase state anchor is not closed")?;
    if body[end + STATE_CLOSE.len()..].contains(STATE_START) {
        anyhow::bail!("the rebase todo contains more than one state anchor");
    }
    let mut base = None;
    let mut onto = None;
    let mut tips = Vec::new();
    let mut scope = Vec::new();
    let mut marker_required = None;
    let mut checkout_allowed = None;
    let mut expected_refs = Vec::new();
    for line in body[..end].lines() {
        let (key, value) = line.split_once(' ').context("a rebase state line has no value")?;
        match key {
            "base" => {
                if base.replace(ObjectId::from_hex(value.as_bytes())?).is_some() {
                    anyhow::bail!("the rebase state has more than one base");
                }
            }
            "onto" => {
                if onto.replace(ObjectId::from_hex(value.as_bytes())?).is_some() {
                    anyhow::bail!("the rebase state has more than one onto target");
                }
            }
            "tip" => tips.push(ObjectId::from_hex(value.as_bytes())?),
            "scope" => scope.push(ObjectId::from_hex(value.as_bytes())?),
            "marker-required" => {
                if marker_required.replace(value.parse()?).is_some() {
                    anyhow::bail!("the rebase state repeats marker-required");
                }
            }
            "checkout-allowed" => {
                if checkout_allowed.replace(value.parse()?).is_some() {
                    anyhow::bail!("the rebase state repeats checkout-allowed");
                }
            }
            "ref" => {
                let (old, value) = value.split_once(' ').context("a captured ref has no follow mode")?;
                let (follows_tip, name) = value.split_once(' ').context("a captured ref has no name")?;
                let old = ObjectId::from_hex(old.as_bytes())?;
                let follows_tip = follows_tip.parse()?;
                let encoded_name = name.as_bytes().as_bstr();
                let (name, consumed) = gix::quote::ansi_c::undo(encoded_name)
                    .map_err(gix::Exn::into_error)
                    .context("could not unquote a captured ref name")?;
                if !encoded_name[consumed..].trim().is_empty() {
                    anyhow::bail!("a captured ref name has trailing data");
                }
                let name = gix::refs::FullName::try_from(name.as_ref()).context("a captured ref name is invalid")?;
                expected_refs.push(rebase::ExpectedRef {
                    name,
                    old,
                    new: old,
                    follows_tip,
                });
            }
            _ => anyhow::bail!("unsupported rebase state field {key:?}"),
        }
    }
    let state = State {
        base: base.context("the rebase state has no base")?,
        onto: onto.context("the rebase state has no onto target")?,
        tips,
        scope,
        marker_required: marker_required.context("the rebase state has no marker requirement")?,
        checkout_allowed: checkout_allowed.context("the rebase state has no checkout capability")?,
        expected_refs,
    };
    validate_state(repo, &state)?;
    Ok(Some(state))
}

fn validate_state(repo: &gix::Repository, state: &State) -> Result<()> {
    repo.find_commit(state.base)
        .context("could not find the recorded rebase base")?;
    repo.find_commit(state.onto)
        .context("could not find the recorded rebase target")?;
    let scope: HashSet<_> = state.scope.iter().copied().collect();
    if scope.len() != state.scope.len() {
        anyhow::bail!("the rebase state contains duplicate scope commits");
    }
    if state.tips.iter().copied().collect::<HashSet<_>>().len() != state.tips.len() {
        anyhow::bail!("the rebase state contains duplicate tips");
    }
    let mut refs = HashSet::new();
    for reference in &state.expected_refs {
        if !refs.insert(reference.name.as_bstr()) {
            anyhow::bail!("the rebase state contains duplicate refs");
        }
        if !scope.contains(&reference.old) {
            anyhow::bail!("a captured ref does not point into the rebase scope");
        }
    }
    for tip in &state.tips {
        repo.find_commit(*tip).context("could not find a recorded rebase tip")?;
    }
    for id in &state.scope {
        let commit = repo
            .find_commit(*id)
            .context("could not find a recorded scope commit")?;
        let parent = commit
            .parent_ids()
            .next()
            .map(gix::Id::detach)
            .context("a recorded scope commit has no parent")?;
        if parent != state.base && !scope.contains(&parent) {
            anyhow::bail!("a recorded scope commit is disconnected from the rebase base");
        }
    }
    Ok(())
}

pub(crate) fn parse(repo: &gix::Repository, edited: &[u8]) -> Result<Option<Parsed>> {
    let input = std::str::from_utf8(edited).context("the rebase todo is not UTF-8")?;
    let Some(state) = parse_state(repo, input)? else {
        return Ok(None);
    };
    let scope: HashSet<_> = state.scope.iter().copied().collect();
    let mut picked = HashMap::<ObjectId, usize>::new();
    let mut steps = Vec::<rebase::PlanStep>::new();
    let mut cursor = None;
    let mut checkout = None;
    let mut marker_count = 0;
    let mut sections = 0;
    let mut section_has_commit = false;
    let mut section_last_step = None;
    let mut in_comment = false;

    for raw in input.lines() {
        let line = raw.trim();
        if in_comment {
            if line.contains("-->") {
                in_comment = false;
            }
            continue;
        }
        if line.starts_with("<!--") {
            in_comment = !line.contains("-->");
            continue;
        }
        if line.is_empty() || line.starts_with("# ") {
            continue;
        }
        if let Some(target) = line.strip_prefix("## fork ") {
            if sections > 0 && !section_has_commit {
                anyhow::bail!("a fork section contains no commits");
            }
            let id = resolve_commit(
                repo,
                target
                    .split_whitespace()
                    .next()
                    .context("a fork heading needs a commit ID")?,
            )?;
            cursor = Some(if let Some(index) = picked.get(&id) {
                rebase::PlanParent::Step(*index)
            } else if scope.contains(&id) {
                anyhow::bail!("a fork target must be picked before it is used");
            } else {
                rebase::PlanParent::Existing(id)
            });
            sections += 1;
            section_has_commit = false;
            section_last_step = None;
            continue;
        }

        let (command, tail) = if let Some(line) = line.strip_prefix('`') {
            let (command, tail) = line
                .split_once('`')
                .context("a Markdown todo command has no closing backtick")?;
            (command, tail.trim())
        } else {
            (line, "")
        };
        let (verb, value) = command.split_once(char::is_whitespace).unwrap_or((command, ""));
        let marked = verb.starts_with('@');
        let verb = verb.strip_prefix('@').unwrap_or(verb);
        if marked {
            marker_count += 1;
            if marker_count > 1 {
                anyhow::bail!("the rebase todo contains more than one @ marker");
            }
            if !state.checkout_allowed || repo.workdir().is_none() {
                anyhow::bail!("the rebase todo cannot select a checkout without a worktree");
            }
        }
        if verb == "squash" {
            let index = section_last_step.context("a squash must follow a command in the same fork")?;
            let id = resolve_commit(
                repo,
                value.split_whitespace().next().context("a squash needs a commit ID")?,
            )?;
            if !scope.contains(&id) {
                anyhow::bail!("a squash is outside the editable history");
            }
            if picked.insert(id, index).is_some() {
                anyhow::bail!("a commit is picked more than once");
            }
            steps[index].squash.push(id);
            if marked {
                checkout = Some(index);
            }
            section_has_commit = true;
            continue;
        }
        let parent = cursor.context("the first todo command must follow a fork heading")?;
        let commit = match verb {
            "pick" => {
                let id = resolve_commit(
                    repo,
                    value.split_whitespace().next().context("a pick needs a commit ID")?,
                )?;
                if !scope.contains(&id) {
                    anyhow::bail!("a pick is outside the editable history");
                }
                if picked.contains_key(&id) {
                    anyhow::bail!("a commit is picked more than once");
                }
                rebase::PlanCommit::Pick(id)
            }
            "empty" => {
                let title = if value.trim().is_empty() { tail } else { value.trim() };
                if title.is_empty() {
                    anyhow::bail!("an empty commit needs a title");
                }
                rebase::PlanCommit::Empty(BString::from(title))
            }
            _ => anyhow::bail!("unsupported rebase todo command {verb:?}"),
        };
        let index = steps.len();
        if let rebase::PlanCommit::Pick(id) = commit {
            picked.insert(id, index);
        }
        steps.push(rebase::PlanStep {
            parent,
            commit,
            squash: Vec::new(),
        });
        cursor = Some(rebase::PlanParent::Step(index));
        section_last_step = Some(index);
        if marked {
            checkout = Some(index);
        }
        section_has_commit = true;
    }
    if sections == 0 {
        anyhow::bail!("the rebase todo has no fork heading");
    }
    if sections > 1 && !section_has_commit {
        anyhow::bail!("the last fork section contains no commits");
    }
    if state.marker_required && marker_count != 1 {
        anyhow::bail!("the current checkout marker must be retained");
    }
    Ok(Some(Parsed {
        plan: rebase::Plan {
            base: state.onto,
            scope: state.scope,
            steps,
            checkout,
            expected_refs: state.expected_refs,
        },
        tips: state.tips,
    }))
}

fn resolve_commit(repo: &gix::Repository, value: &str) -> Result<ObjectId> {
    if value.len() < 4 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        anyhow::bail!("{value:?} is not a commit ID prefix");
    }
    let id = repo
        .rev_parse_single(value)
        .with_context(|| format!("could not resolve commit ID {value:?}"))?;
    id.object()
        .context("could not load a todo object")?
        .try_into_commit()
        .context("a todo ID does not name a commit")?;
    Ok(id.detach())
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    fn repo() -> gix_testtools::Result<(gix_testtools::tempfile::TempDir, gix::Repository)> {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = gix::open_opts(
            fixture.path(),
            gix::open::Options::isolated().config_overrides([
                "core.editor=:".to_owned(),
                "core.abbrev=7".to_owned(),
                "user.name=todo author".to_owned(),
                "user.email=todo@example.com".to_owned(),
            ]),
        )?;
        Ok((fixture, repo))
    }

    fn commits(repo: &gix::Repository) -> gix_testtools::Result<(ObjectId, ObjectId, ObjectId, Vec<Commit>)> {
        let base = repo.rev_parse_single("HEAD~2")?.detach();
        let middle = repo.rev_parse_single("HEAD~1")?.detach();
        let tip = repo.head_id()?.detach();
        Ok((
            base,
            middle,
            tip,
            vec![
                Commit {
                    id: tip,
                    parents: vec![middle],
                    info: "(main) 2000-01-03 author tip".into(),
                },
                Commit {
                    id: middle,
                    parents: vec![base],
                    info: "2000-01-02 author middle * markdown".into(),
                },
            ],
        ))
    }

    fn prepare_test(
        repo: &gix::Repository,
        base: ObjectId,
        onto: ObjectId,
        commits: &[Commit],
        head: Option<ObjectId>,
    ) -> Result<Prepared> {
        prepare(repo, base, onto, commits, head, &[], OntoKind::UpdatedBase)
    }

    fn parse_plan(repo: &gix::Repository, document: &[u8]) -> Result<rebase::Plan> {
        Ok(parse(repo, document)?.context("the test todo was cancelled")?.plan)
    }

    fn with_state(prepared: &Prepared, commands: &str) -> Vec<u8> {
        let document = std::str::from_utf8(&prepared.document).expect("generated todo is UTF-8");
        let start = document.find(STATE_START).expect("generated todo has state");
        let end = document[start..].find(STATE_CLOSE).expect("generated state is closed") + start + STATE_CLOSE.len();
        format!("{}\n{commands}", &document[start..end]).into_bytes()
    }

    #[test]
    fn markdown_flows_from_base_to_tip_and_uses_repository_abbreviations() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let (base, _middle, tip, commits) = commits(&repo)?;
        let prepared = prepare_test(&repo, base, base, &commits, Some(tip))?;
        assert!(!prepared.apply_unchanged);
        let document = String::from_utf8(prepared.document.clone())?;
        assert!(document.starts_with("<!-- Rebase help follows the editable todo. -->"));
        assert!(document.contains(STATE_START), "the todo carries its transaction state");
        assert!(document.contains(&format!("# Rebase from `{}`", base.to_hex_with_len(7))));
        assert!(document.contains(&format!("## fork {} (base) base", base.to_hex_with_len(7))));
        let middle = document.find("`pick ").expect("the oldest pick is shown");
        let tip = document.find("`@pick ").expect("HEAD is marked");
        assert!(middle < tip, "the todo grows from the base towards the tip");
        assert!(
            document.contains("middle \\* markdown"),
            "metadata is escaped for Markdown"
        );
        assert!(
            document.find("# Rebase todo help").expect("help is present") > tip,
            "complete instructions follow the editable todo"
        );
        assert!(
            document.find(STATE_START).expect("state is present")
                > document.find("# Rebase todo help").expect("help is present"),
            "transaction state follows the complete help"
        );
        assert!(document.ends_with("-->\n"), "the trailing state is a Markdown comment");
        assert!(
            document.contains("○"),
            "unsigned commits carry the documented status symbol"
        );
        Ok(())
    }

    #[test]
    fn state_round_trips_non_utf8_ref_names_and_controls_cancellation() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let (base, middle, _tip, _commits) = commits(&repo)?;
        let name = gix::refs::FullName::try_from(BString::from(vec![
            b'r', b'e', b'f', b's', b'/', b'h', b'e', b'a', b'd', b's', b'/', 0xff,
        ]))?;
        let state = State {
            base,
            onto: base,
            tips: vec![middle],
            scope: vec![middle],
            marker_required: false,
            checkout_allowed: true,
            expected_refs: vec![rebase::ExpectedRef {
                name: name.clone(),
                old: middle,
                new: middle,
                follows_tip: true,
            }],
        };
        let mut document = Vec::new();
        write_state(&mut document, &state);
        let document = String::from_utf8(document)?;
        assert!(
            document.contains(r#""refs/heads/\377""#),
            "non-UTF-8 names use Git quoting"
        );
        let parsed = parse_state(&repo, &document)?.context("state is present")?;
        assert_eq!(parsed.expected_refs[0].name, name, "quoted names round-trip losslessly");

        assert!(parse(&repo, b"")?.is_none(), "empty input cancels");
        assert!(parse(&repo, b"pick deadbeef")?.is_none(), "removing the anchor cancels");
        assert!(
            parse(&repo, b"<!-- tix-rebase-state-v2\n-->").is_err(),
            "an unsupported present anchor is rejected"
        );
        Ok(())
    }

    #[test]
    fn an_unchanged_todo_replays_pending_commits_with_normal_plan_semantics() -> gix_testtools::Result {
        let (fixture, repo) = repo()?;
        let (base, middle, _tip, _) = commits(&repo)?;
        let graph = super::super::loaded_graph(&repo)?;
        let mut commit = repo.find_commit(middle)?.decode()?.into_owned()?;
        commit.tree = repo.find_commit(base)?.tree_id()?.detach();
        let marked = rebase::perform(
            &repo,
            &graph,
            rebase::Edit::Replace { target: middle, commit },
            rebase::Signature::InvalidateExisting,
            rebase::Tree::LeaveAsIsAndMark,
        )?
        .complete()?
        .selected
        .expect("the pending replacement selects its rewritten commit");
        let tip = repo.head_id()?.detach();
        let commits = vec![
            Commit {
                id: tip,
                parents: vec![marked],
                info: "tip".into(),
            },
            Commit {
                id: marked,
                parents: vec![base],
                info: "middle".into(),
            },
        ];
        let prepared = prepare_test(&repo, base, base, &commits, Some(tip))?;
        assert!(
            prepared.apply_unchanged,
            "pending commits make an unchanged todo actionable"
        );
        let document = prepared.document.clone();
        let plan = parse_plan(&repo, &document)?;
        let graph = super::super::loaded_graph(&repo)?;
        rebase::perform_plan(&repo, &graph, plan)?.complete()?;

        let mut current = Some(repo.head_id()?.detach());
        while let Some(id) = current {
            let commit = repo.find_commit(id)?.decode()?.into_owned()?;
            assert!(!rebase::has_marker(&commit), "the eager @ ancestry is replayed");
            current = commit.parents.first().copied();
        }
        let files = Command::new("git")
            .arg("-C")
            .arg(fixture.path())
            .args(["ls-tree", "-r", "--name-only", "HEAD"])
            .output()?;
        assert!(files.status.success());
        assert_eq!(files.stdout, b"base\ntip\n", "replay uses the recorded original parent");
        Ok(())
    }

    #[test]
    fn descendant_forks_stay_terse() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let (base, middle, tip, mut commits) = commits(&repo)?;
        let mut sibling = repo.find_commit(tip)?.decode()?.into_owned()?;
        sibling.parents = [middle].into_iter().collect();
        sibling.message = "sibling".into();
        let sibling = repo.write_object(&sibling)?.detach();
        commits.insert(
            0,
            Commit {
                id: sibling,
                parents: vec![middle],
                info: "sibling title".into(),
            },
        );

        let prepared = prepare_test(&repo, base, base, &commits, Some(tip))?;
        let document = String::from_utf8(prepared.document.clone())?;
        assert!(document.contains(&format!("## fork {} (base) base", base.to_hex_with_len(7))));
        assert!(
            document.contains(&format!("## fork {}\n", middle.to_hex_with_len(7))),
            "a fork within the editable tree has no external-anchor annotation"
        );
        let plan = parse_plan(&repo, document.as_bytes())?;
        assert_eq!(plan.steps.len(), 3, "display annotations do not alter the plan");
        Ok(())
    }

    #[test]
    fn update_todo_roots_the_stack_at_the_hidden_tip_and_labels_only_that_heading() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let (base, middle, tip, commits) = commits(&repo)?;
        let mut commit = repo.find_commit(base)?.decode()?.into_owned()?;
        commit.parents = [base].into_iter().collect();
        commit.message = "updated * hidden base\n\n<!-- agent -->".into();
        let onto = repo.write_object(&commit)?.detach();
        repo.notes()
            .map_err(gix::Exn::into_error)?
            .add("refs/notes/commits", onto, "anchor note")
            .map_err(gix::Exn::into_error)?;

        let prepared = prepare_test(&repo, base, onto, &commits, Some(tip))?;
        assert!(
            prepared.apply_unchanged,
            "moving the base makes an unchanged editor document actionable"
        );
        let document = String::from_utf8(prepared.document.clone())?;
        assert!(
            document.contains(&format!(
                "# Rebase from `{}` onto `{}`",
                base.to_hex_with_len(7),
                onto.to_hex_with_len(7)
            )),
            "the update target is explicit in the document title"
        );
        assert!(
            document.contains(&format!(
                "## fork {} (updated-base) \\[A\\] \\[N\\] updated \\* hidden base",
                onto.to_hex_with_len(7)
            )),
            "the unfamiliar fork target carries its escaped UI title"
        );
        assert_eq!(
            document.matches("updated \\* hidden base").count(),
            1,
            "only the new update target is labelled"
        );

        let plan = parse_plan(&repo, document.as_bytes())?;
        assert_eq!(plan.base, onto);
        assert_eq!(plan.steps[0].parent, rebase::PlanParent::Existing(onto));
        let graph = super::super::loaded_graph(&repo)?;
        let outcome = rebase::perform_plan(&repo, &graph, plan)?.complete()?;
        let rewritten_middle = outcome.map(middle).expect("the middle commit is retained");
        assert_eq!(
            repo.find_commit(rewritten_middle)?
                .parent_ids()
                .next()
                .map(gix::Id::detach),
            Some(onto),
            "saving the unchanged update todo rebases the stack onto the hidden tip"
        );
        Ok(())
    }

    #[test]
    fn parses_reordering_forks_empty_commits_and_a_moved_checkout() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let (base, middle, tip, commits) = commits(&repo)?;
        let prepared = prepare_test(&repo, base, base, &commits, Some(tip))?;
        let edited = format!(
            "# Rebase\n\n## fork {}\n`pick {}` ignored\n@empty a new checkpoint\n\n## fork {}\n@pick {}\n",
            base.to_hex_with_len(7),
            tip.to_hex_with_len(7),
            tip.to_hex_with_len(7),
            middle.to_hex_with_len(7),
        );
        let edited = with_state(&prepared, &edited);
        let err = parse(&repo, &edited).expect_err("two checkout markers are invalid");
        assert!(format!("{err:#}").contains("more than one @"));

        let prepared = prepare_test(&repo, base, base, &commits, Some(tip))?;
        let edited = format!(
            "## fork {}\npick {} ignored display metadata\nempty a new checkpoint\n\n## fork {}\n@pick {}\n",
            base.to_hex_with_len(7),
            tip.to_hex_with_len(7),
            tip.to_hex_with_len(7),
            middle.to_hex_with_len(7),
        );
        let edited = with_state(&prepared, &edited);
        let plan = parse_plan(&repo, &edited)?;
        assert_eq!(plan.steps.len(), 3);
        assert_eq!(plan.checkout, Some(2));
        assert_eq!(plan.steps[2].parent, rebase::PlanParent::Step(0));
        assert!(matches!(&plan.steps[1].commit, rebase::PlanCommit::Empty(title) if title == b"a new checkpoint"));
        Ok(())
    }

    #[test]
    fn squash_folds_into_the_previous_command_and_may_carry_checkout() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let (base, middle, tip, commits) = commits(&repo)?;
        let prepared = prepare_test(&repo, base, base, &commits, Some(tip))?;
        let edited = with_state(
            &prepared,
            &format!(
                "## fork {}\npick {}\n`@squash {}` ignored display metadata\n\n## fork {}\nempty side\n",
                base.to_hex_with_len(7),
                middle.to_hex_with_len(7),
                tip.to_hex_with_len(7),
                tip.to_hex_with_len(7),
            ),
        );
        let plan = parse_plan(&repo, &edited)?;
        assert_eq!(plan.steps.len(), 2, "squash does not produce another commit");
        assert_eq!(plan.steps[0].squash, [tip]);
        assert_eq!(plan.checkout, Some(0), "the squash marker selects the folded result");
        assert_eq!(
            plan.steps[1].parent,
            rebase::PlanParent::Step(0),
            "the squashed ID resolves to the folded result as a fork target"
        );

        let invalid = with_state(
            &prepared,
            &format!(
                "## fork {}\n@squash {}\npick {}\n",
                base.to_hex_with_len(7),
                tip.to_hex_with_len(7),
                middle.to_hex_with_len(7),
            ),
        );
        let err = parse(&repo, &invalid).expect_err("a fork cannot begin with squash");
        assert!(format!("{err:#}").contains("same fork"));
        Ok(())
    }

    #[test]
    fn unchanged_marker_cannot_be_removed_but_an_empty_plan_is_valid_without_one() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let (base, _middle, tip, commits) = commits(&repo)?;
        let prepared = prepare_test(&repo, base, base, &commits, Some(tip))?;
        let edited = format!("## fork {}\n", base.to_hex_with_len(7));
        let edited = with_state(&prepared, &edited);
        let err = parse(&repo, &edited).expect_err("HEAD must be moved before its pick is dropped");
        assert!(format!("{err:#}").contains("checkout marker"));

        let prepared = prepare_test(&repo, base, base, &commits, None)?;
        let edited = with_state(&prepared, &format!("## fork {}\n", base.to_hex_with_len(7)));
        let plan = parse_plan(&repo, &edited)?;
        assert!(
            plan.steps.is_empty(),
            "all picks may be dropped when no marker is required"
        );
        Ok(())
    }
}
